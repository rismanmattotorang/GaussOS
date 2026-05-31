//! Application configuration

use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// API server URL
    pub api_url: String,
    /// Theme name
    pub theme: String,
    /// Mouse support enabled
    pub mouse_enabled: bool,
    /// Tick rate in milliseconds
    pub tick_rate: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8080".to_string(),
            theme: "tokyo-night".to_string(),
            mouse_enabled: true,
            tick_rate: 250,
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default config path
    pub fn default_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("io", "gausstwin", "GaussTwin")
            .map(|dirs| dirs.config_dir().join("tui.toml"))
    }
}
