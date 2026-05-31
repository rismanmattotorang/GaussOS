// src/agents/orchestrator.rs
//! Agent Orchestration System
//! Manages multiple agents, coordinates their interactions, and handles workflow execution

use crate::{
    agents::{memory_tools::MemoryTools, tools::ToolRegistry},
    error::{GaussOSError, Result},
    memory::MemoryManager,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap; // Lock-free concurrent HashMap
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{RwLock, Semaphore};
use uuid::Uuid;

/// Agent orchestrator for managing multiple agents with optimized concurrency
pub struct AgentOrchestrator {
    // Use DashMap for lock-free concurrent access
    agents: Arc<DashMap<Uuid, AgentInstance>>,

    // Keep tool registry as is since it's read-heavy
    tool_registry: Arc<RwLock<ToolRegistry>>,
    memory_manager: Arc<MemoryManager>,
    config: OrchestratorConfig,

    // Optimized semaphore for controlling concurrency
    execution_semaphore: Arc<Semaphore>,
    workflow_engine: Arc<WorkflowEngine>,

    // Performance tracking with atomic counters
    metrics: Arc<OrchestratorMetrics>,

    // Agent status index for fast lookups
    status_index: Arc<DashMap<AgentStatus, Vec<Uuid>>>,
}

/// Configuration for the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub max_concurrent_agents: usize,
    pub agent_timeout_seconds: u64,
    pub workflow_timeout_seconds: u64,
    pub enable_agent_communication: bool,
    pub enable_workflow_persistence: bool,
    pub max_workflow_steps: usize,
    pub retry_attempts: u32,
}

/// Individual agent instance
#[derive(Clone)]
pub struct AgentInstance {
    pub id: Uuid,
    pub name: String,
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub execution_count: u64,
    pub error_count: u64,
    pub memory_tools: Arc<MemoryTools>,
    pub context: AgentContext,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub capabilities: Vec<String>,
    pub max_memory_size: usize,
    pub conversation_timeout: Duration,
    pub tool_permissions: crate::agents::tools::ToolPermissions,
    pub system_prompt: Option<String>,
    pub model_config: ModelConfig,
    pub workflow_config: WorkflowConfig,
}

/// Types of agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    Conversational,
    TaskExecutor,
    DataAnalyzer,
    WorkflowCoordinator,
    MemoryManager,
    SystemMonitor,
    Custom(String),
}

/// Agent status
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Processing,
    Waiting,
    Error,
    Shutdown,
}

/// Agent execution context
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub session_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub namespace: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub conversation_history: Vec<ConversationEntry>,
}

/// Conversation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Model configuration for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub stop_sequences: Vec<String>,
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub enable_workflows: bool,
    pub max_workflow_depth: usize,
    pub workflow_timeout: Duration,
    pub enable_parallel_execution: bool,
    pub checkpoint_interval: Option<Duration>,
}

/// Workflow engine for orchestrating complex tasks
pub struct WorkflowEngine {
    workflows: Arc<RwLock<HashMap<Uuid, Workflow>>>,
    execution_history: Arc<RwLock<HashMap<Uuid, WorkflowExecution>>>,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub created_at: DateTime<Utc>,
    pub version: u32,
}

/// Individual workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: Uuid,
    pub name: String,
    pub step_type: WorkflowStepType,
    pub agent_id: Option<Uuid>,
    pub tool_name: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<Uuid>,
    pub timeout: Option<Duration>,
    pub retry_config: RetryConfig,
}

/// Types of workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStepType {
    AgentTask,
    ToolExecution,
    DataTransformation,
    ConditionalBranch,
    ParallelExecution,
    WaitForInput,
    Checkpoint,
}

/// Retry configuration for workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub backoff_multiplier: f32,
    pub max_retry_delay: Duration,
}

/// Workflow execution state
#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub status: WorkflowStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub current_step: Option<Uuid>,
    pub completed_steps: Vec<Uuid>,
    pub failed_steps: Vec<Uuid>,
    pub context: HashMap<String, serde_json::Value>,
    pub results: HashMap<Uuid, serde_json::Value>,
}

/// Workflow execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Atomic metrics for performance tracking
#[derive(Debug)]
pub struct OrchestratorMetrics {
    pub total_agents: AtomicUsize,
    pub active_agents: AtomicUsize,
    pub total_executions: AtomicU64,
    pub failed_executions: AtomicU64,
    pub avg_execution_time_ms: AtomicU64,
}

impl OrchestratorMetrics {
    pub fn new() -> Self {
        Self {
            total_agents: AtomicUsize::new(0),
            active_agents: AtomicUsize::new(0),
            total_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            avg_execution_time_ms: AtomicU64::new(0),
        }
    }
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator
    pub async fn new(
        tool_registry: Arc<RwLock<ToolRegistry>>,
        memory_manager: Arc<MemoryManager>,
        config: OrchestratorConfig,
    ) -> Result<Self> {
        let execution_semaphore = Arc::new(Semaphore::new(config.max_concurrent_agents));

        Ok(Self {
            agents: Arc::new(DashMap::new()),
            tool_registry,
            memory_manager,
            config,
            execution_semaphore,
            workflow_engine: Arc::new(WorkflowEngine::new()),
            metrics: Arc::new(OrchestratorMetrics::new()),
            status_index: Arc::new(DashMap::new()),
        })
    }

    /// Create a new agent instance
    pub async fn create_agent(&self, name: String, config: AgentConfig) -> Result<Uuid> {
        let agent_id = Uuid::new_v4();
        let namespace = crate::core::MemoryNamespace::new(vec![format!("agent_{}", agent_id)]);

        let memory_tools = Arc::new(MemoryTools::new(
            self.memory_manager.clone(),
            namespace,
            config.tool_permissions.clone(),
        ));

        let agent = AgentInstance {
            id: agent_id,
            name,
            config,
            status: AgentStatus::Idle,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            execution_count: 0,
            error_count: 0,
            memory_tools,
            context: AgentContext {
                session_id: None,
                user_id: None,
                namespace: format!("agent_{}", agent_id),
                metadata: HashMap::new(),
                conversation_history: Vec::new(),
            },
        };

        let agent_name = agent.name.clone();
        self.agents.insert(agent_id, agent);
        tracing::info!("Created agent {} with ID {}", agent_name, agent_id);

        Ok(agent_id)
    }

    /// Execute a task with a specific agent
    pub async fn execute_agent_task(
        &self,
        agent_id: &Uuid,
        task: AgentTask,
    ) -> Result<AgentTaskResult> {
        let _permit = self.execution_semaphore.acquire().await.map_err(|e| {
            GaussOSError::system_error(
                "orchestrator".to_string(),
                format!("Failed to acquire execution permit: {}", e),
            )
        })?;

        let start_time = std::time::Instant::now();

        // Check if agent exists
        if let Some(agent_ref) = self.agents.get(agent_id) {
            let agent = agent_ref.clone();
            drop(agent_ref); // Release the reference

            // Update agent status to Processing
            self.update_agent_status(agent_id, AgentStatus::Processing)
                .await?;

            // Execute the task
            let result = match task.task_type {
                AgentTaskType::ToolExecution {
                    tool_name,
                    parameters,
                } => self.execute_tool_task(&agent, &tool_name, parameters).await,
                AgentTaskType::MemoryOperation {
                    operation,
                    parameters,
                } => {
                    self.execute_memory_task(&agent, operation, parameters)
                        .await
                }
                AgentTaskType::Conversation { messages, context } => {
                    self.execute_conversation_task(&agent, messages, context)
                        .await
                }
                AgentTaskType::Workflow { workflow_id } => {
                    self.execute_workflow_task(&agent, workflow_id).await
                }
            };

            // Update agent status and statistics
            match &result {
                Ok(_) => {
                    self.increment_agent_execution_count(agent_id).await?;
                    self.update_agent_status(agent_id, AgentStatus::Idle)
                        .await?;
                }
                Err(_) => {
                    self.increment_agent_error_count(agent_id).await?;
                    self.update_agent_status(agent_id, AgentStatus::Error)
                        .await?;
                }
            }

            let execution_time = start_time.elapsed();

            match result {
                Ok(output) => Ok(AgentTaskResult {
                    agent_id: *agent_id,
                    task_id: task.id,
                    success: true,
                    output: Some(output),
                    error: None,
                    execution_time_ms: execution_time.as_millis() as u64,
                    timestamp: Utc::now(),
                }),
                Err(error) => Ok(AgentTaskResult {
                    agent_id: *agent_id,
                    task_id: task.id,
                    success: false,
                    output: None,
                    error: Some(error.to_string()),
                    execution_time_ms: execution_time.as_millis() as u64,
                    timestamp: Utc::now(),
                }),
            }
        } else {
            Err(GaussOSError::NotFound(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    /// Start a workflow execution
    pub async fn start_workflow(&self, workflow_id: &Uuid) -> Result<Uuid> {
        self.workflow_engine.start_workflow(workflow_id).await
    }

    /// Get agent status
    pub async fn get_agent_status(&self, agent_id: &Uuid) -> Result<AgentStatus> {
        if let Some(agent_ref) = self.agents.get(agent_id) {
            Ok(agent_ref.status.clone())
        } else {
            Err(GaussOSError::NotFound(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    /// List all agents
    pub async fn list_agents(&self) -> Vec<AgentInstance> {
        self.agents
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Shutdown agent
    pub async fn shutdown_agent(&self, agent_id: &Uuid) -> Result<()> {
        self.update_agent_status(agent_id, AgentStatus::Shutdown)
            .await?;
        self.agents.remove(agent_id);
        tracing::info!("Shutdown agent {}", agent_id);
        Ok(())
    }

    async fn execute_tool_task(
        &self,
        agent: &AgentInstance,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut tool_registry = self.tool_registry.write().await;
        tool_registry
            .execute_tool(tool_name, parameters, &agent.config.tool_permissions)
            .await
            .map(|result| result.result)
    }

    async fn execute_memory_task(
        &self,
        agent: &AgentInstance,
        operation: MemoryOperation,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match operation {
            MemoryOperation::Create => {
                let content = parameters["content"].as_str().ok_or_else(|| {
                    GaussOSError::ValidationError("Missing content parameter".to_string())
                })?;
                let memory_type = parameters["memory_type"].as_str().unwrap_or("semantic");

                let memory_type_enum = match memory_type {
                    "semantic" => crate::agents::memory_tools::MemoryType::Semantic,
                    "plaintext" => crate::agents::memory_tools::MemoryType::Plaintext,
                    _ => crate::agents::memory_tools::MemoryType::Semantic,
                };

                let memory_id = agent
                    .memory_tools
                    .create_memory(content, memory_type_enum)
                    .await?;
                Ok(serde_json::json!({ "memory_id": memory_id }))
            }
            MemoryOperation::Search => {
                let query = parameters["query"].as_str().ok_or_else(|| {
                    GaussOSError::ValidationError("Missing query parameter".to_string())
                })?;
                let limit = parameters["limit"].as_u64().map(|l| l as usize);

                let memories = agent.memory_tools.search_memories(query, limit).await?;
                Ok(serde_json::to_value(memories)?)
            }
            MemoryOperation::Update => {
                let memory_id = parameters["memory_id"].as_str().ok_or_else(|| {
                    GaussOSError::ValidationError("Missing memory_id parameter".to_string())
                })?;
                let content = parameters["content"].as_str().ok_or_else(|| {
                    GaussOSError::ValidationError("Missing content parameter".to_string())
                })?;

                let uuid = Uuid::parse_str(memory_id)
                    .map_err(|e| GaussOSError::ValidationError(format!("Invalid UUID: {}", e)))?;

                let result = agent.memory_tools.update_memory(&uuid, content).await?;
                Ok(serde_json::json!({ "result": result }))
            }
            MemoryOperation::Delete => {
                let memory_id = parameters["memory_id"].as_str().ok_or_else(|| {
                    GaussOSError::ValidationError("Missing memory_id parameter".to_string())
                })?;

                let uuid = Uuid::parse_str(memory_id)
                    .map_err(|e| GaussOSError::ValidationError(format!("Invalid UUID: {}", e)))?;

                let result = agent.memory_tools.delete_memory(&uuid).await?;
                Ok(serde_json::json!({ "result": result }))
            }
        }
    }

    async fn execute_conversation_task(
        &self,
        _agent: &AgentInstance,
        messages: Vec<ConversationEntry>,
        _context: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let client = crate::agents::llm::AnthropicClient::from_env();

        // Without an API key, report honestly rather than fabricating a reply.
        if !client.is_configured() {
            return Ok(serde_json::json!({
                "status": "llm_not_configured",
                "provider": "anthropic",
                "response": "Agent LLM is not configured. Set ANTHROPIC_API_KEY to enable live responses.",
            }));
        }

        let system = "You are an assistant agent running inside the GaussOS \
                      memory platform. Be concise and accurate.";
        let turns: Vec<crate::agents::llm::ChatTurn> = messages
            .iter()
            .map(|m| crate::agents::llm::ChatTurn::new(m.role.clone(), m.content.clone()))
            .collect();

        match client.complete(system, &turns).await {
            Ok(text) => Ok(serde_json::json!({
                "status": "completed",
                "provider": "anthropic",
                "model": client.model(),
                "response": text,
            })),
            Err(e) => Ok(serde_json::json!({
                "status": "error",
                "provider": "anthropic",
                "error": e.to_string(),
            })),
        }
    }

    async fn execute_workflow_task(
        &self,
        _agent: &AgentInstance,
        workflow_id: Uuid,
    ) -> Result<serde_json::Value> {
        let execution_id = self.workflow_engine.start_workflow(&workflow_id).await?;
        Ok(serde_json::json!({
            "workflow_id": workflow_id,
            "execution_id": execution_id,
            "status": "started"
        }))
    }

    async fn update_agent_status(&self, agent_id: &Uuid, status: AgentStatus) -> Result<()> {
        if let Some(mut agent) = self.agents.get_mut(agent_id) {
            agent.status = status;
            agent.last_activity = Utc::now();
            Ok(())
        } else {
            Err(GaussOSError::AgentError(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    async fn increment_agent_execution_count(&self, agent_id: &Uuid) -> Result<()> {
        if let Some(mut agent) = self.agents.get_mut(agent_id) {
            agent.execution_count += 1;
            Ok(())
        } else {
            Err(GaussOSError::AgentError(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    async fn increment_agent_error_count(&self, agent_id: &Uuid) -> Result<()> {
        if let Some(mut agent) = self.agents.get_mut(agent_id) {
            agent.error_count += 1;
            Ok(())
        } else {
            Err(GaussOSError::AgentError(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }
}

/// Task for agent execution
#[derive(Debug, Clone)]
pub struct AgentTask {
    pub id: Uuid,
    pub task_type: AgentTaskType,
    pub priority: TaskPriority,
    pub timeout: Option<Duration>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of agent tasks
#[derive(Debug, Clone)]
pub enum AgentTaskType {
    ToolExecution {
        tool_name: String,
        parameters: serde_json::Value,
    },
    MemoryOperation {
        operation: MemoryOperation,
        parameters: serde_json::Value,
    },
    Conversation {
        messages: Vec<ConversationEntry>,
        context: HashMap<String, serde_json::Value>,
    },
    Workflow {
        workflow_id: Uuid,
    },
}

/// Memory operations
#[derive(Debug, Clone)]
pub enum MemoryOperation {
    Create,
    Search,
    Update,
    Delete,
}

/// Task priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Result of agent task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskResult {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_workflow(&self, workflow_id: &Uuid) -> Result<Uuid> {
        let execution_id = Uuid::new_v4();

        let execution = WorkflowExecution {
            id: execution_id,
            workflow_id: *workflow_id,
            status: WorkflowStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            current_step: None,
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            context: HashMap::new(),
            results: HashMap::new(),
        };

        self.execution_history
            .write()
            .await
            .insert(execution_id, execution);

        tracing::info!(
            "Started workflow {} with execution ID {}",
            workflow_id,
            execution_id
        );

        Ok(execution_id)
    }
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            agent_timeout_seconds: 300,
            workflow_timeout_seconds: 3600,
            enable_agent_communication: true,
            enable_workflow_persistence: true,
            max_workflow_steps: 100,
            retry_attempts: 3,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_name: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop_sequences: Vec::new(),
        }
    }
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            enable_workflows: true,
            max_workflow_depth: 10,
            workflow_timeout: Duration::from_secs(3600),
            enable_parallel_execution: true,
            checkpoint_interval: Some(Duration::from_secs(60)),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(60),
        }
    }
}
