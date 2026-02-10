//! Turn pipeline orchestration — Bevy systems for each stage.
//!
//! See: `docs/technical/turn-cycle-architecture.md`
//!
//! Each system gates on the current `TurnCycleStage` via `run_if`,
//! does its work, and advances the stage. The pipeline runs one
//! stage per frame — no stage skipping.
//!
//! Two systems are fully implemented (classify, predict). The remaining
//! four are stubs that advance the stage without doing work — they will
//! be filled in as the corresponding subsystems mature.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemSet;

use storyteller_core::types::turn_cycle::TurnCycleStage;

use crate::components::turn::{ActiveTurnStage, NarratorTask, TurnContext};

// ---------------------------------------------------------------------------
// SystemSet — declared ordering for the turn pipeline
// ---------------------------------------------------------------------------

/// Ordering sets for the turn cycle pipeline.
///
/// Each set corresponds to one stage of the turn pipeline. Sets are
/// configured in the plugin with `.after()` chains to enforce sequential
/// execution within a single frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TurnCycleSets {
    /// Player input reception.
    Input,
    /// Commit previous turn's provisional data (predictions, rendering).
    CommittingPrevious,
    /// Event classifier (ML or keyword fallback).
    Classification,
    /// ML character prediction (parallel across cast).
    Prediction,
    /// Rules engine resolution.
    Resolution,
    /// Three-tier Narrator context assembly.
    ContextAssembly,
    /// Narrator LLM call (async bridge).
    Rendering,
}

// ---------------------------------------------------------------------------
// Run conditions
// ---------------------------------------------------------------------------

/// Run condition: true when the pipeline is in the specified stage.
pub fn in_stage(target: TurnCycleStage) -> impl Fn(Res<ActiveTurnStage>) -> bool {
    move |stage: Res<ActiveTurnStage>| stage.0 == target
}

// ---------------------------------------------------------------------------
// Systems — Classification (REAL)
// ---------------------------------------------------------------------------

/// Classify player input using ML classifier or keyword fallback.
///
/// Reads `TurnContext.player_input`, calls `classify_and_extract()`,
/// writes classification output and advances to `Predicting`.
pub fn classify_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    classifier: Option<Res<ClassifierResource>>,
) {
    let Some(ref input) = turn_ctx.player_input else {
        tracing::warn!("classify_system: no player input in TurnContext");
        stage.0 = stage.0.next();
        return;
    };

    let event_classifier = classifier.as_ref().map(|c| &c.0);
    let (_event_features, classification) =
        crate::context::prediction::classify_and_extract(input, event_classifier, 0);

    turn_ctx.classification = classification;
    stage.0 = stage.0.next();
}

/// Bevy Resource wrapping an optional `EventClassifier`.
///
/// Held as a Resource so that systems can access it via `Option<Res<_>>`.
/// When no ML model is available, this resource is not inserted.
#[derive(Resource)]
pub struct ClassifierResource(pub crate::inference::event_classifier::EventClassifier);

impl std::fmt::Debug for ClassifierResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassifierResource").finish()
    }
}

// ---------------------------------------------------------------------------
// Systems — Prediction (REAL)
// ---------------------------------------------------------------------------

/// Run ML character prediction for all cast members.
///
/// Reads `TurnContext.player_input`, calls `predict_character_behaviors()`,
/// writes predictions and advances to `Resolving`.
pub fn predict_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    predictor: Option<Res<PredictorResource>>,
    scene_res: Option<Res<SceneResource>>,
    grammar_res: Option<Res<GrammarResource>>,
) {
    let Some(ref predictor) = predictor else {
        tracing::debug!("predict_system: no CharacterPredictor, skipping");
        stage.0 = stage.0.next();
        return;
    };

    let Some(ref scene_res) = scene_res else {
        tracing::warn!("predict_system: no SceneResource");
        stage.0 = stage.0.next();
        return;
    };

    let Some(ref grammar_res) = grammar_res else {
        tracing::warn!("predict_system: no GrammarResource");
        stage.0 = stage.0.next();
        return;
    };

    let input = turn_ctx.player_input.as_deref().unwrap_or("");

    let characters: Vec<&storyteller_core::types::character::CharacterSheet> =
        scene_res.characters.iter().collect();

    let classifier_ref = None::<&crate::inference::event_classifier::EventClassifier>;
    let (predictions, classification) = crate::context::prediction::predict_character_behaviors(
        &predictor.0,
        &characters,
        &scene_res.scene,
        input,
        grammar_res.0.as_ref(),
        classifier_ref,
    );

    turn_ctx.predictions = Some(predictions);
    // Only overwrite classification if we got a new one
    if classification.is_some() {
        turn_ctx.classification = classification;
    }

    stage.0 = stage.0.next();
}

/// Bevy Resource wrapping a `CharacterPredictor`.
#[derive(Resource)]
pub struct PredictorResource(pub crate::inference::frame::CharacterPredictor);

impl std::fmt::Debug for PredictorResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PredictorResource").finish()
    }
}

/// Bevy Resource wrapping scene data and cast for the current scene.
#[derive(Debug, Resource)]
pub struct SceneResource {
    /// The current scene's data.
    pub scene: storyteller_core::types::character::SceneData,
    /// Cast members in the current scene.
    pub characters: Vec<storyteller_core::types::character::CharacterSheet>,
}

/// Bevy Resource wrapping an emotional grammar implementation.
#[derive(Resource)]
pub struct GrammarResource(
    pub Box<dyn storyteller_core::traits::emotional_grammar::EmotionalGrammar>,
);

impl std::fmt::Debug for GrammarResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrammarResource").finish()
    }
}

// ---------------------------------------------------------------------------
// Systems — Stubs (advance stage only)
// ---------------------------------------------------------------------------

/// Stub: resolve predictions via rules engine.
///
/// Will eventually call the action resolver to sequence predictions,
/// enforce world constraints, and detect conflicts.
pub fn resolve_system(mut stage: ResMut<ActiveTurnStage>) {
    tracing::debug!("resolve_system: stub — advancing stage");
    stage.0 = stage.0.next();
}

/// Stub: assemble three-tier Narrator context.
///
/// Will eventually call `assemble_narrator_context()` to build
/// preamble + journal + retrieved context.
pub fn assemble_context_system(mut stage: ResMut<ActiveTurnStage>) {
    tracing::debug!("assemble_context_system: stub — advancing stage");
    stage.0 = stage.0.next();
}

/// Stub: commit the previous turn's provisional data.
///
/// Will eventually commit predictions (Hypothesized → Committed),
/// Narrator rendering (Rendered → Committed), update the event ledger,
/// and modify the truth set. On the first turn of a scene, this is a no-op.
///
/// Resets the TurnContext for the new turn's pipeline stages.
pub fn commit_previous_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
) {
    tracing::debug!("commit_previous_system: stub — resetting context for new turn");
    turn_ctx.reset();
    stage.0 = stage.0.next(); // → Classifying
}

/// Stub: start the async Narrator LLM call.
///
/// Will eventually spawn a tokio task, set `NarratorTask::InFlight`,
/// and transition to a polling state. `Rendering` is the final active
/// stage — when complete, the pipeline returns to `AwaitingInput`.
pub fn start_rendering_system(mut stage: ResMut<ActiveTurnStage>, mut _task: ResMut<NarratorTask>) {
    tracing::debug!("start_rendering_system: stub — advancing to AwaitingInput");
    stage.0 = stage.0.next(); // → AwaitingInput
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::prelude::*;

    /// Helper: create a minimal Bevy App with turn cycle resources.
    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ActiveTurnStage>();
        app.init_resource::<TurnContext>();
        app.init_resource::<NarratorTask>();
        app
    }

    #[test]
    fn in_stage_condition_matches() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Classifying;

        let stage = app.world().resource::<ActiveTurnStage>();
        assert!(stage.0 == TurnCycleStage::Classifying);
    }

    #[test]
    fn classify_system_advances_without_input() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Classifying;

        app.add_systems(Update, classify_system);
        app.update();

        // Should advance to Predicting even without input
        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Predicting);
    }

    #[test]
    fn classify_system_with_input_advances() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Classifying;
        app.world_mut().resource_mut::<TurnContext>().player_input =
            Some("I walk to the door".to_string());

        app.add_systems(Update, classify_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Predicting);
    }

    #[test]
    fn predict_system_advances_without_predictor() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Predicting;

        app.add_systems(Update, predict_system);
        app.update();

        // No predictor resource → skip, advance
        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Resolving);
    }

    #[test]
    fn resolve_stub_advances() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Resolving;

        app.add_systems(Update, resolve_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AssemblingContext);
    }

    #[test]
    fn assemble_context_stub_advances() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::AssemblingContext;

        app.add_systems(Update, assemble_context_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Rendering);
    }

    #[test]
    fn commit_previous_resets_and_advances_to_classifying() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<TurnContext>().player_input = Some("test".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Classifying);

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.player_input.is_none(), "TurnContext should be reset");
    }

    #[test]
    fn start_rendering_stub_returns_to_awaiting() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Rendering;

        app.add_systems(Update, start_rendering_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);
    }

    #[test]
    fn full_pipeline_stub_cycle() {
        let mut app = test_app();

        // Register all systems (no run_if conditions — just run in sequence)
        app.add_systems(
            Update,
            (
                commit_previous_system,
                classify_system.after(commit_previous_system),
                predict_system.after(classify_system),
                resolve_system.after(predict_system),
                assemble_context_system.after(resolve_system),
                start_rendering_system.after(assemble_context_system),
            ),
        );

        // Start at CommittingPrevious with input
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<TurnContext>().player_input =
            Some("I look around".to_string());

        // One update should run through all stages
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(
            stage.0,
            TurnCycleStage::AwaitingInput,
            "Full pipeline should complete in one update"
        );
    }
}
