// GaussOS Integration Tests
// Tests for server-TUI-WebUI connectivity and API functionality
//
// Run with: cargo test --test integration_tests
// Run specific test: cargo test --test integration_tests test_name

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

const BASE_URL: &str = "http://localhost:8080";

/// Helper to create an HTTP client with sensible defaults
fn create_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

/// Wait for server to be ready
async fn wait_for_server(client: &Client, max_attempts: u32) -> bool {
    for i in 0..max_attempts {
        match client.get(format!("{}/health", BASE_URL)).send().await {
            Ok(resp) if resp.status().is_success() => return true,
            _ => {
                if i < max_attempts - 1 {
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
    false
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_health_check() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());
    
    let body: Value = resp.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "healthy");
    assert!(body["version"].is_string());
    assert!(body["uptime_seconds"].is_number());
}

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_detailed_health_check() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/health/detailed", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());
    
    let body: Value = resp.json().await.expect("Failed to parse JSON");
    assert!(body["checks"].is_object());
    assert!(body["metrics"].is_object());
}

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_liveness_probe() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/health/live", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());
    
    let body: Value = resp.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_readiness_probe() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/health/ready", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    // Either ready or not_ready, but should respond
    let status = resp.status();
    assert!(status.is_success() || status == 503);
}

// ============================================================================
// Memory API Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_memory_crud_operations() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    // Create a memory
    let create_payload = json!({
        "payload": {"Text": "Test memory content for RAG application"},
        "name": "Test Memory",
        "description": "Integration test memory",
        "tags": ["test", "integration"],
        "namespace": "test"
    });

    let create_resp = client
        .post(format!("{}/api/v1/memories", BASE_URL))
        .json(&create_payload)
        .send()
        .await
        .expect("Failed to create memory");

    assert!(create_resp.status().is_success());
    
    let created: Value = create_resp.json().await.expect("Failed to parse response");
    let memory_id = created["id"].as_str().expect("Missing memory ID");
    
    // Retrieve the memory
    let get_resp = client
        .get(format!("{}/api/v1/memories/{}", BASE_URL, memory_id))
        .send()
        .await
        .expect("Failed to get memory");

    assert!(get_resp.status().is_success());
    
    let retrieved: Value = get_resp.json().await.expect("Failed to parse response");
    assert_eq!(retrieved["id"], memory_id);

    // Update the memory
    let update_payload = json!({
        "payload": {"Text": "Updated test memory content"},
        "name": "Updated Test Memory"
    });

    let update_resp = client
        .put(format!("{}/api/v1/memories/{}", BASE_URL, memory_id))
        .json(&update_payload)
        .send()
        .await
        .expect("Failed to update memory");

    assert!(update_resp.status().is_success());

    // Delete the memory
    let delete_resp = client
        .delete(format!("{}/api/v1/memories/{}", BASE_URL, memory_id))
        .send()
        .await
        .expect("Failed to delete memory");

    assert!(delete_resp.status().is_success());

    // Verify deletion
    let verify_resp = client
        .get(format!("{}/api/v1/memories/{}", BASE_URL, memory_id))
        .send()
        .await
        .expect("Failed to send request");

    assert!(verify_resp.status() == 404);
}

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_memory_search() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    // Create test memories
    for i in 0..3 {
        let payload = json!({
            "payload": {"Text": format!("Searchable content {} for testing", i)},
            "name": format!("Search Test {}", i),
            "tags": ["search", "test"],
            "namespace": "search_test"
        });

        client
            .post(format!("{}/api/v1/memories", BASE_URL))
            .json(&payload)
            .send()
            .await
            .expect("Failed to create test memory");
    }

    // Search for memories
    let search_payload = json!({
        "text": "Searchable content",
        "namespace": "search_test",
        "limit": 10
    });

    let search_resp = client
        .post(format!("{}/api/v1/memories/search", BASE_URL))
        .json(&search_payload)
        .send()
        .await
        .expect("Failed to search memories");

    assert!(search_resp.status().is_success());
    
    let results: Value = search_resp.json().await.expect("Failed to parse response");
    assert!(results["memories"].is_array());
}

// ============================================================================
// Metrics and Status Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_metrics_endpoint() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/metrics", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());
}

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_system_status() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/status", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());
    
    let body: Value = resp.json().await.expect("Failed to parse JSON");
    assert!(body["status"].is_string());
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_authentication_flow() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    // Test login
    let login_payload = json!({
        "username": "test_user",
        "password": "test_password_12345"
    });

    let login_resp = client
        .post(format!("{}/api/v1/auth/login", BASE_URL))
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    // Either successful login or auth error is expected
    let status = login_resp.status();
    assert!(status.is_success() || status == 401);
}

// ============================================================================
// Server-Sent Events (SSE) Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_sse_metrics_stream() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let resp = client
        .get(format!("{}/api/v1/stream/metrics", BASE_URL))
        .send()
        .await
        .expect("Failed to connect to SSE stream");

    assert!(resp.status().is_success());
    // Note: Full SSE testing would require streaming the response
}

// ============================================================================
// GraphQL Tests (when enabled)
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server with graphql feature"]
async fn test_graphql_query() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let query = json!({
        "query": r#"
            {
                health {
                    status
                    version
                }
            }
        "#
    });

    let resp = client
        .post(format!("{}/api/v1/graphql", BASE_URL))
        .json(&query)
        .send()
        .await
        .expect("Failed to send GraphQL request");

    // GraphQL endpoint should respond (200 or 501 if feature disabled)
    let status = resp.status();
    assert!(status.is_success() || status == 501);
}

// ============================================================================
// TUI Integration Tests
// ============================================================================

/// Test that TUI can connect to the server health endpoint
#[tokio::test]
#[ignore = "Requires running server"]
async fn test_tui_server_connectivity() {
    let client = create_client();
    
    // Test the endpoints that TUI uses
    let endpoints = vec![
        "/health",
        "/metrics",
        "/api/v1/memories?limit=100",
        "/api/v1/agents",
    ];

    for endpoint in endpoints {
        let resp = client
            .get(format!("{}{}", BASE_URL, endpoint))
            .send()
            .await
            .expect(&format!("Failed to access endpoint: {}", endpoint));

        // Either success or expected error (404 for empty data)
        let status = resp.status();
        assert!(
            status.is_success() || status == 404,
            "Unexpected status for {}: {}",
            endpoint,
            status
        );
    }
}

// ============================================================================
// WebUI Integration Tests  
// ============================================================================

const WEBUI_URL: &str = "http://localhost:3000";

#[tokio::test]
#[ignore = "Requires running WebUI server"]
async fn test_webui_serves_html() {
    let client = create_client();

    let resp = client
        .get(WEBUI_URL)
        .send()
        .await
        .expect("Failed to access WebUI");

    assert!(resp.status().is_success());
    
    let content_type = resp.headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    
    assert!(content_type.contains("text/html"));
}

#[tokio::test]
#[ignore = "Requires running WebUI server"]
async fn test_webui_proxies_to_backend() {
    let client = create_client();

    // Test that WebUI proxies API requests to backend
    let resp = client
        .get(format!("{}/api/health", WEBUI_URL))
        .send()
        .await
        .expect("Failed to access WebUI API proxy");

    // Either proxied successfully or 503 if backend unavailable
    let status = resp.status();
    assert!(status.is_success() || status == 503);
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_concurrent_requests() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    let mut handles = vec![];

    // Spawn 10 concurrent health check requests
    for _ in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            client_clone
                .get(format!("{}/health", BASE_URL))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    // All requests should succeed
    assert!(results.iter().all(|&r| r), "Not all concurrent requests succeeded");
}

// ============================================================================
// End-to-End RAG Workflow Test
// ============================================================================

#[tokio::test]
#[ignore = "Requires running server"]
async fn test_rag_workflow() {
    let client = create_client();
    
    if !wait_for_server(&client, 5).await {
        eprintln!("Server not available, skipping test");
        return;
    }

    // 1. Create multiple memories simulating RAG knowledge base
    let knowledge_items = vec![
        "GaussOS is a memory-centric operating system for AI applications.",
        "It supports semantic, episodic, and procedural memory types.",
        "The system provides high-performance vector search capabilities.",
    ];

    let mut memory_ids = vec![];

    for (i, content) in knowledge_items.iter().enumerate() {
        let payload = json!({
            "payload": {"Text": content},
            "name": format!("Knowledge Item {}", i),
            "tags": ["knowledge", "rag", "test"],
            "namespace": "rag_test"
        });

        let resp = client
            .post(format!("{}/api/v1/memories", BASE_URL))
            .json(&payload)
            .send()
            .await
            .expect("Failed to create knowledge item");

        if resp.status().is_success() {
            let body: Value = resp.json().await.expect("Failed to parse response");
            if let Some(id) = body["id"].as_str() {
                memory_ids.push(id.to_string());
            }
        }
    }

    // 2. Perform RAG-style search
    let search_payload = json!({
        "text": "memory types",
        "namespace": "rag_test",
        "limit": 5
    });

    let search_resp = client
        .post(format!("{}/api/v1/memories/search", BASE_URL))
        .json(&search_payload)
        .send()
        .await
        .expect("Failed to search");

    assert!(search_resp.status().is_success());

    // 3. Clean up test data
    for id in memory_ids {
        let _ = client
            .delete(format!("{}/api/v1/memories/{}", BASE_URL, id))
            .send()
            .await;
    }
}

// ============================================================================
// Unit Test Examples (can run without server)
// ============================================================================

#[test]
fn test_json_parsing() {
    let json_str = r#"{"status": "healthy", "version": "3.0.0"}"#;
    let value: Value = serde_json::from_str(json_str).expect("Failed to parse JSON");
    
    assert_eq!(value["status"], "healthy");
    assert_eq!(value["version"], "3.0.0");
}

#[test]
fn test_memory_payload_serialization() {
    let payload = json!({
        "payload": {"Text": "Test content"},
        "name": "Test",
        "tags": ["tag1", "tag2"]
    });

    let serialized = serde_json::to_string(&payload).expect("Failed to serialize");
    assert!(serialized.contains("Test content"));
}
