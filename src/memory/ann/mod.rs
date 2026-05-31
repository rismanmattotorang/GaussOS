// src/memory/ann/mod.rs
//! Approximate-nearest-neighbour indexing for GaussOS.
//!
//! * [`hnsw`] — a from-scratch HNSW graph index for sublinear vector search,
//!   with tombstoned deletes and byte-buffer persistence.
//! * [`quantization`] — scalar (int8) and binary vector compression.
//! * [`quantized_index`] — a flat quantized index using binary pre-filter +
//!   scalar rescore (the "oversample + rescore" pattern).

pub mod distance;
pub mod hnsw;
pub mod quantization;
pub mod quantized_index;
pub mod sharded;

pub use hnsw::{Distance, Hnsw, HnswConfig, Neighbor};
pub use quantization::{BinaryQuantized, ScalarQuantized};
pub use quantized_index::QuantizedIndex;
pub use sharded::ShardedHnsw;
