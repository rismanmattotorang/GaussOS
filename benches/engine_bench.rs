//! Phase 0 reproducible performance benchmarks for the GaussOS memory engine.
//!
//! Covers the hot paths that differentiate GaussOS: hybrid retrieval (BM25 +
//! vector + RRF + MMR), the HNSW ANN index (build + query), and Personalized
//! PageRank multi-hop graph retrieval.
//!
//! Run with:  `cargo bench --bench engine_bench`
//! Compile-check only:  `cargo bench --bench engine_bench --no-run`

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gaussos::memory::ann::{Distance, Hnsw, HnswConfig};
use gaussos::memory::graph_retrieval::{GraphRetriever, PprConfig};
use gaussos::memory::retrieval::{HybridRetriever, HybridSearchConfig, RetrievalCandidate};
use gaussos::memory::temporal::{TemporalFact, TemporalFactStore};
use uuid::Uuid;

/// Deterministic pseudo-random f32 generator (xorshift) so benches are stable.
fn rng(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as f32 / u64::MAX as f32) * 2.0 - 1.0
}

fn make_candidates(n: usize, dim: usize) -> Vec<RetrievalCandidate> {
    let words = ["rust", "memory", "agent", "graph", "vector", "temporal", "cache", "index", "query", "engine"];
    let mut state = 0x1234_5678u64;
    (0..n)
        .map(|i| {
            let text = (0..6)
                .map(|j| words[(i + j) % words.len()])
                .collect::<Vec<_>>()
                .join(" ");
            let embedding = Some((0..dim).map(|_| rng(&mut state)).collect());
            RetrievalCandidate {
                id: Uuid::from_u128(i as u128 + 1),
                text,
                embedding,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 0,
                importance: 0.5,
            }
        })
        .collect()
}

fn bench_hybrid(c: &mut Criterion) {
    let dim = 64;
    let mut group = c.benchmark_group("hybrid_retrieval");
    for &n in &[100usize, 1000] {
        let candidates = make_candidates(n, dim);
        let query_emb: Vec<f32> = candidates[0].embedding.clone().unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                // new() builds the BM25 index; search() does fusion + MMR.
                let r = HybridRetriever::new(candidates.clone(), HybridSearchConfig::default());
                black_box(r.search("rust memory engine", Some(&query_emb)))
            });
        });
    }
    group.finish();
}

fn bench_hnsw(c: &mut Criterion) {
    let dim = 64;
    let mut state = 0xABCDu64;

    // Build benchmark.
    let mut build = c.benchmark_group("hnsw_build");
    for &n in &[1000usize, 5000] {
        build.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let mut h = Hnsw::new(Distance::Cosine, HnswConfig::default());
                let mut s = 0x99u64;
                for i in 0..n {
                    let v: Vec<f32> = (0..dim).map(|_| rng(&mut s)).collect();
                    h.insert(Uuid::from_u128(i as u128 + 1), v);
                }
                black_box(h.len())
            });
        });
    }
    build.finish();

    // Query benchmark over a prebuilt index.
    let mut h = Hnsw::new(Distance::Cosine, HnswConfig::default());
    for i in 0..5000 {
        let v: Vec<f32> = (0..dim).map(|_| rng(&mut state)).collect();
        h.insert(Uuid::from_u128(i as u128 + 1), v);
    }
    let query: Vec<f32> = (0..dim).map(|_| rng(&mut state)).collect();
    c.bench_function("hnsw_query_k10_n5000", |b| {
        b.iter(|| black_box(h.search(&query, 10)))
    });
}

fn bench_ppr(c: &mut Criterion) {
    // Build a chain + branches graph of facts.
    let mut store = TemporalFactStore::new();
    for i in 0..500 {
        store.ingest(TemporalFact::new(
            format!("e{i}"),
            "rel",
            format!("e{}", i + 1),
        ));
    }
    let retriever = GraphRetriever::new(PprConfig::default());
    c.bench_function("ppr_multihop_n500", |b| {
        b.iter(|| black_box(retriever.search(&store, &["e0".to_string()])))
    });
}

criterion_group!(benches, bench_hybrid, bench_hnsw, bench_ppr);
criterion_main!(benches);
