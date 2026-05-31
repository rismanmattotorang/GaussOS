//! File-related commands

use crate::state::{AppState, RecentFile};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// File content response
#[derive(Debug, Serialize)]
pub struct FileContent {
    pub path: String,
    pub name: String,
    pub content: String,
    pub size: u64,
    pub modified: String,
}

/// Open a file and read its contents
#[tauri::command]
pub async fn open_file(
    path: PathBuf,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<FileContent, String> {
    // Validate file exists
    if !path.exists() {
        return Err("File not found".into());
    }

    // Get file metadata
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    // Read content
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let modified = metadata
        .modified()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_default();

    // Add to recent files
    let mut state = state.write().await;
    state.add_recent_file(path.clone()).await
        .map_err(|e| format!("Failed to add recent file: {}", e))?;

    Ok(FileContent {
        path: path.to_string_lossy().to_string(),
        name,
        content,
        size: metadata.len(),
        modified,
    })
}

/// Save content to a file
#[tauri::command]
pub async fn save_file(
    path: PathBuf,
    content: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directories: {}", e))?;
    }

    // Write content
    std::fs::write(&path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    // Add to recent files
    let mut state = state.write().await;
    state.add_recent_file(path).await
        .map_err(|e| format!("Failed to add recent file: {}", e))?;

    Ok(())
}

/// Get recent files
#[tauri::command]
pub async fn get_recent_files(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<RecentFile>, String> {
    let state = state.read().await;
    
    // Filter out files that no longer exist
    let files: Vec<RecentFile> = state.recent_files
        .iter()
        .filter(|f| f.path.exists())
        .cloned()
        .collect();

    Ok(files)
}

/// Clear recent files
#[tauri::command]
pub async fn clear_recent_files(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let mut state = state.write().await;
    state.recent_files.clear();

    if let Some(ref db) = state.db {
        db.clear_recent_files()
            .await
            .map_err(|e| format!("Failed to clear recent files: {}", e))?;
    }

    Ok(())
}

/// Watch a directory for changes
#[tauri::command]
pub async fn watch_directory(
    path: PathBuf,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let state = state.read().await;
    
    state.watch_directory(path)
        .await
        .map_err(|e| format!("Failed to watch directory: {}", e))
}

/// Stop watching a directory
#[tauri::command]
pub async fn unwatch_directory(
    path: PathBuf,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let state = state.read().await;
    
    state.unwatch_directory(&path)
        .await
        .map_err(|e| format!("Failed to unwatch directory: {}", e))
}
