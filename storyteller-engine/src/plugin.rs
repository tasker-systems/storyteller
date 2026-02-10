//! Bevy plugin registration for the storyteller engine.
//!
//! The `StorytellerEnginePlugin` registers all systems, components, events,
//! and resources needed to run the storytelling engine within a Bevy App.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::IntoSystemConfigs;
use bevy_ecs::schedule::IntoSystemSetConfigs;

use crate::components::turn::{ActiveTurnStage, NarratorTask, TurnContext};
use crate::systems::turn_cycle::{
    assemble_context_system, classify_system, commit_previous_system, in_stage, predict_system,
    resolve_system, start_rendering_system, TurnCycleSets,
};
use storyteller_core::types::turn_cycle::TurnCycleStage;

/// Main plugin for the storyteller engine.
///
/// Registers turn cycle systems, scene lifecycle, agent systems,
/// event pipeline, and observability infrastructure.
#[derive(Debug)]
pub struct StorytellerEnginePlugin;

impl Plugin for StorytellerEnginePlugin {
    fn build(&self, app: &mut App) {
        // Turn cycle resources
        app.init_resource::<ActiveTurnStage>()
            .init_resource::<TurnContext>()
            .init_resource::<NarratorTask>();

        // System set ordering — sequential pipeline within a single frame
        app.configure_sets(
            Update,
            (
                TurnCycleSets::Input,
                TurnCycleSets::CommittingPrevious.after(TurnCycleSets::Input),
                TurnCycleSets::Classification.after(TurnCycleSets::CommittingPrevious),
                TurnCycleSets::Prediction.after(TurnCycleSets::Classification),
                TurnCycleSets::Resolution.after(TurnCycleSets::Prediction),
                TurnCycleSets::ContextAssembly.after(TurnCycleSets::Resolution),
                TurnCycleSets::Rendering.after(TurnCycleSets::ContextAssembly),
            ),
        );

        // Turn cycle systems — each gated by its stage
        app.add_systems(
            Update,
            (
                commit_previous_system
                    .run_if(in_stage(TurnCycleStage::CommittingPrevious))
                    .in_set(TurnCycleSets::CommittingPrevious),
                classify_system
                    .run_if(in_stage(TurnCycleStage::Classifying))
                    .in_set(TurnCycleSets::Classification),
                predict_system
                    .run_if(in_stage(TurnCycleStage::Predicting))
                    .in_set(TurnCycleSets::Prediction),
                resolve_system
                    .run_if(in_stage(TurnCycleStage::Resolving))
                    .in_set(TurnCycleSets::Resolution),
                assemble_context_system
                    .run_if(in_stage(TurnCycleStage::AssemblingContext))
                    .in_set(TurnCycleSets::ContextAssembly),
                start_rendering_system
                    .run_if(in_stage(TurnCycleStage::Rendering))
                    .in_set(TurnCycleSets::Rendering),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_builds_without_panic() {
        let mut app = App::new();
        app.add_plugins(StorytellerEnginePlugin);
        // One update cycle with default AwaitingInput — no systems should run
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);
    }

    #[test]
    fn plugin_runs_full_pipeline_in_one_frame() {
        let mut app = App::new();
        app.add_plugins(StorytellerEnginePlugin);

        // Set stage to CommittingPrevious with input — simulates player input received
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<TurnContext>().player_input = Some("hello".to_string());

        // One update — all stages fire in sequence (stubs advance immediately),
        // ending back at AwaitingInput with a clean TurnContext.
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(
            stage.0,
            TurnCycleStage::AwaitingInput,
            "full pipeline should complete in one frame"
        );

        let ctx = app.world().resource::<TurnContext>();
        assert!(
            ctx.player_input.is_none(),
            "TurnContext should be reset after commit_previous"
        );
    }
}
