//! API client for GaussTwin TUI
//!
//! Provides HTTP and WebSocket communication with the GaussTwin backend.

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// API client for communicating with the GaussTwin backend
pub struct ApiClient {
    /// Base URL for the API
    base_url: String,
    /// HTTP client
    client: reqwest::Client,
    /// Authentication token
    token: Arc<RwLock<Option<String>>>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the authentication token
    pub async fn set_token(&self, token: Option<String>) {
        let mut t = self.token.write().await;
        *t = token;
    }

    /// Get headers including auth token if available
    async fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        
        if let Some(token) = self.token.read().await.as_ref() {
            if let Ok(auth) = format!("Bearer {}", token).parse() {
                headers.insert(reqwest::header::AUTHORIZATION, auth);
            }
        }
        
        headers
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("GET {}", url);
        
        let response = self
            .client
            .get(&url)
            .headers(self.get_headers().await)
            .send()
            .await?;
        
        self.handle_response(response).await
    }

    /// Make a POST request
    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, endpoint: &str, body: &B) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("POST {}", url);
        
        let response = self
            .client
            .post(&url)
            .headers(self.get_headers().await)
            .json(body)
            .send()
            .await?;
        
        self.handle_response(response).await
    }

    /// Make a PUT request
    pub async fn put<T: DeserializeOwned, B: Serialize>(&self, endpoint: &str, body: &B) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("PUT {}", url);
        
        let response = self
            .client
            .put(&url)
            .headers(self.get_headers().await)
            .json(body)
            .send()
            .await?;
        
        self.handle_response(response).await
    }

    /// Make a DELETE request
    pub async fn delete<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("DELETE {}", url);
        
        let response = self
            .client
            .delete(&url)
            .headers(self.get_headers().await)
            .send()
            .await?;
        
        self.handle_response(response).await
    }

    /// Handle API response
    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("API error {}: {}", status, error_text);
            return Err(anyhow!("API error {}: {}", status, error_text));
        }
        
        let data = response.json::<T>().await?;
        Ok(data)
    }
}

// ============================================================================
// API Types
// ============================================================================

/// API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Paginated response
#[derive(Debug, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

/// Pagination info
#[derive(Debug, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// Simulation from API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiSimulation {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub config: SimulationConfig,
    pub metrics: SimulationMetrics,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimulationConfig {
    pub max_steps: Option<u64>,
    pub time_step: f64,
    pub scheduler: String,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimulationMetrics {
    pub current_step: u64,
    pub elapsed_time: f64,
    pub agent_count: u64,
    pub events_processed: u64,
    pub steps_per_second: f64,
}

/// Agent from API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiAgent {
    pub id: String,
    pub simulation_id: String,
    pub agent_type: String,
    pub state: serde_json::Value,
    pub position: Option<Position>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

/// Health response
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
}

// ============================================================================
// Service Methods
// ============================================================================

impl ApiClient {
    /// Check API health
    pub async fn health(&self) -> Result<HealthResponse> {
        self.get("/health").await
    }

    /// List simulations
    pub async fn list_simulations(&self, page: u32, per_page: u32) -> Result<ApiResponse<PaginatedResponse<ApiSimulation>>> {
        let endpoint = format!("/api/v1/simulations?page={}&per_page={}", page, per_page);
        self.get(&endpoint).await
    }

    /// Get simulation by ID
    pub async fn get_simulation(&self, id: &str) -> Result<ApiResponse<ApiSimulation>> {
        let endpoint = format!("/api/v1/simulations/{}", id);
        self.get(&endpoint).await
    }

    /// Start simulation
    pub async fn start_simulation(&self, id: &str) -> Result<ApiResponse<serde_json::Value>> {
        let endpoint = format!("/api/v1/simulations/{}/start", id);
        self.post(&endpoint, &serde_json::json!({})).await
    }

    /// Pause simulation
    pub async fn pause_simulation(&self, id: &str) -> Result<ApiResponse<serde_json::Value>> {
        let endpoint = format!("/api/v1/simulations/{}/pause", id);
        self.post(&endpoint, &serde_json::json!({})).await
    }

    /// Stop simulation
    pub async fn stop_simulation(&self, id: &str) -> Result<ApiResponse<serde_json::Value>> {
        let endpoint = format!("/api/v1/simulations/{}/stop", id);
        self.post(&endpoint, &serde_json::json!({})).await
    }

    /// Get simulation metrics
    pub async fn get_simulation_metrics(&self, id: &str) -> Result<ApiResponse<SimulationMetrics>> {
        let endpoint = format!("/api/v1/simulations/{}/metrics", id);
        self.get(&endpoint).await
    }

    /// List agents in simulation
    pub async fn list_agents(&self, simulation_id: &str, page: u32, per_page: u32) -> Result<ApiResponse<PaginatedResponse<ApiAgent>>> {
        let endpoint = format!("/api/v1/simulations/{}/agents?page={}&per_page={}", simulation_id, page, per_page);
        self.get(&endpoint).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new("http://localhost:8080");
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
