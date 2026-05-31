// src/memory/ann/mod.rs
//! Approximate-nearest-neighbour indexing for GaussOS.
//!
//! * [`hnsw`] — a from-scratch HNSW graph index for sublinear vector search.
//! * [`quantization`] — scalar (int8) and binary vector compression for
//!   memory-efficient storage and fast Hamming pre-filtering.

pub mod hnsw;
pub mod quantization;

pub use hnsw::{Distance, Hnsw, HnswConfig, Neighbor};
pub use quantization::{BinaryQuantized, ScalarQuantized};
