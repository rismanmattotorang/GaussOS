//! SQLite database for local storage

use crate::commands::simulation::Simulation;
use crate::state::{AppSettings, RecentFile};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Database wrapper
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database connection
    pub async fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().await;

        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Create simulations table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS simulations (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                status TEXT DEFAULT 'stopped',
                agent_count INTEGER DEFAULT 0,
                current_step INTEGER DEFAULT 0,
                total_steps INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                config TEXT DEFAULT '{}'
            )",
            [],
        )?;

        // Create recent_files table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS recent_files (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                last_opened TEXT NOT NULL,
                pinned INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Create cache table for offline data
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                expires_at TEXT
            )",
            [],
        )?;

        Ok(())
    }

    /// Get settings
    pub async fn get_settings(&self) -> Result<Option<AppSettings>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = 'app_settings'",
            [],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save settings
    pub async fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        let conn = self.conn.lock().await;
        let json = serde_json::to_string(settings)?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('app_settings', ?1)",
            params![json],
        )?;

        Ok(())
    }

    /// List all simulations
    pub async fn list_simulations(&self) -> Result<Vec<Simulation>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, agent_count, current_step, total_steps, created_at, updated_at, config
             FROM simulations ORDER BY updated_at DESC"
        )?;

        let simulations = stmt
            .query_map([], |row| {
                let config_str: String = row.get(9)?;
                let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or_default();
                
                Ok(Simulation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    agent_count: row.get(4)?,
                    current_step: row.get(5)?,
                    total_steps: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    config,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(simulations)
    }

    /// Get a simulation by ID
    pub async fn get_simulation(&self, id: &str) -> Result<Option<Simulation>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, description, status, agent_count, current_step, total_steps, created_at, updated_at, config
             FROM simulations WHERE id = ?1",
            params![id],
            |row| {
                let config_str: String = row.get(9)?;
                let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or_default();
                
                Ok(Simulation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    agent_count: row.get(4)?,
                    current_step: row.get(5)?,
                    total_steps: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    config,
                })
            },
        );

        match result {
            Ok(sim) => Ok(Some(sim)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a simulation
    pub async fn create_simulation(&self, simulation: &Simulation) -> Result<()> {
        let conn = self.conn.lock().await;
        let config_json = serde_json::to_string(&simulation.config)?;

        conn.execute(
            "INSERT INTO simulations (id, name, description, status, agent_count, current_step, total_steps, created_at, updated_at, config)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                simulation.id,
                simulation.name,
                simulation.description,
                simulation.status,
                simulation.agent_count,
                simulation.current_step,
                simulation.total_steps,
                simulation.created_at,
                simulation.updated_at,
                config_json,
            ],
        )?;

        Ok(())
    }

    /// Update a simulation
    pub async fn update_simulation(&self, simulation: &Simulation) -> Result<()> {
        let conn = self.conn.lock().await;
        let config_json = serde_json::to_string(&simulation.config)?;

        conn.execute(
            "UPDATE simulations SET name = ?1, description = ?2, status = ?3, agent_count = ?4, 
             current_step = ?5, total_steps = ?6, updated_at = ?7, config = ?8 WHERE id = ?9",
            params![
                simulation.name,
                simulation.description,
                simulation.status,
                simulation.agent_count,
                simulation.current_step,
                simulation.total_steps,
                simulation.updated_at,
                config_json,
                simulation.id,
            ],
        )?;

        Ok(())
    }

    /// Delete a simulation
    pub async fn delete_simulation(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM simulations WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Get recent files
    pub async fn get_recent_files(&self) -> Result<Vec<RecentFile>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT path, name, last_opened, pinned FROM recent_files ORDER BY last_opened DESC"
        )?;

        let files = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                let last_opened: String = row.get(2)?;
                
                Ok(RecentFile {
                    path: std::path::PathBuf::from(path),
                    name: row.get(1)?,
                    last_opened: chrono::DateTime::parse_from_rfc3339(&last_opened)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    pinned: row.get::<_, i32>(3)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Save recent files
    pub async fn save_recent_files(&self, files: &[RecentFile]) -> Result<()> {
        let conn = self.conn.lock().await;

        // Clear existing
        conn.execute("DELETE FROM recent_files", [])?;

        // Insert all
        for file in files {
            conn.execute(
                "INSERT INTO recent_files (path, name, last_opened, pinned) VALUES (?1, ?2, ?3, ?4)",
                params![
                    file.path.to_string_lossy().to_string(),
                    file.name,
                    file.last_opened.to_rfc3339(),
                    file.pinned as i32,
                ],
            )?;
        }

        Ok(())
    }

    /// Clear recent files
    pub async fn clear_recent_files(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM recent_files", [])?;
        Ok(())
    }

    /// Set cache value
    #[allow(dead_code)]
    pub async fn set_cache(&self, key: &str, value: &str, ttl_seconds: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().await;
        
        let expires_at = ttl_seconds.map(|ttl| {
            (chrono::Utc::now() + chrono::Duration::seconds(ttl)).to_rfc3339()
        });

        conn.execute(
            "INSERT OR REPLACE INTO cache (key, value, expires_at) VALUES (?1, ?2, ?3)",
            params![key, value, expires_at],
        )?;

        Ok(())
    }

    /// Get cache value
    #[allow(dead_code)]
    pub async fn get_cache(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT value, expires_at FROM cache WHERE key = ?1",
            params![key],
            |row| {
                let value: String = row.get(0)?;
                let expires_at: Option<String> = row.get(1)?;
                Ok((value, expires_at))
            },
        );

        match result {
            Ok((value, expires_at)) => {
                // Check if expired
                if let Some(exp) = expires_at {
                    if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(&exp) {
                        if exp_time < chrono::Utc::now() {
                            // Expired, delete and return None
                            let _ = conn.execute("DELETE FROM cache WHERE key = ?1", params![key]);
                            return Ok(None);
                        }
                    }
                }
                Ok(Some(value))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete cache value
    #[allow(dead_code)]
    pub async fn delete_cache(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM cache WHERE key = ?1", params![key])?;
        Ok(())
    }

    /// Clean expired cache entries
    #[allow(dead_code)]
    pub async fn clean_expired_cache(&self) -> Result<usize> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        
        let deleted = conn.execute(
            "DELETE FROM cache WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![now],
        )?;

        Ok(deleted)
    }
}
