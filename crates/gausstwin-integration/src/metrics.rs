use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectorMetrics {
    pub connection_count: u64,
    pub request_count: u64,
    pub error_count: u64,
    pub latency_ms: f64,
}

impl ConnectorMetrics {
    pub fn new() -> Self {
        Self {
            connection_count: 0,
            request_count: 0,
            error_count: 0,
            latency_ms: 0.0,
        }
    }

    pub fn increment_connection(&mut self) {
        self.connection_count += 1;
    }

    pub fn increment_request(&mut self) {
        self.request_count += 1;
    }

    pub fn increment_error(&mut self) {
        self.error_count += 1;
    }

    pub fn update_latency(&mut self, latency_ms: f64) {
        self.latency_ms = latency_ms;
    }
} 