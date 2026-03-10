//! Structured LLM provider — fast JSON extraction for event decomposition
//! and action arbitration.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! Distinct from the narrator's `LlmProvider`. The narrator uses a capable
//! model for prose generation. This provider uses a small, fast model
//! (e.g., Qwen2.5:3b-instruct) for structured extraction tasks with
//! constrained JSON output.

use std::time::Duration;

use crate::errors::StorytellerResult;

/// A request for structured JSON extraction from a small LLM.
#[derive(Debug, Clone)]
pub struct StructuredRequest {
    /// System prompt establishing the extraction task.
    pub system: String,
    /// The text to analyze.
    pub input: String,
    /// JSON schema the output must conform to.
    pub output_schema: serde_json::Value,
    /// Temperature (low — extraction, not creativity).
    pub temperature: f32,
}

/// Configuration for the structured LLM service.
#[derive(Debug, Clone)]
pub struct StructuredLlmConfig {
    /// Base URL of the server (e.g., "http://127.0.0.1:11434").
    pub base_url: String,
    /// Model name — distinct from narrator model.
    pub model: String,
    /// Temperature for extraction tasks.
    pub temperature: f32,
    /// Request timeout — these calls should be fast.
    pub timeout: Duration,
}

impl Default for StructuredLlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            model: "qwen2.5:3b-instruct".to_string(),
            temperature: 0.1,
            timeout: Duration::from_secs(10),
        }
    }
}

/// Provider for fast, structured-output LLM calls.
///
/// Used by both event decomposition (D.3) and action arbitration.
/// Implementations connect to Ollama or similar inference servers.
#[async_trait::async_trait]
pub trait StructuredLlmProvider: std::fmt::Debug + Send + Sync {
    /// Send a structured extraction request and receive JSON output.
    async fn extract(&self, request: StructuredRequest) -> StorytellerResult<serde_json::Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_request_builds_with_defaults() {
        let req = StructuredRequest {
            system: "Extract events".to_string(),
            input: "The child laughs at the sprite".to_string(),
            output_schema: serde_json::json!({"type": "object"}),
            temperature: 0.1,
        };
        assert_eq!(req.temperature, 0.1);
        assert!(!req.input.is_empty());
    }

    #[test]
    fn structured_llm_config_has_sensible_defaults() {
        let config = StructuredLlmConfig::default();
        assert_eq!(config.base_url, "http://127.0.0.1:11434");
        assert_eq!(config.model, "qwen2.5:3b-instruct");
        assert_eq!(config.temperature, 0.1);
        assert_eq!(config.timeout, std::time::Duration::from_secs(10));
    }
}
