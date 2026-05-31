//! Integration tests for the research-driven advanced memory capabilities:
//! HNSW ANN index, vector quantization, Personalized-PageRank multi-hop graph
//! retrieval, and the Generative-Agents retrieval scorer.

use chrono::Utc;
use gaussos::memory::ann::{BinaryQuantized, Distance, Hnsw, HnswConfig, ScalarQuantized};
use gaussos::memory::graph_retrieval::{GraphRetriever, PprConfig};
use gaussos::memory::retrieval::RetrievalCandidate;
use gaussos::memory::scoring::{GaWeights, GenerativeAgentScorer};
use gaussos::memory::temporal::{TemporalFact, TemporalFactStore};
use uuid::Uuid;

#[test]
fn hnsw_recovers_nearest_neighbour() {
    let mut index = Hnsw::new(Distance::Cosine, HnswConfig::default());
    for i in 0..200u128 {
        let angle = i as f32 * 0.03;
        index.insert(Uuid::from_u128(i), vec![angle.cos(), angle.sin()]);
    }
    let q_angle = 50.0_f32 * 0.03;
    let res = index.search(&[q_angle.cos(), q_angle.sin()], 3);
    assert!(!res.is_empty());
    // The exact-angle vector (id 50) should be the top hit.
    assert_eq!(res[0].id, Uuid::from_u128(50));
}

#[test]
fn quantization_saves_memory_and_preserves_order() {
    let v = vec![0.9, 0.1, -0.4, 0.7, -0.8, 0.2];
    let sq = ScalarQuantized::encode(&v);
    assert_eq!(sq.byte_len(), v.len()); // int8 = 1 byte/dim vs 4 for f32

    let a = BinaryQuantized::encode(&v);
    let near = BinaryQuantized::encode(&[0.8, 0.2, -0.3, 0.6, -0.9, 0.1]); // same signs
    let far = BinaryQuantized::encode(&[-0.8, -0.2, 0.3, -0.6, 0.9, -0.1]); // opposite
    assert!(a.similarity(&near) > a.similarity(&far));
}

#[test]
fn personalized_pagerank_does_multi_hop() {
    let mut store = TemporalFactStore::new();
    store.ingest(TemporalFact::new("edwin", "met", "investor"));
    store.ingest(TemporalFact::new("investor", "recommended", "Tptn-stock"));
    store.ingest(TemporalFact::new("stranger", "owns", "boat")); // disconnected

    let retriever = GraphRetriever::new(PprConfig::default());
    let hits = retriever.search(&store, &["edwin".to_string()]);

    assert!(!hits.is_empty());
    // The two-hop recommendation should be retrieved and outrank the
    // disconnected stranger/boat fact.
    let rec = hits.iter().position(|h| h.fact.object == "Tptn-stock");
    let boat = hits.iter().position(|h| h.fact.object == "boat");
    assert!(rec.is_some(), "two-hop fact should be retrieved");
    if let (Some(r), Some(b)) = (rec, boat) {
        assert!(r < b);
    }
}

#[test]
fn generative_agent_scorer_blends_signals() {
    let now = Utc::now();
    let make = |seed: u128, importance: f32, age_h: i64, emb: Vec<f32>| RetrievalCandidate {
        id: Uuid::from_u128(seed),
        text: String::new(),
        embedding: Some(emb),
        created_at: now,
        last_accessed: now - chrono::Duration::hours(age_h),
        access_count: 0,
        importance,
    };
    let candidates = vec![
        make(1, 0.9, 0, vec![1.0, 0.0]),   // important, fresh, relevant
        make(2, 0.1, 1000, vec![0.0, 1.0]), // unimportant, stale, irrelevant
    ];
    let scorer = GenerativeAgentScorer::new(GaWeights::default());
    let ranked = scorer.rank(&candidates, Some(&[1.0, 0.0]), now);
    assert_eq!(ranked[0].id, Uuid::from_u128(1));
}
