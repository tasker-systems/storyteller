//! Agent message types — inter-agent communication for the turn cycle.
//!
//! See: `docs/technical/agent-message-catalog.md`, `docs/technical/narrator-architecture.md`
//!
//! These are the typed messages that flow between stages during a single turn.
//! The narrator-centric architecture pipeline:
//! player input → event classification → character prediction (ML) →
//! action resolution (rules engine) → context assembly → narrator rendering.
//!
//! Legacy types (`StorykeeperDirective`, `CharacterIntent`, `ReconcilerOutput`)
//! are retained for the existing multi-agent prototype code path. New code
//! should use `CharacterPrediction` (prediction.rs) and `ResolverOutput`
//! (resolver.rs) instead.

use super::entity::EntityId;

/// Raw player input — the starting point for every turn cycle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerInput {
    /// What the player typed / said.
    pub text: String,
    /// Turn number within the current scene.
    pub turn_number: u32,
}

// ---------------------------------------------------------------------------
// Legacy multi-agent types — retained for existing prototype code path
// ---------------------------------------------------------------------------

/// Storykeeper's filtered context for a single character agent.
///
/// **Legacy**: In the narrator-centric architecture, the Storykeeper becomes a
/// context assembly system that builds `NarratorContextInput` rather than
/// producing per-character directives. Retained for the existing prototype.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorykeeperDirective {
    /// Which character this directive is for.
    pub character_id: EntityId,
    /// The scene context this character is allowed to perceive.
    pub visible_context: String,
    /// The player's input as this character would perceive it.
    pub filtered_input: String,
    /// Any specific guidance from the Storykeeper about what this
    /// character should or shouldn't react to.
    pub guidance: String,
}

/// A character agent's intended action / response.
///
/// **Legacy**: In the narrator-centric architecture, character behavior is
/// predicted by ML models producing `CharacterPrediction` (see prediction.rs).
/// Retained for the existing prototype code path.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterIntent {
    /// Which character produced this intent.
    pub character_id: EntityId,
    /// The character's name (for rendering convenience).
    pub character_name: String,
    /// What the character intends to do or say.
    pub intent: String,
    /// The emotional subtext — what's happening beneath the surface.
    pub emotional_subtext: String,
    /// Internal state notes — private reasoning not visible to other agents.
    pub internal_state: String,
}

/// Reconciler's output — multiple character intents sequenced and harmonized.
///
/// **Legacy**: In the narrator-centric architecture, the Reconciler is replaced
/// by the deterministic Resolver producing `ResolverOutput` (see resolver.rs).
/// Retained for the existing prototype code path.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReconcilerOutput {
    /// Character intents in their final sequence.
    pub sequenced_intents: Vec<CharacterIntent>,
    /// Notes on scene dynamics — what the reconciler observed about
    /// the interaction between characters.
    pub scene_dynamics: String,
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
