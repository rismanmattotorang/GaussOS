// src/graph/executor.rs
//! High-performance executor for graph tasks

use crate::{agents::orchestrator::AgentOrchestrator, core::MemCube, database::MemVault};
use std::time::Duration;

#[derive(Debug)]
pub struct PregelExecutor {
    pub workers: usize,
}

impl PregelExecutor {
    pub fn new(workers: usize) -> Self {
        Self { workers }
    }
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub success: bool,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub tasks_completed: usize,
    pub total_duration: Duration,
}
