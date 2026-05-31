// src/memory/similarity.rs
//! High-performance similarity utilities used across GaussOS memory subsystems.

use rayon::prelude::*;
use std::collections::HashSet;

/// Compute cosine similarity between two float vectors.
/// Returns 0.0 on length mismatch or zero-norm vectors.
#[inline]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.par_iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.par_iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Token-based Jaccard similarity; case-insensitive, splits by whitespace.
#[inline]
pub fn jaccard(a: &str, b: &str) -> f32 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    let intersection = set_a.intersection(&set_b).count() as f32;
    let union = set_a.union(&set_b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}
