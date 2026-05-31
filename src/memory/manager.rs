use crate::performance::metrics as perf;
use crate::{
    core::{ExtractedMemory, MemCube, MemoryNamespace, MemoryPayload, Message, Priority},
    database::{MemVault, SearchQuery},
    error::Result,
    memory::ann::{Distance, Hnsw, HnswConfig, Neighbor},
    memory::decay::{DecayConfig, ForgettingCurve, RetentionPlan},
    memory::graph_retrieval::{GraphHit, GraphRetriever, PprConfig},
    memory::retrieval::{HybridRetriever, HybridSearchConfig, RetrievalCandidate, ScoredMemory},
    memory::schemas::{ExtractionContext, ExtractionMode, SchemaRegistry},
    memory::scoring::{GaScored, GaWeights, GenerativeAgentScorer},
    memory::temporal::{IngestReport, TemporalFact, TemporalFactStore},
};
use crossbeam_queue::SegQueue;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio;
use uuid::Uuid;

pub mod cache;
pub mod consolidation;
pub mod namespace;

pub use consolidation::ConsolidationReport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExtractionRequest {
    pub messages: Vec<Message>,
    pub namespace: Option<MemoryNamespace>,
    pub existing_memories: Option<Vec<Uuid>>,
    pub extraction_mode: ExtractionMode,
    pub schemas: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExtractionResponse {
    pub extracted_memories: Vec<ExtractedMemory>,
    pub created_memory_ids: Vec<Uuid>,
    pub extraction_confidence: f32,
    pub processing_time_ms: u64,
}

/// Advanced memory management system with enterprise features and optimizations
pub struct MemoryManager {
    vault: Arc<dyn MemVault>,

    schema_registry: SchemaRegistry,
    namespaces: namespace::Registry,

    // Tiered caching system
    l1_cache: Arc<cache::MemoryCache>, // Hot data (most recent/frequently accessed)
    l2_cache: Arc<cache::MemoryCache>, // Warm data (moderately accessed)
    l3_cache: Arc<dashmap::DashMap<Uuid, MemCube>>, // Cold data (rarely accessed)

    // Batch processing pipeline
    batch_processor: Arc<BatchProcessor>,

    // Memory consolidation engine
    consolidation_engine: Arc<consolidation::MemoryConsolidator>,

    // Forgetting-curve engine for retention decisions (sleep-time housekeeping)
    forgetting: ForgettingCurve,

    // Default hybrid-retrieval configuration
    retrieval_config: HybridSearchConfig,

    // Bi-temporal knowledge graph of extracted facts
    temporal: Arc<RwLock<TemporalFactStore>>,

    // HNSW approximate-nearest-neighbour index over memory embeddings
    vector_index: Arc<RwLock<Hnsw>>,

    // Performance tracking
    metrics: Arc<MemoryManagerMetrics>,
}

/// A request for hybrid (lexical + semantic) retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridQuery {
    /// Free-text query driving BM25 lexical ranking.
    pub text: String,
    /// Optional dense query embedding driving semantic ranking.
    pub embedding: Option<Vec<f32>>,
    /// Restrict the candidate set to a namespace.
    pub namespace: Option<MemoryNamespace>,
    /// Restrict the candidate set to memories carrying these tags.
    pub tags: Vec<String>,
    /// Restrict the candidate set to a payload type.
    pub payload_type: Option<String>,
    /// Minimum quality score for candidates `[0.0, 1.0]`.
    pub min_quality: Option<f64>,
    /// How many candidates to pull from the vault before re-ranking.
    pub candidate_pool: usize,
    /// How many ranked results to return.
    pub top_k: usize,
}

impl Default for HybridQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            embedding: None,
            namespace: None,
            tags: Vec::new(),
            payload_type: None,
            min_quality: None,
            candidate_pool: 200,
            top_k: 10,
        }
    }
}

/// A retrieved memory paired with the score breakdown that ranked it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMemory {
    pub memory: MemCube,
    pub score: ScoredMemory,
}

/// High-performance batch processor for memory operations
pub struct BatchProcessor {
    /// Pending operations queue
    pending_ops: Arc<SegQueue<MemoryOperation>>,

    /// Batch configuration
    batch_size: usize,
    batch_timeout: Duration,

    /// Processing metrics
    processed_batches: AtomicU64,
    avg_batch_size: AtomicU64,
}

impl BatchProcessor {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            pending_ops: Arc::new(SegQueue::new()),
            batch_size: config.max_batch_size,
            batch_timeout: Duration::from_millis(config.batch_timeout_ms),
            processed_batches: AtomicU64::new(0),
            avg_batch_size: AtomicU64::new(0),
        }
    }

    /// Add operation to batch queue
    pub fn enqueue(&self, operation: MemoryOperation) {
        self.pending_ops.push(operation);
    }

    /// Process batch operations
    pub async fn process_batch(&self, vault: &Arc<dyn MemVault>) -> Result<()> {
        let mut operations = Vec::new();
        let mut stores = Vec::new();
        let mut updates = Vec::new();
        let mut deletes = Vec::new();

        // Collect operations from queue
        while operations.len() < self.batch_size {
            if let Some(op) = self.pending_ops.pop() {
                operations.push(op);
            } else {
                break;
            }
        }

        if operations.is_empty() {
            return Ok(());
        }

        let ops_count = operations.len() as u64;

        // Categorize operations
        for op in operations {
            match op {
                MemoryOperation::Store(memory) => stores.push(memory),
                MemoryOperation::Update(memory) => updates.push(memory),
                MemoryOperation::Delete(id) => deletes.push(id),
                MemoryOperation::BulkStore(memories) => stores.extend(memories),
            }
        }

        // Execute batch operations
        if !stores.is_empty() {
            vault.batch_store(&stores).await?;
        }

        for memory in updates {
            vault.update(&memory).await?;
        }

        for id in deletes {
            vault.delete(&id).await?;
        }

        // Update metrics
        self.processed_batches.fetch_add(1, Ordering::Relaxed);
        let current_avg = self.avg_batch_size.load(Ordering::Relaxed);
        let new_avg = (current_avg + ops_count) / 2;
        self.avg_batch_size.store(new_avg, Ordering::Relaxed);

        Ok(())
    }
}

/// Memory operation for batch processing
#[derive(Debug, Clone)]
pub enum MemoryOperation {
    Store(MemCube),
    Update(MemCube),
    Delete(Uuid),
    BulkStore(Vec<MemCube>),
}

/// Enhanced performance metrics
#[derive(Debug)]
pub struct MemoryManagerMetrics {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub l1_cache_hits: AtomicU64,
    pub l2_cache_hits: AtomicU64,
    pub l3_cache_hits: AtomicU64,
    pub batch_operations: AtomicU64,
    pub avg_operation_time_us: AtomicU64,
    pub memory_usage_bytes: AtomicU64,
    pub consolidation_operations: AtomicU64,
}

impl MemoryManagerMetrics {
    pub fn new() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            l1_cache_hits: AtomicU64::new(0),
            l2_cache_hits: AtomicU64::new(0),
            l3_cache_hits: AtomicU64::new(0),
            batch_operations: AtomicU64::new(0),
            avg_operation_time_us: AtomicU64::new(0),
            memory_usage_bytes: AtomicU64::new(0),
            consolidation_operations: AtomicU64::new(0),
        }
    }
}

/// Configuration for memory manager optimization
#[derive(Debug, Clone)]
pub struct MemoryManagerConfig {
    pub l1_cache_size: usize,
    pub l2_cache_size: usize,
    pub l3_cache_size: usize,
    pub batch_config: BatchConfig,
    pub enable_compression: bool,
    pub enable_prefetching: bool,
    pub consolidation_interval_ms: u64,
    pub cache_eviction_policy: CacheEvictionPolicy,
    /// Default hybrid-retrieval tuning.
    pub retrieval: HybridSearchConfig,
    /// Forgetting-curve / retention tuning.
    pub decay: DecayConfig,
}

impl Default for MemoryManagerConfig {
    fn default() -> Self {
        Self {
            l1_cache_size: 1000,
            l2_cache_size: 10000,
            l3_cache_size: 100000,
            batch_config: BatchConfig::default(),
            enable_compression: true,
            enable_prefetching: false,
            consolidation_interval_ms: 300000, // 5 minutes
            cache_eviction_policy: CacheEvictionPolicy::LRU,
            retrieval: HybridSearchConfig::default(),
            decay: DecayConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CacheEvictionPolicy {
    LRU,
    LFU,
    FIFO,
    Random,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub batch_timeout_ms: u64,
    pub enable_parallel_processing: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            batch_timeout_ms: 100,
            enable_parallel_processing: true,
        }
    }
}

impl MemoryManager {
    pub fn new_optimized(vault: Arc<dyn MemVault>, config: MemoryManagerConfig) -> Self {
        let l1_cache = Arc::new(cache::MemoryCache::new(config.l1_cache_size));
        let l2_cache = Arc::new(cache::MemoryCache::new(config.l2_cache_size));
        let l3_cache = Arc::new(dashmap::DashMap::new());

        let consolidation_engine = Arc::new(consolidation::MemoryConsolidator::new(
            vault.clone(),
            consolidation::ConsolidationConfig::default(),
        ));

        let manager = Self {
            vault,
            schema_registry: SchemaRegistry::new(),
            namespaces: namespace::Registry::default(),
            l1_cache,
            l2_cache,
            l3_cache,
            batch_processor: Arc::new(BatchProcessor::new(config.batch_config)),
            consolidation_engine,
            forgetting: ForgettingCurve::new(config.decay),
            retrieval_config: config.retrieval,
            temporal: Arc::new(RwLock::new(TemporalFactStore::new())),
            vector_index: Arc::new(RwLock::new(Hnsw::new(Distance::Cosine, HnswConfig::default()))),
            metrics: Arc::new(MemoryManagerMetrics::new()),
        };

        // Start background tasks
        manager.start_background_tasks();

        manager
    }

    /// Start background tasks for maintenance
    fn start_background_tasks(&self) {
        let consolidation_engine = self.consolidation_engine.clone();
        let batch_processor = self.batch_processor.clone();
        let vault = self.vault.clone();

        // Start consolidation task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(300000)); // 5 minutes
            loop {
                interval.tick().await;
                if let Err(e) = consolidation_engine.consolidate().await {
                    tracing::error!("Consolidation error: {}", e);
                }
            }
        });

        // Start batch processing task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100)); // 100ms
            loop {
                interval.tick().await;
                if let Err(e) = batch_processor.process_batch(&vault).await {
                    tracing::error!("Batch processing error: {}", e);
                }
            }
        });
    }

    /// Optimized memory retrieval with tiered caching
    pub async fn get_memory_optimized(&self, id: &Uuid) -> Result<Option<MemCube>> {
        let start = std::time::Instant::now();

        // Check L1 cache first (hottest data)
        if let Some(memory) = self.l1_cache.get(id) {
            self.metrics.l1_cache_hits.fetch_add(1, Ordering::Relaxed);
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            self.record_operation_time(start);
            return Ok(Some(memory));
        }

        // Check L2 cache (warm data)
        if let Some(memory) = self.l2_cache.get(id) {
            self.metrics.l2_cache_hits.fetch_add(1, Ordering::Relaxed);
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            
            // Promote to L1 cache
            self.l1_cache.insert(*id, memory.clone());
            
            self.record_operation_time(start);
            return Ok(Some(memory));
        }

        // Check L3 cache (cold data)
        if let Some(memory) = self.l3_cache.get(id) {
            self.metrics.l3_cache_hits.fetch_add(1, Ordering::Relaxed);
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            
            // Promote to L2 cache
            self.l2_cache.insert(*id, memory.clone());
            
            self.record_operation_time(start);
            return Ok(Some(memory.clone()));
        }

        // Cache miss - fetch from vault
        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        
        if let Some(memory) = self.vault.retrieve(id).await? {
            // Store in L1 cache
            self.l1_cache.insert(*id, memory.clone());
            
            self.record_operation_time(start);
            Ok(Some(memory))
        } else {
            self.record_operation_time(start);
            Ok(None)
        }
    }

    /// Batch memory creation for improved throughput
    pub async fn create_memories_batch(&self, memories: Vec<MemCube>) -> Result<Vec<Uuid>> {
        let start = std::time::Instant::now();

        // Validate all memories first
        for memory in &memories {
            if memory.metadata.schema_version.is_some() {
                self.validate_memory_against_schema(memory).await?;
            }
        }

        // Store in vault as batch
        let ids: Vec<Uuid> = memories.iter().map(|m| m.id).collect();
        self.vault.batch_store(&memories).await?;

        // Index embeddings and add to caches.
        {
            let mut index = self.vector_index.write();
            for memory in &memories {
                if let Some(embedding) = memory.payload_embedding() {
                    index.insert(memory.id, embedding.clone());
                }
            }
        }
        for memory in memories {
            self.l1_cache.insert(memory.id, memory);
        }

        self.metrics
            .batch_operations
            .fetch_add(1, Ordering::Relaxed);
        self.record_operation_time(start);
        perf::incr("memory.batch_store_total");

        Ok(ids)
    }

    /// Memory consolidation for namespace optimization
    pub async fn consolidate_memories(
        &self,
        namespace: &MemoryNamespace,
    ) -> Result<ConsolidationReport> {
        let start = std::time::Instant::now();

        // Get all memories in namespace
        let memories = self.get_memories_by_namespace(&namespace.0).await?;
        let memories_count = memories.len();
        
        if memories_count < 2 {
            return Ok(ConsolidationReport {
                memories_processed: memories_count,
                memories_merged: 0,
                memories_removed: 0,
                storage_saved_bytes: 0,
                consolidation_time_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Group similar memories
        let mut groups = HashMap::new();
        for memory in memories {
            let key = self.get_memory_group_key(&memory);
            groups.entry(key).or_insert_with(Vec::new).push(memory);
        }

        let mut merged_count = 0usize;
        let mut removed_count = 0usize;
        let mut storage_saved = 0usize;

        // Consolidate each group
        for (_, group_memories) in groups {
            if group_memories.len() > 1 {
                let group_len = group_memories.len();
                let original_size: usize = group_memories.iter()
                    .map(|m| std::mem::size_of_val(m))
                    .sum();

                // Clone IDs before moving memories
                let ids_to_remove: Vec<_> = group_memories.iter().map(|m| m.id).collect();

                let consolidated = self.merge_similar_memories(group_memories).await?;
                
                // Store consolidated memory
                self.vault.store(&consolidated).await?;
                
                // Remove original memories
                for id in ids_to_remove {
                    self.vault.delete(&id).await?;
                    // Remove from caches
                    self.l1_cache.remove(&id);
                    self.l2_cache.remove(&id);
                    self.l3_cache.remove(&id);
                }

                let final_size = std::mem::size_of_val(&consolidated);
                storage_saved += original_size.saturating_sub(final_size);
                
                merged_count += 1;
                removed_count += group_len;
            }
        }

        self.metrics.consolidation_operations.fetch_add(1, Ordering::Relaxed);

        Ok(ConsolidationReport {
            memories_processed: memories_count,
            memories_merged: merged_count,
            memories_removed: removed_count,
            storage_saved_bytes: storage_saved as u64,
            consolidation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Get memory group key for consolidation
    fn get_memory_group_key(&self, memory: &MemCube) -> String {
        match &memory.payload {
            MemoryPayload::Semantic { schema_type, .. } => format!("semantic_{:?}", schema_type),
            MemoryPayload::Episodic { conversation_id, .. } => format!("episodic_{}", conversation_id),
            MemoryPayload::Procedural { prompt_name, .. } => format!("procedural_{}", prompt_name),
            MemoryPayload::Text(_) => "text".to_string(),
            MemoryPayload::Plaintext { .. } => "plaintext".to_string(),
            MemoryPayload::Parametric { model_type, .. } => format!("parametric_{}", model_type),
            MemoryPayload::Activation { layer_name, .. } => format!("activation_{}", layer_name),
        }
    }

    fn record_operation_time(&self, start: std::time::Instant) {
        let elapsed_us = start.elapsed().as_micros() as u64;

        // Update running average using atomic operations
        let current_avg = self.metrics.avg_operation_time_us.load(Ordering::Relaxed);
        let new_avg = (current_avg + elapsed_us) / 2;
        self.metrics
            .avg_operation_time_us
            .store(new_avg, Ordering::Relaxed);
    }

    // Enhanced memory creation with namespace support
    pub async fn create_memory(&self, mut memory: MemCube) -> Result<Uuid> {
        memory.id = Uuid::new_v4();
        let id = memory.id;

        // Validate memory against schema if specified
        if memory.metadata.schema_version.is_some() {
            self.validate_memory_against_schema(&memory).await?;
        }

        // Index the embedding for approximate-nearest-neighbour search.
        if let Some(embedding) = memory.payload_embedding() {
            self.vector_index.write().insert(id, embedding.clone());
        }

        // Store in vault
        self.vault.store(&memory).await?;

        // Add to L1 cache
        self.l1_cache.insert(id, memory);

        perf::incr("memory.store_total");

        Ok(id)
    }

    pub async fn create_memory_in_namespace(
        &self,
        mut memory: MemCube,
        namespace: &MemoryNamespace,
    ) -> Result<Uuid> {
        memory.namespace = namespace.clone();
        self.create_memory(memory).await
    }

    // LangMem-inspired memory extraction from conversations
    pub async fn extract_memories_from_conversation(
        &self,
        request: MemoryExtractionRequest,
    ) -> Result<MemoryExtractionResponse> {
        let start_time = std::time::Instant::now();

        // Create extraction context
        let context = ExtractionContext {
            user_id: None, // TODO: Extract user_id from namespace
            conversation_id: Uuid::new_v4(),
            domain: None, // TODO: Extract domain from namespace
            existing_memories: request.existing_memories.unwrap_or_default(),
            extraction_mode: request.extraction_mode,
        };

        // Extract memories using schemas
        let extracted_memories = if let Some(schema_names) = request.schemas {
            let mut all_extracted = Vec::new();
            for schema_name in schema_names {
                if let Some(schema) = self.schema_registry.get_schema(&schema_name) {
                    let extracted = schema
                        .extract_from_conversation(&request.messages, Some(&context))
                        .await?;
                    all_extracted.extend(extracted);
                }
            }
            all_extracted
        } else {
            self.schema_registry
                .extract_all_memories(&request.messages, Some(&context))
                .await?
        };

        // Create actual memory cubes from extracted memories
        let mut created_memory_ids = Vec::new();
        let mut total_confidence = 0.0;

        for extracted in &extracted_memories {
            let memory = MemCube::new_with_namespace(
                extracted.payload.clone(),
                request.namespace.clone().unwrap_or_default(),
            );

            let memory_id = self.create_memory(memory).await?;
            created_memory_ids.push(memory_id);
            total_confidence += extracted.extraction_confidence;
        }

        let avg_confidence = if extracted_memories.is_empty() {
            0.0
        } else {
            total_confidence / extracted_memories.len() as f32
        };

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(MemoryExtractionResponse {
            extracted_memories,
            created_memory_ids,
            extraction_confidence: avg_confidence,
            processing_time_ms: processing_time,
        })
    }

    // Get memories by namespace
    pub async fn get_memories_by_namespace(&self, namespace_path: &str) -> Result<Vec<MemCube>> {
        // Create a search query filtered by namespace
        let mut query = SearchQuery::default();
        query.filters.insert(
            "namespace_path".to_string(),
            serde_json::Value::String(namespace_path.to_string()),
        );

        self.vault.search(&query).await
    }

    // Optimize memory organization
    pub async fn optimize_memory_organization(&self) -> Result<()> {
        // Get all namespaces
        let namespaces: Vec<String> = self.namespaces.list();

        for namespace_path in namespaces {
            if let Some(namespace) = self.namespaces.get(&namespace_path) {
                let _consolidation_result = self.consolidate_memories(&namespace).await?;
            }
        }

        Ok(())
    }

    // Share memory across threads (for cross-conversation context)
    pub async fn share_memory_across_threads(
        &self,
        memory_id: &Uuid,
        target_threads: &[String],
    ) -> Result<()> {
        if let Some(original_memory) = self.get_memory(memory_id).await? {
            for thread_id in target_threads {
                // Create a reference memory in each target thread's namespace
                let mut shared_memory = original_memory.clone();
                shared_memory.id = Uuid::new_v4();

                // Add relationship to original
                shared_memory
                    .metadata
                    .relationships
                    .push(crate::core::MemoryRelationship {
                        target_id: *memory_id,
                        relationship_type: crate::core::RelationshipType::Reference,
                        strength: 1.0,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "shared_to_thread".to_string(),
                                serde_json::Value::String(thread_id.clone()),
                            );
                            meta
                        },
                    });

                self.vault.store(&shared_memory).await?;
            }
        }

        Ok(())
    }

    // Consolidate memories from multiple threads
    pub async fn consolidate_thread_memories(
        &self,
        thread_ids: &[String],
        namespace: &MemoryNamespace,
    ) -> Result<ConsolidationReport> {
        let mut all_memories = Vec::new();
        
        for thread_id in thread_ids {
            let thread_namespace = MemoryNamespace(format!("{}/{}", namespace.0, thread_id));
            let memories = self.get_memories_by_namespace(&thread_namespace.0).await?;
            all_memories.extend(memories);
        }

        let all_memories_count = all_memories.len();

        if all_memories_count < 2 {
            return Ok(ConsolidationReport {
                memories_processed: all_memories_count,
                memories_merged: 0,
                memories_removed: 0,
                storage_saved_bytes: 0,
                consolidation_time_ms: 0,
            });
        }

        // Group by similarity and consolidate
        let mut groups = HashMap::new();
        for memory in all_memories {
            let key = self.get_memory_group_key(&memory);
            groups.entry(key).or_insert_with(Vec::new).push(memory);
        }

        let mut merged_count = 0usize;
        let mut removed_count = 0usize;
        let storage_saved = 0usize;

        for (_, group_memories) in groups {
            if group_memories.len() > 1 {
                let group_len = group_memories.len();
                let ids_to_remove: Vec<_> = group_memories.iter().map(|m| m.id).collect();

                let consolidated = self.merge_similar_memories(group_memories).await?;
                self.vault.store(&consolidated).await?;
                
                for id in ids_to_remove {
                    self.vault.delete(&id).await?;
                }
                
                merged_count += 1;
                removed_count += group_len;
            }
        }

        Ok(ConsolidationReport {
            memories_processed: all_memories_count,
            memories_merged: merged_count,
            memories_removed: removed_count,
            storage_saved_bytes: storage_saved as u64,
            consolidation_time_ms: 0,
        })
    }

    /// Get memory by ID with namespace support
    pub async fn get_memory(&self, id: &Uuid) -> Result<Option<MemCube>> {
        self.get_memory_optimized(id).await
    }

    pub async fn update_memory(&self, memory: &MemCube) -> Result<()> {
        let mut updated_memory = memory.clone();
        updated_memory.updated_at = chrono::Utc::now();
        updated_memory.version += 1;

        // Update vault
        self.vault.update(&updated_memory).await?;
        
        // Update caches
        self.l1_cache.insert(memory.id, updated_memory.clone());
        self.l2_cache.insert(memory.id, updated_memory);
        
        perf::incr("memory.update_total");

        Ok(())
    }

    pub async fn delete_memory(&self, id: &Uuid) -> Result<()> {
        // Remove from vault
        self.vault.delete(id).await?;

        // Remove from caches
        self.l1_cache.remove(id);
        self.l2_cache.remove(id);
        self.l3_cache.remove(id);

        // Soft-delete from the ANN index so it stops appearing in results.
        self.vector_index.write().remove(id);

        perf::incr("memory.delete_total");

        Ok(())
    }

    pub async fn search_memories(&self, query: SearchQuery) -> Result<Vec<MemCube>> {
        self.vault.search(&query).await
    }

    pub async fn cleanup_expired(&self) -> Result<()> {
        // Clean up expired memories from caches
        self.l1_cache.cleanup_expired();
        self.l2_cache.cleanup_expired();
        
        // Clean up L3 cache (remove least recently used)
        let l3_size = self.l3_cache.len();
        if l3_size > 100000 { // If L3 cache is too large
            let to_remove: Vec<Uuid> = self.l3_cache
                .iter()
                .take(l3_size / 10) // Remove 10% of entries
                .map(|entry| *entry.key())
                .collect();
            
            for id in to_remove {
                self.l3_cache.remove(&id);
            }
        }

        Ok(())
    }

    // Helper methods for LangMem functionality

    async fn validate_memory_against_schema(&self, memory: &MemCube) -> Result<()> {
        // Extract schema name from memory metadata or payload type
        let schema_name = match &memory.payload {
            MemoryPayload::Semantic { .. } => "user_profile",
            MemoryPayload::Episodic { .. } => "conversation_summary",
            MemoryPayload::Procedural { .. } => "prompt_optimization",
            _ => return Ok(()), // No validation for legacy types
        };

        if let Some(schema) = self.schema_registry.get_schema(schema_name) {
            schema.validate(&memory.payload)?;
        }

        Ok(())
    }

    // Enhanced memory similarity comparison
    fn are_memories_similar(&self, memory1: &MemCube, memory2: &MemCube) -> bool {
        // Check if memories are of the same type
        if std::mem::discriminant(&memory1.payload) != std::mem::discriminant(&memory2.payload) {
            return false;
        }

        // Check namespace
        if memory1.namespace != memory2.namespace {
            return false;
        }

        // Check tags similarity
        let common_tags: Vec<&String> = memory1.metadata.tags
            .iter()
            .filter(|tag| memory2.metadata.tags.contains(tag))
            .collect();
        
        let similarity_ratio = common_tags.len() as f64 / 
            (memory1.metadata.tags.len() + memory2.metadata.tags.len()) as f64 * 2.0;
        
        similarity_ratio > 0.5 // 50% tag similarity threshold
    }

    // Enhanced memory merging algorithm
    async fn merge_similar_memories(&self, memories: Vec<MemCube>) -> Result<MemCube> {
        if memories.is_empty() {
            return Err(crate::error::GaussOSError::ValidationError(
                "Cannot merge empty memory list".to_string(),
            ));
        }

        if memories.len() == 1 {
            return Ok(memories[0].clone());
        }

        // Use the most recent memory as base
        let mut base_memory = memories.iter()
            .max_by_key(|m| m.updated_at)
            .unwrap()
            .clone();

        // Merge metadata
        let mut merged_tags = base_memory.metadata.tags.clone();
        let mut total_access_count = base_memory.metadata.access_count;
        let mut total_quality_score = base_memory.metadata.quality_score;

        for memory in &memories[1..] {
            // Merge tags
            for tag in &memory.metadata.tags {
                if !merged_tags.contains(tag) {
                    merged_tags.push(tag.clone());
                }
            }

            // Accumulate metrics
            total_access_count += memory.metadata.access_count;
            total_quality_score += memory.metadata.quality_score;
        }

        // Update base memory
        base_memory.metadata.tags = merged_tags;
        base_memory.metadata.access_count = total_access_count;
        base_memory.metadata.quality_score = total_quality_score / memories.len() as f64;
        base_memory.metadata.last_accessed = chrono::Utc::now();
        base_memory.updated_at = chrono::Utc::now();
        base_memory.version += 1;

        // Add relationships to merged memories
        for memory in &memories {
            if memory.id != base_memory.id {
                base_memory.metadata.relationships.push(crate::core::MemoryRelationship {
                    target_id: memory.id,
                    relationship_type: crate::core::RelationshipType::Merged,
                    strength: 1.0,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(base_memory)
    }

    // Namespace management
    pub fn register_namespace(&self, namespace: MemoryNamespace) {
        self.namespaces.register(namespace);
    }

    // Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            l1_cache_size: self.l1_cache.len(),
            l2_cache_size: self.l2_cache.len(),
            l3_cache_size: self.l3_cache.len(),
            l1_cache_hits: self.metrics.l1_cache_hits.load(Ordering::Relaxed),
            l2_cache_hits: self.metrics.l2_cache_hits.load(Ordering::Relaxed),
            l3_cache_hits: self.metrics.l3_cache_hits.load(Ordering::Relaxed),
            cache_misses: self.metrics.cache_misses.load(Ordering::Relaxed),
            total_operations: self.metrics.cache_hits.load(Ordering::Relaxed) + self.metrics.cache_misses.load(Ordering::Relaxed),
        }
    }
}

impl MemoryManager {
    /// Hybrid lexical + semantic retrieval over a namespace-scoped candidate
    /// pool, fused with Reciprocal Rank Fusion and re-ranked for diversity and
    /// recency. Returns each memory paired with its score breakdown.
    pub async fn hybrid_search(&self, query: &HybridQuery) -> Result<Vec<RankedMemory>> {
        let start = std::time::Instant::now();

        // 1. Pull a candidate pool from the vault, pushing every caller filter
        //    down so hybrid search honours the same constraints as plain search.
        let mut vault_query = SearchQuery::default();
        if let Some(ns) = &query.namespace {
            vault_query.namespace = Some(ns.0.clone());
            vault_query.include_child_namespaces = true;
        }
        if !query.text.is_empty() {
            vault_query.text = Some(query.text.clone());
        }
        if !query.tags.is_empty() {
            vault_query.tags = query.tags.clone();
        }
        if let Some(pt) = &query.payload_type {
            vault_query.payload_type = Some(pt.clone());
        }
        if let Some(min_q) = query.min_quality {
            vault_query.quality_range = Some(crate::database::QualityRange {
                min: Some(min_q),
                max: Some(1.0),
            });
        }
        // When an embedding is supplied, ask the vault for the nearest
        // neighbours so the candidate pool is similarity-aware rather than an
        // arbitrary slice — otherwise the true match may never be re-ranked.
        if let Some(embedding) = &query.embedding {
            vault_query.vector_search = Some(crate::database::VectorSearchQuery {
                embedding: embedding.clone(),
                similarity_threshold: 0.0,
                metric: crate::database::SimilarityMetric::Cosine,
                top_k: Some(query.candidate_pool),
                ef_search: None,
            });
        }
        vault_query.limit = Some(query.candidate_pool as u64);
        let candidates = self.vault.search(&vault_query).await?;

        // 2. Re-rank in-process with the hybrid engine. Build the candidate
        //    views by borrowing; keep an id -> index map so only the surviving
        //    top_k cubes are cloned, not the whole pool.
        let index: HashMap<Uuid, usize> =
            candidates.iter().enumerate().map(|(i, m)| (m.id, i)).collect();
        let retrieval_candidates: Vec<RetrievalCandidate> =
            candidates.iter().map(RetrievalCandidate::from_memcube).collect();

        let mut cfg = self.retrieval_config.clone();
        cfg.top_k = query.top_k;
        let retriever = HybridRetriever::new(retrieval_candidates, cfg);
        let scored = retriever.search(&query.text, query.embedding.as_deref());

        // 3. Join scores back to full memory cubes (clone only the winners).
        let ranked = scored
            .into_iter()
            .filter_map(|s| {
                index
                    .get(&s.id)
                    .map(|&i| RankedMemory { memory: candidates[i].clone(), score: s })
            })
            .collect();

        self.record_operation_time(start);
        perf::incr("memory.hybrid_search_total");
        Ok(ranked)
    }

    /// Run a forgetting-curve pass over a namespace and act on the result:
    /// archived memories are demoted out of the hot caches, and (optionally)
    /// memories below the forget threshold are deleted. This is the core of a
    /// "sleep-time" consolidation cycle.
    pub async fn run_forgetting_pass(
        &self,
        namespace: &MemoryNamespace,
        delete_forgotten: bool,
    ) -> Result<RetentionPlan> {
        let memories = self.get_memories_by_namespace(&namespace.0).await?;
        let plan = self.forgetting.classify(memories.iter());

        // Cool down archived memories: lower their priority and evict from hot tiers.
        for id in &plan.archive {
            if let Some(mut memory) = self.vault.retrieve(id).await? {
                memory.metadata.priority = Priority::Archive;
                self.vault.update(&memory).await?;
                self.l1_cache.remove(id);
                self.l2_cache.remove(id);
            }
        }

        if delete_forgotten {
            for id in &plan.forget {
                self.delete_memory(id).await?;
            }
        }

        perf::incr("memory.forgetting_pass_total");
        Ok(plan)
    }

    /// Ingest a fact into the bi-temporal knowledge graph, automatically
    /// superseding any conflicting live fact (kept for audit, not deleted).
    pub fn ingest_fact(&self, fact: TemporalFact) -> IngestReport {
        self.temporal.write().ingest(fact)
    }

    /// Run the LLM-driven extract → update ingestion pipeline over a set of
    /// conversation messages, forming durable bi-temporal facts. Uses the
    /// configured LLM provider when available, else a deterministic heuristic
    /// extractor (so it always works offline).
    pub async fn ingest_conversation(
        &self,
        messages: &[Message],
    ) -> crate::memory::ingest::IngestReport {
        let ingestor = crate::memory::ingest::MemoryIngestor::default();
        let llm = crate::agents::llm::LlmClient::from_env();
        // Extract first (may await the LLM) WITHOUT holding the store lock, then
        // apply synchronously under the lock so the future stays Send.
        let facts = ingestor.extract(messages, &llm).await;
        let mut store = self.temporal.write();
        ingestor.apply_all(&mut store, facts)
    }

    /// Detect communities over the current entity graph (GraphRAG-style).
    pub fn detect_communities(&self) -> Vec<crate::memory::community::Community> {
        crate::memory::community::detect_communities(
            &self.temporal.read(),
            &crate::memory::community::CommunityConfig::default(),
        )
    }

    /// All facts the system currently believes about a subject.
    pub fn current_facts_about(&self, subject: &str) -> Vec<TemporalFact> {
        self.temporal
            .read()
            .facts_about(subject)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Full, ordered history of a `(subject, predicate)` attribute including
    /// superseded records — the audit trail.
    pub fn fact_history(&self, subject: &str, predicate: &str) -> Vec<TemporalFact> {
        self.temporal
            .read()
            .history(subject, predicate)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Number of facts tracked in the temporal store.
    pub fn fact_count(&self) -> usize {
        self.temporal.read().len()
    }

    /// Approximate-nearest-neighbour search over indexed embeddings using the
    /// HNSW graph (sublinear, vs the brute-force candidate re-rank in
    /// [`Self::hybrid_search`]). Returns ids with similarity scores.
    pub fn ann_search(&self, query: &[f32], k: usize) -> Vec<Neighbor> {
        self.vector_index.read().search(query, k)
    }

    /// Resolve an ANN search to full memory cubes (cache-first, vault fallback).
    pub async fn ann_search_memories(&self, query: &[f32], k: usize) -> Result<Vec<MemCube>> {
        let neighbors = self.ann_search(query, k);
        let mut out = Vec::with_capacity(neighbors.len());
        for n in neighbors {
            if let Some(m) = self.get_memory(&n.id).await? {
                out.push(m);
            }
        }
        Ok(out)
    }

    /// Number of vectors currently held in the ANN index.
    pub fn vector_index_len(&self) -> usize {
        self.vector_index.read().len()
    }

    /// Serialize the ANN index to a portable byte buffer (for snapshotting /
    /// warm restarts without re-embedding every memory).
    pub fn export_vector_index(&self) -> Vec<u8> {
        self.vector_index.read().to_bytes()
    }

    /// Replace the ANN index from a buffer produced by [`Self::export_vector_index`].
    pub fn import_vector_index(&self, bytes: &[u8]) -> Result<()> {
        let restored = Hnsw::from_bytes(bytes)?;
        *self.vector_index.write() = restored;
        Ok(())
    }

    /// Multi-hop retrieval over the bi-temporal fact graph via Personalized
    /// PageRank (HippoRAG-style), seeded at the given query entities.
    pub fn graph_search(&self, seed_entities: &[String]) -> Vec<GraphHit> {
        let store = self.temporal.read();
        GraphRetriever::new(PprConfig::default()).search(&store, seed_entities)
    }

    /// Generative-Agents retrieval: rank a namespace's memories by the
    /// normalised `recency + importance + relevance` score.
    pub async fn generative_agent_search(
        &self,
        query_embedding: Option<&[f32]>,
        namespace: Option<&MemoryNamespace>,
        weights: GaWeights,
    ) -> Result<Vec<GaScored>> {
        let mut vault_query = SearchQuery::default();
        if let Some(ns) = namespace {
            vault_query.namespace = Some(ns.0.clone());
            vault_query.include_child_namespaces = true;
        }
        vault_query.limit = Some(500);
        let memories = self.vault.search(&vault_query).await?;
        let candidates: Vec<RetrievalCandidate> =
            memories.iter().map(RetrievalCandidate::from_memcube).collect();
        let scorer = GenerativeAgentScorer::new(weights);
        Ok(scorer.rank(&candidates, query_embedding, chrono::Utc::now()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub l1_cache_size: usize,
    pub l2_cache_size: usize,
    pub l3_cache_size: usize,
    pub l1_cache_hits: u64,
    pub l2_cache_hits: u64,
    pub l3_cache_hits: u64,
    pub cache_misses: u64,
    pub total_operations: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            (self.l1_cache_hits + self.l2_cache_hits + self.l3_cache_hits) as f64 / self.total_operations as f64
        }
    }
}
