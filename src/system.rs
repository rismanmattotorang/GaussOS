// src/system.rs
//! Core GaussOS system implementation
//! Orchestrates all components and provides the main system interface

use crate::{
    config::GaussOSConfig,
    database::{DatabaseVault, MemVault},
    error::Result,
    lifecycle::LifecycleManager,
    scheduler::TaskScheduler,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Main GaussOS system orchestrator
pub struct GaussOS {
    /// Configuration
    config: Arc<GaussOSConfig>,

    /// Database layer
    database: Arc<DatabaseVault>,

    /// System lifecycle manager
    lifecycle: Arc<LifecycleManager>,

    /// Task scheduler
    scheduler: Arc<TaskScheduler>,

    /// System state
    state: Arc<RwLock<SystemState>>,
}

#[derive(Debug, Clone)]
pub struct SystemState {
    pub status: SystemStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub last_health_check: DateTime<Utc>,
    pub component_health: ComponentHealth,
}

#[derive(Debug, Clone)]
pub enum SystemStatus {
    Initializing,
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub database: HealthStatus,
    pub api: HealthStatus,
    pub scheduler: HealthStatus,
    pub memory_manager: HealthStatus,
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Down,
}

#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub database_status: HealthStatus,
    pub uptime: std::time::Duration,
    pub last_check: DateTime<Utc>,
}

impl GaussOS {
    /// Create new GaussOS instance
    pub async fn new(database: DatabaseVault) -> Result<Self> {
        let config = Arc::new(GaussOSConfig::default());
        Self::new_with_config(database, config.as_ref().clone()).await
    }

    /// Create new GaussOS instance with custom configuration
    pub async fn new_with_config(database: DatabaseVault, config: GaussOSConfig) -> Result<Self> {
        let config = Arc::new(config);
        let database = Arc::new(database);

        // Initialize components
        let lifecycle = Arc::new(LifecycleManager::new(database.clone()));
        let scheduler = Arc::new(TaskScheduler::new(database.clone()));

        let state = Arc::new(RwLock::new(SystemState {
            status: SystemStatus::Initializing,
            started_at: None,
            last_health_check: Utc::now(),
            component_health: ComponentHealth {
                database: HealthStatus::Down,
                api: HealthStatus::Down,
                scheduler: HealthStatus::Down,
                memory_manager: HealthStatus::Down,
            },
        }));

        Ok(Self {
            config,
            database,
            lifecycle,
            scheduler,
            state,
        })
    }

    /// Start the GaussOS system
    pub async fn start(&self) -> Result<()> {
        info!("Starting GaussOS system...");

        // Update state
        {
            let mut state = self.state.write().await;
            state.status = SystemStatus::Starting;
        }

        // Components will be started when needed
        // TODO: Add proper component initialization

        // Update state
        {
            let mut state = self.state.write().await;
            state.status = SystemStatus::Running;
            state.started_at = Some(Utc::now());
        }

        info!("GaussOS system started successfully");
        Ok(())
    }

    /// Stop the GaussOS system gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Stopping GaussOS system...");

        // Update state
        {
            let mut state = self.state.write().await;
            state.status = SystemStatus::Stopping;
        }

        // Components will be stopped gracefully
        // TODO: Add proper component shutdown

        // Update state
        {
            let mut state = self.state.write().await;
            state.status = SystemStatus::Stopped;
        }

        info!("GaussOS system stopped");
        Ok(())
    }

    /// Stop the GaussOS system
    pub async fn stop(&self) -> Result<()> {
        self.shutdown().await
    }

    /// Perform health check on all components
    pub async fn health_check(&self) -> Result<SystemHealth> {
        // Update component health status
        {
            let mut state = self.state.write().await;
            state.last_health_check = Utc::now();

            // Update database health from a real probe. An empty store is
            // healthy (operational), not a warning.
            state.component_health.database = match self.database.health_check().await {
                Ok(status) => match status.status {
                    crate::database::HealthLevel::Healthy => HealthStatus::Healthy,
                    crate::database::HealthLevel::Warning => HealthStatus::Warning,
                    _ => HealthStatus::Critical,
                },
                // Fall back to a stats probe if the backend has no health_check.
                Err(_) => match self.database.get_stats().await {
                    Ok(_) => HealthStatus::Healthy,
                    Err(_) => HealthStatus::Critical,
                },
            };

            // The API and memory manager are in-process; they are healthy while
            // the process is serving. The scheduler reflects whether it is running.
            state.component_health.api = HealthStatus::Healthy;
            state.component_health.memory_manager = HealthStatus::Healthy;
            state.component_health.scheduler = if self.scheduler.is_running().await {
                HealthStatus::Healthy
            } else {
                HealthStatus::Warning
            };
        }

        Ok(self.get_health().await)
    }

    /// Get system health status
    pub async fn get_health(&self) -> SystemHealth {
        let state = self.state.read().await;
        let uptime = state
            .started_at
            .map(|start| (Utc::now() - start).to_std().unwrap_or_default())
            .unwrap_or_default();

        SystemHealth {
            overall_status: match state.status {
                SystemStatus::Running => HealthStatus::Healthy,
                SystemStatus::Starting | SystemStatus::Stopping => HealthStatus::Warning,
                SystemStatus::Stopped => HealthStatus::Down,
                SystemStatus::Error(_) => HealthStatus::Critical,
                SystemStatus::Initializing => HealthStatus::Warning,
            },
            database_status: state.component_health.database.clone(),
            uptime,
            last_check: state.last_health_check,
        }
    }

    /// Get system configuration
    pub fn config(&self) -> &GaussOSConfig {
        &self.config
    }

    /// Get database reference
    pub fn database(&self) -> &DatabaseVault {
        &self.database
    }
}
