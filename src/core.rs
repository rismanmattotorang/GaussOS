// src/core.rs
//! Core data structures and types for GaussOS
//! Defines memory cubes, payloads, metadata, and fundamental abstractions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core memory cube structure representing a unit of memory in GaussOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemCube {
    /// Unique identifier for the memory cube
    pub id: Uuid,

    /// Structured metadata about the memory
    pub metadata: MemoryMetadata,

    /// The actual content/data payload
    pub payload: MemoryPayload,

    /// Namespace for organizing memories
    pub namespace: MemoryNamespace,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Version counter for optimistic concurrency control
    pub version: u32,
}

/// Comprehensive metadata for memory cubes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Human-readable name for the memory
    pub name: Option<String>,

    /// Detailed description
    pub description: Option<String>,

    /// Searchable tags
    pub tags: Vec<String>,

    /// Priority level for processing and retention
    pub priority: Priority,

    /// Access count for usage tracking
    pub access_count: u64,

    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,

    /// Time-to-live in seconds (None = permanent)
    pub ttl: Option<u64>,

    /// Compression level (0-9)
    pub compression_level: u8,

    /// Custom attributes for extensibility
    pub custom_attributes: HashMap<String, serde_json::Value>,

    /// Relationships to other memories
    pub relationships: Vec<MemoryRelationship>,

    /// Provenance tracking
    pub provenance: ProvenanceInfo,

    /// Schema version for compatibility
    pub schema_version: Option<String>,

    /// Quality score (0.0-1.0)
    pub quality_score: f64,
}

/// Different types of memory payloads supported by GaussOS
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryPayload {
    /// Neural network parameters and weights
    Parametric {
        model_type: String,
        layer_weights: Vec<f32>,
        bias_terms: Option<Vec<f32>>,
        activation_function: String,
        metadata: HashMap<String, serde_json::Value>,
    },

    /// Neural activation patterns
    Activation {
        layer_name: String,
        activation_values: Vec<f32>,
        input_shape: Vec<usize>,
        timestamp: DateTime<Utc>,
        context: Option<String>,
    },

    /// Plain text content
    Text(String),

    /// Plaintext with additional metadata
    Plaintext {
        content: String,
        encoding: String,
        language: Option<String>,
        embeddings: Option<Vec<f32>>,
    },

    /// Semantic knowledge representation
    Semantic {
        content: String,
        schema_type: SemanticType,
        confidence: f32,
        extracted_at: DateTime<Utc>,
        source_context: String,
        embeddings: Option<Vec<f32>>,
        validation_metadata: Option<HashMap<String, serde_json::Value>>,
    },

    /// Episodic memories (conversations, events)
    Episodic {
        conversation_id: Uuid,
        thread_title: String,
        summary: String,
        participants: Vec<String>,
        key_insights: Vec<String>,
        success_metrics: HashMap<String, f32>,
        conversation_metadata: ConversationMetadata,
    },

    /// Procedural memories (prompts, workflows)
    Procedural {
        prompt_name: String,
        prompt_content: String,
        optimization_history: Vec<PromptOptimization>,
        performance_metrics: PromptMetrics,
        last_optimized: DateTime<Utc>,
        optimization_strategy: OptimizationStrategy,
    },
}

/// Memory namespace for organizing and accessing memories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MemoryNamespace(pub String);

impl Default for MemoryNamespace {
    fn default() -> Self {
        Self("default".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SemanticType {
    UserProfile,
    UserPreference,
    KnowledgeFact,
    DomainConcept,
    Relationship,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationMetadata {
    pub turn_count: u32,
    pub duration_seconds: Option<u32>,
    pub interaction_type: InteractionType,
    pub quality_score: Option<f32>,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionType {
    Question,
    Instruction,
    Conversation,
    Feedback,
    Collaboration,
    Learning,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptOptimization {
    pub timestamp: DateTime<Utc>,
    pub previous_version: String,
    pub changes_made: Vec<String>,
    pub reason: String,
    pub optimizer_type: OptimizationStrategy,
    pub performance_delta: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptMetrics {
    pub success_rate: f32,
    pub average_response_quality: f32,
    pub user_satisfaction: f32,
    pub execution_count: u64,
    pub last_evaluated: DateTime<Utc>,
    pub response_time_ms: f32,
    pub token_efficiency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationStrategy {
    Gradient,
    Metaprompt,
    PromptMemory,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Critical = 4,
    High = 3,
    Medium = 2,
    Normal = 1,
    Low = 0,
    Archive = -1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelationship {
    pub target_id: Uuid,
    pub relationship_type: RelationshipType,
    pub strength: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Parent,
    Child,
    Sibling,
    Dependency,
    Reference,
    Similar,
    ConversationThread,
    OptimizationChain,
    Merged,
    Snapshot,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    pub source: String,
    pub creator: String,
    pub creation_method: String,
    pub parent_memories: Vec<Uuid>,
    pub transformation_history: Vec<TransformationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationRecord {
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub executor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub id: String,
    pub payload: MemoryPayload,
    pub extraction_confidence: f32,
    pub extraction_method: String,
    pub source_messages: Vec<Uuid>,
}

impl MemCube {
    /// Create a new memory cube with the given payload
    pub fn new(payload: MemoryPayload) -> Self {
        Self::new_with_namespace(payload, MemoryNamespace::default())
    }

    /// Create a new memory cube with a specific namespace
    pub fn new_with_namespace(payload: MemoryPayload, namespace: MemoryNamespace) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            metadata: MemoryMetadata {
                name: None,
                description: None,
                tags: Vec::new(),
                priority: Priority::Normal,
                access_count: 0,
                last_accessed: now,
                ttl: None,
                compression_level: 0,
                custom_attributes: HashMap::new(),
                relationships: Vec::new(),
                provenance: ProvenanceInfo {
                    source: "system".to_string(),
                    creator: "system".to_string(),
                    creation_method: "direct".to_string(),
                    parent_memories: Vec::new(),
                    transformation_history: Vec::new(),
                },
                schema_version: Some("1.1.0".to_string()),
                quality_score: 0.0,
            },
            payload,
            namespace,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Get a compact fingerprint of the memory cube
    pub fn fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.version.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Increment access count and update last accessed time
    pub fn increment_access(&mut self) {
        self.metadata.access_count += 1;
        self.metadata.last_accessed = Utc::now();
    }

    /// Check if the memory has expired based on TTL
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.metadata.ttl {
            let age = Utc::now().timestamp() as u64 - self.created_at.timestamp() as u64;
            age > ttl
        } else {
            false
        }
    }

    /// Get namespace path as vector of strings
    pub fn get_namespace_path(&self) -> Vec<String> {
        self.namespace.0.split('/').map(|s| s.to_string()).collect()
    }

    /// Check if this is a semantic memory
    pub fn is_semantic_memory(&self) -> bool {
        matches!(self.payload, MemoryPayload::Semantic { .. })
    }

    /// Check if this is an episodic memory
    pub fn is_episodic_memory(&self) -> bool {
        matches!(self.payload, MemoryPayload::Episodic { .. })
    }

    /// Check if this is a procedural memory
    pub fn is_procedural_memory(&self) -> bool {
        matches!(self.payload, MemoryPayload::Procedural { .. })
    }

    /// Get a human-readable summary of the content
    pub fn get_content_summary(&self) -> String {
        match &self.payload {
            MemoryPayload::Parametric { model_type, .. } => format!("Model: {}", model_type),
            MemoryPayload::Activation { layer_name, .. } => format!("Activation: {}", layer_name),
            MemoryPayload::Text(content) => content.clone(),
            MemoryPayload::Plaintext { content, .. } => content.clone(),
            MemoryPayload::Semantic { content, .. } => content.clone(),
            MemoryPayload::Episodic { thread_title, .. } => thread_title.clone(),
            MemoryPayload::Procedural { prompt_name, .. } => format!("Prompt: {}", prompt_name),
        }
    }

    /// Returns a reference to embedding vector if present in payload
    pub fn payload_embedding(&self) -> Option<&Vec<f32>> {
        match &self.payload {
            MemoryPayload::Semantic {
                embeddings: Some(vec),
                ..
            } => Some(vec),
            MemoryPayload::Plaintext {
                embeddings: Some(vec),
                ..
            } => Some(vec),
            _ => None,
        }
    }
}

impl MemoryNamespace {
    pub fn new(path: Vec<String>) -> Self {
        Self(path.join("/"))
    }

    pub fn user_namespace(user_id: String, domain: Option<String>) -> Self {
        let mut path = vec!["users".to_string(), user_id.clone()];
        if let Some(d) = &domain {
            path.push(d.clone());
        }

        Self(path.join("/"))
    }

    pub fn public_namespace(domain: String) -> Self {
        Self(["public".to_string(), domain].join("/"))
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl MemoryPayload {
    /// Calculate the approximate size of the payload in bytes
    pub fn len(&self) -> usize {
        match self {
            MemoryPayload::Parametric {
                layer_weights,
                bias_terms,
                model_type,
                activation_function,
                metadata,
            } => {
                let weights_size = layer_weights.len() * std::mem::size_of::<f32>();
                let bias_size = bias_terms
                    .as_ref()
                    .map(|b| b.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0);
                let string_size = model_type.len() + activation_function.len();
                let metadata_size = serde_json::to_string(metadata).unwrap_or_default().len();
                weights_size + bias_size + string_size + metadata_size
            }
            MemoryPayload::Activation {
                activation_values,
                layer_name,
                input_shape,
                context,
                ..
            } => {
                let activation_size = activation_values.len() * std::mem::size_of::<f32>();
                let string_size = layer_name.len() + context.as_ref().map(|c| c.len()).unwrap_or(0);
                let shape_size = input_shape.len() * std::mem::size_of::<usize>();
                activation_size + string_size + shape_size
            }
            MemoryPayload::Text(content) => content.len(),
            MemoryPayload::Plaintext {
                content,
                encoding,
                language,
                embeddings,
            } => {
                let text_size = content.len() + encoding.len();
                let lang_size = language.as_ref().map(|l| l.len()).unwrap_or(0);
                let embed_size = embeddings
                    .as_ref()
                    .map(|e| e.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0);
                text_size + lang_size + embed_size
            }
            MemoryPayload::Semantic {
                content,
                embeddings,
                source_context,
                validation_metadata,
                ..
            } => {
                let content_size = content.len() + source_context.len();
                let embed_size = embeddings
                    .as_ref()
                    .map(|e| e.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0);
                let metadata_size = validation_metadata
                    .as_ref()
                    .map(|m| serde_json::to_string(m).unwrap_or_default().len())
                    .unwrap_or(0);
                content_size + embed_size + metadata_size
            }
            MemoryPayload::Episodic {
                thread_title,
                summary,
                participants,
                key_insights,
                conversation_metadata,
                ..
            } => {
                let string_size = thread_title.len() + summary.len();
                let participants_size = participants.iter().map(|p| p.len()).sum::<usize>();
                let insights_size = key_insights.iter().map(|i| i.len()).sum::<usize>();
                let metadata_size = std::mem::size_of_val(conversation_metadata);
                string_size + participants_size + insights_size + metadata_size
            }
            MemoryPayload::Procedural {
                prompt_name,
                prompt_content,
                optimization_history,
                performance_metrics,
                ..
            } => {
                let text_size = prompt_name.len() + prompt_content.len();
                let history_size = optimization_history.len() * 128; // Approximate size
                let metrics_size = std::mem::size_of_val(performance_metrics);
                text_size + history_size + metrics_size
            }
        }
    }
}
