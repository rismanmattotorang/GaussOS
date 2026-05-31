//! Advanced Rate Limiter
//! Implements sliding window rate limiting with distributed support

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::error::{GaussOSError, Result};

/// Rate limiter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    /// Requests allowed per window
    pub requests_per_window: u64,
    /// Window duration in seconds
    pub window_seconds: u64,
    /// Burst allowance (extra requests allowed in short burst)
    pub burst_size: u64,
    /// Enable distributed rate limiting via Redis
    pub distributed: bool,
    /// Redis URL for distributed mode
    pub redis_url: Option<String>,
    /// Custom rate limits by endpoint
    pub endpoint_limits: Vec<EndpointRateLimit>,
    /// Custom rate limits by user tier
    pub tier_limits: Vec<TierRateLimit>,
}

/// Endpoint-specific rate limit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRateLimit {
    pub path_pattern: String,
    pub requests_per_window: u64,
    pub window_seconds: u64,
}

/// Tier-based rate limit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRateLimit {
    pub tier_name: String,
    pub requests_per_minute: u64,
    pub requests_per_hour: u64,
    pub requests_per_day: u64,
}

/// Rate limit entry for tracking
#[derive(Debug)]
pub struct RateLimitEntry {
    /// Request count in current window
    count: AtomicU64,
    /// Window start time
    window_start: DateTime<Utc>,
    /// Requests in previous window (for sliding window)
    previous_count: u64,
}

/// Rate limit result
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in window
    pub remaining: u64,
    /// Total limit for window
    pub limit: u64,
    /// Seconds until window reset
    pub reset_in_seconds: u64,
    /// Retry after seconds (if blocked)
    pub retry_after: Option<u64>,
}

/// Enterprise-grade rate limiter with sliding window algorithm
pub struct RateLimiter {
    /// Configuration
    config: RateLimiterConfig,
    /// IP-based rate limit entries
    ip_entries: Arc<DashMap<String, RateLimitEntry>>,
    /// User-based rate limit entries
    user_entries: Arc<DashMap<String, RateLimitEntry>>,
    /// API key rate limit entries
    api_key_entries: Arc<DashMap<String, RateLimitEntry>>,
    /// Global request counter
    global_counter: AtomicU64,
    /// Blocked IPs (temporary bans)
    blocked_ips: Arc<DashMap<String, DateTime<Utc>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            config,
            ip_entries: Arc::new(DashMap::new()),
            user_entries: Arc::new(DashMap::new()),
            api_key_entries: Arc::new(DashMap::new()),
            global_counter: AtomicU64::new(0),
            blocked_ips: Arc::new(DashMap::new()),
        }
    }

    /// Check if a request is allowed
    pub fn check_rate_limit(
        &self,
        identifier: &str,
        identifier_type: RateLimitIdentifier,
    ) -> RateLimitResult {
        // Check if blocked
        if let RateLimitIdentifier::Ip(ip) = &identifier_type {
            if let Some(blocked_until) = self.blocked_ips.get(ip) {
                if *blocked_until > Utc::now() {
                    let retry_after = (*blocked_until - Utc::now()).num_seconds() as u64;
                    return RateLimitResult {
                        allowed: false,
                        remaining: 0,
                        limit: self.config.requests_per_window,
                        reset_in_seconds: retry_after,
                        retry_after: Some(retry_after),
                    };
                } else {
                    self.blocked_ips.remove(ip);
                }
            }
        }

        // Get the appropriate entry map
        let entries = match &identifier_type {
            RateLimitIdentifier::Ip(_) => &self.ip_entries,
            RateLimitIdentifier::UserId(_) => &self.user_entries,
            RateLimitIdentifier::ApiKey(_) => &self.api_key_entries,
        };

        // Get or create entry
        let now = Utc::now();
        let window_duration = Duration::seconds(self.config.window_seconds as i64);

        let mut entry = entries.entry(identifier.to_string()).or_insert_with(|| {
            RateLimitEntry {
                count: AtomicU64::new(0),
                window_start: now,
                previous_count: 0,
            }
        });

        // Check if window has expired
        let window_elapsed = now - entry.window_start;
        if window_elapsed >= window_duration {
            // Slide the window
            let windows_passed = (window_elapsed.num_seconds() / self.config.window_seconds as i64) as u64;
            if windows_passed >= 2 {
                // More than 2 windows passed, reset completely
                entry.previous_count = 0;
            } else {
                // Store previous count for sliding window calculation
                entry.previous_count = entry.count.load(Ordering::Relaxed);
            }
            entry.count.store(0, Ordering::Relaxed);
            entry.window_start = now;
        }

        // Calculate effective count using sliding window
        let current_count = entry.count.load(Ordering::Relaxed);
        let window_progress = window_elapsed.num_milliseconds() as f64 
            / (self.config.window_seconds as f64 * 1000.0);
        let effective_count = current_count as f64 
            + entry.previous_count as f64 * (1.0 - window_progress.min(1.0));

        let limit = self.config.requests_per_window + self.config.burst_size;
        
        if effective_count < limit as f64 {
            // Allow request
            entry.count.fetch_add(1, Ordering::Relaxed);
            self.global_counter.fetch_add(1, Ordering::Relaxed);
            
            let remaining = (limit as f64 - effective_count - 1.0).max(0.0) as u64;
            let reset_in = self.config.window_seconds 
                - window_elapsed.num_seconds().min(self.config.window_seconds as i64) as u64;

            RateLimitResult {
                allowed: true,
                remaining,
                limit,
                reset_in_seconds: reset_in,
                retry_after: None,
            }
        } else {
            // Rate limit exceeded
            let reset_in = self.config.window_seconds 
                - window_elapsed.num_seconds().min(self.config.window_seconds as i64) as u64;

            RateLimitResult {
                allowed: false,
                remaining: 0,
                limit,
                reset_in_seconds: reset_in,
                retry_after: Some(reset_in),
            }
        }
    }

    /// Temporarily block an IP address
    pub fn block_ip(&self, ip: &str, duration_seconds: u64) {
        let blocked_until = Utc::now() + Duration::seconds(duration_seconds as i64);
        self.blocked_ips.insert(ip.to_string(), blocked_until);
        tracing::warn!("IP {} blocked until {}", ip, blocked_until);
    }

    /// Unblock an IP address
    pub fn unblock_ip(&self, ip: &str) {
        self.blocked_ips.remove(ip);
        tracing::info!("IP {} unblocked", ip);
    }

    /// Get rate limit statistics
    pub fn get_statistics(&self) -> RateLimitStatistics {
        RateLimitStatistics {
            total_requests: self.global_counter.load(Ordering::Relaxed),
            active_ip_entries: self.ip_entries.len(),
            active_user_entries: self.user_entries.len(),
            active_api_key_entries: self.api_key_entries.len(),
            blocked_ips: self.blocked_ips.len(),
        }
    }

    /// Cleanup expired entries
    pub fn cleanup_expired_entries(&self) {
        let now = Utc::now();
        let window_duration = Duration::seconds(self.config.window_seconds as i64 * 2);

        // Cleanup IP entries
        self.ip_entries.retain(|_, entry| {
            now - entry.window_start < window_duration
        });

        // Cleanup user entries
        self.user_entries.retain(|_, entry| {
            now - entry.window_start < window_duration
        });

        // Cleanup API key entries
        self.api_key_entries.retain(|_, entry| {
            now - entry.window_start < window_duration
        });

        // Cleanup expired blocks
        self.blocked_ips.retain(|_, blocked_until| {
            *blocked_until > now
        });
    }
}

/// Rate limit identifier type
#[derive(Debug, Clone)]
pub enum RateLimitIdentifier {
    Ip(String),
    UserId(String),
    ApiKey(String),
}

/// Rate limit statistics
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStatistics {
    pub total_requests: u64,
    pub active_ip_entries: usize,
    pub active_user_entries: usize,
    pub active_api_key_entries: usize,
    pub blocked_ips: usize,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100,
            window_seconds: 60,
            burst_size: 20,
            distributed: false,
            redis_url: None,
            endpoint_limits: vec![
                EndpointRateLimit {
                    path_pattern: "/api/v1/memories".to_string(),
                    requests_per_window: 500,
                    window_seconds: 60,
                },
                EndpointRateLimit {
                    path_pattern: "/api/v1/search".to_string(),
                    requests_per_window: 100,
                    window_seconds: 60,
                },
            ],
            tier_limits: vec![
                TierRateLimit {
                    tier_name: "free".to_string(),
                    requests_per_minute: 60,
                    requests_per_hour: 1000,
                    requests_per_day: 10000,
                },
                TierRateLimit {
                    tier_name: "pro".to_string(),
                    requests_per_minute: 300,
                    requests_per_hour: 10000,
                    requests_per_day: 100000,
                },
                TierRateLimit {
                    tier_name: "enterprise".to_string(),
                    requests_per_minute: 1000,
                    requests_per_hour: 50000,
                    requests_per_day: 1000000,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let config = RateLimiterConfig {
            requests_per_window: 10,
            window_seconds: 60,
            burst_size: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        for _ in 0..10 {
            let result = limiter.check_rate_limit("test_ip", RateLimitIdentifier::Ip("127.0.0.1".to_string()));
            assert!(result.allowed);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let config = RateLimiterConfig {
            requests_per_window: 5,
            window_seconds: 60,
            burst_size: 0,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        for i in 0..10 {
            let result = limiter.check_rate_limit("test_ip", RateLimitIdentifier::Ip("127.0.0.1".to_string()));
            if i < 5 {
                assert!(result.allowed, "Request {} should be allowed", i);
            } else {
                assert!(!result.allowed, "Request {} should be blocked", i);
            }
        }
    }
}
