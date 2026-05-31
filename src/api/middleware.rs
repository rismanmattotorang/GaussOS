// src/api/middleware.rs
//! Enterprise middleware for GaussOS API
//! Provides authentication, rate limiting, logging, and security features

use crate::{
    api::AppState,
    error::{GaussOSError, Result},
};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            requests_per_hour: 1000,
            burst_size: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

/// Rate limiter implementation
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check_rate_limit(&self, identifier: &str) -> Result<bool> {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window_start = now - self.config.window_size;

        // Get or create request history for this identifier
        let request_history = requests.entry(identifier.to_string()).or_insert_with(Vec::new);

        // Remove old requests outside the window
        request_history.retain(|&timestamp| timestamp > window_start);

        // Check if we're within the rate limit
        if request_history.len() >= self.config.requests_per_minute as usize {
            return Ok(false);
        }

        // Add current request
        request_history.push(now);
        Ok(true)
    }

    pub async fn cleanup_old_requests(&self) {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window_start = now - self.config.window_size;

        // Remove old entries
        requests.retain(|_, history| {
            history.retain(|&timestamp| timestamp > window_start);
            !history.is_empty()
        });
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> std::result::Result<Response, GaussOSError> {
    // Extract authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Check for API key
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Validate authentication
    if !auth_header.starts_with("Bearer ") && api_key.is_empty() {
        return Err(GaussOSError::AuthenticationFailed {
            reason: "Missing authentication token".to_string(),
            context: None,
        });
    }

    // TODO: Implement proper JWT validation
    // For now, accept any non-empty token
    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        api_key
    };

    if token.is_empty() {
        return Err(GaussOSError::AuthenticationFailed {
            reason: "Invalid authentication token".to_string(),
            context: None,
        });
    }

    // Add user context to request
    let mut request = request;
    request.extensions_mut().insert(UserContext {
        user_id: Uuid::new_v4(), // TODO: Extract from JWT
        permissions: vec!["read".to_string(), "write".to_string()],
        api_key: api_key.to_string(),
    });

    Ok(next.run(request).await)
}

/// Admin authentication middleware
pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> std::result::Result<Response, GaussOSError> {
    // First run regular auth middleware
    let response = auth_middleware(State(state), headers, request, next).await?;

    // TODO: Check for admin permissions
    // For now, allow all authenticated requests
    Ok(response)
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> std::result::Result<Response, GaussOSError> {
    // Get client identifier (IP address or API key)
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let identifier = if !api_key.is_empty() {
        api_key
    } else {
        client_ip
    };

    // Check rate limit
    if let Some(rate_limiter) = &state.rate_limiter {
        let limiter = rate_limiter.read().await;
        match limiter.check_rate_limit(identifier).await {
            Ok(allowed) if !allowed => {
                return Err(GaussOSError::RateLimitExceeded(
                    format!("Rate limit exceeded. Retry after {} seconds", 60)
                ));
            }
            Err(e) => {
                return Err(e);
            }
            _ => {}
        }
    }

    Ok(next.run(request).await)
}

/// Request ID middleware
pub async fn request_id_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // Generate or extract request ID
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add request ID to request extensions
    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        start_time: Instant::now(),
    });

    let mut response = next.run(request).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    response
}

/// Metrics middleware
pub async fn metrics_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    // Record metrics
    let duration = start_time.elapsed();
    let status = response.status();

    // Update metrics in state
    {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests += 1;
        metrics.total_responses += 1;
        
        // Update average response time
        let new_avg = (metrics.avg_response_time_ms + duration.as_millis() as f64) / 2.0;
        metrics.avg_response_time_ms = new_avg;
        
        // Track errors
        if status.is_server_error() || status.is_client_error() {
            metrics.total_errors += 1;
        }
        
        // Update last_updated timestamp
        metrics.last_updated = chrono::Utc::now();
    }

    response
}

/// Logging middleware
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start_time = Instant::now();

    // Log request
    tracing::info!(
        method = %method,
        path = %path,
        "Incoming request"
    );

    let response = next.run(request).await;

    // Log response
    let duration = start_time.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        path = %path,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

/// CORS middleware
pub async fn cors_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    // Add CORS headers
    response.headers_mut().insert(
        "access-control-allow-origin",
        HeaderValue::from_static("*"),
    );
    response.headers_mut().insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    response.headers_mut().insert(
        "access-control-allow-headers",
        HeaderValue::from_static("Content-Type, Authorization, X-API-Key, X-Request-ID"),
    );
    response.headers_mut().insert(
        "access-control-max-age",
        HeaderValue::from_static("86400"),
    );

    response
}

/// Security headers middleware
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    // Add security headers
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        "x-frame-options",
        HeaderValue::from_static("DENY"),
    );
    response.headers_mut().insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );
    response.headers_mut().insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    response.headers_mut().insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline';"),
    );

    response
}

/// Request validation middleware
pub async fn validation_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Check request size
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                const MAX_REQUEST_SIZE: usize = 100 * 1024 * 1024; // 100MB
                if length > MAX_REQUEST_SIZE {
                    return Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .body(axum::body::Body::from("Request too large"))
                        .unwrap();
                }
            }
        }
    }

    // Check for required headers
    let user_agent = request.headers().get("user-agent");
    if user_agent.is_none() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(axum::body::Body::from("User-Agent header required"))
            .unwrap();
    }

    next.run(request).await
}

/// Error handling middleware
pub async fn error_handling_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Extract path before consuming request
    let path = request.uri().path().to_string();
    
    match next.run(request).await {
        response if response.status().is_success() => response,
        response => {
            // Log error responses
            let status = response.status();
            
            if status.is_server_error() {
                tracing::error!(
                    status = %status,
                    path = %path,
                    "Server error occurred"
                );
            } else if status.is_client_error() {
                tracing::warn!(
                    status = %status,
                    path = %path,
                    "Client error occurred"
                );
            }

            response
        }
    }
}

/// Request context for tracking
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub start_time: Instant,
}

/// User context for authentication
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Uuid,
    pub permissions: Vec<String>,
    pub api_key: String,
}

/// API metrics
#[derive(Debug)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub request_duration_ms: u64,
    pub status_codes: HashMap<String, u64>,
    pub endpoint_usage: HashMap<String, u64>,
    pub error_count: u64,
    pub last_updated: std::time::Instant,
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            request_duration_ms: 0,
            status_codes: HashMap::new(),
            endpoint_usage: HashMap::new(),
            error_count: 0,
            last_updated: std::time::Instant::now(),
        }
    }
}

impl ApiMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }

        let success_count = self.status_codes
            .iter()
            .filter(|(status, _)| {
                status.parse::<u16>()
                    .map(|s| s < 400)
                    .unwrap_or(false)
            })
            .map(|(_, count)| count)
            .sum::<u64>();

        success_count as f64 / self.total_requests as f64
    }

    pub fn get_average_response_time(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.request_duration_ms as f64 / self.total_requests as f64
    }
}

/// Session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub user_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub permissions: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    pub fn new(user_id: Uuid, permissions: Vec<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            expires_at: now + chrono::Duration::hours(1), // 1 hour
            permissions,
            metadata: HashMap::new(),
        }
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    pub fn extend(&mut self, hours: i64) {
        self.expires_at = chrono::Utc::now() + chrono::Duration::hours(hours);
    }
}

/// Session manager
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, user_id: Uuid, permissions: Vec<String>) -> Session {
        let session = Session::new(user_id, permissions);
        let session_id = session.session_id.clone();
        
        self.sessions.write().await.insert(session_id, session.clone());
        session
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| !session.is_expired());
    }

    pub async fn get_active_sessions_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

/// WebSocket connection manager
#[derive(Debug, Default)]
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
}

#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub connection_id: String,
    pub user_id: Option<Uuid>,
    pub connected_at: std::time::Instant,
    pub last_activity: std::time::Instant,
    pub subscriptions: Vec<String>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, connection_id: String, user_id: Option<Uuid>) {
        let connection = WebSocketConnection {
            connection_id: connection_id.clone(),
            user_id,
            connected_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            subscriptions: Vec::new(),
        };

        self.connections.write().await.insert(connection_id, connection);
    }

    pub async fn remove_connection(&self, connection_id: &str) {
        self.connections.write().await.remove(connection_id);
    }

    pub async fn update_activity(&self, connection_id: &str) {
        if let Some(connection) = self.connections.write().await.get_mut(connection_id) {
            connection.last_activity = std::time::Instant::now();
        }
    }

    pub async fn get_active_connections_count(&self) -> usize {
        self.connections.read().await.len()
    }

    pub async fn cleanup_inactive_connections(&self, timeout: Duration) {
        let now = std::time::Instant::now();
        let mut connections = self.connections.write().await;
        connections.retain(|_, conn| now - conn.last_activity < timeout);
    }
}

/// CSRF protection middleware
pub async fn csrf_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> std::result::Result<Response, GaussOSError> {
    // Skip CSRF check for GET, HEAD, OPTIONS (safe methods)
    let method = request.method().clone();
    if method == axum::http::Method::GET 
        || method == axum::http::Method::HEAD 
        || method == axum::http::Method::OPTIONS {
        return Ok(next.run(request).await);
    }
    
    // Check for CSRF token in header
    let csrf_token = headers
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    // Check origin header
    let origin = headers
        .get("Origin")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    // For API requests with proper authentication, we allow through
    let has_auth = headers.contains_key("Authorization") || headers.contains_key("X-API-Key");
    
    if has_auth {
        return Ok(next.run(request).await);
    }
    
    // Validate origin for non-API requests
    let allowed_origins = ["http://localhost:3000", "http://localhost:8080", "https://gaussos.local"];
    
    if !origin.is_empty() && !allowed_origins.iter().any(|o| origin.starts_with(o)) {
        tracing::warn!(origin = %origin, "CSRF check failed: invalid origin");
        return Err(GaussOSError::AuthorizationFailed {
            resource: "csrf".to_string(),
            reason: "Invalid origin".to_string(),
            context: None,
        });
    }
    
    Ok(next.run(request).await)
}

/// Input sanitization utility
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(10000) // Max 10k characters
        .collect::<String>()
        .trim()
        .to_string()
}

/// Validate UUID format
pub fn validate_uuid(id: &str) -> Result<Uuid> {
    Uuid::parse_str(id).map_err(|_| GaussOSError::ValidationError("Invalid UUID format".to_string()))
}
