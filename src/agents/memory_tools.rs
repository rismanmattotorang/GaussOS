// src/agents/memory_tools.rs
use crate::{
    agents::tools::{AgentTool, ToolExecutionResult, ToolPermission},
    core::{
        ConversationMetadata, InteractionType, MemCube, MemoryNamespace, MemoryPayload,
        SemanticType,
    },
    error::{GaussOSError, Result},
    memory::{ExtractionMode, MemoryExtractionRequest, MemoryManager},
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

/// Memory tools for agent access to the memory system
#[derive(Clone)]
pub struct MemoryTools {
    pub manager: Arc<MemoryManager>,
    pub namespace: MemoryNamespace,
    pub permissions: crate::agents::tools::ToolPermissions,
}

/// Memory summary for agent consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub confidence: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub namespace: String,
}

/// Memory type enum for agent tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Semantic,
    Episodic,
    Procedural,
    Plaintext,
    Parametric,
    Activation,
}

impl MemoryTools {
    pub fn new(
        manager: Arc<MemoryManager>,
        namespace: MemoryNamespace,
        permissions: crate::agents::tools::ToolPermissions,
    ) -> Self {
        Self {
            manager,
            namespace,
            permissions,
        }
    }

    pub async fn create_memory(
        &self,
        content: &str,
        memory_type: MemoryType,
    ) -> crate::error::Result<String> {
        // Create appropriate payload based on memory type
        let payload = match memory_type {
            MemoryType::Semantic => MemoryPayload::Semantic {
                content: content.to_string(),
                schema_type: SemanticType::UserPreference,
                confidence: 0.8,
                extracted_at: Utc::now(),
                source_context: "agent_created".to_string(),
                embeddings: None,
                validation_metadata: None,
            },
            MemoryType::Plaintext => MemoryPayload::Plaintext {
                content: content.to_string(),
                encoding: "utf-8".to_string(),
                language: Some("en".to_string()),
                embeddings: None,
            },
            _ => {
                return Err(crate::error::GaussOSError::ValidationError(
                    "Memory type not supported for agent creation".to_string(),
                ));
            }
        };

        let memory = MemCube::new_with_namespace(payload, self.namespace.clone());
        let id = self.manager.create_memory(memory).await?;
        Ok(id.to_string())
    }

    pub async fn search_memories(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> crate::error::Result<Vec<MemorySummary>> {
        let mut search_query = crate::database::SearchQuery::default();
        search_query.text = Some(query.to_string());
        search_query.limit = limit.map(|l| l as u64);

        // Add namespace filter
        search_query.filters.insert(
            "namespace_path".to_string(),
            serde_json::Value::String(self.namespace.to_string()),
        );

        let memories = self.manager.search_memories(search_query).await?;

        Ok(memories
            .into_iter()
            .map(|mem| {
                let tags = mem.metadata.tags.clone();
                let namespace_path = mem.get_namespace_path().join("/");
                MemorySummary {
                    id: mem.id.to_string(),
                    content: mem.get_content_summary(),
                    memory_type: payload_to_memory_type(&mem.payload),
                    confidence: extract_confidence(&mem.payload),
                    created_at: mem.created_at,
                    tags,
                    namespace: namespace_path,
                }
            })
            .collect())
    }

    pub async fn update_memory(&self, id: &Uuid, content: &str) -> crate::error::Result<String> {
        if let Some(mut memory) = self.manager.get_memory(id).await? {
            // Update content based on memory type
            match &mut memory.payload {
                MemoryPayload::Semantic { content: c, .. } => {
                    *c = content.to_string();
                }
                MemoryPayload::Plaintext { content: c, .. } => {
                    *c = content.to_string();
                }
                _ => {
                    return Err(crate::error::GaussOSError::ValidationError(
                        "Memory type not supported for content updates".to_string(),
                    ));
                }
            }

            self.manager.update_memory(&memory).await?;
            Ok("Memory updated successfully".to_string())
        } else {
            Err(crate::error::GaussOSError::memory_not_found(*id))
        }
    }

    pub async fn delete_memory(&self, id: &Uuid) -> crate::error::Result<String> {
        self.manager.delete_memory(id).await?;
        Ok("Memory deleted successfully".to_string())
    }
}

fn payload_to_memory_type(payload: &MemoryPayload) -> MemoryType {
    match payload {
        MemoryPayload::Semantic { .. } => MemoryType::Semantic,
        MemoryPayload::Episodic { .. } => MemoryType::Episodic,
        MemoryPayload::Procedural { .. } => MemoryType::Procedural,
        MemoryPayload::Plaintext { .. } => MemoryType::Plaintext,
        MemoryPayload::Parametric { .. } => MemoryType::Parametric,
        MemoryPayload::Activation { .. } => MemoryType::Activation,
        MemoryPayload::Text(_) => MemoryType::Plaintext,
    }
}

fn extract_confidence(payload: &MemoryPayload) -> f32 {
    match payload {
        MemoryPayload::Semantic { confidence, .. } => *confidence,
        _ => 1.0,
    }
}
