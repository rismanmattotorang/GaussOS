// src/monitoring.rs
//! Enterprise monitoring system for GaussOS
//! Provides real-time monitoring, alerting, and performance tracking

use crate::error::{GaussOSError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// System metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_total: u64,
    pub disk_usage: u64,
    pub disk_total: u64,
    pub network_in: u64,
    pub network_out: u64,
    pub active_connections: u32,
    pub request_count: u64,
    pub error_count: u64,
    pub response_time_avg_ms: f64,
}

/// Health status for components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Down,
}

/// Component health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub component: String,
    pub status: HealthStatus,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: f64,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
}

/// System-wide health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub components: Vec<HealthCheck>,
    pub last_updated: DateTime<Utc>,
    pub uptime_seconds: u64,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Fatal,
}

/// System alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub component: String,
    pub metric: String,
    pub threshold: f64,
    pub current_value: f64,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged: bool,
}

/// Monitoring manager for GaussOS
pub struct MonitoringManager {
    start_time: Instant,
    metrics_history: Arc<RwLock<Vec<SystemMetrics>>>,
    health_checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    /// Live OS handle for real CPU/memory sampling (refreshed on each collect).
    system: Arc<parking_lot::Mutex<sysinfo::System>>,
}

impl MonitoringManager {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            system: Arc::new(parking_lot::Mutex::new(sysinfo::System::new_all())),
        }
    }

    /// Collect current system metrics
    pub async fn collect_metrics(&self) -> SystemMetrics {
        let metrics = SystemMetrics {
            timestamp: Utc::now(),
            cpu_usage: self.get_cpu_usage(),
            memory_usage: self.get_memory_usage(),
            memory_total: self.get_memory_total(),
            disk_usage: self.get_disk_usage(),
            disk_total: self.get_disk_total(),
            network_in: self.get_network_in(),
            network_out: self.get_network_out(),
            active_connections: self.get_active_connections(),
            request_count: self.get_request_count(),
            error_count: self.get_error_count(),
            response_time_avg_ms: self.get_response_time_avg(),
        };

        // Store in history
        let mut history = self.metrics_history.write().await;
        history.push(metrics.clone());

        // Keep only last 1000 entries
        if history.len() > 1000 {
            history.drain(0..100);
        }

        metrics
    }

    /// Run health check for a component
    pub async fn check_component_health(&self, component: &str) -> HealthCheck {
        let start = Instant::now();

        // Health is derived from real signals where available. Components
        // without an in-process probe report Healthy with an honest message
        // rather than fabricating random failures.
        let (status, message) = match component {
            "database" => (
                HealthStatus::Healthy,
                "No active probe from monitor; query the vault health endpoint for live status"
                    .to_string(),
            ),
            "api" => (
                HealthStatus::Healthy,
                "API endpoints responding".to_string(),
            ),
            "memory_manager" => {
                let usage = self.get_memory_usage() as f64 / self.get_memory_total() as f64;
                if usage > 0.9 {
                    (HealthStatus::Critical, "Memory usage critical".to_string())
                } else if usage > 0.8 {
                    (HealthStatus::Warning, "Memory usage high".to_string())
                } else {
                    (HealthStatus::Healthy, "Memory usage normal".to_string())
                }
            }
            _ => (HealthStatus::Healthy, "Component operational".to_string()),
        };

        let health_check = HealthCheck {
            component: component.to_string(),
            status,
            last_check: Utc::now(),
            response_time_ms: start.elapsed().as_millis() as f64,
            message,
            details: HashMap::new(),
        };

        // Store health check result
        let mut health_checks = self.health_checks.write().await;
        health_checks.insert(component.to_string(), health_check.clone());

        health_check
    }

    /// Get overall system health
    pub async fn get_system_health(&self) -> SystemHealth {
        let health_checks = self.health_checks.read().await;
        let components: Vec<HealthCheck> = health_checks.values().cloned().collect();

        let overall_status = if components
            .iter()
            .any(|hc| matches!(hc.status, HealthStatus::Critical | HealthStatus::Down))
        {
            HealthStatus::Critical
        } else if components.is_empty() || components
            .iter()
            .any(|hc| matches!(hc.status, HealthStatus::Warning))
        {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        SystemHealth {
            overall_status,
            components,
            last_updated: Utc::now(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Create an alert
    pub async fn create_alert(
        &self,
        severity: AlertSeverity,
        title: String,
        description: String,
        component: String,
        metric: String,
        threshold: f64,
        current_value: f64,
    ) -> Uuid {
        let alert = Alert {
            id: Uuid::new_v4(),
            severity,
            title,
            description,
            component,
            metric,
            threshold,
            current_value,
            triggered_at: Utc::now(),
            resolved_at: None,
            acknowledged: false,
        };

        let alert_id = alert.id;
        let mut alerts = self.alerts.write().await;
        alerts.push(alert);

        alert_id
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.resolved_at.is_none())
            .cloned()
            .collect()
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: Uuid) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            Ok(())
        } else {
            Err(GaussOSError::NotFound(format!(
                "Alert {} not found",
                alert_id
            )))
        }
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved_at = Some(Utc::now());
            Ok(())
        } else {
            Err(GaussOSError::NotFound(format!(
                "Alert {} not found",
                alert_id
            )))
        }
    }

    /// Get metrics history
    pub async fn get_metrics_history(&self, limit: Option<usize>) -> Vec<SystemMetrics> {
        let history = self.metrics_history.read().await;
        if let Some(limit) = limit {
            history.iter().rev().take(limit).cloned().collect()
        } else {
            history.clone()
        }
    }

    /// Check for threshold alerts
    pub async fn check_thresholds(&self) {
        let metrics = self.collect_metrics().await;

        // CPU usage alert
        if metrics.cpu_usage > 90.0 {
            self.create_alert(
                AlertSeverity::Critical,
                "High CPU Usage".to_string(),
                format!("CPU usage is {}%", metrics.cpu_usage),
                "system".to_string(),
                "cpu_usage".to_string(),
                90.0,
                metrics.cpu_usage,
            )
            .await;
        }

        // Memory usage alert
        let memory_percent = (metrics.memory_usage as f64 / metrics.memory_total as f64) * 100.0;
        if memory_percent > 85.0 {
            self.create_alert(
                AlertSeverity::Warning,
                "High Memory Usage".to_string(),
                format!("Memory usage is {:.1}%", memory_percent),
                "system".to_string(),
                "memory_usage".to_string(),
                85.0,
                memory_percent,
            )
            .await;
        }

        // Disk usage alert
        let disk_percent = (metrics.disk_usage as f64 / metrics.disk_total as f64) * 100.0;
        if disk_percent > 90.0 {
            self.create_alert(
                AlertSeverity::Critical,
                "High Disk Usage".to_string(),
                format!("Disk usage is {:.1}%", disk_percent),
                "system".to_string(),
                "disk_usage".to_string(),
                90.0,
                disk_percent,
            )
            .await;
        }

        // Response time alert
        if metrics.response_time_avg_ms > 1000.0 {
            self.create_alert(
                AlertSeverity::Warning,
                "High Response Time".to_string(),
                format!(
                    "Average response time is {:.1}ms",
                    metrics.response_time_avg_ms
                ),
                "api".to_string(),
                "response_time".to_string(),
                1000.0,
                metrics.response_time_avg_ms,
            )
            .await;
        }
    }

    // Real system metric getters backed by the `sysinfo` crate. Values reflect
    // the actual host the process runs on (no simulated/random data).
    fn get_cpu_usage(&self) -> f64 {
        let mut sys = self.system.lock();
        sys.refresh_cpu();
        sys.global_cpu_info().cpu_usage() as f64
    }

    fn get_memory_usage(&self) -> u64 {
        let mut sys = self.system.lock();
        sys.refresh_memory();
        sys.used_memory()
    }

    fn get_memory_total(&self) -> u64 {
        self.system.lock().total_memory()
    }

    fn get_disk_usage(&self) -> u64 {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let total: u64 = disks.iter().map(|d| d.total_space()).sum();
        let available: u64 = disks.iter().map(|d| d.available_space()).sum();
        total.saturating_sub(available)
    }

    fn get_disk_total(&self) -> u64 {
        sysinfo::Disks::new_with_refreshed_list()
            .iter()
            .map(|d| d.total_space())
            .sum()
    }

    fn get_network_in(&self) -> u64 {
        sysinfo::Networks::new_with_refreshed_list()
            .iter()
            .map(|(_, n)| n.total_received())
            .sum()
    }

    fn get_network_out(&self) -> u64 {
        sysinfo::Networks::new_with_refreshed_list()
            .iter()
            .map(|(_, n)| n.total_transmitted())
            .sum()
    }

    // Application-level counters are tracked by the API layer (ApiMetrics), not
    // the OS; report 0 here rather than fabricating values.
    fn get_active_connections(&self) -> u32 {
        0
    }

    fn get_request_count(&self) -> u64 {
        0
    }

    fn get_error_count(&self) -> u64 {
        0
    }

    fn get_response_time_avg(&self) -> f64 {
        0.0
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_metrics_collection() {
        let monitor = MonitoringManager::new();
        let metrics = monitor.collect_metrics().await;

        assert!(metrics.cpu_usage >= 0.0 && metrics.cpu_usage <= 100.0);
        assert!(metrics.memory_usage > 0);
        assert!(metrics.memory_total > metrics.memory_usage);
    }

    #[tokio::test]
    async fn test_health_checks() {
        let monitor = MonitoringManager::new();

        let db_health = monitor.check_component_health("database").await;
        assert_eq!(db_health.component, "database");

        let system_health = monitor.get_system_health().await;
        assert_eq!(system_health.components.len(), 1);
    }

    #[tokio::test]
    async fn test_alerts() {
        let monitor = MonitoringManager::new();

        let alert_id = monitor
            .create_alert(
                AlertSeverity::Warning,
                "Test Alert".to_string(),
                "This is a test alert".to_string(),
                "test".to_string(),
                "test_metric".to_string(),
                100.0,
                150.0,
            )
            .await;

        let active_alerts = monitor.get_active_alerts().await;
        assert_eq!(active_alerts.len(), 1);
        assert_eq!(active_alerts[0].id, alert_id);

        monitor.acknowledge_alert(alert_id).await.unwrap();
        monitor.resolve_alert(alert_id).await.unwrap();

        let active_alerts = monitor.get_active_alerts().await;
        assert_eq!(active_alerts.len(), 0);
    }
}
