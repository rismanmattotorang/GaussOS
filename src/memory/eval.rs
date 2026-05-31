// src/memory/eval.rs
//! Retrieval evaluation harness (Phase 0, roadmap #1).
//!
//! Credible memory systems must be *measured*, not asserted. This module
//! provides the retrieval-quality core of a benchmark harness (LoCoMo /
//! LongMemEval style): given labelled cases of `query → relevant memory ids`
//! and any retriever, it computes standard IR metrics — **recall@k,
//! precision@k, MRR, hit‑rate** — deterministically and with no LLM required.
//!
//! The answer-correctness layer (LLM‑judge over generated answers) plugs in on
//! top via [`crate::agents::llm`]; the retrieval metrics here are the part that
//! can (and should) be verified offline and in CI.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// One labelled retrieval case: a query and the set of ids that should be found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    pub query: String,
    pub relevant_ids: Vec<Uuid>,
}

/// Aggregate retrieval metrics over a set of cases at cutoff `k`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalMetrics {
    pub k: usize,
    pub cases: usize,
    /// Mean fraction of a case's relevant ids found within the top-k.
    pub recall_at_k: f64,
    /// Mean fraction of the top-k that are relevant.
    pub precision_at_k: f64,
    /// Mean reciprocal rank of the first relevant result.
    pub mrr: f64,
    /// Fraction of cases with at least one relevant result in the top-k.
    pub hit_rate: f64,
}

impl RetrievalMetrics {
    /// Compact one-line report.
    pub fn report(&self) -> String {
        format!(
            "n={} k={}  recall@k={:.3}  precision@k={:.3}  MRR={:.3}  hit_rate={:.3}",
            self.cases, self.k, self.recall_at_k, self.precision_at_k, self.mrr, self.hit_rate
        )
    }
}

/// Evaluate a retriever over `cases` at cutoff `k`. `retrieve(query)` must
/// return ranked memory ids (best first).
pub fn evaluate<F>(cases: &[EvalCase], k: usize, mut retrieve: F) -> RetrievalMetrics
where
    F: FnMut(&str) -> Vec<Uuid>,
{
    if cases.is_empty() || k == 0 {
        return RetrievalMetrics {
            k,
            cases: 0,
            recall_at_k: 0.0,
            precision_at_k: 0.0,
            mrr: 0.0,
            hit_rate: 0.0,
        };
    }

    let mut sum_recall = 0.0;
    let mut sum_precision = 0.0;
    let mut sum_mrr = 0.0;
    let mut hits = 0usize;

    for case in cases {
        let relevant: HashSet<Uuid> = case.relevant_ids.iter().copied().collect();
        if relevant.is_empty() {
            continue;
        }
        let ranked = retrieve(&case.query);
        let topk: Vec<Uuid> = ranked.into_iter().take(k).collect();

        let found = topk.iter().filter(|id| relevant.contains(id)).count();
        sum_recall += found as f64 / relevant.len() as f64;
        sum_precision += found as f64 / k as f64;

        if let Some(pos) = topk.iter().position(|id| relevant.contains(id)) {
            sum_mrr += 1.0 / (pos as f64 + 1.0);
            hits += 1;
        }
    }

    let n = cases.len() as f64;
    RetrievalMetrics {
        k,
        cases: cases.len(),
        recall_at_k: sum_recall / n,
        precision_at_k: sum_precision / n,
        mrr: sum_mrr / n,
        hit_rate: hits as f64 / n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn perfect_retriever_scores_one() {
        let cases = vec![
            EvalCase { query: "a".into(), relevant_ids: vec![id(1)] },
            EvalCase { query: "b".into(), relevant_ids: vec![id(2)] },
        ];
        let m = evaluate(&cases, 5, |q| match q {
            "a" => vec![id(1), id(9)],
            _ => vec![id(2), id(8)],
        });
        assert!((m.recall_at_k - 1.0).abs() < 1e-9);
        assert!((m.mrr - 1.0).abs() < 1e-9);
        assert!((m.hit_rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn mrr_reflects_rank() {
        let cases = vec![EvalCase { query: "a".into(), relevant_ids: vec![id(1)] }];
        // Relevant doc at rank 2 → MRR 0.5.
        let m = evaluate(&cases, 5, |_| vec![id(9), id(1)]);
        assert!((m.mrr - 0.5).abs() < 1e-9);
    }

    #[test]
    fn miss_scores_zero() {
        let cases = vec![EvalCase { query: "a".into(), relevant_ids: vec![id(1)] }];
        let m = evaluate(&cases, 3, |_| vec![id(7), id(8), id(9)]);
        assert_eq!(m.recall_at_k, 0.0);
        assert_eq!(m.hit_rate, 0.0);
        assert_eq!(m.mrr, 0.0);
    }

    #[test]
    fn empty_is_safe() {
        let m = evaluate(&[], 5, |_| vec![]);
        assert_eq!(m.cases, 0);
    }
}
