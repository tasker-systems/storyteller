//! Engine state — holds all scene data and services for a running session.

use std::sync::Arc;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::structured_llm::StructuredLlmProvider;
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_engine::inference::event_classifier::EventClassifier;
use storyteller_engine::inference::frame::CharacterPredictor;

use crate::session_log::SessionLog;

/// All mutable and immutable state for a running workshop session.
///
/// Held inside `tokio::sync::Mutex<Option<EngineState>>` as Tauri managed state.
/// `None` means no scene is loaded yet.
#[derive(Debug)]
pub struct EngineState {
    /// The loaded scene definition.
    pub scene: SceneData,
    /// Character sheets for the scene's cast.
    pub characters: Vec<CharacterSheet>,
    /// Rolling compressed journal of the scene so far.
    pub journal: SceneJournal,
    /// The LLM provider (Ollama).
    pub llm: Arc<dyn LlmProvider>,
    /// ML character predictor (optional — graceful fallback).
    pub predictor: Option<CharacterPredictor>,
    /// ML event classifier (optional — graceful fallback).
    pub event_classifier: Option<EventClassifier>,
    /// Structured LLM provider for event decomposition (optional).
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    /// Emotional grammar for ML predictions.
    pub grammar: PlutchikWestern,
    /// Session log for JSONL recording.
    pub session_log: SessionLog,
    /// Current turn number.
    pub turn_count: u32,
    /// Session ID for persisted sessions (None for classic/non-persisted scenes).
    pub session_id: Option<String>,
}
