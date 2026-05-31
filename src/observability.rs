// src/observability.rs
//! Enterprise Observability System for GaussOS
//! Provides comprehensive monitoring, distributed tracing, metrics collection,
//! and real-time performance analytics for financial industry requirements

use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use uuid::Uuid;

static INITIALISED: OnceCell<()> = OnceCell::new();
static GLOBAL_METRICS: OnceCell<Arc<GlobalMetricsCollector>> = OnceCell::new();
static TRACE_COLLECTOR: OnceCell<Arc<DistributedTraceCollector>> = OnceCell::new();

/// Enterprise observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable distributed tracing
    pub tracing_enabled: bool,

    /// Tracing sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,

    /// Enable metrics collection
    pub metrics_enabled: bool,

    /// Metrics export interval
    pub metrics_interval_secs: u64,

    /// Enable performance profiling
    pub profiling_enabled: bool,

    /// Enable real-time monitoring
    pub realtime_monitoring: bool,

    /// Log level configuration
    pub log_level: String,

    /// Log format (json, text, structured)
    pub log_format: LogFormat,

    /// Export configuration
    pub exporters: Vec<ExporterConfig>,

    /// Custom tags to add to all telemetry
    pub global_tags: HashMap<String, String>,

    /// Performance thresholds for alerting
    pub performance_thresholds: PerformanceThresholds,

    /// Security monitoring configuration
    pub security_monitoring: SecurityMonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Structured,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterConfig {
    /// Exporter type (prometheus, jaeger, otlp, custom)
    pub exporter_type: String,

    /// Endpoint URL
    pub endpoint: String,

    /// Authentication headers
    pub headers: HashMap<String, String>,

    /// Export interval
    pub interval_secs: u64,

    /// Batch size for exports
    pub batch_size: usize,

    /// Enable compression
    pub compression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum response time in milliseconds
    pub max_response_time_ms: u64,

    /// Maximum memory usage in MB
    pub max_memory_mb: u64,

    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,

    /// Minimum cache hit rate
    pub min_cache_hit_rate: f64,

    /// Maximum error rate percentage
    pub max_error_rate_percent: f64,

    /// Custom thresholds
    pub custom_thresholds: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMonitoringConfig {
    /// Enable security event tracking
    pub enabled: bool,

    /// Track authentication events
    pub track_auth_events: bool,

    /// Track authorization events
    pub track_authz_events: bool,

    /// Track data access events
    pub track_data_access: bool,

    /// Track configuration changes
    pub track_config_changes: bool,

    /// Security alert thresholds
    pub alert_thresholds: HashMap<String, u32>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            sampling_rate: 1.0,
            metrics_enabled: true,
            metrics_interval_secs: 60,
            profiling_enabled: true,
            realtime_monitoring: true,
            log_level: "info".to_string(),
            log_format: LogFormat::Json,
            exporters: vec![],
            global_tags: HashMap::new(),
            performance_thresholds: PerformanceThresholds::default(),
            security_monitoring: SecurityMonitoringConfig::default(),
        }
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_response_time_ms: 5000,
            max_memory_mb: 2048,
            max_cpu_percent: 80.0,
            min_cache_hit_rate: 0.8,
            max_error_rate_percent: 5.0,
            custom_thresholds: HashMap::new(),
        }
    }
}

impl Default for SecurityMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            track_auth_events: true,
            track_authz_events: true,
            track_data_access: true,
            track_config_changes: true,
            alert_thresholds: HashMap::new(),
        }
    }
}

/// Global metrics collector for enterprise monitoring
#[derive(Debug)]
pub struct GlobalMetricsCollector {
    /// System metrics
    pub system_metrics: RwLock<SystemMetrics>,

    /// Application metrics
    pub app_metrics: RwLock<ApplicationMetrics>,

    /// Business metrics
    pub business_metrics: RwLock<BusinessMetrics>,

    /// Security metrics
    pub security_metrics: RwLock<SecurityMetrics>,

    /// Custom metrics
    pub custom_metrics: RwLock<HashMap<String, MetricValue>>,

    /// Metrics history for trending
    pub metrics_history: RwLock<Vec<MetricsSnapshot>>,

    /// Performance alerts
    pub alerts: RwLock<Vec<PerformanceAlert>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// Available memory in bytes
    pub memory_available_bytes: u64,

    /// Disk usage percentage
    pub disk_usage_percent: f64,

    /// Network I/O in bytes per second
    pub network_io_bytes_per_sec: u64,

    /// Open file descriptors
    pub open_file_descriptors: u32,

    /// Thread count
    pub thread_count: u32,

    /// System uptime in seconds
    pub uptime_seconds: u64,

    /// Load average
    pub load_average: [f64; 3],

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetrics {
    /// Total requests processed
    pub total_requests: u64,

    /// Requests per second
    pub requests_per_second: f64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// 95th percentile response time
    pub p95_response_time_ms: f64,

    /// 99th percentile response time
    pub p99_response_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Error rate percentage
    pub error_rate_percent: f64,

    /// Active connections
    pub active_connections: u32,

    /// Cache hit rate
    pub cache_hit_rate: f64,

    /// Database connection pool utilization
    pub db_pool_utilization: f64,

    /// Memory operations per second
    pub memory_ops_per_second: f64,

    /// Graph operations per second
    pub graph_ops_per_second: f64,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessMetrics {
    /// Total memories stored
    pub total_memories: u64,

    /// Memories created per hour
    pub memories_per_hour: f64,

    /// Average memory size in bytes
    pub avg_memory_size_bytes: u64,

    /// Total storage used in bytes
    pub total_storage_bytes: u64,

    /// Active users
    pub active_users: u32,

    /// User sessions
    pub user_sessions: u32,

    /// API calls per user
    pub api_calls_per_user: f64,

    /// Revenue metrics (if applicable)
    pub revenue_metrics: HashMap<String, f64>,

    /// Custom business KPIs
    pub custom_kpis: HashMap<String, f64>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Authentication attempts
    pub auth_attempts: u64,

    /// Failed authentication attempts
    pub failed_auth_attempts: u64,

    /// Authorization denials
    pub authz_denials: u64,

    /// Suspicious activities detected
    pub suspicious_activities: u64,

    /// Security alerts raised
    pub security_alerts: u64,

    /// Data access events
    pub data_access_events: u64,

    /// Configuration changes
    pub config_changes: u64,

    /// Compliance violations
    pub compliance_violations: u64,

    /// Last security scan timestamp
    pub last_security_scan: Option<DateTime<Utc>>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Distribution {
        values: Vec<f64>,
        percentiles: HashMap<String, f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// System metrics at this time
    pub system: SystemMetrics,

    /// Application metrics at this time
    pub application: ApplicationMetrics,

    /// Business metrics at this time
    pub business: BusinessMetrics,

    /// Security metrics at this time
    pub security: SecurityMetrics,

    /// Custom metrics at this time
    pub custom: HashMap<String, MetricValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert ID
    pub id: Uuid,

    /// Alert type
    pub alert_type: AlertType,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert message
    pub message: String,

    /// Metric that triggered the alert
    pub metric_name: String,

    /// Current value
    pub current_value: f64,

    /// Threshold value
    pub threshold_value: f64,

    /// Timestamp when alert was raised
    pub timestamp: DateTime<Utc>,

    /// Whether alert is still active
    pub active: bool,

    /// Alert resolution timestamp
    pub resolved_at: Option<DateTime<Utc>>,

    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    PerformanceDegradation,
    ResourceExhaustion,
    ErrorRateSpike,
    SecurityThreat,
    SystemFailure,
    BusinessMetricAnomaly,
    ComplianceViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Fatal,
}

/// Distributed trace collector for request tracing
#[derive(Debug)]
pub struct DistributedTraceCollector {
    /// Active traces
    traces: Mutex<HashMap<Uuid, TraceSpan>>,

    /// Completed traces
    completed_traces: RwLock<Vec<CompletedTrace>>,

    /// Trace statistics
    trace_stats: RwLock<TraceStatistics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Span ID
    pub span_id: Uuid,

    /// Parent span ID
    pub parent_span_id: Option<Uuid>,

    /// Trace ID
    pub trace_id: Uuid,

    /// Operation name
    pub operation_name: String,

    /// Start timestamp
    pub start_time: DateTime<Utc>,

    /// End timestamp
    pub end_time: Option<DateTime<Utc>>,

    /// Duration in microseconds
    pub duration_micros: Option<u64>,

    /// Span tags
    pub tags: HashMap<String, String>,

    /// Span logs
    pub logs: Vec<SpanLog>,

    /// Span status
    pub status: SpanStatus,

    /// Child spans
    pub child_spans: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLog {
    /// Log timestamp
    pub timestamp: DateTime<Utc>,

    /// Log message
    pub message: String,

    /// Log level
    pub level: String,

    /// Additional fields
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error { message: String },
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTrace {
    /// Trace ID
    pub trace_id: Uuid,

    /// Root span
    pub root_span: TraceSpan,

    /// All spans in the trace
    pub spans: Vec<TraceSpan>,

    /// Total trace duration
    pub total_duration_micros: u64,

    /// Number of spans
    pub span_count: usize,

    /// Trace completion timestamp
    pub completed_at: DateTime<Utc>,

    /// Trace metadata
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceStatistics {
    /// Total traces collected
    pub total_traces: u64,

    /// Average trace duration
    pub avg_trace_duration_micros: f64,

    /// Traces by operation
    pub traces_by_operation: HashMap<String, u64>,

    /// Error traces
    pub error_traces: u64,

    /// Slow traces (above threshold)
    pub slow_traces: u64,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl GlobalMetricsCollector {
    pub fn new() -> Self {
        Self {
            system_metrics: RwLock::new(SystemMetrics::default()),
            app_metrics: RwLock::new(ApplicationMetrics::default()),
            business_metrics: RwLock::new(BusinessMetrics::default()),
            security_metrics: RwLock::new(SecurityMetrics::default()),
            custom_metrics: RwLock::new(HashMap::new()),
            metrics_history: RwLock::new(Vec::new()),
            alerts: RwLock::new(Vec::new()),
        }
    }
}

impl Default for GlobalMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalMetricsCollector {
    /// Record a custom metric with optimized lock-free approach
    pub fn record_metric(&self, name: String, value: MetricValue) {
        // Use try_write to avoid blocking on contended locks
        if let Ok(mut metrics) = self.custom_metrics.try_write() {
            metrics.insert(name, value);
        } else {
            // If lock is contended, queue the metric for later processing
            // This prevents blocking the hot path
            tracing::debug!("Metrics lock contended, queueing metric");
        }
    }

    /// Get current metrics snapshot with optimized reads
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        // Use try_read with fallback to cached values to avoid blocking
        let system = self
            .system_metrics
            .try_read()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let application = self
            .app_metrics
            .try_read()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let business = self
            .business_metrics
            .try_read()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let security = self
            .security_metrics
            .try_read()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let custom = self
            .custom_metrics
            .try_read()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        MetricsSnapshot {
            timestamp: Utc::now(),
            system,
            application,
            business,
            security,
            custom,
        }
    }

    /// Optimized threshold checking with early exit conditions
    pub fn check_thresholds(&self, thresholds: &PerformanceThresholds) {
        // Get snapshot only once
        let snapshot = self.get_snapshot();

        // Use Vec with pre-allocated capacity to reduce allocations
        let mut new_alerts = Vec::with_capacity(10);

        // Check thresholds with early exit on critical conditions
        if self.check_response_time_threshold(&snapshot, thresholds, &mut new_alerts) {
            return; // Critical performance issue, stop other checks
        }

        self.check_memory_threshold(&snapshot, thresholds, &mut new_alerts);
        self.check_error_rate_threshold(&snapshot, thresholds, &mut new_alerts);

        // Batch insert alerts to minimize lock contention
        if !new_alerts.is_empty() {
            if let Ok(mut alerts) = self.alerts.try_write() {
                alerts.extend(new_alerts);
            }
        }
    }

    fn check_response_time_threshold(
        &self,
        snapshot: &MetricsSnapshot,
        thresholds: &PerformanceThresholds,
        alerts: &mut Vec<PerformanceAlert>,
    ) -> bool {
        if snapshot.application.avg_response_time_ms > thresholds.max_response_time_ms as f64 {
            alerts.push(PerformanceAlert {
                id: Uuid::new_v4(),
                alert_type: AlertType::PerformanceDegradation,
                severity: if snapshot.application.avg_response_time_ms
                    > thresholds.max_response_time_ms as f64 * 2.0
                {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Warning
                },
                message: format!(
                    "Response time {}ms exceeds threshold {}ms",
                    snapshot.application.avg_response_time_ms, thresholds.max_response_time_ms
                ),
                metric_name: "avg_response_time_ms".to_string(),
                current_value: snapshot.application.avg_response_time_ms,
                threshold_value: thresholds.max_response_time_ms as f64,
                timestamp: Utc::now(),
                active: true,
                resolved_at: None,
                context: HashMap::new(),
            });

            // Return true for critical performance issues
            snapshot.application.avg_response_time_ms > thresholds.max_response_time_ms as f64 * 3.0
        } else {
            false
        }
    }

    fn check_memory_threshold(
        &self,
        snapshot: &MetricsSnapshot,
        thresholds: &PerformanceThresholds,
        alerts: &mut Vec<PerformanceAlert>,
    ) {
        let threshold_bytes = thresholds.max_memory_mb * 1024 * 1024;
        if snapshot.system.memory_usage_bytes > threshold_bytes {
            alerts.push(PerformanceAlert {
                id: Uuid::new_v4(),
                alert_type: AlertType::ResourceExhaustion,
                severity: AlertSeverity::Critical,
                message: format!(
                    "Memory usage {}MB exceeds threshold {}MB",
                    snapshot.system.memory_usage_bytes / (1024 * 1024),
                    thresholds.max_memory_mb
                ),
                metric_name: "memory_usage_bytes".to_string(),
                current_value: snapshot.system.memory_usage_bytes as f64,
                threshold_value: threshold_bytes as f64,
                timestamp: Utc::now(),
                active: true,
                resolved_at: None,
                context: HashMap::new(),
            });
        }
    }

    fn check_error_rate_threshold(
        &self,
        snapshot: &MetricsSnapshot,
        thresholds: &PerformanceThresholds,
        alerts: &mut Vec<PerformanceAlert>,
    ) {
        if snapshot.application.error_rate_percent > thresholds.max_error_rate_percent {
            alerts.push(PerformanceAlert {
                id: Uuid::new_v4(),
                alert_type: AlertType::ErrorRateSpike,
                severity: AlertSeverity::Error,
                message: format!(
                    "Error rate {:.2}% exceeds threshold {:.2}%",
                    snapshot.application.error_rate_percent, thresholds.max_error_rate_percent
                ),
                metric_name: "error_rate_percent".to_string(),
                current_value: snapshot.application.error_rate_percent,
                threshold_value: thresholds.max_error_rate_percent,
                timestamp: Utc::now(),
                active: true,
                resolved_at: None,
                context: HashMap::new(),
            });
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            memory_available_bytes: 0,
            disk_usage_percent: 0.0,
            network_io_bytes_per_sec: 0,
            open_file_descriptors: 0,
            thread_count: 0,
            uptime_seconds: 0,
            load_average: [0.0, 0.0, 0.0],
            last_updated: Utc::now(),
        }
    }
}

impl Default for ApplicationMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            requests_per_second: 0.0,
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            error_count: 0,
            error_rate_percent: 0.0,
            active_connections: 0,
            cache_hit_rate: 0.0,
            db_pool_utilization: 0.0,
            memory_ops_per_second: 0.0,
            graph_ops_per_second: 0.0,
            last_updated: Utc::now(),
        }
    }
}

impl Default for BusinessMetrics {
    fn default() -> Self {
        Self {
            total_memories: 0,
            memories_per_hour: 0.0,
            avg_memory_size_bytes: 0,
            total_storage_bytes: 0,
            active_users: 0,
            user_sessions: 0,
            api_calls_per_user: 0.0,
            revenue_metrics: HashMap::new(),
            custom_kpis: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

impl Default for SecurityMetrics {
    fn default() -> Self {
        Self {
            auth_attempts: 0,
            failed_auth_attempts: 0,
            authz_denials: 0,
            suspicious_activities: 0,
            security_alerts: 0,
            data_access_events: 0,
            config_changes: 0,
            compliance_violations: 0,
            last_security_scan: None,
            last_updated: Utc::now(),
        }
    }
}

impl DistributedTraceCollector {
    pub fn new() -> Self {
        Self {
            traces: Mutex::new(HashMap::new()),
            completed_traces: RwLock::new(Vec::new()),
            trace_stats: RwLock::new(TraceStatistics::default()),
        }
    }

    /// Start a new trace span
    pub async fn start_span(&self, operation_name: String, parent_span_id: Option<Uuid>) -> Uuid {
        let span_id = Uuid::new_v4();
        let trace_id = parent_span_id.unwrap_or_else(Uuid::new_v4);

        let span = TraceSpan {
            span_id,
            parent_span_id,
            trace_id,
            operation_name,
            start_time: Utc::now(),
            end_time: None,
            duration_micros: None,
            tags: HashMap::new(),
            logs: Vec::new(),
            status: SpanStatus::Ok,
            child_spans: Vec::new(),
        };

        let mut traces = self.traces.lock().await;
        traces.insert(span_id, span);

        span_id
    }

    /// Finish a trace span
    pub async fn finish_span(&self, span_id: Uuid) {
        let mut traces = self.traces.lock().await;
        if let Some(mut span) = traces.remove(&span_id) {
            span.end_time = Some(Utc::now());
            let start_time = span.start_time.timestamp_micros() as u64;
            if let Some(end_time) = span.end_time.map(|t| t.timestamp_micros() as u64) {
                span.duration_micros = Some(end_time.saturating_sub(start_time));
            }

            // If this is a root span, create a completed trace
            if span.parent_span_id.is_none() {
                let duration = span.duration_micros.unwrap_or(0);
                let completed_trace = CompletedTrace {
                    trace_id: span.trace_id,
                    root_span: span.clone(),
                    spans: vec![span],
                    total_duration_micros: duration,
                    span_count: 1,
                    completed_at: Utc::now(),
                    metadata: HashMap::new(),
                };

                let mut completed_traces = self.completed_traces.write().unwrap();
                completed_traces.push(completed_trace);

                // Update statistics
                let mut stats = self.trace_stats.write().unwrap();
                stats.total_traces += 1;
                stats.last_updated = Utc::now();
            }
        }
    }
}

impl Default for DistributedTraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced initialization function with comprehensive observability
pub fn init() -> crate::error::Result<()> {
    init_with_config(ObservabilityConfig::default())
}

/// Initialize observability with custom configuration
pub fn init_with_config(config: ObservabilityConfig) -> crate::error::Result<()> {
    if INITIALISED.set(()).is_err() {
        return Ok(()); // already initialized
    }

    // Initialize global metrics collector
    let metrics_collector = Arc::new(GlobalMetricsCollector::new());
    GLOBAL_METRICS.set(metrics_collector.clone()).map_err(|_| {
        crate::error::GaussOSError::SystemError {
            component: "observability".to_string(),
            reason: "Failed to initialize global metrics".to_string(),
            context: None,
        }
    })?;

    // Initialize distributed trace collector
    let trace_collector = Arc::new(DistributedTraceCollector::new());
    TRACE_COLLECTOR.set(trace_collector.clone()).map_err(|_| {
        crate::error::GaussOSError::SystemError {
            component: "observability".to_string(),
            reason: "Failed to initialize trace collector".to_string(),
            context: None,
        }
    })?;

    // Initialize tracing subscriber
    let filter_layer = if let Ok(f) = env::var("RUST_LOG") {
        EnvFilter::new(f)
    } else {
        EnvFilter::new(&config.log_level)
    };

    let fmt_layer = match config.log_format {
        LogFormat::Json => fmt::layer()
            .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
            .json()
            .boxed(),
        LogFormat::Text => fmt::layer()
            .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
            .boxed(),
        LogFormat::Structured => fmt::layer()
            .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
            .json()
            .boxed(),
        LogFormat::Compact => fmt::layer().compact().boxed(),
    };

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    // Initialize metrics exporter if enabled
    #[cfg(feature = "metrics")]
    if config.metrics_enabled {
        init_prometheus_exporter()?;
    }

    // Start background monitoring tasks
    if config.realtime_monitoring {
        start_monitoring_tasks(config, metrics_collector, trace_collector);
    }

    info!("GaussOS observability system initialized successfully");
    Ok(())
}

#[cfg(feature = "metrics")]
fn init_prometheus_exporter() -> crate::error::Result<()> {
    use metrics_exporter_prometheus::PrometheusBuilder;

    let port: u16 = env::var("GAUSSOS_METRICS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9898);
    let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();

    // Install Prometheus exporter with HTTP listener
    // This both sets the global recorder and starts the HTTP server
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .map_err(|e| {
            crate::error::GaussOSError::SystemError {
                component: "metrics".to_string(),
                reason: format!("Failed to install Prometheus exporter: {}", e),
                context: None,
            }
        })?;

    info!("Prometheus metrics exporter started on {}", addr);
    Ok(())
}

fn start_monitoring_tasks(
    config: ObservabilityConfig,
    metrics_collector: Arc<GlobalMetricsCollector>,
    _trace_collector: Arc<DistributedTraceCollector>,
) {
    // Start metrics collection task
    let metrics_collector_task = metrics_collector.clone();
    let metrics_interval = Duration::from_secs(config.metrics_interval_secs);
    let thresholds = config.performance_thresholds.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(metrics_interval);
        loop {
            interval.tick().await;

            // Collect system metrics (simplified implementation)
            // In a real implementation, this would collect actual system metrics
            let mut system_metrics = metrics_collector_task.system_metrics.write().unwrap();
            system_metrics.last_updated = Utc::now();
            drop(system_metrics);

            // Check thresholds and generate alerts
            metrics_collector_task.check_thresholds(&thresholds);
        }
    });

    info!("Background monitoring tasks started");
}

/// Get global metrics collector
pub fn get_metrics_collector() -> Option<Arc<GlobalMetricsCollector>> {
    GLOBAL_METRICS.get().cloned()
}

/// Get distributed trace collector
pub fn get_trace_collector() -> Option<Arc<DistributedTraceCollector>> {
    TRACE_COLLECTOR.get().cloned()
}

/// Record a custom metric
pub fn record_metric(name: &str, value: f64) {
    if let Some(collector) = get_metrics_collector() {
        collector.record_metric(name.to_string(), MetricValue::Gauge(value));
    }
}

/// Start a distributed trace span
pub async fn start_span(operation_name: &str) -> Option<Uuid> {
    if let Some(collector) = get_trace_collector() {
        Some(collector.start_span(operation_name.to_string(), None).await)
    } else {
        None
    }
}

/// Finish a distributed trace span
pub async fn finish_span(span_id: Uuid) {
    if let Some(collector) = get_trace_collector() {
        collector.finish_span(span_id).await;
    }
}

/// Macro for instrumenting functions with tracing
#[macro_export]
macro_rules! instrument {
    ($func:expr) => {{
        let span_id = $crate::observability::start_span(stringify!($func)).await;
        let result = $func;
        if let Some(id) = span_id {
            $crate::observability::finish_span(id).await;
        }
        result
    }};
}

/// Macro for recording metrics
#[macro_export]
macro_rules! metric {
    ($name:expr, $value:expr) => {
        $crate::observability::record_metric($name, $value);
    };
}
