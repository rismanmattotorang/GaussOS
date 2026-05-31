//! System-related commands

use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// System information
#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub hostname: String,
    pub cpu_cores: usize,
    pub memory_total: u64,
}

/// Application paths
#[derive(Debug, Serialize)]
pub struct AppPaths {
    pub data_dir: String,
    pub cache_dir: String,
    pub log_dir: String,
    pub config_dir: String,
}

/// Update check result
#[derive(Debug, Serialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
}

/// Get system information
#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    Ok(SystemInfo {
        os: std::env::consts::OS.to_string(),
        os_version: System::os_version().unwrap_or_else(|| "unknown".to_string()),
        arch: std::env::consts::ARCH.to_string(),
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
        cpu_cores: sys.cpus().len(),
        memory_total: sys.total_memory(),
    })
}

/// Get application paths
#[tauri::command]
pub async fn get_app_paths(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<AppPaths, String> {
    let state = state.read().await;

    let data_dir = state.get_app_data_dir()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let cache_dir = state.get_cache_dir()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let log_dir = state.get_log_dir()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let config_dir = data_dir.clone();

    Ok(AppPaths {
        data_dir,
        cache_dir,
        log_dir,
        config_dir,
    })
}

/// Check for updates
#[tauri::command]
pub async fn check_for_updates(
    app: tauri::AppHandle,
) -> Result<UpdateCheckResult, String> {
    use tauri_plugin_updater::UpdaterExt;

    let current_version = app.package_info().version.to_string();

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    Ok(UpdateCheckResult {
                        available: true,
                        current_version,
                        latest_version: Some(update.version.clone()),
                        release_notes: update.body.clone(),
                        download_url: None,
                    })
                }
                Ok(None) => {
                    Ok(UpdateCheckResult {
                        available: false,
                        current_version,
                        latest_version: None,
                        release_notes: None,
                        download_url: None,
                    })
                }
                Err(e) => {
                    tracing::warn!("Update check failed: {}", e);
                    Ok(UpdateCheckResult {
                        available: false,
                        current_version,
                        latest_version: None,
                        release_notes: None,
                        download_url: None,
                    })
                }
            }
        }
        Err(e) => {
            tracing::warn!("Updater not available: {}", e);
            Ok(UpdateCheckResult {
                available: false,
                current_version,
                latest_version: None,
                release_notes: None,
                download_url: None,
            })
        }
    }
}
