//! End-to-end tests proving the storage layer is correctly wired from the
//! `MemVault` backends up through `MemoryManager`, including the `SearchQuery`
//! semantics that every backend previously ignored.

use gaussos::core::{MemCube, MemoryNamespace, MemoryPayload, Priority};
use gaussos::database::{InMemoryVault, MemVault, QualityRange, SearchQuery};
use gaussos::memory::manager::{MemoryManager, MemoryManagerConfig};
use std::sync::Arc;

fn text(ns: &str, content: &str, quality: f64) -> MemCube {
    let mut c = MemCube::new_with_namespace(
        MemoryPayload::Text(content.to_string()),
        MemoryNamespace(ns.to_string()),
    );
    c.metadata.quality_score = quality;
    c
}

#[tokio::test]
async fn in_memory_vault_full_crud_and_filters() {
    let vault = InMemoryVault::new();
    vault.store(&text("users/alice", "likes coffee", 0.9)).await.unwrap();
    vault.store(&text("users/alice", "low quality note", 0.1)).await.unwrap();
    vault.store(&text("users/bob", "likes tea", 0.9)).await.unwrap();

    // Namespace + quality filter must both be honoured.
    let mut q = SearchQuery::default();
    q.namespace = Some("users/alice".to_string());
    q.quality_range = Some(QualityRange { min: Some(0.5), max: None });
    let results = vault.search(&q).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_content_summary(), "likes coffee");
}

#[tokio::test]
async fn memory_manager_namespace_query_is_correct() {
    // MemoryManager::get_memories_by_namespace injects a `namespace_path`
    // custom filter that backends used to drop — verify it now works end to end.
    let vault: Arc<dyn MemVault> = Arc::new(InMemoryVault::new());
    let manager = MemoryManager::new_optimized(vault, MemoryManagerConfig::default());

    manager
        .create_memory_in_namespace(
            text("dummy", "alice-1", 0.8),
            &MemoryNamespace("users/alice".to_string()),
        )
        .await
        .unwrap();
    manager
        .create_memory_in_namespace(
            text("dummy", "bob-1", 0.8),
            &MemoryNamespace("users/bob".to_string()),
        )
        .await
        .unwrap();

    let alice = manager.get_memories_by_namespace("users/alice").await.unwrap();
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].namespace.0, "users/alice");
}

#[tokio::test]
async fn hybrid_search_through_manager_returns_ranked_memories() {
    let vault: Arc<dyn MemVault> = Arc::new(InMemoryVault::new());
    let manager = MemoryManager::new_optimized(vault, MemoryManagerConfig::default());

    for (i, content) in ["rust memory engine", "python notebook", "rust async runtime"]
        .iter()
        .enumerate()
    {
        let mut c = text("docs", content, 0.5 + i as f64 * 0.1);
        c.metadata.tags = vec!["doc".to_string()];
        manager.create_memory(c).await.unwrap();
    }

    let query = gaussos::memory::manager::HybridQuery {
        text: "rust".to_string(),
        embedding: None,
        namespace: None,
        tags: vec![],
        payload_type: None,
        min_quality: None,
        candidate_pool: 50,
        top_k: 5,
    };
    let ranked = manager.hybrid_search(&query).await.unwrap();
    assert!(!ranked.is_empty());
    // The "python" doc has no lexical overlap with "rust" and must not appear.
    assert!(ranked
        .iter()
        .all(|r| !r.memory.get_content_summary().contains("python")));
}

#[tokio::test]
async fn priority_archive_excluded_by_default() {
    let vault = InMemoryVault::new();
    let mut archived = text("a", "old archived", 0.9);
    archived.metadata.priority = Priority::Archive;
    vault.store(&archived).await.unwrap();
    vault.store(&text("a", "active", 0.9)).await.unwrap();

    let results = vault.search(&SearchQuery::default()).await.unwrap();
    // Archive-priority memories are excluded unless include_archived is set.
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_content_summary(), "active");
}
