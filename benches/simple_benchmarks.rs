// benches/simple_benchmarks.rs
//! Simple benchmarks for GaussOS core operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gaussos::{
    core::{MemCube, MemoryPayload},
    monitoring::MonitoringManager,
};

/// Benchmark memory creation operations
fn bench_memory_creation(c: &mut Criterion) {
    c.bench_function("create_memory_cube", |b| {
        b.iter(|| {
            let payload = MemoryPayload::Text(black_box("Test memory content".to_string()));
            let memory = MemCube::new(payload);
            black_box(memory)
        })
    });
}

/// Benchmark memory serialization
fn bench_memory_serialization(c: &mut Criterion) {
    let payload = MemoryPayload::Text("Test memory content for serialization".to_string());
    let memory = MemCube::new(payload);

    c.bench_function("serialize_memory", |b| {
        b.iter(|| {
            let serialized = serde_json::to_string(&memory).unwrap();
            black_box(serialized)
        })
    });
}

/// Benchmark memory deserialization
fn bench_memory_deserialization(c: &mut Criterion) {
    let payload = MemoryPayload::Text("Test memory content for deserialization".to_string());
    let memory = MemCube::new(payload);
    let serialized = serde_json::to_string(&memory).unwrap();

    c.bench_function("deserialize_memory", |b| {
        b.iter(|| {
            let deserialized: MemCube = serde_json::from_str(&serialized).unwrap();
            black_box(deserialized)
        })
    });
}

/// Benchmark monitoring operations
fn bench_monitoring_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let monitor = MonitoringManager::new();

    c.bench_function("collect_metrics", |b| {
        b.to_async(&rt).iter(|| async {
            let metrics = monitor.collect_metrics().await;
            black_box(metrics)
        })
    });

    c.bench_function("health_check", |b| {
        b.to_async(&rt).iter(|| async {
            let health = monitor.check_component_health("test_component").await;
            black_box(health)
        })
    });
}

/// Benchmark cosine similarity calculation
fn bench_cosine_similarity(c: &mut Criterion) {
    let embeddings_a: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
    let embeddings_b: Vec<f32> = (0..512).map(|i| (i + 100) as f32 / 512.0).collect();

    c.bench_function("cosine_similarity_512d", |b| {
        b.iter(|| {
            let similarity = gaussos::utils::cosine_similarity(
                black_box(&embeddings_a),
                black_box(&embeddings_b),
            );
            black_box(similarity)
        })
    });
}

/// Benchmark different payload types
fn bench_payload_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("payload_types");

    // Text payload
    group.bench_function("text_payload", |b| {
        b.iter(|| {
            let payload = MemoryPayload::Text(black_box("Simple text content".to_string()));
            let memory = MemCube::new(payload);
            black_box(memory)
        })
    });

    // Plaintext payload with embeddings
    group.bench_function("plaintext_payload", |b| {
        b.iter(|| {
            let payload = MemoryPayload::Plaintext {
                content: black_box("Plaintext with metadata".to_string()),
                encoding: "utf-8".to_string(),
                language: Some("en".to_string()),
                embeddings: Some(vec![0.1, 0.2, 0.3, 0.4, 0.5]),
            };
            let memory = MemCube::new(payload);
            black_box(memory)
        })
    });

    // Semantic payload
    group.bench_function("semantic_payload", |b| {
        b.iter(|| {
            let payload = MemoryPayload::Semantic {
                content: black_box("Semantic knowledge representation".to_string()),
                schema_type: gaussos::core::SemanticType::KnowledgeFact,
                confidence: 0.95,
                extracted_at: chrono::Utc::now(),
                source_context: "test context".to_string(),
                embeddings: Some(vec![0.1; 128]),
                validation_metadata: None,
            };
            let memory = MemCube::new(payload);
            black_box(memory)
        })
    });

    group.finish();
}

/// Benchmark memory fingerprinting
fn bench_memory_fingerprint(c: &mut Criterion) {
    let payload = MemoryPayload::Text("Test content for fingerprinting".to_string());
    let memory = MemCube::new(payload);

    c.bench_function("memory_fingerprint", |b| {
        b.iter(|| {
            let fingerprint = memory.fingerprint();
            black_box(fingerprint)
        })
    });
}

/// Benchmark bulk memory operations
fn bench_bulk_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_operations");

    for size in [10, 50, 100].iter() {
        group.bench_with_input(format!("create_{}_memories", size), size, |b, &size| {
            b.iter(|| {
                let memories: Vec<MemCube> = (0..size)
                    .map(|i| {
                        let payload = MemoryPayload::Text(format!("Bulk memory {}", i));
                        MemCube::new(payload)
                    })
                    .collect();
                black_box(memories)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_memory_creation,
    bench_memory_serialization,
    bench_memory_deserialization,
    bench_monitoring_operations,
    bench_cosine_similarity,
    bench_payload_types,
    bench_memory_fingerprint,
    bench_bulk_operations
);

criterion_main!(benches);
