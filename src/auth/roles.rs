// src/auth/roles.rs
//! Role-Based Access Control (RBAC)
//! Provides comprehensive role and permission management

use crate::{
    database::MemVault,
    error::{GaussOSError, Result},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Role Manager for RBAC operations
pub struct RoleManager {
    database: Arc<dyn MemVault>,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub is_system_role: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    /// Permission resource (e.g., "memory", "user", "system")
    pub resource: String,
    /// Permission action (e.g., "read", "write", "delete", "admin")
    pub action: String,
    /// Optional resource scope (e.g., namespace, specific ID)
    pub scope: Option<String>,
    /// Permission conditions
    pub conditions: Vec<PermissionCondition>,
}

/// Permission condition for fine-grained access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PermissionCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

/// Condition operators for permission evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Regex,
}

/// Role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub assigned_by: Uuid,
    pub assigned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub conditions: Vec<AssignmentCondition>,
}

/// Assignment condition for conditional role assignments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentCondition {
    pub condition_type: AssignmentConditionType,
    pub value: serde_json::Value,
}

/// Types of assignment conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssignmentConditionType {
    TimeRange,
    IpRange,
    Location,
    Custom,
}

/// Role creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Permission check context
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub user_id: Uuid,
    pub resource_id: Option<String>,
    pub namespace: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RoleManager {
    /// Create a new role manager
    pub fn new(database: Arc<dyn MemVault>) -> Self {
        Self { database }
    }

    /// Create a new role
    pub async fn create_role(&self, request: CreateRoleRequest) -> Result<Role> {
        // Validate role name uniqueness
        if self.get_role_by_name(&request.name).await?.is_some() {
            return Err(GaussOSError::ConflictError(format!(
                "Role '{}' already exists",
                request.name
            )));
        }

        let role = Role {
            id: Uuid::new_v4(),
            name: request.name,
            description: request.description,
            permissions: request.permissions,
            is_system_role: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: request.metadata.unwrap_or_default(),
        };

        self.store_role(&role).await?;
        Ok(role)
    }

    /// Get role by ID
    pub async fn get_role(&self, role_id: &Uuid) -> Result<Option<Role>> {
        self.get_role_by_id(role_id).await
    }

    /// Get role by name
    pub async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>> {
        self.find_role_by_name(name).await
    }

    /// Update role
    pub async fn update_role(&self, role_id: &Uuid, updates: UpdateRoleRequest) -> Result<Role> {
        let mut role = self
            .get_role_by_id(role_id)
            .await?
            .ok_or_else(|| GaussOSError::NotFound(format!("Role {} not found", role_id)))?;

        if role.is_system_role && !updates.allow_system_role_updates {
            return Err(GaussOSError::AuthorizationError(
                "Cannot modify system roles".to_string(),
            ));
        }

        if let Some(name) = updates.name {
            role.name = name;
        }
        if let Some(description) = updates.description {
            role.description = description;
        }
        if let Some(permissions) = updates.permissions {
            role.permissions = permissions;
        }
        if let Some(metadata) = updates.metadata {
            role.metadata = metadata;
        }

        role.updated_at = Utc::now();
        self.update_role_in_db(&role).await?;
        Ok(role)
    }

    /// Delete role
    pub async fn delete_role(&self, role_id: &Uuid) -> Result<()> {
        let role = self
            .get_role_by_id(role_id)
            .await?
            .ok_or_else(|| GaussOSError::NotFound(format!("Role {} not found", role_id)))?;

        if role.is_system_role {
            return Err(GaussOSError::AuthorizationError(
                "Cannot delete system roles".to_string(),
            ));
        }

        // Check if role is assigned to any users
        let assignments = self.get_role_assignments(role_id).await?;
        if !assignments.is_empty() {
            return Err(GaussOSError::ConflictError(
                "Cannot delete role that is assigned to users".to_string(),
            ));
        }

        self.delete_role_from_db(role_id).await
    }

    /// List all roles
    pub async fn list_roles(&self, include_system: bool) -> Result<Vec<Role>> {
        let mut roles = self.get_all_roles().await?;
        if !include_system {
            roles.retain(|r| !r.is_system_role);
        }
        Ok(roles)
    }

    /// Assign role to user
    pub async fn assign_role(
        &self,
        user_id: &Uuid,
        role_id: &Uuid,
        assigned_by: &Uuid,
        expires_at: Option<DateTime<Utc>>,
        conditions: Vec<AssignmentCondition>,
    ) -> Result<RoleAssignment> {
        // Verify role exists
        if self.get_role_by_id(role_id).await?.is_none() {
            return Err(GaussOSError::NotFound(format!(
                "Role {} not found",
                role_id
            )));
        }

        // Check if assignment already exists
        if let Some(existing) = self.get_user_role_assignment(user_id, role_id).await? {
            return Err(GaussOSError::ConflictError(
                "Role already assigned to user".to_string(),
            ));
        }

        let assignment = RoleAssignment {
            id: Uuid::new_v4(),
            user_id: *user_id,
            role_id: *role_id,
            assigned_by: *assigned_by,
            assigned_at: Utc::now(),
            expires_at,
            conditions,
        };

        self.store_role_assignment(&assignment).await?;
        Ok(assignment)
    }

    /// Revoke role from user
    pub async fn revoke_role(&self, user_id: &Uuid, role_id: &Uuid) -> Result<()> {
        if let Some(assignment) = self.get_user_role_assignment(user_id, role_id).await? {
            self.delete_role_assignment(&assignment.id).await?;
        }
        Ok(())
    }

    /// Get user's roles
    pub async fn get_user_roles(&self, user_id: &Uuid) -> Result<Vec<Role>> {
        let assignments = self.get_user_role_assignments(user_id).await?;
        let mut roles = Vec::new();

        for assignment in assignments {
            // Check if assignment is still valid
            if let Some(expires_at) = assignment.expires_at {
                if Utc::now() > expires_at {
                    continue;
                }
            }

            if let Some(role) = self.get_role_by_id(&assignment.role_id).await? {
                roles.push(role);
            }
        }

        Ok(roles)
    }

    /// Get user's effective permissions
    pub async fn get_user_permissions(&self, user_id: &Uuid) -> Result<Vec<Permission>> {
        let roles = self.get_user_roles(user_id).await?;
        let mut permissions = HashSet::new();

        for role in roles {
            for permission in role.permissions {
                permissions.insert(permission);
            }
        }

        Ok(permissions.into_iter().collect())
    }

    /// Check if user has specific permission
    pub async fn has_permission(
        &self,
        user_id: &Uuid,
        required_permission: &Permission,
        context: &PermissionContext,
    ) -> Result<bool> {
        let user_permissions = self.get_user_permissions(user_id).await?;

        for permission in user_permissions {
            if self.permission_matches(&permission, required_permission, context) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Initialize system roles
    pub async fn initialize_system_roles(&self) -> Result<()> {
        let system_roles = self.get_default_system_roles();

        for role in system_roles {
            if self.get_role_by_name(&role.name).await?.is_none() {
                self.store_role(&role).await?;
            }
        }

        Ok(())
    }

    /// Check if permission matches required permission
    fn permission_matches(
        &self,
        user_permission: &Permission,
        required_permission: &Permission,
        context: &PermissionContext,
    ) -> bool {
        // Check resource match
        if user_permission.resource != "*"
            && user_permission.resource != required_permission.resource
        {
            return false;
        }

        // Check action match
        if user_permission.action != "*" && user_permission.action != required_permission.action {
            return false;
        }

        // Check scope match
        match (&user_permission.scope, &required_permission.scope) {
            (Some(user_scope), Some(required_scope)) => {
                if user_scope != "*" && user_scope != required_scope {
                    return false;
                }
            }
            (Some(user_scope), None) => {
                if user_scope != "*" {
                    return false;
                }
            }
            _ => {}
        }

        // Check conditions
        for condition in &user_permission.conditions {
            if !self.evaluate_condition(condition, context) {
                return false;
            }
        }

        true
    }

    /// Evaluate permission condition
    fn evaluate_condition(
        &self,
        condition: &PermissionCondition,
        context: &PermissionContext,
    ) -> bool {
        let context_value = match condition.field.as_str() {
            "user_id" => serde_json::json!(context.user_id.to_string()),
            "namespace" => serde_json::json!(context.namespace),
            "ip_address" => serde_json::json!(context.ip_address.map(|ip| ip.to_string())),
            "timestamp" => serde_json::json!(context.timestamp.timestamp()),
            field => context
                .metadata
                .get(field)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        };

        match condition.operator {
            ConditionOperator::Equals => context_value == condition.value,
            ConditionOperator::NotEquals => context_value != condition.value,
            ConditionOperator::Contains => {
                if let (serde_json::Value::String(haystack), serde_json::Value::String(needle)) =
                    (&context_value, &condition.value)
                {
                    haystack.contains(needle)
                } else {
                    false
                }
            }
            ConditionOperator::NotContains => {
                if let (serde_json::Value::String(haystack), serde_json::Value::String(needle)) =
                    (&context_value, &condition.value)
                {
                    !haystack.contains(needle)
                } else {
                    true
                }
            }
            ConditionOperator::GreaterThan => {
                if let (Some(ctx_num), Some(cond_num)) =
                    (context_value.as_f64(), condition.value.as_f64())
                {
                    ctx_num > cond_num
                } else {
                    false
                }
            }
            ConditionOperator::LessThan => {
                if let (Some(ctx_num), Some(cond_num)) =
                    (context_value.as_f64(), condition.value.as_f64())
                {
                    ctx_num < cond_num
                } else {
                    false
                }
            }
            ConditionOperator::In => {
                if let serde_json::Value::Array(array) = &condition.value {
                    array.contains(&context_value)
                } else {
                    false
                }
            }
            ConditionOperator::NotIn => {
                if let serde_json::Value::Array(array) = &condition.value {
                    !array.contains(&context_value)
                } else {
                    true
                }
            }
            ConditionOperator::Regex => {
                // In a real implementation, compile and cache regex patterns
                if let (serde_json::Value::String(text), serde_json::Value::String(pattern)) =
                    (&context_value, &condition.value)
                {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        regex.is_match(text)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Get default system roles
    fn get_default_system_roles(&self) -> Vec<Role> {
        vec![
            Role {
                id: Uuid::new_v4(),
                name: "admin".to_string(),
                description: "System Administrator".to_string(),
                permissions: vec![Permission {
                    resource: "*".to_string(),
                    action: "*".to_string(),
                    scope: None,
                    conditions: Vec::new(),
                }],
                is_system_role: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                metadata: HashMap::new(),
            },
            Role {
                id: Uuid::new_v4(),
                name: "user".to_string(),
                description: "Regular User".to_string(),
                permissions: vec![
                    Permission {
                        resource: "memory".to_string(),
                        action: "read".to_string(),
                        scope: None,
                        conditions: Vec::new(),
                    },
                    Permission {
                        resource: "memory".to_string(),
                        action: "write".to_string(),
                        scope: None,
                        conditions: Vec::new(),
                    },
                ],
                is_system_role: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                metadata: HashMap::new(),
            },
            Role {
                id: Uuid::new_v4(),
                name: "readonly".to_string(),
                description: "Read-only User".to_string(),
                permissions: vec![Permission {
                    resource: "memory".to_string(),
                    action: "read".to_string(),
                    scope: None,
                    conditions: Vec::new(),
                }],
                is_system_role: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                metadata: HashMap::new(),
            },
        ]
    }

    // Database operations (would be implemented with actual database calls)
    async fn store_role(&self, role: &Role) -> Result<()> {
        tracing::info!("Storing role: {}", role.name);
        Ok(())
    }

    async fn get_role_by_id(&self, id: &Uuid) -> Result<Option<Role>> {
        Ok(None)
    }

    async fn find_role_by_name(&self, name: &str) -> Result<Option<Role>> {
        Ok(None)
    }

    async fn update_role_in_db(&self, role: &Role) -> Result<()> {
        tracing::info!("Updating role: {}", role.name);
        Ok(())
    }

    async fn delete_role_from_db(&self, id: &Uuid) -> Result<()> {
        tracing::info!("Deleting role: {}", id);
        Ok(())
    }

    async fn get_all_roles(&self) -> Result<Vec<Role>> {
        Ok(Vec::new())
    }

    async fn store_role_assignment(&self, assignment: &RoleAssignment) -> Result<()> {
        tracing::info!("Storing role assignment: {}", assignment.id);
        Ok(())
    }

    async fn get_user_role_assignment(
        &self,
        user_id: &Uuid,
        role_id: &Uuid,
    ) -> Result<Option<RoleAssignment>> {
        Ok(None)
    }

    async fn get_user_role_assignments(&self, user_id: &Uuid) -> Result<Vec<RoleAssignment>> {
        Ok(Vec::new())
    }

    async fn get_role_assignments(&self, role_id: &Uuid) -> Result<Vec<RoleAssignment>> {
        Ok(Vec::new())
    }

    async fn delete_role_assignment(&self, id: &Uuid) -> Result<()> {
        tracing::info!("Deleting role assignment: {}", id);
        Ok(())
    }
}

/// Update role request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<Permission>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub allow_system_role_updates: bool,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: &str, action: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            scope: None,
            conditions: Vec::new(),
        }
    }

    /// Add scope to permission
    pub fn with_scope(mut self, scope: &str) -> Self {
        self.scope = Some(scope.to_string());
        self
    }

    /// Add condition to permission
    pub fn with_condition(mut self, condition: PermissionCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Create wildcard permission (all resources, all actions)
    pub fn wildcard() -> Self {
        Self::new("*", "*")
    }
}

impl PermissionCondition {
    /// Create a new permission condition
    pub fn new(field: &str, operator: ConditionOperator, value: serde_json::Value) -> Self {
        Self {
            field: field.to_string(),
            operator,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new("memory", "read")
            .with_scope("test")
            .with_condition(PermissionCondition::new(
                "namespace",
                ConditionOperator::Equals,
                serde_json::json!("test"),
            ));

        assert_eq!(perm.resource, "memory");
        assert_eq!(perm.action, "read");
        assert_eq!(perm.scope, Some("test".to_string()));
        assert_eq!(perm.conditions.len(), 1);
    }

    #[test]
    fn test_wildcard_permission() {
        let perm = Permission::wildcard();
        assert_eq!(perm.resource, "*");
        assert_eq!(perm.action, "*");
    }
}
