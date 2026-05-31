// src/database/cache.rs
//! Database Query Cache
//! Provides intelligent caching of query results with TTL and eviction policies

use crate::error::{GaussOSError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

/// Query cache for database results
pub struct QueryCache {
    config: CacheConfig,
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    metrics: Arc<RwLock<CacheMetrics>>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_size_mb: u64,
    pub default_ttl_seconds: u64,
    pub eviction_policy: EvictionPolicy,
    pub enable_compression: bool,
    pub cache_strategy: CacheStrategy,
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Ttl,
    Random,
}

/// Cache strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    ReadThrough,
    WriteThrough,
    WriteBack,
    CacheAside,
}

/// Cache metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub eviction_count: u64,
    pub memory_usage_mb: f64,
    pub entry_count: u64,
    pub average_ttl_seconds: f64,
}

/// Cached result entry
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
}

/// Cache health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealthStatus {
    pub status: super::HealthLevel,
    pub hit_rate: f64,
    pub memory_usage_mb: f64,
    pub evictions_per_minute: f64,
}

impl QueryCache {
    pub async fn new(config: CacheConfig) -> Result<Self> {
        Ok(Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(CacheMetrics {
                hit_count: 0,
                miss_count: 0,
                hit_rate: 0.0,
                eviction_count: 0,
                memory_usage_mb: 0.0,
                entry_count: 0,
                average_ttl_seconds: 0.0,
            })),
        })
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut cache = self.cache.write().await;
        if let Some(entry) = cache.get_mut(key) {
            if entry.expires_at > Utc::now() {
                entry.access_count += 1;
                entry.last_accessed = Utc::now();

                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.hit_count += 1;

                return Some(entry.data.clone());
            } else {
                // Entry expired, remove it
                cache.remove(key);
            }
        }

        // Cache miss
        let mut metrics = self.metrics.write().await;
        metrics.miss_count += 1;
        None
    }

    pub async fn put(&self, key: String, data: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let ttl = ttl.unwrap_or_else(|| Duration::from_secs(self.config.default_ttl_seconds));
        let expires_at = Utc::now()
            + chrono::Duration::from_std(ttl).map_err(|e| {
                GaussOSError::system_error("cache".to_string(), format!("Invalid TTL: {}", e))
            })?;

        let entry = CachedResult {
            data,
            created_at: Utc::now(),
            expires_at,
            access_count: 0,
            last_accessed: Utc::now(),
        };

        let mut cache = self.cache.write().await;
        cache.insert(key, entry);

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.entry_count = cache.len() as u64;

        Ok(())
    }

    pub async fn get_health_status(&self) -> Result<CacheHealthStatus> {
        let metrics = self.metrics.read().await;
        let hit_rate = if metrics.hit_count + metrics.miss_count > 0 {
            metrics.hit_count as f64 / (metrics.hit_count + metrics.miss_count) as f64
        } else {
            0.0
        };

        let status = if hit_rate > 0.8 {
            super::HealthLevel::Healthy
        } else if hit_rate > 0.5 {
            super::HealthLevel::Warning
        } else {
            super::HealthLevel::Critical
        };

        Ok(CacheHealthStatus {
            status,
            hit_rate,
            memory_usage_mb: metrics.memory_usage_mb,
            evictions_per_minute: 0.0, // TODO: Calculate actual evictions per minute
        })
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_mb: 100,
            default_ttl_seconds: 300,
            eviction_policy: EvictionPolicy::Lru,
            enable_compression: true,
            cache_strategy: CacheStrategy::ReadThrough,
        }
    }
}
