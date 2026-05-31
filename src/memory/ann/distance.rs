// src/memory/ann/distance.rs
//! Distance / similarity kernels for the ANN layer (Phase 2, #12).
//!
//! These are written for **auto-vectorisation**: the hot loops accumulate over
//! 8-lane chunks with independent accumulators, which LLVM lowers to packed SIMD
//! (SSE/AVX/AVX-512, depending on `target-cpu`) without any unsafe intrinsics or
//! extra dependencies. Building with `RUSTFLAGS="-C target-cpu=native"` (or the
//! release profile's `target-feature`) lets the compiler emit AVX-512 where the
//! CPU supports it, while the code stays portable everywhere else.
//!
//! Centralising the kernels here also removes the duplicated cosine
//! implementations that had crept into several modules.

const LANES: usize = 8;

/// Dot product `Σ aᵢ·bᵢ`. Returns 0 on length mismatch.
#[inline]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let n = a.len();
    let chunks = n / LANES;
    // Independent lane accumulators so the compiler can vectorise the reduction.
    let mut acc = [0.0f32; LANES];
    for c in 0..chunks {
        let base = c * LANES;
        for l in 0..LANES {
            acc[l] += a[base + l] * b[base + l];
        }
    }
    let mut sum: f32 = acc.iter().sum();
    for i in (chunks * LANES)..n {
        sum += a[i] * b[i];
    }
    sum
}

/// Squared L2 distance `Σ (aᵢ-bᵢ)²`.
#[inline]
pub fn l2_sq(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    let n = a.len();
    let chunks = n / LANES;
    let mut acc = [0.0f32; LANES];
    for c in 0..chunks {
        let base = c * LANES;
        for l in 0..LANES {
            let d = a[base + l] - b[base + l];
            acc[l] += d * d;
        }
    }
    let mut sum: f32 = acc.iter().sum();
    for i in (chunks * LANES)..n {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum
}

/// Cosine similarity in `[-1, 1]`; 0 on mismatch or zero-norm vectors.
#[inline]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let n = a.len();
    let chunks = n / LANES;
    let mut dot_acc = [0.0f32; LANES];
    let mut na_acc = [0.0f32; LANES];
    let mut nb_acc = [0.0f32; LANES];
    for c in 0..chunks {
        let base = c * LANES;
        for l in 0..LANES {
            let x = a[base + l];
            let y = b[base + l];
            dot_acc[l] += x * y;
            na_acc[l] += x * x;
            nb_acc[l] += y * y;
        }
    }
    let mut d: f32 = dot_acc.iter().sum();
    let mut na: f32 = na_acc.iter().sum();
    let mut nb: f32 = nb_acc.iter().sum();
    for i in (chunks * LANES)..n {
        d += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        d / (na.sqrt() * nb.sqrt())
    }
}

/// Hamming distance over bit-packed `u64` lanes (popcount).
#[inline]
pub fn hamming_u64(a: &[u64], b: &[u64]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_dot(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn dot_matches_naive_across_lengths() {
        for len in [0usize, 1, 7, 8, 9, 16, 33, 100] {
            let a: Vec<f32> = (0..len).map(|i| i as f32 * 0.5 - 3.0).collect();
            let b: Vec<f32> = (0..len).map(|i| (i as f32).sin()).collect();
            let got = dot(&a, &b);
            let want = naive_dot(&a, &b);
            assert!((got - want).abs() < 1e-3, "len {len}: {got} vs {want}");
        }
    }

    #[test]
    fn cosine_basic() {
        assert!((cosine(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert!((cosine(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn l2_sq_basic() {
        assert!((l2_sq(&[0.0, 0.0], &[3.0, 4.0]) - 25.0).abs() < 1e-6);
        assert_eq!(l2_sq(&[1.0], &[1.0, 2.0]), f32::INFINITY);
    }

    #[test]
    fn hamming_popcount() {
        assert_eq!(hamming_u64(&[0b1010], &[0b0011]), 2);
        assert_eq!(hamming_u64(&[u64::MAX], &[0]), 64);
    }
}
