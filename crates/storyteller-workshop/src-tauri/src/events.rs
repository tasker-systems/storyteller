//! Debug inspector events — single tagged enum emitted on `"workshop:debug"`.
//!
//! All events share a consistent envelope: a `type` discriminator field (from the
//! serde tag) plus a `turn` field, with phase-specific payload fields alongside.
//! The frontend listens to one Tauri event and dispatches on `type`.

use serde::{Deserialize, Serialize};
use storyteller_core::types::character::CharacterSheet;
use storyteller_core::types::resolver::ResolverOutput;

/// The single event channel name used for all debug inspector events.
pub const DEBUG_EVENT_CHANNEL: &str = "workshop:debug";

/// All debug inspector events emitted during turn processing.
///
/// Serializes as `{ "type": "phase_started", "turn": 1, "phase": "prediction" }` etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebugEvent {
    /// A pipeline phase has started processing.
    #[serde(rename = "phase_started")]
    PhaseStarted { turn: u32, phase: String },

    /// ML character predictions completed.
    #[serde(rename = "prediction_complete")]
    PredictionComplete {
        turn: u32,
        resolver_output: ResolverOutput,
        timing_ms: u64,
        model_loaded: bool,
    },

    /// Context assembly completed — the three tiers rendered as text.
    #[serde(rename = "context_assembled")]
    ContextAssembled {
        turn: u32,
        preamble_text: String,
        journal_text: String,
        retrieved_text: String,
        token_counts: TokenCounts,
        timing_ms: u64,
    },

    /// Character data for the turn.
    #[serde(rename = "characters_updated")]
    CharactersUpdated {
        turn: u32,
        characters: Vec<CharacterSheet>,
        emotional_markers: Vec<String>,
    },

    /// Event classification results.
    #[serde(rename = "events_classified")]
    EventsClassified {
        turn: u32,
        classifications: Vec<String>,
        classifier_loaded: bool,
    },

    /// Narrator LLM call completed — raw prompt and response.
    #[serde(rename = "narrator_complete")]
    NarratorComplete {
        turn: u32,
        system_prompt: String,
        user_message: String,
        raw_response: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        tokens_used: u32,
        timing_ms: u64,
    },

    /// An error occurred during a pipeline phase.
    #[serde(rename = "error")]
    Error {
        turn: u32,
        phase: String,
        message: String,
    },
}

/// Token counts per context assembly tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    pub preamble: u32,
    pub journal: u32,
    pub retrieved: u32,
    pub total: u32,
}
