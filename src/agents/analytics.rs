// src/agents/analytics.rs
//! Agent Analytics and Performance Monitoring
//! Tracks agent performance, usage patterns, and system metrics

use crate::error::{GaussOSError, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Agent analytics system for tracking performance and usage
pub struct AgentAnalytics {
    metrics: Arc<RwLock<HashMap<Uuid, AgentMetrics>>>,
    usage_stats: Arc<RwLock<HashMap<Uuid, UsageStats>>>,
    performance_history: Arc<RwLock<HashMap<Uuid, Vec<PerformanceSnapshot>>>>,
    config: AnalyticsConfig,
}

/// Configuration for analytics system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    pub enable_real_time_metrics: bool,
    pub metrics_retention_days: u32,
    pub performance_snapshot_interval: std::time::Duration,
    pub enable_detailed_logging: bool,
    pub enable_anomaly_detection: bool,
    pub anomaly_threshold: f64,
}

/// Comprehensive agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: Uuid,
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: f64,
    pub min_execution_time_ms: u64,
    pub max_execution_time_ms: u64,
    pub last_activity: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub error_rate: f64,
    pub throughput_per_hour: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub tool_usage_stats: HashMap<String, ToolUsageMetrics>,
}

/// Tool-specific usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageMetrics {
    pub tool_name: String,
    pub usage_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub average_execution_time_ms: f64,
    pub last_used: DateTime<Utc>,
}

/// Usage statistics for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub agent_id: Uuid,
    pub daily_stats: HashMap<String, DailyUsage>, // Date -> Usage
    pub hourly_stats: HashMap<u8, HourlyUsage>,   // Hour -> Usage
    pub user_interactions: HashMap<Uuid, UserInteractionStats>,
    pub conversation_stats: ConversationStats,
    pub resource_usage: ResourceUsage,
}

/// Daily usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub task_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_execution_time_ms: u64,
    pub unique_users: u32,
    pub peak_hour: u8,
    pub peak_tasks_per_hour: u64,
}

/// Hourly usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyUsage {
    pub hour: u8,
    pub task_count: u64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub concurrent_users: u32,
}

/// User interaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionStats {
    pub user_id: Uuid,
    pub total_interactions: u64,
    pub total_time_spent_ms: u64,
    pub favorite_tools: Vec<String>,
    pub satisfaction_score: Option<f64>,
    pub last_interaction: DateTime<Utc>,
}

/// Conversation-related statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStats {
    pub total_conversations: u64,
    pub active_conversations: u64,
    pub average_conversation_length: f64,
    pub average_response_time_ms: f64,
    pub conversation_completion_rate: f64,
    pub user_satisfaction_scores: Vec<f64>,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_mb: f64,
    pub network_io_mb: f64,
    pub database_queries: u64,
    pub cache_hit_rate: f64,
}

/// Performance metrics for specific time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: DateTime<Utc>,
    pub response_time_ms: u64,
    pub throughput_per_second: f64,
    pub error_rate: f64,
    pub resource_utilization: f64,
    pub user_satisfaction: Option<f64>,
    pub anomalies_detected: Vec<AnomalyReport>,
}

/// Performance snapshot for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub metrics: AgentMetrics,
    pub system_load: f64,
    pub memory_pressure: f64,
    pub active_connections: u32,
    pub queue_length: u32,
}

/// Anomaly detection report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub metric_value: f64,
    pub expected_range: (f64, f64),
    pub confidence: f64,
}

/// Types of anomalies that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    HighResponseTime,
    HighErrorRate,
    UnusualUsagePattern,
    ResourceSpike,
    PerformanceDegradation,
    UnexpectedDowntime,
}

/// Severity levels for anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AgentAnalytics {
    /// Create a new analytics system
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            performance_history: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Record a task execution
    pub async fn record_task_execution(
        &self,
        agent_id: &Uuid,
        success: bool,
        execution_time_ms: u64,
        tool_name: Option<&str>,
    ) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        let agent_metrics = metrics.entry(*agent_id).or_insert_with(|| AgentMetrics {
            agent_id: *agent_id,
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            total_execution_time_ms: 0,
            average_execution_time_ms: 0.0,
            min_execution_time_ms: u64::MAX,
            max_execution_time_ms: 0,
            last_activity: Utc::now(),
            uptime_seconds: 0,
            error_rate: 0.0,
            throughput_per_hour: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            tool_usage_stats: HashMap::new(),
        });

        // Update basic metrics
        agent_metrics.total_tasks += 1;
        if success {
            agent_metrics.successful_tasks += 1;
        } else {
            agent_metrics.failed_tasks += 1;
        }

        // Update timing metrics
        agent_metrics.total_execution_time_ms += execution_time_ms;
        agent_metrics.average_execution_time_ms =
            agent_metrics.total_execution_time_ms as f64 / agent_metrics.total_tasks as f64;
        agent_metrics.min_execution_time_ms =
            agent_metrics.min_execution_time_ms.min(execution_time_ms);
        agent_metrics.max_execution_time_ms =
            agent_metrics.max_execution_time_ms.max(execution_time_ms);
        agent_metrics.last_activity = Utc::now();

        // Update error rate
        agent_metrics.error_rate =
            agent_metrics.failed_tasks as f64 / agent_metrics.total_tasks as f64;

        // Update tool-specific metrics
        if let Some(tool) = tool_name {
            let tool_metrics = agent_metrics
                .tool_usage_stats
                .entry(tool.to_string())
                .or_insert_with(|| ToolUsageMetrics {
                    tool_name: tool.to_string(),
                    usage_count: 0,
                    success_count: 0,
                    failure_count: 0,
                    average_execution_time_ms: 0.0,
                    last_used: Utc::now(),
                });

            tool_metrics.usage_count += 1;
            if success {
                tool_metrics.success_count += 1;
            } else {
                tool_metrics.failure_count += 1;
            }
            tool_metrics.average_execution_time_ms = (tool_metrics.average_execution_time_ms
                * (tool_metrics.usage_count - 1) as f64
                + execution_time_ms as f64)
                / tool_metrics.usage_count as f64;
            tool_metrics.last_used = Utc::now();
        }

        // Record daily usage
        self.record_daily_usage(agent_id, success, execution_time_ms)
            .await?;

        // Check for anomalies
        if self.config.enable_anomaly_detection {
            self.check_for_anomalies(agent_id, agent_metrics).await?;
        }

        Ok(())
    }

    /// Get metrics for a specific agent
    pub async fn get_agent_metrics(&self, agent_id: &Uuid) -> Result<Option<AgentMetrics>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.get(agent_id).cloned())
    }

    /// Get usage statistics for a specific agent
    pub async fn get_usage_stats(&self, agent_id: &Uuid) -> Result<Option<UsageStats>> {
        let stats = self.usage_stats.read().await;
        Ok(stats.get(agent_id).cloned())
    }

    /// Get performance metrics for a time period
    pub async fn get_performance_metrics(
        &self,
        agent_id: &Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<PerformanceSnapshot>> {
        let history = self.performance_history.read().await;
        if let Some(snapshots) = history.get(agent_id) {
            let filtered: Vec<PerformanceSnapshot> = snapshots
                .iter()
                .filter(|snapshot| {
                    snapshot.timestamp >= start_time && snapshot.timestamp <= end_time
                })
                .cloned()
                .collect();
            Ok(filtered)
        } else {
            Ok(Vec::new())
        }
    }

    /// Generate a comprehensive performance report
    pub async fn generate_performance_report(&self, agent_id: &Uuid) -> Result<PerformanceReport> {
        let metrics = self.get_agent_metrics(agent_id).await?.ok_or_else(|| {
            GaussOSError::NotFound(format!("No metrics found for agent {}", agent_id))
        })?;

        let usage_stats = self
            .get_usage_stats(agent_id)
            .await?
            .unwrap_or_else(|| UsageStats {
                agent_id: *agent_id,
                daily_stats: HashMap::new(),
                hourly_stats: HashMap::new(),
                user_interactions: HashMap::new(),
                conversation_stats: ConversationStats {
                    total_conversations: 0,
                    active_conversations: 0,
                    average_conversation_length: 0.0,
                    average_response_time_ms: 0.0,
                    conversation_completion_rate: 0.0,
                    user_satisfaction_scores: Vec::new(),
                },
                resource_usage: ResourceUsage {
                    memory_usage_mb: 0.0,
                    cpu_usage_percent: 0.0,
                    disk_usage_mb: 0.0,
                    network_io_mb: 0.0,
                    database_queries: 0,
                    cache_hit_rate: 0.0,
                },
            });

        let now = Utc::now();
        let week_ago = now - Duration::days(7);
        let performance_history = self
            .get_performance_metrics(agent_id, week_ago, now)
            .await?;

        Ok(PerformanceReport {
            agent_id: *agent_id,
            generated_at: now,
            metrics,
            usage_stats,
            performance_trends: self.calculate_performance_trends(&performance_history),
            recommendations: self.generate_recommendations(agent_id).await?,
        })
    }

    /// Record daily usage statistics
    async fn record_daily_usage(
        &self,
        agent_id: &Uuid,
        success: bool,
        execution_time_ms: u64,
    ) -> Result<()> {
        let mut usage_stats = self.usage_stats.write().await;
        let stats = usage_stats.entry(*agent_id).or_insert_with(|| UsageStats {
            agent_id: *agent_id,
            daily_stats: HashMap::new(),
            hourly_stats: HashMap::new(),
            user_interactions: HashMap::new(),
            conversation_stats: ConversationStats {
                total_conversations: 0,
                active_conversations: 0,
                average_conversation_length: 0.0,
                average_response_time_ms: 0.0,
                conversation_completion_rate: 0.0,
                user_satisfaction_scores: Vec::new(),
            },
            resource_usage: ResourceUsage {
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                disk_usage_mb: 0.0,
                network_io_mb: 0.0,
                database_queries: 0,
                cache_hit_rate: 0.0,
            },
        });

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let daily_usage = stats
            .daily_stats
            .entry(today.clone())
            .or_insert_with(|| DailyUsage {
                date: today,
                task_count: 0,
                success_count: 0,
                failure_count: 0,
                total_execution_time_ms: 0,
                unique_users: 0,
                peak_hour: 0,
                peak_tasks_per_hour: 0,
            });

        daily_usage.task_count += 1;
        if success {
            daily_usage.success_count += 1;
        } else {
            daily_usage.failure_count += 1;
        }
        daily_usage.total_execution_time_ms += execution_time_ms;

        Ok(())
    }

    /// Check for performance anomalies
    async fn check_for_anomalies(&self, agent_id: &Uuid, metrics: &AgentMetrics) -> Result<()> {
        let mut anomalies = Vec::new();

        // Check for high error rate
        if metrics.error_rate > self.config.anomaly_threshold {
            anomalies.push(AnomalyReport {
                anomaly_type: AnomalyType::HighErrorRate,
                severity: if metrics.error_rate > 0.5 {
                    AnomalySeverity::Critical
                } else {
                    AnomalySeverity::High
                },
                description: format!("Error rate is {:.2}%", metrics.error_rate * 100.0),
                detected_at: Utc::now(),
                metric_value: metrics.error_rate,
                expected_range: (0.0, self.config.anomaly_threshold),
                confidence: 0.95,
            });
        }

        // Check for high response time
        if metrics.average_execution_time_ms > 5000.0 {
            anomalies.push(AnomalyReport {
                anomaly_type: AnomalyType::HighResponseTime,
                severity: AnomalySeverity::Medium,
                description: format!(
                    "Average response time is {:.0}ms",
                    metrics.average_execution_time_ms
                ),
                detected_at: Utc::now(),
                metric_value: metrics.average_execution_time_ms,
                expected_range: (0.0, 5000.0),
                confidence: 0.9,
            });
        }

        // Log anomalies
        for anomaly in anomalies {
            tracing::warn!(
                "Anomaly detected for agent {}: {:?} - {}",
                agent_id,
                anomaly.anomaly_type,
                anomaly.description
            );
        }

        Ok(())
    }

    /// Calculate performance trends
    fn calculate_performance_trends(&self, history: &[PerformanceSnapshot]) -> PerformanceTrends {
        if history.is_empty() {
            return PerformanceTrends::default();
        }

        let response_times: Vec<f64> = history
            .iter()
            .map(|s| s.metrics.average_execution_time_ms)
            .collect();

        let error_rates: Vec<f64> = history.iter().map(|s| s.metrics.error_rate).collect();

        PerformanceTrends {
            response_time_trend: self.calculate_trend(&response_times),
            error_rate_trend: self.calculate_trend(&error_rates),
            throughput_trend: 0.0,     // Placeholder
            resource_usage_trend: 0.0, // Placeholder
        }
    }

    /// Calculate trend for a series of values
    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        // Linear regression slope
        (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2))
    }

    /// Generate performance recommendations
    async fn generate_recommendations(&self, agent_id: &Uuid) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();

        if let Some(metrics) = self.get_agent_metrics(agent_id).await? {
            if metrics.error_rate > 0.1 {
                recommendations.push(
                    "Consider reviewing error handling and adding more robust retry logic"
                        .to_string(),
                );
            }

            if metrics.average_execution_time_ms > 3000.0 {
                recommendations.push(
                    "Optimize tool execution times or consider caching frequently accessed data"
                        .to_string(),
                );
            }

            if metrics.total_tasks > 1000 && metrics.tool_usage_stats.len() < 3 {
                recommendations.push(
                    "Consider expanding the agent's tool repertoire for better versatility"
                        .to_string(),
                );
            }
        }

        Ok(recommendations)
    }
}

/// Performance trends analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceTrends {
    pub response_time_trend: f64,
    pub error_rate_trend: f64,
    pub throughput_trend: f64,
    pub resource_usage_trend: f64,
}

/// Comprehensive performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub agent_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub metrics: AgentMetrics,
    pub usage_stats: UsageStats,
    pub performance_trends: PerformanceTrends,
    pub recommendations: Vec<String>,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enable_real_time_metrics: true,
            metrics_retention_days: 30,
            performance_snapshot_interval: std::time::Duration::from_secs(300), // 5 minutes
            enable_detailed_logging: true,
            enable_anomaly_detection: true,
            anomaly_threshold: 0.1, // 10% error rate threshold
        }
    }
}
