// tests/unit_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::{MemCube, MemoryPayload, Priority},
        database::{MemVault, PostgresVault},
        memory::MemoryManager,
    };
    use chrono::Utc;
    use std::sync::Arc;
    use tokio_test;
    use uuid::Uuid;

    fn create_test_memory() -> MemCube {
        let payload = MemoryPayload::Plaintext {
            content: "Test memory content".to_string(),
            encoding: "utf-8".to_string(),
            language: Some("en".to_string()),
            embeddings: None,
        };

        let mut memory = MemCube::new(payload);
        memory.metadata.name = Some("Test Memory".to_string());
        memory.metadata.description = Some("A test memory for unit testing".to_string());
        memory.metadata.tags = vec!["test".to_string(), "unit".to_string()];
        memory.metadata.priority = Priority::High;

        memory
    }

    #[tokio::test]
    async fn test_memory_cube_creation() {
        let memory = create_test_memory();

        assert!(memory.id != Uuid::nil());
        assert_eq!(memory.version, 1);
        assert!(!memory.is_expired());

        let fingerprint = memory.fingerprint();
        assert!(!fingerprint.is_empty());
        assert_eq!(fingerprint.len(), 16); // 64-bit hash as hex
    }

    #[tokio::test]
    async fn test_memory_cube_access_tracking() {
        let mut memory = create_test_memory();
        let initial_count = memory.metadata.access_count;
        let initial_time = memory.metadata.last_accessed;

        // Small delay to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        memory.increment_access();

        assert_eq!(memory.metadata.access_count, initial_count + 1);
        assert!(memory.metadata.last_accessed > initial_time);
    }

    #[tokio::test]
    async fn test_memory_expiration() {
        let mut memory = create_test_memory();

        // Memory without TTL should not expire
        assert!(!memory.is_expired());

        // Set TTL to 1 second ago
        memory.metadata.ttl = Some(1);
        memory.created_at = Utc::now() - chrono::Duration::seconds(2);

        assert!(memory.is_expired());
    }

    #[tokio::test]
    async fn test_different_payload_types() {
        // Test Parametric payload
        let parametric_payload = MemoryPayload::Parametric {
            model_type: "transformer".to_string(),
            layer_weights: vec![0.1, 0.2, 0.3, 0.4],
            bias_terms: Some(vec![0.01, 0.02]),
            activation_function: "relu".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let parametric_memory = MemCube::new(parametric_payload);
        assert!(matches!(
            parametric_memory.payload,
            MemoryPayload::Parametric { .. }
        ));

        // Test Activation payload
        let activation_payload = MemoryPayload::Activation {
            layer_name: "layer_1".to_string(),
            activation_values: vec![1.0, 2.0, 3.0],
            input_shape: vec![1, 3],
            timestamp: Utc::now(),
            context: Some("test context".to_string()),
        };

        let activation_memory = MemCube::new(activation_payload);
        assert!(matches!(
            activation_memory.payload,
            MemoryPayload::Activation { .. }
        ));

        // Test Plaintext payload
        let plaintext_payload = MemoryPayload::Plaintext {
            content: "Hello, world!".to_string(),
            encoding: "utf-8".to_string(),
            language: Some("en".to_string()),
            embeddings: Some(vec![0.1, 0.2, 0.3]),
        };

        let plaintext_memory = MemCube::new(plaintext_payload);
        assert!(matches!(
            plaintext_memory.payload,
            MemoryPayload::Plaintext { .. }
        ));
    }

    #[tokio::test]
    async fn test_memory_serialization() {
        let memory = create_test_memory();

        // Test JSON serialization
        let json = serde_json::to_string(&memory).expect("Failed to serialize memory");
        let deserialized: MemCube =
            serde_json::from_str(&json).expect("Failed to deserialize memory");

        assert_eq!(memory.id, deserialized.id);
        assert_eq!(memory.version, deserialized.version);
        assert_eq!(memory.metadata.name, deserialized.metadata.name);
    }

    #[tokio::test]
    async fn test_scheduler_priority_ordering() {
        use crate::scheduler::MemScheduler;

        let mut scheduler = MemScheduler::new();

        // Create memories with different priorities
        let mut low_priority = create_test_memory();
        low_priority.metadata.priority = Priority::Low;

        let mut high_priority = create_test_memory();
        high_priority.metadata.priority = Priority::High;

        let mut critical_priority = create_test_memory();
        critical_priority.metadata.priority = Priority::Critical;

        // Schedule memories
        scheduler.schedule_memory(&low_priority);
        scheduler.schedule_memory(&high_priority);
        scheduler.schedule_memory(&critical_priority);

        // Next memory should be critical priority
        let next_id = scheduler.get_next_memory().unwrap();
        assert_eq!(next_id, critical_priority.id);

        // Then high priority
        let next_id = scheduler.get_next_memory().unwrap();
        assert_eq!(next_id, high_priority.id);

        // Finally low priority
        let next_id = scheduler.get_next_memory().unwrap();
        assert_eq!(next_id, low_priority.id);
    }

    #[tokio::test]
    async fn test_lifecycle_state_transitions() {
        use crate::lifecycle::{MemLifecycle, MemoryState};

        let mut lifecycle = MemLifecycle::new();
        let memory_id = Uuid::new_v4();

        // Initial state
        lifecycle.set_state(memory_id, MemoryState::Active);
        assert!(matches!(
            lifecycle.get_state(&memory_id),
            Some(MemoryState::Active)
        ));

        // Transition to cached
        assert!(lifecycle.transition_to_cached(memory_id));
        assert!(matches!(
            lifecycle.get_state(&memory_id),
            Some(MemoryState::Cached)
        ));

        // Transition to archived
        assert!(lifecycle.transition_to_archived(memory_id));
        assert!(matches!(
            lifecycle.get_state(&memory_id),
            Some(MemoryState::Archived)
        ));

        // Mark as expired
        lifecycle.mark_expired(memory_id);
        assert!(matches!(
            lifecycle.get_state(&memory_id),
            Some(MemoryState::Expired)
        ));
    }
}
