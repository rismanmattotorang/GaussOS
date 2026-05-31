// src/auth/api_keys.rs
//! API Key Management
//! Provides secure API key generation, validation, and permission management

use crate::{
    database::MemVault,
    error::{GaussOSError, Result},
};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// API Key Manager
pub struct ApiKeyManager {
    database: Arc<dyn MemVault>,
}

/// API Key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub permissions: ApiKeyPermissions,
    pub rate_limit_per_hour: u32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub is_active: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// API Key permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPermissions {
    /// Allowed operations
    pub operations: Vec<String>,
    /// Allowed namespaces (empty means all)
    pub namespaces: Vec<String>,
    /// Maximum requests per hour
    pub rate_limit: Option<u32>,
    /// IP address restrictions
    pub allowed_ips: Vec<String>,
    /// Time-based restrictions
    pub time_restrictions: Option<TimeRestrictions>,
    /// Additional custom permissions
    pub custom: HashMap<String, serde_json::Value>,
}

/// Time-based access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    /// Allowed hours (0-23)
    pub allowed_hours: Vec<u8>,
    /// Allowed days of week (0=Sunday, 6=Saturday)
    pub allowed_days: Vec<u8>,
    /// Timezone for time restrictions
    pub timezone: String,
}

/// API Key creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: ApiKeyPermissions,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// API Key validation result
#[derive(Debug, Clone)]
pub struct ApiKeyValidation {
    pub is_valid: bool,
    pub api_key: Option<ApiKey>,
    pub error: Option<String>,
    pub rate_limit_remaining: Option<u32>,
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new(database: Arc<dyn MemVault>) -> Self {
        Self { database }
    }

    /// Generate a new API key
    pub async fn create_api_key(
        &self,
        user_id: &Uuid,
        request: CreateApiKeyRequest,
    ) -> Result<(ApiKey, String)> {
        // Generate secure random API key
        let raw_key = self.generate_secure_key();
        let key_hash = self.hash_api_key(&raw_key)?;

        let api_key = ApiKey {
            id: Uuid::new_v4(),
            user_id: *user_id,
            name: request.name,
            key_hash,
            permissions: request.permissions,
            rate_limit_per_hour: 1000, // Default rate limit
            expires_at: request.expires_at,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            is_active: true,
            metadata: request.metadata.unwrap_or_default(),
        };

        // Store in database
        self.store_api_key(&api_key).await?;

        // Return the API key object and the raw key (only time it's available)
        Ok((api_key, raw_key))
    }

    /// Validate an API key
    pub async fn validate_api_key(&self, raw_key: &str) -> Result<ApiKeyValidation> {
        // Hash the provided key to compare with stored hash
        let key_hash = self.hash_api_key(raw_key)?;

        // Retrieve API key from database
        match self.get_api_key_by_hash(&key_hash).await? {
            Some(mut api_key) => {
                // Check if key is active
                if !api_key.is_active {
                    return Ok(ApiKeyValidation {
                        is_valid: false,
                        api_key: None,
                        error: Some("API key is disabled".to_string()),
                        rate_limit_remaining: None,
                    });
                }

                // Check expiration
                if let Some(expires_at) = api_key.expires_at {
                    if Utc::now() > expires_at {
                        return Ok(ApiKeyValidation {
                            is_valid: false,
                            api_key: None,
                            error: Some("API key has expired".to_string()),
                            rate_limit_remaining: None,
                        });
                    }
                }

                // Check rate limits
                let rate_limit_remaining = self.check_rate_limit(&api_key).await?;
                if rate_limit_remaining == 0 {
                    return Ok(ApiKeyValidation {
                        is_valid: false,
                        api_key: Some(api_key),
                        error: Some("Rate limit exceeded".to_string()),
                        rate_limit_remaining: Some(0),
                    });
                }

                // Update last used timestamp and usage count
                api_key.last_used = Some(Utc::now());
                api_key.usage_count += 1;
                self.update_api_key(&api_key).await?;

                Ok(ApiKeyValidation {
                    is_valid: true,
                    api_key: Some(api_key),
                    error: None,
                    rate_limit_remaining: Some(rate_limit_remaining),
                })
            }
            None => Ok(ApiKeyValidation {
                is_valid: false,
                api_key: None,
                error: Some("Invalid API key".to_string()),
                rate_limit_remaining: None,
            }),
        }
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, api_key_id: &Uuid) -> Result<()> {
        if let Some(mut api_key) = self.get_api_key_by_id(api_key_id).await? {
            api_key.is_active = false;
            self.update_api_key(&api_key).await?;
            tracing::info!("API key revoked: {}", api_key_id);
        }
        Ok(())
    }

    /// List API keys for a user
    pub async fn list_user_api_keys(&self, user_id: &Uuid) -> Result<Vec<ApiKey>> {
        self.get_api_keys_by_user(user_id).await
    }

    /// Update API key permissions
    pub async fn update_permissions(
        &self,
        api_key_id: &Uuid,
        permissions: ApiKeyPermissions,
    ) -> Result<()> {
        if let Some(mut api_key) = self.get_api_key_by_id(api_key_id).await? {
            api_key.permissions = permissions;
            self.update_api_key(&api_key).await?;
        }
        Ok(())
    }

    /// Check if API key has specific permission
    pub fn has_permission(&self, api_key: &ApiKey, operation: &str, namespace: &str) -> bool {
        // Check if operation is allowed
        if !api_key
            .permissions
            .operations
            .contains(&operation.to_string())
            && !api_key.permissions.operations.contains(&"*".to_string())
        {
            return false;
        }

        // Check namespace restrictions
        if !api_key.permissions.namespaces.is_empty()
            && !api_key
                .permissions
                .namespaces
                .contains(&namespace.to_string())
            && !api_key.permissions.namespaces.contains(&"*".to_string())
        {
            return false;
        }

        // Check time restrictions
        if let Some(restrictions) = &api_key.permissions.time_restrictions {
            if !self.check_time_restrictions(restrictions) {
                return false;
            }
        }

        true
    }

    /// Generate a secure random API key
    fn generate_secure_key(&self) -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        let prefix = "gos"; // GaussOS prefix
        let random_part: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        format!("{}_{}", prefix, random_part)
    }

    /// Hash an API key using Argon2
    fn hash_api_key(&self, key: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(key.as_bytes(), &salt)
            .map_err(|e| {
                GaussOSError::AuthenticationError(format!("Failed to hash API key: {}", e))
            })
            .map(|hash| hash.to_string())
    }

    /// Verify an API key against its hash
    fn verify_api_key(&self, key: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            GaussOSError::AuthenticationError(format!("Invalid hash format: {}", e))
        })?;

        Ok(Argon2::default()
            .verify_password(key.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Check time-based restrictions
    fn check_time_restrictions(&self, restrictions: &TimeRestrictions) -> bool {
        let now = Utc::now();

        // For simplicity, using UTC. In production, parse timezone
        let hour = now.hour() as u8;
        let weekday = now.weekday().num_days_from_sunday() as u8;

        // Check allowed hours
        if !restrictions.allowed_hours.is_empty() && !restrictions.allowed_hours.contains(&hour) {
            return false;
        }

        // Check allowed days
        if !restrictions.allowed_days.is_empty() && !restrictions.allowed_days.contains(&weekday) {
            return false;
        }

        true
    }

    /// Check rate limit for API key
    async fn check_rate_limit(&self, api_key: &ApiKey) -> Result<u32> {
        // In a real implementation, this would check Redis or similar
        // For now, return the full rate limit
        Ok(api_key.rate_limit_per_hour)
    }

    /// Store API key in database
    async fn store_api_key(&self, api_key: &ApiKey) -> Result<()> {
        // Implementation would store in the database
        // For now, this is a placeholder
        tracing::info!("Storing API key: {}", api_key.id);
        Ok(())
    }

    /// Update API key in database
    async fn update_api_key(&self, api_key: &ApiKey) -> Result<()> {
        // Implementation would update in the database
        tracing::info!("Updating API key: {}", api_key.id);
        Ok(())
    }

    /// Get API key by hash
    async fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>> {
        // Implementation would query the database
        // For now, return None
        Ok(None)
    }

    /// Get API key by ID
    async fn get_api_key_by_id(&self, id: &Uuid) -> Result<Option<ApiKey>> {
        // Implementation would query the database
        Ok(None)
    }

    /// Get API keys by user
    async fn get_api_keys_by_user(&self, user_id: &Uuid) -> Result<Vec<ApiKey>> {
        // Implementation would query the database
        Ok(Vec::new())
    }
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self {
            operations: vec!["memory:read".to_string()],
            namespaces: Vec::new(), // Empty means all namespaces
            rate_limit: Some(1000),
            allowed_ips: Vec::new(),
            time_restrictions: None,
            custom: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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
            use chrono::Utc;
            use std::collections::HashMap;
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
            use chrono::Utc;
            Ok(crate::database::BackupResult {
                backup_id: uuid::Uuid::new_v4(),
                size_bytes: 0,
                duration_ms: 0,
                checksum: "mock_checksum".to_string(),
                metadata: crate::database::BackupMetadata {
                    timestamp: Utc::now(),
                    database_version: "1.0.0".to_string(),
                    record_count: 0,
                    compression_ratio: 1.0,
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
            use chrono::Utc;
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

    #[tokio::test]
    async fn test_api_key_creation() {
        let mock_vault = Arc::new(MockMemVault);
        let manager = ApiKeyManager::new(mock_vault);

        let request = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            permissions: ApiKeyPermissions::default(),
            expires_at: None,
            metadata: None,
        };

        let user_id = Uuid::new_v4();
        let result = manager.create_api_key(&user_id, request).await;
        assert!(result.is_ok());

        let (api_key, raw_key) = result.unwrap();
        assert_eq!(api_key.user_id, user_id);
        assert!(raw_key.starts_with("gos_"));
    }

    #[test]
    fn test_permission_check() {
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test".to_string(),
            key_hash: "hash".to_string(),
            permissions: ApiKeyPermissions {
                operations: vec!["memory:read".to_string(), "memory:write".to_string()],
                namespaces: vec!["test".to_string()],
                rate_limit: Some(1000),
                allowed_ips: Vec::new(),
                time_restrictions: None,
                custom: HashMap::new(),
            },
            rate_limit_per_hour: 1000,
            expires_at: None,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            is_active: true,
            metadata: HashMap::new(),
        };

        let mock_vault = Arc::new(MockMemVault);
        let manager = ApiKeyManager::new(mock_vault);

        assert!(manager.has_permission(&api_key, "memory:read", "test"));
        assert!(!manager.has_permission(&api_key, "memory:delete", "test"));
        assert!(!manager.has_permission(&api_key, "memory:read", "other"));
    }
}
