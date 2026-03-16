//! Types for the Tauri ↔ Svelte boundary.
//!
//! Every struct here derives `ts_rs::TS` and generates a TypeScript interface
//! in `src/lib/generated/`. These are the source of truth — the generated `.ts`
//! files replace the hand-authored `types.ts` interfaces.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Scene information returned after composition.
/// `id` is the UUIDv7 `entity_id` — the canonical identifier for all
/// downstream RPC calls. The CLI uses slugs for human ergonomics, but the
/// workshop uses entity_ids directly since the UI provides selection, not typing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct SceneInfo {
    pub session_id: String,
    pub title: String,
    pub setting_description: String,
    pub cast: Vec<String>,
    pub opening_prose: String,
}

/// Result of a player turn.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct TurnResult {
    pub turn: u32,
    pub narrator_prose: String,
    pub timing: TurnTiming,
    pub context_tokens: ContextTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct TurnTiming {
    pub prediction_ms: u64,
    pub assembly_ms: u64,
    pub narrator_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct ContextTokens {
    pub preamble: u32,
    pub journal: u32,
    pub retrieved: u32,
    pub total: u32,
}

/// Health status enum — mirrors `storyteller_core::types::health::HealthStatus`
/// but with `TS` derive for TypeScript generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

impl From<storyteller_core::types::health::HealthStatus> for HealthStatus {
    fn from(s: storyteller_core::types::health::HealthStatus) -> Self {
        match s {
            storyteller_core::types::health::HealthStatus::Healthy => Self::Healthy,
            storyteller_core::types::health::HealthStatus::Degraded => Self::Degraded,
            storyteller_core::types::health::HealthStatus::Unavailable => Self::Unavailable,
        }
    }
}

/// Server health report.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct HealthReport {
    pub status: HealthStatus,
    pub subsystems: Vec<SubsystemStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct SubsystemStatus {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
}

/// Genre summary for the wizard catalog.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct GenreSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub archetype_count: u32,
    pub profile_count: u32,
    pub dynamic_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct ArchetypeSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub scene_type: String,
    pub tension_min: f64,
    pub tension_max: f64,
    pub cast_size_min: u32,
    pub cast_size_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct DynamicSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct SettingSummary {
    pub id: String,
    pub name: String,
}

/// Session summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct SessionInfo {
    pub session_id: String,
    pub genre: String,
    pub profile: String,
    pub title: String,
    pub cast_names: Vec<String>,
    pub turn_count: u32,
    pub created_at: String,
}

/// Turn summary for session resume.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct TurnSummary {
    pub turn: u32,
    pub player_input: Option<String>,
    pub narrator_output: String,
}

/// Resume result with scene info and turn history.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct ResumeResult {
    pub scene_info: SceneInfo,
    pub turns: Vec<TurnSummary>,
}

/// Scene selections for composition (received from Svelte wizard).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct SceneSelections {
    pub genre_id: String,
    pub profile_id: String,
    pub cast: Vec<CastSelection>,
    pub dynamics: Vec<DynamicSelection>,
    pub setting_override: Option<String>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct CastSelection {
    pub archetype_id: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct DynamicSelection {
    pub dynamic_id: String,
    pub cast_index_a: u32,
    pub cast_index_b: u32,
}

/// Debug events emitted on the "workshop:debug" Tauri event channel.
///
/// Each variant is translated from an `EngineEvent` proto payload in the
/// streaming command handlers.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "../../src/lib/generated/")]
pub enum DebugEvent {
    #[serde(rename = "phase_started")]
    PhaseStarted { turn: u32, phase: String },

    #[serde(rename = "prediction_complete")]
    PredictionComplete {
        turn: u32,
        raw_json: String,
        timing_ms: u64,
        model_loaded: bool,
    },

    #[serde(rename = "context_assembled")]
    ContextAssembled {
        turn: u32,
        preamble_text: String,
        journal_text: String,
        retrieved_text: String,
        token_counts: ContextTokens,
        timing_ms: u64,
    },

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

    #[serde(rename = "event_decomposed")]
    EventDecomposed {
        turn: u32,
        raw_json: Option<String>,
        timing_ms: u64,
        model: String,
        error: Option<String>,
    },

    #[serde(rename = "intent_synthesized")]
    IntentSynthesized {
        turn: u32,
        intent_statements: String,
        timing_ms: u64,
    },

    #[serde(rename = "action_arbitrated")]
    ActionArbitrated {
        turn: u32,
        verdict: String,
        details: String,
        player_input: String,
        timing_ms: u64,
    },

    #[serde(rename = "goals_generated")]
    GoalsGenerated {
        turn: u32,
        scene_goals: Vec<String>,
        character_goals: Vec<String>,
        scene_direction: Option<String>,
        character_drives: Vec<String>,
        player_context: Option<String>,
        timing_ms: u64,
    },

    #[serde(rename = "error")]
    Error {
        turn: u32,
        phase: String,
        message: String,
    },
}

/// Log entry from the server's tracing stream.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: std::collections::HashMap<String, String>,
}
