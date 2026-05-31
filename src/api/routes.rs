// src/api/routes.rs
//! API routes configuration
//! Defines all HTTP routes for the GaussOS REST API

use crate::api::{handlers, middleware, AppState};
use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
    Router,
};

/// Create the main API router with all routes configured
pub fn create_api_router(state: AppState) -> Router<AppState> {
    Router::new()
        // Health and status routes
        .route("/health", get(handlers::health_check))
        .route("/status", get(handlers::system_status))
        .route("/metrics", get(handlers::metrics))
        // Memory management routes
        .route("/memories", post(handlers::create_memory))
        .route("/memories", get(handlers::list_memories))
        .route("/memories/search", post(handlers::search_memories))
        .route("/memories/:id", get(handlers::get_memory))
        .route("/memories/:id", put(handlers::update_memory))
        .route("/memories/:id", delete(handlers::delete_memory))
        // Memory extraction routes
        .route("/memories/extract", post(handlers::extract_memories))
        // Bi-temporal knowledge-graph routes
        .route("/facts", post(handlers::ingest_fact))
        .route("/facts/:subject", get(handlers::get_facts))
        .route("/facts/:subject/:predicate", get(handlers::get_fact_history))
        // Graph routes
        .route("/graph/nodes", get(handlers::get_graph_nodes))
        .route("/graph/edges", get(handlers::get_graph_edges))
        .route("/graph/analyze", post(handlers::analyze_graph))
        // Authentication routes (optional)
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/refresh", post(handlers::refresh_token))
        // Admin routes
        .route("/admin/backup", post(handlers::create_backup))
        .route("/admin/restore", post(handlers::restore_backup))
        .route("/admin/optimize", post(handlers::optimize_database))
        .route("/admin/forget", post(handlers::forget_memories))
        // WebSocket routes
        .route("/ws", get(handlers::websocket_handler))
        // Add authentication middleware for protected routes
        .layer(from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        // Add request logging and metrics
        .layer(from_fn_with_state(state, middleware::logging_middleware))
}

/// Create admin-only routes
pub fn create_admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/system/shutdown", post(handlers::shutdown_system))
        .route("/system/restart", post(handlers::restart_system))
        .route("/system/config", get(handlers::get_system_config))
        .route("/system/config", put(handlers::update_system_config))
        .layer(from_fn_with_state(state, middleware::admin_auth_middleware))
}
