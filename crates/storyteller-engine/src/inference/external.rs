//! External server LLM provider (e.g., Ollama).
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Implements `LlmProvider` for external inference servers running locally
//! or on the network. The primary LLM path for the prototype — communicates
//! via HTTP to Ollama's `/api/chat` endpoint.

use std::time::Duration;

use storyteller_core::errors::StorytellerError;
use storyteller_core::traits::llm::{
    CompletionRequest, CompletionResponse, LlmProvider, Message, MessageRole,
};
use tracing::instrument;

/// Configuration for an external LLM server.
#[derive(Debug, Clone)]
pub struct ExternalServerConfig {
    /// Base URL of the server (e.g., "http://localhost:11434").
    pub base_url: String,
    /// Model name to use (e.g., "mistral", "llama3.1").
    pub model: String,
    /// Request timeout.
    pub timeout: Duration,
}

impl Default for ExternalServerConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "qwen2.5:14b".to_string(),
            timeout: Duration::from_secs(120),
        }
    }
}

/// LLM provider that calls an external server (Ollama, vLLM, etc.).
#[derive(Debug)]
pub struct ExternalServerProvider {
    config: ExternalServerConfig,
    client: reqwest::Client,
}

impl ExternalServerProvider {
    /// Create a new provider with the given configuration.
    pub fn new(config: ExternalServerConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("failed to build HTTP client");
        Self { config, client }
    }

    /// Create a provider pointing at a local Ollama instance with the given model.
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new(ExternalServerConfig {
            model: model.into(),
            ..Default::default()
        })
    }
}

// -- Ollama API types (private, just for serialization) -----------------------

#[derive(Debug, serde::Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
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
    num_predict: u32,
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

// -- LlmProvider implementation -----------------------------------------------

#[async_trait::async_trait]
impl LlmProvider for ExternalServerProvider {
    #[instrument(skip(self, request), fields(model = %self.config.model))]
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> storyteller_core::StorytellerResult<CompletionResponse> {
        let url = format!("{}/api/chat", self.config.base_url);

        // Build Ollama messages: system prompt first, then conversation history.
        let mut messages = Vec::with_capacity(request.messages.len() + 1);
        messages.push(OllamaMessage {
            role: "system".to_string(),
            content: request.system_prompt,
        });
        for msg in &request.messages {
            messages.push(OllamaMessage {
                role: match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                }
                .to_string(),
                content: msg.content.clone(),
            });
        }

        let ollama_request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        tracing::debug!("sending request to Ollama: {url}");

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| StorytellerError::Llm(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "failed to read body".to_string());
            return Err(StorytellerError::Llm(format!(
                "Ollama returned {status}: {body}"
            )));
        }

        let ollama_response: OllamaChatResponse = response
            .json()
            .await
            .map_err(|e| StorytellerError::Llm(format!("failed to parse Ollama response: {e}")))?;

        let tokens_used = ollama_response.eval_count + ollama_response.prompt_eval_count;

        tracing::debug!(
            tokens_used,
            content_len = ollama_response.message.content.len(),
            "Ollama response received"
        );

        Ok(CompletionResponse {
            content: ollama_response.message.content,
            tokens_used,
        })
    }
}

// -- Convenience for building requests ----------------------------------------

/// Build a single-turn completion request.
pub fn single_turn_request(
    system_prompt: impl Into<String>,
    user_message: impl Into<String>,
    max_tokens: u32,
    temperature: f32,
) -> CompletionRequest {
    CompletionRequest {
        system_prompt: system_prompt.into(),
        messages: vec![Message {
            role: MessageRole::User,
            content: user_message.into(),
        }],
        max_tokens,
        temperature,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_points_to_localhost() {
        let config = ExternalServerConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "qwen2.5:14b");
    }

    #[test]
    fn ollama_constructor_uses_custom_model() {
        let provider = ExternalServerProvider::ollama("llama3.1");
        assert_eq!(provider.config.model, "llama3.1");
        assert_eq!(provider.config.base_url, "http://localhost:11434");
    }

    #[test]
    fn single_turn_request_builds_correctly() {
        let req = single_turn_request("You are a narrator.", "What happens next?", 500, 0.7);
        assert_eq!(req.system_prompt, "You are a narrator.");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, MessageRole::User);
        assert_eq!(req.max_tokens, 500);
    }

    // Integration test — requires a running Ollama instance.
    // Enable with: cargo test --features test-llm
    #[cfg(feature = "test-llm")]
    #[tokio::test]
    async fn ollama_integration() {
        let provider = ExternalServerProvider::ollama("mistral");
        let request = single_turn_request(
            "You are a helpful assistant. Respond in one sentence.",
            "What color is the sky?",
            100,
            0.3,
        );
        let response = provider.complete(request).await;
        assert!(response.is_ok(), "Ollama call failed: {response:?}");
        let response = response.unwrap();
        assert!(!response.content.is_empty(), "Empty response from Ollama");
        println!("Ollama response: {}", response.content);
        println!("Tokens used: {}", response.tokens_used);
    }
}
