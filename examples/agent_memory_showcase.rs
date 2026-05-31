//! GaussOS agent-memory showcase.
//!
//! Demonstrates the four pillars that make GaussOS a complete agent-memory
//! engine: hybrid retrieval, bi-temporal knowledge, the forgetting curve, and
//! the hierarchical L0→L3 pyramid.
//!
//! Run with: `cargo run --example agent_memory_showcase`

use chrono::Utc;
use gaussos::core::{MemCube, MemoryPayload, Message, MessageRole, Priority};
use gaussos::memory::decay::ForgettingCurve;
use gaussos::memory::hierarchy::{HierarchyBuilder, MemoryLayer};
use gaussos::memory::retrieval::{HybridRetriever, HybridSearchConfig, RetrievalCandidate};
use gaussos::memory::temporal::{TemporalFact, TemporalFactStore};
use uuid::Uuid;

fn main() {
    println!("=== GaussOS Agent Memory Showcase ===\n");

    hybrid_retrieval_demo();
    bitemporal_demo();
    forgetting_demo();
    hierarchy_demo();
}

fn hybrid_retrieval_demo() {
    println!("[1] Hybrid Retrieval (BM25 + vector + RRF + MMR)");
    let docs = [
        ("rust async memory management", vec![1.0, 0.0, 0.0]),
        ("python pandas data analysis", vec![0.0, 1.0, 0.0]),
        ("memory consolidation algorithms in rust", vec![0.9, 0.1, 0.0]),
        ("javascript front-end frameworks", vec![0.0, 0.0, 1.0]),
    ];
    let candidates: Vec<RetrievalCandidate> = docs
        .iter()
        .enumerate()
        .map(|(i, (text, emb))| RetrievalCandidate {
            id: Uuid::from_u128(i as u128 + 1),
            text: text.to_string(),
            embedding: Some(emb.clone()),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            importance: 0.5,
        })
        .collect();

    let retriever = HybridRetriever::new(candidates, HybridSearchConfig::default());
    let results = retriever.search("rust memory", Some(&[1.0, 0.0, 0.0]));
    for (rank, r) in results.iter().enumerate() {
        println!(
            "    #{rank} score={:.4} bm25_rank={} vec_rank={} (id={})",
            r.score, r.bm25_rank, r.vector_rank, r.id
        );
    }
    println!();
}

fn bitemporal_demo() {
    println!("[2] Bi-temporal knowledge (fact invalidation, not deletion)");
    let mut store = TemporalFactStore::new();
    store.ingest(TemporalFact::new("user:edwin", "employer", "OldCorp"));
    let report = store.ingest(TemporalFact::new("user:edwin", "employer", "Kalbe"));
    println!("    superseded {} prior fact(s)", report.superseded.len());

    println!("    current belief:");
    for f in store.current_value("user:edwin", "employer") {
        println!("      {} {} {}", f.subject, f.predicate, f.object);
    }
    println!("    full history (audit trail):");
    for f in store.history("user:edwin", "employer") {
        println!(
            "      {} (live={}, valid_until={:?})",
            f.object,
            f.is_live(),
            f.invalid_at
        );
    }
    println!();
}

fn forgetting_demo() {
    println!("[3] Forgetting curve & salience-based retention");
    let fc = ForgettingCurve::default();

    let mut hot = MemCube::new(MemoryPayload::Text("frequently used preference".into()));
    hot.metadata.access_count = 80;
    hot.metadata.quality_score = 0.9;
    hot.metadata.priority = Priority::High;

    let mut cold = MemCube::new(MemoryPayload::Text("stale trivia".into()));
    cold.metadata.last_accessed = Utc::now() - chrono::Duration::days(400);
    cold.metadata.priority = Priority::Low;

    for (label, cube) in [("hot", &hot), ("cold", &cold)] {
        let r = fc.retention(cube, Utc::now());
        println!(
            "    {label:>4}: score={:.3} recency={:.3} -> {:?}",
            r.score, r.recency, r.action
        );
    }
    println!();
}

fn hierarchy_demo() {
    println!("[4] Hierarchical L0->L3 memory pyramid (progressive disclosure)");
    let messages = vec![
        Message {
            role: MessageRole::User,
            content: "I'm a data engineer at Kalbe. I prefer dark mode and concise answers."
                .into(),
            timestamp: Utc::now(),
            metadata: None,
        },
        Message {
            role: MessageRole::Assistant,
            content: "Got it. I'll keep things concise.".into(),
            timestamp: Utc::now(),
            metadata: None,
        },
    ];
    let h = HierarchyBuilder::default().build(&messages);
    println!(
        "    layers: raw={} atomic={} scenario={}",
        h.layer(MemoryLayer::Raw).len(),
        h.layer(MemoryLayer::Atomic).len(),
        h.layer(MemoryLayer::Scenario).len(),
    );
    if let Some(top) = h.top_context().first() {
        println!("    top context node: \"{}\"", top.content);
        let evidence = h.evidence(&top.id);
        println!("    drill-down to {} raw evidence node(s)", evidence.len());
    }
    println!();
}
