// src/agents/llm.rs
//! Anthropic (Claude) client for the agent layer.
//!
//! Provides the real LLM backing for agent conversation execution and any
//! future LLM-driven memory extraction/reflection. It is configured entirely
//! from the environment so the binary needs no code changes to enable it:
//!
//! * `ANTHROPIC_API_KEY` — required to make real calls. When unset, the client
//!   reports `is_configured() == false` and callers degrade gracefully (they
//!   return an honest "not configured" status rather than a fabricated reply).
//! * `ANTHROPIC_MODEL` — optional model id (default: `claude-sonnet-4-6`).
//! * `ANTHROPIC_BASE_URL` — optional override (e.g. a proxy/gateway).
//!
//! The system prompt is sent with `cache_control: ephemeral` so repeated agent
//! turns benefit from Anthropic prompt caching.

use crate::error::{GaussOSError, Result};
use serde::Serialize;

const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// A single chat turn passed to the model.
#[derive(Debug, Clone, Serialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

impl ChatTurn {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: role.into(), content: content.into() }
    }
}

/// Minimal Anthropic Messages API client.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: Option<String>,
    model: String,
    base_url: String,
    max_tokens: u32,
}

impl AnthropicClient {
    /// Build a client from environment variables.
    pub fn from_env() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
        Self {
            http: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_string(),
            max_tokens: 1024,
        }
    }

    /// True when an API key is present and real calls can be made.
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Normalise an arbitrary role to one Anthropic accepts (`user`/`assistant`).
    fn norm_role(role: &str) -> &'static str {
        match role.to_lowercase().as_str() {
            "assistant" | "model" => "assistant",
            _ => "user",
        }
    }

    /// Send a completion request and return the assistant's text.
    pub async fn complete(&self, system: &str, turns: &[ChatTurn]) -> Result<String> {
        let key = self
            .api_key
            .as_ref()
            .ok_or_else(|| GaussOSError::NotImplemented("ANTHROPIC_API_KEY not set".to_string()))?;

        let messages: Vec<serde_json::Value> = turns
            .iter()
            .map(|t| serde_json::json!({ "role": Self::norm_role(&t.role), "content": t.content }))
            .collect();

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            // Cache the (stable) system prompt across agent turns.
            "system": [{ "type": "text", "text": system, "cache_control": { "type": "ephemeral" } }],
            "messages": messages,
        });

        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(format!("Anthropic request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let detail = resp.text().await.unwrap_or_default();
            return Err(GaussOSError::NetworkError(format!(
                "Anthropic API returned {status}: {detail}"
            )));
        }

        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GaussOSError::NetworkError(format!("Anthropic decode failed: {e}")))?;

        // Messages API returns { content: [{ type: "text", text: "..." }, ...] }.
        let text = value["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        Ok(text)
    }
}

impl Default for AnthropicClient {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_without_key() {
        // Ensure no key is seen for this assertion.
        std::env::remove_var("ANTHROPIC_API_KEY");
        let client = AnthropicClient::from_env();
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn complete_errors_when_unconfigured() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        let client = AnthropicClient::from_env();
        let err = client
            .complete("system", &[ChatTurn::new("user", "hi")])
            .await;
        assert!(err.is_err());
    }

    #[test]
    fn role_normalisation() {
        assert_eq!(AnthropicClient::norm_role("assistant"), "assistant");
        assert_eq!(AnthropicClient::norm_role("system"), "user");
        assert_eq!(AnthropicClient::norm_role("tool"), "user");
        assert_eq!(AnthropicClient::norm_role("USER"), "user");
    }

    #[test]
    fn default_model_is_current() {
        std::env::remove_var("ANTHROPIC_MODEL");
        let client = AnthropicClient::from_env();
        assert_eq!(client.model(), DEFAULT_MODEL);
    }
}
