//! Safe lock-free data structures for high-performance concurrent operations

use crate::core::MemCube;
use crate::error::{GaussOSError, Result};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// High-performance lock-free memory cache
pub struct LockFreeMemoryCache {
    storage: Arc<DashMap<Uuid, CacheEntry>>,
    metadata: Arc<CacheMetadata>,
    metrics: Arc<AtomicMetrics>,
    config: CacheConfig,
}

#[derive(Debug)]
pub struct CacheEntry {
    pub data: Arc<MemCube>,
    pub last_accessed: AtomicU64,
    pub access_count: AtomicU64,
    pub size_bytes: usize,
}

#[derive(Debug)]
pub struct CacheMetadata {
    pub total_size: AtomicUsize,
    pub entry_count: AtomicUsize,
    pub eviction_count: AtomicU64,
}

#[derive(Debug)]
pub struct CacheConfig {
    pub max_size: usize,
    pub max_memory_bytes: usize,
    pub enable_metrics: bool,
    pub enable_eviction: bool,
}

impl LockFreeMemoryCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            storage: Arc::new(DashMap::new()),
            metadata: Arc::new(CacheMetadata {
                total_size: AtomicUsize::new(0),
                entry_count: AtomicUsize::new(0),
                eviction_count: AtomicU64::new(0),
            }),
            metrics: Arc::new(AtomicMetrics::new()),
            config,
        }
    }

    pub fn get(&self, key: &Uuid) -> Option<Arc<MemCube>> {
        if let Some(entry) = self.storage.get(key) {
            // Safe timestamp conversion with fallback
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            entry.last_accessed.store(now, Ordering::Relaxed);
            entry.access_count.fetch_add(1, Ordering::Relaxed);

            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.data.clone())
        } else {
            self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&self, key: Uuid, value: Arc<MemCube>) -> Result<bool> {
        let entry_size = std::mem::size_of_val(&*value) + value.payload.len();

        if self.needs_eviction(entry_size) && self.config.enable_eviction {
            self.evict_lru()?;
        }

        // Safe timestamp with fallback
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = CacheEntry {
            data: value,
            last_accessed: AtomicU64::new(now),
            access_count: AtomicU64::new(1),
            size_bytes: entry_size,
        };

        let inserted = self.storage.insert(key, entry).is_none();
        if inserted {
            self.metadata.entry_count.fetch_add(1, Ordering::Relaxed);
            self.metadata
                .total_size
                .fetch_add(entry_size, Ordering::Relaxed);
        }

        Ok(inserted)
    }

    fn needs_eviction(&self, new_entry_size: usize) -> bool {
        let current_size = self.metadata.total_size.load(Ordering::Relaxed);
        let current_count = self.metadata.entry_count.load(Ordering::Relaxed);

        current_size + new_entry_size > self.config.max_memory_bytes
            || current_count >= self.config.max_size
    }

    fn evict_lru(&self) -> Result<()> {
        let mut oldest_key: Option<Uuid> = None;
        let mut oldest_time = u64::MAX;

        // Find the least recently used entry
        for entry in self.storage.iter() {
            let access_time = entry.last_accessed.load(Ordering::Relaxed);
            if access_time < oldest_time {
                oldest_time = access_time;
                oldest_key = Some(*entry.key());
            }
        }

        if let Some(key) = oldest_key {
            if let Some((_, entry)) = self.storage.remove(&key) {
                self.metadata.entry_count.fetch_sub(1, Ordering::Relaxed);
                self.metadata
                    .total_size
                    .fetch_sub(entry.size_bytes, Ordering::Relaxed);
                self.metadata.eviction_count.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        }

        Err(GaussOSError::CacheError(
            "No entries available for eviction".to_string(),
        ))
    }

    pub fn len(&self) -> usize {
        self.metadata.entry_count.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        self.storage.clear();
        self.metadata.entry_count.store(0, Ordering::Relaxed);
        self.metadata.total_size.store(0, Ordering::Relaxed);
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.len(),
            capacity: self.config.max_size,
            generation: 0,
            hit_rate: self.hit_rate(),
            metrics: self.metrics.snapshot(),
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.metrics.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.metrics.cache_misses.load(Ordering::Relaxed) as f64;

        if hits + misses > 0.0 {
            hits / (hits + misses)
        } else {
            0.0
        }
    }

    /// Bulk operation for better performance
    pub fn get_many(&self, keys: &[Uuid]) -> Vec<(Uuid, Option<Arc<MemCube>>)> {
        keys.iter().map(|key| (*key, self.get(key))).collect()
    }

    /// Optimized contains check without triggering cache statistics
    pub fn contains_key(&self, key: &Uuid) -> bool {
        self.storage.contains_key(key)
    }

    /// Get cache utilization as percentage
    pub fn utilization(&self) -> f64 {
        let current_size = self.metadata.total_size.load(Ordering::Relaxed) as f64;
        let max_size = self.config.max_memory_bytes as f64;

        if max_size > 0.0 {
            (current_size / max_size) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
pub struct AtomicMetrics {
    reads: AtomicU64,
    writes: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    inserts: AtomicU64,
    updates: AtomicU64,
    removals: AtomicU64,
    evictions: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl AtomicMetrics {
    pub fn new() -> Self {
        Self {
            reads: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            inserts: AtomicU64::new(0),
            updates: AtomicU64::new(0),
            removals: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            inserts: self.inserts.load(Ordering::Relaxed),
            updates: self.updates.load(Ordering::Relaxed),
            removals: self.removals.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub generation: u64,
    pub hit_rate: f64,
    pub metrics: MetricsSnapshot,
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub reads: u64,
    pub writes: u64,
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub updates: u64,
    pub removals: u64,
    pub evictions: u64,
}
