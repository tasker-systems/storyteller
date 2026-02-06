//! Cloud LLM provider implementation.
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Implements `LlmProvider` for cloud APIs (Claude, GPT, etc.) via reqwest.
//! Feature-gated behind `cloud-llm` (enabled by default).

#[cfg(feature = "cloud-llm")]
use storyteller_core::traits::llm::{CompletionRequest, CompletionResponse, LlmProvider};

/// Cloud-based LLM provider using HTTP APIs.
#[cfg(feature = "cloud-llm")]
#[derive(Debug)]
pub struct CloudLlmProvider {
    _client: reqwest::Client,
}

#[cfg(feature = "cloud-llm")]
#[async_trait::async_trait]
impl LlmProvider for CloudLlmProvider {
    async fn complete(
        &self,
        _request: CompletionRequest,
    ) -> storyteller_core::StorytellerResult<CompletionResponse> {
        todo!("Cloud LLM provider not yet implemented")
    }
}
