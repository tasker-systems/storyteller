//! Async narrator rendering bridge — spawns LLM calls and polls for completion.
//!
//! The Narrator LLM call is the one truly async operation in the turn pipeline.
//! All other stages (classification, prediction, resolution, context assembly)
//! are synchronous CPU-bound work.
//!
//! This module encapsulates the async lifecycle management: spawning a tokio
//! task via `TokioRuntime`, polling a oneshot receiver each frame, and
//! extracting the result into `TurnContext`.
//!
//! The `rendering_system` participates in the pipeline via `run_if` stage
//! gating (same as all other turn cycle systems), but manages an async
//! state machine (`NarratorTask`) internally rather than completing in
//! a single frame.
//!
//! See: `docs/technical/turn-cycle-architecture.md` (Async Bridge section)

use bevy_ecs::prelude::*;

use storyteller_core::traits::NoopObserver;

use crate::components::turn::{
    ActiveTurnStage, NarratorResource, NarratorTask, TokioRuntime, TurnContext,
};

/// Narrator rendering — state-machine dispatching on `NarratorTask`.
///
/// This system handles three states:
///
/// - **Idle**: Extract narrator context, spawn async LLM task via
///   `NarratorResource` + `TokioRuntime`, set `InFlight`. Stay in Rendering.
/// - **InFlight**: Poll the oneshot receiver. If result is ready, extract
///   rendering into TurnContext, reset to Idle, advance to AwaitingInput.
///   If not ready, stay in Rendering for the next frame.
/// - **Complete**: Direct completion (tests, sync fallback). Extract
///   rendering, reset to Idle, advance to AwaitingInput.
///
/// When `NarratorResource` or `TokioRuntime` is absent, the system
/// skips rendering and advances immediately (graceful degradation).
pub fn rendering_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    mut task: ResMut<NarratorTask>,
    narrator_res: Option<Res<NarratorResource>>,
    runtime: Option<Res<TokioRuntime>>,
) {
    // Take the current task state via mem::take (NarratorTask: Default → Idle)
    let current = std::mem::take(&mut *task);

    match current {
        NarratorTask::Idle => {
            // Extract context for the narrator
            let Some(context) = turn_ctx.narrator_context.clone() else {
                tracing::warn!("rendering_system: no narrator_context — skipping");
                stage.0 = stage.0.next();
                return;
            };

            // Need both NarratorResource and TokioRuntime to spawn async
            let (Some(narrator), Some(rt)) = (narrator_res, runtime) else {
                tracing::debug!("rendering_system: no NarratorResource or TokioRuntime — skipping");
                stage.0 = stage.0.next();
                return;
            };

            let (tx, rx) = tokio::sync::oneshot::channel();
            let agent = narrator.0.clone();

            rt.0.spawn(async move {
                let agent = agent.lock().await;
                let result = agent.render(&context, &NoopObserver).await;
                // If the receiver was dropped, that's fine — the result is discarded
                let _ = tx.send(result);
            });

            tracing::debug!("rendering_system: spawned async narrator task");
            *task = NarratorTask::InFlight(rx);
            // Stay in Rendering — don't advance stage
        }
        NarratorTask::InFlight(mut rx) => {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(rendering) => {
                            tracing::debug!(
                                chars = rendering.text.len(),
                                "rendering_system: narrator rendering complete"
                            );
                            turn_ctx.rendering = Some(rendering);
                        }
                        Err(e) => {
                            tracing::error!("rendering_system: narrator error: {e}");
                        }
                    }
                    *task = NarratorTask::Idle;
                    stage.0 = stage.0.next(); // → AwaitingInput
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    // Not ready yet — put receiver back and stay in Rendering
                    *task = NarratorTask::InFlight(rx);
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    tracing::error!("rendering_system: oneshot channel closed unexpectedly");
                    *task = NarratorTask::Idle;
                    stage.0 = stage.0.next(); // → AwaitingInput
                }
            }
        }
        NarratorTask::Complete(result) => {
            match result {
                Ok(rendering) => {
                    tracing::debug!(
                        chars = rendering.text.len(),
                        "rendering_system: direct completion"
                    );
                    turn_ctx.rendering = Some(rendering);
                }
                Err(e) => {
                    tracing::error!("rendering_system: direct completion error: {e}");
                }
            }
            *task = NarratorTask::Idle;
            stage.0 = stage.0.next(); // → AwaitingInput
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::prelude::*;
    use storyteller_core::types::message::NarratorRendering;
    use storyteller_core::types::resolver::ResolverOutput;
    use storyteller_core::types::turn_cycle::TurnCycleStage;
    use storyteller_core::StorytellerResult;

    use crate::components::turn::{PendingInput, TurnHistory};

    /// Helper: create a minimal Bevy App with turn cycle resources.
    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ActiveTurnStage>();
        app.init_resource::<TurnContext>();
        app.init_resource::<NarratorTask>();
        app.init_resource::<TurnHistory>();
        app.init_resource::<PendingInput>();
        app
    }

    fn mock_narrator_context() -> storyteller_core::types::narrator_context::NarratorContextInput {
        use storyteller_core::types::narrator_context::*;
        use storyteller_core::types::scene::SceneId;

        NarratorContextInput {
            preamble: PersistentPreamble {
                narrator_identity: "Test narrator".to_string(),
                anti_patterns: vec![],
                setting_description: "A test scene".to_string(),
                cast_descriptions: vec![],
                boundaries: vec![],
            },
            journal: SceneJournal::new(SceneId::new(), 1200),
            retrieved: vec![],
            resolver_output: ResolverOutput {
                sequenced_actions: vec![],
                original_predictions: vec![],
                scene_dynamics: String::new(),
                conflicts: vec![],
            },
            player_input_summary: "test".to_string(),
            estimated_tokens: 100,
        }
    }

    #[test]
    fn rendering_skips_without_narrator_resource() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        // Set narrator_context so system doesn't bail on missing context
        app.world_mut()
            .resource_mut::<TurnContext>()
            .narrator_context = Some(mock_narrator_context());

        app.add_systems(Update, rendering_system);
        app.update();

        // Should advance past Rendering since no NarratorResource
        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);
    }

    #[test]
    fn rendering_skips_without_context() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        app.add_systems(Update, rendering_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);
    }

    #[test]
    fn rendering_complete_extracts_rendering() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        // Pre-set NarratorTask to Complete
        *app.world_mut().resource_mut::<NarratorTask>() =
            NarratorTask::Complete(Ok(NarratorRendering {
                text: "The moonlight catches the frost.".to_string(),
                stage_directions: Some("gentle".to_string()),
            }));

        app.add_systems(Update, rendering_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.rendering.is_some());
        assert!(ctx.rendering.as_ref().unwrap().text.contains("moonlight"));
    }

    #[test]
    fn rendering_complete_error_advances() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        *app.world_mut().resource_mut::<NarratorTask>() = NarratorTask::Complete(Err(
            storyteller_core::errors::StorytellerError::Config("test error".into()),
        ));

        app.add_systems(Update, rendering_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);

        let ctx = app.world().resource::<TurnContext>();
        assert!(
            ctx.rendering.is_none(),
            "Error should not produce rendering"
        );
    }

    #[test]
    fn rendering_inflight_stays_when_not_ready() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        // Create a oneshot where the sender is still alive but hasn't sent
        let (tx, rx) = tokio::sync::oneshot::channel();
        *app.world_mut().resource_mut::<NarratorTask>() = NarratorTask::InFlight(rx);

        app.add_systems(Update, rendering_system);
        app.update();

        // Should stay in Rendering
        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Rendering);

        // Task should still be InFlight
        let task = app.world().resource::<NarratorTask>();
        assert!(matches!(task, NarratorTask::InFlight(_)));

        // Keep tx alive to prevent channel close
        drop(tx);
    }

    #[test]
    fn rendering_inflight_advances_when_ready() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        let (tx, rx) = tokio::sync::oneshot::channel();
        // Send the result before polling
        tx.send(Ok(NarratorRendering {
            text: "A quiet dawn.".to_string(),
            stage_directions: None,
        }))
        .expect("send should succeed");

        *app.world_mut().resource_mut::<NarratorTask>() = NarratorTask::InFlight(rx);

        app.add_systems(Update, rendering_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.rendering.is_some());
        assert!(ctx.rendering.as_ref().unwrap().text.contains("quiet dawn"));
    }

    #[test]
    fn rendering_inflight_closed_channel_advances() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        let (tx, rx) = tokio::sync::oneshot::channel::<StorytellerResult<NarratorRendering>>();
        // Drop sender to close channel
        drop(tx);

        *app.world_mut().resource_mut::<NarratorTask>() = NarratorTask::InFlight(rx);

        app.add_systems(Update, rendering_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);
    }
}
