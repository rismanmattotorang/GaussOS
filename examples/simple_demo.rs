// examples/simple_demo.rs
//! Simple demonstration of GaussOS core functionality

use gaussos::{
    core::{MemCube, MemoryPayload},
    database::SkyTableVault,
    error::Result,
    memory::{MemoryManager, MemoryManagerConfig},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 GaussOS Simple Demo");
    println!("======================\n");

    // Initialize SkyTable database (which uses in-memory storage for demo)
    let vault = Arc::new(SkyTableVault::new(Default::default()));
    let config = MemoryManagerConfig::default();
    let memory_manager = Arc::new(MemoryManager::new_optimized(vault, config));

    // Create a simple memory
    let payload = MemoryPayload::Plaintext {
        content: "Hello, GaussOS! This is a simple memory.".to_string(),
        encoding: "utf-8".to_string(),
        language: Some("en".to_string()),
        embeddings: None,
    };

    let memory = MemCube::new(payload);
    let memory_id = memory.id;

    // Store the memory
    println!("📝 Creating memory...");
    memory_manager.create_memory(memory).await?;
    println!("✅ Memory created with ID: {}", memory_id);

    // Retrieve the memory
    println!("\n🔍 Retrieving memory...");
    if let Some(retrieved) = memory_manager.get_memory(&memory_id).await? {
        println!("✅ Memory retrieved:");
        println!("   ID: {}", retrieved.id);
        println!("   Content: {}", retrieved.get_content_summary());
        println!("   Created: {}", retrieved.created_at);
    }

    // Search for memories
    println!("\n🔎 Searching memories...");
    let search_query = gaussos::database::SearchQuery {
        text: Some("GaussOS".to_string()),
        ..Default::default()
    };

    let results = memory_manager.search_memories(search_query).await?;
    println!("✅ Found {} memories", results.len());

    println!("\n🎯 Demo completed successfully!");
    Ok(())
}
