//! Simulation-related commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::RwLock;

/// Simulation data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub agent_count: u64,
    pub current_step: u64,
    pub total_steps: u64,
    pub created_at: String,
    pub updated_at: String,
    pub config: serde_json::Value,
}

/// Simulation creation request
#[derive(Debug, Deserialize)]
pub struct CreateSimulationRequest {
    pub name: String,
    pub description: Option<String>,
    pub config: serde_json::Value,
}

/// Simulation update request
#[derive(Debug, Deserialize)]
pub struct UpdateSimulationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// List all simulations
#[tauri::command]
pub async fn list_simulations(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<Simulation>, String> {
    let state = state.read().await;

    if let Some(ref db) = state.db {
        db.list_simulations()
            .await
            .map_err(|e| format!("Failed to list simulations: {}", e))
    } else {
        Err("Database not initialized".into())
    }
}

/// Get a specific simulation
#[tauri::command]
pub async fn get_simulation(
    id: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Simulation, String> {
    let state = state.read().await;

    if let Some(ref db) = state.db {
        db.get_simulation(&id)
            .await
            .map_err(|e| format!("Failed to get simulation: {}", e))?
            .ok_or_else(|| "Simulation not found".into())
    } else {
        Err("Database not initialized".into())
    }
}

/// Create a new simulation
#[tauri::command]
pub async fn create_simulation(
    request: CreateSimulationRequest,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Simulation, String> {
    let state = state.read().await;

    if let Some(ref db) = state.db {
        let simulation = Simulation {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            description: request.description.unwrap_or_default(),
            status: "stopped".into(),
            agent_count: 0,
            current_step: 0,
            total_steps: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            config: request.config,
        };

        db.create_simulation(&simulation)
            .await
            .map_err(|e| format!("Failed to create simulation: {}", e))?;

        Ok(simulation)
    } else {
        Err("Database not initialized".into())
    }
}

/// Update a simulation
#[tauri::command]
pub async fn update_simulation(
    id: String,
    request: UpdateSimulationRequest,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Simulation, String> {
    let state = state.read().await;

    if let Some(ref db) = state.db {
        let mut simulation = db
            .get_simulation(&id)
            .await
            .map_err(|e| format!("Failed to get simulation: {}", e))?
            .ok_or_else(|| "Simulation not found".to_string())?;

        if let Some(name) = request.name {
            simulation.name = name;
        }
        if let Some(description) = request.description {
            simulation.description = description;
        }
        if let Some(config) = request.config {
            simulation.config = config;
        }
        simulation.updated_at = chrono::Utc::now().to_rfc3339();

        db.update_simulation(&simulation)
            .await
            .map_err(|e| format!("Failed to update simulation: {}", e))?;

        Ok(simulation)
    } else {
        Err("Database not initialized".into())
    }
}

/// Delete a simulation
#[tauri::command]
pub async fn delete_simulation(
    id: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let state = state.read().await;

    if let Some(ref db) = state.db {
        db.delete_simulation(&id)
            .await
            .map_err(|e| format!("Failed to delete simulation: {}", e))
    } else {
        Err("Database not initialized".into())
    }
}

/// Start a simulation
#[tauri::command]
pub async fn start_simulation(
    id: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let mut state = state.write().await;

    // Update simulation status
    if let Some(ref db) = state.db {
        let mut simulation = db
            .get_simulation(&id)
            .await
            .map_err(|e| format!("Failed to get simulation: {}", e))?
            .ok_or_else(|| "Simulation not found".to_string())?;

        simulation.status = "running".into();
        simulation.updated_at = chrono::Utc::now().to_rfc3339();

        db.update_simulation(&simulation)
            .await
            .map_err(|e| format!("Failed to update simulation: {}", e))?;

        // Update current simulation state
        state.current_simulation = Some(crate::state::SimulationState {
            id: simulation.id,
            name: simulation.name,
            file_path: None,
            status: crate::state::SimulationStatus::Running,
            modified: false,
        });
    }

    // Emit event to frontend
    let _ = state.app_handle.emit("simulation-started", &id);

    Ok(())
}

/// Pause a simulation
#[tauri::command]
pub async fn pause_simulation(
    id: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let mut state = state.write().await;

    if let Some(ref db) = state.db {
        let mut simulation = db
            .get_simulation(&id)
            .await
            .map_err(|e| format!("Failed to get simulation: {}", e))?
            .ok_or_else(|| "Simulation not found".to_string())?;

        simulation.status = "paused".into();
        simulation.updated_at = chrono::Utc::now().to_rfc3339();

        db.update_simulation(&simulation)
            .await
            .map_err(|e| format!("Failed to update simulation: {}", e))?;

        if let Some(ref mut current) = state.current_simulation {
            if current.id == id {
                current.status = crate::state::SimulationStatus::Paused;
            }
        }
    }

    let _ = state.app_handle.emit("simulation-paused", &id);

    Ok(())
}

/// Stop a simulation
#[tauri::command]
pub async fn stop_simulation(
    id: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let mut state = state.write().await;

    if let Some(ref db) = state.db {
        let mut simulation = db
            .get_simulation(&id)
            .await
            .map_err(|e| format!("Failed to get simulation: {}", e))?
            .ok_or_else(|| "Simulation not found".to_string())?;

        simulation.status = "stopped".into();
        simulation.updated_at = chrono::Utc::now().to_rfc3339();

        db.update_simulation(&simulation)
            .await
            .map_err(|e| format!("Failed to update simulation: {}", e))?;

        if let Some(ref mut current) = state.current_simulation {
            if current.id == id {
                current.status = crate::state::SimulationStatus::Stopped;
            }
        }
    }

    let _ = state.app_handle.emit("simulation-stopped", &id);

    Ok(())
}

/// Export a simulation to a file
#[tauri::command]
pub async fn export_simulation(
    id: String,
    path: PathBuf,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    let state = state.read().await;

    if let Some(ref db) = state.db {
        let simulation = db
            .get_simulation(&id)
            .await
            .map_err(|e| format!("Failed to get simulation: {}", e))?
            .ok_or_else(|| "Simulation not found".to_string())?;

        let json = serde_json::to_string_pretty(&simulation)
            .map_err(|e| format!("Failed to serialize simulation: {}", e))?;

        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    } else {
        Err("Database not initialized".into())
    }
}

/// Import a simulation from a file
#[tauri::command]
pub async fn import_simulation(
    path: PathBuf,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Simulation, String> {
    let state = state.read().await;

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut simulation: Simulation = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse simulation: {}", e))?;

    // Generate new ID for imported simulation
    simulation.id = uuid::Uuid::new_v4().to_string();
    simulation.created_at = chrono::Utc::now().to_rfc3339();
    simulation.updated_at = chrono::Utc::now().to_rfc3339();

    if let Some(ref db) = state.db {
        db.create_simulation(&simulation)
            .await
            .map_err(|e| format!("Failed to import simulation: {}", e))?;
    }

    Ok(simulation)
}
