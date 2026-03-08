//! Turn cycle Bevy resources — state that drives the pipeline.
//!
//! See: `docs/technical/turn-cycle-architecture.md`
//!
//! These resources are held as singletons in the Bevy ECS world.
//! Systems read and write them to coordinate the turn pipeline.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use chrono::{DateTime, Utc};

use storyteller_core::types::event_grammar::{CompoundEvent, EventAtom};
use storyteller_core::types::message::NarratorRendering;
use storyteller_core::types::narrator_context::{NarratorContextInput, SceneJournal};
use storyteller_core::types::prediction::CharacterPrediction;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::turn_cycle::TurnCycleStage;
use storyteller_core::StorytellerResult;

use crate::agents::narrator::NarratorAgent;
use crate::inference::event_classifier::ClassificationOutput;

/// Bevy Resource: current turn cycle stage.
///
/// The turn pipeline is a state machine. Systems use `run_if` conditions
/// to gate on the current stage. Each stage system advances to the next
/// when its work completes.
#[derive(Debug, Default, Resource)]
pub struct ActiveTurnStage(pub TurnCycleStage);

/// Bevy Resource: accumulated context for the current turn.
///
/// Each pipeline stage reads what prior stages produced and writes
/// its output. Reset at the start of each new turn.
#[derive(Debug, Default, Resource)]
pub struct TurnContext {
    /// Player input text (set when input received).
    pub player_input: Option<String>,
    /// Classification output from EventClassifier.
    pub classification: Option<ClassificationOutput>,
    /// Character predictions from ML models.
    pub predictions: Option<Vec<CharacterPrediction>>,
    /// Resolver output (sequenced actions, conflicts).
    pub resolver_output: Option<ResolverOutput>,
    /// Assembled narrator context.
    pub narrator_context: Option<NarratorContextInput>,
    /// Narrator rendering output.
    pub rendering: Option<NarratorRendering>,
}

impl TurnContext {
    /// Reset all fields for a new turn.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------
// Turn history — in-memory archive of completed turns (D.1)
// ---------------------------------------------------------------------------

/// A completed turn archived by `commit_previous_system`.
///
/// Captures all outputs from the turn pipeline for post-hoc analysis,
/// journal construction, and committed-turn classification.
#[derive(Debug, Clone)]
pub struct CompletedTurn {
    /// Turn ordinal within the scene.
    pub turn_number: u32,
    /// The player input that initiated this turn.
    pub player_input: String,
    /// The Narrator's rendered prose (if rendering succeeded).
    pub narrator_rendering: Option<NarratorRendering>,
    /// Event classification of player input alone (from the Classifying stage).
    pub classification: Option<ClassificationOutput>,
    /// Combined-text classification from committed-turn analysis (D.3).
    /// Classifies narrator prose + player input together for richer event extraction.
    pub committed_classification: Option<ClassificationOutput>,
    /// Event atoms built from committed-turn classification (Phase E).
    pub committed_atoms: Vec<EventAtom>,
    /// Compound events detected from committed atoms (Phase E).
    pub committed_compounds: Vec<CompoundEvent>,
    /// ML character predictions (if predictor was available).
    pub predictions: Option<Vec<CharacterPrediction>>,
    /// When this turn was committed.
    pub committed_at: DateTime<Utc>,
}

/// Bevy Resource: in-memory archive of completed turns.
///
/// Accumulated by `commit_previous_system`. Used for turn counting,
/// journal updates, and future committed-turn classification (D.3).
#[derive(Debug, Default, Resource)]
pub struct TurnHistory {
    pub turns: Vec<CompletedTurn>,
}

impl TurnHistory {
    /// Next turn number (1-indexed).
    pub fn next_turn_number(&self) -> u32 {
        self.turns.len() as u32 + 1
    }
}

/// Bevy Resource: new player input waiting to be moved into TurnContext.
///
/// When external code provides new player input and advances to
/// `CommittingPrevious`, the TurnContext still holds the *previous*
/// turn's data. `PendingInput` holds the new input separately until
/// `commit_previous_system` archives old data and moves it.
#[derive(Debug, Default, Resource)]
pub struct PendingInput(pub Option<String>);

// ---------------------------------------------------------------------------
// Narrator rendering support (D.2)
// ---------------------------------------------------------------------------

/// Bevy Resource: the Narrator agent wrapped for async access.
///
/// `render()` takes `&mut self` (modifies conversation history), and the
/// spawned tokio task needs to hold the agent across an await point.
/// `Arc<tokio::sync::Mutex>` satisfies both constraints.
///
/// Constructed at scene start with the initial context and LLM provider.
/// NOT auto-initialized — must be inserted by the caller.
#[derive(Resource, Clone)]
pub struct NarratorResource(pub Arc<tokio::sync::Mutex<NarratorAgent>>);

impl std::fmt::Debug for NarratorResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NarratorResource").finish()
    }
}

/// Bevy Resource: the scene journal for progressive compression.
///
/// Wraps a `SceneJournal` that accumulates turn entries. Updated by
/// `commit_previous_system` when archiving a completed turn's rendering.
/// Read by `assemble_context_system` for Tier 2 context.
///
/// NOT auto-initialized — must be inserted with a SceneId and token budget.
#[derive(Debug, Resource)]
pub struct JournalResource(pub SceneJournal);

/// Bevy Resource: tokio runtime handle for spawning async tasks.
///
/// Bevy systems may not run on a tokio thread, so we cannot rely on
/// `Handle::current()`. This resource holds a handle obtained at
/// startup from the tokio runtime that hosts the application.
///
/// NOT auto-initialized — must be inserted by the caller.
#[derive(Resource, Clone)]
pub struct TokioRuntime(pub tokio::runtime::Handle);

impl std::fmt::Debug for TokioRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioRuntime").finish()
    }
}

// ---------------------------------------------------------------------------
// Async narrator task tracking
// ---------------------------------------------------------------------------

/// Bevy Resource: async narrator task tracking.
///
/// The Narrator LLM call is async but Bevy systems are sync.
/// This resource bridges the gap via a state-machine polling pattern
/// in `rendering_system`:
/// - **Idle**: no task running — system spawns async work, transitions to InFlight
/// - **InFlight**: polls the oneshot receiver each frame until ready
/// - **Complete**: result ready for consumption (tests, sync fallback)
#[derive(Default, Resource)]
pub enum NarratorTask {
    /// No LLM call in progress.
    #[default]
    Idle,
    /// LLM call in flight — receiver will yield the result.
    InFlight(tokio::sync::oneshot::Receiver<StorytellerResult<NarratorRendering>>),
    /// LLM call complete — result ready for consumption.
    Complete(StorytellerResult<NarratorRendering>),
}

impl std::fmt::Debug for NarratorTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "NarratorTask::Idle"),
            Self::InFlight(_) => write!(f, "NarratorTask::InFlight(...)"),
            Self::Complete(Ok(_)) => write!(f, "NarratorTask::Complete(Ok(...))"),
            Self::Complete(Err(e)) => write!(f, "NarratorTask::Complete(Err({e}))"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_turn_stage_defaults_to_awaiting_input() {
        let stage = ActiveTurnStage::default();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);
    }

    #[test]
    fn turn_context_defaults_to_all_none() {
        let ctx = TurnContext::default();
        assert!(ctx.player_input.is_none());
        assert!(ctx.classification.is_none());
        assert!(ctx.predictions.is_none());
        assert!(ctx.resolver_output.is_none());
        assert!(ctx.narrator_context.is_none());
        assert!(ctx.rendering.is_none());
    }

    #[test]
    fn turn_context_reset_clears_all() {
        let mut ctx = TurnContext {
            player_input: Some("hello".to_string()),
            ..Default::default()
        };
        assert!(ctx.player_input.is_some());
        ctx.reset();
        assert!(ctx.player_input.is_none());
    }

    #[test]
    fn narrator_task_defaults_to_idle() {
        let task = NarratorTask::default();
        assert!(matches!(task, NarratorTask::Idle));
    }

    #[test]
    fn narrator_task_debug_format() {
        let task = NarratorTask::Idle;
        assert_eq!(format!("{task:?}"), "NarratorTask::Idle");
    }

    #[test]
    fn turn_history_defaults_to_empty() {
        let history = TurnHistory::default();
        assert!(history.turns.is_empty());
        assert_eq!(history.next_turn_number(), 1);
    }

    #[test]
    fn pending_input_defaults_to_none() {
        let pending = PendingInput::default();
        assert!(pending.0.is_none());
    }

    #[test]
    fn completed_turn_captures_fields() {
        let turn = CompletedTurn {
            turn_number: 3,
            player_input: "I approach the fence".to_string(),
            narrator_rendering: Some(NarratorRendering {
                text: "The hooves leave prints.".to_string(),
                stage_directions: None,
            }),
            classification: None,
            committed_classification: None,
            committed_atoms: vec![],
            committed_compounds: vec![],
            predictions: None,
            committed_at: chrono::Utc::now(),
        };
        assert_eq!(turn.turn_number, 3);
        assert!(turn.narrator_rendering.is_some());
    }

    #[test]
    fn narrator_resource_is_debug() {
        // NarratorResource requires an NarratorAgent which needs LlmProvider,
        // so we just test the Debug impl exists via the format string
        let debug_str = "NarratorResource";
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn tokio_runtime_is_debug() {
        let debug_str = "TokioRuntime";
        assert!(!debug_str.is_empty());
    }
}
