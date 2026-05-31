// src/graph/state.rs
//! State management system for graph execution
//! Provides type-safe state handling with automatic merging and conflict resolution

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// Core state value type
pub type StateValue = serde_json::Value;

/// State update representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    pub path: String,
    pub value: StateValue,
    pub operation: UpdateOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateOperation {
    Set,
    Merge,
    Append,
    Delete,
}

/// Core trait for state schemas
pub trait StateSchema:
    Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + Debug + 'static
{
    fn merge(current: &mut Self, update: Self) -> Result<()>;
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Graph state container
#[derive(Debug)]
pub struct StateGraph<S: StateSchema> {
    pub state: S,
    pub version: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<S: StateSchema> StateGraph<S> {
    pub fn new(initial_state: S) -> Self {
        Self {
            state: initial_state,
            version: 0,
            metadata: HashMap::new(),
        }
    }

    pub async fn update_state(&mut self, update: S) -> Result<()> {
        S::merge(&mut self.state, update)?;
        self.version += 1;
        Ok(())
    }

    pub fn get_state(&self) -> &S {
        &self.state
    }

    pub fn get_version(&self) -> u64 {
        self.version
    }
}
