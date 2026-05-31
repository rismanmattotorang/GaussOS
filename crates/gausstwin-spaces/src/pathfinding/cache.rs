//! Pathfinding Cache Module
//!
//! High-performance caching for pathfinding results to avoid redundant computation.
//! Supports LRU eviction, TTL-based expiration, and thread-safe concurrent access.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{Cost, Path, Point};

/// Cache key for pathfinding queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathCacheKey {
    start: PointKey,
    goal: PointKey,
}

impl PathCacheKey {
    /// Create a new cache key
    pub fn new(start: Point, goal: Point) -> Self {
        Self {
            start: PointKey::from(start),
            goal: PointKey::from(goal),
        }
    }
}

/// Hashable point representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PointKey {
    x: i64,
    y: i64,
    z: i64,
}

impl From<Point> for PointKey {
    fn from(p: Point) -> Self {
        const SCALE: f64 = 1000.0;
        Self {
            x: (p.x * SCALE) as i64,
            y: (p.y * SCALE) as i64,
            z: (p.z * SCALE) as i64,
        }
    }
}

impl Hash for PointKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
        self.z.hash(state);
    }
}

/// Cached path entry
#[derive(Debug, Clone)]
pub struct CachedPath {
    /// The computed path
    pub path: Path,
    /// Total cost of the path
    pub cost: Cost,
    /// When this entry was created
    pub created_at: Instant,
    /// Last access time
    pub last_accessed: Instant,
    /// Number of times this entry was accessed
    pub access_count: u64,
    /// Version of the graph when this path was computed
    pub graph_version: u64,
}

impl CachedPath {
    /// Create a new cached path entry
    pub fn new(path: Path, cost: Cost, graph_version: u64) -> Self {
        let now = Instant::now();
        Self {
            path,
            cost,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            graph_version,
        }
    }
    
    /// Check if this entry has expired
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
    
    /// Check if this entry is stale (graph has changed)
    pub fn is_stale(&self, current_graph_version: u64) -> bool {
        self.graph_version != current_graph_version
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Time-to-live for entries
    pub ttl_seconds: u64,
    /// Enable LRU eviction
    pub enable_lru: bool,
    /// Minimum accesses before entry is considered valuable
    pub min_accesses_for_value: u64,
    /// Enable cache statistics
    pub enable_stats: bool,
}

impl Default for PathCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            ttl_seconds: 300, // 5 minutes
            enable_lru: true,
            min_accesses_for_value: 2,
            enable_stats: true,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathCacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total evictions
    pub evictions: u64,
    /// Total expirations
    pub expirations: u64,
    /// Current entry count
    pub entry_count: usize,
    /// Average path length in cache
    pub avg_path_length: f64,
    /// Average path cost in cache
    pub avg_path_cost: f64,
}

impl PathCacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Thread-safe pathfinding cache with LRU eviction
pub struct PathCache {
    /// Cache storage
    cache: RwLock<HashMap<PathCacheKey, CachedPath>>,
    /// LRU tracking (key -> last access time)
    lru_tracker: RwLock<HashMap<PathCacheKey, Instant>>,
    /// Configuration
    config: PathCacheConfig,
    /// Statistics
    stats: RwLock<PathCacheStats>,
    /// Current graph version
    graph_version: AtomicU64,
}

impl PathCache {
    /// Create a new path cache with default configuration
    pub fn new() -> Self {
        Self::with_config(PathCacheConfig::default())
    }
    
    /// Create a new path cache with custom configuration
    pub fn with_config(config: PathCacheConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::with_capacity(config.max_entries)),
            lru_tracker: RwLock::new(HashMap::with_capacity(config.max_entries)),
            config,
            stats: RwLock::new(PathCacheStats::default()),
            graph_version: AtomicU64::new(0),
        }
    }
    
    /// Get a path from the cache
    pub fn get(&self, start: Point, goal: Point) -> Option<CachedPath> {
        let key = PathCacheKey::new(start, goal);
        let ttl = Duration::from_secs(self.config.ttl_seconds);
        let current_version = self.graph_version.load(Ordering::Relaxed);
        
        // Try to read from cache
        let entry = {
            let cache = self.cache.read();
            cache.get(&key).cloned()
        };
        
        match entry {
            Some(mut cached) => {
                // Check expiration and staleness
                if cached.is_expired(ttl) {
                    self.remove(&key);
                    if self.config.enable_stats {
                        self.stats.write().expirations += 1;
                        self.stats.write().misses += 1;
                    }
                    return None;
                }
                
                if cached.is_stale(current_version) {
                    self.remove(&key);
                    if self.config.enable_stats {
                        self.stats.write().misses += 1;
                    }
                    return None;
                }
                
                // Update access tracking
                cached.last_accessed = Instant::now();
                cached.access_count += 1;
                
                // Update in cache
                {
                    let mut cache = self.cache.write();
                    if let Some(entry) = cache.get_mut(&key) {
                        entry.last_accessed = cached.last_accessed;
                        entry.access_count = cached.access_count;
                    }
                }
                
                // Update LRU tracker
                if self.config.enable_lru {
                    self.lru_tracker.write().insert(key, Instant::now());
                }
                
                if self.config.enable_stats {
                    self.stats.write().hits += 1;
                }
                
                Some(cached)
            }
            None => {
                if self.config.enable_stats {
                    self.stats.write().misses += 1;
                }
                None
            }
        }
    }
    
    /// Insert a path into the cache
    pub fn insert(&self, start: Point, goal: Point, path: Path, cost: Cost) {
        let key = PathCacheKey::new(start, goal);
        let graph_version = self.graph_version.load(Ordering::Relaxed);
        let entry = CachedPath::new(path, cost, graph_version);
        
        // Check if we need to evict
        {
            let cache = self.cache.read();
            if cache.len() >= self.config.max_entries {
                drop(cache); // Release read lock before eviction
                self.evict_entries();
            }
        }
        
        // Insert new entry
        {
            let mut cache = self.cache.write();
            cache.insert(key, entry);
        }
        
        // Update LRU tracker
        if self.config.enable_lru {
            self.lru_tracker.write().insert(key, Instant::now());
        }
        
        // Update stats
        if self.config.enable_stats {
            self.update_stats();
        }
    }
    
    /// Remove an entry from the cache
    fn remove(&self, key: &PathCacheKey) {
        self.cache.write().remove(key);
        if self.config.enable_lru {
            self.lru_tracker.write().remove(key);
        }
    }
    
    /// Evict entries based on LRU policy
    fn evict_entries(&self) {
        if !self.config.enable_lru {
            // Simple eviction: remove oldest entries
            let mut cache = self.cache.write();
            let to_remove = cache.len().saturating_sub(self.config.max_entries) + 1;
            
            let keys_to_remove: Vec<_> = cache
                .iter()
                .take(to_remove)
                .map(|(k, _)| *k)
                .collect();
            
            for key in keys_to_remove {
                cache.remove(&key);
            }
            
            if self.config.enable_stats {
                self.stats.write().evictions += to_remove as u64;
            }
            return;
        }
        
        // LRU eviction
        let lru_tracker = self.lru_tracker.read();
        let cache = self.cache.read();
        
        let to_remove = cache.len().saturating_sub(self.config.max_entries) + 1;
        
        // Find least recently used entries
        let mut entries: Vec<_> = lru_tracker.iter().collect();
        entries.sort_by(|a, b| a.1.cmp(b.1));
        
        let keys_to_remove: Vec<_> = entries
            .iter()
            .take(to_remove)
            .map(|(k, _)| **k)
            .collect();
        
        drop(lru_tracker);
        drop(cache);
        
        // Remove entries
        {
            let mut cache = self.cache.write();
            let mut lru = self.lru_tracker.write();
            
            for key in &keys_to_remove {
                cache.remove(key);
                lru.remove(key);
            }
        }
        
        if self.config.enable_stats {
            self.stats.write().evictions += keys_to_remove.len() as u64;
        }
    }
    
    /// Update cache statistics
    fn update_stats(&self) {
        let cache = self.cache.read();
        let mut stats = self.stats.write();
        
        stats.entry_count = cache.len();
        
        if !cache.is_empty() {
            let total_length: usize = cache.values().map(|e| e.path.len()).sum();
            let total_cost: f64 = cache.values().map(|e| e.cost).sum();
            
            stats.avg_path_length = total_length as f64 / cache.len() as f64;
            stats.avg_path_cost = total_cost / cache.len() as f64;
        }
    }
    
    /// Invalidate all cache entries (call when graph changes)
    pub fn invalidate(&self) {
        self.graph_version.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.write().clear();
        self.lru_tracker.write().clear();
        
        if self.config.enable_stats {
            let mut stats = self.stats.write();
            stats.entry_count = 0;
            stats.avg_path_length = 0.0;
            stats.avg_path_cost = 0.0;
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> PathCacheStats {
        self.stats.read().clone()
    }
    
    /// Get current entry count
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }
    
    /// Prune expired entries
    pub fn prune_expired(&self) {
        let ttl = Duration::from_secs(self.config.ttl_seconds);
        let mut expired_keys = Vec::new();
        
        {
            let cache = self.cache.read();
            for (key, entry) in cache.iter() {
                if entry.is_expired(ttl) {
                    expired_keys.push(*key);
                }
            }
        }
        
        if !expired_keys.is_empty() {
            let mut cache = self.cache.write();
            let mut lru = self.lru_tracker.write();
            
            for key in &expired_keys {
                cache.remove(key);
                lru.remove(key);
            }
            
            if self.config.enable_stats {
                self.stats.write().expirations += expired_keys.len() as u64;
            }
        }
    }
}

impl Default for PathCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Bidirectional cache that stores paths in both directions
pub struct BidirectionalPathCache {
    inner: PathCache,
}

impl BidirectionalPathCache {
    /// Create a new bidirectional cache
    pub fn new() -> Self {
        Self {
            inner: PathCache::new(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: PathCacheConfig) -> Self {
        Self {
            inner: PathCache::with_config(config),
        }
    }
    
    /// Get a path (checks both directions)
    pub fn get(&self, start: Point, goal: Point) -> Option<CachedPath> {
        // Try forward direction
        if let Some(cached) = self.inner.get(start, goal) {
            return Some(cached);
        }
        
        // Try reverse direction
        if let Some(mut cached) = self.inner.get(goal, start) {
            // Reverse the path
            cached.path.reverse();
            return Some(cached);
        }
        
        None
    }
    
    /// Insert a path (stores in both directions)
    pub fn insert(&self, start: Point, goal: Point, path: Path, cost: Cost) {
        // Insert forward
        self.inner.insert(start, goal, path.clone(), cost);
        
        // Insert reverse
        let mut reverse_path = path;
        reverse_path.reverse();
        self.inner.insert(goal, start, reverse_path, cost);
    }
    
    /// Invalidate cache
    pub fn invalidate(&self) {
        self.inner.invalidate();
    }
    
    /// Clear cache
    pub fn clear(&self) {
        self.inner.clear();
    }
    
    /// Get statistics
    pub fn stats(&self) -> PathCacheStats {
        self.inner.stats()
    }
}

impl Default for BidirectionalPathCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Point3;
    
    #[test]
    fn test_cache_basic_operations() {
        let cache = PathCache::new();
        
        let start = Point3::new(0.0, 0.0, 0.0);
        let goal = Point3::new(10.0, 10.0, 0.0);
        let path = vec![start, Point3::new(5.0, 5.0, 0.0), goal];
        
        // Insert
        cache.insert(start, goal, path.clone(), 14.14);
        
        // Retrieve
        let cached = cache.get(start, goal).unwrap();
        assert_eq!(cached.path.len(), 3);
        assert!((cached.cost - 14.14).abs() < 0.01);
        
        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
    }
    
    #[test]
    fn test_cache_miss() {
        let cache = PathCache::new();
        
        let start = Point3::new(0.0, 0.0, 0.0);
        let goal = Point3::new(10.0, 10.0, 0.0);
        
        let result = cache.get(start, goal);
        assert!(result.is_none());
        
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }
    
    #[test]
    fn test_cache_invalidation() {
        let cache = PathCache::new();
        
        let start = Point3::new(0.0, 0.0, 0.0);
        let goal = Point3::new(10.0, 10.0, 0.0);
        let path = vec![start, goal];
        
        cache.insert(start, goal, path, 10.0);
        assert!(cache.get(start, goal).is_some());
        
        // Invalidate
        cache.invalidate();
        
        // Should be stale now
        assert!(cache.get(start, goal).is_none());
    }
    
    #[test]
    fn test_bidirectional_cache() {
        let cache = BidirectionalPathCache::new();
        
        let start = Point3::new(0.0, 0.0, 0.0);
        let goal = Point3::new(10.0, 0.0, 0.0);
        let path = vec![start, Point3::new(5.0, 0.0, 0.0), goal];
        
        cache.insert(start, goal, path.clone(), 10.0);
        
        // Forward lookup
        let forward = cache.get(start, goal).unwrap();
        assert_eq!(forward.path.first().unwrap(), &start);
        
        // Reverse lookup
        let reverse = cache.get(goal, start).unwrap();
        assert_eq!(reverse.path.first().unwrap(), &goal);
    }
    
    #[test]
    fn test_lru_eviction() {
        let config = PathCacheConfig {
            max_entries: 3,
            ..Default::default()
        };
        let cache = PathCache::with_config(config);
        
        // Insert more entries than max
        for i in 0..5 {
            let start = Point3::new(0.0, 0.0, 0.0);
            let goal = Point3::new(i as f64, 0.0, 0.0);
            let path = vec![start, goal];
            cache.insert(start, goal, path, i as f64);
        }
        
        // Should have evicted some entries
        assert!(cache.len() <= 3);
        
        let stats = cache.stats();
        assert!(stats.evictions > 0);
    }
}
