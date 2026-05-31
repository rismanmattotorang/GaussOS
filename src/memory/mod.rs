//! Memory subsystem for GaussOS
//! Provides comprehensive memory management, extraction, and processing capabilities

pub mod extraction;
pub mod manager;
pub mod schemas;
pub mod similarity;
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
// Temporarily disabled advanced module exports
// pub use advanced::{
//     TwoPhaseMemoryProcessor, AdvancedMemoryOperation, ExtractionEngine, UpdateEngine,
//     ConflictResolver, QualityValidator, Conversation, ProcessingContext, ProcessingResult,
// };
