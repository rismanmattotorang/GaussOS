// src/agents/conversation.rs
//! Conversation Management for Agents
//! Handles conversation context, history, and state management

use crate::{
    core::MemCube,
    error::{GaussOSError, Result},
    memory::MemoryManager,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Conversation manager for handling agent conversations
pub struct ConversationManager {
    conversations: Arc<RwLock<HashMap<Uuid, ConversationContext>>>,
    memory_manager: Arc<MemoryManager>,
    config: ConversationConfig,
}

/// Configuration for conversation management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationConfig {
    pub max_history_length: usize,
    pub auto_save_interval: std::time::Duration,
    pub enable_context_compression: bool,
    pub context_window_size: usize,
    pub enable_memory_persistence: bool,
    pub conversation_timeout: std::time::Duration,
}

/// Conversation context containing state and history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: ConversationStatus,
    pub history: ConversationHistory,
    pub metadata: HashMap<String, serde_json::Value>,
    pub context_variables: HashMap<String, serde_json::Value>,
    pub timeout_seconds: Option<u64>,
}

/// Conversation history with messages and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
    pub total_tokens: u32,
    pub compressed_history: Option<String>,
}

/// Individual conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: Option<u32>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub attachments: Vec<MessageAttachment>,
}

/// Message roles in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: Option<u64>,
}

/// Message attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub id: Uuid,
    pub attachment_type: AttachmentType,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of message attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentType {
    Image,
    Document,
    Audio,
    Video,
    Code,
    Memory,
    Custom(String),
}

/// Conversation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationStatus {
    Active,
    Paused,
    Completed,
    Archived,
    Error,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new(memory_manager: Arc<MemoryManager>, config: ConversationConfig) -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            memory_manager,
            config,
        }
    }

    /// Start a new conversation
    pub async fn start_conversation(
        &self,
        agent_id: Uuid,
        user_id: Option<Uuid>,
        session_id: Option<Uuid>,
    ) -> Result<Uuid> {
        let conversation_id = Uuid::new_v4();

        let context = ConversationContext {
            id: conversation_id,
            agent_id,
            user_id,
            session_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: ConversationStatus::Active,
            history: ConversationHistory {
                messages: Vec::new(),
                total_tokens: 0,
                compressed_history: None,
            },
            metadata: HashMap::new(),
            context_variables: HashMap::new(),
            timeout_seconds: None,
        };

        self.conversations
            .write()
            .await
            .insert(conversation_id, context);

        tracing::info!(
            "Started conversation {} for agent {}",
            conversation_id,
            agent_id
        );

        Ok(conversation_id)
    }

    /// Add a message to the conversation
    pub async fn add_message(
        &self,
        conversation_id: &Uuid,
        role: MessageRole,
        content: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Uuid> {
        let message_id = Uuid::new_v4();

        let message = ConversationMessage {
            id: message_id,
            role,
            content,
            timestamp: Utc::now(),
            token_count: None, // TODO: Implement token counting
            metadata: metadata.unwrap_or_default(),
            tool_calls: None,
            attachments: Vec::new(),
        };

        let mut conversations = self.conversations.write().await;
        if let Some(context) = conversations.get_mut(conversation_id) {
            context.history.messages.push(message);
            context.updated_at = Utc::now();

            // Check if history needs compression
            if context.history.messages.len() > self.config.max_history_length {
                self.compress_history(context).await?;
            }

            // Persist to memory if enabled
            if self.config.enable_memory_persistence {
                self.persist_conversation_to_memory(context).await?;
            }
        } else {
            return Err(GaussOSError::NotFound(format!(
                "Conversation {} not found",
                conversation_id
            )));
        }

        Ok(message_id)
    }

    /// Get conversation context
    pub async fn get_conversation(
        &self,
        conversation_id: &Uuid,
    ) -> Result<Option<ConversationContext>> {
        let conversations = self.conversations.read().await;
        Ok(conversations.get(conversation_id).cloned())
    }

    /// Get conversation history
    pub async fn get_history(&self, conversation_id: &Uuid) -> Result<ConversationHistory> {
        let conversations = self.conversations.read().await;
        conversations
            .get(conversation_id)
            .map(|context| context.history.clone())
            .ok_or_else(|| {
                GaussOSError::NotFound(format!("Conversation {} not found", conversation_id))
            })
    }

    /// Update conversation status
    pub async fn update_status(
        &self,
        conversation_id: &Uuid,
        status: ConversationStatus,
    ) -> Result<()> {
        let mut conversations = self.conversations.write().await;
        if let Some(context) = conversations.get_mut(conversation_id) {
            context.status = status;
            context.updated_at = Utc::now();
        } else {
            return Err(GaussOSError::NotFound(format!(
                "Conversation {} not found",
                conversation_id
            )));
        }
        Ok(())
    }

    /// Set context variable
    pub async fn set_context_variable(
        &self,
        conversation_id: &Uuid,
        key: String,
        value: serde_json::Value,
    ) -> Result<()> {
        let mut conversations = self.conversations.write().await;
        if let Some(context) = conversations.get_mut(conversation_id) {
            context.context_variables.insert(key, value);
            context.updated_at = Utc::now();
        } else {
            return Err(GaussOSError::NotFound(format!(
                "Conversation {} not found",
                conversation_id
            )));
        }
        Ok(())
    }

    /// Get context variable
    pub async fn get_context_variable(
        &self,
        conversation_id: &Uuid,
        key: &str,
    ) -> Result<Option<serde_json::Value>> {
        let conversations = self.conversations.read().await;
        Ok(conversations
            .get(conversation_id)
            .and_then(|context| context.context_variables.get(key))
            .cloned())
    }

    /// Archive conversation
    pub async fn archive_conversation(&self, conversation_id: &Uuid) -> Result<()> {
        self.update_status(conversation_id, ConversationStatus::Archived)
            .await?;

        // Optionally move to long-term storage
        if self.config.enable_memory_persistence {
            if let Some(context) = self.get_conversation(conversation_id).await? {
                self.persist_conversation_to_memory(&context).await?;
            }
        }

        Ok(())
    }

    /// Clean up expired conversations
    pub async fn cleanup_expired_conversations(&self) -> Result<usize> {
        let mut conversations = self.conversations.write().await;
        let now = Utc::now();
        let timeout =
            chrono::Duration::from_std(self.config.conversation_timeout).map_err(|e| {
                GaussOSError::system_error(
                    "conversation".to_string(),
                    format!("Invalid timeout duration: {}", e),
                )
            })?;

        let expired_keys: Vec<Uuid> = conversations
            .iter()
            .filter(|(_, context)| now - context.updated_at > timeout)
            .map(|(id, _)| *id)
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            conversations.remove(&key);
        }

        tracing::info!("Cleaned up {} expired conversations", count);
        Ok(count)
    }

    /// Compress conversation history
    async fn compress_history(&self, context: &mut ConversationContext) -> Result<()> {
        if !self.config.enable_context_compression {
            return Ok(());
        }

        // Simple compression: keep recent messages and summarize older ones
        if context.history.messages.len() > self.config.context_window_size {
            let keep_count = self.config.context_window_size / 2;
            let to_compress = context.history.messages.split_off(keep_count);

            // Create a summary of compressed messages
            let summary = format!(
                "Conversation summary: {} messages from {} to {}",
                to_compress.len(),
                to_compress
                    .first()
                    .map(|m| m.timestamp.to_rfc3339())
                    .unwrap_or_default(),
                to_compress
                    .last()
                    .map(|m| m.timestamp.to_rfc3339())
                    .unwrap_or_default()
            );

            context.history.compressed_history = Some(summary);
            tracing::debug!(
                "Compressed {} messages for conversation {}",
                to_compress.len(),
                context.id
            );
        }

        Ok(())
    }

    /// Persist conversation to memory system
    async fn persist_conversation_to_memory(&self, context: &ConversationContext) -> Result<()> {
        let conversation_data = serde_json::to_value(context)?;

        let payload = crate::core::MemoryPayload::Semantic {
            content: format!(
                "Conversation {} between agent {} and user {:?}",
                context.id, context.agent_id, context.user_id
            ),
            schema_type: crate::core::SemanticType::UserPreference,
            confidence: 0.8,
            extracted_at: Utc::now(),
            source_context: "conversation_manager".to_string(),
            embeddings: None,
            validation_metadata: match conversation_data {
                serde_json::Value::Object(map) => Some(
                    map.into_iter()
                        .collect::<HashMap<String, serde_json::Value>>(),
                ),
                _ => None,
            },
        };

        let namespace = crate::core::MemoryNamespace::new(vec![
            "conversations".to_string(),
            context.agent_id.to_string(),
        ]);
        let memory = MemCube::new_with_namespace(payload, namespace);

        self.memory_manager.create_memory(memory).await?;

        Ok(())
    }
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_history_length: 100,
            auto_save_interval: std::time::Duration::from_secs(300), // 5 minutes
            enable_context_compression: true,
            context_window_size: 50,
            enable_memory_persistence: true,
            conversation_timeout: std::time::Duration::from_secs(3600), // 1 hour
        }
    }
}

impl ConversationMessage {
    /// Add a tool call to the message
    pub fn add_tool_call(&mut self, tool_call: ToolCall) {
        if self.tool_calls.is_none() {
            self.tool_calls = Some(Vec::new());
        }
        self.tool_calls.as_mut().unwrap().push(tool_call);
    }

    /// Add an attachment to the message
    pub fn add_attachment(&mut self, attachment: MessageAttachment) {
        self.attachments.push(attachment);
    }
}
