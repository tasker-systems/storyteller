//! Shared engine providers — LLM and ML resources shared across sessions.
//!
//! `EngineProviders` is constructed once at server startup and stored in
//! `Arc`. It is stateless (no per-session data), so concurrent sessions
//! share the same provider instances without contention.

use std::sync::Arc;

use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::structured_llm::StructuredLlmProvider;

/// Shared providers for LLM and ML inference.
///
/// These are stateless and safe to share across concurrent sessions.
/// Constructed once at server startup.
///
/// ## Provider roles
///
/// - `narrator_llm` — capable model (e.g. llama3.1:8b) used for prose generation
/// - `structured_llm` — small fast model (e.g. qwen2.5:3b-instruct) for JSON extraction
/// - `intent_llm` — small model for NPC intent synthesis (3b-instruct via Ollama)
/// - `predictor_available` — whether the ONNX character predictor is loaded
///
/// `CharacterPredictor` and `PlutchikWestern` will be added when the turn
/// pipeline is wired in Tasks 12+.
#[derive(Clone)]
pub struct EngineProviders {
    pub narrator_llm: Arc<dyn LlmProvider>,
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    pub predictor_available: bool,
}

impl std::fmt::Debug for EngineProviders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineProviders")
            .field("narrator_llm", &"Arc<dyn LlmProvider>")
            .field("structured_llm", &self.structured_llm.is_some())
            .field("intent_llm", &self.intent_llm.is_some())
            .field("predictor_available", &self.predictor_available)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::{
        errors::StorytellerResult,
        traits::llm::{CompletionRequest, CompletionResponse},
    };

    #[derive(Debug)]
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(&self, _req: CompletionRequest) -> StorytellerResult<CompletionResponse> {
            Ok(CompletionResponse {
                content: "mock response".to_string(),
                tokens_used: 10,
            })
        }
    }

    #[test]
    fn engine_providers_debug_does_not_expose_internals() {
        let providers = EngineProviders {
            narrator_llm: Arc::new(MockLlm),
            structured_llm: None,
            intent_llm: None,
            predictor_available: false,
        };

        let debug_str = format!("{providers:?}");
        assert!(debug_str.contains("narrator_llm"));
        assert!(debug_str.contains("structured_llm: false"));
        assert!(debug_str.contains("intent_llm: false"));
        assert!(debug_str.contains("predictor_available: false"));
        // Should not expose raw pointer addresses or internal state
        assert!(!debug_str.contains("0x"));
    }

    #[test]
    fn engine_providers_clone_shares_arc() {
        let providers = EngineProviders {
            narrator_llm: Arc::new(MockLlm),
            structured_llm: None,
            intent_llm: None,
            predictor_available: true,
        };

        let cloned = providers.clone();
        // Same Arc pointer
        assert!(Arc::ptr_eq(&providers.narrator_llm, &cloned.narrator_llm));
        assert_eq!(providers.predictor_available, cloned.predictor_available);
    }
}
