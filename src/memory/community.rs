// src/memory/community.rs
//! Community detection over the entity graph (Phase 1, roadmap #5).
//!
//! GraphRAG-style sensemaking starts by clustering the knowledge graph into
//! communities and summarising each. GaussOS detects communities over the
//! entity graph induced by the bi-temporal fact store using **label
//! propagation** — a near-linear, dependency-free algorithm that needs no
//! parameter tuning and yields well-connected groups. Each community gets a
//! heuristic summary (extendable to an LLM-written summary feeding the L2/L3
//! hierarchy).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::memory::temporal::TemporalFactStore;

/// A detected community of related entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: usize,
    /// Entity labels in this community, most-connected first.
    pub members: Vec<String>,
    /// Facts whose endpoints are both inside this community.
    pub fact_count: usize,
    /// Heuristic, human-readable summary.
    pub summary: String,
}

/// Tunables for label propagation.
#[derive(Debug, Clone)]
pub struct CommunityConfig {
    pub max_iters: usize,
    /// Random-but-deterministic tie-break seed for stable output.
    pub seed: u64,
}

impl Default for CommunityConfig {
    fn default() -> Self {
        Self { max_iters: 20, seed: 0x6D5A }
    }
}

/// Detect communities over the live entity graph of `store`.
pub fn detect_communities(store: &TemporalFactStore, config: &CommunityConfig) -> Vec<Community> {
    // 1. Build the undirected entity graph from currently-valid facts.
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut labels_text: Vec<String> = Vec::new();
    let mut adj: Vec<Vec<usize>> = Vec::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();

    let mut intern = |s: &str, index: &mut HashMap<String, usize>, labels_text: &mut Vec<String>, adj: &mut Vec<Vec<usize>>| -> usize {
        if let Some(&i) = index.get(s) {
            i
        } else {
            let i = labels_text.len();
            index.insert(s.to_string(), i);
            labels_text.push(s.to_string());
            adj.push(Vec::new());
            i
        }
    };

    for f in store.current_facts() {
        let a = intern(&f.subject, &mut index, &mut labels_text, &mut adj);
        let b = intern(&f.object, &mut index, &mut labels_text, &mut adj);
        if a != b {
            adj[a].push(b);
            adj[b].push(a);
            edges.push((a, b));
        }
    }

    let n = labels_text.len();
    if n == 0 {
        return Vec::new();
    }

    // 2. Label propagation: each node starts in its own community, then
    //    repeatedly adopts the most common label among its neighbours.
    let mut label: Vec<usize> = (0..n).collect();
    // Deterministic visit order seeded for reproducibility.
    let mut order: Vec<usize> = (0..n).collect();
    let mut state = config.seed | 1;
    for i in (1..n).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state as usize) % (i + 1);
        order.swap(i, j);
    }

    for _ in 0..config.max_iters {
        let mut changed = false;
        for &node in &order {
            if adj[node].is_empty() {
                continue;
            }
            let mut counts: HashMap<usize, usize> = HashMap::new();
            for &nb in &adj[node] {
                *counts.entry(label[nb]).or_insert(0) += 1;
            }
            // Pick the most frequent neighbour label; tie-break on smaller id
            // for determinism.
            if let Some((&best, _)) = counts
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            {
                if label[node] != best {
                    label[node] = best;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // 3. Group nodes by final label.
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (node, &lbl) in label.iter().enumerate() {
        groups.entry(lbl).or_default().push(node);
    }

    // Degree for ordering members within a community.
    let degree: Vec<usize> = adj.iter().map(|a| a.len()).collect();

    let mut communities: Vec<Community> = groups
        .into_values()
        .map(|mut nodes| {
            nodes.sort_by(|a, b| degree[*b].cmp(&degree[*a]).then(a.cmp(b)));
            let members: Vec<String> = nodes.iter().map(|&i| labels_text[i].clone()).collect();
            let member_set: std::collections::HashSet<usize> = nodes.iter().copied().collect();
            let fact_count = edges
                .iter()
                .filter(|(a, b)| member_set.contains(a) && member_set.contains(b))
                .count();
            let summary = summarize(&members, fact_count);
            Community { id: 0, members, fact_count, summary }
        })
        .collect();

    // Stable, largest-first ordering, then assign ids.
    communities.sort_by(|a, b| b.members.len().cmp(&a.members.len()).then(a.members.cmp(&b.members)));
    for (i, c) in communities.iter_mut().enumerate() {
        c.id = i;
    }
    communities
}

fn summarize(members: &[String], fact_count: usize) -> String {
    let preview: Vec<&str> = members.iter().take(5).map(|s| s.as_str()).collect();
    if members.len() == 1 {
        format!("Isolated entity: {}", members[0])
    } else {
        format!(
            "Community of {} entities ({} internal facts), centred on: {}",
            members.len(),
            fact_count,
            preview.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::temporal::TemporalFact;

    #[test]
    fn separates_disconnected_components() {
        let mut store = TemporalFactStore::new();
        // Component 1: alice - bob - kalbe
        store.ingest(TemporalFact::new("alice", "knows", "bob"));
        store.ingest(TemporalFact::new("bob", "works_at", "kalbe"));
        // Component 2: carol - tea
        store.ingest(TemporalFact::new("carol", "likes", "tea"));

        let comms = detect_communities(&store, &CommunityConfig::default());
        assert!(comms.len() >= 2);
        // alice and kalbe should land in the same community...
        let find = |e: &str| comms.iter().position(|c| c.members.iter().any(|m| m == e));
        assert_eq!(find("alice"), find("kalbe"));
        // ...and carol in a different one from alice.
        assert_ne!(find("carol"), find("alice"));
    }

    #[test]
    fn empty_store_no_communities() {
        let store = TemporalFactStore::new();
        assert!(detect_communities(&store, &CommunityConfig::default()).is_empty());
    }

    #[test]
    fn summary_mentions_members() {
        let mut store = TemporalFactStore::new();
        store.ingest(TemporalFact::new("x", "rel", "y"));
        let comms = detect_communities(&store, &CommunityConfig::default());
        assert!(comms.iter().any(|c| c.summary.contains("entit")));
    }
}
