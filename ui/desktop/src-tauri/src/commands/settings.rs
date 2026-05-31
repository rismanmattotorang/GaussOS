//! Settings-related commands

use crate::state::{AppSettings, AppState};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// Get current settings
#[tauri::command]
pub async fn get_settings(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<AppSettings, String> {
    let state = state.read().await;
    Ok(state.settings.clone())
}

/// Update settings request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub auto_save: Option<bool>,
    pub auto_save_interval: Option<u32>,
    pub check_updates_on_start: Option<bool>,
    pub start_with_system: Option<bool>,
    pub minimize_to_tray: Option<bool>,
    pub show_notifications: Option<bool>,
    pub api_endpoint: Option<String>,
    pub max_recent_files: Option<usize>,
    pub editor_font_size: Option<u32>,
    pub animation_enabled: Option<bool>,
}

/// Update settings
#[tauri::command]
pub async fn update_settings(
    request: UpdateSettingsRequest,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<AppSettings, String> {
    let mut state = state.write().await;

    // Apply updates
    if let Some(theme) = request.theme {
        state.settings.theme = theme;
    }
    if let Some(language) = request.language {
        state.settings.language = language;
    }
    if let Some(auto_save) = request.auto_save {
        state.settings.auto_save = auto_save;
    }
    if let Some(interval) = request.auto_save_interval {
        state.settings.auto_save_interval = interval;
    }
    if let Some(check) = request.check_updates_on_start {
        state.settings.check_updates_on_start = check;
    }
    if let Some(start) = request.start_with_system {
        state.settings.start_with_system = start;
        // TODO: Register/unregister auto-start
    }
    if let Some(minimize) = request.minimize_to_tray {
        state.settings.minimize_to_tray = minimize;
    }
    if let Some(notifications) = request.show_notifications {
        state.settings.show_notifications = notifications;
    }
    if let Some(endpoint) = request.api_endpoint {
        state.settings.api_endpoint = endpoint;
    }
    if let Some(max_files) = request.max_recent_files {
        state.settings.max_recent_files = max_files;
    }
    if let Some(font_size) = request.editor_font_size {
        state.settings.editor_font_size = font_size;
    }
    if let Some(animation) = request.animation_enabled {
        state.settings.animation_enabled = animation;
    }

    // Save to database
    state.save_settings()
        .await
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(state.settings.clone())
}

/// Reset settings to defaults
#[tauri::command]
pub async fn reset_settings(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<AppSettings, String> {
    let mut state = state.write().await;
    state.settings = AppSettings::default();

    state.save_settings()
        .await
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(state.settings.clone())
}
