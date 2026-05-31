// src/memory/ann/quantized_index.rs
//! Quantized ANN index with the "oversample + rescore" pattern (Phase 2, #11).
//!
//! Combines the two quantizers from [`super::quantization`] into a single
//! memory-efficient index:
//!
//! * Each vector is stored as a **binary code** (1 bit/dim) *and* an **int8
//!   scalar code** (8 bits/dim) — together ~9 bits/dim versus 32 bits/dim for
//!   `f32` (~3.5× smaller), and no full-precision copy is kept.
//! * **Search** first ranks all vectors by **Hamming similarity** on the binary
//!   codes (hardware `popcount`, extremely fast), keeps the top `k × oversample`
//!   candidates, then **rescores** only those with cosine over the decoded
//!   int8 vectors and returns the top `k`.
//!
//! This is the standard billion-scale trick: a cheap coarse filter over compact
//! codes, then a precise rerank over a small candidate set — high recall at a
//! fraction of the memory and distance-compute cost.

use uuid::Uuid;

use super::hnsw::Neighbor;
use super::quantization::{BinaryQuantized, ScalarQuantized};

/// A flat, quantized nearest-neighbour index.
#[derive(Debug, Default)]
pub struct QuantizedIndex {
    dim: usize,
    ids: Vec<Uuid>,
    binary: Vec<BinaryQuantized>,
    scalar: Vec<ScalarQuantized>,
    /// How many extra candidates (× k) to keep from the Hamming pre-filter.
    oversample: usize,
}

impl QuantizedIndex {
    /// Create an index. `oversample` controls the recall/speed trade-off of the
    /// pre-filter (typical 4–16; higher = better recall, more rescoring).
    pub fn new(oversample: usize) -> Self {
        Self { oversample: oversample.max(1), ..Default::default() }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Bytes used by the stored codes (excludes id bookkeeping).
    pub fn code_bytes(&self) -> usize {
        self.binary.iter().map(|b| b.byte_len()).sum::<usize>()
            + self.scalar.iter().map(|s| s.byte_len()).sum::<usize>()
    }

    /// Bytes the same vectors would use as raw `f32` (for a savings comparison).
    pub fn full_precision_bytes(&self) -> usize {
        self.ids.len() * self.dim * std::mem::size_of::<f32>()
    }

    /// Add a vector under `id`.
    pub fn add(&mut self, id: Uuid, vector: &[f32]) {
        if self.dim == 0 {
            self.dim = vector.len();
        }
        self.ids.push(id);
        self.binary.push(BinaryQuantized::encode(vector));
        self.scalar.push(ScalarQuantized::encode(vector));
    }

    /// Search for the `k` nearest neighbours: Hamming pre-filter → cosine
    /// rescore over decoded int8 vectors.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<Neighbor> {
        if self.is_empty() || k == 0 {
            return Vec::new();
        }
        let qb = BinaryQuantized::encode(query);

        // Stage 1: coarse rank by binary similarity (popcount).
        let mut coarse: Vec<(f32, usize)> = self
            .binary
            .iter()
            .enumerate()
            .map(|(i, b)| (qb.similarity(b), i))
            .collect();
        let pool = (k * self.oversample).min(coarse.len());
        // Partial select the top `pool` by binary similarity.
        coarse.sort_by(|a, b| b.0.total_cmp(&a.0));
        coarse.truncate(pool);

        // Stage 2: precise rescore with cosine over the decoded int8 vectors.
        let mut rescored: Vec<(f32, usize)> = coarse
            .into_iter()
            .map(|(_, i)| (cosine(query, &self.scalar[i].decode()), i))
            .collect();
        rescored.sort_by(|a, b| b.0.total_cmp(&a.0));
        rescored.truncate(k);

        rescored
            .into_iter()
            .map(|(score, i)| Neighbor { id: self.ids[i], score: score.clamp(0.0, 1.0) })
            .collect()
    }
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
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn finds_exact_match() {
        let mut idx = QuantizedIndex::new(8);
        idx.add(id(1), &[1.0, 0.0, 0.0]);
        idx.add(id(2), &[0.0, 1.0, 0.0]);
        idx.add(id(3), &[0.0, 0.0, 1.0]);
        let res = idx.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(res[0].id, id(1));
    }

    #[test]
    fn uses_less_memory_than_f32() {
        let mut idx = QuantizedIndex::new(8);
        for i in 0..50u128 {
            let v: Vec<f32> = (0..128).map(|j| ((i + j) as f32).sin()).collect();
            idx.add(id(i), &v);
        }
        // ~9 bits/dim vs 32 bits/dim → codes are well under half the f32 size.
        assert!(idx.code_bytes() < idx.full_precision_bytes() / 2);
    }

    #[test]
    fn high_recall_against_brute_force() {
        // Deterministic dataset; compare quantized top-1 to exact f32 cosine.
        let dim = 32;
        let n = 400usize;
        let mut state = 0xC0FFEEu64;
        let mut rng = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f32 / u64::MAX as f32) * 2.0 - 1.0
        };
        let data: Vec<Vec<f32>> = (0..n).map(|_| (0..dim).map(|_| rng()).collect()).collect();

        let mut idx = QuantizedIndex::new(16);
        for (i, v) in data.iter().enumerate() {
            idx.add(id(i as u128), v);
        }

        let mut hits = 0;
        let trials = 40;
        for _ in 0..trials {
            let q: Vec<f32> = (0..dim).map(|_| rng()).collect();
            let truth = (0..n)
                .max_by(|&a, &b| cosine(&data[a], &q).total_cmp(&cosine(&data[b], &q)))
                .unwrap();
            let got = idx.search(&q, 5);
            if got.iter().any(|nb| nb.id == id(truth as u128)) {
                hits += 1;
            }
        }
        // With oversample=16, recall@5 should be high despite quantization.
        assert!(hits as f32 / trials as f32 >= 0.8, "recall too low: {}/{}", hits, trials);
    }

    #[test]
    fn empty_is_safe() {
        let idx = QuantizedIndex::new(8);
        assert!(idx.search(&[1.0, 0.0], 5).is_empty());
    }
}
