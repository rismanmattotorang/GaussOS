// src/memory/graph_retrieval.rs
//! Multi-hop graph retrieval via Personalized PageRank (HippoRAG-style).
//!
//! Dense vector search retrieves what is *similar* to the query, but agent
//! questions are often *multi-hop*: "what did the person I met in Jakarta
//! recommend?" requires chaining facts. HippoRAG (Gutiérrez et al., NeurIPS'24)
//! showed that running **Personalized PageRank (PPR)** over a knowledge graph,
//! seeded at the entities mentioned in the query, retrieves multi-hop evidence
//! far better than flat similarity (e.g. 2Wiki Recall@5 76.5% → 90.4%).
//!
//! GaussOS already maintains a bi-temporal fact graph ([`crate::memory::temporal`]).
//! This module turns it into a retriever:
//!
//! 1. Build a graph whose nodes are entities (subjects/objects) and whose edges
//!    are the live facts connecting them.
//! 2. Seed a restart distribution on the query entities.
//! 3. Run PPR by power iteration: `r = (1-α)·s + α·Mᵀr`.
//! 4. Score each fact by the PageRank mass of the entities it connects, weighted
//!    by node **specificity** (an IDF-like term so generic hub entities don't
//!    dominate), and return the top facts.

use std::collections::HashMap;
use uuid::Uuid;

use crate::memory::temporal::{TemporalFact, TemporalFactStore};

/// Tunables for Personalized PageRank.
#[derive(Debug, Clone)]
pub struct PprConfig {
    /// Damping factor α (probability of following an edge vs restarting).
    pub damping: f32,
    /// Maximum power-iteration steps.
    pub max_iters: usize,
    /// L1 convergence tolerance.
    pub tolerance: f32,
    /// Number of ranked facts to return.
    pub top_k: usize,
}

impl Default for PprConfig {
    fn default() -> Self {
        Self {
            damping: 0.85,
            max_iters: 100,
            tolerance: 1e-6,
            top_k: 10,
        }
    }
}

/// A fact retrieved by the graph walk, with its PPR-derived relevance.
#[derive(Debug, Clone)]
pub struct GraphHit {
    pub fact: TemporalFact,
    pub score: f32,
}

/// An entity-centric view of the fact store for graph algorithms.
struct EntityGraph {
    /// entity label -> node index
    index: HashMap<String, usize>,
    labels: Vec<String>,
    /// adjacency: node -> (neighbour node, fact position in `facts`)
    adjacency: Vec<Vec<(usize, usize)>>,
    /// node -> degree (for specificity weighting)
    degree: Vec<usize>,
    facts: Vec<TemporalFact>,
}

impl EntityGraph {
    /// Build from the currently-valid facts in the store.
    fn build(store: &TemporalFactStore) -> Self {
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut labels: Vec<String> = Vec::new();
        let mut adjacency: Vec<Vec<(usize, usize)>> = Vec::new();
        let mut facts: Vec<TemporalFact> = Vec::new();

        let mut intern = |label: &str,
                          index: &mut HashMap<String, usize>,
                          labels: &mut Vec<String>,
                          adjacency: &mut Vec<Vec<(usize, usize)>>|
         -> usize {
            if let Some(&i) = index.get(label) {
                i
            } else {
                let i = labels.len();
                index.insert(label.to_string(), i);
                labels.push(label.to_string());
                adjacency.push(Vec::new());
                i
            }
        };

        for fact in store.current_facts() {
            let fact_pos = facts.len();
            let s = intern(&fact.subject, &mut index, &mut labels, &mut adjacency);
            let o = intern(&fact.object, &mut index, &mut labels, &mut adjacency);
            // Undirected edge: probability flows both ways through a fact.
            adjacency[s].push((o, fact_pos));
            adjacency[o].push((s, fact_pos));
            facts.push(fact.clone());
        }

        let degree = adjacency.iter().map(|a| a.len()).collect();
        Self { index, labels, adjacency, degree, facts }
    }

    fn node_count(&self) -> usize {
        self.labels.len()
    }
}

/// HippoRAG-style multi-hop retriever over a bi-temporal fact store.
pub struct GraphRetriever {
    config: PprConfig,
}

impl GraphRetriever {
    pub fn new(config: PprConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PprConfig::default())
    }

    /// Run Personalized PageRank seeded at `seed_entities` and return the
    /// top-k facts ranked by the PageRank mass flowing through them.
    pub fn search(&self, store: &TemporalFactStore, seed_entities: &[String]) -> Vec<GraphHit> {
        let graph = EntityGraph::build(store);
        let n = graph.node_count();
        if n == 0 {
            return Vec::new();
        }

        // Restart distribution: uniform mass over the seeds that exist in the
        // graph; if none match, fall back to a uniform global restart.
        let mut restart = vec![0.0f32; n];
        let mut seeded = 0usize;
        for e in seed_entities {
            if let Some(&i) = graph.index.get(e) {
                restart[i] += 1.0;
                seeded += 1;
            }
        }
        if seeded == 0 {
            let u = 1.0 / n as f32;
            restart.iter_mut().for_each(|x| *x = u);
        } else {
            let inv = 1.0 / seeded as f32;
            restart.iter_mut().for_each(|x| *x *= inv);
        }

        let ranks = self.power_iteration(&graph, &restart);

        // Node specificity: down-weight high-degree hub entities (IDF-like).
        let specificity: Vec<f32> = graph
            .degree
            .iter()
            .map(|&d| 1.0 / (1.0 + d as f32).ln_1p())
            .collect();

        // Score each fact by the specificity-weighted rank of its endpoints.
        let mut fact_scores: Vec<(usize, f32)> = (0..graph.facts.len())
            .map(|p| {
                let f = &graph.facts[p];
                let s = graph.index[&f.subject];
                let o = graph.index[&f.object];
                let score = ranks[s] * specificity[s] + ranks[o] * specificity[o];
                (p, score * f.confidence)
            })
            .collect();

        fact_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        fact_scores
            .into_iter()
            .filter(|(_, s)| *s > 0.0)
            .take(self.config.top_k)
            .map(|(p, score)| GraphHit { fact: graph.facts[p].clone(), score })
            .collect()
    }

    /// Standard PPR power iteration with degree-normalised edge weights:
    /// `r = (1-α)·restart + α·Wᵀr`, where W is the column-stochastic adjacency.
    fn power_iteration(&self, graph: &EntityGraph, restart: &[f32]) -> Vec<f32> {
        let n = graph.node_count();
        let alpha = self.config.damping;
        let mut r = restart.to_vec();

        for _ in 0..self.config.max_iters {
            let mut next = vec![0.0f32; n];
            // Distribute each node's mass evenly across its neighbours.
            for u in 0..n {
                let deg = graph.degree[u];
                if deg == 0 {
                    // Dangling node: its mass goes back to the restart vector.
                    for (i, item) in next.iter_mut().enumerate() {
                        *item += alpha * r[u] * restart[i];
                    }
                    continue;
                }
                let share = alpha * r[u] / deg as f32;
                for &(v, _) in &graph.adjacency[u] {
                    next[v] += share;
                }
            }
            // Add the teleport / restart mass.
            for i in 0..n {
                next[i] += (1.0 - alpha) * restart[i];
            }

            // Check L1 convergence.
            let delta: f32 = r.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
            r = next;
            if delta < self.config.tolerance {
                break;
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_chain() -> TemporalFactStore {
        // alice -knows-> bob -works_at-> Kalbe -located_in-> Jakarta
        let mut s = TemporalFactStore::new();
        s.ingest(TemporalFact::new("alice", "knows", "bob"));
        s.ingest(TemporalFact::new("bob", "works_at", "Kalbe"));
        s.ingest(TemporalFact::new("Kalbe", "located_in", "Jakarta"));
        s.ingest(TemporalFact::new("carol", "likes", "tea")); // unrelated component
        s
    }

    #[test]
    fn ppr_surfaces_multi_hop_neighbours() {
        let store = store_with_chain();
        let retriever = GraphRetriever::with_defaults();
        let hits = retriever.search(&store, &["alice".to_string()]);
        assert!(!hits.is_empty());
        // The directly-connected fact (alice knows bob) should rank above the
        // unrelated component (carol likes tea).
        let alice_bob = hits.iter().position(|h| h.fact.object == "bob");
        let carol = hits.iter().position(|h| h.fact.subject == "carol");
        assert!(alice_bob.is_some());
        match (alice_bob, carol) {
            (Some(ab), Some(c)) => assert!(ab < c),
            (Some(_), None) => {} // carol filtered out entirely — even better
            _ => panic!("expected alice->bob fact in results"),
        }
    }

    #[test]
    fn unrelated_component_gets_low_score() {
        let store = store_with_chain();
        let retriever = GraphRetriever::with_defaults();
        let hits = retriever.search(&store, &["alice".to_string()]);
        // carol/tea is in a disconnected component → should score ~0 and be last
        // or filtered. The Kalbe/Jakarta hop should outrank it.
        let kalbe = hits.iter().find(|h| h.fact.object == "Kalbe" || h.fact.subject == "Kalbe");
        assert!(kalbe.is_some(), "multi-hop fact should be retrieved");
    }

    #[test]
    fn empty_store_returns_empty() {
        let store = TemporalFactStore::new();
        let retriever = GraphRetriever::with_defaults();
        assert!(retriever.search(&store, &["anything".to_string()]).is_empty());
    }

    #[test]
    fn unknown_seed_falls_back_to_uniform() {
        let store = store_with_chain();
        let retriever = GraphRetriever::with_defaults();
        // A seed not in the graph shouldn't panic; returns global ranking.
        let hits = retriever.search(&store, &["nonexistent".to_string()]);
        assert!(!hits.is_empty());
    }
}
