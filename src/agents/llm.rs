// src/agents/llm.rs
//! Flexible, multi-provider LLM client for the agent layer.
//!
//! GaussOS speaks two wire protocols and presets the major providers on top of
//! them, so the agent layer works with whatever you have a key for — no code
//! changes, only environment variables:
//!
//! | Provider     | Protocol            | Default base URL                                   | Key env (or `LLM_API_KEY`) |
//! |--------------|---------------------|----------------------------------------------------|----------------------------|
//! | `anthropic`  | Anthropic Messages  | `https://api.anthropic.com`                        | `ANTHROPIC_API_KEY`        |
//! | `openai`     | OpenAI ChatCompletions | `https://api.openai.com/v1`                     | `OPENAI_API_KEY`           |
//! | `deepseek`   | OpenAI-compatible   | `https://api.deepseek.com/v1`                      | `DEEPSEEK_API_KEY`         |
//! | `qwen`       | OpenAI-compatible   | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` | `DASHSCOPE_API_KEY`  |
//! | `byteplus`   | OpenAI-compatible   | `https://ark.ap-southeast.bytepluses.com/api/v3`   | `BYTEPLUS_API_KEY`         |
//! | `openrouter` | OpenAI-compatible   | `https://openrouter.ai/api/v1`                     | `OPENROUTER_API_KEY`       |
//! | `custom`     | OpenAI-compatible   | `LLM_BASE_URL` (required)                          | `LLM_API_KEY`              |
//!
//! Selection & overrides (all optional):
//! * `LLM_PROVIDER` — one of the names above. If unset, the first provider with
//!   a key present is auto-selected (else `anthropic`).
//! * `LLM_MODEL` — overrides the provider's default model.
//! * `LLM_BASE_URL` — overrides the provider's base URL (required for `custom`).
//! * `LLM_API_KEY` — a generic key that overrides the provider-specific key env.
//!
//! When no key is found the client reports `is_configured() == false` and
//! callers degrade gracefully instead of fabricating output.

use crate::error::{GaussOSError, Result};
use serde::Serialize;

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Wire protocol a provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Anthropic Messages API (`/v1/messages`, `x-api-key`).
    Anthropic,
    /// OpenAI Chat Completions (`/chat/completions`, `Authorization: Bearer`).
    OpenAi,
}

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

/// Static preset for a provider: protocol, default base URL, default model, and
/// the environment variable holding its API key.
struct Preset {
    name: &'static str,
    protocol: Protocol,
    base_url: &'static str,
    model: &'static str,
    key_env: &'static str,
}

/// All known providers, in auto-detection priority order.
const PRESETS: &[Preset] = &[
    Preset { name: "anthropic", protocol: Protocol::Anthropic, base_url: "https://api.anthropic.com", model: "claude-sonnet-4-6", key_env: "ANTHROPIC_API_KEY" },
    Preset { name: "openai", protocol: Protocol::OpenAi, base_url: "https://api.openai.com/v1", model: "gpt-4o-mini", key_env: "OPENAI_API_KEY" },
    Preset { name: "deepseek", protocol: Protocol::OpenAi, base_url: "https://api.deepseek.com/v1", model: "deepseek-chat", key_env: "DEEPSEEK_API_KEY" },
    Preset { name: "qwen", protocol: Protocol::OpenAi, base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1", model: "qwen-plus", key_env: "DASHSCOPE_API_KEY" },
    Preset { name: "byteplus", protocol: Protocol::OpenAi, base_url: "https://ark.ap-southeast.bytepluses.com/api/v3", model: "skylark-pro", key_env: "BYTEPLUS_API_KEY" },
    Preset { name: "openrouter", protocol: Protocol::OpenAi, base_url: "https://openrouter.ai/api/v1", model: "openai/gpt-4o-mini", key_env: "OPENROUTER_API_KEY" },
    Preset { name: "custom", protocol: Protocol::OpenAi, base_url: "", model: "gpt-4o-mini", key_env: "LLM_API_KEY" },
];

fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.trim().is_empty())
}

/// A configured LLM client for the selected provider.
#[derive(Debug, Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    provider: String,
    protocol: Protocol,
    api_key: Option<String>,
    model: String,
    base_url: String,
    max_tokens: u32,
}

impl LlmClient {
    /// Build a client from the environment, resolving provider, key, model, and
    /// base URL with sensible defaults and overrides.
    pub fn from_env() -> Self {
        // 1. Resolve the provider: explicit LLM_PROVIDER, else first with a key.
        let provider_name = env_nonempty("LLM_PROVIDER")
            .map(|s| s.to_lowercase())
            .filter(|s| preset(s).is_some())
            .or_else(|| {
                PRESETS
                    .iter()
                    .find(|p| env_nonempty("LLM_API_KEY").is_some() || env_nonempty(p.key_env).is_some())
                    .map(|p| p.name.to_string())
            })
            .unwrap_or_else(|| "anthropic".to_string());

        let p = preset(&provider_name).unwrap_or(&PRESETS[0]);

        // 2. Key: generic LLM_API_KEY wins, else the provider-specific env.
        let api_key = env_nonempty("LLM_API_KEY").or_else(|| env_nonempty(p.key_env));

        // 3. Model and base URL: generic overrides win, else the preset default.
        //    Legacy ANTHROPIC_MODEL is still honoured for the anthropic provider.
        let model = env_nonempty("LLM_MODEL")
            .or_else(|| {
                if p.protocol == Protocol::Anthropic {
                    env_nonempty("ANTHROPIC_MODEL")
                } else {
                    None
                }
            })
            .unwrap_or_else(|| p.model.to_string());

        let base_url = env_nonempty("LLM_BASE_URL")
            .or_else(|| {
                if p.protocol == Protocol::Anthropic {
                    env_nonempty("ANTHROPIC_BASE_URL")
                } else {
                    None
                }
            })
            .unwrap_or_else(|| p.base_url.to_string());

        Self {
            http: reqwest::Client::new(),
            provider: provider_name,
            protocol: p.protocol,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_string(),
            max_tokens: 1024,
        }
    }

    /// True when an API key is present (and, for `custom`, a base URL is set).
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some() && !self.base_url.is_empty()
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn norm_role(role: &str) -> &'static str {
        match role.to_lowercase().as_str() {
            "assistant" | "model" => "assistant",
            "system" => "system",
            _ => "user",
        }
    }

    /// Send a completion request and return the assistant's text. Dispatches to
    /// the provider's protocol.
    pub async fn complete(&self, system: &str, turns: &[ChatTurn]) -> Result<String> {
        let key = self.api_key.as_ref().ok_or_else(|| {
            GaussOSError::NotImplemented(format!(
                "No API key for LLM provider '{}' (set LLM_API_KEY or the provider key env)",
                self.provider
            ))
        })?;
        if self.base_url.is_empty() {
            return Err(GaussOSError::NotImplemented(
                "LLM_BASE_URL is required for the 'custom' provider".to_string(),
            ));
        }

        match self.protocol {
            Protocol::Anthropic => self.complete_anthropic(key, system, turns).await,
            Protocol::OpenAi => self.complete_openai(key, system, turns).await,
        }
    }

    async fn complete_anthropic(&self, key: &str, system: &str, turns: &[ChatTurn]) -> Result<String> {
        let messages: Vec<serde_json::Value> = turns
            .iter()
            // Anthropic accepts only user/assistant turns; the system prompt is
            // a top-level field.
            .map(|t| serde_json::json!({ "role": Self::norm_anthropic_role(&t.role), "content": t.content }))
            .collect();

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
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
            return Err(GaussOSError::NetworkError(format!("Anthropic API returned {status}: {detail}")));
        }
        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GaussOSError::NetworkError(format!("Anthropic decode failed: {e}")))?;
        Ok(value["content"]
            .as_array()
            .map(|blocks| blocks.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join(""))
            .unwrap_or_default())
    }

    fn norm_anthropic_role(role: &str) -> &'static str {
        match role.to_lowercase().as_str() {
            "assistant" | "model" => "assistant",
            _ => "user",
        }
    }

    async fn complete_openai(&self, key: &str, system: &str, turns: &[ChatTurn]) -> Result<String> {
        let mut messages: Vec<serde_json::Value> =
            vec![serde_json::json!({ "role": "system", "content": system })];
        for t in turns {
            messages.push(serde_json::json!({ "role": Self::norm_role(&t.role), "content": t.content }));
        }

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": messages,
        });

        let mut req = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .header("authorization", format!("Bearer {key}"))
            .header("content-type", "application/json");
        // OpenRouter recommends attribution headers (harmless elsewhere).
        if self.provider == "openrouter" {
            req = req
                .header("http-referer", "https://github.com/gaussos/gaussos")
                .header("x-title", "GaussOS");
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(format!("{} request failed: {e}", self.provider)))?;

        let status = resp.status();
        if !status.is_success() {
            let detail = resp.text().await.unwrap_or_default();
            return Err(GaussOSError::NetworkError(format!(
                "{} API returned {status}: {detail}",
                self.provider
            )));
        }
        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GaussOSError::NetworkError(format!("{} decode failed: {e}", self.provider)))?;
        Ok(value["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Backwards-compatible alias for code/tests that referenced the
/// Anthropic-specific client name.
pub type AnthropicClient = LlmClient;

#[cfg(test)]
mod tests {
    use super::*;

    // These tests mutate shared process env, so they must not run concurrently.
    // Serialize them on a process-wide lock (tolerating poisoning).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn clear_env() {
        for k in [
            "LLM_PROVIDER", "LLM_API_KEY", "LLM_MODEL", "LLM_BASE_URL",
            "ANTHROPIC_API_KEY", "ANTHROPIC_MODEL", "ANTHROPIC_BASE_URL",
            "OPENAI_API_KEY", "DEEPSEEK_API_KEY", "DASHSCOPE_API_KEY",
            "BYTEPLUS_API_KEY", "OPENROUTER_API_KEY",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn defaults_to_anthropic_when_unconfigured() {
        let _g = env_guard();
        clear_env();
        let c = LlmClient::from_env();
        assert_eq!(c.provider(), "anthropic");
        assert_eq!(c.model(), "claude-sonnet-4-6");
        assert!(!c.is_configured());
    }

    #[test]
    fn explicit_provider_presets() {
        let _g = env_guard();
        clear_env();
        std::env::set_var("LLM_PROVIDER", "deepseek");
        std::env::set_var("DEEPSEEK_API_KEY", "sk-test");
        let c = LlmClient::from_env();
        assert_eq!(c.provider(), "deepseek");
        assert_eq!(c.model(), "deepseek-chat");
        assert_eq!(c.protocol, Protocol::OpenAi);
        assert!(c.is_configured());
        clear_env();
    }

    #[test]
    fn generic_key_and_overrides() {
        let _g = env_guard();
        clear_env();
        std::env::set_var("LLM_PROVIDER", "openrouter");
        std::env::set_var("LLM_API_KEY", "sk-generic");
        std::env::set_var("LLM_MODEL", "deepseek/deepseek-chat");
        let c = LlmClient::from_env();
        assert_eq!(c.provider(), "openrouter");
        assert_eq!(c.model(), "deepseek/deepseek-chat");
        assert!(c.is_configured());
        clear_env();
    }

    #[test]
    fn auto_detects_provider_by_key() {
        let _g = env_guard();
        clear_env();
        std::env::set_var("OPENAI_API_KEY", "sk-openai");
        let c = LlmClient::from_env();
        assert_eq!(c.provider(), "openai");
        assert!(c.is_configured());
        clear_env();
    }

    #[test]
    fn custom_requires_base_url() {
        let _g = env_guard();
        clear_env();
        std::env::set_var("LLM_PROVIDER", "custom");
        std::env::set_var("LLM_API_KEY", "sk-x");
        let c = LlmClient::from_env();
        // No LLM_BASE_URL → not fully configured.
        assert!(!c.is_configured());
        std::env::set_var("LLM_BASE_URL", "http://localhost:11434/v1");
        let c = LlmClient::from_env();
        assert!(c.is_configured());
        clear_env();
    }

    #[tokio::test]
    async fn complete_errors_when_unconfigured() {
        let _g = env_guard();
        clear_env();
        let c = LlmClient::from_env();
        assert!(c.complete("sys", &[ChatTurn::new("user", "hi")]).await.is_err());
    }

    #[test]
    fn role_normalisation() {
        assert_eq!(LlmClient::norm_role("assistant"), "assistant");
        assert_eq!(LlmClient::norm_role("system"), "system");
        assert_eq!(LlmClient::norm_role("tool"), "user");
        assert_eq!(LlmClient::norm_anthropic_role("system"), "user");
    }
}
