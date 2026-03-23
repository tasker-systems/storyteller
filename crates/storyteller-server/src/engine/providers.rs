// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Shared engine providers — LLM and ML resources shared across sessions.
//!
//! `EngineProviders` is constructed once at server startup and stored in
//! `Arc`. It is stateless (no per-session data), so concurrent sessions
//! share the same provider instances without contention.

use std::sync::Arc;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::structured_llm::StructuredLlmProvider;
use storyteller_engine::inference::frame::CharacterPredictor;

/// Shared providers for LLM and ML inference.
///
/// These are stateless and safe to share across concurrent sessions.
/// Constructed once at server startup.
///
/// ## Provider roles
///
/// - `narrator_llm` — capable model (e.g. qwen2.5:14b) used for prose generation
/// - `structured_llm` — small fast model (e.g. qwen2.5:3b-instruct) for JSON extraction
/// - `intent_llm` — small model for NPC intent synthesis (3b-instruct via Ollama)
/// - `predictor` — optional ONNX character predictor (absent if model not on disk)
/// - `grammar` — emotion grammar used by prediction enrichment
#[derive(Clone)]
pub struct EngineProviders {
    pub narrator_llm: Arc<dyn LlmProvider>,
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    pub predictor: Option<Arc<CharacterPredictor>>,
    pub grammar: Arc<PlutchikWestern>,
    /// Model name for the narrator LLM (for observability/debug inspector).
    pub narrator_model: String,
    /// Model name for the decomposition LLM (for observability/debug inspector).
    pub decomposition_model: String,
}

impl std::fmt::Debug for EngineProviders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineProviders")
            .field("narrator_llm", &"Arc<dyn LlmProvider>")
            .field("structured_llm", &self.structured_llm.is_some())
            .field("intent_llm", &self.intent_llm.is_some())
            .field("predictor", &self.predictor.is_some())
            .field("narrator_model", &self.narrator_model)
            .field("decomposition_model", &self.decomposition_model)
            .finish()
    }
}

/// Convenience accessor — consistent with the old `predictor_available` field.
pub fn predictor_available(providers: &EngineProviders) -> bool {
    providers.predictor.is_some()
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
            predictor: None,
            grammar: Arc::new(PlutchikWestern::new()),
            narrator_model: "test-model".to_string(),
            decomposition_model: "test-decomp-model".to_string(),
        };

        let debug_str = format!("{providers:?}");
        assert!(debug_str.contains("narrator_llm"));
        assert!(debug_str.contains("structured_llm: false"));
        assert!(debug_str.contains("intent_llm: false"));
        assert!(debug_str.contains("predictor: false"));
        // Should not expose raw pointer addresses or internal state
        assert!(!debug_str.contains("0x"));
    }

    #[test]
    fn engine_providers_clone_shares_arc() {
        let providers = EngineProviders {
            narrator_llm: Arc::new(MockLlm),
            structured_llm: None,
            intent_llm: None,
            predictor: None,
            grammar: Arc::new(PlutchikWestern::new()),
            narrator_model: "test-model".to_string(),
            decomposition_model: "test-decomp-model".to_string(),
        };

        let cloned = providers.clone();
        // Same Arc pointer
        assert!(Arc::ptr_eq(&providers.narrator_llm, &cloned.narrator_llm));
        assert!(Arc::ptr_eq(&providers.grammar, &cloned.grammar));
    }
}
