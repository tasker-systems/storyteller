//! Shared traits used across the storyteller workspace.

pub mod emotional_grammar;
pub mod game_design;
pub mod llm;
pub mod phase_observer;

pub use emotional_grammar::EmotionalGrammar;
pub use game_design::GameDesignSystem;
pub use llm::LlmProvider;
pub use phase_observer::{CollectingObserver, NoopObserver, PhaseObserver};
