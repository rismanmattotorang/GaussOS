// src/memory/ann/quantization.rs
//! Vector quantization for memory-efficient storage and fast pre-filtering.
//!
//! Full-precision `f32` embeddings dominate the memory footprint of a vector
//! store (a 1024-dim vector is 4 KiB). Quantization trades a small, recoverable
//! amount of recall for large memory savings — the standard technique behind
//! billion-scale vector search:
//!
//! * **Scalar quantization (int8)**: ~4× smaller, ~1-2% recall loss. Each
//!   dimension is linearly mapped from `[min, max]` to `[-127, 127]`.
//! * **Binary quantization (1 bit/dim)**: ~32× smaller. Each dimension becomes
//!   its sign bit; distance is Hamming via hardware `popcount` (`u64::count_ones`),
//!   which is extremely fast. Used as a coarse pre-filter, then re-ranked with
//!   full-precision vectors (the "oversample + rescore" pattern).

use serde::{Deserialize, Serialize};

/// An int8-quantized vector with the affine parameters needed to decode it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantized {
    pub codes: Vec<i8>,
    /// Value mapped to code 0.
    pub offset: f32,
    /// Width of one quantization step.
    pub scale: f32,
}

impl ScalarQuantized {
    /// Quantize an `f32` vector to int8 using its own min/max range.
    pub fn encode(vector: &[f32]) -> Self {
        if vector.is_empty() {
            return Self { codes: Vec::new(), offset: 0.0, scale: 1.0 };
        }
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &x in vector {
            min = min.min(x);
            max = max.max(x);
        }
        // Map [min, max] -> [-127, 127]. Guard a zero-width range.
        let range = (max - min).max(f32::EPSILON);
        let scale = range / 254.0;
        let offset = min;
        let codes = vector
            .iter()
            .map(|&x| {
                let q = ((x - offset) / scale).round() - 127.0;
                q.clamp(-127.0, 127.0) as i8
            })
            .collect();
        Self { codes, offset, scale }
    }

    /// Reconstruct the approximate `f32` vector.
    pub fn decode(&self) -> Vec<f32> {
        self.codes
            .iter()
            .map(|&c| (c as f32 + 127.0) * self.scale + self.offset)
            .collect()
    }

    /// Bytes used by the codes (excluding the two scalars).
    pub fn byte_len(&self) -> usize {
        self.codes.len()
    }
}

/// A binary-quantized vector: one sign bit per dimension, packed into `u64`s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryQuantized {
    /// Bit-packed sign bits (`x >= 0` → 1).
    pub bits: Vec<u64>,
    /// Original dimensionality (bits beyond this in the last word are zero).
    pub dim: usize,
}

impl BinaryQuantized {
    /// Quantize by taking the sign of each dimension.
    pub fn encode(vector: &[f32]) -> Self {
        let words = vector.len().div_ceil(64);
        let mut bits = vec![0u64; words];
        for (i, &x) in vector.iter().enumerate() {
            if x >= 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        Self { bits, dim: vector.len() }
    }

    /// Hamming distance (number of differing sign bits) via popcount.
    pub fn hamming(&self, other: &BinaryQuantized) -> u32 {
        self.bits
            .iter()
            .zip(&other.bits)
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }

    /// Similarity in `[0, 1]`: fraction of agreeing sign bits.
    pub fn similarity(&self, other: &BinaryQuantized) -> f32 {
        if self.dim == 0 {
            return 1.0;
        }
        let d = self.hamming(other) as f32;
        1.0 - d / self.dim as f32
    }

    /// Bytes used by the packed bits.
    pub fn byte_len(&self) -> usize {
        self.bits.len() * 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_roundtrip_is_close() {
        let v = vec![-1.5, 0.0, 0.3, 2.7, -0.9, 1.1];
        let q = ScalarQuantized::encode(&v);
        let d = q.decode();
        for (a, b) in v.iter().zip(&d) {
            assert!((a - b).abs() < 0.05, "{} vs {}", a, b);
        }
        // 4x smaller than f32 (1 byte vs 4).
        assert_eq!(q.byte_len(), v.len());
    }

    #[test]
    fn scalar_handles_constant_vector() {
        let v = vec![3.0, 3.0, 3.0];
        let q = ScalarQuantized::encode(&v);
        let d = q.decode();
        for x in d {
            assert!((x - 3.0).abs() < 0.01);
        }
    }

    #[test]
    fn binary_hamming_and_similarity() {
        let a = BinaryQuantized::encode(&[1.0, -1.0, 1.0, -1.0]);
        let b = BinaryQuantized::encode(&[1.0, -1.0, 1.0, -1.0]);
        let c = BinaryQuantized::encode(&[-1.0, 1.0, -1.0, 1.0]);
        assert_eq!(a.hamming(&b), 0);
        assert!((a.similarity(&b) - 1.0).abs() < 1e-6);
        assert_eq!(a.hamming(&c), 4);
        assert!((a.similarity(&c) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn binary_packs_32x() {
        // 128-dim f32 = 512 bytes; binary = 16 bytes (2 u64 words) → 32x.
        let v: Vec<f32> = (0..128).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let q = BinaryQuantized::encode(&v);
        assert_eq!(q.byte_len(), 16);
        assert_eq!(v.len() * 4 / q.byte_len(), 32);
    }

    #[test]
    fn binary_similarity_orders_by_sign_agreement() {
        let query = BinaryQuantized::encode(&[1.0, 1.0, 1.0, 1.0]);
        let near = BinaryQuantized::encode(&[1.0, 1.0, 1.0, -1.0]);
        let far = BinaryQuantized::encode(&[-1.0, -1.0, -1.0, -1.0]);
        assert!(query.similarity(&near) > query.similarity(&far));
    }
}
