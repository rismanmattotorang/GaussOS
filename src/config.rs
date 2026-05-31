// src/config.rs
//! Configuration Management for GaussOS
//!
//! Provides comprehensive configuration management for all GaussOS components
//! including database, API server, authentication, performance tuning, and enterprise features.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main GaussOS configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GaussOSConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// API configuration
    pub api: crate::api::ApiConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Performance configuration
    pub performance: PerformanceConfig,

    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,

    /// Server port
    pub port: u16,

    /// Enable production mode
    pub production: bool,

    /// Worker threads
    pub workers: Option<usize>,

    /// Maximum connections
    pub max_connections: u32,

    /// Keep alive timeout
    pub keep_alive: Duration,

    /// Request timeout
    pub request_timeout: Duration,

    /// Maximum request body size
    pub max_body_size: usize,

    /// Enable CORS
    pub enable_cors: bool,

    /// Enable request tracing
    pub enable_tracing: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type (hybrid, postgres, surreal)
    pub db_type: String,

    /// PostgreSQL connection string
    pub postgres_url: String,

    /// SurrealDB connection string
    pub surreal_url: String,

    /// Connection pool size
    pub pool_size: u32,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Query timeout
    pub query_timeout: Duration,

    /// Enable migrations
    pub enable_migrations: bool,

    /// Backup configuration
    pub backup: BackupConfig,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,

    /// Backup directory
    pub directory: PathBuf,

    /// Backup interval in hours
    pub interval_hours: u32,

    /// Number of backups to retain
    pub retention_count: u32,

    /// Enable compression
    pub compression: bool,

    /// Enable encryption
    pub encryption: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,

    /// JWT expiration time in seconds
    pub jwt_expiry_seconds: u64,

    /// Enable API key authentication
    pub enable_api_keys: bool,

    /// Enable session management
    pub enable_sessions: bool,

    /// Session timeout in seconds
    pub session_timeout_seconds: u64,

    /// Enable two-factor authentication
    pub enable_2fa: bool,

    /// Password requirements
    pub password: PasswordConfig,
}

/// Password configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordConfig {
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
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable SIMD optimizations
    pub enable_simd: bool,

    /// Enable parallel processing
    pub enable_parallel: bool,

    /// Number of worker threads
    pub worker_threads: usize,

    /// Memory cache size in MB
    pub cache_size_mb: u64,

    /// Enable compression
    pub enable_compression: bool,

    /// Compression level (1-9)
    pub compression_level: u8,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log format (json, plain)
    pub format: String,

    /// Log to file
    pub file: Option<PathBuf>,

    /// Log to console
    pub console: bool,

    /// Enable structured logging
    pub structured: bool,

    /// Log rotation
    pub rotation: LogRotationConfig,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Enable log rotation
    pub enabled: bool,

    /// Maximum file size in MB
    pub max_size_mb: u64,

    /// Maximum number of files
    pub max_files: u32,

    /// Rotation schedule (daily, weekly, monthly)
    pub schedule: String,
}



impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            production: false,
            workers: None,
            max_connections: 1000,
            keep_alive: Duration::from_secs(60),
            request_timeout: Duration::from_secs(30),
            max_body_size: 16 * 1024 * 1024, // 16MB
            enable_cors: true,
            enable_tracing: true,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: "hybrid".to_string(),
            postgres_url: "postgresql://localhost:5432/gaussos".to_string(),
            surreal_url: "memory".to_string(),
            pool_size: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            enable_migrations: true,
            backup: BackupConfig::default(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: PathBuf::from("./backups"),
            interval_hours: 24,
            retention_count: 7,
            compression: true,
            encryption: false,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change_me_in_production".to_string(),
            jwt_expiry_seconds: 3600, // 1 hour
            enable_api_keys: true,
            enable_sessions: true,
            session_timeout_seconds: 7200, // 2 hours
            enable_2fa: false,
            password: PasswordConfig::default(),
        }
    }
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_simd: true,
            enable_parallel: true,
            worker_threads: num_cpus::get(),
            cache_size_mb: 256,
            enable_compression: true,
            compression_level: 6,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            file: None,
            console: true,
            structured: true,
            rotation: LogRotationConfig::default(),
        }
    }
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: 100,
            max_files: 10,
            schedule: "daily".to_string(),
        }
    }
}

/// Load configuration from file
pub fn load_config(path: Option<PathBuf>) -> crate::Result<GaussOSConfig> {
    if let Some(config_path) = path {
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| {
                crate::error::GaussOSError::ConfigError(format!(
                    "Failed to read config file: {}",
                    e
                ))
            })?;

            let config: GaussOSConfig = toml::from_str(&content).map_err(|e| {
                crate::error::GaussOSError::ConfigError(format!("Failed to parse config: {}", e))
            })?;

            Ok(config)
        } else {
            Ok(GaussOSConfig::default())
        }
    } else {
        Ok(GaussOSConfig::default())
    }
}

/// Save configuration to file
pub fn save_config(config: &GaussOSConfig, path: &PathBuf) -> crate::Result<()> {
    let content = toml::to_string_pretty(config).map_err(|e| {
        crate::error::GaussOSError::ConfigError(format!("Failed to serialize config: {}", e))
    })?;

    std::fs::write(path, content).map_err(|e| {
        crate::error::GaussOSError::ConfigError(format!("Failed to write config file: {}", e))
    })?;

    Ok(())
}
