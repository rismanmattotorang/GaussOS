// tests/performance_tests.rs
//! Performance tests for GaussOS enterprise features
//! Tests core performance, enterprise error handling, observability, and graph processing

#[cfg(test)]
mod performance_tests {
    use gaussos::{
        core::{MemCube, MemoryPayload},
        database::InMemoryVault,
        error::{ErrorRecoveryManager, GaussOSError},
        graph::{GraphEvent, RealtimeGraphProcessor},
        memory::MemoryManager,
        observability::ObservabilitySystem,
        performance::{AdaptiveOptimizer, RealtimeMonitor},
    };
    use std::{collections::HashMap, sync::Arc, time::Instant};
    use tokio::task::JoinSet;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_concurrent_memory_operations() {
        let vault = Arc::new(InMemoryVault::new());
        let memory_manager = Arc::new(MemoryManager::new(vault, 1000));

        let mut join_set = JoinSet::new();
        let num_operations = 1000; // Increased for enterprise testing

        let start_time = Instant::now();

        // Spawn concurrent memory creation tasks
        for i in 0..num_operations {
            let manager = memory_manager.clone();
            join_set.spawn(async move {
                let payload = MemoryPayload::Plaintext {
                    content: format!("Test memory content {}", i),
                    encoding: "utf-8".to_string(),
                    language: Some("en".to_string()),
                    embeddings: None,
                };

                let memory = MemCube::new(payload);
                manager.create_memory(memory).await
            });
        }

        // Wait for all operations to complete
        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            results.push(result.unwrap().unwrap());
        }

        let duration = start_time.elapsed();
        let ops_per_second = num_operations as f64 / duration.as_secs_f64();

        assert_eq!(results.len(), num_operations);
        println!(
            "Created {} memories in {:?} ({:.2} ops/sec)",
            num_operations, duration, ops_per_second
        );

        // Benchmark should achieve at least 50 ops/sec
        assert!(
            ops_per_second > 50.0,
            "Performance too slow: {:.2} ops/sec",
            ops_per_second
        );
    }

    #[tokio::test]
    async fn test_memory_cache_performance() {
        let vault = Arc::new(
            PostgresVault::new("postgresql://test:test@localhost/test_db")
                .await
                .unwrap(),
        );
        let memory_manager = Arc::new(MemoryManager::new(vault, 1000));

        // Create test memory
        let payload = MemoryPayload::Plaintext {
            content: "Cache performance test".to_string(),
            encoding: "utf-8".to_string(),
            language: Some("en".to_string()),
            embeddings: None,
        };

        let memory = MemCube::new(payload);
        let memory_id = memory_manager.create_memory(memory).await.unwrap();

        // Measure cache hit performance
        let start_time = Instant::now();
        let num_reads = 1000;

        for _ in 0..num_reads {
            let result = memory_manager.get_memory(&memory_id).await.unwrap();
            assert!(result.is_some());
        }

        let duration = start_time.elapsed();
        let reads_per_second = num_reads as f64 / duration.as_secs_f64();

        println!(
            "Performed {} cache reads in {:?} ({:.2} reads/sec)",
            num_reads, duration, reads_per_second
        );

        // Cache reads should be very fast
        assert!(
            reads_per_second > 1000.0,
            "Cache performance too slow: {:.2} reads/sec",
            reads_per_second
        );
    }

    #[tokio::test]
    async fn test_large_memory_handling() {
        let vault = Arc::new(
            PostgresVault::new("postgresql://test:test@localhost/test_db")
                .await
                .unwrap(),
        );
        let memory_manager = Arc::new(MemoryManager::new(vault, 100));

        // Create large memory with 10MB of data
        let large_content = "x".repeat(10 * 1024 * 1024); // 10MB
        let payload = MemoryPayload::Plaintext {
            content: large_content,
            encoding: "utf-8".to_string(),
            language: Some("en".to_string()),
            embeddings: None,
        };

        let memory = MemCube::new(payload);

        let start_time = Instant::now();
        let memory_id = memory_manager.create_memory(memory).await.unwrap();
        let create_duration = start_time.elapsed();

        println!("Created large memory (10MB) in {:?}", create_duration);

        // Retrieve large memory
        let start_time = Instant::now();
        let retrieved = memory_manager.get_memory(&memory_id).await.unwrap();
        let retrieve_duration = start_time.elapsed();

        println!("Retrieved large memory (10MB) in {:?}", retrieve_duration);

        assert!(retrieved.is_some());

        // Large memory operations should complete within reasonable time
        assert!(
            create_duration.as_secs() < 5,
            "Large memory creation too slow"
        );
        assert!(
            retrieve_duration.as_secs() < 2,
            "Large memory retrieval too slow"
        );
    }
}
