// src/api/mod.rs
//! REST API Module for GaussOS
//! Provides comprehensive HTTP endpoints for memory operations, authentication, and administration

pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod streaming;
pub mod graphql;

use crate::{
    database::DatabaseVault,
    error::{GaussOSError, Result},
    memory::manager::{MemoryManager, MemoryManagerConfig},
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, HeaderName, Method, StatusCode},
    middleware::{from_fn, from_fn_with_state},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use uuid::Uuid;

pub use handlers::*;

/// Enterprise API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Server host address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Enable TLS/SSL encryption
    pub tls: bool,

    /// TLS certificate file path
    pub cert: Option<PathBuf>,

    /// TLS private key file path
    pub key: Option<PathBuf>,

    /// Enable WebSocket support
    pub websocket: bool,

    /// Maximum concurrent connections
    pub max_connections: u32,

    /// Request timeout in seconds
    pub timeout: u64,

    /// Enable API rate limiting
    pub rate_limit: bool,

    /// Rate limit configuration
    pub rate_limit_config: RateLimitConfig,

    /// CORS configuration
    pub cors: CorsConfig,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Security headers configuration
    pub security: SecurityConfig,

    /// Performance optimization settings
    pub performance: PerformanceConfig,

    /// Enterprise features
    pub enterprise: EnterpriseConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitConfig {
    /// Requests per minute per IP
    pub requests_per_minute: u32,

    /// Requests per hour per API key
    pub requests_per_hour: u32,

    /// Burst allowance
    pub burst_size: u32,

    /// Rate limit storage (redis, memory)
    pub storage: String,

    /// Redis connection string for distributed rate limiting
    pub redis_url: Option<String>,

    /// Custom rate limit headers
    pub custom_headers: bool,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Exposed headers
    pub expose_headers: Vec<String>,

    /// Allow credentials
    pub allow_credentials: bool,

    /// Max age for preflight requests
    pub max_age: Duration,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,

    /// JWT expiration time
    pub jwt_expiry: Duration,

    /// Enable API key authentication
    pub api_keys: bool,

    /// Enable OAuth2
    pub oauth2: bool,

    /// OAuth2 providers
    pub oauth2_providers: Vec<OAuth2Provider>,

    /// Enable multi-factor authentication
    pub mfa: bool,

    /// Session timeout
    pub session_timeout: Duration,

    /// Password policy
    pub password_policy: PasswordPolicy,
}

/// OAuth2 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuth2Provider {
    /// Provider name
    pub name: String,

    /// Client ID
    pub client_id: String,

    /// Client secret
    pub client_secret: String,

    /// Authorization URL
    pub auth_url: String,

    /// Token URL
    pub token_url: String,

    /// Scopes
    pub scopes: Vec<String>,
}

/// Password policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PasswordPolicy {
    /// Minimum password length
    pub min_length: u8,

    /// Require uppercase letters
    pub require_uppercase: bool,

    /// Require lowercase letters
    pub require_lowercase: bool,

    /// Require numbers
    pub require_numbers: bool,

    /// Require special characters
    pub require_special: bool,

    /// Password expiry days
    pub expiry_days: Option<u32>,

    /// Password history count
    pub history_count: u8,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingConfig {
    /// Enable request logging
    pub requests: bool,

    /// Enable response logging
    pub responses: bool,

    /// Enable error logging
    pub errors: bool,

    /// Log level
    pub level: String,

    /// Log format (json, text)
    pub format: String,

    /// Log to file
    pub file: Option<PathBuf>,

    /// Log rotation configuration
    pub rotation: LogRotationConfig,

    /// Include sensitive data in logs
    pub include_sensitive: bool,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogRotationConfig {
    /// Maximum file size in MB
    pub max_size_mb: u64,

    /// Maximum number of files to keep
    pub max_files: u32,

    /// Rotation period (daily, weekly, monthly)
    pub period: String,

    /// Compression enabled
    pub compress: bool,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Enable security headers
    pub enabled: bool,

    /// Content Security Policy
    pub csp: Option<String>,

    /// HTTP Strict Transport Security
    pub hsts: bool,

    /// HSTS max age
    pub hsts_max_age: u32,

    /// X-Frame-Options
    pub frame_options: String,

    /// X-Content-Type-Options
    pub content_type_options: bool,

    /// Referrer Policy
    pub referrer_policy: String,

    /// Feature Policy
    pub feature_policy: Option<String>,
}

/// Performance optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceConfig {
    /// Enable response compression
    pub compression: bool,

    /// Compression level (1-9)
    pub compression_level: u8,

    /// Enable HTTP/2
    pub http2: bool,

    /// Keep-alive timeout
    pub keep_alive: Duration,

    /// Connection pool size
    pub connection_pool_size: u32,

    /// Request buffer size
    pub request_buffer_size: usize,

    /// Response buffer size
    pub response_buffer_size: usize,

    /// Enable caching
    pub caching: bool,

    /// Cache configuration
    pub cache_config: CacheConfig,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    /// Cache type (memory, redis, hybrid)
    pub cache_type: String,

    /// Cache size in MB
    pub size_mb: u64,

    /// Default TTL in seconds
    pub default_ttl: u64,

    /// Redis connection string
    pub redis_url: Option<String>,

    /// Enable cache compression
    pub compression: bool,
}

/// Enterprise features configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseConfig {
    /// Enable audit logging
    pub audit_logging: bool,

    /// Enable compliance monitoring
    pub compliance: bool,

    /// Enable advanced monitoring
    pub monitoring: bool,

    /// Enable distributed tracing
    pub tracing: bool,

    /// Enable metrics collection
    pub metrics: bool,

    /// Metrics configuration
    pub metrics_config: MetricsConfig,

    /// Enable health checks
    pub health_checks: bool,

    /// Health check configuration
    pub health_config: HealthConfig,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsConfig {
    /// Metrics endpoint path
    pub endpoint: String,

    /// Enable Prometheus metrics
    pub prometheus: bool,

    /// Metrics export interval
    pub export_interval: Duration,

    /// Custom metrics
    pub custom_metrics: Vec<String>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthConfig {
    /// Health check endpoint path
    pub endpoint: String,

    /// Check interval
    pub check_interval: Duration,

    /// Health check timeout
    pub timeout: Duration,

    /// Enabled health checks
    pub checks: Vec<String>,
}

/// Application state with enhanced features
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub database: Arc<dyn crate::database::MemVault>,

    /// Memory manager providing hybrid retrieval, forgetting, and temporal facts
    pub memory_manager: Arc<MemoryManager>,

    /// Application configuration
    pub config: Arc<crate::config::GaussOSConfig>,

    /// API metrics
    pub metrics: Arc<RwLock<ApiMetrics>>,

    /// Rate limiter
    pub rate_limiter: Option<Arc<RwLock<RateLimiter>>>,

    /// Cache manager
    pub cache: Option<Arc<RwLock<CacheManager>>>,

    /// Session manager
    pub sessions: Arc<RwLock<SessionManager>>,

    /// WebSocket connections
    pub websockets: Arc<RwLock<WebSocketManager>>,
}

/// Enhanced API metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiMetrics {
    /// Total requests
    pub total_requests: u64,

    /// Total responses
    pub total_responses: u64,

    /// Total errors
    pub total_errors: u64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// Requests per second
    pub requests_per_second: f64,

    /// Current active connections
    pub active_connections: u32,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Cache hit rate
    pub cache_hit_rate: f64,

    /// Rate limit violations
    pub rate_limit_violations: u64,

    /// Authentication failures
    pub auth_failures: u64,

    /// WebSocket connections
    pub websocket_connections: u32,

    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Rate limiter implementation
#[derive(Debug)]
pub struct RateLimiter {
    /// Rate limit entries
    pub entries: HashMap<String, RateLimitEntry>,

    /// Configuration
    pub config: RateLimitConfig,
}

impl RateLimiter {
    /// Check if request is allowed under rate limit
    pub async fn check_rate_limit(&self, identifier: &str) -> crate::error::Result<bool> {
        if let Some(entry) = self.entries.get(identifier) {
            let now = Utc::now();
            if now < entry.reset_at {
                return Ok(entry.remaining > 0);
            }
        }
        // If no entry or window expired, allow the request
        Ok(true)
    }
}

/// Rate limit entry
#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    /// Remaining requests
    pub remaining: u32,

    /// Reset timestamp
    pub reset_at: DateTime<Utc>,

    /// Last request timestamp
    pub last_request: DateTime<Utc>,
}

/// Cache manager
#[derive(Debug)]
pub struct CacheManager {
    /// Cache entries
    pub entries: HashMap<String, CacheEntry>,

    /// Configuration
    pub config: CacheConfig,
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached data
    pub data: Vec<u8>,

    /// Expiry timestamp
    pub expires_at: DateTime<Utc>,

    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,

    /// Access count
    pub access_count: u64,
}

/// Session manager
#[derive(Debug, Default)]
pub struct SessionManager {
    /// Active sessions
    pub sessions: HashMap<String, Session>,
}

/// User session
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID
    pub id: String,

    /// User ID
    pub user_id: Uuid,

    /// Session data
    pub data: HashMap<String, serde_json::Value>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Expires at timestamp
    pub expires_at: DateTime<Utc>,
}

/// WebSocket connection manager
#[derive(Debug, Default)]
pub struct WebSocketManager {
    /// Active connections
    pub connections: HashMap<String, WebSocketConnection>,
}

/// WebSocket connection
#[derive(Debug)]
pub struct WebSocketConnection {
    /// Connection ID
    pub id: String,

    /// User ID
    pub user_id: Option<Uuid>,

    /// Connection timestamp
    pub connected_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Subscribed channels
    pub channels: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            tls: false,
            cert: None,
            key: None,
            websocket: true,
            max_connections: 1000,
            timeout: 30,
            rate_limit: true,
            rate_limit_config: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            enterprise: EnterpriseConfig::default(),
        }
    }
}

/// Enhanced router creation function
pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        // Order middleware by frequency of execution (most common first)
        .layer(from_fn(middleware::request_id_middleware))
        .layer(from_fn_with_state(state.clone(), middleware::metrics_middleware))
        .layer(from_fn_with_state(state.clone(), middleware::auth_middleware))
        .layer(from_fn_with_state(state.clone(), middleware::rate_limit_middleware))
        // Configure CORS with optimized settings
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([
                    HeaderName::from_static("content-type"),
                    HeaderName::from_static("authorization"),
                    HeaderName::from_static("x-api-key"),
                ])
                .max_age(Duration::from_secs(3600)),
        ) // Cache preflight for 1 hour
        // Request size limits with different tiers
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB for file uploads
        // Timeout with graceful degradation
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        // Tracing with optimized field extraction
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::extract::Request<_>| {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    path = %request.uri().path(),
                    version = ?request.version(),
                )
            }),
        )
        // Routes organized by frequency (most used first)
        .route("/api/v1/memories", get(handlers::list_memories))
        .route("/api/v1/memories/:id", get(handlers::get_memory))
        .route("/api/v1/memories", post(handlers::create_memory))
        .route("/api/v1/memories/:id", put(handlers::update_memory))
        .route("/api/v1/memories/:id", delete(handlers::delete_memory))
        .route("/api/v1/memories/search", post(handlers::search_memories))
        // Approximate-nearest-neighbour (HNSW) vector search
        .route("/api/v1/memories/ann-search", post(handlers::ann_search))
        // Retrieval Playground: white-box lexical vs vector vs hybrid comparison
        .route("/api/v1/retrieval/compare", post(handlers::retrieval_compare))
        // Active LLM provider status (for the first-run wizard / settings)
        .route("/api/v1/llm/status", get(handlers::llm_status))
        // Bi-temporal knowledge graph + multi-hop graph retrieval
        .route("/api/v1/facts", post(handlers::ingest_fact))
        .route("/api/v1/facts/graph", get(handlers::facts_graph))
        .route("/api/v1/facts/graph-search", post(handlers::graph_search))
        .route("/api/v1/facts/:subject", get(handlers::get_facts))
        .route("/api/v1/facts/:subject/:predicate", get(handlers::get_fact_history))
        // Health and metrics endpoints (both root and /api/v1 for the web UI)
        .route("/health", get(handlers::health_check))
        .route("/metrics", get(handlers::metrics))
        .route("/api/v1/health", get(handlers::detailed_health_check))
        .route("/api/v1/metrics", get(handlers::metrics))
        // Administrative endpoints (less frequent)
        .route("/api/v1/admin/stats", get(handlers::admin_stats))
        .route("/api/v1/admin/backup", post(handlers::create_backup))
        .route("/api/v1/admin/forget", post(handlers::forget_memories))
        .with_state(state)
}

/// Enhanced server startup function
pub async fn start_server(config: ApiConfig) -> crate::Result<()> {
    // Default to the embedded SurrealDB backend (real SurrealQL engine, no
    // external server). Fall back to the in-memory vault if it cannot start.
    let database: Arc<dyn crate::database::MemVault> =
        match crate::database::SurrealVault::new("mem://").await {
            Ok(v) => Arc::new(v),
            Err(e) => {
                tracing::warn!("Embedded SurrealDB unavailable ({}); using in-memory vault", e);
                Arc::new(crate::database::InMemoryVault::new())
            }
        };
    let app_state = AppState {
        memory_manager: Arc::new(MemoryManager::new_optimized(
            database.clone(),
            MemoryManagerConfig::default(),
        )),
        database,
        config: Arc::new(crate::config::GaussOSConfig::default()),
        metrics: Arc::new(RwLock::new(ApiMetrics::default())),
        rate_limiter: None,
        cache: None,
        sessions: Arc::new(RwLock::new(SessionManager::default())),
        websockets: Arc::new(RwLock::new(WebSocketManager::default())),
    };

    let app = create_router(app_state.clone()).with_state(app_state.clone());

    let addr = format!("{}:{}", config.host, config.port);

    if config.tls {
        // TLS server implementation would go here
        println!("Starting TLS server on {}", addr);
    } else {
        // Regular HTTP server
        println!("Starting HTTP server on {}", addr);
    }

    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        crate::error::GaussOSError::NetworkError(format!("Failed to bind to {}: {}", addr, e))
    })?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::GaussOSError::NetworkError(format!("Server error: {}", e)))?;

    Ok(())
}

/// Example API server with configuration
pub async fn list_memories(_state: State<AppState>) -> Result<Json<Vec<serde_json::Value>>> {
    // Mock implementation
    let memories = vec![
        json!({
            "id": "mem_001",
            "content": "Sample memory content",
            "type": "semantic",
            "created_at": "2024-01-01T00:00:00Z"
        }),
        json!({
            "id": "mem_002",
            "content": "Another memory entry",
            "type": "procedural",
            "created_at": "2024-01-01T01:00:00Z"
        }),
    ];

    Ok(Json(memories))
}
