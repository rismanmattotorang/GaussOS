// src/server.rs
//! GaussOS Enterprise Server
//! High-performance server with full API, WebSocket, and streaming support

use crate::{
    api::{create_router, AppState, ApiConfig, ApiMetrics, RateLimiter, RateLimitConfig,
          CacheManager, CacheConfig, SessionManager, WebSocketManager, streaming},
    config::GaussOSConfig,
    database::{DatabaseFactory, DatabaseVault, HybridMemoryVault, MemVault},
    error::{GaussOSError, Result},
};
use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderName, Method},
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info, warn, error};

/// GaussOS Enterprise Server
/// Provides comprehensive API, WebSocket, and streaming capabilities
pub struct GaussOSServer {
    config: GaussOSConfig,
    api_config: ApiConfig,
    database: Arc<dyn MemVault>,
}

impl GaussOSServer {
    /// Create a new GaussOS server with default configuration
    pub async fn new(config: GaussOSConfig) -> Result<Self> {
        Self::with_api_config(config, ApiConfig::default()).await
    }

    /// Create a new GaussOS server with custom API configuration
    pub async fn with_api_config(config: GaussOSConfig, api_config: ApiConfig) -> Result<Self> {
        // Initialize database based on configuration
        let database = Self::initialize_database(&config).await?;
        
        Ok(Self {
            config,
            api_config,
            database,
        })
    }

    /// Initialize the appropriate database backend
    async fn initialize_database(config: &GaussOSConfig) -> Result<Arc<dyn MemVault>> {
        info!("Initializing database backend...");
        
        // Default to the embedded SurrealDB backend (real SurrealQL engine, no
        // external server required); fall back to the in-memory vault if it
        // cannot start, so the server always comes up and serves correctly.
        match crate::database::SurrealVault::new("mem://").await {
            Ok(vault) => {
                info!("Embedded SurrealDB vault initialized successfully");
                Ok(Arc::new(vault) as Arc<dyn MemVault>)
            }
            Err(e) => {
                warn!("Embedded SurrealDB unavailable ({}); using in-memory vault", e);
                Ok(Arc::new(crate::database::InMemoryVault::new()) as Arc<dyn MemVault>)
            }
        }
    }

    /// Build the complete router with all routes and middleware
    fn build_router(&self, app_state: AppState) -> Router {
        // Public routes (no authentication required)
        let public_routes = Router::new()
            .route("/health", get(crate::api::handlers::health_check))
            .route("/health/detailed", get(crate::api::handlers::detailed_health_check))
            .route("/health/live", get(crate::api::handlers::liveness_probe))
            .route("/health/ready", get(crate::api::handlers::readiness_probe))
            .route("/metrics", get(crate::api::handlers::metrics))
            .route("/status", get(crate::api::handlers::system_status));

        // API v1 routes
        let api_v1_routes = Router::new()
            // Memory management
            .route("/memories", get(crate::api::handlers::list_memories))
            .route("/memories", post(crate::api::handlers::create_memory))
            .route("/memories/search", post(crate::api::handlers::search_memories))
            .route("/memories/extract", post(crate::api::handlers::extract_memories))
            .route("/memories/:id", get(crate::api::handlers::get_memory))
            .route("/memories/:id", axum::routing::put(crate::api::handlers::update_memory))
            .route("/memories/:id", axum::routing::delete(crate::api::handlers::delete_memory))
            .route("/memories/:id/similar", get(crate::api::handlers::find_similar))
            .route("/memories/:id/snapshot", post(crate::api::handlers::create_snapshot))
            .route("/memories/:id/rollback", post(crate::api::handlers::rollback_memory))
            .route("/memories/:id/provenance", get(crate::api::handlers::get_provenance))
            // Approximate-nearest-neighbour vector search
            .route("/memories/ann-search", post(crate::api::handlers::ann_search))
            // Retrieval Playground (white-box lexical vs vector vs hybrid)
            .route("/retrieval/compare", post(crate::api::handlers::retrieval_compare))
            // Active LLM provider status
            .route("/llm/status", get(crate::api::handlers::llm_status))
            // Health/metrics under /api/v1 too (the web UI calls these)
            .route("/health", get(crate::api::handlers::detailed_health_check))
            .route("/metrics", get(crate::api::handlers::metrics))
            // Bi-temporal knowledge graph + multi-hop retrieval
            .route("/facts", post(crate::api::handlers::ingest_fact))
            .route("/facts/graph-search", post(crate::api::handlers::graph_search))
            .route("/facts/:subject", get(crate::api::handlers::get_facts))
            .route("/facts/:subject/:predicate", get(crate::api::handlers::get_fact_history))
            .route("/admin/forget", post(crate::api::handlers::forget_memories))
            // Graph operations
            .route("/graph/nodes", get(crate::api::handlers::get_graph_nodes))
            .route("/graph/edges", get(crate::api::handlers::get_graph_edges))
            .route("/graph/analyze", post(crate::api::handlers::analyze_graph))
            // Authentication
            .route("/auth/login", post(crate::api::handlers::login))
            .route("/auth/logout", post(crate::api::handlers::logout))
            .route("/auth/refresh", post(crate::api::handlers::refresh_token))
            // Admin operations
            .route("/admin/stats", get(crate::api::handlers::admin_stats))
            .route("/admin/backup", post(crate::api::handlers::create_backup))
            .route("/admin/restore", post(crate::api::handlers::restore_backup))
            .route("/admin/optimize", post(crate::api::handlers::optimize_database))
            .route("/admin/gc", post(crate::api::handlers::garbage_collect))
            // System
            .route("/system/config", get(crate::api::handlers::get_system_config))
            .route("/system/config", axum::routing::put(crate::api::handlers::update_system_config));

        // Streaming routes (SSE)
        let streaming_routes = Router::new()
            .route("/stream/metrics", get(streaming::metrics_stream))
            .route("/stream/memories", get(streaming::memory_events_stream))
            .route("/stream/agents", get(streaming::agent_events_stream))
            .route("/stream/dashboard", get(streaming::dashboard_stream));

        // WebSocket routes
        let ws_routes = Router::new()
            .route("/ws", get(crate::api::handlers::websocket_handler));

        // GraphQL routes (when feature is enabled)
        #[cfg(feature = "graphql")]
        let graphql_routes = crate::api::graphql::graphql_routes(app_state.clone());

        // Combine all routes
        let app = Router::new()
            // Public routes at root level
            .merge(public_routes)
            // API v1 routes with prefix
            .nest("/api/v1", api_v1_routes)
            // Streaming routes
            .nest("/api/v1", streaming_routes)
            // WebSocket routes
            .merge(ws_routes);
        
        // Add GraphQL routes when feature is enabled
        #[cfg(feature = "graphql")]
        let app = app.nest("/api/v1", graphql_routes);
        
        // Final app with middleware
        let app = app
            // Configure middleware layers (order matters - applied in reverse order)
            .layer(from_fn(crate::api::middleware::request_id_middleware))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::DELETE,
                        Method::OPTIONS,
                        Method::PATCH,
                    ])
                    .allow_headers([
                        HeaderName::from_static("content-type"),
                        HeaderName::from_static("authorization"),
                        HeaderName::from_static("x-api-key"),
                        HeaderName::from_static("x-request-id"),
                        HeaderName::from_static("x-csrf-token"),
                    ])
                    .expose_headers([
                        HeaderName::from_static("x-request-id"),
                        HeaderName::from_static("x-response-time"),
                    ])
                    .max_age(Duration::from_secs(3600)),
            )
            .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB
            .layer(TimeoutLayer::new(Duration::from_secs(self.api_config.timeout)))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &axum::extract::Request<_>| {
                        tracing::info_span!(
                            "http_request",
                            method = %request.method(),
                            path = %request.uri().path(),
                            request_id = tracing::field::Empty,
                        )
                    }),
            )
            .with_state(app_state);

        app
    }

    /// Start the GaussOS server
    pub async fn run(self) -> Result<()> {
        // Create rate limiter if enabled
        let rate_limiter = if self.api_config.rate_limit {
            Some(Arc::new(RwLock::new(RateLimiter {
                entries: std::collections::HashMap::new(),
                config: self.api_config.rate_limit_config.clone(),
            })))
        } else {
            None
        };

        // Create application state
        let app_state = AppState {
            memory_manager: Arc::new(crate::memory::manager::MemoryManager::new_optimized(
                self.database.clone(),
                crate::memory::manager::MemoryManagerConfig::default(),
            )),
            database: self.database.clone(),
            config: Arc::new(self.config.clone()),
            metrics: Arc::new(RwLock::new(ApiMetrics::default())),
            rate_limiter,
            cache: Some(Arc::new(RwLock::new(CacheManager {
                entries: std::collections::HashMap::new(),
                config: CacheConfig::default(),
            }))),
            sessions: Arc::new(RwLock::new(SessionManager::default())),
            websockets: Arc::new(RwLock::new(WebSocketManager::default())),
        };

        // Build router
        let app = self.build_router(app_state);

        // Bind to address
        let addr = format!("{}:{}", self.api_config.host, self.api_config.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            GaussOSError::NetworkError(format!("Failed to bind to {}: {}", addr, e))
        })?;

        // Print startup banner
        Self::print_startup_banner(&addr, &self.api_config);

        // Start the server
        info!("GaussOS server starting on http://{}", addr);
        
        axum::serve(listener, app)
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .map_err(|e| GaussOSError::NetworkError(format!("Server error: {}", e)))?;

        info!("GaussOS server shut down gracefully");
        Ok(())
    }

    /// Print startup banner with server information
    fn print_startup_banner(addr: &str, config: &ApiConfig) {
        let banner = format!(r#"
╔══════════════════════════════════════════════════════════════════╗
║                                                                  ║
║   🧠 GaussOS v{} - Enterprise AI Memory Platform             ║
║                                                                  ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║   Server:     http://{}                                   ║
║   Health:     http://{}/health                            ║
║   Metrics:    http://{}/metrics                           ║
║   API Docs:   http://{}/api/v1                            ║
║                                                                  ║
║   Features:                                                      ║
║   ✓ REST API with full CRUD operations                           ║
║   ✓ Server-Sent Events for real-time streaming                   ║
║   ✓ WebSocket support for bidirectional communication            ║
║   ✓ SIMD-accelerated vector operations                           ║
║   ✓ Lock-free concurrent data structures                         ║
║   ✓ Enterprise-grade security with JWT/API Keys                  ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
"#,
            env!("CARGO_PKG_VERSION"),
            addr,
            addr,
            addr,
            addr
        );
        println!("{}", banner);
    }

    /// Setup graceful shutdown handler
    async fn shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal, initiating shutdown...");
            },
            _ = terminate => {
                info!("Received terminate signal, initiating shutdown...");
            },
        }
    }
}

/// Quick server startup function for simple use cases
pub async fn start_server(config: ApiConfig) -> Result<()> {
    let server = GaussOSServer::with_api_config(
        GaussOSConfig::default(),
        config,
    ).await?;
    server.run().await
}

/// Start server with GaussOS configuration
pub async fn start_with_config(config: GaussOSConfig, api_config: ApiConfig) -> Result<()> {
    let server = GaussOSServer::with_api_config(config, api_config).await?;
    server.run().await
}