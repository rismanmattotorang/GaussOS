//! Integration tests for GaussOS's advanced agent-memory capabilities:
//! hybrid retrieval (RRF), bi-temporal facts, forgetting curve, and the
//! hierarchical L0→L3 memory pyramid.

use chrono::Utc;
use gaussos::core::{MemCube, MemoryPayload, Message, MessageRole, Priority};
use gaussos::memory::decay::{ForgettingCurve, RetentionAction};
use gaussos::memory::hierarchy::{HierarchyBuilder, MemoryLayer};
use gaussos::memory::retrieval::{HybridRetriever, HybridSearchConfig, RetrievalCandidate};
use gaussos::memory::temporal::{TemporalFact, TemporalFactStore};
use uuid::Uuid;

fn candidate(seed: u128, text: &str, emb: Option<Vec<f32>>) -> RetrievalCandidate {
    RetrievalCandidate {
        id: Uuid::from_u128(seed),
        text: text.to_string(),
        embedding: emb,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
        importance: 0.5,
    }
}

#[test]
fn hybrid_retrieval_combines_lexical_and_semantic() {
    let candidates = vec![
        candidate(1, "rust async memory management system", Some(vec![1.0, 0.0, 0.0])),
        candidate(2, "python data science notebook", Some(vec![0.0, 1.0, 0.0])),
        candidate(3, "memory consolidation in rust", Some(vec![0.9, 0.1, 0.0])),
    ];
    let retriever = HybridRetriever::new(candidates, HybridSearchConfig::default());
    let results = retriever.search("rust memory", Some(&[1.0, 0.0, 0.0]));

    assert!(!results.is_empty());
    // The python doc should not be the top result.
    assert_ne!(results[0].id, Uuid::from_u128(2));
    // Top result should have contributions from at least one ranked list.
    assert!(results[0].bm25_rank > 0 || results[0].vector_rank > 0);
}

#[test]
fn bitemporal_store_tracks_corrections() {
    let mut store = TemporalFactStore::new();
    store.ingest(TemporalFact::new("user:edwin", "role", "Engineer"));
    let promotion = store.ingest(TemporalFact::new("user:edwin", "role", "Director"));

    // The new fact supersedes the old one.
    assert_eq!(promotion.superseded.len(), 1);

    // Current belief reflects the latest value.
    let current = store.current_value("user:edwin", "role");
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].object, "Director");

    // History is preserved (nothing deleted).
    assert_eq!(store.history("user:edwin", "role").len(), 2);
}

#[test]
fn forgetting_curve_classifies_memories() {
    let fc = ForgettingCurve::default();

    let mut hot = MemCube::new(MemoryPayload::Text("important".into()));
    hot.metadata.access_count = 100;
    hot.metadata.quality_score = 0.95;
    hot.metadata.priority = Priority::High;
    hot.metadata.last_accessed = Utc::now();

    let mut stale = MemCube::new(MemoryPayload::Text("forgotten".into()));
    stale.metadata.access_count = 0;
    stale.metadata.quality_score = 0.0;
    stale.metadata.priority = Priority::Low;
    stale.metadata.last_accessed = Utc::now() - chrono::Duration::days(3650);

    assert_eq!(fc.retention(&hot, Utc::now()).action, RetentionAction::Retain);
    assert_ne!(
        fc.retention(&stale, Utc::now()).action,
        RetentionAction::Retain
    );
}

#[test]
fn hierarchy_supports_progressive_disclosure_and_drilldown() {
    let messages = vec![
        Message {
            role: MessageRole::User,
            content: "I work at Kalbe Corp as a senior data engineer. I prefer concise answers."
                .into(),
            timestamp: Utc::now(),
            metadata: None,
        },
        Message {
            role: MessageRole::Assistant,
            content: "Understood, I will keep responses concise.".into(),
            timestamp: Utc::now(),
            metadata: None,
        },
    ];

    let hierarchy = HierarchyBuilder::default().build(&messages);

    assert!(!hierarchy.layer(MemoryLayer::Raw).is_empty());
    assert!(!hierarchy.layer(MemoryLayer::Atomic).is_empty());

    // Top context is the highest populated layer (scenario here).
    let top = hierarchy.top_context();
    assert!(!top.is_empty());

    // Every atomic fact can be traced back to raw evidence.
    for fact in hierarchy.layer(MemoryLayer::Atomic) {
        let evidence = hierarchy.evidence(&fact.id);
        assert!(!evidence.is_empty(), "atomic fact must have raw evidence");
    }
}
