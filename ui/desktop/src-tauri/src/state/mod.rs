//! Application state management

use crate::db::Database;
use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

/// Application state
pub struct AppState {
    /// Tauri app handle
    pub app_handle: AppHandle,
    /// Database connection
    pub db: Option<Database>,
    /// Application settings
    pub settings: AppSettings,
    /// Recent files
    pub recent_files: Vec<RecentFile>,
    /// File watchers
    pub watchers: Arc<Mutex<HashMap<PathBuf, RecommendedWatcher>>>,
    /// Current simulation state
    pub current_simulation: Option<SimulationState>,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub auto_save: bool,
    pub auto_save_interval: u32,
    pub check_updates_on_start: bool,
    pub start_with_system: bool,
    pub minimize_to_tray: bool,
    pub show_notifications: bool,
    pub api_endpoint: String,
    pub max_recent_files: usize,
    pub editor_font_size: u32,
    pub animation_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "en".into(),
            auto_save: true,
            auto_save_interval: 60,
            check_updates_on_start: true,
            start_with_system: false,
            minimize_to_tray: true,
            show_notifications: true,
            api_endpoint: "https://api.gausstwin.io".into(),
            max_recent_files: 10,
            editor_font_size: 14,
            animation_enabled: true,
        }
    }
}

/// Recent file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: chrono::DateTime<chrono::Utc>,
    pub pinned: bool,
}

/// Current simulation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub id: String,
    pub name: String,
    pub file_path: Option<PathBuf>,
    pub status: SimulationStatus,
    pub modified: bool,
}

/// Simulation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SimulationStatus {
    Running,
    Paused,
    Stopped,
    Completed,
    Error,
}

impl AppState {
    /// Create new application state
    pub fn new(app_handle: AppHandle) -> Result<Self> {
        Ok(Self {
            app_handle,
            db: None,
            settings: AppSettings::default(),
            recent_files: Vec::new(),
            watchers: Arc::new(Mutex::new(HashMap::new())),
            current_simulation: None,
        })
    }

    /// Initialize database
    pub async fn init_database(&mut self) -> Result<()> {
        let app_dir = self.get_app_data_dir()?;
        let db_path = app_dir.join("gausstwin.db");

        let db = Database::new(&db_path).await?;
        db.migrate().await?;

        // Load settings from database
        if let Ok(Some(settings)) = db.get_settings().await {
            self.settings = settings;
        }

        // Load recent files
        if let Ok(files) = db.get_recent_files().await {
            self.recent_files = files;
        }

        self.db = Some(db);
        Ok(())
    }

    /// Get application data directory
    pub fn get_app_data_dir(&self) -> Result<PathBuf> {
        let app_dir = directories::ProjectDirs::from("io", "gausstwin", "GaussTwin")
            .ok_or_else(|| anyhow::anyhow!("Failed to get app directory"))?
            .data_dir()
            .to_path_buf();

        if !app_dir.exists() {
            std::fs::create_dir_all(&app_dir)?;
        }

        Ok(app_dir)
    }

    /// Get log directory
    pub fn get_log_dir(&self) -> Result<PathBuf> {
        let log_dir = self.get_app_data_dir()?.join("logs");
        if !log_dir.exists() {
            std::fs::create_dir_all(&log_dir)?;
        }
        Ok(log_dir)
    }

    /// Get cache directory
    pub fn get_cache_dir(&self) -> Result<PathBuf> {
        let cache_dir = directories::ProjectDirs::from("io", "gausstwin", "GaussTwin")
            .ok_or_else(|| anyhow::anyhow!("Failed to get cache directory"))?
            .cache_dir()
            .to_path_buf();

        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }

        Ok(cache_dir)
    }

    /// Add recent file
    pub async fn add_recent_file(&mut self, path: PathBuf) -> Result<()> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Remove if already exists
        self.recent_files.retain(|f| f.path != path);

        // Add to front
        self.recent_files.insert(
            0,
            RecentFile {
                path: path.clone(),
                name,
                last_opened: chrono::Utc::now(),
                pinned: false,
            },
        );

        // Limit to max recent files
        while self.recent_files.len() > self.settings.max_recent_files {
            self.recent_files.pop();
        }

        // Save to database
        if let Some(ref db) = self.db {
            db.save_recent_files(&self.recent_files).await?;
        }

        Ok(())
    }

    /// Watch a directory for file changes
    pub async fn watch_directory(&self, path: PathBuf) -> Result<()> {
        let app_handle = self.app_handle.clone();
        let path_clone = path.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let _ = app_handle.emit("file-changed", FileChangeEvent {
                    paths: event.paths.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                    kind: format!("{:?}", event.kind),
                });
            }
        })?;

        watcher.watch(&path, RecursiveMode::Recursive)?;

        let mut watchers = self.watchers.lock().await;
        watchers.insert(path_clone, watcher);

        Ok(())
    }

    /// Stop watching a directory
    pub async fn unwatch_directory(&self, path: &PathBuf) -> Result<()> {
        let mut watchers = self.watchers.lock().await;
        if let Some(mut watcher) = watchers.remove(path) {
            watcher.unwatch(path)?;
        }
        Ok(())
    }

    /// Save settings
    pub async fn save_settings(&self) -> Result<()> {
        if let Some(ref db) = self.db {
            db.save_settings(&self.settings).await?;
        }
        Ok(())
    }

    /// Cleanup before exit
    pub async fn cleanup(&self) -> Result<()> {
        // Stop all file watchers
        let mut watchers = self.watchers.lock().await;
        watchers.clear();

        tracing::info!("Application cleanup complete");
        Ok(())
    }
}

/// File change event
#[derive(Debug, Clone, Serialize)]
pub struct FileChangeEvent {
    pub paths: Vec<String>,
    pub kind: String,
}
