//! Engine state — holds all scene data and services for a running session.

use std::sync::Arc;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::structured_llm::StructuredLlmProvider;
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_engine::inference::frame::CharacterPredictor;
use storyteller_engine::inference::intention_generation::GeneratedIntentions;
use storyteller_engine::scene_composer::ComposedGoals;
use storyteller_ml::prediction_history::PredictionHistory;

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
    /// Structured LLM provider for event decomposition (optional).
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    /// Intent synthesis LLM provider — plain completion, same 3b model (optional).
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    /// Emotional grammar for ML predictions.
    pub grammar: PlutchikWestern,
    /// Session log for JSONL recording.
    pub session_log: SessionLog,
    /// Current turn number.
    pub turn_count: u32,
    /// Session ID for persisted sessions (None for classic/non-persisted scenes).
    pub session_id: Option<String>,
    /// Entity ID of the player-controlled character (None if not identified).
    pub player_entity_id: Option<EntityId>,
    /// Accumulated prediction history for turn-over-turn ML context.
    pub prediction_history: PredictionHistory,
    /// Composition-time intentions (scene direction + character drives).
    /// Persists across turns for preamble injection and intent synthesis context.
    pub generated_intentions: Option<GeneratedIntentions>,
    /// Composed scene/character goals for player context re-derivation.
    pub composed_goals: Option<ComposedGoals>,
}
