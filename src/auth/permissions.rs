// src/auth/permissions.rs
//! Permission Management and Checking
//! Provides sophisticated permission evaluation and namespace-based access control

use crate::{
    auth::roles::{
        Permission, PermissionCondition, PermissionContext as RolePermissionContext, RoleManager,
    },
    error::{GaussOSError, Result},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Permission checker for evaluating user permissions
pub struct PermissionChecker {
    role_manager: Arc<RoleManager>,
}

/// Enhanced permission context with additional metadata
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub user_id: Uuid,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub namespace: Option<String>,
    pub action: String,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Namespace permission definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespacePermission {
    pub namespace: String,
    pub user_id: Uuid,
    pub permission_type: NamespacePermissionType,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub conditions: Vec<PermissionCondition>,
}

/// Types of namespace permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NamespacePermissionType {
    Read,
    Write,
    Delete,
    Admin,
    Execute,
}

/// Permission check result
#[derive(Debug, Clone)]
pub struct PermissionResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub matched_permissions: Vec<Permission>,
    pub denied_permissions: Vec<Permission>,
    pub evaluation_time_ms: u64,
}

/// Permission cache entry
#[derive(Debug, Clone)]
struct PermissionCacheEntry {
    pub result: bool,
    pub expires_at: DateTime<Utc>,
    pub permission_hash: u64,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new(role_manager: Arc<RoleManager>) -> Self {
        Self { role_manager }
    }

    /// Check if user has permission for a specific action
    pub async fn check_permission(&self, context: &PermissionContext) -> Result<PermissionResult> {
        let start_time = std::time::Instant::now();

        // Create required permission from context
        let required_permission = Permission {
            resource: context.resource_type.clone(),
            action: context.action.clone(),
            scope: context.namespace.clone(),
            conditions: Vec::new(),
        };

        // Convert to role permission context
        let role_context = RolePermissionContext {
            user_id: context.user_id,
            resource_id: context.resource_id.clone(),
            namespace: context.namespace.clone(),
            ip_address: context.ip_address,
            timestamp: context.timestamp,
            metadata: context.metadata.clone(),
        };

        // Check role-based permissions
        let has_role_permission = self
            .role_manager
            .has_permission(&context.user_id, &required_permission, &role_context)
            .await?;

        // Check namespace-specific permissions
        let has_namespace_permission = self.check_namespace_permission(context).await?;

        // Evaluate special conditions
        let special_conditions_met = self.evaluate_special_conditions(context).await?;

        let allowed = has_role_permission || has_namespace_permission || special_conditions_met;

        let evaluation_time = start_time.elapsed().as_millis() as u64;

        Ok(PermissionResult {
            allowed,
            reason: if !allowed {
                Some("Insufficient permissions".to_string())
            } else {
                None
            },
            matched_permissions: if allowed {
                vec![required_permission.clone()]
            } else {
                Vec::new()
            },
            denied_permissions: if !allowed {
                vec![required_permission.clone()]
            } else {
                Vec::new()
            },
            evaluation_time_ms: evaluation_time,
        })
    }

    /// Check multiple permissions at once
    pub async fn check_permissions_batch(
        &self,
        contexts: &[PermissionContext],
    ) -> Result<Vec<PermissionResult>> {
        let mut results = Vec::with_capacity(contexts.len());

        // TODO: Implement parallel permission checking for better performance
        for context in contexts {
            results.push(self.check_permission(context).await?);
        }

        Ok(results)
    }

    /// Check if user has admin permissions
    pub async fn is_admin(&self, user_id: &Uuid) -> Result<bool> {
        let context = PermissionContext {
            user_id: *user_id,
            resource_type: "*".to_string(),
            resource_id: None,
            namespace: None,
            action: "*".to_string(),
            ip_address: None,
            user_agent: None,
            timestamp: Utc::now(),
            session_id: None,
            request_id: None,
            metadata: HashMap::new(),
        };

        let result = self.check_permission(&context).await?;
        Ok(result.allowed)
    }

    /// Check namespace-specific permissions
    async fn check_namespace_permission(&self, context: &PermissionContext) -> Result<bool> {
        if let Some(namespace) = &context.namespace {
            // Get namespace permissions for user
            let namespace_permissions = self
                .get_namespace_permissions(&context.user_id, namespace)
                .await?;

            for perm in namespace_permissions {
                if self.namespace_permission_matches(&perm, context) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if namespace permission matches the context
    fn namespace_permission_matches(
        &self,
        permission: &NamespacePermission,
        context: &PermissionContext,
    ) -> bool {
        // Check if permission type allows the action
        let action_allowed = match (&permission.permission_type, context.action.as_str()) {
            (NamespacePermissionType::Admin, _) => true,
            (NamespacePermissionType::Read, "read") => true,
            (NamespacePermissionType::Write, "write" | "read") => true,
            (NamespacePermissionType::Delete, "delete" | "write" | "read") => true,
            (NamespacePermissionType::Execute, "execute" | "read") => true,
            _ => false,
        };

        if !action_allowed {
            return false;
        }

        // Check expiration
        if let Some(expires_at) = permission.expires_at {
            if Utc::now() > expires_at {
                return false;
            }
        }

        // Check conditions
        for condition in &permission.conditions {
            if !self.evaluate_namespace_condition(condition, context) {
                return false;
            }
        }

        true
    }

    /// Evaluate namespace permission condition
    fn evaluate_namespace_condition(
        &self,
        condition: &PermissionCondition,
        context: &PermissionContext,
    ) -> bool {
        // Similar to role condition evaluation but with namespace-specific context
        let context_value = match condition.field.as_str() {
            "user_id" => serde_json::json!(context.user_id.to_string()),
            "namespace" => serde_json::json!(context.namespace),
            "resource_type" => serde_json::json!(context.resource_type),
            "action" => serde_json::json!(context.action),
            "ip_address" => serde_json::json!(context.ip_address.map(|ip| ip.to_string())),
            "timestamp" => serde_json::json!(context.timestamp.timestamp()),
            field => context
                .metadata
                .get(field)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        };

        // Use same evaluation logic as role conditions
        match condition.operator {
            crate::auth::roles::ConditionOperator::Equals => context_value == condition.value,
            crate::auth::roles::ConditionOperator::NotEquals => context_value != condition.value,
            // ... other operators (similar to roles.rs)
            _ => false, // Simplified for brevity
        }
    }

    /// Evaluate special conditions (IP restrictions, time-based access, etc.)
    async fn evaluate_special_conditions(&self, context: &PermissionContext) -> Result<bool> {
        // Check IP-based restrictions
        if let Some(ip) = context.ip_address {
            if !self.is_ip_allowed(&context.user_id, ip).await? {
                return Ok(false);
            }
        }

        // Check time-based restrictions
        if !self
            .is_time_allowed(&context.user_id, context.timestamp)
            .await?
        {
            return Ok(false);
        }

        // Check rate limiting
        if !self
            .check_rate_limits(&context.user_id, &context.action)
            .await?
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check if IP address is allowed for user
    async fn is_ip_allowed(&self, user_id: &Uuid, ip: std::net::IpAddr) -> Result<bool> {
        // In a real implementation, check against IP allowlist/blocklist
        // For now, allow all IPs
        Ok(true)
    }

    /// Check if current time is allowed for user
    async fn is_time_allowed(&self, user_id: &Uuid, timestamp: DateTime<Utc>) -> Result<bool> {
        // In a real implementation, check against time-based restrictions
        // For now, allow all times
        Ok(true)
    }

    /// Check rate limits for user actions
    async fn check_rate_limits(&self, user_id: &Uuid, action: &str) -> Result<bool> {
        // In a real implementation, check against rate limiting rules
        // For now, allow all actions
        Ok(true)
    }

    /// Get namespace permissions for user
    async fn get_namespace_permissions(
        &self,
        user_id: &Uuid,
        namespace: &str,
    ) -> Result<Vec<NamespacePermission>> {
        // In a real implementation, query database for namespace permissions
        Ok(Vec::new())
    }

    /// Grant namespace permission to user
    pub async fn grant_namespace_permission(
        &self,
        user_id: &Uuid,
        namespace: &str,
        permission_type: NamespacePermissionType,
        granted_by: &Uuid,
        expires_at: Option<DateTime<Utc>>,
        conditions: Vec<PermissionCondition>,
    ) -> Result<NamespacePermission> {
        let permission = NamespacePermission {
            namespace: namespace.to_string(),
            user_id: *user_id,
            permission_type,
            granted_by: *granted_by,
            granted_at: Utc::now(),
            expires_at,
            conditions,
        };

        // Store in database
        self.store_namespace_permission(&permission).await?;
        Ok(permission)
    }

    /// Revoke namespace permission from user
    pub async fn revoke_namespace_permission(
        &self,
        user_id: &Uuid,
        namespace: &str,
        permission_type: NamespacePermissionType,
    ) -> Result<()> {
        self.delete_namespace_permission(user_id, namespace, &permission_type)
            .await
    }

    /// List namespace permissions for user
    pub async fn list_user_namespace_permissions(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<NamespacePermission>> {
        self.get_user_namespace_permissions(user_id).await
    }

    /// Validate permission syntax
    pub fn validate_permission(&self, permission: &Permission) -> Result<()> {
        if permission.resource.is_empty() {
            return Err(GaussOSError::ValidationError(
                "Resource cannot be empty".to_string(),
            ));
        }

        if permission.action.is_empty() {
            return Err(GaussOSError::ValidationError(
                "Action cannot be empty".to_string(),
            ));
        }

        // Validate conditions
        for condition in &permission.conditions {
            self.validate_condition(condition)?;
        }

        Ok(())
    }

    /// Validate permission condition
    fn validate_condition(&self, condition: &PermissionCondition) -> Result<()> {
        if condition.field.is_empty() {
            return Err(GaussOSError::ValidationError(
                "Condition field cannot be empty".to_string(),
            ));
        }

        // Validate condition value based on operator
        match condition.operator {
            crate::auth::roles::ConditionOperator::In
            | crate::auth::roles::ConditionOperator::NotIn => {
                if !condition.value.is_array() {
                    return Err(GaussOSError::ValidationError(
                        "In/NotIn operators require array values".to_string(),
                    ));
                }
            }
            crate::auth::roles::ConditionOperator::Regex => {
                if let Some(pattern) = condition.value.as_str() {
                    // TODO: Implement regex validation when regex crate is available
                    // if regex::Regex::new(pattern).is_err() {
                    //     return Err(GaussOSError::ValidationError(
                    //         "Invalid regex pattern".to_string()
                    //     ));
                    // }
                    // For now, just validate that it's a string
                    if pattern.is_empty() {
                        return Err(GaussOSError::ValidationError(
                            "Regex pattern cannot be empty".to_string(),
                        ));
                    }
                } else {
                    return Err(GaussOSError::ValidationError(
                        "Regex operator requires string value".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    // Database operations (placeholders)
    async fn store_namespace_permission(&self, permission: &NamespacePermission) -> Result<()> {
        tracing::info!(
            "Storing namespace permission for user {} in namespace {}",
            permission.user_id,
            permission.namespace
        );
        Ok(())
    }

    async fn delete_namespace_permission(
        &self,
        user_id: &Uuid,
        namespace: &str,
        permission_type: &NamespacePermissionType,
    ) -> Result<()> {
        tracing::info!(
            "Deleting namespace permission for user {} in namespace {}",
            user_id,
            namespace
        );
        Ok(())
    }

    async fn get_user_namespace_permissions(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<NamespacePermission>> {
        Ok(Vec::new())
    }
}

impl PermissionContext {
    /// Create a new permission context
    pub fn new(user_id: Uuid, resource_type: &str, action: &str) -> Self {
        Self {
            user_id,
            resource_type: resource_type.to_string(),
            resource_id: None,
            namespace: None,
            action: action.to_string(),
            ip_address: None,
            user_agent: None,
            timestamp: Utc::now(),
            session_id: None,
            request_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Add resource ID to context
    pub fn with_resource_id(mut self, resource_id: &str) -> Self {
        self.resource_id = Some(resource_id.to_string());
        self
    }

    /// Add namespace to context
    pub fn with_namespace(mut self, namespace: &str) -> Self {
        self.namespace = Some(namespace.to_string());
        self
    }

    /// Add IP address to context
    pub fn with_ip_address(mut self, ip: std::net::IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Add session ID to context
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Add request ID to context
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }

    /// Add metadata to context
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

impl NamespacePermissionType {
    /// Check if this permission type includes another type
    pub fn includes(&self, other: &NamespacePermissionType) -> bool {
        match (self, other) {
            (NamespacePermissionType::Admin, _) => true,
            (
                NamespacePermissionType::Delete,
                NamespacePermissionType::Write | NamespacePermissionType::Read,
            ) => true,
            (NamespacePermissionType::Write, NamespacePermissionType::Read) => true,
            (a, b) => a == b,
        }
    }

    /// Get all permissions included by this type
    pub fn included_permissions(&self) -> Vec<NamespacePermissionType> {
        match self {
            NamespacePermissionType::Admin => vec![
                NamespacePermissionType::Admin,
                NamespacePermissionType::Delete,
                NamespacePermissionType::Write,
                NamespacePermissionType::Read,
                NamespacePermissionType::Execute,
            ],
            NamespacePermissionType::Delete => vec![
                NamespacePermissionType::Delete,
                NamespacePermissionType::Write,
                NamespacePermissionType::Read,
            ],
            NamespacePermissionType::Write => vec![
                NamespacePermissionType::Write,
                NamespacePermissionType::Read,
            ],
            NamespacePermissionType::Read => vec![NamespacePermissionType::Read],
            NamespacePermissionType::Execute => vec![
                NamespacePermissionType::Execute,
                NamespacePermissionType::Read,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::roles::Permission;

    #[test]
    fn test_permission_context_creation() {
        let user_id = Uuid::new_v4();
        let context = PermissionContext::new(user_id, "memory", "read")
            .with_namespace("test")
            .with_resource_id("memory-123")
            .with_metadata("priority", serde_json::json!("high"));

        assert_eq!(context.user_id, user_id);
        assert_eq!(context.resource_type, "memory");
        assert_eq!(context.action, "read");
        assert_eq!(context.namespace, Some("test".to_string()));
        assert_eq!(context.resource_id, Some("memory-123".to_string()));
        assert!(context.metadata.contains_key("priority"));
    }

    #[test]
    fn test_namespace_permission_inclusion() {
        assert!(NamespacePermissionType::Admin.includes(&NamespacePermissionType::Read));
        assert!(NamespacePermissionType::Delete.includes(&NamespacePermissionType::Write));
        assert!(NamespacePermissionType::Write.includes(&NamespacePermissionType::Read));
        assert!(!NamespacePermissionType::Read.includes(&NamespacePermissionType::Write));
    }

    #[test]
    fn test_included_permissions() {
        let admin_perms = NamespacePermissionType::Admin.included_permissions();
        assert_eq!(admin_perms.len(), 5);
        assert!(admin_perms.contains(&NamespacePermissionType::Read));
        assert!(admin_perms.contains(&NamespacePermissionType::Write));
        assert!(admin_perms.contains(&NamespacePermissionType::Delete));
        assert!(admin_perms.contains(&NamespacePermissionType::Execute));
        assert!(admin_perms.contains(&NamespacePermissionType::Admin));
    }
}
