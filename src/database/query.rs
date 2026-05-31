// src/database/query.rs
//! Canonical `SearchQuery` evaluation.
//!
//! Every storage backend used to interpret `SearchQuery` differently — and most
//! ignored the majority of its fields (quality range, date range, tag logic,
//! vector search, the custom `filters` map, sort, pagination). That made search
//! results backend-dependent and, in several cases, simply wrong (e.g. a
//! `quality_range` filter returned low-quality memories anyway, and the
//! `namespace_path` filter that `MemoryManager` itself injects was silently
//! dropped).
//!
//! This module defines the semantics **once**. A backend does whatever coarse
//! fetching it can do efficiently (by namespace/tag/text), then hands the rows
//! to [`apply_search_query`], which guarantees correct, uniform filtering,
//! vector ranking, sorting, and pagination across every backend.

use crate::core::{MemCube, MemoryPayload, Priority};
use crate::database::{SearchQuery, SimilarityMetric, SortDirection, TagLogic};

/// Stable type name for a payload, used by `payload_type` / `memory_type` filters.
pub fn payload_type_name(payload: &MemoryPayload) -> &'static str {
    match payload {
        MemoryPayload::Parametric { .. } => "parametric",
        MemoryPayload::Activation { .. } => "activation",
        MemoryPayload::Text(_) => "text",
        MemoryPayload::Plaintext { .. } => "plaintext",
        MemoryPayload::Semantic { .. } => "semantic",
        MemoryPayload::Episodic { .. } => "episodic",
        MemoryPayload::Procedural { .. } => "procedural",
    }
}

/// Lower-cased priority label for the `priority` filter.
fn priority_name(p: &Priority) -> &'static str {
    match p {
        Priority::Critical => "critical",
        Priority::High => "high",
        Priority::Medium => "medium",
        Priority::Normal => "normal",
        Priority::Low => "low",
        Priority::Archive => "archive",
    }
}

/// Does `cube`'s namespace match the query namespace (optionally including
/// hierarchical children, e.g. query `users/alice` matches `users/alice/work`)?
fn namespace_matches(cube: &MemCube, ns: &str, include_children: bool) -> bool {
    let cube_ns = &cube.namespace.0;
    if cube_ns == ns {
        return true;
    }
    include_children && cube_ns.starts_with(&format!("{ns}/"))
}

/// Free-text match: case-insensitive substring over the content summary, name,
/// description and tags.
fn text_matches(cube: &MemCube, needle: &str) -> bool {
    let n = needle.to_lowercase();
    if cube.get_content_summary().to_lowercase().contains(&n) {
        return true;
    }
    if let Some(name) = &cube.metadata.name {
        if name.to_lowercase().contains(&n) {
            return true;
        }
    }
    if let Some(desc) = &cube.metadata.description {
        if desc.to_lowercase().contains(&n) {
            return true;
        }
    }
    cube.metadata.tags.iter().any(|t| t.to_lowercase().contains(&n))
}

fn tags_match(cube: &MemCube, tags: &[String], logic: &TagLogic) -> bool {
    if tags.is_empty() {
        return true;
    }
    let has = |t: &String| cube.metadata.tags.contains(t);
    match logic {
        TagLogic::And => tags.iter().all(has),
        TagLogic::Or => tags.iter().any(has),
        TagLogic::Not => !tags.iter().any(has),
    }
}

/// Apply a custom `filters` entry. Recognises the well-known `namespace_path`
/// key (used internally by `MemoryManager`); every other key is matched against
/// the memory's `custom_attributes` by JSON equality.
fn custom_filter_matches(cube: &MemCube, key: &str, value: &serde_json::Value) -> bool {
    match key {
        "namespace_path" => value.as_str().map(|s| cube.namespace.0 == s).unwrap_or(false),
        other => cube
            .metadata
            .custom_attributes
            .get(other)
            .map(|v| v == value)
            .unwrap_or(false),
    }
}

fn similarity(metric: &SimilarityMetric, a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    match metric {
        SimilarityMetric::Cosine | SimilarityMetric::DotProduct => {
            let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
            if matches!(metric, SimilarityMetric::DotProduct) {
                return dot;
            }
            let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
            if na == 0.0 || nb == 0.0 {
                0.0
            } else {
                dot / (na * nb)
            }
        }
        SimilarityMetric::Euclidean => {
            let d: f32 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f32>().sqrt();
            1.0 / (1.0 + d)
        }
        SimilarityMetric::Manhattan => {
            let d: f32 = a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum();
            1.0 / (1.0 + d)
        }
        SimilarityMetric::Hamming => {
            let d = a.iter().zip(b).filter(|(x, y)| (*x - *y).abs() > f32::EPSILON).count();
            1.0 - d as f32 / a.len() as f32
        }
        SimilarityMetric::Jaccard => {
            // Treat non-zero dims as set membership.
            let mut inter = 0usize;
            let mut union = 0usize;
            for (x, y) in a.iter().zip(b) {
                let xn = x.abs() > f32::EPSILON;
                let yn = y.abs() > f32::EPSILON;
                if xn && yn {
                    inter += 1;
                }
                if xn || yn {
                    union += 1;
                }
            }
            if union == 0 {
                0.0
            } else {
                inter as f32 / union as f32
            }
        }
    }
}

/// Does this memory pass every scalar predicate in the query?
fn passes_filters(cube: &MemCube, query: &SearchQuery) -> bool {
    // Archived handling: by default exclude archive-priority memories.
    if !query.include_archived && matches!(cube.metadata.priority, Priority::Archive) {
        return false;
    }
    if let Some(ns) = &query.namespace {
        if !namespace_matches(cube, ns, query.include_child_namespaces) {
            return false;
        }
    }
    if let Some(text) = &query.text {
        if !text.is_empty() && !text_matches(cube, text) {
            return false;
        }
    }
    if !tags_match(cube, &query.tags, &query.tag_logic) {
        return false;
    }
    if let Some(pt) = &query.payload_type {
        if !payload_type_name(&cube.payload).eq_ignore_ascii_case(pt) {
            return false;
        }
    }
    if let Some(mt) = &query.memory_type {
        if !payload_type_name(&cube.payload).eq_ignore_ascii_case(mt) {
            return false;
        }
    }
    if let Some(pri) = &query.priority {
        if !priority_name(&cube.metadata.priority).eq_ignore_ascii_case(pri) {
            return false;
        }
    }
    if let Some(qr) = &query.quality_range {
        if let Some(min) = qr.min {
            if cube.metadata.quality_score < min {
                return false;
            }
        }
        if let Some(max) = qr.max {
            if cube.metadata.quality_score > max {
                return false;
            }
        }
    }
    if let Some(dr) = &query.date_range {
        if let Some(start) = dr.start {
            if cube.created_at < start {
                return false;
            }
        }
        if let Some(end) = dr.end {
            if cube.created_at > end {
                return false;
            }
        }
    }
    for (k, v) in &query.filters {
        if !custom_filter_matches(cube, k, v) {
            return false;
        }
    }
    true
}

/// Sort `cubes` in place according to `query.sort` (no-op when a vector search
/// already imposed a similarity ordering).
fn apply_sort(cubes: &mut [MemCube], query: &SearchQuery) {
    let Some(sort) = &query.sort else { return };
    let asc = matches!(sort.direction, SortDirection::Asc);
    cubes.sort_by(|a, b| {
        let ord = match sort.field.as_str() {
            "quality_score" => a
                .metadata
                .quality_score
                .total_cmp(&b.metadata.quality_score),
            "access_count" => a.metadata.access_count.cmp(&b.metadata.access_count),
            "last_accessed" => a.metadata.last_accessed.cmp(&b.metadata.last_accessed),
            "updated_at" => a.updated_at.cmp(&b.updated_at),
            // Default and explicit "created_at".
            _ => a.created_at.cmp(&b.created_at),
        };
        if asc {
            ord
        } else {
            ord.reverse()
        }
    });
}

/// Evaluate a [`SearchQuery`] over an arbitrary set of candidate memories,
/// returning the correctly filtered, ranked, and paginated result.
///
/// This is the single source of truth for query semantics; all backends route
/// their final result through it.
pub fn apply_search_query(candidates: Vec<MemCube>, query: &SearchQuery) -> Vec<MemCube> {
    // 1. Scalar predicates.
    let mut filtered: Vec<MemCube> = candidates
        .into_iter()
        .filter(|c| passes_filters(c, query))
        .collect();

    // 2. Vector search (when present) replaces the sort with similarity order
    //    and applies the similarity threshold + top_k.
    if let Some(vs) = &query.vector_search {
        let mut scored: Vec<(f32, MemCube)> = filtered
            .into_iter()
            .filter_map(|c| {
                // Compute the score first so the embedding borrow ends before `c` moves.
                let score = c
                    .payload_embedding()
                    .map(|e| similarity(&vs.metric, &vs.embedding, e));
                score.map(|s| (s, c))
            })
            .filter(|(s, _)| *s as f64 >= vs.similarity_threshold)
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        if let Some(k) = vs.top_k {
            scored.truncate(k);
        }
        filtered = scored.into_iter().map(|(_, c)| c).collect();
    } else {
        apply_sort(&mut filtered, query);
    }

    // 3. Pagination.
    let offset = query.offset.unwrap_or(0) as usize;
    let limit = query.limit.unwrap_or(u64::MAX) as usize;
    filtered.into_iter().skip(offset).take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{MemCube, MemoryNamespace, MemoryPayload};
    use crate::database::{QualityRange, VectorSearchQuery};

    fn text_cube(ns: &str, content: &str, quality: f64, tags: &[&str]) -> MemCube {
        let mut c = MemCube::new_with_namespace(
            MemoryPayload::Text(content.to_string()),
            MemoryNamespace(ns.to_string()),
        );
        c.metadata.quality_score = quality;
        c.metadata.tags = tags.iter().map(|s| s.to_string()).collect();
        c
    }

    #[test]
    fn quality_range_is_enforced() {
        let cubes = vec![
            text_cube("a", "low", 0.1, &[]),
            text_cube("a", "high", 0.9, &[]),
        ];
        let mut q = SearchQuery::default();
        q.quality_range = Some(QualityRange { min: Some(0.8), max: None });
        let out = apply_search_query(cubes, &q);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].get_content_summary(), "high");
    }

    #[test]
    fn namespace_path_custom_filter_works() {
        let cubes = vec![text_cube("users/alice", "x", 0.5, &[]), text_cube("users/bob", "y", 0.5, &[])];
        let mut q = SearchQuery::default();
        q.filters.insert(
            "namespace_path".to_string(),
            serde_json::Value::String("users/alice".to_string()),
        );
        let out = apply_search_query(cubes, &q);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].namespace.0, "users/alice");
    }

    #[test]
    fn include_child_namespaces() {
        let cubes = vec![
            text_cube("users/alice", "x", 0.5, &[]),
            text_cube("users/alice/work", "y", 0.5, &[]),
            text_cube("users/bob", "z", 0.5, &[]),
        ];
        let mut q = SearchQuery::default();
        q.namespace = Some("users/alice".to_string());
        q.include_child_namespaces = true;
        let out = apply_search_query(cubes, &q);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn tag_logic_or_and_not() {
        let cubes = vec![
            text_cube("a", "1", 0.5, &["x"]),
            text_cube("a", "2", 0.5, &["y"]),
            text_cube("a", "3", 0.5, &["x", "y"]),
        ];
        let mut q = SearchQuery::default();
        q.tags = vec!["x".into(), "y".into()];
        q.tag_logic = TagLogic::Or;
        assert_eq!(apply_search_query(cubes.clone(), &q).len(), 3);
        q.tag_logic = TagLogic::And;
        assert_eq!(apply_search_query(cubes.clone(), &q).len(), 1);
        q.tag_logic = TagLogic::Not;
        assert_eq!(apply_search_query(cubes, &q).len(), 0);
    }

    #[test]
    fn vector_search_ranks_and_thresholds() {
        let mut a = text_cube("a", "a", 0.5, &[]);
        let mut b = text_cube("a", "b", 0.5, &[]);
        a.payload = MemoryPayload::Plaintext {
            content: "a".into(),
            encoding: "utf8".into(),
            language: None,
            embeddings: Some(vec![1.0, 0.0]),
        };
        b.payload = MemoryPayload::Plaintext {
            content: "b".into(),
            encoding: "utf8".into(),
            language: None,
            embeddings: Some(vec![0.0, 1.0]),
        };
        let mut q = SearchQuery::default();
        q.vector_search = Some(VectorSearchQuery {
            embedding: vec![1.0, 0.0],
            similarity_threshold: 0.5,
            metric: SimilarityMetric::Cosine,
            top_k: Some(10),
            ef_search: None,
        });
        let out = apply_search_query(vec![a, b], &q);
        // Only `a` clears the 0.5 cosine threshold against [1,0].
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].get_content_summary(), "a");
    }

    #[test]
    fn pagination_and_sort() {
        let cubes: Vec<MemCube> = (0..5).map(|i| text_cube("a", &i.to_string(), i as f64 / 10.0, &[])).collect();
        let mut q = SearchQuery::default();
        q.sort = Some(crate::database::SortOptions {
            field: "quality_score".into(),
            direction: SortDirection::Desc,
            nulls: crate::database::NullsOrder::Last,
        });
        q.offset = Some(1);
        q.limit = Some(2);
        let out = apply_search_query(cubes, &q);
        assert_eq!(out.len(), 2);
        // Highest quality is 0.4 ("4"); after skipping it, expect "3" then "2".
        assert_eq!(out[0].get_content_summary(), "3");
        assert_eq!(out[1].get_content_summary(), "2");
    }
}
