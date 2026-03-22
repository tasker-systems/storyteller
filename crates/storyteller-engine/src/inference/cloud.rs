// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Cloud LLM provider implementation.
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Implements `LlmProvider` for cloud APIs (Claude, GPT, etc.) via reqwest.
//! Deferred — the prototype uses `ExternalServerProvider` (Ollama) instead.

use storyteller_core::traits::llm::{CompletionRequest, CompletionResponse, LlmProvider};

/// Cloud-based LLM provider using HTTP APIs.
#[derive(Debug)]
pub struct CloudLlmProvider {
    _client: reqwest::Client,
}

#[async_trait::async_trait]
impl LlmProvider for CloudLlmProvider {
    async fn complete(
        &self,
        _request: CompletionRequest,
    ) -> storyteller_core::StorytellerResult<CompletionResponse> {
        todo!("Cloud LLM provider not yet implemented")
    }
}
