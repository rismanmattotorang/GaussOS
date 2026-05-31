// src/memory/retrieval.rs
//! Hybrid retrieval engine for GaussOS.
//!
//! This module implements a state-of-the-art retrieval pipeline that fuses
//! lexical (BM25) and semantic (dense vector) signals, then re-ranks the
//! candidates for diversity and recency. The design is informed by the best
//! ideas from contemporary agent-memory systems:
//!
//! * **Reciprocal Rank Fusion (RRF)** of BM25 + vector rankings — the hybrid
//!   retrieval strategy popularised by Tencent's TencentDB-Agent-Memory, which
//!   balances keyword recall with semantic recall.
//! * **Maximal Marginal Relevance (MMR)** re-ranking — reduces redundancy in
//!   the returned set so an agent receives diverse, non-overlapping context.
//! * **Temporal decay & salience boosting** — borrowing from cognitive memory
//!   models (and Letta/Zep's emphasis on recency), more recent and more
//!   important memories surface first when relevance ties.
//!
//! The engine is intentionally storage-agnostic: it operates on lightweight
//! [`RetrievalCandidate`] values so it can rank results coming from any
//! `MemVault` backend, an in-memory cache, or a knowledge-graph traversal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::core::MemCube;

/// A single retrievable unit handed to the [`HybridRetriever`].
///
/// Candidates are cheap to construct from a [`MemCube`] via
/// [`RetrievalCandidate::from_memcube`], but can also be built manually for
/// graph nodes, summaries, or any other addressable memory artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    pub id: Uuid,
    /// Lexical content used for BM25 scoring.
    pub text: String,
    /// Optional dense embedding used for cosine similarity.
    pub embedding: Option<Vec<f32>>,
    /// When the memory was created (used for recency decay).
    pub created_at: DateTime<Utc>,
    /// When the memory was last accessed (reinforcement signal).
    pub last_accessed: DateTime<Utc>,
    /// Number of times the memory has been accessed.
    pub access_count: u64,
    /// Intrinsic importance / quality in `[0.0, 1.0]`.
    pub importance: f32,
}

impl RetrievalCandidate {
    /// Build a candidate from a [`MemCube`], pulling text, embedding and the
    /// usage statistics needed for recency/salience scoring.
    pub fn from_memcube(cube: &MemCube) -> Self {
        Self {
            id: cube.id,
            text: cube.get_content_summary(),
            embedding: cube.payload_embedding().cloned(),
            created_at: cube.created_at,
            last_accessed: cube.metadata.last_accessed,
            access_count: cube.metadata.access_count,
            importance: cube.metadata.quality_score.clamp(0.0, 1.0) as f32,
        }
    }
}

/// Tunable parameters for the hybrid retrieval pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    /// BM25 term-frequency saturation parameter.
    pub bm25_k1: f32,
    /// BM25 length-normalisation parameter.
    pub bm25_b: f32,
    /// RRF dampening constant (commonly 60).
    pub rrf_k: f32,
    /// Weight applied to the BM25 ranked list during fusion.
    pub bm25_weight: f32,
    /// Weight applied to the vector ranked list during fusion.
    pub vector_weight: f32,
    /// MMR trade-off between relevance (1.0) and diversity (0.0).
    pub mmr_lambda: f32,
    /// Enable MMR diversity re-ranking of the fused list.
    pub enable_mmr: bool,
    /// Half-life (seconds) of the exponential recency decay. `None` disables it.
    pub recency_half_life_secs: Option<f64>,
    /// How strongly recency + importance modulate the final score `[0.0, 1.0]`.
    pub salience_weight: f32,
    /// Maximum number of results to return.
    pub top_k: usize,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            bm25_k1: 1.2,
            bm25_b: 0.75,
            rrf_k: 60.0,
            bm25_weight: 1.0,
            vector_weight: 1.0,
            mmr_lambda: 0.7,
            enable_mmr: true,
            recency_half_life_secs: Some(7.0 * 24.0 * 3600.0), // one week
            salience_weight: 0.15,
            top_k: 10,
        }
    }
}

/// A scored search result with a full breakdown of the contributing signals,
/// supporting the "white-box debuggability" goal: every ranking decision is
/// inspectable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub id: Uuid,
    /// Final fused + boosted score used for ordering.
    pub score: f32,
    /// Normalised BM25 lexical score in `[0.0, 1.0]`.
    pub bm25_score: f32,
    /// Cosine similarity in `[0.0, 1.0]` (0 when no embedding).
    pub vector_score: f32,
    /// Recency decay multiplier in `[0.0, 1.0]`.
    pub recency_score: f32,
    /// Importance / salience in `[0.0, 1.0]`.
    pub importance: f32,
    /// 1-based rank in the BM25 list (0 if absent).
    pub bm25_rank: usize,
    /// 1-based rank in the vector list (0 if absent).
    pub vector_rank: usize,
}

/// Lower-cased alphanumeric tokenizer shared by BM25 indexing and querying.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

/// Raw cosine similarity in `[-1.0, 1.0]`; returns 0 on mismatch/zero-norm.
/// Used for *ranking*, where the sign matters and clamping would erase it.
fn cosine_raw(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Cosine similarity clamped to `[0.0, 1.0]` for use as a diversity/display
/// signal (negative similarities collapse to 0 redundancy).
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    cosine_raw(a, b).clamp(0.0, 1.0)
}

/// Token-set Jaccard similarity in `[0.0, 1.0]`; a text-only fallback for
/// diversity when one of the candidates has no embedding.
fn jaccard(a: &str, b: &str) -> f32 {
    use std::collections::HashSet;
    let sa: HashSet<&str> = a.split_whitespace().collect();
    let sb: HashSet<&str> = b.split_whitespace().collect();
    if sa.is_empty() && sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count() as f32;
    let union = sa.union(&sb).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// A hybrid lexical + semantic retriever over a fixed candidate set.
///
/// Build it once per query batch with [`HybridRetriever::new`]; the BM25
/// statistics (document frequencies, average document length) are precomputed
/// so multiple queries against the same corpus are cheap.
pub struct HybridRetriever {
    config: HybridSearchConfig,
    candidates: Vec<RetrievalCandidate>,
    /// Term frequency per document.
    doc_tf: Vec<HashMap<String, u32>>,
    /// Document length (token count) per document.
    doc_len: Vec<usize>,
    /// Document frequency per term across the corpus.
    doc_freq: HashMap<String, u32>,
    avg_doc_len: f32,
}

impl HybridRetriever {
    /// Index a candidate set and precompute BM25 statistics.
    pub fn new(candidates: Vec<RetrievalCandidate>, config: HybridSearchConfig) -> Self {
        let mut doc_tf = Vec::with_capacity(candidates.len());
        let mut doc_len = Vec::with_capacity(candidates.len());
        let mut doc_freq: HashMap<String, u32> = HashMap::new();

        for cand in &candidates {
            let tokens = tokenize(&cand.text);
            let mut tf: HashMap<String, u32> = HashMap::new();
            for tok in &tokens {
                *tf.entry(tok.clone()).or_insert(0) += 1;
            }
            for term in tf.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
            doc_len.push(tokens.len());
            doc_tf.push(tf);
        }

        let total_len: usize = doc_len.iter().sum();
        let avg_doc_len = if doc_len.is_empty() {
            0.0
        } else {
            total_len as f32 / doc_len.len() as f32
        };

        Self {
            config,
            candidates,
            doc_tf,
            doc_len,
            doc_freq,
            avg_doc_len,
        }
    }

    /// Number of indexed candidates.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Inverse document frequency for a term (BM25 variant, always >= 0).
    fn idf(&self, term: &str) -> f32 {
        let n = self.candidates.len() as f32;
        let df = *self.doc_freq.get(term).unwrap_or(&0) as f32;
        // BM25 "plus 1" idf keeps the value non-negative even for common terms.
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }

    /// Raw BM25 score for a single document against query terms whose IDF has
    /// been precomputed once per query (IDF is corpus-wide, not per-document).
    fn bm25_score(&self, doc_idx: usize, query_idf: &HashMap<&str, f32>) -> f32 {
        if self.avg_doc_len == 0.0 {
            return 0.0;
        }
        let k1 = self.config.bm25_k1;
        let b = self.config.bm25_b;
        let dl = self.doc_len[doc_idx] as f32;
        let tf_map = &self.doc_tf[doc_idx];

        let mut score = 0.0f32;
        for (term, idf) in query_idf {
            let tf = *tf_map.get(*term).unwrap_or(&0) as f32;
            if tf == 0.0 {
                continue;
            }
            let denom = tf + k1 * (1.0 - b + b * dl / self.avg_doc_len);
            score += idf * (tf * (k1 + 1.0)) / denom;
        }
        score
    }

    /// Execute a hybrid search. `query_text` drives BM25, `query_embedding`
    /// (when provided) drives the dense vector ranking. Either may be empty.
    pub fn search(
        &self,
        query_text: &str,
        query_embedding: Option<&[f32]>,
    ) -> Vec<ScoredMemory> {
        if self.candidates.is_empty() {
            return Vec::new();
        }

        let query_terms = tokenize(query_text);

        // Precompute IDF once per unique query term (corpus-wide, not per-doc).
        let mut query_idf: HashMap<&str, f32> = HashMap::new();
        for term in &query_terms {
            query_idf
                .entry(term.as_str())
                .or_insert_with(|| self.idf(term));
        }

        // --- Lexical (BM25) ranking ---
        let mut bm25: Vec<(usize, f32)> = (0..self.candidates.len())
            .map(|i| (i, self.bm25_score(i, &query_idf)))
            .collect();
        bm25.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let bm25_max = bm25.first().map(|(_, s)| *s).unwrap_or(0.0);
        // index -> (rank starting at 1, normalised score)
        let mut bm25_info: HashMap<usize, (usize, f32)> = HashMap::new();
        for (rank, (idx, score)) in bm25.iter().enumerate() {
            if *score > 0.0 {
                let norm = if bm25_max > 0.0 { score / bm25_max } else { 0.0 };
                bm25_info.insert(*idx, (rank + 1, norm));
            }
        }

        // --- Semantic (vector) ranking ---
        // Every candidate that *has* an embedding is ranked by raw cosine
        // (sign preserved), so a query always retrieves its nearest neighbours
        // even when the best matches have low or zero similarity. The reported
        // score is clamped to [0,1]; ordering uses the raw value.
        let mut vec_info: HashMap<usize, (usize, f32)> = HashMap::new();
        if let Some(q) = query_embedding {
            let mut sims: Vec<(usize, f32)> = self
                .candidates
                .iter()
                .enumerate()
                .filter_map(|(i, c)| c.embedding.as_ref().map(|e| (i, cosine_raw(q, e))))
                .collect();
            sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (rank, (idx, sim)) in sims.iter().enumerate() {
                vec_info.insert(*idx, (rank + 1, sim.clamp(0.0, 1.0)));
            }
        }

        // --- Reciprocal Rank Fusion of the two ranked lists ---
        let now = Utc::now();
        let mut fused: Vec<ScoredMemory> = Vec::with_capacity(self.candidates.len());
        for (idx, cand) in self.candidates.iter().enumerate() {
            let (bm25_rank, bm25_norm) = bm25_info.get(&idx).copied().unwrap_or((0, 0.0));
            let (vec_rank, vec_sim) = vec_info.get(&idx).copied().unwrap_or((0, 0.0));

            // A candidate present in neither list contributes nothing.
            if bm25_rank == 0 && vec_rank == 0 {
                continue;
            }

            let mut rrf = 0.0f32;
            if bm25_rank > 0 {
                rrf += self.config.bm25_weight / (self.config.rrf_k + bm25_rank as f32);
            }
            if vec_rank > 0 {
                rrf += self.config.vector_weight / (self.config.rrf_k + vec_rank as f32);
            }

            let recency = self.recency_multiplier(cand, now);
            // Blend the fused relevance with a salience boost (recency + importance).
            let salience = (recency + cand.importance) * 0.5;
            let final_score =
                rrf * (1.0 - self.config.salience_weight + self.config.salience_weight * salience);

            fused.push(ScoredMemory {
                id: cand.id,
                score: final_score,
                bm25_score: bm25_norm,
                vector_score: vec_sim,
                recency_score: recency,
                importance: cand.importance,
                bm25_rank,
                vector_rank: vec_rank,
            });
        }

        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        if self.config.enable_mmr {
            self.mmr_rerank(fused)
        } else {
            fused.truncate(self.config.top_k);
            fused
        }
    }

    /// Exponential recency decay in `[0.0, 1.0]` based on the configured
    /// half-life. Falls back to 1.0 when decay is disabled.
    fn recency_multiplier(&self, cand: &RetrievalCandidate, now: DateTime<Utc>) -> f32 {
        match self.config.recency_half_life_secs {
            Some(half_life) if half_life > 0.0 => {
                let age = (now - cand.last_accessed).num_seconds().max(0) as f64;
                let decay = 0.5f64.powf(age / half_life);
                decay as f32
            }
            _ => 1.0,
        }
    }

    /// Maximal Marginal Relevance re-ranking for result diversity. Greedily
    /// selects the next item maximising `λ·relevance − (1−λ)·maxSimToSelected`.
    /// The max-similarity-to-selected is cached and updated incrementally, so
    /// the pass is `O(top_k · remaining)` rather than `O(top_k² · remaining)`.
    fn mmr_rerank(&self, ranked: Vec<ScoredMemory>) -> Vec<ScoredMemory> {
        let lambda = self.config.mmr_lambda;
        let limit = self.config.top_k.min(ranked.len());
        let cand_by_id: HashMap<Uuid, &RetrievalCandidate> =
            self.candidates.iter().map(|c| (c.id, c)).collect();

        let mut remaining: Vec<ScoredMemory> = ranked;
        let mut selected: Vec<ScoredMemory> = Vec::with_capacity(limit);
        // Cached max similarity of each remaining item to the selected set.
        let mut max_sim: Vec<f32> = vec![0.0; remaining.len()];

        while selected.len() < limit && !remaining.is_empty() {
            let mut best_idx = 0usize;
            let mut best_mmr = f32::NEG_INFINITY;
            for (i, cand) in remaining.iter().enumerate() {
                let mmr = lambda * cand.score - (1.0 - lambda) * max_sim[i];
                if mmr > best_mmr {
                    best_mmr = mmr;
                    best_idx = i;
                }
            }

            let chosen = remaining.remove(best_idx);
            max_sim.remove(best_idx);

            // Refresh each remaining item's max similarity against the new pick.
            if let Some(chosen_cand) = cand_by_id.get(&chosen.id) {
                for (i, cand) in remaining.iter().enumerate() {
                    if let Some(c) = cand_by_id.get(&cand.id) {
                        let sim = Self::pair_similarity(c, chosen_cand);
                        if sim > max_sim[i] {
                            max_sim[i] = sim;
                        }
                    }
                }
            }
            selected.push(chosen);
        }
        selected
    }

    /// Diversity similarity between two candidates: cosine when both carry
    /// embeddings, otherwise token-set Jaccard on their text. This avoids the
    /// asymmetry where embedding-less candidates would always look maximally
    /// diverse and crowd out genuinely novel results.
    fn pair_similarity(a: &RetrievalCandidate, b: &RetrievalCandidate) -> f32 {
        match (a.embedding.as_ref(), b.embedding.as_ref()) {
            (Some(ea), Some(eb)) => cosine(ea, eb),
            _ => jaccard(&a.text, &b.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id_seed: u128, text: &str, emb: Option<Vec<f32>>, age_secs: i64) -> RetrievalCandidate {
        RetrievalCandidate {
            id: Uuid::from_u128(id_seed),
            text: text.to_string(),
            embedding: emb,
            created_at: Utc::now() - chrono::Duration::seconds(age_secs),
            last_accessed: Utc::now() - chrono::Duration::seconds(age_secs),
            access_count: 0,
            importance: 0.5,
        }
    }

    #[test]
    fn bm25_ranks_keyword_matches_first() {
        let cands = vec![
            cand(1, "the quick brown fox jumps", None, 0),
            cand(2, "a lazy dog sleeps all day", None, 0),
            cand(3, "the fox and the hound", None, 0),
        ];
        let retriever = HybridRetriever::new(cands, HybridSearchConfig::default());
        let results = retriever.search("fox", None);
        assert!(!results.is_empty());
        // Both fox docs should outrank the dog doc.
        let dog = results.iter().find(|r| r.id == Uuid::from_u128(2));
        assert!(dog.is_none() || dog.unwrap().score < results[0].score);
    }

    #[test]
    fn vector_search_surfaces_semantic_match() {
        let cands = vec![
            cand(1, "unrelated text", Some(vec![1.0, 0.0, 0.0]), 0),
            cand(2, "another doc", Some(vec![0.0, 1.0, 0.0]), 0),
        ];
        let retriever = HybridRetriever::new(cands, HybridSearchConfig::default());
        let results = retriever.search("", Some(&[0.0, 1.0, 0.0]));
        assert_eq!(results.first().map(|r| r.id), Some(Uuid::from_u128(2)));
    }

    #[test]
    fn rrf_fuses_both_signals() {
        let cands = vec![
            cand(1, "rust memory systems", Some(vec![1.0, 0.0]), 0),
            cand(2, "garden vegetables", Some(vec![0.9, 0.1]), 0),
        ];
        let retriever = HybridRetriever::new(cands, HybridSearchConfig::default());
        let results = retriever.search("rust memory", Some(&[1.0, 0.0]));
        // Doc 1 wins on both lexical and vector signals.
        assert_eq!(results[0].id, Uuid::from_u128(1));
        assert!(results[0].bm25_rank > 0 && results[0].vector_rank > 0);
    }

    #[test]
    fn recency_breaks_ties_toward_newer() {
        let mut cfg = HybridSearchConfig::default();
        cfg.salience_weight = 0.9;
        cfg.enable_mmr = false;
        let cands = vec![
            cand(1, "same content here", None, 60 * 60 * 24 * 30), // 30 days old
            cand(2, "same content here", None, 1),                 // fresh
        ];
        let retriever = HybridRetriever::new(cands, cfg);
        let results = retriever.search("same content", None);
        assert_eq!(results[0].id, Uuid::from_u128(2));
    }

    #[test]
    fn empty_corpus_returns_empty() {
        let retriever = HybridRetriever::new(vec![], HybridSearchConfig::default());
        assert!(retriever.search("anything", None).is_empty());
    }
}
