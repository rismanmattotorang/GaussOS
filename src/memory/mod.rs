//! Memory subsystem for GaussOS
//! Provides comprehensive memory management, extraction, and processing capabilities

pub mod decay;
pub mod extraction;
pub mod hierarchy;
pub mod manager;
pub mod retrieval;
pub mod schemas;
pub mod similarity;
pub mod temporal;
// pub mod advanced; // Temporarily disabled due to compilation issues

pub use extraction::{
    ExtractionPhase, ExtractionStrategy, MemoryConsolidator, ParallelEntityExtractor,
    RustMemoryExtractor, SimdTextProcessor,
};
pub use manager::{
    ConsolidationReport, MemoryExtractionRequest, MemoryExtractionResponse, MemoryManager,
    MemoryManagerConfig,
};
pub use schemas::{
    ConversationSummarySchema, ExtractionMode, MemorySchema, PromptOptimizationSchema,
    SchemaRegistry, UserProfileSchema,
};
pub use decay::{DecayConfig, ForgettingCurve, RetentionAction, RetentionPlan, RetentionScore};
pub use hierarchy::{HierarchyBuilder, LayerNode, MemoryHierarchy, MemoryLayer};
pub use retrieval::{HybridRetriever, HybridSearchConfig, RetrievalCandidate, ScoredMemory};
pub use temporal::{IngestReport, TemporalFact, TemporalFactStore};
// Temporarily disabled advanced module exports
// pub use advanced::{
//     TwoPhaseMemoryProcessor, AdvancedMemoryOperation, ExtractionEngine, UpdateEngine,
//     ConflictResolver, QualityValidator, Conversation, ProcessingContext, ProcessingResult,
// };
