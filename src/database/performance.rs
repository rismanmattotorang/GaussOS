// src/database/performance.rs
//! Database Performance Monitoring
//! Provides query profiling, slow query detection, and performance analytics

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Performance monitor for database operations
pub struct PerformanceMonitor {
    config: super::PerformanceConfig,
    query_profiler: QueryProfiler,
    slow_query_logger: SlowQueryLogger,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// Query profiler for analyzing query performance
pub struct QueryProfiler {
    enabled: bool,
    sample_rate: f64,
    profiles: Arc<RwLock<HashMap<String, QueryProfile>>>,
}

/// Slow query logger
pub struct SlowQueryLogger {
    threshold_ms: u64,
    slow_queries: Arc<RwLock<Vec<SlowQuery>>>,
}

/// Query performance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    pub query_hash: String,
    pub query_text: String,
    pub execution_count: u64,
    pub total_time_ms: u64,
    pub average_time_ms: f64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub error_count: u64,
    pub rows_examined_avg: f64,
    pub rows_returned_avg: f64,
}

/// Slow query record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQuery {
    pub id: String,
    pub query_text: String,
    pub execution_time_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub database: String,
    pub user: Option<String>,
    pub explain_plan: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_queries: u64,
    pub slow_queries: u64,
    pub error_queries: u64,
    pub average_query_time_ms: f64,
    pub p95_query_time_ms: f64,
    pub p99_query_time_ms: f64,
    pub queries_per_second: f64,
    pub cache_hit_rate: f64,
    pub index_usage_rate: f64,
    pub last_updated: DateTime<Utc>,
}

/// Performance health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHealthStatus {
    pub status: super::HealthLevel,
    pub average_query_time_ms: f64,
    pub slow_queries_per_minute: f64,
    pub error_rate: f64,
}

impl PerformanceMonitor {
    pub fn new(config: super::PerformanceConfig) -> Self {
        Self {
            query_profiler: QueryProfiler::new(config.enable_query_profiling, config.sample_rate),
            slow_query_logger: SlowQueryLogger::new(config.slow_query_threshold_ms),
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                total_queries: 0,
                slow_queries: 0,
                error_queries: 0,
                average_query_time_ms: 0.0,
                p95_query_time_ms: 0.0,
                p99_query_time_ms: 0.0,
                queries_per_second: 0.0,
                cache_hit_rate: 0.0,
                index_usage_rate: 0.0,
                last_updated: Utc::now(),
            })),
            config,
        }
    }

    pub async fn record_query(&self, query: &str, duration_ms: u64, success: bool) -> Result<()> {
        // Update metrics
        let mut metrics = self.metrics.write().unwrap();
        metrics.total_queries += 1;
        if !success {
            metrics.error_queries += 1;
        }

        // Update average query time
        metrics.average_query_time_ms = (metrics.average_query_time_ms
            * (metrics.total_queries - 1) as f64
            + duration_ms as f64)
            / metrics.total_queries as f64;

        metrics.last_updated = Utc::now();

        // Record slow query if threshold exceeded
        if duration_ms > self.config.slow_query_threshold_ms {
            metrics.slow_queries += 1;
            self.slow_query_logger
                .record_slow_query(query, duration_ms)
                .await?;
        }

        // Profile query if enabled
        if self.config.enable_query_profiling {
            self.query_profiler
                .record_query(query, duration_ms, success)
                .await?;
        }

        Ok(())
    }

    pub async fn get_health_status(&self) -> Result<PerformanceHealthStatus> {
        let metrics = self.metrics.read().unwrap();

        let error_rate = if metrics.total_queries > 0 {
            metrics.error_queries as f64 / metrics.total_queries as f64
        } else {
            0.0
        };

        let status = if metrics.average_query_time_ms < 100.0 && error_rate < 0.01 {
            super::HealthLevel::Healthy
        } else if metrics.average_query_time_ms < 1000.0 && error_rate < 0.05 {
            super::HealthLevel::Warning
        } else {
            super::HealthLevel::Critical
        };

        Ok(PerformanceHealthStatus {
            status,
            average_query_time_ms: metrics.average_query_time_ms,
            slow_queries_per_minute: 0.0, // TODO: Calculate actual rate
            error_rate,
        })
    }

    pub async fn get_slow_queries(&self, limit: Option<u32>) -> Result<Vec<SlowQuery>> {
        self.slow_query_logger.get_slow_queries(limit).await
    }
}

impl QueryProfiler {
    pub fn new(enabled: bool, sample_rate: f64) -> Self {
        Self {
            enabled,
            sample_rate,
            profiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_query(&self, query: &str, duration_ms: u64, success: bool) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Simple sampling
        if rand::random::<f64>() > self.sample_rate {
            return Ok(());
        }

        let query_hash = format!("{:x}", md5::compute(query.as_bytes()));
        let mut profiles = self.profiles.write().unwrap();

        let profile = profiles
            .entry(query_hash.clone())
            .or_insert_with(|| QueryProfile {
                query_hash: query_hash.clone(),
                query_text: query.to_string(),
                execution_count: 0,
                total_time_ms: 0,
                average_time_ms: 0.0,
                min_time_ms: u64::MAX,
                max_time_ms: 0,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                error_count: 0,
                rows_examined_avg: 0.0,
                rows_returned_avg: 0.0,
            });

        profile.execution_count += 1;
        profile.total_time_ms += duration_ms;
        profile.average_time_ms = profile.total_time_ms as f64 / profile.execution_count as f64;
        profile.min_time_ms = profile.min_time_ms.min(duration_ms);
        profile.max_time_ms = profile.max_time_ms.max(duration_ms);
        profile.last_seen = Utc::now();

        if !success {
            profile.error_count += 1;
        }

        Ok(())
    }
}

impl SlowQueryLogger {
    pub fn new(threshold_ms: u64) -> Self {
        Self {
            threshold_ms,
            slow_queries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn record_slow_query(&self, query: &str, duration_ms: u64) -> Result<()> {
        let slow_query = SlowQuery {
            id: uuid::Uuid::new_v4().to_string(),
            query_text: query.to_string(),
            execution_time_ms: duration_ms,
            timestamp: Utc::now(),
            database: "default".to_string(),
            user: None,
            explain_plan: None,
            parameters: None,
        };

        let mut slow_queries = self.slow_queries.write().unwrap();
        slow_queries.push(slow_query);

        // Keep only the last 1000 slow queries
        if slow_queries.len() > 1000 {
            let excess = slow_queries.len() - 1000;
            slow_queries.drain(0..excess);
        }

        Ok(())
    }

    pub async fn get_slow_queries(&self, limit: Option<u32>) -> Result<Vec<SlowQuery>> {
        let slow_queries = self.slow_queries.read().unwrap();
        let limit = limit.unwrap_or(100) as usize;

        Ok(slow_queries.iter().rev().take(limit).cloned().collect())
    }
}
