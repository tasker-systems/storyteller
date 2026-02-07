//! Shared traits used across the storyteller workspace.

pub mod emotional_grammar;
pub mod llm;

pub use emotional_grammar::EmotionalGrammar;
pub use llm::LlmProvider;
