//! LLM provider abstraction.
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Design decision: Agents don't know which LLM provider is active.
//! The `LlmProvider` trait abstracts over cloud APIs (Claude, GPT),
//! local inference (candle), and external servers (Ollama).

use crate::errors::StorytellerResult;

/// A request to an LLM for text completion.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// System prompt establishing the agent's role and constraints.
    pub system_prompt: String,
    /// The user/agent message to respond to.
    pub messages: Vec<Message>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
}

/// A single message in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    /// Role of the message sender.
    pub role: MessageRole,
    /// Content of the message.
    pub content: String,
}

/// Role in the LLM conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// System instruction.
    System,
    /// User or agent input.
    User,
    /// LLM response.
    Assistant,
}

/// Response from an LLM provider.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// Generated text content.
    pub content: String,
    /// Number of tokens used in the response.
    pub tokens_used: u32,
}

/// Abstraction over LLM backends.
///
/// Implementations: `CloudLlmProvider`, `CandleLlmProvider`, `ExternalServerProvider`.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync + std::fmt::Debug {
    /// Send a completion request and receive a response.
    async fn complete(&self, request: CompletionRequest) -> StorytellerResult<CompletionResponse>;
}
