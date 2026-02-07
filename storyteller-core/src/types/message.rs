//! Agent message types — inter-agent communication for the turn cycle.
//!
//! See: `docs/technical/agent-message-catalog.md`
//!
//! These are the typed messages that flow between agents during a single turn.
//! Each message represents a specific handoff in the pipeline:
//! player input → storykeeper filtering → character deliberation →
//! reconciliation → narrator rendering.

use super::entity::EntityId;

/// Raw player input — the starting point for every turn cycle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerInput {
    /// What the player typed / said.
    pub text: String,
    /// Turn number within the current scene.
    pub turn_number: u32,
}

/// Storykeeper's filtered context for a single character agent.
///
/// The Storykeeper sees the full scene state and player input, then
/// produces one of these for each character in the scene — tailored
/// to that character's information boundary.
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
/// This is what the character *wants* to do — not yet rendered in
/// story voice. The Narrator takes these intents and weaves them
/// into narrative prose.
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
/// For a two-character scene this is relatively simple. For larger casts,
/// the reconciler resolves temporal overlaps, action conflicts, and
/// surfaces dramatic potential.
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
/// These are deliberately abstract — the player-facing layer can translate
/// them into thematic language, while the system layer uses them for
/// tracing spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TurnPhaseKind {
    /// Player input received and persisted.
    InputReceived,
    /// Classifier agents processing input.
    Classifying,
    /// Storykeeper evaluating full state, producing directives.
    StorykeeperEvaluating,
    /// Character agents deliberating (parallel).
    CharactersDeliberating,
    /// Reconciler sequencing character intents.
    Reconciling,
    /// Narrator rendering final output.
    NarratorRendering,
    /// Turn complete, post-turn reconciliation done.
    Complete,
}
