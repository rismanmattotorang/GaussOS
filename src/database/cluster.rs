// src/database/cluster.rs
//! Distributed-mode primitives: consistent-hash sharding.
//!
//! In distributed mode, memories are partitioned across a set of nodes. We use
//! a **consistent hash ring** with virtual nodes so that adding or removing a
//! node only remaps a small fraction (`~1/N`) of keys instead of reshuffling
//! everything. Each physical node is placed at `vnodes` positions on a 64-bit
//! ring; a key is routed to the first node clockwise from the key's hash.
//!
//! This module is pure (no networking) so it is fully unit-testable. The
//! networking/replication layer that consumes [`ShardRouter`] is built on top.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

fn hash64<T: Hash>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

/// Configuration for a cluster node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: String,
    pub address: String,
}

impl NodeConfig {
    pub fn new(id: impl Into<String>, address: impl Into<String>) -> Self {
        Self { id: id.into(), address: address.into() }
    }
}

/// Cluster-wide configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub nodes: Vec<NodeConfig>,
    /// Virtual nodes per physical node (higher = smoother distribution).
    pub vnodes: u32,
    /// Number of replicas each key is stored on (for redundancy).
    pub replication_factor: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self { nodes: Vec::new(), vnodes: 128, replication_factor: 1 }
    }
}

/// A consistent-hash ring over node ids with virtual nodes.
#[derive(Debug, Clone, Default)]
pub struct HashRing {
    /// ring position -> node id
    ring: BTreeMap<u64, String>,
    vnodes: u32,
}

impl HashRing {
    pub fn new(vnodes: u32) -> Self {
        Self { ring: BTreeMap::new(), vnodes: vnodes.max(1) }
    }

    /// Add a node, placing `vnodes` virtual points on the ring.
    pub fn add_node(&mut self, node_id: &str) {
        for v in 0..self.vnodes {
            let point = hash64(&format!("{node_id}#{v}"));
            self.ring.insert(point, node_id.to_string());
        }
    }

    /// Remove a node and all its virtual points.
    pub fn remove_node(&mut self, node_id: &str) {
        self.ring.retain(|_, n| n != node_id);
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// The node responsible for `key` (first node clockwise from the key hash).
    pub fn node_for<K: Hash>(&self, key: &K) -> Option<&str> {
        if self.ring.is_empty() {
            return None;
        }
        let h = hash64(key);
        // First ring point >= h, wrapping around to the first point.
        let node = self
            .ring
            .range(h..)
            .next()
            .or_else(|| self.ring.iter().next())
            .map(|(_, n)| n.as_str());
        node
    }

    /// The `n` distinct nodes responsible for `key`, walking clockwise. Used for
    /// replication; returns fewer than `n` only if the cluster is smaller.
    pub fn nodes_for<K: Hash>(&self, key: &K, n: usize) -> Vec<String> {
        if self.ring.is_empty() || n == 0 {
            return Vec::new();
        }
        let h = hash64(key);
        let mut out: Vec<String> = Vec::with_capacity(n);
        // Iterate clockwise from h, then wrap, collecting distinct node ids.
        let clockwise = self
            .ring
            .range(h..)
            .chain(self.ring.iter())
            .map(|(_, node)| node);
        for node in clockwise {
            if !out.iter().any(|existing| existing == node) {
                out.push(node.clone());
                if out.len() == n {
                    break;
                }
            }
        }
        out
    }

    /// Number of distinct physical nodes on the ring.
    pub fn node_count(&self) -> usize {
        let mut nodes: Vec<&String> = self.ring.values().collect();
        nodes.sort();
        nodes.dedup();
        nodes.len()
    }
}

/// Routes keys to nodes using a [`HashRing`], honouring the replication factor.
#[derive(Debug, Clone)]
pub struct ShardRouter {
    ring: HashRing,
    replication_factor: usize,
}

impl ShardRouter {
    pub fn new(config: &ClusterConfig) -> Self {
        let mut ring = HashRing::new(config.vnodes);
        for node in &config.nodes {
            ring.add_node(&node.id);
        }
        Self { ring, replication_factor: config.replication_factor.max(1) }
    }

    /// The primary node owning `key`.
    pub fn primary_for<K: Hash>(&self, key: &K) -> Option<&str> {
        self.ring.node_for(key)
    }

    /// All nodes (primary + replicas) that should hold `key`.
    pub fn replicas_for<K: Hash>(&self, key: &K) -> Vec<String> {
        self.ring.nodes_for(key, self.replication_factor)
    }

    /// True if `node_id` is among the replicas responsible for `key`.
    pub fn owns<K: Hash>(&self, node_id: &str, key: &K) -> bool {
        self.replicas_for(key).iter().any(|n| n == node_id)
    }

    pub fn add_node(&mut self, node_id: &str) {
        self.ring.add_node(node_id);
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.ring.remove_node(node_id);
    }

    pub fn node_count(&self) -> usize {
        self.ring.node_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ring_with(nodes: &[&str], vnodes: u32) -> HashRing {
        let mut r = HashRing::new(vnodes);
        for n in nodes {
            r.add_node(n);
        }
        r
    }

    #[test]
    fn empty_ring_returns_none() {
        let r = HashRing::new(16);
        assert!(r.node_for(&"k").is_none());
        assert!(r.nodes_for(&"k", 3).is_empty());
    }

    #[test]
    fn routing_is_deterministic() {
        let r = ring_with(&["a", "b", "c"], 64);
        let k = "user/alice/memory-42";
        assert_eq!(r.node_for(&k), r.node_for(&k));
        assert_eq!(r.node_count(), 3);
    }

    #[test]
    fn distribution_is_reasonably_balanced() {
        let r = ring_with(&["a", "b", "c", "d"], 256);
        let mut counts = std::collections::HashMap::new();
        for _ in 0..4000 {
            let key = Uuid::new_v4();
            *counts.entry(r.node_for(&key).unwrap().to_string()).or_insert(0u32) += 1;
        }
        assert_eq!(counts.len(), 4);
        // With 256 vnodes each node should get a meaningful share (>10%).
        for (_, c) in counts {
            assert!(c > 400, "node under-loaded: {c}");
        }
    }

    #[test]
    fn replicas_are_distinct_nodes() {
        let r = ring_with(&["a", "b", "c"], 128);
        let reps = r.nodes_for(&"some-key", 3);
        assert_eq!(reps.len(), 3);
        let mut uniq = reps.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), 3, "replicas must be on distinct nodes");
    }

    #[test]
    fn replicas_capped_at_cluster_size() {
        let r = ring_with(&["a", "b"], 64);
        // Asked for 5 but only 2 nodes exist.
        assert_eq!(r.nodes_for(&"k", 5).len(), 2);
    }

    #[test]
    fn removing_a_node_remaps_only_its_keys() {
        let r1 = ring_with(&["a", "b", "c"], 256);
        let mut r2 = r1.clone();
        r2.remove_node("c");

        let mut moved = 0;
        let mut total = 0;
        for _ in 0..3000 {
            let key = Uuid::new_v4();
            let before = r1.node_for(&key).unwrap().to_string();
            let after = r2.node_for(&key).unwrap().to_string();
            total += 1;
            if before != after {
                moved += 1;
                // keys only move off the removed node, never between survivors
                assert_eq!(before, "c");
            }
        }
        // Only ~1/3 of keys (those on "c") should remap.
        let frac = moved as f64 / total as f64;
        assert!(frac < 0.45, "too many keys remapped: {frac}");
    }

    #[test]
    fn shard_router_owns_primary_and_replicas() {
        let cfg = ClusterConfig {
            nodes: vec![
                NodeConfig::new("a", "10.0.0.1:9000"),
                NodeConfig::new("b", "10.0.0.2:9000"),
                NodeConfig::new("c", "10.0.0.3:9000"),
            ],
            vnodes: 128,
            replication_factor: 2,
        };
        let router = ShardRouter::new(&cfg);
        let key = "users/bob/note-7";
        let primary = router.primary_for(&key).unwrap().to_string();
        let replicas = router.replicas_for(&key);
        assert_eq!(replicas.len(), 2);
        assert_eq!(replicas[0], primary);
        assert!(router.owns(&primary, &key));
    }
}
