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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_event_channel_matches_frontend_constant() {
        assert_eq!(DEBUG_EVENT_CHANNEL, "workshop:debug");
    }

    #[test]
    fn phase_started_serializes_with_type_tag() {
        let event = DebugEvent::PhaseStarted {
            turn: 1,
            phase: "prediction".to_string(),
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "phase_started");
        assert_eq!(json["turn"], 1);
        assert_eq!(json["phase"], "prediction");
    }

    #[test]
    fn error_event_serializes_with_type_tag() {
        let event = DebugEvent::Error {
            turn: 3,
            phase: "narrator".to_string(),
            message: "LLM timeout".to_string(),
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "error");
        assert_eq!(json["turn"], 3);
        assert_eq!(json["phase"], "narrator");
        assert_eq!(json["message"], "LLM timeout");
    }

    #[test]
    fn context_assembled_serializes_token_counts() {
        let event = DebugEvent::ContextAssembled {
            turn: 2,
            preamble_text: "narrator text".to_string(),
            journal_text: String::new(),
            retrieved_text: String::new(),
            token_counts: TokenCounts {
                preamble: 450,
                journal: 300,
                retrieved: 200,
                total: 950,
            },
            timing_ms: 12,
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "context_assembled");
        assert_eq!(json["token_counts"]["preamble"], 450);
        assert_eq!(json["token_counts"]["total"], 950);
    }

    #[test]
    fn narrator_complete_serializes_all_fields() {
        let event = DebugEvent::NarratorComplete {
            turn: 1,
            system_prompt: "You are a narrator.".to_string(),
            user_message: "I look around.".to_string(),
            raw_response: "The room is dimly lit.".to_string(),
            model: "qwen2.5:14b".to_string(),
            temperature: 0.8,
            max_tokens: 400,
            tokens_used: 120,
            timing_ms: 2500,
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "narrator_complete");
        assert_eq!(json["model"], "qwen2.5:14b");
        // f32 → f64 conversion introduces precision noise; check approximate equality
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.8).abs() < 0.001, "temperature was {temp}");
        assert_eq!(json["timing_ms"], 2500);
    }

    #[test]
    fn phase_started_round_trips_through_json() {
        let original = DebugEvent::PhaseStarted {
            turn: 5,
            phase: "context".to_string(),
        };
        let json_str = serde_json::to_string(&original).expect("serialize");
        let restored: DebugEvent = serde_json::from_str(&json_str).expect("deserialize");

        match restored {
            DebugEvent::PhaseStarted { turn, phase } => {
                assert_eq!(turn, 5);
                assert_eq!(phase, "context");
            }
            _ => panic!("expected PhaseStarted variant"),
        }
    }

    #[test]
    fn token_counts_serializes_correctly() {
        let counts = TokenCounts {
            preamble: 600,
            journal: 800,
            retrieved: 400,
            total: 1800,
        };
        let json = serde_json::to_value(&counts).expect("serialize");
        assert_eq!(json["preamble"], 600);
        assert_eq!(json["journal"], 800);
        assert_eq!(json["retrieved"], 400);
        assert_eq!(json["total"], 1800);
    }
}
