// src/memory/scoring.rs
//! Generative-Agents retrieval scoring (Park et al., 2023).
//!
//! The "Generative Agents" memory stream ranks memories by a weighted sum of
//! three signals, each **min-max normalised to `[0, 1]`** across the candidate
//! set so they are comparable:
//!
//! ```text
//! score = w_recency·recency + w_importance·importance + w_relevance·relevance
//! ```
//!
//! * **recency** = `0.995^(hours since last access)` — exponential decay.
//! * **importance** = the memory's intrinsic salience (the paper uses an
//!   LLM-assigned 1–10 "poignancy"; GaussOS uses the stored quality score, and
//!   callers may override it with an LLM rating).
//! * **relevance** = cosine similarity between the query and memory embeddings.
//!
//! In the original work all three weights are `1.0`. This is a deterministic,
//! pure-Rust complement to the lexical/RRF [`super::retrieval`] path: where RRF
//! fuses *rankings*, this fuses *normalised scores* and is the better fit when
//! recency and importance should weigh as heavily as semantic relevance (e.g.
//! conversational/companion agents).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::retrieval::RetrievalCandidate;

/// Weights for the three Generative-Agents signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaWeights {
    pub recency: f32,
    pub importance: f32,
    pub relevance: f32,
    /// Recency decay base applied per hour since last access (paper: 0.995).
    pub recency_decay: f32,
    pub top_k: usize,
}

impl Default for GaWeights {
    fn default() -> Self {
        Self {
            recency: 1.0,
            importance: 1.0,
            relevance: 1.0,
            recency_decay: 0.995,
            top_k: 10,
        }
    }
}

/// A scored memory with the three normalised components exposed for inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaScored {
    pub id: Uuid,
    pub score: f32,
    pub recency: f32,
    pub importance: f32,
    pub relevance: f32,
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())).clamp(0.0, 1.0)
}

/// Min-max normalise a slice to `[0, 1]` in place. A constant slice maps to 1.0
/// (the signal carries no discriminating information, so it neither helps nor
/// hurts the ranking).
fn min_max_normalize(xs: &mut [f32]) {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &x in xs.iter() {
        lo = lo.min(x);
        hi = hi.max(x);
    }
    let range = hi - lo;
    if range <= f32::EPSILON {
        xs.iter_mut().for_each(|x| *x = 1.0);
    } else {
        xs.iter_mut().for_each(|x| *x = (*x - lo) / range);
    }
}

/// Generative-Agents memory scorer.
#[derive(Debug, Clone, Default)]
pub struct GenerativeAgentScorer {
    weights: GaWeights,
}

impl GenerativeAgentScorer {
    pub fn new(weights: GaWeights) -> Self {
        Self { weights }
    }

    /// Score and rank candidates. `query_embedding` may be `None`, in which case
    /// relevance is zero for every candidate and ranking falls back to
    /// recency + importance.
    pub fn rank(
        &self,
        candidates: &[RetrievalCandidate],
        query_embedding: Option<&[f32]>,
        now: DateTime<Utc>,
    ) -> Vec<GaScored> {
        if candidates.is_empty() {
            return Vec::new();
        }

        let mut recency: Vec<f32> = candidates
            .iter()
            .map(|c| {
                let hours = (now - c.last_accessed).num_seconds().max(0) as f32 / 3600.0;
                self.weights.recency_decay.powf(hours)
            })
            .collect();

        let mut importance: Vec<f32> = candidates.iter().map(|c| c.importance).collect();

        let mut relevance: Vec<f32> = candidates
            .iter()
            .map(|c| match (query_embedding, c.embedding.as_ref()) {
                (Some(q), Some(e)) => cosine(q, e),
                _ => 0.0,
            })
            .collect();

        // Normalise each component across the candidate set (the paper's recipe).
        min_max_normalize(&mut recency);
        min_max_normalize(&mut importance);
        min_max_normalize(&mut relevance);

        let mut scored: Vec<GaScored> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| GaScored {
                id: c.id,
                score: self.weights.recency * recency[i]
                    + self.weights.importance * importance[i]
                    + self.weights.relevance * relevance[i],
                recency: recency[i],
                importance: importance[i],
                relevance: relevance[i],
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.weights.top_k);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(seed: u128, importance: f32, age_hours: i64, emb: Option<Vec<f32>>) -> RetrievalCandidate {
        RetrievalCandidate {
            id: Uuid::from_u128(seed),
            text: String::new(),
            embedding: emb,
            created_at: Utc::now(),
            last_accessed: Utc::now() - chrono::Duration::hours(age_hours),
            access_count: 0,
            importance,
        }
    }

    #[test]
    fn recency_favours_recent_when_other_signals_tie() {
        let scorer = GenerativeAgentScorer::default();
        let cands = vec![
            cand(1, 0.5, 1000, None), // old
            cand(2, 0.5, 0, None),    // fresh
        ];
        let ranked = scorer.rank(&cands, None, Utc::now());
        assert_eq!(ranked[0].id, Uuid::from_u128(2));
    }

    #[test]
    fn importance_breaks_ties() {
        let scorer = GenerativeAgentScorer::default();
        let cands = vec![
            cand(1, 0.1, 0, None),
            cand(2, 0.9, 0, None),
        ];
        let ranked = scorer.rank(&cands, None, Utc::now());
        assert_eq!(ranked[0].id, Uuid::from_u128(2));
    }

    #[test]
    fn relevance_uses_query_embedding() {
        let scorer = GenerativeAgentScorer::default();
        let cands = vec![
            cand(1, 0.5, 0, Some(vec![1.0, 0.0])),
            cand(2, 0.5, 0, Some(vec![0.0, 1.0])),
        ];
        let ranked = scorer.rank(&cands, Some(&[0.0, 1.0]), Utc::now());
        assert_eq!(ranked[0].id, Uuid::from_u128(2));
    }

    #[test]
    fn constant_component_does_not_break_scoring() {
        // All same importance and age; only relevance should decide.
        let scorer = GenerativeAgentScorer::default();
        let cands = vec![
            cand(1, 0.5, 5, Some(vec![1.0, 0.0])),
            cand(2, 0.5, 5, Some(vec![0.9, 0.1])),
        ];
        let ranked = scorer.rank(&cands, Some(&[1.0, 0.0]), Utc::now());
        assert_eq!(ranked[0].id, Uuid::from_u128(1));
    }

    #[test]
    fn empty_returns_empty() {
        let scorer = GenerativeAgentScorer::default();
        assert!(scorer.rank(&[], None, Utc::now()).is_empty());
    }
}
