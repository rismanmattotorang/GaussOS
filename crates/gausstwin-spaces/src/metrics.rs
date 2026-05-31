use metrics::{Counter, Gauge, Histogram, Key, Unit};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Performance metrics for space operations
#[derive(Debug)]
pub struct SpaceMetrics {
    /// Total number of agents
    agent_count: AtomicU64,
    
    /// Operation latencies
    latencies: RwLock<HashMap<String, Histogram>>,
    
    /// Operation counters
    counters: RwLock<HashMap<String, Counter>>,
    
    /// Gauges for various metrics
    gauges: RwLock<HashMap<String, Gauge>>,
}

impl SpaceMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            agent_count: AtomicU64::new(0),
            latencies: RwLock::new(HashMap::new()),
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
        }
    }
    
    /// Record the latency of an operation
    pub fn record_latency(&self, operation: &str, start: Instant) {
        let duration = start.elapsed();
        let mut latencies = self.latencies.write();
        let histogram = latencies
            .entry(operation.to_string())
            .or_insert_with(|| {
                metrics::register_histogram!(
                    Key::from_parts(operation, "latency"),
                    Unit::Microseconds
                )
            });
        histogram.record(duration.as_micros() as f64);
    }
    
    /// Increment a counter
    pub fn increment_counter(&self, name: &str, value: u64) {
        let mut counters = self.counters.write();
        let counter = counters
            .entry(name.to_string())
            .or_insert_with(|| {
                metrics::register_counter!(
                    Key::from_parts(name, "count")
                )
            });
        counter.increment(value);
    }
    
    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write();
        let gauge = gauges
            .entry(name.to_string())
            .or_insert_with(|| {
                metrics::register_gauge!(
                    Key::from_parts(name, "value")
                )
            });
        gauge.set(value);
    }
    
    /// Update agent count
    pub fn update_agent_count(&self, delta: i64) {
        if delta >= 0 {
            self.agent_count.fetch_add(delta as u64, Ordering::Relaxed);
        } else {
            self.agent_count.fetch_sub((-delta) as u64, Ordering::Relaxed);
        }
    }
    
    /// Get current agent count
    pub fn get_agent_count(&self) -> u64 {
        self.agent_count.load(Ordering::Relaxed)
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.agent_count.store(0, Ordering::Relaxed);
        self.latencies.write().clear();
        self.counters.write().clear();
        self.gauges.write().clear();
    }
}

/// Convenience macro for timing operations
#[macro_export]
macro_rules! time_operation {
    ($metrics:expr, $operation:expr, $body:expr) => {{
        let start = std::time::Instant::now();
        let result = $body;
        $metrics.record_latency($operation, start);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_metrics_recording() {
        let metrics = SpaceMetrics::new();
        
        // Test counter
        metrics.increment_counter("test_counter", 1);
        metrics.increment_counter("test_counter", 2);
        
        // Test gauge
        metrics.set_gauge("test_gauge", 42.0);
        
        // Test latency
        let start = Instant::now();
        thread::sleep(Duration::from_millis(10));
        metrics.record_latency("test_latency", start);
        
        // Test agent count
        metrics.update_agent_count(5);
        assert_eq!(metrics.get_agent_count(), 5);
        metrics.update_agent_count(-2);
        assert_eq!(metrics.get_agent_count(), 3);
    }
    
    #[test]
    fn test_time_operation_macro() {
        let metrics = SpaceMetrics::new();
        
        let result = time_operation!(metrics, "test_operation", {
            thread::sleep(Duration::from_millis(10));
            42
        });
        
        assert_eq!(result, 42);
    }
} 