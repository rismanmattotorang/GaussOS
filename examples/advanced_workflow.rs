//! Advanced GaussOS Example – End-to-End Workflow Execution
//! -------------------------------------------------------
//! This example demonstrates how to:
//! 1. Initialise GaussOS with high-performance settings.
//! 2. Register a custom agent with tool permissions.
//! 3. Create and execute a multi-step workflow that stores, searches and analyses memories.
//! 4. Collect performance metrics at the end.
//!
//! Run with:
//! ```bash
//! cargo run --release --example advanced_workflow
//! ```

use gaussos::agents::{
    AgentConfig, AgentOrchestrator, AgentTaskType, AgentType, MemoryOperation, TaskPriority,
    WorkflowConfig,
};
use gaussos::performance::CacheStrategy;
use gaussos::{GaussOS, MemCube, MemoryPayload};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Spin-up GaussOS with all the goodies enabled.
    let gaussos = GaussOS::builder()
        .enable_lock_free_operations(true)
        .enable_simd_acceleration(true)
        .cache_strategy(CacheStrategy::ARC)
        .build()
        .await?;

    // 2. Register a super-charged agent that can talk to memories.
    let orchestrator = Arc::new(
        AgentOrchestrator::new(
            gaussos.tool_registry().clone(),
            gaussos.memory_manager().clone(),
            Default::default(),
        )
        .await?,
    );

    let agent_id = orchestrator
        .create_agent(
            "simd-agent".into(),
            AgentConfig {
                agent_type: AgentType::TaskExecutor,
                capabilities: vec!["memory.create".into(), "memory.search".into()],
                max_memory_size: 4096,
                conversation_timeout: std::time::Duration::from_secs(30),
                tool_permissions: Default::default(),
                system_prompt: None,
                model_config: Default::default(),
                workflow_config: WorkflowConfig::default(),
            },
        )
        .await?;

    // 3. Construct a simple workflow: Store ➜ Search ➜ Log
    let mem = MemCube::new(MemoryPayload::Text(
        "GaussOS unleashes SIMD power for blazing fast analytics".into(),
    ));

    orchestrator
        .execute_agent_task(
            &agent_id,
            gaussos::agents::AgentTask {
                id: Uuid::new_v4(),
                task_type: AgentTaskType::MemoryOperation {
                    operation: MemoryOperation::Create,
                    parameters: serde_json::json!({ "memory": mem }),
                },
                priority: TaskPriority::High,
                timeout: None,
                metadata: Default::default(),
            },
        )
        .await?;

    // Search for it again.
    orchestrator
        .execute_agent_task(
            &agent_id,
            gaussos::agents::AgentTask {
                id: Uuid::new_v4(),
                task_type: AgentTaskType::MemoryOperation {
                    operation: MemoryOperation::Search,
                    parameters: serde_json::json!({ "query": "SIMD power" }),
                },
                priority: TaskPriority::Normal,
                timeout: None,
                metadata: Default::default(),
            },
        )
        .await?;

    // 4. Grab and display performance numbers.
    let metrics = gaussos.performance_metrics();
    println!(
        "Throughput: {:.2} ops/s | Cache hit-rate: {:.2}%",
        metrics.ops_per_second,
        metrics.cache_hit_rate * 100.0
    );

    Ok(())
}
