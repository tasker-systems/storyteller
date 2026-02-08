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

/// Turn cycle phase transitions.
///
/// Emitted at each pipeline stage boundary. All three observability layers
/// (system tracing, session debug, player progress) observe these same events,
/// differing only in filtering and formatting.
///
/// See: `docs/technical/infrastructure-architecture.md` § Observability
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnPhase {
    /// Which turn this belongs to.
    pub turn_number: u32,
    /// The phase we just entered.
    pub phase: TurnPhaseKind,
    /// Milliseconds elapsed since the turn started.
    pub elapsed_ms: u64,
    /// Optional detail for debug/session observability.
    pub detail: Option<String>,
}

/// The phases of a single turn cycle.
///
/// Reflects the narrator-centric architecture pipeline:
/// input → classification → prediction → resolution → context assembly → rendering.
///
/// These are deliberately abstract — the player-facing layer can translate
/// them into thematic language, while the system layer uses them for
/// tracing spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TurnPhaseKind {
    /// Player input received and persisted to the event ledger.
    InputReceived,
    /// Event classifier processing raw input into typed events.
    Classifying,
    /// ML models predicting character behavior (parallel across cast).
    CharacterPrediction,
    /// Resolver sequencing predictions, enforcing world constraints.
    Resolving,
    /// Storykeeper assembling three-tier context for the Narrator.
    ContextAssembly,
    /// Narrator rendering final output from assembled context.
    Rendering,
    /// Turn complete, post-turn processing done.
    Complete,
}
