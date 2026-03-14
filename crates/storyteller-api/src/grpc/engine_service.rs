//! gRPC EngineService implementation.

use std::sync::Arc;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::engine::{Composition, EngineProviders, EngineStateManager};
use crate::persistence::SessionStore;
use crate::proto::storyteller_engine_server::StorytellerEngine;
use crate::proto::*;

use storyteller_composer::{
    compose::{CastSelection, DynamicSelection, SceneSelections},
    SceneComposer,
};

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
        // providers cloned for future LLM wiring (Tasks 12+)
        let _providers = self.providers.clone();

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

            // Phase 2: Narrator (placeholder — LLM integration in Tasks 12+)
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "narrator".to_string(),
                    }),
                )))
                .await;

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::NarratorComplete(NarratorComplete {
                        prose: "[Narrator LLM integration pending — scene composed successfully]"
                            .to_string(),
                        generation_ms: 0,
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

            // Turn complete
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(0),
                    engine_event::Payload::TurnComplete(TurnComplete {
                        turn: 0,
                        total_ms: 0,
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
        // providers cloned for future LLM wiring
        let _providers = self.providers.clone();

        tokio::spawn(async move {
            let mut event_ids: Vec<String> = Vec::new();
            let snapshot = state_manager.get_runtime_snapshot(&session_id);
            let turn = snapshot.as_ref().map(|s| s.turn_count + 1).unwrap_or(1);

            // Phase 1: Event Decomposition
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "decomposition".to_string(),
                    }),
                )))
                .await;

            // TODO: Call decompose_events() with structured LLM
            if let Ok(eid) = session_store.events.append(
                &session_id,
                "decomposition",
                Some(turn),
                &serde_json::json!({"input": input, "status": "placeholder"}),
            ) {
                event_ids.push(eid);
            }

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Decomposition(DecompositionComplete {
                        raw_json: serde_json::json!({"status": "placeholder"}).to_string(),
                    }),
                )))
                .await;

            // Phase 2: ML Prediction
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "prediction".to_string(),
                    }),
                )))
                .await;

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Prediction(PredictionComplete {
                        raw_json: serde_json::json!({"status": "placeholder"}).to_string(),
                    }),
                )))
                .await;

            // Phase 3: Arbitration
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Arbitration(ArbitrationComplete {
                        verdict: "Permitted".to_string(),
                        details: "placeholder".to_string(),
                    }),
                )))
                .await;

            // Phase 4: Intent Synthesis
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::IntentSynthesis(IntentSynthesisComplete {
                        intent_statements: String::new(),
                    }),
                )))
                .await;

            // Phase 5: Context Assembly
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::Context(ContextAssembled {
                        preamble_tokens: 0,
                        journal_tokens: 0,
                        retrieved_tokens: 0,
                        total_tokens: 0,
                    }),
                )))
                .await;

            // Phase 6: Narrator
            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::PhaseStarted(PhaseStarted {
                        phase: "narrator".to_string(),
                    }),
                )))
                .await;

            // TODO: Call narrator LLM with assembled context
            let narrator_output = "[Narrator pipeline integration pending]".to_string();

            if let Ok(eid) = session_store.events.append(
                &session_id,
                "narrator_complete",
                Some(turn),
                &serde_json::json!({"prose": narrator_output}),
            ) {
                event_ids.push(eid);
            }

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::NarratorComplete(NarratorComplete {
                        prose: narrator_output.clone(),
                        generation_ms: 0,
                    }),
                )))
                .await;

            // Update state
            state_manager
                .update_runtime_snapshot(&session_id, |snap| {
                    let mut new = snap.clone();
                    new.turn_count = turn;
                    new.journal_entries.push(narrator_output.clone());
                    new
                })
                .await;

            // Persist turn
            let turn_entry = crate::persistence::TurnEntry {
                turn,
                timestamp: Utc::now().to_rfc3339(),
                player_input: Some(input),
                event_ids,
            };
            let _ = session_store.turns.append(&session_id, &turn_entry);

            let _ = tx
                .send(Ok(make_event(
                    &session_id,
                    Some(turn),
                    engine_event::Payload::TurnComplete(TurnComplete { turn, total_ms: 0 }),
                )))
                .await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn resume_session(
        &self,
        _request: Request<ResumeSessionRequest>,
    ) -> Result<Response<Self::ResumeSessionStream>, Status> {
        Err(Status::unimplemented("ResumeSession not yet implemented"))
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

    async fn check_llm_status(&self, _request: Request<()>) -> Result<Response<LlmStatus>, Status> {
        Ok(Response::new(LlmStatus {
            narrator_available: true, // placeholder
            narrator_model: String::new(),
            decomposition_available: self.providers.structured_llm.is_some(),
            decomposition_model: String::new(),
            intent_available: self.providers.intent_llm.is_some(),
            intent_model: String::new(),
            predictor_available: self.providers.predictor_available,
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
