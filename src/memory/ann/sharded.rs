// src/memory/ann/sharded.rs
//! Sharded HNSW index for concurrency (Phase 2, #13).
//!
//! A single `RwLock<Hnsw>` serialises all writes, so concurrent ingestion
//! contends on one lock. `ShardedHnsw` partitions vectors across `N` independent
//! HNSW graphs keyed by `hash(id) % N`, each behind its own lock. Inserts and
//! deletes touch only one shard (so writers to different shards never block each
//! other), and a query fans out across shards and merges the per-shard top-k
//! into a global top-k.
//!
//! This is the contention-reducing, concurrency-friendly index; combined with
//! the per-shard `parking_lot` locks it keeps write throughput high under
//! parallel ingestion.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use parking_lot::RwLock;
use uuid::Uuid;

use super::hnsw::{Distance, Hnsw, HnswConfig, Neighbor};

/// A concurrency-friendly, sharded HNSW index.
pub struct ShardedHnsw {
    shards: Vec<RwLock<Hnsw>>,
}

impl ShardedHnsw {
    /// Create an index with `num_shards` independent HNSW graphs.
    pub fn new(num_shards: usize, distance: Distance, config: HnswConfig) -> Self {
        let n = num_shards.max(1);
        let shards = (0..n)
            .map(|i| {
                // Give each shard a distinct RNG seed for independent layering.
                let mut cfg = config.clone();
                cfg.seed = config.seed.wrapping_add(i as u64 * 0x9E37_79B9);
                RwLock::new(Hnsw::new(distance, cfg))
            })
            .collect();
        Self { shards }
    }

    fn shard_of(&self, id: &Uuid) -> usize {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        (h.finish() as usize) % self.shards.len()
    }

    /// Insert a vector; touches only the owning shard.
    pub fn insert(&self, id: Uuid, vector: Vec<f32>) {
        let s = self.shard_of(&id);
        self.shards[s].write().insert(id, vector);
    }

    /// Soft-delete a vector from its shard. Returns true if present.
    pub fn remove(&self, id: &Uuid) -> bool {
        let s = self.shard_of(id);
        self.shards[s].write().remove(id)
    }

    /// Total live vectors across all shards.
    pub fn len(&self) -> usize {
        self.shards.iter().map(|s| s.read().len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Fan out the query across all shards and merge into a global top-k by
    /// score (higher is better).
    pub fn search(&self, query: &[f32], k: usize) -> Vec<Neighbor> {
        let mut merged: Vec<Neighbor> = Vec::new();
        for shard in &self.shards {
            // Pull k from each shard so the global top-k is correct.
            merged.extend(shard.read().search(query, k));
        }
        merged.sort_by(|a, b| b.score.total_cmp(&a.score));
        merged.truncate(k);
        merged
    }

    /// Compact every shard, physically dropping tombstoned nodes.
    pub fn compact(&self) {
        for shard in &self.shards {
            shard.write().compact();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn distributes_and_finds_global_nearest() {
        let idx = ShardedHnsw::new(4, Distance::Cosine, HnswConfig::default());
        // Spread points around a circle so the nearest is unambiguous.
        for i in 0..200u128 {
            let a = i as f32 * 0.03;
            idx.insert(id(i), vec![a.cos(), a.sin()]);
        }
        assert_eq!(idx.len(), 200);
        assert_eq!(idx.shard_count(), 4);

        let q_angle = 50.0_f32 * 0.03;
        let res = idx.search(&[q_angle.cos(), q_angle.sin()], 1);
        // The exact-angle point (id 50) is the global nearest regardless of shard.
        assert_eq!(res[0].id, id(50));
    }

    #[test]
    fn delete_removes_from_correct_shard() {
        let idx = ShardedHnsw::new(4, Distance::Cosine, HnswConfig::default());
        idx.insert(id(1), vec![1.0, 0.0, 0.0]);
        idx.insert(id(2), vec![0.9, 0.1, 0.0]);
        assert_eq!(idx.len(), 2);
        assert!(idx.remove(&id(1)));
        assert_eq!(idx.len(), 1);
        let res = idx.search(&[1.0, 0.0, 0.0], 2);
        assert!(res.iter().all(|n| n.id != id(1)));
    }

    #[test]
    fn concurrent_inserts_across_shards() {
        use std::sync::Arc;
        use std::thread;
        let idx = Arc::new(ShardedHnsw::new(8, Distance::Cosine, HnswConfig::default()));
        let mut handles = Vec::new();
        for t in 0..4u128 {
            let idx = idx.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100u128 {
                    let n = t * 1000 + i;
                    idx.insert(id(n), vec![(n as f32).cos(), (n as f32).sin()]);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(idx.len(), 400);
    }
}
