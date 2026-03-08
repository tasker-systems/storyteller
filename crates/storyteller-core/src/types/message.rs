//! Agent message types — inter-agent communication for the turn cycle.
//!
//! See: `docs/technical/agent-message-catalog.md`, `docs/technical/narrator-architecture.md`
//!
//! These are the typed messages that flow between stages during a single turn.
//! The narrator-centric architecture pipeline:
//! player input → event classification → character prediction (ML) →
//! action resolution (rules engine) → context assembly → narrator rendering.
//!
//! Character behavior predictions are in `prediction.rs`.
//! Resolver output is in `resolver.rs`.
//! Narrator context assembly is in `narrator_context.rs`.

/// Raw player input — the starting point for every turn cycle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerInput {
    /// What the player typed / said.
    pub text: String,
    /// Turn number within the current scene.
    pub turn_number: u32,
}

/// The narrator's final rendered output — what the player reads.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarratorRendering {
    /// The narrative prose the player sees.
    pub text: String,
    /// Optional stage directions / internal notes for logging.
    pub stage_directions: Option<String>,
}

// ---------------------------------------------------------------------------
// Turn phase observability — the single source of truth for all three layers
// ---------------------------------------------------------------------------

/// Turn cycle stage transitions.
///
/// Emitted at each pipeline stage boundary. All three observability layers
/// (system tracing, session debug, player progress) observe these same events,
/// differing only in filtering and formatting.
///
/// Uses `TurnCycleStage` directly — there is a single lifecycle concept
/// for pipeline orchestration and observability.
///
/// See: `docs/technical/infrastructure-architecture.md` § Observability
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnPhase {
    /// Which turn this belongs to.
    pub turn_number: u32,
    /// The pipeline stage we just entered.
    pub stage: super::turn_cycle::TurnCycleStage,
    /// Milliseconds elapsed since the turn started.
    pub elapsed_ms: u64,
    /// Optional detail for debug/session observability.
    pub detail: Option<String>,
}
