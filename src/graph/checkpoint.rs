// src/graph/checkpoint.rs
//! Checkpoint system for graph state persistence

use crate::error::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub type CheckpointId = Uuid;

#[async_trait]
pub trait Checkpointer: Send + Sync {
    async fn save(&self, data: Vec<u8>) -> Result<CheckpointId>;
    async fn load(&self, id: CheckpointId) -> Result<Option<Vec<u8>>>;
}

#[async_trait]
pub trait CheckpointStorage: Send + Sync {
    async fn store(&self, id: CheckpointId, data: Vec<u8>) -> Result<()>;
    async fn retrieve(&self, id: CheckpointId) -> Result<Option<Vec<u8>>>;
}
