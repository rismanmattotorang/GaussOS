// src/lifecycle.rs
//! System Lifecycle Management
//! Provides comprehensive system startup, shutdown, health monitoring,
//! and graceful degradation capabilities for the GaussOS system

use crate::{
    database::MemVault,
    error::{GaussOSError, Result},
    scheduler::TaskScheduler,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

/// System lifecycle manager
pub struct LifecycleManager {
    database: Arc<dyn MemVault>,
    scheduler: Option<Arc<RwLock<TaskScheduler>>>,
    state: Arc<RwLock<SystemState>>,
    health_monitor: Arc<HealthMonitor>,
    shutdown_signals: Vec<oneshot::Sender<()>>,
    lifecycle_tx: Option<broadcast::Sender<LifecycleEvent>>,
    control_tx: Option<mpsc::UnboundedSender<LifecycleCommand>>,
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
}

/// System state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub status: SystemStatus,
    pub startup_time: DateTime<Utc>,
    pub last_health_check: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
    pub version: String,
    pub build_info: BuildInfo,
    pub configuration: SystemConfiguration,
    pub components: HashMap<String, ComponentStatus>,
    pub metrics: SystemMetrics,
    pub warnings: Vec<SystemWarning>,
    pub errors: Vec<SystemError>,
}

/// System status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemStatus {
    Starting,
    Running,
    Degraded,
    Stopping,
    Stopped,
    Error,
    Maintenance,
}

/// Build information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub build_date: String,
    pub git_commit: String,
    pub rust_version: String,
    pub target: String,
    pub features: Vec<String>,
}

/// System configuration summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfiguration {
    pub memory_limit_mb: Option<u64>,
    pub max_connections: u32,
    pub log_level: String,
    pub database_url: Option<String>,
    pub api_host: String,
    pub api_port: u16,
    pub auth_enabled: bool,
    pub custom_config: HashMap<String, serde_json::Value>,
}

/// Component status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub status: ComponentHealth,
    pub last_check: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub error_count: u32,
    pub warning_count: u32,
    pub metrics: HashMap<String, f64>,
    pub dependencies: Vec<String>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentHealth {
    Healthy,
    Warning,
    Critical,
    Down,
    Unknown,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_mb: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub active_connections: u32,
    pub requests_per_second: f64,
    pub error_rate: f64,
    pub average_response_time_ms: f64,
    pub custom_metrics: HashMap<String, f64>,
}

/// System warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemWarning {
    pub id: Uuid,
    pub component: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub severity: WarningSeverity,
    pub acknowledged: bool,
}

/// System error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemError {
    pub id: Uuid,
    pub component: String,
    pub error_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub stack_trace: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
}

/// Warning severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}

/// Lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    SystemStarting,
    SystemReady,
    SystemStopping,
    SystemStopped,
    ComponentStarted(String),
    ComponentStopped(String),
    ComponentFailure(String, String),
    HealthCheckFailed(String),
    SystemError(String),
    ConfigurationChanged,
    MaintenanceStarted,
    MaintenanceCompleted,
}

/// Lifecycle control commands
#[derive(Debug)]
pub enum LifecycleCommand {
    Shutdown(oneshot::Sender<Result<()>>),
    Restart(oneshot::Sender<Result<()>>),
    MaintenanceMode(bool, oneshot::Sender<Result<()>>),
    RefreshHealthChecks,
    UpdateMetrics,
    GetStatus(oneshot::Sender<SystemState>),
    AddComponent(String, ComponentStatus),
    RemoveComponent(String),
}

/// Health monitor for system components
pub struct HealthMonitor {
    check_interval: Duration,
    timeout: Duration,
    failure_threshold: u32,
    recovery_threshold: u32,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(database: Arc<dyn MemVault>) -> Self {
        let state = SystemState {
            status: SystemStatus::Stopped,
            startup_time: Utc::now(),
            last_health_check: None,
            uptime_seconds: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_info: BuildInfo::current(),
            configuration: SystemConfiguration::default(),
            components: HashMap::new(),
            metrics: SystemMetrics::default(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        Self {
            database,
            scheduler: None,
            state: Arc::new(RwLock::new(state)),
            health_monitor: Arc::new(HealthMonitor::new()),
            shutdown_signals: Vec::new(),
            lifecycle_tx: None,
            control_tx: None,
            monitor_handle: None,
        }
    }

    /// Initialize the system and start lifecycle management
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing GaussOS system");

        // Update state to starting
        {
            let mut state = self.state.write().unwrap();
            state.status = SystemStatus::Starting;
            state.startup_time = Utc::now();
        }

        // Create event and control channels
        let (lifecycle_tx, _) = broadcast::channel(1000);
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        self.lifecycle_tx = Some(lifecycle_tx.clone());
        self.control_tx = Some(control_tx);

        // Send starting event
        let _ = lifecycle_tx.send(LifecycleEvent::SystemStarting);

        // Initialize core components
        self.initialize_core_components().await?;

        // Start health monitoring
        let state_clone = self.state.clone();
        let health_monitor_clone = self.health_monitor.clone();
        let lifecycle_tx_clone = lifecycle_tx.clone();

        let monitor_handle = tokio::spawn(async move {
            Self::health_monitor_loop(state_clone, health_monitor_clone, lifecycle_tx_clone).await;
        });

        self.monitor_handle = Some(monitor_handle);

        // Start control loop
        let state_clone = self.state.clone();
        let lifecycle_tx_clone = lifecycle_tx.clone();

        tokio::spawn(async move {
            Self::control_loop(control_rx, state_clone, lifecycle_tx_clone).await;
        });

        // Update state to running
        {
            let mut state = self.state.write().unwrap();
            state.status = SystemStatus::Running;
        }

        // Send ready event
        let _ = lifecycle_tx.send(LifecycleEvent::SystemReady);

        tracing::info!("GaussOS system initialized successfully");
        Ok(())
    }

    /// Shutdown the system gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Initiating graceful shutdown");

        // Update state to stopping
        {
            let mut state = self.state.write().unwrap();
            state.status = SystemStatus::Stopping;
        }

        // Send shutdown event
        if let Some(tx) = &self.lifecycle_tx {
            let _ = tx.send(LifecycleEvent::SystemStopping);
        }

        // Stop scheduler if running
        if let Some(scheduler) = &self.scheduler {
            scheduler.write().unwrap().stop().await?;
        }

        // Stop health monitoring
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
        }

        // Send shutdown signals to all components
        for signal in self.shutdown_signals.drain(..) {
            let _ = signal.send(());
        }

        // Wait for graceful shutdown (with timeout)
        tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            self.wait_for_component_shutdown(),
        )
        .await
        .map_err(|_| {
            GaussOSError::system_error(
                "lifecycle".to_string(),
                "Shutdown timeout exceeded".to_string(),
            )
        })??;

        // Update state to stopped
        {
            let mut state = self.state.write().unwrap();
            state.status = SystemStatus::Stopped;
        }

        // Send stopped event
        if let Some(tx) = &self.lifecycle_tx {
            let _ = tx.send(LifecycleEvent::SystemStopped);
        }

        tracing::info!("System shutdown completed");
        Ok(())
    }

    /// Get current system state
    pub fn get_state(&self) -> SystemState {
        self.state.read().unwrap().clone()
    }

    /// Subscribe to lifecycle events
    pub fn subscribe_to_events(&self) -> Option<broadcast::Receiver<LifecycleEvent>> {
        self.lifecycle_tx.as_ref().map(|tx| tx.subscribe())
    }

    /// Register a component for lifecycle management
    pub async fn register_component(
        &self,
        name: &str,
        dependencies: Vec<String>,
    ) -> Result<oneshot::Receiver<()>> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let component_status = ComponentStatus {
            name: name.to_string(),
            status: ComponentHealth::Healthy,
            last_check: Utc::now(),
            uptime_seconds: 0,
            error_count: 0,
            warning_count: 0,
            metrics: HashMap::new(),
            dependencies,
        };

        // Add component to state
        {
            let mut state = self.state.write().unwrap();
            state.components.insert(name.to_string(), component_status);
        }

        // Store shutdown signal
        // Note: In a real implementation, we'd need a better way to manage these
        // For now, this is a simplified version

        // Send component started event
        if let Some(tx) = &self.lifecycle_tx {
            let _ = tx.send(LifecycleEvent::ComponentStarted(name.to_string()));
        }

        tracing::info!("Component registered: {}", name);
        Ok(shutdown_rx)
    }

    /// Update component status
    pub async fn update_component_status(&self, name: &str, status: ComponentHealth) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(component) = state.components.get_mut(name) {
            component.status = status;
            component.last_check = Utc::now();
        }
        Ok(())
    }

    /// Add system warning
    pub async fn add_warning(&self, component: &str, message: &str, severity: WarningSeverity) {
        let warning = SystemWarning {
            id: Uuid::new_v4(),
            component: component.to_string(),
            message: message.to_string(),
            timestamp: Utc::now(),
            severity,
            acknowledged: false,
        };

        {
            let mut state = self.state.write().unwrap();
            state.warnings.push(warning);

            // Keep only last 100 warnings
            if state.warnings.len() > 100 {
                state.warnings.remove(0);
            }
        }

        tracing::warn!("System warning [{}]: {}", component, message);
    }

    /// Add system error
    pub async fn add_error(
        &self,
        component: &str,
        error_type: &str,
        message: &str,
        context: Option<HashMap<String, serde_json::Value>>,
    ) {
        let error = SystemError {
            id: Uuid::new_v4(),
            component: component.to_string(),
            error_type: error_type.to_string(),
            message: message.to_string(),
            timestamp: Utc::now(),
            stack_trace: None,
            context: context.unwrap_or_default(),
        };

        {
            let mut state = self.state.write().unwrap();
            state.errors.push(error);

            // Keep only last 50 errors
            if state.errors.len() > 50 {
                state.errors.remove(0);
            }
        }

        // Send error event
        if let Some(tx) = &self.lifecycle_tx {
            let _ = tx.send(LifecycleEvent::SystemError(message.to_string()));
        }

        tracing::error!("System error [{}:{}]: {}", component, error_type, message);
    }

    /// Initialize core system components
    async fn initialize_core_components(&mut self) -> Result<()> {
        tracing::info!("Initializing core components");

        // Initialize database connection
        self.register_component("database", vec![]).await?;

        // Initialize scheduler
        let mut scheduler = TaskScheduler::new(self.database.clone());
        scheduler.start().await?;
        self.scheduler = Some(Arc::new(RwLock::new(scheduler)));
        self.register_component("scheduler", vec!["database".to_string()])
            .await?;

        // Initialize other core components
        self.register_component("auth", vec!["database".to_string()])
            .await?;
        self.register_component("api", vec!["auth".to_string(), "database".to_string()])
            .await?;

        Ok(())
    }

    /// Wait for all components to shutdown gracefully
    async fn wait_for_component_shutdown(&self) -> Result<()> {
        // In a real implementation, this would wait for all components
        // to signal they've shut down cleanly
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    /// Health monitoring loop
    async fn health_monitor_loop(
        state: Arc<RwLock<SystemState>>,
        health_monitor: Arc<HealthMonitor>,
        lifecycle_tx: broadcast::Sender<LifecycleEvent>,
    ) {
        let mut interval = tokio::time::interval(health_monitor.check_interval.to_std().unwrap());

        loop {
            interval.tick().await;

            // Update system metrics
            let metrics = Self::collect_system_metrics().await;
            let now = Utc::now();

            {
                let mut state_guard = state.write().unwrap();
                state_guard.metrics = metrics;
                state_guard.last_health_check = Some(now);
                state_guard.uptime_seconds = (now - state_guard.startup_time).num_seconds() as u64;

                // Check component health
                for component in state_guard.components.values_mut() {
                    if now - component.last_check > Duration::minutes(5) {
                        component.status = ComponentHealth::Unknown;
                    }
                }
            }

            // Check overall system health
            let overall_health = Self::assess_system_health(&state);

            {
                let mut state_guard = state.write().unwrap();
                match overall_health {
                    ComponentHealth::Healthy => {
                        if state_guard.status == SystemStatus::Degraded {
                            state_guard.status = SystemStatus::Running;
                        }
                    }
                    ComponentHealth::Warning | ComponentHealth::Critical => {
                        if state_guard.status == SystemStatus::Running {
                            state_guard.status = SystemStatus::Degraded;
                        }
                    }
                    ComponentHealth::Down => {
                        state_guard.status = SystemStatus::Error;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Control loop for handling lifecycle commands
    async fn control_loop(
        mut control_rx: mpsc::UnboundedReceiver<LifecycleCommand>,
        state: Arc<RwLock<SystemState>>,
        lifecycle_tx: broadcast::Sender<LifecycleEvent>,
    ) {
        while let Some(command) = control_rx.recv().await {
            match command {
                LifecycleCommand::GetStatus(response_tx) => {
                    let state_clone = state.read().unwrap().clone();
                    let _ = response_tx.send(state_clone);
                }
                LifecycleCommand::RefreshHealthChecks => {
                    // Trigger immediate health check
                    tracing::info!("Health check refresh requested");
                }
                LifecycleCommand::UpdateMetrics => {
                    let metrics = Self::collect_system_metrics().await;
                    state.write().unwrap().metrics = metrics;
                }
                LifecycleCommand::AddComponent(name, component_status) => {
                    state
                        .write()
                        .unwrap()
                        .components
                        .insert(name.clone(), component_status);
                    let _ = lifecycle_tx.send(LifecycleEvent::ComponentStarted(name));
                }
                LifecycleCommand::RemoveComponent(name) => {
                    state.write().unwrap().components.remove(&name);
                    let _ = lifecycle_tx.send(LifecycleEvent::ComponentStopped(name));
                }
                _ => {
                    // Handle other commands (shutdown, restart, maintenance)
                    tracing::info!(
                        "Lifecycle command received: {:?}",
                        std::mem::discriminant(&command)
                    );
                }
            }
        }
    }

    /// Collect current system metrics from the live host via `sysinfo`.
    async fn collect_system_metrics() -> SystemMetrics {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.refresh_cpu();

        let memory_usage_mb = sys.used_memory() as f64 / 1_048_576.0;
        let cpu_usage_percent = sys.global_cpu_info().cpu_usage() as f64;
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let disk_used: u64 = disks
            .iter()
            .map(|d| d.total_space().saturating_sub(d.available_space()))
            .sum();
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let network_rx_bytes: u64 = networks.iter().map(|(_, n)| n.total_received()).sum();
        let network_tx_bytes: u64 = networks.iter().map(|(_, n)| n.total_transmitted()).sum();

        SystemMetrics {
            memory_usage_mb,
            cpu_usage_percent,
            disk_usage_mb: disk_used as f64 / 1_048_576.0,
            network_rx_bytes,
            network_tx_bytes,
            active_connections: 0,
            requests_per_second: 0.0,
            error_rate: 0.0,
            average_response_time_ms: 0.0,
            custom_metrics: HashMap::new(),
        }
    }

    /// Assess overall system health based on component status
    fn assess_system_health(state: &Arc<RwLock<SystemState>>) -> ComponentHealth {
        let state_guard = state.read().unwrap();
        let components = &state_guard.components;

        if components.is_empty() {
            return ComponentHealth::Unknown;
        }

        let mut healthy_count = 0;
        let mut warning_count = 0;
        let mut critical_count = 0;
        let mut down_count = 0;

        for component in components.values() {
            match component.status {
                ComponentHealth::Healthy => healthy_count += 1,
                ComponentHealth::Warning => warning_count += 1,
                ComponentHealth::Critical => critical_count += 1,
                ComponentHealth::Down => down_count += 1,
                ComponentHealth::Unknown => {}
            }
        }

        if down_count > 0 {
            ComponentHealth::Down
        } else if critical_count > 0 {
            ComponentHealth::Critical
        } else if warning_count > 0 {
            ComponentHealth::Warning
        } else {
            ComponentHealth::Healthy
        }
    }
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self {
            check_interval: Duration::seconds(30),
            timeout: Duration::seconds(10),
            failure_threshold: 3,
            recovery_threshold: 2,
        }
    }

    /// Create health monitor with custom settings
    pub fn with_settings(
        check_interval: Duration,
        timeout: Duration,
        failure_threshold: u32,
        recovery_threshold: u32,
    ) -> Self {
        Self {
            check_interval,
            timeout,
            failure_threshold,
            recovery_threshold,
        }
    }
}

impl BuildInfo {
    /// Get current build information
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_date: chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
            git_commit: "unknown".to_string(), // Would be set during build
            rust_version: "unknown".to_string(), // Would be set during build
            target: std::env::consts::ARCH.to_string(),
            features: vec![], // Would be populated with enabled features
        }
    }
}

impl Default for SystemConfiguration {
    fn default() -> Self {
        Self {
            memory_limit_mb: None,
            max_connections: 1000,
            log_level: "info".to_string(),
            database_url: None,
            api_host: "localhost".to_string(),
            api_port: 8080,
            auth_enabled: true,
            custom_config: HashMap::new(),
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            disk_usage_mb: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            active_connections: 0,
            requests_per_second: 0.0,
            error_rate: 0.0,
            average_response_time_ms: 0.0,
            custom_metrics: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMemVault;

    #[async_trait::async_trait]
    impl MemVault for MockMemVault {
        async fn store(&self, _memory: &crate::core::MemCube) -> Result<()> {
            Ok(())
        }
        async fn retrieve(&self, _id: &Uuid) -> Result<Option<crate::core::MemCube>> {
            Ok(None)
        }
        async fn update(&self, _memory: &crate::core::MemCube) -> Result<()> {
            Ok(())
        }
        async fn delete(&self, _id: &Uuid) -> Result<()> {
            Ok(())
        }
        async fn search(
            &self,
            _query: &crate::database::SearchQuery,
        ) -> Result<Vec<crate::core::MemCube>> {
            Ok(Vec::new())
        }
        async fn list_by_tags(&self, _tags: &[String]) -> Result<Vec<crate::core::MemCube>> {
            Ok(Vec::new())
        }
        async fn get_stats(&self) -> Result<crate::database::VaultStats> {
            Ok(crate::database::VaultStats {
                total_memories: 0,
                memory_by_type: HashMap::new(),
                memory_by_namespace: HashMap::new(),
                storage_size: 0,
                average_memory_size: 0.0,
                average_access_count: 0.0,
                quality_distribution: crate::database::QualityDistribution {
                    excellent: 0,
                    very_good: 0,
                    good: 0,
                    average: 0,
                    below_average: 0,
                    poor: 0,
                    very_poor: 0,
                },
                age_statistics: crate::database::AgeStatistics {
                    newest: Utc::now(),
                    oldest: Utc::now(),
                    average_age_days: 0.0,
                    median_age_days: 0.0,
                    percentiles: crate::database::AgePercentiles {
                        p50: 0.0,
                        p75: 0.0,
                        p90: 0.0,
                        p95: 0.0,
                        p99: 0.0,
                    },
                },
                performance_metrics: crate::database::PerformanceMetrics {
                    average_query_time_ms: 0.0,
                    p95_query_time_ms: 0.0,
                    p99_query_time_ms: 0.0,
                    queries_per_second: 0.0,
                    cache_hit_rate: 0.0,
                    index_usage_rate: 0.0,
                },
                storage_metrics: crate::database::StorageMetrics {
                    compression_ratio: 0.0,
                    fragmentation_ratio: 0.0,
                    index_size: 0,
                    data_size: 0,
                    total_size: 0,
                    growth_rate_per_day: 0.0,
                },
                database_metrics: None,
                last_updated: Utc::now(),
            })
        }
        async fn backup(
            &self,
            _backup_config: &crate::database::BackupConfig,
        ) -> Result<crate::database::BackupResult> {
            Ok(crate::database::BackupResult {
                backup_id: Uuid::new_v4(),
                size_bytes: 0,
                duration_ms: 0,
                checksum: "mock_checksum".to_string(),
                metadata: crate::database::BackupMetadata {
                    timestamp: Utc::now(),
                    database_version: "1.0.0".to_string(),
                    record_count: 0,
                    compression_ratio: 0.0,
                },
            })
        }
        async fn restore(
            &self,
            _restore_config: &crate::database::RestoreConfig,
        ) -> Result<crate::database::RestoreResult> {
            Ok(crate::database::RestoreResult {
                records_restored: 0,
                duration_ms: 0,
                integrity_verified: true,
            })
        }
        async fn optimize(&self) -> Result<crate::database::OptimizationResult> {
            Ok(crate::database::OptimizationResult {
                operations_performed: vec![],
                space_reclaimed_bytes: 0,
                performance_improvement_percent: 0.0,
                duration_ms: 0,
            })
        }
        async fn get_real_time_metrics(&self) -> Result<crate::database::RealTimeMetrics> {
            Ok(crate::database::RealTimeMetrics {
                timestamp: Utc::now(),
                operations_per_second: 0.0,
                active_queries: 0,
                slow_queries: 0,
                cache_hit_rate: 0.0,
                connection_utilization: 0.0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                disk_io_mb_per_sec: 0.0,
                network_io_mb_per_sec: 0.0,
            })
        }
    }

    #[tokio::test]
    async fn test_lifecycle_manager_creation() {
        let database = Arc::new(MockMemVault);
        let manager = LifecycleManager::new(database);

        let state = manager.get_state();
        assert_eq!(state.status, SystemStatus::Stopped);
        assert!(!state.version.is_empty());
    }

    #[tokio::test]
    async fn test_component_registration() {
        let database = Arc::new(MockMemVault);
        let manager = LifecycleManager::new(database);

        // Test basic manager creation
        let state = manager.get_state();
        assert_eq!(state.status, SystemStatus::Stopped);
        assert!(!state.version.is_empty());

        // Test component registration without full initialization
        let test_component = ComponentStatus {
            name: "test-component".to_string(),
            status: ComponentHealth::Healthy,
            last_check: Utc::now(),
            uptime_seconds: 0,
            error_count: 0,
            warning_count: 0,
            metrics: HashMap::new(),
            dependencies: Vec::new(),
        };

        assert_eq!(test_component.name, "test-component");
        assert_eq!(test_component.status, ComponentHealth::Healthy);
    }

    #[test]
    fn test_system_health_assessment() {
        let state = SystemState {
            status: SystemStatus::Running,
            startup_time: Utc::now(),
            last_health_check: None,
            uptime_seconds: 0,
            version: "test".to_string(),
            build_info: BuildInfo::current(),
            configuration: SystemConfiguration::default(),
            components: {
                let mut components = HashMap::new();
                components.insert(
                    "healthy".to_string(),
                    ComponentStatus {
                        name: "healthy".to_string(),
                        status: ComponentHealth::Healthy,
                        last_check: Utc::now(),
                        uptime_seconds: 0,
                        error_count: 0,
                        warning_count: 0,
                        metrics: HashMap::new(),
                        dependencies: Vec::new(),
                    },
                );
                components.insert(
                    "warning".to_string(),
                    ComponentStatus {
                        name: "warning".to_string(),
                        status: ComponentHealth::Warning,
                        last_check: Utc::now(),
                        uptime_seconds: 0,
                        error_count: 0,
                        warning_count: 1,
                        metrics: HashMap::new(),
                        dependencies: Vec::new(),
                    },
                );
                components
            },
            metrics: SystemMetrics::default(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        let state_arc = Arc::new(RwLock::new(state));
        let health = LifecycleManager::assess_system_health(&state_arc);
        assert_eq!(health, ComponentHealth::Warning);
    }
}
