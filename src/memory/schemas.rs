// src/memory/schemas.rs
use crate::{
    core::{
        ConversationMetadata, ExtractedMemory, InteractionType, MemoryPayload, Message,
        OptimizationStrategy, PromptMetrics, SemanticType,
    },
    error::{GaussOSError, Result},
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for memory schema validation and extraction
#[async_trait]
pub trait MemorySchema: Send + Sync {
    /// Schema name for identification
    fn name(&self) -> &str;

    /// Schema version for compatibility
    fn version(&self) -> &str;

    /// Validate a memory payload against this schema
    fn validate(&self, payload: &MemoryPayload) -> Result<()>;

    /// Extract structured memory from conversation messages
    async fn extract_from_conversation(
        &self,
        messages: &[Message],
        context: Option<&ExtractionContext>,
    ) -> Result<Vec<ExtractedMemory>>;

    /// Get schema-specific validation rules
    fn get_validation_rules(&self) -> ValidationRules;
}

/// Context for memory extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionContext {
    pub user_id: Option<String>,
    pub conversation_id: Uuid,
    pub domain: Option<String>,
    pub existing_memories: Vec<Uuid>,
    pub extraction_mode: ExtractionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionMode {
    Incremental, // Extract only new information
    Complete,    // Extract all relevant information
    UpdateOnly,  // Only update existing memories
}

/// Validation rules for memory schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    pub required_fields: Vec<String>,
    pub field_constraints: HashMap<String, FieldConstraint>,
    pub content_validation: ContentValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldConstraint {
    MinLength(usize),
    MaxLength(usize),
    Range(f32, f32),
    OneOf(Vec<String>),
    Pattern(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentValidation {
    pub min_confidence: f32,
    pub require_source_context: bool,
    pub max_extraction_age_hours: Option<u32>,
}

/// User profile memory schema
pub struct UserProfileSchema;

#[async_trait]
impl MemorySchema for UserProfileSchema {
    fn name(&self) -> &str {
        "user_profile"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn validate(&self, payload: &MemoryPayload) -> Result<()> {
        match payload {
            MemoryPayload::Semantic {
                schema_type: SemanticType::UserProfile | SemanticType::UserPreference,
                content,
                confidence,
                ..
            } => {
                if content.is_empty() {
                    return Err(GaussOSError::ValidationError(
                        "User profile content cannot be empty".to_string(),
                    ));
                }
                if *confidence < 0.3 {
                    return Err(GaussOSError::ValidationError(
                        "User profile confidence too low".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(GaussOSError::ValidationError(
                "Invalid payload type for user profile schema".to_string(),
            )),
        }
    }

    async fn extract_from_conversation(
        &self,
        messages: &[Message],
        context: Option<&ExtractionContext>,
    ) -> Result<Vec<ExtractedMemory>> {
        let mut extracted = Vec::new();

        // Simple pattern-based extraction for demo
        // In real implementation, this would use an LLM
        for (i, message) in messages.iter().enumerate() {
            if message.content.to_lowercase().contains("i prefer")
                || message.content.to_lowercase().contains("i like")
                || message.content.to_lowercase().contains("my name is")
            {
                let payload = MemoryPayload::Semantic {
                    content: message.content.clone(),
                    schema_type: SemanticType::UserPreference,
                    confidence: 0.8,
                    extracted_at: Utc::now(),
                    source_context: format!("Message {} in conversation", i + 1),
                    embeddings: None,
                    validation_metadata: Some({
                        let mut metadata = HashMap::new();
                        metadata.insert(
                            "extraction_method".to_string(),
                            serde_json::Value::String("pattern_based".to_string()),
                        );
                        metadata
                    }),
                };

                extracted.push(ExtractedMemory {
                    id: Uuid::new_v4().to_string(),
                    payload,
                    extraction_confidence: 0.8,
                    extraction_method: "user_profile_schema".to_string(),
                    source_messages: vec![Uuid::new_v4()], // Would be actual message IDs
                });
            }
        }

        Ok(extracted)
    }

    fn get_validation_rules(&self) -> ValidationRules {
        let mut field_constraints = HashMap::new();
        field_constraints.insert("content".to_string(), FieldConstraint::MinLength(5));
        field_constraints.insert("confidence".to_string(), FieldConstraint::Range(0.0, 1.0));

        ValidationRules {
            required_fields: vec!["content".to_string(), "confidence".to_string()],
            field_constraints,
            content_validation: ContentValidation {
                min_confidence: 0.3,
                require_source_context: true,
                max_extraction_age_hours: Some(24),
            },
        }
    }
}

/// Conversation summary schema
pub struct ConversationSummarySchema;

#[async_trait]
impl MemorySchema for ConversationSummarySchema {
    fn name(&self) -> &str {
        "conversation_summary"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn validate(&self, payload: &MemoryPayload) -> Result<()> {
        match payload {
            MemoryPayload::Episodic {
                summary,
                participants,
                conversation_metadata,
                ..
            } => {
                if summary.is_empty() {
                    return Err(GaussOSError::ValidationError(
                        "Conversation summary cannot be empty".to_string(),
                    ));
                }
                if participants.is_empty() {
                    return Err(GaussOSError::ValidationError(
                        "Conversation must have at least one participant".to_string(),
                    ));
                }
                if conversation_metadata.turn_count == 0 {
                    return Err(GaussOSError::ValidationError(
                        "Conversation must have at least one turn".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(GaussOSError::ValidationError(
                "Invalid payload type for conversation summary schema".to_string(),
            )),
        }
    }

    async fn extract_from_conversation(
        &self,
        messages: &[Message],
        _context: Option<&ExtractionContext>,
    ) -> Result<Vec<ExtractedMemory>> {
        if messages.is_empty() {
            return Ok(Vec::new());
        }

        // Extract conversation summary
        let participants: Vec<String> = messages
            .iter()
            .map(|m| format!("{:?}", m.role))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let turn_count = messages.len() as u32;

        // Simple summarization (would use LLM in real implementation)
        let summary = if messages.len() > 10 {
            format!(
                "Long conversation with {} turns covering various topics",
                turn_count
            )
        } else {
            format!("Short conversation with {} turns", turn_count)
        };

        // Determine interaction type based on content
        let interaction_type = if messages.iter().any(|m| m.content.contains("?")) {
            InteractionType::Question
        } else if messages.iter().any(|m| {
            m.content.to_lowercase().contains("please")
                || m.content.to_lowercase().contains("can you")
        }) {
            InteractionType::Instruction
        } else {
            InteractionType::Conversation
        };

        let payload = MemoryPayload::Episodic {
            conversation_id: Uuid::new_v4(),
            thread_title: "Extracted Conversation".to_string(),
            summary,
            participants,
            key_insights: extract_key_insights(messages),
            success_metrics: calculate_success_metrics(messages),
            conversation_metadata: ConversationMetadata {
                turn_count,
                duration_seconds: None,
                interaction_type,
                quality_score: Some(0.75), // Default quality score
                topics: extract_topics(messages),
            },
        };

        Ok(vec![ExtractedMemory {
            id: Uuid::new_v4().to_string(),
            payload,
            extraction_confidence: 0.85,
            extraction_method: "conversation_summary_schema".to_string(),
            source_messages: vec![Uuid::new_v4()], // Would be actual message IDs
        }])
    }

    fn get_validation_rules(&self) -> ValidationRules {
        let mut field_constraints = HashMap::new();
        field_constraints.insert("summary".to_string(), FieldConstraint::MinLength(10));
        field_constraints.insert(
            "turn_count".to_string(),
            FieldConstraint::Range(1.0, 1000.0),
        );

        ValidationRules {
            required_fields: vec!["summary".to_string(), "participants".to_string()],
            field_constraints,
            content_validation: ContentValidation {
                min_confidence: 0.5,
                require_source_context: false,
                max_extraction_age_hours: None,
            },
        }
    }
}

/// Prompt optimization schema
pub struct PromptOptimizationSchema;

#[async_trait]
impl MemorySchema for PromptOptimizationSchema {
    fn name(&self) -> &str {
        "prompt_optimization"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn validate(&self, payload: &MemoryPayload) -> Result<()> {
        match payload {
            MemoryPayload::Procedural {
                prompt_name,
                prompt_content,
                performance_metrics,
                ..
            } => {
                if prompt_name.is_empty() {
                    return Err(GaussOSError::ValidationError(
                        "Prompt name cannot be empty".to_string(),
                    ));
                }
                if prompt_content.is_empty() {
                    return Err(GaussOSError::ValidationError(
                        "Prompt content cannot be empty".to_string(),
                    ));
                }
                if performance_metrics.execution_count == 0 {
                    return Err(GaussOSError::ValidationError(
                        "Prompt must have been executed at least once".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(GaussOSError::ValidationError(
                "Invalid payload type for prompt optimization schema".to_string(),
            )),
        }
    }

    async fn extract_from_conversation(
        &self,
        messages: &[Message],
        _context: Option<&ExtractionContext>,
    ) -> Result<Vec<ExtractedMemory>> {
        // Extract prompts and their performance from conversation
        // This is a simplified implementation
        let mut extracted = Vec::new();

        for (i, message) in messages.iter().enumerate() {
            if message.content.to_lowercase().contains("prompt")
                || message.content.to_lowercase().contains("instruction")
            {
                let payload = MemoryPayload::Procedural {
                    prompt_name: format!("Extracted Prompt {}", i + 1),
                    prompt_content: message.content.clone(),
                    optimization_history: Vec::new(),
                    performance_metrics: PromptMetrics {
                        success_rate: 0.75,
                        average_response_quality: 0.8,
                        user_satisfaction: 0.7,
                        execution_count: 1,
                        last_evaluated: Utc::now(),
                        response_time_ms: 1500.0,
                        token_efficiency: 0.85,
                    },
                    last_optimized: Utc::now(),
                    optimization_strategy: OptimizationStrategy::Gradient,
                };

                extracted.push(ExtractedMemory {
                    id: Uuid::new_v4().to_string(),
                    payload,
                    extraction_confidence: 0.7,
                    extraction_method: "prompt_optimization_schema".to_string(),
                    source_messages: vec![Uuid::new_v4()],
                });
            }
        }

        Ok(extracted)
    }

    fn get_validation_rules(&self) -> ValidationRules {
        let mut field_constraints = HashMap::new();
        field_constraints.insert("prompt_name".to_string(), FieldConstraint::MinLength(1));
        field_constraints.insert("prompt_content".to_string(), FieldConstraint::MinLength(10));

        ValidationRules {
            required_fields: vec!["prompt_name".to_string(), "prompt_content".to_string()],
            field_constraints,
            content_validation: ContentValidation {
                min_confidence: 0.6,
                require_source_context: true,
                max_extraction_age_hours: Some(48),
            },
        }
    }
}

/// Schema registry for managing all memory schemas
pub struct SchemaRegistry {
    schemas: HashMap<String, Box<dyn MemorySchema>>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            schemas: HashMap::new(),
        };

        // Register built-in schemas
        registry.register(Box::new(UserProfileSchema));
        registry.register(Box::new(ConversationSummarySchema));
        registry.register(Box::new(PromptOptimizationSchema));

        registry
    }

    pub fn register(&mut self, schema: Box<dyn MemorySchema>) {
        self.schemas.insert(schema.name().to_string(), schema);
    }

    pub fn get_schema(&self, name: &str) -> Option<&dyn MemorySchema> {
        self.schemas.get(name).map(|s| s.as_ref())
    }

    pub fn list_schemas(&self) -> Vec<&str> {
        self.schemas.keys().map(|s| s.as_str()).collect()
    }

    pub async fn extract_all_memories(
        &self,
        messages: &[Message],
        context: Option<&ExtractionContext>,
    ) -> Result<Vec<ExtractedMemory>> {
        let mut all_extracted = Vec::new();

        for schema in self.schemas.values() {
            let extracted = schema.extract_from_conversation(messages, context).await?;
            all_extracted.extend(extracted);
        }

        Ok(all_extracted)
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for conversation analysis

fn extract_key_insights(messages: &[Message]) -> Vec<String> {
    let mut insights = Vec::new();

    for message in messages {
        if message.content.to_lowercase().contains("important")
            || message.content.to_lowercase().contains("key")
            || message.content.to_lowercase().contains("critical")
        {
            insights.push(message.content.clone());
        }
    }

    if insights.is_empty() {
        insights.push("No specific insights identified".to_string());
    }

    insights
}

fn calculate_success_metrics(messages: &[Message]) -> HashMap<String, f32> {
    let mut metrics = HashMap::new();

    let total_messages = messages.len() as f32;
    let question_count = messages.iter().filter(|m| m.content.contains("?")).count() as f32;

    metrics.insert(
        "engagement_score".to_string(),
        if total_messages > 5.0 { 0.8 } else { 0.5 },
    );
    metrics.insert(
        "question_ratio".to_string(),
        if total_messages > 0.0 {
            question_count / total_messages
        } else {
            0.0
        },
    );
    metrics.insert("conversation_quality".to_string(), 0.75);

    metrics
}

fn extract_topics(messages: &[Message]) -> Vec<String> {
    let mut topics = Vec::new();

    // Simple topic extraction based on keywords
    let topic_keywords = [
        (
            "technology",
            &[
                "AI",
                "machine learning",
                "computer",
                "software",
                "programming",
            ],
        ),
        (
            "science",
            &["research", "study", "experiment", "data", "analysis"],
        ),
        (
            "business",
            &["project", "team", "management", "strategy", "goals"],
        ),
        ("personal", &["I", "my", "me", "personal", "myself"]),
    ];

    for message in messages {
        let content_lower = message.content.to_lowercase();
        for (topic, keywords) in &topic_keywords {
            if keywords
                .iter()
                .any(|&keyword| content_lower.contains(keyword)) && !topics.contains(&topic.to_string())
            {
                topics.push(topic.to_string());
            }
        }
    }

    if topics.is_empty() {
        topics.push("general".to_string());
    }

    topics
}
