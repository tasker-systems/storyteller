//! Turn cycle Bevy resources — state that drives the pipeline.
//!
//! See: `docs/technical/turn-cycle-architecture.md`
//!
//! These resources are held as singletons in the Bevy ECS world.
//! Systems read and write them to coordinate the turn pipeline.

use bevy_ecs::prelude::*;

use storyteller_core::types::message::NarratorRendering;
use storyteller_core::types::prediction::CharacterPrediction;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::turn_cycle::TurnCycleStage;
use storyteller_core::StorytellerResult;

use crate::inference::event_classifier::ClassificationOutput;
use storyteller_core::types::narrator_context::NarratorContextInput;

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

/// Bevy Resource: async narrator task tracking.
///
/// The Narrator LLM call is async but Bevy systems are sync.
/// This resource bridges the gap via a polling pattern:
/// 1. `start_rendering_system` spawns the LLM task and sets `InFlight`
/// 2. `poll_rendering_system` checks the receiver each frame
/// 3. When complete, the result is moved to `TurnContext.rendering`
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
}
