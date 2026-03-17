//! Turn pipeline orchestration — synchronous Bevy systems for each stage.
//!
//! See: `docs/technical/turn-cycle-architecture.md`
//!
//! Each system gates on the current `TurnCycleStage` via `run_if`,
//! does its work, and advances the stage. The pipeline runs one
//! stage per frame — no stage skipping.
//!
//! Four synchronous systems live here:
//! - `commit_previous_system`: archives previous turn, moves PendingInput
//! - `enrichment_system`: unified sub-pipeline (classification, prediction,
//!    arbitration, intent synthesis) managed by EnrichmentPhase
//! - `assemble_context_system`: three-tier Narrator context assembly
//!
//! The async narrator rendering system lives in [`super::rendering`].

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemSet;

use storyteller_core::traits::NoopObserver;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::turn_cycle::{EnrichmentPhase, TurnCycleStage};

use crate::components::turn::{
    ActiveTurnStage, CompletedTurn, EnrichmentState, JournalResource, PendingInput,
    StructuredLlmResource, TokioRuntime, TurnContext, TurnHistory,
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
    /// Unified enrichment: classification, prediction, arbitration, intent synthesis.
    Enrichment,
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
// Systems — Enrichment (unified sub-pipeline)
// ---------------------------------------------------------------------------

/// Unified enrichment system managing the sub-pipeline internally.
///
/// Loops through all `EnrichmentPhase` stages in a single Bevy update:
/// - EventClassification: pass-through (decomposition at server layer via 3b-instruct)
/// - BehaviorPrediction: ML character prediction (parallel across cast)
/// - GameSystemArbitration: action arbitration + wrap predictions into ResolverOutput
/// - IntentSynthesis: pass-through placeholder
/// - Complete: reset EnrichmentState, advance top-level stage to AssemblingContext
pub fn enrichment_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut enrichment: ResMut<EnrichmentState>,
    mut turn_ctx: ResMut<TurnContext>,
    predictor: Option<Res<PredictorResource>>,
    scene_res: Option<Res<SceneResource>>,
    grammar_res: Option<Res<GrammarResource>>,
) {
    loop {
        match enrichment.0 {
            EnrichmentPhase::EventClassification => {
                // Pass-through: event decomposition happens at server layer via
                // structured 3b-instruct LLM, not in the Bevy pipeline.
                // Future: may migrate decomposition here.
                tracing::debug!("enrichment_system: EventClassification (pass-through)");
                enrichment.0 = enrichment.0.next();
            }
            EnrichmentPhase::BehaviorPrediction => {
                // ML character prediction (logic from former predict_system)
                if let (Some(ref predictor), Some(ref scene_res), Some(ref grammar_res)) =
                    (&predictor, &scene_res, &grammar_res)
                {
                    let input = turn_ctx.player_input.as_deref().unwrap_or("");
                    let characters: Vec<&storyteller_core::types::character::CharacterSheet> =
                        scene_res.characters.iter().collect();

                    let event_features = storyteller_ml::feature_schema::EventFeatureInput {
                        event_type: storyteller_core::types::prediction::EventType::Interaction,
                        emotional_register:
                            storyteller_core::types::prediction::EmotionalRegister::Neutral,
                        confidence: 0.5,
                        target_count: characters.len().saturating_sub(1) as u8,
                    };
                    let predictions = crate::context::prediction::predict_character_behaviors(
                        &predictor.0,
                        &characters,
                        &scene_res.scene,
                        input,
                        grammar_res.0.as_ref(),
                        event_features,
                        &std::collections::HashMap::new(),
                    );
                    turn_ctx.predictions = Some(predictions);
                } else {
                    tracing::debug!(
                        "enrichment_system: BehaviorPrediction skipped (missing resources)"
                    );
                }
                enrichment.0 = enrichment.0.next();
            }
            EnrichmentPhase::GameSystemArbitration => {
                // Action arbitration + wrap predictions into ResolverOutput
                // (logic from former resolve_system)
                if let Some(ref input) = turn_ctx.player_input {
                    let result = crate::systems::arbitration::check_action_possibility(
                        input,
                        &[], // genre_constraints — will come from scene resource later
                        &storyteller_core::types::capability_lexicon::CapabilityLexicon::new(),
                        None, // actor_zone — will come from spatial tracking later
                    );

                    tracing::debug!(
                        permitted = result.is_permitted(),
                        impossible = result.is_impossible(),
                        ambiguous = result.is_ambiguous(),
                        "enrichment_system: action arbitration check"
                    );

                    turn_ctx.arbitration = Some(result);
                }

                let predictions = turn_ctx.predictions.clone().unwrap_or_default();
                let resolver_output = ResolverOutput {
                    sequenced_actions: vec![],
                    original_predictions: predictions,
                    scene_dynamics: String::new(),
                    conflicts: vec![],
                    intent_statements: None,
                };
                turn_ctx.resolver_output = Some(resolver_output);

                tracing::debug!(
                    "enrichment_system: GameSystemArbitration wrapped predictions into ResolverOutput"
                );
                enrichment.0 = enrichment.0.next();
            }
            EnrichmentPhase::IntentSynthesis => {
                // Pass-through placeholder: intent synthesis will be implemented
                // in a future task.
                tracing::debug!("enrichment_system: IntentSynthesis (pass-through)");
                enrichment.0 = enrichment.0.next();
            }
            EnrichmentPhase::Complete => {
                // Reset enrichment state for next turn, advance top-level stage
                enrichment.0 = EnrichmentPhase::default();
                stage.0 = stage.0.next(); // → AssemblingContext
                break;
            }
        }
    }
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
        intent_statements: None,
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
        None, // player_entity_id — Bevy system doesn't track player entity yet
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
/// 5. Advance to `Enriching`.
///
/// On the first turn of a scene, there is no previous data to archive —
/// the system simply moves PendingInput and advances.
pub fn commit_previous_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    mut pending: ResMut<PendingInput>,
    mut history: ResMut<TurnHistory>,
    journal: Option<ResMut<JournalResource>>,
    structured_llm: Option<Res<StructuredLlmResource>>,
    runtime: Option<Res<TokioRuntime>>,
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
            // Prefer small LLM decomposition when available
            if let (Some(ref slm), Some(ref rt)) = (&structured_llm, &runtime) {
                match rt
                    .0
                    .block_on(crate::inference::event_decomposition::decompose_events(
                        slm.0.as_ref(),
                        &combined_text,
                    )) {
                    Ok(decomp) => {
                        tracing::debug!(
                            turn_number,
                            event_count = decomp.events.len(),
                            entity_count = decomp.entities.len(),
                            "commit_previous_system: LLM event decomposition"
                        );
                        Some(decomp.to_classification_output())
                    }
                    Err(e) => {
                        tracing::warn!("commit_previous_system: LLM decomposition failed: {e}");
                        None
                    }
                }
            } else {
                // No structured LLM available — classification will be None.
                // Task 7 will reorder the pipeline so decomposition feeds prediction.
                tracing::debug!(
                    turn_number,
                    combined_text_len = combined_text.len(),
                    "commit_previous_system: no structured LLM, skipping classification"
                );

                None
            }
        });

        // Phase E: Build event atoms and detect compositions
        let (committed_atoms, committed_compounds) = committed_classification
            .as_ref()
            .map(|classification| {
                let scene_id = storyteller_core::types::scene::SceneId::new();
                let turn_id = storyteller_core::types::event::TurnId::new();
                let atoms = crate::context::event_composition::build_event_atoms(
                    classification,
                    scene_id,
                    turn_id,
                );
                let compounds = crate::context::event_composition::detect_compositions(&atoms);
                tracing::debug!(
                    turn_number,
                    atom_count = atoms.len(),
                    compound_count = compounds.len(),
                    "commit_previous_system: Phase E composition detection"
                );
                (atoms, compounds)
            })
            .unwrap_or_default();

        let completed = CompletedTurn {
            turn_number,
            player_input: turn_ctx.player_input.clone().unwrap_or_default(),
            narrator_rendering: turn_ctx.rendering.clone(),
            classification: turn_ctx.classification.clone(),
            committed_classification,
            committed_atoms,
            committed_compounds,
            predictions: turn_ctx.predictions.clone(),
            arbitration: turn_ctx.arbitration.clone(),
            committed_at: chrono::Utc::now(),
        };

        tracing::debug!(
            turn_number,
            has_rendering = completed.narrator_rendering.is_some(),
            has_classification = completed.classification.is_some(),
            has_committed_classification = completed.committed_classification.is_some(),
            atom_count = completed.committed_atoms.len(),
            compound_count = completed.committed_compounds.len(),
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

    stage.0 = stage.0.next(); // → Enriching
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
        app.init_resource::<EnrichmentState>();
        app
    }

    // -----------------------------------------------------------------------
    // Stage condition
    // -----------------------------------------------------------------------

    #[test]
    fn in_stage_condition_matches() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;

        let stage = app.world().resource::<ActiveTurnStage>();
        assert!(stage.0 == TurnCycleStage::Enriching);
    }

    // -----------------------------------------------------------------------
    // enrichment_system
    // -----------------------------------------------------------------------

    #[test]
    fn enrichment_advances_through_all_phases_without_resources() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;

        app.add_systems(Update, enrichment_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AssemblingContext);

        // EnrichmentState should be reset to default
        let enrichment = app.world().resource::<EnrichmentState>();
        assert_eq!(enrichment.0, EnrichmentPhase::EventClassification);

        // Without predictor, predictions should be None; resolver output still set
        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.predictions.is_none());
        assert!(ctx.resolver_output.is_some());
    }

    #[test]
    fn enrichment_sets_arbitration_with_player_input() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;
        app.world_mut().resource_mut::<TurnContext>().player_input =
            Some("I walk through the meadow".to_string());

        app.add_systems(Update, enrichment_system);
        app.update();

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.arbitration.is_some());
        assert!(ctx.arbitration.as_ref().unwrap().is_permitted());
        assert!(ctx.resolver_output.is_some());

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AssemblingContext);
    }

    #[test]
    fn enrichment_skips_arbitration_without_player_input() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;
        // No player input set

        app.add_systems(Update, enrichment_system);
        app.update();

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.arbitration.is_none());
        assert!(ctx.resolver_output.is_some());
    }

    #[test]
    fn enrichment_wraps_predictions_into_resolver_output() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;
        app.world_mut().resource_mut::<TurnContext>().predictions = Some(vec![]);

        app.add_systems(Update, enrichment_system);
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::AssemblingContext);

        let ctx = app.world().resource::<TurnContext>();
        assert!(ctx.resolver_output.is_some());
    }

    #[test]
    fn enrichment_handles_no_predictions() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;
        // predictions is None

        app.add_systems(Update, enrichment_system);
        app.update();

        let ctx = app.world().resource::<TurnContext>();
        let resolver = ctx.resolver_output.as_ref().unwrap();
        assert!(resolver.original_predictions.is_empty());
    }

    #[test]
    fn enrichment_resets_state_after_completion() {
        let mut app = test_app();
        app.world_mut().resource_mut::<ActiveTurnStage>().0 = TurnCycleStage::Enriching;

        app.add_systems(Update, enrichment_system);
        app.update();

        let enrichment = app.world().resource::<EnrichmentState>();
        assert_eq!(
            enrichment.0,
            EnrichmentPhase::EventClassification,
            "EnrichmentState should be reset to default after completion"
        );
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
        assert_eq!(stage.0, TurnCycleStage::Enriching);

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
            intent_statements: None,
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
                enrichment_system.after(commit_previous_system),
                assemble_context_system.after(enrichment_system),
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
                enrichment_system.after(commit_previous_system),
                assemble_context_system.after(enrichment_system),
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
            committed_atoms: vec![],
            committed_compounds: vec![],
            predictions: None,
            arbitration: None,
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

        // No StructuredLlmResource → committed_classification is None
        app.add_systems(Update, commit_previous_system);
        app.update();

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
        // Without StructuredLlmResource, committed_classification is None
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
        // No StructuredLlmResource inserted — committed_classification stays None
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
        // Without StructuredLlmResource, committed_classification is None
        assert!(history.turns[0].committed_classification.is_none());
    }

    #[test]
    fn commit_previous_works_without_structured_llm() {
        // Verifies backward compatibility — the system works without
        // StructuredLlmResource; committed_classification will be None.
        let mut app = App::new();
        app.init_resource::<ActiveTurnStage>();
        app.init_resource::<TurnContext>();
        app.init_resource::<PendingInput>();
        app.init_resource::<TurnHistory>();

        // Set up previous turn data
        {
            let mut ctx = app.world_mut().resource_mut::<TurnContext>();
            ctx.rendering = Some(NarratorRendering {
                text: "The old man nods.".to_string(),
                stage_directions: None,
            });
            ctx.player_input = Some("I nod back.".to_string());
        }
        {
            let mut pending = app.world_mut().resource_mut::<PendingInput>();
            pending.0 = Some("What next?".to_string());
        }
        {
            let mut stage = app.world_mut().resource_mut::<ActiveTurnStage>();
            stage.0 = TurnCycleStage::CommittingPrevious;
        }

        app.add_systems(
            Update,
            commit_previous_system.run_if(in_stage(TurnCycleStage::CommittingPrevious)),
        );
        app.update();

        let stage = app.world().resource::<ActiveTurnStage>();
        assert_eq!(stage.0, TurnCycleStage::Enriching);

        let history = app.world().resource::<TurnHistory>();
        assert_eq!(history.turns.len(), 1);
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
