// src/auth/session.rs
//! Session Management
//! Provides secure session handling, tracking, and lifecycle management

use crate::{
    database::MemVault,
    error::{GaussOSError, Result},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Session Manager for handling user sessions
pub struct SessionManager {
    database: Arc<dyn MemVault>,
    session_timeout: Duration,
}

/// User session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub refresh_token_hash: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub device_fingerprint: Option<String>,
    pub is_revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<Uuid>,
    pub session_data: SessionData,
}

/// Session data with custom metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub is_mobile: bool,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session creation request
#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    pub user_id: Uuid,
    pub token_hash: String,
    pub refresh_token_hash: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub device_fingerprint: Option<String>,
    pub session_data: SessionData,
}

/// Session update request
#[derive(Debug, Clone)]
pub struct UpdateSessionRequest {
    pub last_accessed: Option<DateTime<Utc>>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub session_data: Option<SessionData>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub revoked_sessions: u64,
    pub expired_sessions: u64,
    pub sessions_by_device: HashMap<String, u64>,
    pub sessions_by_location: HashMap<String, u64>,
    pub average_session_duration: f64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(database: Arc<dyn MemVault>, session_timeout_seconds: i64) -> Self {
        Self {
            database,
            session_timeout: Duration::seconds(session_timeout_seconds),
        }
    }

    /// Create a new session
    pub async fn create_session(&self, request: CreateSessionRequest) -> Result<Session> {
        let now = Utc::now();
        let expires_at = now + self.session_timeout;

        let session = Session {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            token_hash: request.token_hash,
            refresh_token_hash: request.refresh_token_hash,
            expires_at,
            created_at: now,
            last_accessed: now,
            ip_address: request.ip_address,
            user_agent: request.user_agent,
            device_fingerprint: request.device_fingerprint,
            is_revoked: false,
            revoked_at: None,
            revoked_by: None,
            session_data: request.session_data,
        };

        self.store_session(&session).await?;
        tracing::info!(
            "Created session {} for user {}",
            session.id,
            session.user_id
        );
        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &Uuid) -> Result<Option<Session>> {
        if let Some(session) = self.get_session_by_id(session_id).await? {
            if self.is_session_valid(&session) {
                Ok(Some(session))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Get session by token hash
    pub async fn get_session_by_token(&self, token_hash: &str) -> Result<Option<Session>> {
        if let Some(session) = self.find_session_by_token(token_hash).await? {
            if self.is_session_valid(&session) {
                Ok(Some(session))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Update session (e.g., last accessed time)
    pub async fn update_session(
        &self,
        session_id: &Uuid,
        request: UpdateSessionRequest,
    ) -> Result<()> {
        if let Some(mut session) = self.get_session_by_id(session_id).await? {
            if let Some(last_accessed) = request.last_accessed {
                session.last_accessed = last_accessed;
            }

            if let Some(ip_address) = request.ip_address {
                session.ip_address = Some(ip_address);
            }

            if let Some(user_agent) = request.user_agent {
                session.user_agent = Some(user_agent);
            }

            if let Some(session_data) = request.session_data {
                session.session_data = session_data;
            }

            self.save_session(&session).await?;
        }

        Ok(())
    }

    /// Revoke a session
    pub async fn revoke_session(&self, session_id: &Uuid, revoked_by: Option<&Uuid>) -> Result<()> {
        if let Some(mut session) = self.get_session_by_id(session_id).await? {
            session.is_revoked = true;
            session.revoked_at = Some(Utc::now());
            session.revoked_by = revoked_by.copied();

            self.save_session(&session).await?;
            tracing::info!(
                "Revoked session {} for user {}",
                session.id,
                session.user_id
            );
        }

        Ok(())
    }

    /// Revoke all sessions for a user
    pub async fn revoke_user_sessions(
        &self,
        user_id: &Uuid,
        revoked_by: Option<&Uuid>,
    ) -> Result<u32> {
        let sessions = self.get_user_sessions(user_id).await?;
        let mut revoked_count = 0;

        for session in sessions {
            if !session.is_revoked && self.is_session_valid(&session) {
                self.revoke_session(&session.id, revoked_by).await?;
                revoked_count += 1;
            }
        }

        tracing::info!("Revoked {} sessions for user {}", revoked_count, user_id);
        Ok(revoked_count)
    }

    /// Refresh session expiration
    pub async fn refresh_session(&self, session_id: &Uuid) -> Result<DateTime<Utc>> {
        if let Some(mut session) = self.get_session_by_id(session_id).await? {
            let new_expires_at = Utc::now() + self.session_timeout;
            session.expires_at = new_expires_at;
            session.last_accessed = Utc::now();

            self.save_session(&session).await?;
            Ok(new_expires_at)
        } else {
            Err(GaussOSError::NotFound("Session not found".to_string()))
        }
    }

    /// Get all active sessions for a user
    pub async fn get_user_active_sessions(&self, user_id: &Uuid) -> Result<Vec<Session>> {
        let sessions = self.get_user_sessions(user_id).await?;
        Ok(sessions
            .into_iter()
            .filter(|s| self.is_session_valid(s))
            .collect())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u32> {
        let expired_sessions = self.find_expired_sessions().await?;
        let mut cleaned_count = 0;

        for session in expired_sessions {
            self.delete_session(&session.id).await?;
            cleaned_count += 1;
        }

        if cleaned_count > 0 {
            tracing::info!("Cleaned up {} expired sessions", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> Result<SessionStats> {
        let all_sessions = self.get_all_sessions().await?;
        let now = Utc::now();

        let mut stats = SessionStats {
            total_sessions: 0,
            active_sessions: 0,
            revoked_sessions: 0,
            expired_sessions: 0,
            sessions_by_device: HashMap::new(),
            sessions_by_location: HashMap::new(),
            average_session_duration: 0.0,
        };

        let mut total_duration = 0f64;
        let mut duration_count = 0u64;

        for session in all_sessions {
            stats.total_sessions += 1;

            if session.is_revoked {
                stats.revoked_sessions += 1;
            } else if session.expires_at < now {
                stats.expired_sessions += 1;
            } else {
                stats.active_sessions += 1;
            }

            // Track by device type
            if let Some(device_type) = &session.session_data.device_type {
                *stats
                    .sessions_by_device
                    .entry(device_type.clone())
                    .or_insert(0) += 1;
            }

            // Track by location
            if let Some(location) = &session.session_data.location {
                *stats
                    .sessions_by_location
                    .entry(location.clone())
                    .or_insert(0) += 1;
            }

            // Calculate session duration
            let end_time = session.revoked_at.unwrap_or(session.expires_at.min(now));
            let duration = (end_time - session.created_at).num_seconds() as f64;
            total_duration += duration;
            duration_count += 1;
        }

        if duration_count > 0 {
            stats.average_session_duration = total_duration / duration_count as f64;
        }

        Ok(stats)
    }

    /// Check if session is valid (not expired, not revoked)
    fn is_session_valid(&self, session: &Session) -> bool {
        !session.is_revoked && session.expires_at > Utc::now()
    }

    /// Detect suspicious session activity
    pub async fn detect_suspicious_activity(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<SuspiciousActivity>> {
        let sessions = self.get_user_sessions(user_id).await?;
        let mut suspicious = Vec::new();

        // Check for multiple concurrent sessions from different locations
        let active_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| self.is_session_valid(s))
            .collect();

        if active_sessions.len() > 5 {
            suspicious.push(SuspiciousActivity {
                activity_type: SuspiciousActivityType::TooManySessions,
                details: format!("{} concurrent sessions", active_sessions.len()),
                session_ids: active_sessions.iter().map(|s| s.id).collect(),
                detected_at: Utc::now(),
            });
        }

        // Check for sessions from different IP addresses
        let unique_ips: std::collections::HashSet<_> = active_sessions
            .iter()
            .filter_map(|s| s.ip_address)
            .collect();

        if unique_ips.len() > 3 {
            suspicious.push(SuspiciousActivity {
                activity_type: SuspiciousActivityType::MultipleIpAddresses,
                details: format!("{} different IP addresses", unique_ips.len()),
                session_ids: active_sessions.iter().map(|s| s.id).collect(),
                detected_at: Utc::now(),
            });
        }

        // Check for rapid session creation
        let recent_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| (Utc::now() - s.created_at).num_hours() < 1)
            .collect();

        if recent_sessions.len() > 10 {
            suspicious.push(SuspiciousActivity {
                activity_type: SuspiciousActivityType::RapidSessionCreation,
                details: format!(
                    "{} sessions created in the last hour",
                    recent_sessions.len()
                ),
                session_ids: recent_sessions.iter().map(|s| s.id).collect(),
                detected_at: Utc::now(),
            });
        }

        Ok(suspicious)
    }

    // Database operations (placeholders)
    async fn store_session(&self, session: &Session) -> Result<()> {
        tracing::debug!("Storing session: {}", session.id);
        Ok(())
    }

    async fn get_session_by_id(&self, id: &Uuid) -> Result<Option<Session>> {
        Ok(None)
    }

    async fn find_session_by_token(&self, token_hash: &str) -> Result<Option<Session>> {
        Ok(None)
    }

    async fn save_session(&self, session: &Session) -> Result<()> {
        tracing::debug!("Saving session: {}", session.id);
        Ok(())
    }

    async fn delete_session(&self, id: &Uuid) -> Result<()> {
        tracing::debug!("Deleting session: {}", id);
        Ok(())
    }

    async fn get_user_sessions(&self, user_id: &Uuid) -> Result<Vec<Session>> {
        Ok(Vec::new())
    }

    async fn get_all_sessions(&self) -> Result<Vec<Session>> {
        Ok(Vec::new())
    }

    async fn find_expired_sessions(&self) -> Result<Vec<Session>> {
        Ok(Vec::new())
    }
}

/// Suspicious activity detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousActivity {
    pub activity_type: SuspiciousActivityType,
    pub details: String,
    pub session_ids: Vec<Uuid>,
    pub detected_at: DateTime<Utc>,
}

/// Types of suspicious activities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuspiciousActivityType {
    TooManySessions,
    MultipleIpAddresses,
    RapidSessionCreation,
    UnusualLocation,
    SuspiciousUserAgent,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            device_type: None,
            location: None,
            is_mobile: false,
            browser: None,
            os: None,
            metadata: HashMap::new(),
        }
    }
}

impl SessionData {
    /// Create session data from user agent string
    pub fn from_user_agent(user_agent: &str) -> Self {
        let mut data = Self::default();

        // Simple user agent parsing (in production, use a proper library)
        if user_agent.contains("Mobile")
            || user_agent.contains("Android")
            || user_agent.contains("iPhone")
        {
            data.is_mobile = true;
            data.device_type = Some("mobile".to_string());
        } else {
            data.device_type = Some("desktop".to_string());
        }

        if user_agent.contains("Chrome") {
            data.browser = Some("Chrome".to_string());
        } else if user_agent.contains("Firefox") {
            data.browser = Some("Firefox".to_string());
        } else if user_agent.contains("Safari") {
            data.browser = Some("Safari".to_string());
        }

        if user_agent.contains("Windows") {
            data.os = Some("Windows".to_string());
        } else if user_agent.contains("Mac") {
            data.os = Some("macOS".to_string());
        } else if user_agent.contains("Linux") {
            data.os = Some("Linux".to_string());
        }

        data
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_session_data_from_user_agent() {
        let mobile_ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0 Mobile/15E148 Safari/604.1";
        let data = SessionData::from_user_agent(mobile_ua);

        assert!(data.is_mobile);
        assert_eq!(data.device_type, Some("mobile".to_string()));
        assert_eq!(data.browser, Some("Safari".to_string()));
    }

    #[test]
    fn test_session_validity() {
        let manager = SessionManager::new(
            Arc::new(MockMemVault),
            3600, // 1 hour
        );

        // Valid session
        let session = Session {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token_hash: "hash".to_string(),
            refresh_token_hash: None,
            expires_at: Utc::now() + Duration::hours(1),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            ip_address: None,
            user_agent: None,
            device_fingerprint: None,
            is_revoked: false,
            revoked_at: None,
            revoked_by: None,
            session_data: SessionData::default(),
        };

        assert!(manager.is_session_valid(&session));

        // Expired session
        let mut expired_session = session.clone();
        expired_session.expires_at = Utc::now() - Duration::hours(1);
        assert!(!manager.is_session_valid(&expired_session));

        // Revoked session
        let mut revoked_session = session;
        revoked_session.is_revoked = true;
        assert!(!manager.is_session_valid(&revoked_session));
    }

    // Mock MemVault for testing
    struct MockMemVault;

    #[async_trait::async_trait]
    impl MemVault for MockMemVault {
        async fn store(&self, _memory: &crate::core::MemCube) -> Result<()> {
            Ok(())
        }
        async fn retrieve(&self, _id: &Uuid) -> Result<Option<crate::core::MemCube>> {
            Ok(None)
        }
        async fn update(&self, _memory: &crate::core::MemCube) -> Result<()> {
            Ok(())
        }
        async fn delete(&self, _id: &Uuid) -> Result<()> {
            Ok(())
        }
        async fn search(
            &self,
            _query: &crate::database::SearchQuery,
        ) -> Result<Vec<crate::core::MemCube>> {
            Ok(Vec::new())
        }
        async fn list_by_tags(&self, _tags: &[String]) -> Result<Vec<crate::core::MemCube>> {
            Ok(Vec::new())
        }
        async fn get_stats(&self) -> Result<crate::database::VaultStats> {
            Ok(crate::database::VaultStats {
                total_memories: 0,
                memory_by_type: HashMap::new(),
                memory_by_namespace: HashMap::new(),
                storage_size: 0,
                average_memory_size: 0.0,
                average_access_count: 0.0,
                quality_distribution: crate::database::QualityDistribution {
                    excellent: 0,
                    very_good: 0,
                    good: 0,
                    average: 0,
                    below_average: 0,
                    poor: 0,
                    very_poor: 0,
                },
                age_statistics: crate::database::AgeStatistics {
                    newest: Utc::now(),
                    oldest: Utc::now(),
                    average_age_days: 0.0,
                    median_age_days: 0.0,
                    percentiles: crate::database::AgePercentiles {
                        p50: 0.0,
                        p75: 0.0,
                        p90: 0.0,
                        p95: 0.0,
                        p99: 0.0,
                    },
                },
                performance_metrics: crate::database::PerformanceMetrics {
                    average_query_time_ms: 0.0,
                    p95_query_time_ms: 0.0,
                    p99_query_time_ms: 0.0,
                    queries_per_second: 0.0,
                    cache_hit_rate: 0.0,
                    index_usage_rate: 0.0,
                },
                storage_metrics: crate::database::StorageMetrics {
                    compression_ratio: 0.0,
                    fragmentation_ratio: 0.0,
                    index_size: 0,
                    data_size: 0,
                    total_size: 0,
                    growth_rate_per_day: 0.0,
                },
                database_metrics: None,
                last_updated: Utc::now(),
            })
        }
        async fn backup(
            &self,
            _backup_config: &crate::database::BackupConfig,
        ) -> Result<crate::database::BackupResult> {
            Ok(crate::database::BackupResult {
                backup_id: Uuid::new_v4(),
                size_bytes: 0,
                duration_ms: 0,
                checksum: "mock_checksum".to_string(),
                metadata: crate::database::BackupMetadata {
                    timestamp: Utc::now(),
                    database_version: "1.0.0".to_string(),
                    record_count: 0,
                    compression_ratio: 0.0,
                },
            })
        }
        async fn restore(
            &self,
            _restore_config: &crate::database::RestoreConfig,
        ) -> Result<crate::database::RestoreResult> {
            Ok(crate::database::RestoreResult {
                records_restored: 0,
                duration_ms: 0,
                integrity_verified: true,
            })
        }
        async fn optimize(&self) -> Result<crate::database::OptimizationResult> {
            Ok(crate::database::OptimizationResult {
                operations_performed: vec![],
                space_reclaimed_bytes: 0,
                performance_improvement_percent: 0.0,
                duration_ms: 0,
            })
        }
        async fn get_real_time_metrics(&self) -> Result<crate::database::RealTimeMetrics> {
            Ok(crate::database::RealTimeMetrics {
                timestamp: Utc::now(),
                operations_per_second: 0.0,
                active_queries: 0,
                slow_queries: 0,
                cache_hit_rate: 0.0,
                connection_utilization: 0.0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                disk_io_mb_per_sec: 0.0,
                network_io_mb_per_sec: 0.0,
            })
        }
    }
}
