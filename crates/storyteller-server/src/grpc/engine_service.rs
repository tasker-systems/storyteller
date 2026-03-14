//! gRPC EngineService implementation.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::engine::{Composition, EngineProviders, EngineStateManager, RuntimeSnapshot};
use crate::persistence::SessionStore;
use crate::proto::storyteller_engine_server::StorytellerEngine;
use crate::proto::*;

use storyteller_composer::{
    compose::{CastSelection, DynamicSelection, SceneSelections},
    goals::ComposedGoals,
    SceneComposer,
};
use storyteller_core::{
    traits::{
        phase_observer::{CollectingObserver, PhaseEventDetail},
        NoopObserver,
    },
    types::{
        capability_lexicon::CapabilityLexicon,
        character::{CharacterSheet, SceneData},
        entity::EntityId,
        prediction::{EmotionalRegister, EventType},
    },
};
use storyteller_engine::{
    agents::narrator::NarratorAgent,
    context::{
        assemble_narrator_context,
        journal::add_turn,
        prediction::{decomposition_to_event_features, predict_character_behaviors},
        DEFAULT_TOTAL_TOKEN_BUDGET,
    },
    inference::{
        event_decomposition::{
            event_decomposition_schema, event_decomposition_system_prompt, EventDecomposition,
        },
        intent_synthesis::synthesize_intents,
        intention_generation::{intentions_to_preamble, GeneratedIntentions},
    },
    systems::arbitration::check_action_possibility,
};
use storyteller_ml::feature_schema::EventFeatureInput;

/// gRPC implementation of the `StorytellerEngine` proto service.
pub struct EngineServiceImpl {
    composer: Arc<SceneComposer>,
    state_manager: Arc<EngineStateManager>,
    session_store: Arc<SessionStore>,
    providers: Arc<EngineProviders>,
}

impl EngineServiceImpl {
    pub fn new(
        composer: Arc<SceneComposer>,
        state_manager: Arc<EngineStateManager>,
        session_store: Arc<SessionStore>,
        providers: Arc<EngineProviders>,
    ) -> Self {
        Self {
            composer,
            state_manager,
            session_store,
            providers,
        }
    }
}

impl std::fmt::Debug for EngineServiceImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineServiceImpl")
            .field("state_manager", &self.state_manager)
            .field("providers", &self.providers)
            .finish()
    }
}

/// Build an `EngineEvent` with a fresh UUIDv7 and current timestamp.
///
/// Free function (not a method) so it's usable inside `tokio::spawn` closures
/// without capturing `self`.
fn make_event(session_id: &str, turn: Option<u32>, payload: engine_event::Payload) -> EngineEvent {
    EngineEvent {
        event_id: Uuid::now_v7().to_string(),
        session_id: session_id.to_string(),
        turn,
        timestamp: Utc::now().to_rfc3339(),
        payload: Some(payload),
    }
}

#[tonic::async_trait]
impl StorytellerEngine for EngineServiceImpl {
    type ComposeSceneStream = ReceiverStream<Result<EngineEvent, Status>>;
    type SubmitInputStream = ReceiverStream<Result<EngineEvent, Status>>;
    type ResumeSessionStream = ReceiverStream<Result<EngineEvent, Status>>;
    type GetSessionEventsStream = ReceiverStream<Result<StoredEvent, Status>>;
    type StreamLogsStream = ReceiverStream<Result<LogEntry, Status>>;

    async fn compose_scene(
        &self,
        request: Request<ComposeSceneRequest>,
    ) -> Result<Response<Self::ComposeSceneStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        let composer = self.composer.clone();
        let state_manager = self.state_manager.clone();
        let session_store = self.session_store.clone();
        let providers = self.providers.clone();

        tokio::spawn(async move {
            let session_id = session_store
                .create_session()
                .unwrap_or_else(|_| Uuid::now_v7().to_string());

            // Phase 1: Composition
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "composition".to_string(),
                    }),
                )))
                .await;

            // Map proto CastMember → composer CastSelection
            let cast: Vec<CastSelection> = req
                .cast
                .into_iter()
                .map(|c| CastSelection {
                    archetype_id: c.archetype_id,
                    name: c.name,
                    role: c.role,
                })
                .collect();

            // Map proto DynamicPairing → composer DynamicSelection
            let dynamics: Vec<DynamicSelection> = req
                .dynamics
                .into_iter()
                .map(|d| DynamicSelection {
                    dynamic_id: d.dynamic_id,
                    cast_index_a: d.cast_index_a as usize,
                    cast_index_b: d.cast_index_b as usize,
                })
                .collect();

            let selections = SceneSelections {
                genre_id: req.genre_id,
                profile_id: req.profile_id,
                cast,
                dynamics,
                title_override: req.title_override,
                setting_override: req.setting_override,
                seed: req.seed,
            };

            let composed = match composer.compose(&selections) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx
                        .send(Ok(make_event(
                            &session_id,
                            Some(0),
                            engine_event::Payload::Error(ErrorOccurred {
                                phase: "composition".to_string(),
                                message: e,
                            }),
                        )))
                        .await;
                    return;
                }
            };

            let cast_names: Vec<String> =
                composed.characters.iter().map(|c| c.name.clone()).collect();

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::SceneComposed(SceneComposed {
                        title: composed.scene.title.clone(),
                        setting_description: composed.scene.setting.description.clone(),
                        cast_names: cast_names.clone(),
                        composition_json: serde_json::to_string(&composed).unwrap_or_default(),
                    }),
                )))
                .await;

            // Goal intersection
            let goals = composer.intersect_goals(&selections, &composed);

            // Persist composition to session directory
            let composition_value = serde_json::json!({
                "selections": selections,
                "scene": composed.scene,
                "characters": composed.characters,
                "goals": goals,
                "intentions": null,
            });

            if let Err(e) = session_store
                .composition
                .write(&session_id, &composition_value)
            {
                tracing::error!("Failed to persist composition for session {session_id}: {e}");
            }

            // Emit goals event
            let scene_goal_strs: Vec<String> = goals
                .scene_goals
                .iter()
                .map(|g| format!("{} ({})", g.goal_id, g.category))
                .collect();

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::Goals(GoalsGenerated {
                        scene_goals: scene_goal_strs,
                        character_goals: vec![],
                        scene_direction: None,
                        character_drives: None,
                        player_context: None,
                        timing_ms: 0,
                    }),
                )))
                .await;

            // Phase 2: Opening Narrator
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "narrator".to_string(),
                    }),
                )))
                .await;

            // Assemble opening context from composed data
            let characters_refs: Vec<&CharacterSheet> = composed.characters.iter().collect();
            let entity_ids: Vec<EntityId> =
                composed.characters.iter().map(|c| c.entity_id).collect();

            // Find player character: protagonist role
            let player_entity_id = composed
                .scene
                .cast
                .iter()
                .find(|c| c.role.to_lowercase().contains("protagonist"))
                .map(|c| c.entity_id);

            let opening_journal = storyteller_core::types::narrator_context::SceneJournal::new(
                composed.scene.scene_id,
                1200,
            );
            let opening_resolver = storyteller_core::types::resolver::ResolverOutput {
                sequenced_actions: vec![],
                original_predictions: vec![],
                scene_dynamics: "Opening — the scene is just beginning".to_string(),
                conflicts: vec![],
                intent_statements: None,
            };

            let obs = CollectingObserver::new();
            let context = assemble_narrator_context(
                &composed.scene,
                &characters_refs,
                &opening_journal,
                &opening_resolver,
                "",
                &entity_ids,
                DEFAULT_TOTAL_TOKEN_BUDGET,
                &obs,
                player_entity_id,
            );

            let narrator = NarratorAgent::new(&context, Arc::clone(&providers.narrator_llm))
                .with_temperature(0.8);
            let noop = NoopObserver;
            let llm_start = Instant::now();
            let opening_prose = match narrator.render_opening(&noop).await {
                Ok(r) => r.text,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        session = %session_id,
                        "Opening narrator render failed — degrading to placeholder"
                    );
                    "[The scene begins.]".to_string()
                }
            };
            let narrator_ms = llm_start.elapsed().as_millis() as u64;

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::NarratorComplete(NarratorComplete {
                        prose: opening_prose.clone(),
                        generation_ms: narrator_ms,
                    }),
                )))
                .await;

            // Register session in EngineStateManager
            let composition = Composition {
                scene: serde_json::to_value(&composed.scene).unwrap_or_default(),
                characters: composed
                    .characters
                    .iter()
                    .map(|c| serde_json::to_value(c).unwrap_or_default())
                    .collect(),
                goals: Some(serde_json::to_value(&goals).unwrap_or_default()),
                intentions: None,
                selections: serde_json::to_value(&selections).unwrap_or_default(),
            };
            state_manager.create_session(&session_id, composition);

            // Update initial snapshot: record turn 0 journal entry and player entity
            let noop_journal = NoopObserver;
            let mut opening_journal_with_prose = opening_journal;
            add_turn(
                &mut opening_journal_with_prose,
                0,
                &opening_prose,
                entity_ids,
                vec![],
                &noop_journal,
            );
            state_manager
                .update_runtime_snapshot(&session_id, move |_snap| RuntimeSnapshot {
                    turn_count: 0,
                    player_entity_id,
                    journal: opening_journal_with_prose,
                    prediction_history: Default::default(),
                })
                .await;

            // Turn complete
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::TurnComplete(TurnComplete {
                        turn: 0,
                        total_ms: narrator_ms,
                    }),
                )))
                .await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn submit_input(
        &self,
        request: Request<SubmitInputRequest>,
    ) -> Result<Response<Self::SubmitInputStream>, Status> {
        let req = request.into_inner();
        let session_id = req.session_id.clone();
        let input = req.input;

        if !self.state_manager.has_session(&session_id) {
            return Err(Status::not_found("session not found"));
        }

        let (tx, rx) = mpsc::channel(32);
        let state_manager = self.state_manager.clone();
        let session_store = self.session_store.clone();
        let providers = self.providers.clone();

        tokio::spawn(async move {
            let total_start = Instant::now();
            let mut event_ids: Vec<String> = Vec::new();

            // --- Load composition (immutable) ---
            let composition = match state_manager.get_composition(&session_id) {
                Some(c) => c,
                None => {
                    let _ = tx
                        .send(Err(Status::not_found("session composition not found")))
                        .await;
                    return;
                }
            };

            // Deserialize typed domain objects from JSON
            let scene: SceneData = match serde_json::from_value(composition.scene.clone()) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx
                        .send(Err(Status::internal(format!("Invalid scene data: {e}"))))
                        .await;
                    return;
                }
            };
            let characters: Vec<CharacterSheet> = match composition
                .characters
                .iter()
                .map(|v| serde_json::from_value(v.clone()))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(cs) => cs,
                Err(e) => {
                    let _ = tx
                        .send(Err(Status::internal(format!(
                            "Invalid character data: {e}"
                        ))))
                        .await;
                    return;
                }
            };
            let characters_refs: Vec<&CharacterSheet> = characters.iter().collect();
            let entity_ids: Vec<EntityId> = characters.iter().map(|c| c.entity_id).collect();

            // Optional goal types (graceful degradation if missing/malformed)
            let composed_goals: Option<ComposedGoals> = composition
                .goals
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let generated_intentions: Option<GeneratedIntentions> = composition
                .intentions
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            // --- Load runtime snapshot ---
            let snapshot = match state_manager.get_runtime_snapshot(&session_id) {
                Some(s) => s,
                None => {
                    let _ = tx
                        .send(Err(Status::not_found("session snapshot not found")))
                        .await;
                    return;
                }
            };
            let turn = snapshot.turn_count + 1;

            // Derive player entity ID from scene cast (protagonist role)
            let player_entity_id = scene
                .cast
                .iter()
                .find(|c| c.role.to_lowercase().contains("protagonist"))
                .map(|c| c.entity_id)
                .or(snapshot.player_entity_id);

            // --- Phase 1: Event Decomposition ---
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "decomposition".to_string(),
                    }),
                )))
                .await;

            let decomp_start = Instant::now();
            let mut persisted_decomposition: Option<serde_json::Value> = None;
            let mut event_decomposition: Option<EventDecomposition> = None;

            if let Some(ref structured_llm) = providers.structured_llm {
                // Include last narrator prose so the model can resolve pronouns
                let decomp_input = if let Some(last_entry) = snapshot.journal.entries.last() {
                    format!("[Narrator]\n{}\n\n[Player]\n{}", last_entry.content, input)
                } else {
                    input.clone()
                };

                let request = storyteller_core::traits::structured_llm::StructuredRequest {
                    system: event_decomposition_system_prompt(),
                    input: decomp_input,
                    output_schema: event_decomposition_schema(),
                    temperature: 0.1,
                };
                match structured_llm.extract(request).await {
                    Ok(raw_json) => {
                        tracing::debug!(
                            raw = %serde_json::to_string(&raw_json).unwrap_or_default(),
                            "Event decomposition raw response"
                        );
                        persisted_decomposition = Some(raw_json.clone());
                        event_decomposition = EventDecomposition::from_json(&raw_json).ok();
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Event decomposition LLM call failed");
                    }
                }
            }
            let _decomposition_ms = decomp_start.elapsed().as_millis() as u64;

            let decomp_json = persisted_decomposition
                .clone()
                .unwrap_or_else(|| serde_json::json!({"status": "skipped"}));
            if let Ok(eid) =
                session_store
                    .events
                    .append(&session_id, "decomposition", Some(turn), &decomp_json)
            {
                event_ids.push(eid);
            }

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Decomposition(DecompositionComplete {
                        raw_json: serde_json::to_string(&decomp_json).unwrap_or_default(),
                    }),
                )))
                .await;

            // Derive event features from decomposition for ML prediction
            let event_features = if let Some(ref decomposition) = event_decomposition {
                decomposition_to_event_features(decomposition)
            } else {
                EventFeatureInput {
                    event_type: EventType::Interaction,
                    emotional_register: EmotionalRegister::Neutral,
                    confidence: 0.5,
                    target_count: characters.len().saturating_sub(1) as u8,
                }
            };

            // --- Phase 2: ML Prediction ---
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "prediction".to_string(),
                    }),
                )))
                .await;

            let predict_start = Instant::now();
            let mut resolver_output = if let Some(ref predictor) = providers.predictor {
                let predictions = predict_character_behaviors(
                    predictor,
                    &characters_refs,
                    &scene,
                    &input,
                    &*providers.grammar,
                    event_features,
                    snapshot.prediction_history.as_map(),
                );
                storyteller_core::types::resolver::ResolverOutput {
                    sequenced_actions: vec![],
                    original_predictions: predictions,
                    scene_dynamics: "ML-predicted character behavior".to_string(),
                    conflicts: vec![],
                    intent_statements: None,
                }
            } else {
                storyteller_core::types::resolver::ResolverOutput {
                    sequenced_actions: vec![],
                    original_predictions: vec![],
                    scene_dynamics: "Character behavior (ML predictor not available)".to_string(),
                    conflicts: vec![],
                    intent_statements: None,
                }
            };
            let _prediction_ms = predict_start.elapsed().as_millis() as u64;

            // Build updated prediction history from this turn's predictions
            let mut new_prediction_history = snapshot.prediction_history.clone();
            for pred in &resolver_output.original_predictions {
                new_prediction_history.push_from_prediction(pred);
            }

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Prediction(PredictionComplete {
                        raw_json: serde_json::to_string(&resolver_output.original_predictions)
                            .unwrap_or_default(),
                    }),
                )))
                .await;

            // --- Phase 3: Action Arbitration ---
            let arb_start = Instant::now();
            let arbitration_result =
                check_action_possibility(&input, &[], &CapabilityLexicon::new(), None);
            let arb_ms = arb_start.elapsed().as_millis() as u64;

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Arbitration(ArbitrationComplete {
                        verdict: format!("{:?}", arbitration_result),
                        details: format!("Arbitration completed in {arb_ms}ms"),
                    }),
                )))
                .await;

            // --- Phase 4: Intent Synthesis ---
            let intent_start = Instant::now();
            let journal_tail = snapshot
                .journal
                .entries
                .iter()
                .rev()
                .take(2)
                .rev()
                .map(|e| e.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");

            let intent_statements = if let Some(ref intent_llm) = providers.intent_llm {
                synthesize_intents(
                    intent_llm.as_ref(),
                    &characters_refs,
                    &resolver_output.original_predictions,
                    &journal_tail,
                    &input,
                    &scene,
                    player_entity_id,
                    generated_intentions.as_ref(),
                )
                .await
            } else {
                None
            };
            let _intent_ms = intent_start.elapsed().as_millis() as u64;

            resolver_output.intent_statements = intent_statements.clone();

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::IntentSynthesis(IntentSynthesisComplete {
                        intent_statements: intent_statements.unwrap_or_default(),
                    }),
                )))
                .await;

            // --- Phase 5: Context Assembly ---
            let observer = CollectingObserver::new();
            let assembly_start = Instant::now();
            let emotional_markers = extract_emotional_markers(&input);

            let mut context = assemble_narrator_context(
                &scene,
                &characters_refs,
                &snapshot.journal,
                &resolver_output,
                &input,
                &entity_ids,
                DEFAULT_TOTAL_TOKEN_BUDGET,
                &observer,
                player_entity_id,
            );
            let _assembly_ms = assembly_start.elapsed().as_millis() as u64;

            // Inject composition-time goal intentions into preamble
            if let Some(ref intentions) = generated_intentions {
                let (scene_direction, character_drives) = intentions_to_preamble(intentions);
                context.preamble.scene_direction = Some(scene_direction);
                context.preamble.character_drives = character_drives;
            }
            // Inject player context from composed goals
            if let Some(ref goals) = composed_goals {
                use storyteller_composer::goals::GoalVisibility;
                context.preamble.player_context = player_entity_id
                    .and_then(|pid| goals.character_goals.get(&pid))
                    .map(|player_goals| {
                        player_goals
                            .iter()
                            .filter(|g| {
                                matches!(
                                    g.visibility,
                                    GoalVisibility::Overt | GoalVisibility::Signaled
                                )
                            })
                            .map(|g| g.goal_id.replace('_', " "))
                            .collect::<Vec<_>>()
                            .join("; ")
                    })
                    .filter(|s| !s.is_empty());
            }

            // Extract token counts from observer (drains events)
            let token_counts = extract_token_counts_from_observer(&observer);

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Context(ContextAssembled {
                        preamble_tokens: token_counts.0,
                        journal_tokens: token_counts.1,
                        retrieved_tokens: token_counts.2,
                        total_tokens: token_counts.3,
                    }),
                )))
                .await;

            // --- Phase 6: Narrator Rendering ---
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "narrator".to_string(),
                    }),
                )))
                .await;

            let narrator = NarratorAgent::new(&context, Arc::clone(&providers.narrator_llm))
                .with_temperature(0.8);
            let noop = NoopObserver;
            let llm_start = Instant::now();
            let rendering = match narrator.render(&context, &noop).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        session = %session_id,
                        "Narrator render failed"
                    );
                    let _ = tx
                        .send(Err(Status::internal(format!("Narrator failed: {e}"))))
                        .await;
                    return;
                }
            };
            let narrator_ms = llm_start.elapsed().as_millis() as u64;

            if let Ok(eid) = session_store.events.append(
                &session_id,
                "narrator_complete",
                Some(turn),
                &serde_json::json!({"prose": rendering.text}),
            ) {
                event_ids.push(eid);
            }

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::NarratorComplete(NarratorComplete {
                        prose: rendering.text.clone(),
                        generation_ms: narrator_ms,
                    }),
                )))
                .await;

            // --- Update journal (clone from snapshot, add new turn) ---
            let noop_journal = NoopObserver;
            let mut new_journal = snapshot.journal.clone();
            add_turn(
                &mut new_journal,
                turn,
                &rendering.text,
                entity_ids,
                emotional_markers,
                &noop_journal,
            );

            // --- Update runtime snapshot ---
            state_manager
                .update_runtime_snapshot(&session_id, move |_snap| RuntimeSnapshot {
                    turn_count: turn,
                    player_entity_id,
                    journal: new_journal,
                    prediction_history: new_prediction_history,
                })
                .await;

            // --- Persist turn ---
            let turn_entry = crate::persistence::TurnEntry {
                turn,
                timestamp: Utc::now().to_rfc3339(),
                player_input: Some(input),
                event_ids,
            };
            let _ = session_store.turns.append(&session_id, &turn_entry);

            let total_ms = total_start.elapsed().as_millis() as u64;
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::TurnComplete(TurnComplete { turn, total_ms }),
                )))
                .await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn resume_session(
        &self,
        request: Request<ResumeSessionRequest>,
    ) -> Result<Response<Self::ResumeSessionStream>, Status> {
        let session_id = request.into_inner().session_id;
        let (tx, rx) = mpsc::channel(32);

        let state_manager = self.state_manager.clone();
        let session_store = self.session_store.clone();

        tokio::spawn(async move {
            // Load composition
            let comp = match session_store.composition.read(&session_id) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx
                        .send(Ok(make_event(
                            &session_id,
                            None,
                            engine_event::Payload::Error(ErrorOccurred {
                                phase: "resume".to_string(),
                                message: format!("load composition: {e}"),
                            }),
                        )))
                        .await;
                    return;
                }
            };

            // Hydrate composition
            let composition = crate::engine::Composition {
                scene: comp.get("scene").cloned().unwrap_or_default(),
                characters: comp
                    .get("characters")
                    .and_then(|c| c.as_array())
                    .map(|a| a.to_vec())
                    .unwrap_or_default(),
                goals: comp.get("goals").cloned(),
                intentions: comp.get("intentions").cloned(),
                selections: comp.get("selections").cloned().unwrap_or_default(),
            };

            // Emit SceneComposed
            let cast_names: Vec<String> = composition
                .characters
                .iter()
                .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
                .map(|s| s.to_string())
                .collect();

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    None,
                    engine_event::Payload::SceneComposed(SceneComposed {
                        title: composition
                            .scene
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string(),
                        setting_description: composition
                            .scene
                            .get("setting")
                            .and_then(|s| s.get("description"))
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        cast_names,
                        composition_json: String::new(),
                    }),
                )))
                .await;

            // Load turns and reconstruct state
            let turns = session_store
                .turns
                .read_all(&session_id)
                .unwrap_or_default();
            let turn_count = turns.len() as u32;

            state_manager.create_session(&session_id, composition);
            state_manager
                .update_runtime_snapshot(&session_id, |snap| {
                    let mut new = snap.clone();
                    new.turn_count = turn_count;
                    new
                })
                .await;

            // Emit TurnComplete for each existing turn
            for turn in &turns {
                let _ = tx
                    .send(Ok(make_event(
                        &session_id,
                        Some(turn.turn),
                        engine_event::Payload::TurnComplete(TurnComplete {
                            turn: turn.turn,
                            total_ms: 0,
                        }),
                    )))
                    .await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list_sessions(&self, _request: Request<()>) -> Result<Response<SessionList>, Status> {
        let session_ids = self
            .session_store
            .list_session_ids()
            .map_err(|e| Status::internal(format!("list sessions: {e}")))?;

        let mut summaries = Vec::new();
        for id in session_ids {
            if let Ok(comp) = self.session_store.composition.read(&id) {
                let turn_count = self.session_store.turns.turn_count(&id).unwrap_or(0) as u32;

                summaries.push(SessionSummary {
                    session_id: id,
                    genre: comp
                        .get("selections")
                        .and_then(|s| s.get("genre_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    profile: comp
                        .get("selections")
                        .and_then(|s| s.get("profile_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    title: comp
                        .get("scene")
                        .and_then(|s| s.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled")
                        .to_string(),
                    cast_names: comp
                        .get("characters")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                    turn_count,
                    created_at: String::new(), // TODO: from directory mtime
                });
            }
        }

        Ok(Response::new(SessionList {
            sessions: summaries,
        }))
    }

    async fn get_scene_state(
        &self,
        request: Request<GetSceneStateRequest>,
    ) -> Result<Response<SceneState>, Status> {
        let session_id = &request.get_ref().session_id;

        let composition = self
            .state_manager
            .get_composition(session_id)
            .ok_or_else(|| Status::not_found(format!("session {session_id} not found")))?;

        let snapshot = self.state_manager.get_runtime_snapshot(session_id);

        let characters: Vec<CharacterState> = composition
            .characters
            .iter()
            .filter_map(|c| {
                Some(CharacterState {
                    entity_id: c.get("entity_id")?.as_str()?.to_string(),
                    name: c.get("name")?.as_str()?.to_string(),
                    role: c
                        .get("backstory")
                        .and_then(|b| b.as_str())
                        .unwrap_or("")
                        .to_string(),
                    performance_notes: c
                        .get("performance_notes")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect();

        let current_turn = snapshot.map(|s| s.turn_count).unwrap_or(0);

        Ok(Response::new(SceneState {
            session_id: session_id.to_string(),
            title: composition
                .scene
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            setting_description: composition
                .scene
                .get("setting")
                .and_then(|s| s.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string(),
            characters,
            scene_goals_json: composition.goals.as_ref().map(|g| g.to_string()),
            intentions_json: composition.intentions.as_ref().map(|i| i.to_string()),
            current_turn,
        }))
    }

    async fn check_health(
        &self,
        _request: Request<()>,
    ) -> Result<Response<crate::proto::HealthResponse>, Status> {
        let providers = &self.providers;

        let mut subsystems = vec![crate::proto::SubsystemHealth {
            name: "narrator_llm".to_string(),
            status: "healthy".to_string(),
            message: None,
        }];

        subsystems.push(crate::proto::SubsystemHealth {
            name: "structured_llm".to_string(),
            status: if providers.structured_llm.is_some() {
                "healthy".to_string()
            } else {
                "unavailable".to_string()
            },
            message: if providers.structured_llm.is_none() {
                Some("Structured LLM provider not configured".to_string())
            } else {
                None
            },
        });

        subsystems.push(crate::proto::SubsystemHealth {
            name: "intent_llm".to_string(),
            status: if providers.intent_llm.is_some() {
                "healthy".to_string()
            } else {
                "unavailable".to_string()
            },
            message: if providers.intent_llm.is_none() {
                Some("Intent LLM provider not configured".to_string())
            } else {
                None
            },
        });

        subsystems.push(crate::proto::SubsystemHealth {
            name: "predictor".to_string(),
            status: if providers.predictor.is_some() {
                "healthy".to_string()
            } else {
                "unavailable".to_string()
            },
            message: if providers.predictor.is_none() {
                Some("Character predictor model not loaded".to_string())
            } else {
                None
            },
        });

        // Rollup: any unavailable subsystem → server is degraded (still up)
        let has_unavailable = subsystems.iter().any(|s| s.status == "unavailable");
        let overall_status = if has_unavailable {
            "degraded".to_string()
        } else {
            "healthy".to_string()
        };

        Ok(Response::new(crate::proto::HealthResponse {
            status: overall_status,
            subsystems,
        }))
    }

    async fn get_prediction_history(
        &self,
        _request: Request<PredictionHistoryRequest>,
    ) -> Result<Response<PredictionHistoryResponse>, Status> {
        Err(Status::unimplemented(
            "GetPredictionHistory not yet implemented",
        ))
    }

    async fn get_session_events(
        &self,
        _request: Request<SessionEventsRequest>,
    ) -> Result<Response<Self::GetSessionEventsStream>, Status> {
        Err(Status::unimplemented(
            "GetSessionEvents not yet implemented",
        ))
    }

    async fn stream_logs(
        &self,
        _request: Request<LogFilter>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        Err(Status::unimplemented("StreamLogs not yet implemented"))
    }
}

// ---------------------------------------------------------------------------
// Private pipeline helpers
// ---------------------------------------------------------------------------

/// Extract rough emotional markers from player input.
///
/// Naive prototype implementation — looks for emotionally charged words.
/// Mirrors the workshop `extract_emotional_markers`.
fn extract_emotional_markers(input: &str) -> Vec<String> {
    let lower = input.to_lowercase();
    let mut markers = Vec::new();
    let emotional_words = [
        ("cry", "sadness"),
        ("weep", "sadness"),
        ("tear", "sadness"),
        ("laugh", "joy"),
        ("smile", "joy"),
        ("angry", "anger"),
        ("shout", "anger"),
        ("afraid", "fear"),
        ("scared", "fear"),
        ("surprise", "surprise"),
        ("wonder", "anticipation"),
        ("hope", "anticipation"),
        ("flute", "recognition"),
        ("music", "recognition"),
        ("remember", "recognition"),
    ];
    for (word, marker) in &emotional_words {
        if lower.contains(word) {
            markers.push((*marker).to_string());
        }
    }
    markers
}

/// Extract token counts from a `CollectingObserver` after context assembly.
///
/// Returns `(preamble, journal, retrieved, total)`. Drains the observer's
/// event buffer — call this once after `assemble_narrator_context`.
fn extract_token_counts_from_observer(observer: &CollectingObserver) -> (u32, u32, u32, u32) {
    let events = observer.take_events();
    for event in &events {
        if let PhaseEventDetail::ContextAssembled {
            preamble_tokens,
            journal_tokens,
            retrieved_tokens,
            total_tokens,
            ..
        } = &event.detail
        {
            return (
                *preamble_tokens,
                *journal_tokens,
                *retrieved_tokens,
                *total_tokens,
            );
        }
    }
    (0, 0, 0, 0)
}
