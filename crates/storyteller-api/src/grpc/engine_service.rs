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

    // --- Stub implementations for remaining RPCs (implemented in Tasks 13-14) ---

    async fn submit_input(
        &self,
        _request: Request<SubmitInputRequest>,
    ) -> Result<Response<Self::SubmitInputStream>, Status> {
        Err(Status::unimplemented("SubmitInput not yet implemented"))
    }

    async fn resume_session(
        &self,
        _request: Request<ResumeSessionRequest>,
    ) -> Result<Response<Self::ResumeSessionStream>, Status> {
        Err(Status::unimplemented("ResumeSession not yet implemented"))
    }

    async fn list_sessions(&self, _request: Request<()>) -> Result<Response<SessionList>, Status> {
        Err(Status::unimplemented("ListSessions not yet implemented"))
    }

    async fn get_scene_state(
        &self,
        _request: Request<GetSceneStateRequest>,
    ) -> Result<Response<SceneState>, Status> {
        Err(Status::unimplemented("GetSceneState not yet implemented"))
    }

    async fn check_llm_status(&self, _request: Request<()>) -> Result<Response<LlmStatus>, Status> {
        Err(Status::unimplemented("CheckLlmStatus not yet implemented"))
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
