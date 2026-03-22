// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Ollama-backed structured LLM provider for JSON extraction.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! Implements `StructuredLlmProvider` for Ollama's `/api/chat` endpoint with
//! `format: "json"` to constrain output. Used for fast, structured extraction
//! tasks (event decomposition, action arbitration) with small models like
//! Qwen2.5:3b-instruct.

use storyteller_core::errors::StorytellerError;
use storyteller_core::traits::structured_llm::{
    StructuredLlmConfig, StructuredLlmProvider, StructuredRequest,
};
use tracing::instrument;

/// Ollama-backed provider for structured JSON extraction.
#[derive(Debug)]
pub struct OllamaStructuredProvider {
    /// Provider configuration.
    pub config: StructuredLlmConfig,
    client: reqwest::Client,
}

impl OllamaStructuredProvider {
    /// Create a new provider with the given configuration.
    pub fn new(config: StructuredLlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("failed to build HTTP client");
        Self { config, client }
    }

    /// Create a provider with default configuration (local Ollama, qwen2.5:3b-instruct).
    pub fn default_local() -> Self {
        Self::new(StructuredLlmConfig::default())
    }
}

// -- Ollama API types (private, just for serialization) -----------------------

#[derive(Debug, serde::Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    format: String,
    options: OllamaOptions,
}

#[derive(Debug, serde::Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, serde::Serialize)]
struct OllamaOptions {
    temperature: f32,
}

#[derive(Debug, serde::Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    #[serde(default)]
    eval_count: u32,
    #[serde(default)]
    prompt_eval_count: u32,
}

#[derive(Debug, serde::Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

// -- JSON extraction helper ---------------------------------------------------

/// Parse JSON from an LLM response that may contain markdown fences,
/// bare JSON, or JSON embedded in surrounding text.
pub fn extract_json_from_response(raw: &str) -> Result<serde_json::Value, StorytellerError> {
    let trimmed = raw.trim();

    // Try direct parse first (bare JSON).
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }

    // Try extracting from markdown fences: ```json ... ``` or ``` ... ```
    if let Some(json_str) = extract_from_fences(trimmed) {
        if let Ok(value) = serde_json::from_str(json_str.trim()) {
            return Ok(value);
        }
    }

    // Try finding a JSON object or array in surrounding text.
    if let Some(value) = extract_embedded_json(trimmed) {
        return Ok(value);
    }

    Err(StorytellerError::Inference(format!(
        "failed to extract JSON from LLM response: {raw}"
    )))
}

/// Extract content between markdown code fences.
fn extract_from_fences(text: &str) -> Option<&str> {
    // Match ```json\n...\n``` or ```\n...\n```
    let start = if let Some(pos) = text.find("```json") {
        pos + "```json".len()
    } else if let Some(pos) = text.find("```") {
        pos + "```".len()
    } else {
        return None;
    };

    let remaining = &text[start..];
    let end = remaining.find("```")?;
    Some(&remaining[..end])
}

/// Find the first balanced JSON object or array in text.
fn extract_embedded_json(text: &str) -> Option<serde_json::Value> {
    for (i, ch) in text.char_indices() {
        if ch == '{' || ch == '[' {
            let closing = if ch == '{' { '}' } else { ']' };
            // Search backwards from end for matching close bracket.
            if let Some(end) = text.rfind(closing) {
                if end > i {
                    let candidate = &text[i..=end];
                    if let Ok(value) = serde_json::from_str(candidate) {
                        return Some(value);
                    }
                }
            }
        }
    }
    None
}

// -- StructuredLlmProvider implementation -------------------------------------

#[async_trait::async_trait]
impl StructuredLlmProvider for OllamaStructuredProvider {
    #[instrument(skip(self, request), fields(model = %self.config.model))]
    async fn extract(
        &self,
        request: StructuredRequest,
    ) -> storyteller_core::StorytellerResult<serde_json::Value> {
        let url = format!("{}/api/chat", self.config.base_url);

        let schema_hint = format!(
            "You MUST respond with valid JSON matching this schema:\n{}",
            serde_json::to_string_pretty(&request.output_schema)
                .unwrap_or_else(|_| request.output_schema.to_string()),
        );

        let system_content = format!("{}\n\n{schema_hint}", request.system);

        let messages = vec![
            OllamaMessage {
                role: "system".to_string(),
                content: system_content,
            },
            OllamaMessage {
                role: "user".to_string(),
                content: request.input,
            },
        ];

        let ollama_request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            format: "json".to_string(),
            options: OllamaOptions {
                temperature: request.temperature,
            },
        };

        tracing::debug!("sending structured request to Ollama: {url}");

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| StorytellerError::Inference(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "failed to read body".to_string());
            return Err(StorytellerError::Inference(format!(
                "Ollama returned {status}: {body}"
            )));
        }

        let ollama_response: OllamaChatResponse = response.json().await.map_err(|e| {
            StorytellerError::Inference(format!("failed to parse Ollama response: {e}"))
        })?;

        let tokens_used = ollama_response.eval_count + ollama_response.prompt_eval_count;

        tracing::debug!(
            tokens_used,
            content_len = ollama_response.message.content.len(),
            "structured Ollama response received"
        );

        extract_json_from_response(&ollama_response.message.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_builds_from_config() {
        let config = StructuredLlmConfig::default();
        let provider = OllamaStructuredProvider::new(config);
        assert_eq!(provider.config.model, "qwen2.5:3b-instruct");
    }

    #[test]
    fn provider_implements_debug() {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let debug = format!("{provider:?}");
        assert!(debug.contains("OllamaStructuredProvider"));
    }

    #[test]
    fn parse_llm_json_extracts_from_markdown_fences() {
        let raw = "```json\n{\"events\": []}\n```";
        let parsed = extract_json_from_response(raw).unwrap();
        assert_eq!(parsed, serde_json::json!({"events": []}));
    }

    #[test]
    fn parse_llm_json_handles_bare_json() {
        let raw = "{\"events\": []}";
        let parsed = extract_json_from_response(raw).unwrap();
        assert_eq!(parsed, serde_json::json!({"events": []}));
    }

    #[test]
    fn parse_llm_json_handles_text_around_json() {
        let raw = "Here is the result:\n{\"events\": []}\nDone.";
        let parsed = extract_json_from_response(raw).unwrap();
        assert_eq!(parsed, serde_json::json!({"events": []}));
    }

    #[cfg(feature = "test-llm")]
    #[tokio::test]
    async fn ollama_structured_extraction() {
        use storyteller_core::traits::structured_llm::StructuredRequest;

        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let request = StructuredRequest {
            system: "Return a JSON object with a single key 'greeting' containing 'hello'."
                .to_string(),
            input: "Say hello.".to_string(),
            output_schema: serde_json::json!({"type": "object", "properties": {"greeting": {"type": "string"}}}),
            temperature: 0.1,
        };
        let result = provider.extract(request).await;
        assert!(result.is_ok(), "Structured extraction failed: {result:?}");
        let json = result.unwrap();
        assert!(
            json.get("greeting").is_some(),
            "Missing 'greeting' key: {json}"
        );
    }
}
