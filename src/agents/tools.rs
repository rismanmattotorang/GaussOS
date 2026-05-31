// src/agents/tools.rs
use crate::error::{GaussOSError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for agent tools
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Tool name for identification
    fn name(&self) -> &str;

    /// Tool description for agents
    fn description(&self) -> &str;

    /// JSON schema for tool parameters
    fn schema(&self) -> serde_json::Value;

    /// Execute the tool with given arguments
    async fn execute(&self, _name: &str, _args: serde_json::Value) -> Result<ToolExecutionResult>;

    /// Get required permissions for this tool
    fn required_permissions(&self) -> Vec<ToolPermission>;
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub result: serde_json::Value,
    pub message: String,
    pub execution_time_ms: u64,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ToolExecutionResult {
    pub fn success(result: serde_json::Value, message: String) -> Self {
        Self {
            success: true,
            result,
            message,
            execution_time_ms: 0,
            metadata: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            result: serde_json::Value::Null,
            message,
            execution_time_ms: 0,
            metadata: None,
        }
    }

    pub fn with_execution_time(mut self, execution_time_ms: u64) -> Self {
        self.execution_time_ms = execution_time_ms;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Tool permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolPermission {
    ReadMemory,
    WriteMemory,
    DeleteMemory,
    SearchMemory,
    ManageNamespaces,
    OptimizePrompts,
    SubmitReflection,
    ReadSystemInfo,
    WriteSystemConfig,
}

/// Tool permissions for agent access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub allowed_permissions: Vec<ToolPermission>,
    pub namespace_restrictions: Option<Vec<String>>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub max_calls_per_minute: u32,
    pub max_calls_per_hour: u32,
}

impl ToolPermissions {
    pub fn new() -> Self {
        Self {
            allowed_permissions: Vec::new(),
            namespace_restrictions: None,
            rate_limit: None,
        }
    }

    pub fn with_memory_access() -> Self {
        Self {
            allowed_permissions: vec![
                ToolPermission::ReadMemory,
                ToolPermission::WriteMemory,
                ToolPermission::SearchMemory,
            ],
            namespace_restrictions: None,
            rate_limit: Some(RateLimit {
                max_calls_per_minute: 60,
                max_calls_per_hour: 1000,
            }),
        }
    }

    pub fn with_full_access() -> Self {
        Self {
            allowed_permissions: vec![
                ToolPermission::ReadMemory,
                ToolPermission::WriteMemory,
                ToolPermission::DeleteMemory,
                ToolPermission::SearchMemory,
                ToolPermission::ManageNamespaces,
                ToolPermission::OptimizePrompts,
                ToolPermission::SubmitReflection,
                ToolPermission::ReadSystemInfo,
            ],
            namespace_restrictions: None,
            rate_limit: Some(RateLimit {
                max_calls_per_minute: 100,
                max_calls_per_hour: 2000,
            }),
        }
    }

    pub fn has_permission(&self, permission: &ToolPermission) -> bool {
        self.allowed_permissions.contains(permission)
    }

    pub fn can_access_namespace(&self, namespace: &str) -> bool {
        if let Some(restrictions) = &self.namespace_restrictions {
            restrictions
                .iter()
                .any(|pattern| namespace.starts_with(pattern))
        } else {
            true
        }
    }
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool registry for managing agent tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn AgentTool>>,
    usage_stats: HashMap<String, ToolUsageStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub average_execution_time_ms: f64,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            usage_stats: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, tool: Box<dyn AgentTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), tool);
        self.usage_stats.insert(
            name,
            ToolUsageStats {
                total_calls: 0,
                successful_calls: 0,
                failed_calls: 0,
                average_execution_time_ms: 0.0,
                last_used: chrono::Utc::now(),
            },
        );
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn AgentTool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn get_tool_schemas(&self) -> HashMap<String, serde_json::Value> {
        self.tools
            .iter()
            .map(|(name, tool)| (name.clone(), tool.schema()))
            .collect()
    }

    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        args: serde_json::Value,
        permissions: &ToolPermissions,
    ) -> Result<ToolExecutionResult> {
        let start_time = std::time::Instant::now();

        // Get tool
        let tool = self.tools.get(tool_name).ok_or_else(|| {
            GaussOSError::ValidationError(format!("Tool '{}' not found", tool_name))
        })?;

        // Check permissions
        for required_permission in tool.required_permissions() {
            if !permissions.has_permission(&required_permission) {
                return Ok(ToolExecutionResult::error(format!(
                    "Missing permission: {:?}",
                    required_permission
                )));
            }
        }

        // Execute tool
        let result = tool.execute(tool_name, args).await;
        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update usage stats
        if let Some(stats) = self.usage_stats.get_mut(tool_name) {
            stats.total_calls += 1;
            stats.last_used = chrono::Utc::now();

            match &result {
                Ok(exec_result) => {
                    if exec_result.success {
                        stats.successful_calls += 1;
                    } else {
                        stats.failed_calls += 1;
                    }
                }
                Err(_) => {
                    stats.failed_calls += 1;
                }
            }

            // Update average execution time
            stats.average_execution_time_ms = (stats.average_execution_time_ms
                * (stats.total_calls - 1) as f64
                + execution_time as f64)
                / stats.total_calls as f64;
        }

        match result {
            Ok(mut exec_result) => {
                exec_result.execution_time_ms = execution_time;
                Ok(exec_result)
            }
            Err(e) => Ok(
                ToolExecutionResult::error(format!("Tool execution failed: {}", e))
                    .with_execution_time(execution_time),
            ),
        }
    }

    pub fn get_usage_stats(&self, tool_name: &str) -> Option<&ToolUsageStats> {
        self.usage_stats.get(tool_name)
    }

    pub fn get_all_usage_stats(&self) -> &HashMap<String, ToolUsageStats> {
        &self.usage_stats
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiting tracker for tools
#[derive(Debug)]
pub struct RateLimitTracker {
    call_history: HashMap<String, Vec<chrono::DateTime<chrono::Utc>>>,
}

impl RateLimitTracker {
    pub fn new() -> Self {
        Self {
            call_history: HashMap::new(),
        }
    }

    pub fn check_rate_limit(&mut self, user_id: &str, rate_limit: &RateLimit) -> bool {
        let now = chrono::Utc::now();
        let user_history = self
            .call_history
            .entry(user_id.to_string())
            .or_insert_with(Vec::new);

        // Clean old entries
        user_history.retain(|&timestamp| now.signed_duration_since(timestamp).num_hours() < 1);

        // Check hourly limit
        if user_history.len() >= rate_limit.max_calls_per_hour as usize {
            return false;
        }

        // Check minute limit
        let minute_calls = user_history
            .iter()
            .filter(|&&timestamp| now.signed_duration_since(timestamp).num_minutes() < 1)
            .count();

        if minute_calls >= rate_limit.max_calls_per_minute as usize {
            return false;
        }

        // Record this call
        user_history.push(now);
        true
    }
}

impl Default for RateLimitTracker {
    fn default() -> Self {
        Self::new()
    }
}
