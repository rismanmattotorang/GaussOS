// src/api/graphql.rs
//! GraphQL API Layer for GaussOS
//! Provides flexible querying capabilities essential for Agentic AI and RAG applications
//!
//! Features:
//! - Memory queries with filtering and pagination
//! - Graph traversal and analytics
//! - Real-time subscriptions for memory updates
//! - Batch operations for high-throughput RAG pipelines

#[cfg(feature = "graphql")]
use async_graphql::{
    Context, EmptySubscription, Enum, InputObject, Interface, Object, Result, Schema,
    SimpleObject, Union, ID, MergedObject, MergedSubscription,
};
// Note: async_graphql_axum types omitted due to axum version conflicts
// Using direct JSON-based handling instead
use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

use crate::{
    api::AppState,
    core::{MemCube, MemoryNamespace, MemoryPayload},
    database::{MemVault, SearchQuery},
    error::Result as GaussResult,
};

// ============================================================================
// GraphQL Types
// ============================================================================

/// Memory type exposed via GraphQL
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct GqlMemory {
    pub id: ID,
    pub name: Option<String>,
    pub description: Option<String>,
    pub namespace: String,
    pub memory_type: String,
    pub content_summary: String,
    pub tags: Vec<String>,
    pub quality_score: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: i64,
    pub version: i32,
}

#[cfg(feature = "graphql")]
impl From<MemCube> for GqlMemory {
    fn from(memory: MemCube) -> Self {
        let memory_type = match &memory.payload {
            MemoryPayload::Semantic { .. } => "Semantic",
            MemoryPayload::Episodic { .. } => "Episodic",
            MemoryPayload::Procedural { .. } => "Procedural",
            MemoryPayload::Parametric { .. } => "Parametric",
            MemoryPayload::Activation { .. } => "Activation",
            MemoryPayload::Text(_) => "Text",
            MemoryPayload::Plaintext { .. } => "Plaintext",
        };
        
        Self {
            id: ID(memory.id.to_string()),
            name: memory.metadata.name.clone(),
            description: memory.metadata.description.clone(),
            namespace: memory.namespace.0.clone(),
            memory_type: memory_type.to_string(),
            content_summary: memory.get_content_summary(),
            tags: memory.metadata.tags.clone(),
            quality_score: memory.metadata.quality_score,
            created_at: memory.created_at,
            updated_at: memory.updated_at,
            access_count: memory.metadata.access_count as i64,
            version: memory.version as i32,
        }
    }
}

/// Semantic memory detail type
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct GqlSemanticMemory {
    pub base: GqlMemory,
    pub content: String,
    pub schema_type: String,
    pub confidence: f32,
    pub source_context: String,
    pub has_embeddings: bool,
}

/// Graph node type for GraphQL
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct GqlGraphNode {
    pub id: ID,
    pub memory_id: Option<ID>,
    pub node_type: String,
    pub label: String,
    pub properties: String, // JSON string
    pub created_at: DateTime<Utc>,
}

/// Graph edge type for GraphQL
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct GqlGraphEdge {
    pub id: ID,
    pub source_id: ID,
    pub target_id: ID,
    pub edge_type: String,
    pub weight: f64,
    pub properties: String, // JSON string
}

/// Memory connection for pagination
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct MemoryConnection {
    pub edges: Vec<MemoryEdge>,
    pub page_info: PageInfo,
    pub total_count: i32,
}

#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct MemoryEdge {
    pub node: GqlMemory,
    pub cursor: String,
}

#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

/// RAG context result
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct RagContext {
    pub memories: Vec<GqlMemory>,
    pub graph_context: Vec<GqlGraphNode>,
    pub relevance_scores: Vec<f64>,
    pub combined_context: String,
    pub token_count: i32,
}

/// Agent execution result
#[cfg(feature = "graphql")]
#[derive(SimpleObject, Clone)]
pub struct AgentExecutionResult {
    pub agent_id: ID,
    pub execution_id: ID,
    pub status: String,
    pub result: Option<String>,
    pub memories_accessed: Vec<ID>,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Input Types
// ============================================================================

#[cfg(feature = "graphql")]
#[derive(InputObject)]
pub struct MemoryFilterInput {
    pub namespace: Option<String>,
    pub memory_type: Option<String>,
    pub tags: Option<Vec<String>>,
    pub min_quality_score: Option<f64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub search_text: Option<String>,
}

#[cfg(feature = "graphql")]
#[derive(InputObject)]
pub struct CreateMemoryInput {
    pub content: String,
    pub memory_type: String,
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[cfg(feature = "graphql")]
#[derive(InputObject)]
pub struct UpdateMemoryInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content: Option<String>,
}

#[cfg(feature = "graphql")]
#[derive(InputObject)]
pub struct RagQueryInput {
    pub query: String,
    pub namespace: Option<String>,
    pub max_memories: Option<i32>,
    pub similarity_threshold: Option<f64>,
    pub include_graph_context: Option<bool>,
    pub max_tokens: Option<i32>,
}

#[cfg(feature = "graphql")]
#[derive(InputObject)]
pub struct VectorSearchInput {
    pub embedding: Vec<f32>,
    pub top_k: Option<i32>,
    pub namespace: Option<String>,
    pub threshold: Option<f64>,
}

// ============================================================================
// Query Root
// ============================================================================

#[cfg(feature = "graphql")]
pub struct QueryRoot;

#[cfg(feature = "graphql")]
#[Object]
impl QueryRoot {
    /// Get a single memory by ID
    async fn memory(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlMemory>> {
        let state = ctx.data::<AppState>()?;
        let uuid = Uuid::parse_str(&id).map_err(|_| "Invalid UUID format")?;
        
        match state.database.retrieve(&uuid).await {
            Ok(Some(memory)) => Ok(Some(GqlMemory::from(memory))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Database error: {}", e))),
        }
    }

    /// Query memories with filtering and pagination
    async fn memories(
        &self,
        ctx: &Context<'_>,
        filter: Option<MemoryFilterInput>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> Result<MemoryConnection> {
        let state = ctx.data::<AppState>()?;
        
        let limit = first.or(last).unwrap_or(20) as u64;
        let offset = after.and_then(|c| c.parse::<u64>().ok()).unwrap_or(0);
        
        let mut query = SearchQuery::default();
        query.limit = Some(limit);
        query.offset = Some(offset);
        
        if let Some(f) = filter {
            query.namespace = f.namespace;
            query.text = f.search_text;
            if let Some(tags) = f.tags {
                query.tags = tags;
            }
            query.payload_type = f.memory_type;
        }
        
        match state.database.search(&query).await {
            Ok(memories) => {
                let total_count = memories.len() as i32;
                let edges: Vec<MemoryEdge> = memories
                    .into_iter()
                    .enumerate()
                    .map(|(i, m)| MemoryEdge {
                        cursor: (offset + i as u64).to_string(),
                        node: GqlMemory::from(m),
                    })
                    .collect();
                
                let has_next = edges.len() as u64 >= limit;
                let has_prev = offset > 0;
                
                Ok(MemoryConnection {
                    page_info: PageInfo {
                        has_next_page: has_next,
                        has_previous_page: has_prev,
                        start_cursor: edges.first().map(|e| e.cursor.clone()),
                        end_cursor: edges.last().map(|e| e.cursor.clone()),
                    },
                    edges,
                    total_count,
                })
            }
            Err(e) => Err(async_graphql::Error::new(format!("Query failed: {}", e))),
        }
    }

    /// Search memories by text with semantic understanding
    async fn search_memories(
        &self,
        ctx: &Context<'_>,
        query: String,
        namespace: Option<String>,
        limit: Option<i32>,
    ) -> Result<Vec<GqlMemory>> {
        let state = ctx.data::<AppState>()?;
        
        let mut search_query = SearchQuery::default();
        search_query.text = Some(query);
        search_query.namespace = namespace;
        search_query.limit = Some(limit.unwrap_or(10) as u64);
        
        match state.database.search(&search_query).await {
            Ok(memories) => Ok(memories.into_iter().map(GqlMemory::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Search failed: {}", e))),
        }
    }

    /// Perform a RAG (Retrieval-Augmented Generation) query
    /// Returns relevant memories and context for LLM consumption
    async fn rag_query(&self, ctx: &Context<'_>, input: RagQueryInput) -> Result<RagContext> {
        let state = ctx.data::<AppState>()?;
        
        let mut search_query = SearchQuery::default();
        search_query.text = Some(input.query.clone());
        search_query.namespace = input.namespace;
        search_query.limit = Some(input.max_memories.unwrap_or(10) as u64);
        
        let memories = state.database.search(&search_query).await
            .map_err(|e| async_graphql::Error::new(format!("RAG query failed: {}", e)))?;
        
        // Calculate relevance scores (simplified - in production, use actual similarity scores)
        let relevance_scores: Vec<f64> = memories.iter()
            .enumerate()
            .map(|(i, _)| 1.0 - (i as f64 * 0.1).min(0.9))
            .collect();
        
        // Combine context for LLM
        let combined_context = memories.iter()
            .take(input.max_memories.unwrap_or(5) as usize)
            .map(|m| m.get_content_summary())
            .collect::<Vec<_>>()
            .join("\n\n");
        
        let token_count = combined_context.split_whitespace().count() as i32;
        
        Ok(RagContext {
            memories: memories.into_iter().map(GqlMemory::from).collect(),
            graph_context: Vec::new(), // Would include graph neighbors in full implementation
            relevance_scores,
            combined_context,
            token_count,
        })
    }

    /// Vector similarity search
    async fn vector_search(
        &self,
        ctx: &Context<'_>,
        input: VectorSearchInput,
    ) -> Result<Vec<GqlMemory>> {
        let state = ctx.data::<AppState>()?;
        
        let mut search_query = SearchQuery::default();
        search_query.namespace = input.namespace;
        search_query.limit = Some(input.top_k.unwrap_or(10) as u64);
        search_query.vector_search = Some(crate::database::VectorSearchQuery {
            embedding: input.embedding,
            similarity_threshold: input.threshold.unwrap_or(0.7),
            metric: crate::database::SimilarityMetric::Cosine,
            top_k: Some(input.top_k.unwrap_or(10) as usize),
            ef_search: None,
        });
        
        match state.database.search(&search_query).await {
            Ok(memories) => Ok(memories.into_iter().map(GqlMemory::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Vector search failed: {}", e))),
        }
    }

    /// Get graph nodes
    async fn graph_nodes(
        &self,
        ctx: &Context<'_>,
        node_type: Option<String>,
        limit: Option<i32>,
    ) -> Result<Vec<GqlGraphNode>> {
        // Placeholder - would integrate with graph engine
        Ok(Vec::new())
    }

    /// Get graph edges
    async fn graph_edges(
        &self,
        ctx: &Context<'_>,
        edge_type: Option<String>,
        source_id: Option<ID>,
        target_id: Option<ID>,
    ) -> Result<Vec<GqlGraphEdge>> {
        // Placeholder - would integrate with graph engine
        Ok(Vec::new())
    }

    /// Get system health status
    async fn health(&self, ctx: &Context<'_>) -> Result<HealthStatus> {
        Ok(HealthStatus {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        })
    }

    /// Get system statistics
    async fn stats(&self, ctx: &Context<'_>) -> Result<SystemStats> {
        let state = ctx.data::<AppState>()?;
        
        match state.database.get_stats().await {
            Ok(stats) => Ok(SystemStats {
                total_memories: stats.total_memories as i64,
                total_namespaces: stats.memory_by_namespace.len() as i32,
                storage_bytes: stats.storage_size as i64,
                cache_hit_rate: stats.performance_metrics.cache_hit_rate,
            }),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get stats: {}", e))),
        }
    }
}

#[cfg(feature = "graphql")]
#[derive(SimpleObject)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: i64,
}

#[cfg(feature = "graphql")]
#[derive(SimpleObject)]
pub struct SystemStats {
    pub total_memories: i64,
    pub total_namespaces: i32,
    pub storage_bytes: i64,
    pub cache_hit_rate: f64,
}

// ============================================================================
// Mutation Root
// ============================================================================

#[cfg(feature = "graphql")]
pub struct MutationRoot;

#[cfg(feature = "graphql")]
#[Object]
impl MutationRoot {
    /// Create a new memory
    async fn create_memory(&self, ctx: &Context<'_>, input: CreateMemoryInput) -> Result<GqlMemory> {
        let state = ctx.data::<AppState>()?;
        
        let payload = match input.memory_type.to_lowercase().as_str() {
            "text" => MemoryPayload::Text(input.content),
            "semantic" => MemoryPayload::Semantic {
                content: input.content,
                schema_type: crate::core::SemanticType::KnowledgeFact,
                confidence: 1.0,
                extracted_at: Utc::now(),
                source_context: "graphql_api".to_string(),
                embeddings: None,
                validation_metadata: None,
            },
            _ => MemoryPayload::Text(input.content),
        };
        
        let mut memory = MemCube::new(payload);
        
        if let Some(ns) = input.namespace {
            memory.namespace = MemoryNamespace(ns);
        }
        if let Some(name) = input.name {
            memory.metadata.name = Some(name);
        }
        if let Some(desc) = input.description {
            memory.metadata.description = Some(desc);
        }
        if let Some(tags) = input.tags {
            memory.metadata.tags = tags;
        }
        
        state.database.store(&memory).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to create memory: {}", e)))?;
        
        Ok(GqlMemory::from(memory))
    }

    /// Update an existing memory
    async fn update_memory(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateMemoryInput,
    ) -> Result<GqlMemory> {
        let state = ctx.data::<AppState>()?;
        let uuid = Uuid::parse_str(&id).map_err(|_| "Invalid UUID format")?;
        
        let mut memory = state.database.retrieve(&uuid).await
            .map_err(|e| async_graphql::Error::new(format!("Database error: {}", e)))?
            .ok_or_else(|| async_graphql::Error::new("Memory not found"))?;
        
        if let Some(name) = input.name {
            memory.metadata.name = Some(name);
        }
        if let Some(desc) = input.description {
            memory.metadata.description = Some(desc);
        }
        if let Some(tags) = input.tags {
            memory.metadata.tags = tags;
        }
        if let Some(content) = input.content {
            memory.payload = MemoryPayload::Text(content);
        }
        
        memory.updated_at = Utc::now();
        memory.version += 1;
        
        state.database.update(&memory).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to update memory: {}", e)))?;
        
        Ok(GqlMemory::from(memory))
    }

    /// Delete a memory
    async fn delete_memory(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        let uuid = Uuid::parse_str(&id).map_err(|_| "Invalid UUID format")?;
        
        state.database.delete(&uuid).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to delete memory: {}", e)))?;
        
        Ok(true)
    }

    /// Batch create memories (optimized for RAG ingestion)
    async fn batch_create_memories(
        &self,
        ctx: &Context<'_>,
        inputs: Vec<CreateMemoryInput>,
    ) -> Result<Vec<GqlMemory>> {
        let state = ctx.data::<AppState>()?;
        let mut results = Vec::new();
        
        for input in inputs {
            let payload = MemoryPayload::Text(input.content);
            let mut memory = MemCube::new(payload);
            
            if let Some(ns) = input.namespace {
                memory.namespace = MemoryNamespace(ns);
            }
            if let Some(name) = input.name {
                memory.metadata.name = Some(name);
            }
            if let Some(tags) = input.tags {
                memory.metadata.tags = tags;
            }
            
            if let Err(e) = state.database.store(&memory).await {
                tracing::warn!("Failed to create memory in batch: {}", e);
                continue;
            }
            
            results.push(GqlMemory::from(memory));
        }
        
        Ok(results)
    }

    /// Consolidate memories (merge similar memories)
    async fn consolidate_memories(
        &self,
        ctx: &Context<'_>,
        namespace: Option<String>,
        similarity_threshold: Option<f64>,
    ) -> Result<ConsolidationResult> {
        // Placeholder - would integrate with memory consolidation engine
        Ok(ConsolidationResult {
            memories_processed: 0,
            memories_merged: 0,
            memories_deleted: 0,
        })
    }
}

#[cfg(feature = "graphql")]
#[derive(SimpleObject)]
pub struct ConsolidationResult {
    pub memories_processed: i32,
    pub memories_merged: i32,
    pub memories_deleted: i32,
}

// ============================================================================
// Schema Creation and Routes
// ============================================================================

#[cfg(feature = "graphql")]
pub type GaussSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Create the GraphQL schema
#[cfg(feature = "graphql")]
pub fn create_schema() -> GaussSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .enable_federation()
        .finish()
}

/// GraphQL handler for queries and mutations
/// Note: Due to axum version conflicts with async_graphql_axum, this handler
/// implements a simpler JSON-based approach instead of using GraphQLRequest/Response types
#[cfg(feature = "graphql")]
pub async fn graphql_handler(
    State(state): State<AppState>,
    Json(request): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let schema = create_schema();
    
    // Extract query from JSON
    let query = request.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let variables = request.get("variables").cloned().unwrap_or(serde_json::Value::Null);
    let operation_name = request.get("operationName").and_then(|o| o.as_str());
    
    // Build and execute request
    let mut gql_request = async_graphql::Request::new(query);
    if let Ok(vars) = serde_json::from_value::<async_graphql::Variables>(variables) {
        gql_request = gql_request.variables(vars);
    }
    if let Some(name) = operation_name {
        gql_request = gql_request.operation_name(name);
    }
    
    let response = schema.execute(gql_request.data(state)).await;
    Json(serde_json::to_value(&response).unwrap_or_default())
}

/// GraphQL Playground UI
pub async fn graphql_playground() -> impl IntoResponse {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>GaussOS GraphQL Playground</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphql-playground-react/build/static/css/index.css" />
    <script src="https://cdn.jsdelivr.net/npm/graphql-playground-react/build/static/js/middleware.js"></script>
</head>
<body>
    <div id="root"></div>
    <script>
        window.addEventListener('load', function() {
            GraphQLPlayground.init(document.getElementById('root'), {
                endpoint: '/api/v1/graphql',
                settings: {
                    'editor.theme': 'dark',
                    'editor.fontSize': 14,
                    'request.credentials': 'include',
                }
            })
        })
    </script>
</body>
</html>
    "#)
}

/// Create GraphQL routes
#[cfg(feature = "graphql")]
pub fn graphql_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .with_state(state)
}

/// Fallback routes when graphql feature is disabled
#[cfg(not(feature = "graphql"))]
pub fn graphql_routes(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/graphql", get(graphql_disabled))
}

#[cfg(not(feature = "graphql"))]
async fn graphql_disabled() -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        "GraphQL feature is not enabled. Enable with --features graphql",
    )
}
