// src/graph/pregel.rs
//! Core Pregel runtime for high-performance graph execution
//! Implements the Bulk Synchronous Parallel model with Rust's superior concurrency

use crate::error::Result;
use crate::graph::{ExecutionConfig, StateValue};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use uuid::Uuid;

/// Core trait for graph nodes
#[async_trait]
pub trait PregelNode: Send + Sync + Debug {
    async fn execute(&self, input: StateValue, config: ExecutionConfig) -> Result<NodeOutput>;
    fn get_triggers(&self) -> &[String];
    fn get_writes(&self) -> &[String];
    fn node_id(&self) -> &str;
}

/// Output from node execution
#[derive(Debug, Clone)]
pub struct NodeOutput {
    pub values: HashMap<String, StateValue>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub next_nodes: Vec<String>,
}

impl NodeOutput {
    pub fn new(values: HashMap<String, StateValue>) -> Self {
        Self {
            values,
            metadata: HashMap::new(),
            next_nodes: Vec::new(),
        }
    }
}

/// Executable task for the runtime
#[derive(Debug)]
pub struct ExecutableTask {
    pub id: Uuid,
    pub node_id: String,
    pub input: StateValue,
    pub config: ExecutionConfig,
}

/// High-performance Pregel runtime
#[derive(Debug)]
pub struct PregelRuntime {
    pub name: String,
    pub nodes: HashMap<String, Box<dyn PregelNode>>,
    pub step_count: usize,
    pub max_steps: usize,
}

impl PregelRuntime {
    pub fn new(name: String) -> Self {
        Self {
            name,
            nodes: HashMap::new(),
            step_count: 0,
            max_steps: 100,
        }
    }

    pub fn add_node(&mut self, id: String, node: Box<dyn PregelNode>) {
        self.nodes.insert(id, node);
    }

    pub async fn execute_step(&mut self, tasks: Vec<ExecutableTask>) -> Result<Vec<NodeOutput>> {
        let mut results = Vec::new();

        for task in tasks {
            if let Some(node) = self.nodes.get(&task.node_id) {
                let output = node.execute(task.input, task.config).await?;
                results.push(output);
            }
        }

        self.step_count += 1;
        Ok(results)
    }
}
