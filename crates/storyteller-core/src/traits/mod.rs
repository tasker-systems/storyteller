// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Shared traits used across the storyteller workspace.

pub mod emotional_grammar;
pub mod game_design;
pub mod llm;
pub mod phase_observer;
pub mod storykeeper;
pub mod structured_llm;

pub use emotional_grammar::EmotionalGrammar;
pub use game_design::GameDesignSystem;
pub use llm::{narrator_token_channel, LlmProvider, NarratorTokenSender, NarratorTokenStream};
pub use phase_observer::{CollectingObserver, NoopObserver, PhaseObserver};
pub use storykeeper::{Storykeeper, StorykeeperCommit, StorykeeperLifecycle, StorykeeperQuery};
pub use structured_llm::{StructuredLlmConfig, StructuredLlmProvider, StructuredRequest};
