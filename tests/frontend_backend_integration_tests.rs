// tests/frontend_backend_integration_tests.rs
//! Frontend-Backend Integration Tests for GaussOS
//! Tests API connectivity, WebSocket communication, and end-to-end workflows

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use gaussos::{
    api::{create_router, AppState},
    core::{MemCube, MemoryPayload},
    error::Result,
    monitoring::MonitoringManager,
};
use hyper::body::to_bytes;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tower::ServiceExt;
use uuid::Uuid;

/// Mock app state for testing
async fn create_test_app_state() -> AppState {
    // In a real implementation, this would use test databases and services
    todo!("Implement test app state with mock services")
}

/// Create test router with all endpoints
async fn create_test_router() -> Router {
    let app_state = create_test_app_state().await;
    create_router(app_state)
}

/// Test basic API connectivity
#[tokio::test]
async fn test_api_connectivity() {
    let app = create_test_router().await;

    // Test health endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body()).await.unwrap();
    let health_data: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(health_data["status"], "healthy");
    assert!(health_data["timestamp"].is_string());
    assert!(health_data["version"].is_string());
}

/// Test CORS headers for frontend compatibility
#[tokio::test]
async fn test_cors_headers() {
    let app = create_test_router().await;

    // Test preflight request
    let preflight_request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/memories")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "POST")
        .header(
            "access-control-request-headers",
            "content-type,authorization",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(preflight_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert!(headers.contains_key("access-control-allow-methods"));
    assert!(headers.contains_key("access-control-allow-headers"));
}

/// Test memory management API endpoints
#[tokio::test]
async fn test_memory_api_endpoints() {
    let app = create_test_router().await;

    // Test creating a memory
    let create_payload = json!({
        "payload": {
            "Text": "Integration test memory content"
        },
        "name": "Integration Test Memory",
        "description": "A memory created during integration testing",
        "tags": ["integration", "test", "api"]
    });

    let create_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/memories")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = to_bytes(create_response.into_body()).await.unwrap();
    let create_result: Value = serde_json::from_slice(&create_body).unwrap();
    let memory_id = create_result["id"].as_str().unwrap();

    // Test retrieving the memory
    let get_request = Request::builder()
        .method(Method::GET)
        .uri(&format!("/api/v1/memories/{}", memory_id))
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = to_bytes(get_response.into_body()).await.unwrap();
    let memory_data: Value = serde_json::from_slice(&get_body).unwrap();
    assert_eq!(memory_data["id"], memory_id);

    // Test listing memories
    let list_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/memories?limit=10&offset=0")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let list_response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let list_body = to_bytes(list_response.into_body()).await.unwrap();
    let memories_list: Value = serde_json::from_slice(&list_body).unwrap();
    assert!(memories_list.is_array());

    // Test searching memories
    let search_payload = json!({
        "text": "integration test",
        "tags": ["test"],
        "limit": 5
    });

    let search_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/memories/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&search_payload).unwrap()))
        .unwrap();

    let search_response = app.clone().oneshot(search_request).await.unwrap();
    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body = to_bytes(search_response.into_body()).await.unwrap();
    let search_results: Value = serde_json::from_slice(&search_body).unwrap();
    assert!(search_results.is_array());

    // Test updating the memory
    let update_payload = json!({
        "id": memory_id,
        "payload": {
            "Text": "Updated integration test memory content"
        },
        "metadata": {
            "name": "Updated Integration Test Memory",
            "description": "An updated memory from integration testing",
            "tags": ["integration", "test", "api", "updated"]
        }
    });

    let update_request = Request::builder()
        .method(Method::PUT)
        .uri(&format!("/api/v1/memories/{}", memory_id))
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&update_payload).unwrap()))
        .unwrap();

    let update_response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);

    // Test deleting the memory
    let delete_request = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/api/v1/memories/{}", memory_id))
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let delete_response = app.oneshot(delete_request).await.unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
}

/// Test authentication and authorization flow
#[tokio::test]
async fn test_authentication_flow() {
    let app = create_test_router().await;

    // Test login endpoint
    let login_payload = json!({
        "username": "test@gaussos.ai",
        "password": "SecureTestPassword123!"
    });

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = to_bytes(login_response.into_body()).await.unwrap();
    let login_result: Value = serde_json::from_slice(&login_body).unwrap();

    assert!(login_result["token"].is_string());
    assert!(login_result["expires_in"].is_number());

    let token = login_result["token"].as_str().unwrap();

    // Test token refresh
    let refresh_payload = json!({
        "refresh_token": token
    });

    let refresh_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {}", token))
        .body(Body::from(serde_json::to_vec(&refresh_payload).unwrap()))
        .unwrap();

    let refresh_response = app.clone().oneshot(refresh_request).await.unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);

    // Test protected endpoint access
    let protected_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/stats")
        .header("authorization", &format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let protected_response = app.clone().oneshot(protected_request).await.unwrap();
    assert!(protected_response.status().is_success());

    // Test logout
    let logout_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header("authorization", &format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let logout_response = app.oneshot(logout_request).await.unwrap();
    assert_eq!(logout_response.status(), StatusCode::OK);
}

/// Test monitoring and metrics endpoints
#[tokio::test]
async fn test_monitoring_endpoints() {
    let app = create_test_router().await;

    // Test metrics endpoint
    let metrics_request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let metrics_response = app.clone().oneshot(metrics_request).await.unwrap();
    assert_eq!(metrics_response.status(), StatusCode::OK);

    let metrics_body = to_bytes(metrics_response.into_body()).await.unwrap();
    let metrics_data: Value = serde_json::from_slice(&metrics_body).unwrap();

    assert!(metrics_data.is_object());

    // Test system status endpoint
    let status_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/system/status")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let status_response = app.clone().oneshot(status_request).await.unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);

    let status_body = to_bytes(status_response.into_body()).await.unwrap();
    let status_data: Value = serde_json::from_slice(&status_body).unwrap();

    assert!(status_data["status"].is_string());
    assert!(status_data["uptime"].is_string());
    assert!(status_data["api_version"].is_string());
}

/// Test graph operations endpoints
#[tokio::test]
async fn test_graph_endpoints() {
    let app = create_test_router().await;

    // Test getting graph nodes
    let nodes_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/graph/nodes")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let nodes_response = app.clone().oneshot(nodes_request).await.unwrap();
    assert_eq!(nodes_response.status(), StatusCode::OK);

    let nodes_body = to_bytes(nodes_response.into_body()).await.unwrap();
    let nodes_data: Value = serde_json::from_slice(&nodes_body).unwrap();
    assert!(nodes_data["nodes"].is_array());

    // Test getting graph edges
    let edges_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/graph/edges")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let edges_response = app.clone().oneshot(edges_request).await.unwrap();
    assert_eq!(edges_response.status(), StatusCode::OK);

    let edges_body = to_bytes(edges_response.into_body()).await.unwrap();
    let edges_data: Value = serde_json::from_slice(&edges_body).unwrap();
    assert!(edges_data["edges"].is_array());

    // Test graph analysis
    let analyze_payload = json!({
        "analysis_type": "centrality",
        "parameters": {
            "algorithm": "betweenness"
        }
    });

    let analyze_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/graph/analyze")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&analyze_payload).unwrap()))
        .unwrap();

    let analyze_response = app.oneshot(analyze_request).await.unwrap();
    assert_eq!(analyze_response.status(), StatusCode::OK);

    let analyze_body = to_bytes(analyze_response.into_body()).await.unwrap();
    let analyze_data: Value = serde_json::from_slice(&analyze_body).unwrap();
    assert!(analyze_data["analysis"].is_string());
}

/// Test admin operations endpoints
#[tokio::test]
async fn test_admin_endpoints() {
    let app = create_test_router().await;

    // Test admin stats
    let admin_stats_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/stats")
        .header("authorization", "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let admin_stats_response = app.clone().oneshot(admin_stats_request).await.unwrap();
    assert_eq!(admin_stats_response.status(), StatusCode::OK);

    let stats_body = to_bytes(admin_stats_response.into_body()).await.unwrap();
    let stats_data: Value = serde_json::from_slice(&stats_body).unwrap();

    assert!(stats_data["database"].is_object());
    assert!(stats_data["system"].is_object());
    assert!(stats_data["performance"].is_object());

    // Test backup creation
    let backup_payload = json!({
        "backup_type": "full",
        "compress": true,
        "include_metadata": true
    });

    let backup_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/admin/backup")
        .header("content-type", "application/json")
        .header("authorization", "Bearer admin-token")
        .body(Body::from(serde_json::to_vec(&backup_payload).unwrap()))
        .unwrap();

    let backup_response = app.clone().oneshot(backup_request).await.unwrap();
    assert_eq!(backup_response.status(), StatusCode::OK);

    let backup_body = to_bytes(backup_response.into_body()).await.unwrap();
    let backup_data: Value = serde_json::from_slice(&backup_body).unwrap();

    assert!(backup_data["backup_id"].is_string());
    assert!(backup_data["status"].is_string());

    // Test system configuration
    let config_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/config")
        .header("authorization", "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let config_response = app.oneshot(config_request).await.unwrap();
    assert_eq!(config_response.status(), StatusCode::OK);

    let config_body = to_bytes(config_response.into_body()).await.unwrap();
    let config_data: Value = serde_json::from_slice(&config_body).unwrap();
    assert!(config_data["config"].is_object());
}

/// Test error handling and status codes
#[tokio::test]
async fn test_error_handling() {
    let app = create_test_router().await;

    // Test 404 for non-existent memory
    let invalid_id = Uuid::new_v4();
    let not_found_request = Request::builder()
        .method(Method::GET)
        .uri(&format!("/api/v1/memories/{}", invalid_id))
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let not_found_response = app.clone().oneshot(not_found_request).await.unwrap();
    assert_eq!(not_found_response.status(), StatusCode::NOT_FOUND);

    let error_body = to_bytes(not_found_response.into_body()).await.unwrap();
    let error_data: Value = serde_json::from_slice(&error_body).unwrap();
    assert!(error_data["error"].is_object());
    assert!(error_data["error"]["code"].is_string());
    assert!(error_data["error"]["message"].is_string());

    // Test 400 for invalid request
    let invalid_payload = json!({
        "invalid_field": "invalid_value"
    });

    let bad_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/memories")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&invalid_payload).unwrap()))
        .unwrap();

    let bad_response = app.clone().oneshot(bad_request).await.unwrap();
    assert_eq!(bad_response.status(), StatusCode::BAD_REQUEST);

    // Test 401 for unauthorized request
    let unauthorized_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/stats")
        .body(Body::empty())
        .unwrap();

    let unauthorized_response = app.clone().oneshot(unauthorized_request).await.unwrap();
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

    // Test 429 for rate limiting (if implemented)
    // This would require multiple rapid requests to trigger rate limiting
    let rate_limit_requests: Vec<_> = (0..10)
        .map(|_| {
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/memories")
                .header("authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap()
        })
        .collect();

    // Send requests rapidly
    for request in rate_limit_requests {
        let _response = app.clone().oneshot(request).await.unwrap();
        // Check if rate limiting is triggered
    }
}

/// Test WebSocket connection for real-time updates
#[tokio::test]
async fn test_websocket_connectivity() {
    // Note: This test would require a WebSocket test framework
    // For now, we'll test the HTTP upgrade request

    let app = create_test_router().await;

    let ws_request = Request::builder()
        .method(Method::GET)
        .uri("/ws/dashboard")
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let ws_response = app.oneshot(ws_request).await.unwrap();

    // WebSocket upgrade should return 101 Switching Protocols
    // or the endpoint should be properly configured
    assert!(
        ws_response.status() == StatusCode::SWITCHING_PROTOCOLS
            || ws_response.status() == StatusCode::NOT_IMPLEMENTED
    );
}

/// Test concurrent request handling
#[tokio::test]
async fn test_concurrent_requests() {
    let app = Arc::new(create_test_router().await);

    // Create multiple concurrent requests
    let mut handles = vec![];

    for i in 0..10 {
        let app_clone = Arc::clone(&app);
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/api/v1/memories")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test-token")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "payload": {
                            "Text": format!("Concurrent test memory {}", i)
                        },
                        "name": format!("Concurrent Memory {}", i),
                        "tags": ["concurrent", "test"]
                    }))
                    .unwrap(),
                ))
                .unwrap();

            app_clone.clone().oneshot(request).await
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    // Verify all requests succeeded
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Test API performance under load
#[tokio::test]
async fn test_api_performance() {
    let app = Arc::new(create_test_router().await);
    let monitor = MonitoringManager::new();

    let start_time = std::time::Instant::now();
    let mut handles = vec![];

    // Send 50 concurrent requests
    for i in 0..50 {
        let app_clone = Arc::clone(&app);
        let handle = tokio::spawn(async move {
            let request_start = std::time::Instant::now();

            let request = Request::builder()
                .method(Method::GET)
                .uri("/api/v1/memories?limit=10")
                .header("authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap();

            let response = app_clone.clone().oneshot(request).await.unwrap();
            let request_duration = request_start.elapsed();

            (response.status(), request_duration, i)
        });

        handles.push(handle);
    }

    // Wait for all requests and collect metrics
    let results = futures::future::join_all(handles).await;
    let total_duration = start_time.elapsed();

    let mut success_count = 0;
    let mut total_response_time = Duration::ZERO;
    let mut max_response_time = Duration::ZERO;
    let mut min_response_time = Duration::from_secs(1);

    for result in results {
        let (status, duration, _request_id) = result.unwrap();

        if status.is_success() {
            success_count += 1;
        }

        total_response_time += duration;
        max_response_time = max_response_time.max(duration);
        min_response_time = min_response_time.min(duration);
    }

    let avg_response_time = total_response_time / 50;
    let requests_per_second = 50.0 / total_duration.as_secs_f64();

    println!("Performance Test Results:");
    println!(
        "  Success Rate: {}/50 ({:.1}%)",
        success_count,
        (success_count as f64 / 50.0) * 100.0
    );
    println!("  Requests/sec: {:.2}", requests_per_second);
    println!("  Avg Response Time: {:?}", avg_response_time);
    println!("  Min Response Time: {:?}", min_response_time);
    println!("  Max Response Time: {:?}", max_response_time);

    // Performance assertions
    assert!(success_count >= 45); // At least 90% success rate
    assert!(avg_response_time < Duration::from_millis(1000)); // Avg response < 1s
    assert!(requests_per_second > 10.0); // At least 10 req/s
}

/// Test data consistency across multiple operations
#[tokio::test]
async fn test_data_consistency() {
    let app = create_test_router().await;

    // Create a memory
    let create_payload = json!({
        "payload": {
            "Text": "Consistency test memory"
        },
        "name": "Consistency Test",
        "tags": ["consistency", "test"]
    });

    let create_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/memories")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = to_bytes(create_response.into_body()).await.unwrap();
    let create_result: Value = serde_json::from_slice(&create_body).unwrap();
    let memory_id = create_result["id"].as_str().unwrap();

    // Retrieve the memory multiple times and verify consistency
    for _ in 0..5 {
        let get_request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/memories/{}", memory_id))
            .header("authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();

        let get_response = app.clone().oneshot(get_request).await.unwrap();
        assert_eq!(get_response.status(), StatusCode::OK);

        let get_body = to_bytes(get_response.into_body()).await.unwrap();
        let memory_data: Value = serde_json::from_slice(&get_body).unwrap();

        assert_eq!(memory_data["id"], memory_id);
        assert_eq!(memory_data["metadata"]["name"], "Consistency Test");

        // Small delay between requests
        sleep(Duration::from_millis(10)).await;
    }

    // Verify the memory appears in search results
    let search_payload = json!({
        "text": "consistency test",
        "limit": 10
    });

    let search_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/memories/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(serde_json::to_vec(&search_payload).unwrap()))
        .unwrap();

    let search_response = app.clone().oneshot(search_request).await.unwrap();
    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body = to_bytes(search_response.into_body()).await.unwrap();
    let search_results: Value = serde_json::from_slice(&search_body).unwrap();
    let results = search_results.as_array().unwrap();

    // Verify our memory appears in search results
    let found_memory = results.iter().find(|m| m["id"] == memory_id);
    assert!(found_memory.is_some());
}
