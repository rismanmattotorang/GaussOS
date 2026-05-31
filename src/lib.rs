// src/lib.rs
//! # GaussOS - Advanced AI Memory Management System
//!
//! GaussOS is a high-performance, memory-centric operating system designed for AI applications.
//! It provides sophisticated memory management, graph-based relationships, and advanced
//! processing capabilities with support for multiple database backends.
//!
//! ## Features
//!
//! - **Advanced Memory Management**: Two-phase processing pipeline with SIMD acceleration
//! - **Hybrid Database Support**: PostgreSQL for relational data, SurrealDB for memory operations
//! - **Graph-Based Relationships**: Dynamic memory relationship graphs with real-time updates
//! - **Comprehensive Security**: JWT, API keys, RBAC, and audit logging
//! - **High Performance**: Lock-free operations, parallel processing, and optimized algorithms
//! - **Real-time Operations**: WebSocket support for live updates and notifications
//! - **Enterprise Ready**: Production-grade logging, monitoring, and administration tools
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gaussos::{
//!     GaussOS,
//!     database::{HybridConfig, DatabaseFactory},
//!     auth::AuthConfig,
//!     api::ApiConfig,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize GaussOS with hybrid database
//!     let hybrid_config = HybridConfig::default();
//!     let database = DatabaseFactory::create_hybrid_vault(hybrid_config).await?;
//!     
//!     // Start the system
//!     let gaussos = GaussOS::new(database).await?;
//!     gaussos.start().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! GaussOS follows a modular, layered architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     API Layer (REST/WebSocket)              │
//! ├─────────────────────────────────────────────────────────────┤
//! │                     Authentication & Authorization          │
//! ├─────────────────────────────────────────────────────────────┤
//! │    Memory Management    │    Graph Processing    │ Agents   │
//! ├─────────────────────────────────────────────────────────────┤
//! │              Performance Layer (SIMD, Parallel)            │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    Database Abstraction                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │         PostgreSQL          │         SurrealDB             │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)] // Allow temporarily unused functions during development
#![allow(unused_variables)] // Allow unused parameters in trait implementations
#![allow(unused_imports)] // Allow imports for future functionality
#![allow(clippy::too_many_arguments)] // Allow complex function signatures for enterprise features
#![allow(clippy::large_enum_variant)] // Allow large enum variants for comprehensive data structures
#![deny(unsafe_code)] // Enforce memory safety
#![allow(missing_docs)] // Temporarily allow missing documentation to keep build clean
#![warn(rust_2018_idioms)] // Encourage modern Rust patterns

// Core modules
pub mod config;
pub mod core;
pub mod error;

// Database and storage
pub mod database;

// Memory management
pub mod memory;

// Authentication and security
pub mod auth;

// API and networking
pub mod api;

// Server module
pub mod server;

// Graph processing (directory-based module)
pub mod graph;

// Performance optimization (directory-based module)
pub mod performance;

// Agent system
pub mod agents;

// System lifecycle
pub mod lifecycle;
pub mod scheduler;

// CLI interface
#[cfg(feature = "cli")]
pub mod cli;

// TUI Admin Application
#[cfg(feature = "tui")]
pub mod tui;

// Main GaussOS system
mod system;

// Re-export commonly used types
pub use config::GaussOSConfig;
pub use core::{
    ExtractedMemory, MemCube, MemoryMetadata, MemoryNamespace, MemoryPayload, Message, Priority,
    SemanticType,
};
pub use error::{GaussOSError, Result};
pub use system::GaussOS;

// Re-export database types
pub use database::{
    DatabaseConfig, DatabaseFactory, DatabaseVault, HybridConfig, HybridMemoryVault,
    HybridMetricsSnapshot, MemVault, SearchQuery, VaultStats,
};

// Re-export memory management
pub use memory::{
    ConsolidationReport,
    // Temporarily disabled advanced module exports
    // TwoPhaseMemoryProcessor, AdvancedMemoryOperation, ExtractionEngine,
    // UpdateEngine, ConflictResolver, QualityValidator, Conversation,
    // ProcessingContext, ProcessingResult
    ExtractionMode,
    MemoryExtractionRequest,
    MemoryExtractionResponse,
    MemoryManager,
    RustMemoryExtractor,
    SchemaRegistry,
};

// Re-export authentication
pub use auth::AuthService;
pub use config::AuthConfig;

// Re-export server
pub use server::{GaussOSServer, start_server, start_with_config};

// Re-export API types
pub use api::{create_router, ApiConfig, ApiMetrics, AppState};

// Re-export graph processing
pub use graph::{
    CentralityType, Community, GraphAnalytics, GraphConfig, GraphEdge, GraphEvent, GraphMetrics,
    GraphNode, MemoryGraph,
};

// Re-export performance utilities
pub use performance::{
    AtomicMetrics,
    LockFreeMemoryCache,
    MetricsSnapshot,
    // ParallelMemoryProcessor, ProcessingResult,  // Removed due to missing parallel module
    SimdSimilarity,
    VectorizedOperations,
};

// Re-export agent system (placeholder for now)
// pub use agents::{
//     AgentTool, ToolRegistry, ToolPermissions, MemoryTools,
//     MemorySummary, MemoryType
// };

/// Current version of GaussOS
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build information - lazy static to handle option unwrapping
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: VERSION,
        git_hash: option_env!("GIT_HASH").unwrap_or("unknown"),
        build_date: option_env!("BUILD_DATE").unwrap_or("unknown"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
    }
}

/// Build information structure
#[derive(Debug, Clone)]
pub struct BuildInfo {
    /// Version string
    pub version: &'static str,
    /// Git commit hash
    pub git_hash: &'static str,
    /// Build date
    pub build_date: &'static str,
    /// Rust version used for build
    pub rust_version: &'static str,
}

/// Initialize GaussOS with default configuration
///
/// This is a convenience function that sets up GaussOS with sensible defaults.
/// For production use, create your own configuration.
///
/// # Example
///
/// ```rust,no_run
/// use gaussos;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let gaussos = gaussos::init_default().await?;
///     gaussos.start().await?;
///     Ok(())
/// }
/// ```
pub async fn init_default() -> Result<GaussOS> {
    let config = GaussOSConfig::default();
    let database = DatabaseFactory::create_hybrid_vault().await?;
    GaussOS::new_with_config(DatabaseVault::Hybrid(database), config).await
}

/// Initialize GaussOS with custom configuration
///
/// # Arguments
///
/// * `config` - GaussOS configuration
///
/// # Example
///
/// ```rust,no_run
/// use gaussos::{GaussOSConfig, database::DatabaseFactory, database::DatabaseVault};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut config = GaussOSConfig::default();
///     config.api.port = 9090;
///     
///     let database = DatabaseFactory::create_hybrid_vault().await?;
///     let gaussos = gaussos::init_with_config(DatabaseVault::Hybrid(database), config).await?;
///     gaussos.start().await?;
///     Ok(())
/// }
/// ```
pub async fn init_with_config(database: DatabaseVault, config: GaussOSConfig) -> Result<GaussOS> {
    GaussOS::new_with_config(database, config).await
}

/// Utility functions and helpers
pub mod utils {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Generate a unique request ID
    pub fn generate_request_id() -> String {
        use uuid::Uuid;
        Uuid::new_v4().to_string()
    }

    /// Get current timestamp in milliseconds
    pub fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Get current timestamp in seconds
    pub fn current_timestamp_s() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Convert bytes to human-readable format
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
        const THRESHOLD: u64 = 1024;

        if bytes < THRESHOLD {
            return format!("{} B", bytes);
        }

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD as f64;
            unit_index += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_index])
    }

    /// Format duration in human-readable format
    pub fn format_duration(duration: std::time::Duration) -> String {
        let total_seconds = duration.as_secs();

        if total_seconds < 60 {
            format!("{}s", total_seconds)
        } else if total_seconds < 3600 {
            let minutes = total_seconds / 60;
            let seconds = total_seconds % 60;
            format!("{}m {}s", minutes, seconds)
        } else if total_seconds < 86400 {
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            format!("{}h {}m", hours, minutes)
        } else {
            let days = total_seconds / 86400;
            let hours = (total_seconds % 86400) / 3600;
            format!("{}d {}h", days, hours)
        }
    }

    /// Validate memory namespace
    pub fn validate_namespace(namespace: &str) -> Result<()> {
        if namespace.is_empty() {
            return Err(GaussOSError::ValidationError(
                "Namespace cannot be empty".to_string(),
            ));
        }

        if namespace.len() > 255 {
            return Err(GaussOSError::ValidationError(
                "Namespace too long".to_string(),
            ));
        }

        // Check for valid characters (alphanumeric, dash, underscore, dot)
        if !namespace
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(GaussOSError::ValidationError(
                "Namespace contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate memory payload size
    pub fn validate_payload_size(size: usize, max_size: usize) -> Result<()> {
        if size > max_size {
            return Err(GaussOSError::ValidationError(format!(
                "Payload size {} exceeds maximum {}",
                format_bytes(size as u64),
                format_bytes(max_size as u64)
            )));
        }
        Ok(())
    }

    /// Calculate similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

/// Testing utilities for development and integration tests
#[cfg(feature = "testing")]
pub mod testing {
    use super::*;

    /// Create a test GaussOS instance with in-memory database
    pub async fn create_test_gaussos() -> Result<GaussOS> {
        let config = GaussOSConfig::default();
        let database = DatabaseFactory::create_postgres_vault(&config.database.postgres_url).await?;
        GaussOS::new_with_config(database::DatabaseVault::Postgres(database), config).await
    }

    /// Create test memory cubes for testing
    pub fn create_test_memory_cubes(count: usize) -> Vec<MemCube> {
        (0..count)
            .map(|i| {
                let payload = MemoryPayload::Text(format!("Test memory content {}", i));
                let mut cube = MemCube::new(payload);
                cube.metadata.name = Some(format!("test_memory_{}", i));
                cube.metadata.tags = vec![format!("test"), format!("memory_{}", i)];
                cube.metadata.priority = Priority::Medium;
                cube.namespace = MemoryNamespace("test".to_string());
                cube
            })
            .collect()
    }

    /// Assert that two memory cubes are equal (ignoring timestamps)
    pub fn assert_memory_equal(a: &MemCube, b: &MemCube) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.metadata.name, b.metadata.name);
        assert_eq!(a.metadata.tags, b.metadata.tags);
        assert_eq!(a.namespace, b.namespace);
        assert_eq!(a.payload, b.payload);
        assert_eq!(a.version, b.version);
    }
}

// Feature flags for conditional compilation
#[cfg(feature = "web-ui")]
pub mod web_ui {
    //! Web UI components for GaussOS administration
    // Note: Implementation would be added when web-ui feature is developed
}

#[cfg(feature = "metrics")]
pub mod metrics {
    //! Advanced metrics and monitoring
    // Note: Implementation would be added when metrics feature is developed
}

pub mod monitoring;
pub mod observability;
