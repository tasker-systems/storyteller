//! Turn pipeline orchestration — synchronous Bevy systems for each stage.
//!
//! See: `docs/technical/turn-cycle-architecture.md`
//!
//! Each system gates on the current `TurnCycleStage` via `run_if`,
//! does its work, and advances the stage. The pipeline runs one
//! stage per frame — no stage skipping.
//!
//! Five synchronous systems live here:
//! - `commit_previous_system`: archives previous turn, moves PendingInput
//! - `classify_system`: ML event classification (or keyword fallback)
//! - `predict_system`: ML character prediction (parallel across cast)
//! - `resolve_system`: pass-through wrapping predictions into ResolverOutput
//! - `assemble_context_system`: three-tier Narrator context assembly
//!
//! The async narrator rendering system lives in [`super::rendering`].

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemSet;

use storyteller_core::traits::NoopObserver;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::turn_cycle::TurnCycleStage;

use crate::components::turn::{
    ActiveTurnStage, CompletedTurn, JournalResource, PendingInput, TurnContext, TurnHistory,
};

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

/// Resolve predictions via rules engine (pass-through for now).
///
/// Wraps ML predictions into a `ResolverOutput`. Real resolver logic
/// (RPG-like mechanics, conflict detection, graduated success) is future work.
pub fn resolve_system(mut stage: ResMut<ActiveTurnStage>, mut turn_ctx: ResMut<TurnContext>) {
    let predictions = turn_ctx.predictions.clone().unwrap_or_default();

    let resolver_output = ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: predictions,
        scene_dynamics: String::new(),
        conflicts: vec![],
    };

    turn_ctx.resolver_output = Some(resolver_output);

    tracing::debug!("resolve_system: wrapped predictions into ResolverOutput");
    stage.0 = stage.0.next();
}

/// Assemble three-tier Narrator context from scene data, journal, and resolver output.
///
/// Calls the existing `assemble_narrator_context()` with data sourced from
/// Bevy resources. Falls back gracefully when optional resources are missing.
pub fn assemble_context_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    scene_res: Option<Res<SceneResource>>,
    journal_res: Option<Res<JournalResource>>,
) {
    let Some(ref scene_res) = scene_res else {
        tracing::warn!("assemble_context_system: no SceneResource — skipping");
        stage.0 = stage.0.next();
        return;
    };

    let resolver_output = turn_ctx.resolver_output.clone().unwrap_or(ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: vec![],
        scene_dynamics: String::new(),
        conflicts: vec![],
    });

    let characters: Vec<&storyteller_core::types::character::CharacterSheet> =
        scene_res.characters.iter().collect();

    let empty_journal = storyteller_core::types::narrator_context::SceneJournal::new(
        storyteller_core::types::scene::SceneId::new(),
        1200,
    );
    let journal = journal_res.as_ref().map(|j| &j.0).unwrap_or(&empty_journal);

    let player_input = turn_ctx.player_input.as_deref().unwrap_or("");

    let context = crate::context::assemble_narrator_context(
        &scene_res.scene,
        &characters,
        journal,
        &resolver_output,
        player_input,
        &[], // referenced_entities — empty for now
        crate::context::DEFAULT_TOTAL_TOKEN_BUDGET,
        &NoopObserver,
    );

    tracing::debug!(
        estimated_tokens = context.estimated_tokens,
        "assemble_context_system: assembled three-tier context"
    );

    turn_ctx.narrator_context = Some(context);
    stage.0 = stage.0.next();
}

/// Build the combined text for committed-turn classification (D.3).
///
/// Concatenates narrator rendering text and player input with a newline
/// separator. Handles partial data (only rendering, only input, neither).
/// Returns `None` when neither text is available or both are empty.
fn build_committed_text(rendering: Option<&str>, player_input: Option<&str>) -> Option<String> {
    match (
        rendering.filter(|s| !s.is_empty()),
        player_input.filter(|s| !s.is_empty()),
    ) {
        (Some(r), Some(p)) => Some(format!("{r}\n{p}")),
        (Some(r), None) => Some(r.to_string()),
        (None, Some(p)) => Some(p.to_string()),
        (None, None) => None,
    }
}

/// Commit the previous turn's provisional data, then prepare for the new turn.
///
/// 1. If TurnContext has previous data (rendering or classification), archive
///    as a `CompletedTurn` and add rendering text to the journal.
/// 2. Run committed-turn classification on the combined narrator + player text (D.3).
/// 3. Reset TurnContext for the new turn.
/// 4. Move `PendingInput` → `TurnContext.player_input`.
/// 5. Advance to `Classifying`.
///
/// On the first turn of a scene, there is no previous data to archive —
/// the system simply moves PendingInput and advances.
pub fn commit_previous_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    mut pending: ResMut<PendingInput>,
    mut history: ResMut<TurnHistory>,
    journal: Option<ResMut<JournalResource>>,
    classifier: Option<Res<ClassifierResource>>,
) {
    let has_previous_data = turn_ctx.rendering.is_some() || turn_ctx.classification.is_some();

    if has_previous_data {
        let turn_number = history.next_turn_number();

        // Add rendering to journal if available
        if let Some(ref rendering) = turn_ctx.rendering {
            if let Some(mut journal_res) = journal {
                crate::context::journal::add_turn(
                    &mut journal_res.0,
                    turn_number,
                    &rendering.text,
                    vec![],
                    vec![], // TODO: extract emotional markers from rendering
                    &NoopObserver,
                );
            }
        }

        // D.3: Committed-turn classification on combined text
        let committed_classification = build_committed_text(
            turn_ctx.rendering.as_ref().map(|r| r.text.as_str()),
            turn_ctx.player_input.as_deref(),
        )
        .and_then(|combined_text| {
            let event_classifier = classifier.as_ref().map(|c| &c.0);
            let (_event_features, classification) =
                crate::context::prediction::classify_and_extract(
                    &combined_text,
                    event_classifier,
                    0,
                );

            tracing::debug!(
                turn_number,
                has_committed_classification = classification.is_some(),
                combined_text_len = combined_text.len(),
                "commit_previous_system: committed-turn classification"
            );

            classification
        });

        let completed = CompletedTurn {
            turn_number,
            player_input: turn_ctx.player_input.clone().unwrap_or_default(),
            narrator_rendering: turn_ctx.rendering.clone(),
            classification: turn_ctx.classification.clone(),
            committed_classification,
            predictions: turn_ctx.predictions.clone(),
            committed_at: chrono::Utc::now(),
        };

        tracing::debug!(
            turn_number,
            has_rendering = completed.narrator_rendering.is_some(),
            has_classification = completed.classification.is_some(),
            has_committed_classification = completed.committed_classification.is_some(),
            "commit_previous_system: archived turn"
        );

        history.turns.push(completed);
    } else {
        tracing::debug!("commit_previous_system: first turn — no previous data to archive");
    }

    // Reset context for the new turn
    turn_ctx.reset();

    // Move pending input into the fresh context
    turn_ctx.player_input = pending.0.take();

    stage.0 = stage.0.next(); // → Classifying
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::prelude::*;
    use storyteller_core::types::message::NarratorRendering;

    use crate::components::turn::NarratorTask;
    use crate::systems::rendering::rendering_system;

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

    // -----------------------------------------------------------------------
    // Stage condition
    // -----------------------------------------------------------------------

    #[test]
    fn in_stage_condition_matches() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Classifying;

        let stage = app.world().resource::<ActiveTurnStage>();
        assert!(stage.0 == TurnCycleStage::Classifying);
    }

    // -----------------------------------------------------------------------
    // classify_system
    // -----------------------------------------------------------------------

    #[test]
    fn classify_system_advances_without_input() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Classifying;

        app.add_systems(Update, classify_system);
        app.update();

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

    // -----------------------------------------------------------------------
    // predict_system
    // -----------------------------------------------------------------------

    #[test]
    fn predict_system_advances_without_predictor() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Predicting;

        app.add_systems(Update, predict_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Resolving);
    }

    // -----------------------------------------------------------------------
    // commit_previous_system
    // -----------------------------------------------------------------------

    #[test]
    fn commit_previous_moves_pending_input_and_advances() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("new input".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Classifying);

        let ctx = app.world().resource::<TurnContext>();
        assert_eq!(ctx.player_input.as_deref(), Some("new input"));

        let pending = app.world().resource::<PendingInput>();
        assert!(pending.0.is_none(), "PendingInput should be consumed");
    }

    #[test]
    fn commit_previous_first_turn_no_archive() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("first input".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert!(history.turns.is_empty(), "First turn should not archive");

        let ctx = app.world().resource::<TurnContext>();
        assert_eq!(ctx.player_input.as_deref(), Some("first input"));
    }

    #[test]
    fn commit_previous_archives_when_rendering_present() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some("previous input".to_string());
            ctx.rendering = Some(NarratorRendering {
                text: "The hooves leave prints.".to_string(),
                stage_directions: None,
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next input".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        assert_eq!(history.turns[0].player_input, "previous input");
        assert!(history.turns[0].narrator_rendering.is_some());
        assert_eq!(history.turns[0].turn_number, 1);

        let ctx = app.world().resource::<TurnContext>();
        assert_eq!(ctx.player_input.as_deref(), Some("next input"));
    }

    #[test]
    fn commit_previous_archives_when_classification_present() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some("previous input".to_string());
            ctx.classification = Some(crate::inference::event_classifier::ClassificationOutput {
                event_kinds: vec![],
                entity_mentions: vec![],
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next input".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        assert!(history.turns[0].classification.is_some());
    }

    #[test]
    fn turn_history_accumulates_across_commits() {
        let mut app = test_app();

        app.add_systems(Update, commit_previous_system);

        // First commit: nothing to archive, just set up
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("turn 1".to_string());
        app.update();

        // Simulate pipeline completing: set rendering
        app.world_mut().resource_mut::<TurnContext>().rendering = Some(NarratorRendering {
            text: "Rendering 1".to_string(),
            stage_directions: None,
        });

        // Second commit: archives turn 1
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("turn 2".to_string());
        app.update();

        // Simulate pipeline completing
        app.world_mut().resource_mut::<TurnContext>().rendering = Some(NarratorRendering {
            text: "Rendering 2".to_string(),
            stage_directions: None,
        });

        // Third commit: archives turn 2
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("turn 3".to_string());
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 2);
        assert_eq!(history.turns[0].turn_number, 1);
        assert_eq!(history.turns[1].turn_number, 2);
    }

    #[test]
    fn commit_previous_updates_journal() {
        let mut app = test_app();
        let journal = storyteller_core::types::narrator_context::SceneJournal::new(
            storyteller_core::types::scene::SceneId::new(),
            1200,
        );
        app.world_mut().insert_resource(JournalResource(journal));

        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some("test".to_string());
            ctx.rendering = Some(NarratorRendering {
                text: "The scene unfolds.".to_string(),
                stage_directions: None,
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let journal = app.world().resource::<JournalResource>();
        assert_eq!(journal.0.turn_count(), 1);
        assert!(journal.0.entries[0].content.contains("The scene unfolds"));
    }

    // -----------------------------------------------------------------------
    // resolve_system
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_wraps_predictions_into_resolver_output() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Resolving;
        app.world_mut().resource_mut::<TurnContext>().predictions = Some(vec![]);

        app.add_systems(Update, resolve_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AssemblingContext);

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.resolver_output.is_some());
    }

    #[test]
    fn resolve_handles_no_predictions() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Resolving;
        // predictions is None

        app.add_systems(Update, resolve_system);
        app.update();

        let ctx = app.world().resource::<TurnContext>();
        let resolver = ctx.resolver_output.as_ref().unwrap();
        assert!(resolver.original_predictions.is_empty());
    }

    // -----------------------------------------------------------------------
    // assemble_context_system
    // -----------------------------------------------------------------------

    #[test]
    fn assemble_context_advances_without_scene() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::AssemblingContext;

        app.add_systems(Update, assemble_context_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Rendering);

        // No narrator_context since no scene
        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.narrator_context.is_none());
    }

    #[test]
    fn assemble_context_populates_narrator_context() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::AssemblingContext;

        // Insert SceneResource with workshop data
        let scene_res = SceneResource {
            scene: crate::workshop::the_flute_kept::scene(),
            characters: vec![
                crate::workshop::the_flute_kept::bramblehoof(),
                crate::workshop::the_flute_kept::pyotir(),
            ],
        };
        app.world_mut().insert_resource(scene_res);

        // Set some resolver output
        app.world_mut()
            .resource_mut::<TurnContext>()
            .resolver_output = Some(ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: vec![],
            scene_dynamics: String::new(),
            conflicts: vec![],
        });
        app.world_mut().resource_mut::<TurnContext>().player_input =
            Some("I approach slowly".to_string());

        app.add_systems(Update, assemble_context_system);
        app.update();

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.narrator_context.is_some());
        let nc = ctx.narrator_context.as_ref().unwrap();
        assert_eq!(nc.preamble.cast_descriptions.len(), 2);
        assert_eq!(nc.player_input_summary, "I approach slowly");
        assert!(nc.estimated_tokens > 0);
    }

    // -----------------------------------------------------------------------
    // Full pipeline (includes rendering_system from sibling module)
    // -----------------------------------------------------------------------

    #[test]
    fn full_pipeline_with_pending_input() {
        let mut app = test_app();

        // Register all systems in sequence
        app.add_systems(
            Update,
            (
                commit_previous_system,
                classify_system.after(commit_previous_system),
                predict_system.after(classify_system),
                resolve_system.after(predict_system),
                assemble_context_system.after(resolve_system),
                rendering_system.after(assemble_context_system),
            ),
        );

        // Start at CommittingPrevious with pending input
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("I look around".to_string());

        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(
            stage.0,
            TurnCycleStage::AwaitingInput,
            "Full pipeline should complete in one update"
        );

        let ctx = app.world().resource::<TurnContext>();
        assert_eq!(
            ctx.player_input.as_deref(),
            Some("I look around"),
            "Player input should be preserved through pipeline"
        );
    }

    #[test]
    fn full_pipeline_with_scene_populates_context() {
        let mut app = test_app();

        let scene_res = SceneResource {
            scene: crate::workshop::the_flute_kept::scene(),
            characters: vec![
                crate::workshop::the_flute_kept::bramblehoof(),
                crate::workshop::the_flute_kept::pyotir(),
            ],
        };
        app.world_mut().insert_resource(scene_res);

        app.add_systems(
            Update,
            (
                commit_previous_system,
                classify_system.after(commit_previous_system),
                predict_system.after(classify_system),
                resolve_system.after(predict_system),
                assemble_context_system.after(resolve_system),
                rendering_system.after(assemble_context_system),
            ),
        );

        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("I approach the fence".to_string());

        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AwaitingInput);

        // Context was assembled (even though rendering skipped without NarratorResource)
        let ctx = app.world().resource::<TurnContext>();
        assert!(
            ctx.narrator_context.is_some(),
            "Should have assembled narrator context"
        );
        assert!(ctx.resolver_output.is_some(), "Should have resolver output");
    }

    // -----------------------------------------------------------------------
    // TurnHistory helpers
    // -----------------------------------------------------------------------

    #[test]
    fn turn_history_next_turn_number() {
        let mut history = TurnHistory::default();
        assert_eq!(history.next_turn_number(), 1);

        history.turns.push(CompletedTurn {
            turn_number: 1,
            player_input: "test".to_string(),
            narrator_rendering: None,
            classification: None,
            committed_classification: None,
            predictions: None,
            committed_at: chrono::Utc::now(),
        });
        assert_eq!(history.next_turn_number(), 2);
    }

    // -----------------------------------------------------------------------
    // D.3: Committed-turn classification
    // -----------------------------------------------------------------------

    #[test]
    fn build_committed_text_both_texts() {
        let result =
            super::build_committed_text(Some("The narrator speaks."), Some("I walk forward"));
        assert_eq!(
            result.as_deref(),
            Some("The narrator speaks.\nI walk forward")
        );
    }

    #[test]
    fn build_committed_text_rendering_only() {
        let result = super::build_committed_text(Some("The narrator speaks."), None);
        assert_eq!(result.as_deref(), Some("The narrator speaks."));
    }

    #[test]
    fn build_committed_text_input_only() {
        let result = super::build_committed_text(None, Some("I walk forward"));
        assert_eq!(result.as_deref(), Some("I walk forward"));
    }

    #[test]
    fn build_committed_text_neither() {
        let result = super::build_committed_text(None, None);
        assert!(result.is_none());
    }

    #[test]
    fn build_committed_text_empty_strings() {
        let result = super::build_committed_text(Some(""), Some(""));
        assert!(result.is_none());
    }

    #[test]
    fn build_committed_text_one_empty_one_present() {
        let result = super::build_committed_text(Some(""), Some("I walk forward"));
        assert_eq!(result.as_deref(), Some("I walk forward"));

        let result = super::build_committed_text(Some("The narrator speaks."), Some(""));
        assert_eq!(result.as_deref(), Some("The narrator speaks."));
    }

    #[test]
    fn committed_classification_with_both_texts() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some("I walk to the door".to_string());
            ctx.rendering = Some(NarratorRendering {
                text: "The old house creaks in the wind.".to_string(),
                stage_directions: None,
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next input".to_string());

        // No ClassifierResource → keyword fallback → committed_classification is None
        // (keyword fallback returns None for ClassificationOutput)
        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        // Without ML classifier, committed_classification is None (keyword fallback)
        assert!(history.turns[0].committed_classification.is_none());
        // Per-input classification should still be None (wasn't set on TurnContext)
        assert!(history.turns[0].classification.is_none());
    }

    #[test]
    fn committed_classification_rendering_only() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            // No player_input — only rendering triggers archival
            ctx.rendering = Some(NarratorRendering {
                text: "The old house creaks.".to_string(),
                stage_directions: None,
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        // Combined text is just the rendering — fallback classification returns None
        assert!(history.turns[0].committed_classification.is_none());
    }

    #[test]
    fn committed_classification_input_only() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some("I walk to the door".to_string());
            // classification present to trigger archival, but no rendering
            ctx.classification = Some(crate::inference::event_classifier::ClassificationOutput {
                event_kinds: vec![],
                entity_mentions: vec![],
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        // Combined text is just player input — fallback returns None
        assert!(history.turns[0].committed_classification.is_none());
    }

    #[test]
    fn committed_classification_neither_text() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            // classification present to trigger archival, but no input or rendering
            ctx.classification = Some(crate::inference::event_classifier::ClassificationOutput {
                event_kinds: vec![],
                entity_mentions: vec![],
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        assert!(history.turns[0].committed_classification.is_none());
    }

    #[test]
    fn committed_classification_empty_strings() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some(String::new());
            ctx.rendering = Some(NarratorRendering {
                text: String::new(),
                stage_directions: None,
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        // Both texts empty → build_committed_text returns None → no classification
        assert!(history.turns[0].committed_classification.is_none());
    }

    #[test]
    fn committed_classification_without_classifier() {
        let mut app = test_app();
        // No ClassifierResource inserted
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.player_input = Some("I approach the gate".to_string());
            ctx.rendering = Some(NarratorRendering {
                text: "A figure stands at the gate.".to_string(),
                stage_directions: None,
            });
        }
        app.world_mut().resource_mut::<PendingInput>().0 = Some("next".to_string());

        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        // Without ClassifierResource, keyword fallback → None for ClassificationOutput
        assert!(history.turns[0].committed_classification.is_none());
    }

    #[test]
    fn committed_classification_preserved_in_history() {
        let mut app = test_app();
        app.add_systems(Update, commit_previous_system);

        // Turn 1: set up with rendering
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("turn 1 input".to_string());
        app.update();

        // Simulate turn 1 completing
        app.world_mut().resource_mut::<TurnContext>().rendering = Some(NarratorRendering {
            text: "The scene unfolds before you.".to_string(),
            stage_directions: None,
        });

        // Turn 2: archives turn 1
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("turn 2 input".to_string());
        app.update();

        // Simulate turn 2 completing
        app.world_mut().resource_mut::<TurnContext>().rendering = Some(NarratorRendering {
            text: "A second scene emerges.".to_string(),
            stage_directions: None,
        });

        // Turn 3: archives turn 2
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::CommittingPrevious;
        app.world_mut().resource_mut::<PendingInput>().0 = Some("turn 3 input".to_string());
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 2);
        // Both archived turns should have the committed_classification field
        // (None without ML classifier, but the field exists and is preserved)
        assert!(history.turns[0].committed_classification.is_none());
        assert!(history.turns[1].committed_classification.is_none());
        assert_eq!(history.turns[0].turn_number, 1);
        assert_eq!(history.turns[1].turn_number, 2);
    }
}
