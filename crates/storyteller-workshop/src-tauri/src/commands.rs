//! Thin gRPC wrapper commands for the Tauri ↔ Svelte boundary.
//!
//! Each command acquires the shared [`StorytellerClient`] mutex, calls the
//! appropriate RPC, translates the proto response to a ts-rs annotated type
//! from [`crate::types`], and returns it to the frontend.

use std::sync::Arc;

use storyteller_client::{
    engine_event, CastMember, ComposeSceneRequest, DynamicPairing, GetSceneStateRequest,
    ResumeSessionRequest, StorytellerClient, SubmitInputRequest,
};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::types::{
    ArchetypeSummary, ContextTokens, DebugEvent, DynamicSummary, GenreOptionsResult, GenreSummary,
    HealthReport, ProfileSummary, ResumeResult, SceneInfo, SceneSelections, SessionInfo,
    SettingSummary, SubsystemStatus, TurnResult, TurnSummary, TurnTiming,
};

/// Shared client state managed by Tauri.
///
/// `Option` because the client connects asynchronously after app startup —
/// Tauri 2's setup callback runs before the async runtime is fully available.
pub type ClientState = Arc<Mutex<Option<StorytellerClient>>>;

const DEBUG_CHANNEL: &str = "workshop:debug";

const NOT_CONNECTED: &str = "Server not connected yet. Is storyteller-server running?";

// ---------------------------------------------------------------------------
// Unary commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn check_health(client: State<'_, ClientState>) -> Result<HealthReport, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let health = c.check_health().await.map_err(|e| e.to_string())?;

    Ok(HealthReport {
        status: health.status.into(),
        subsystems: health
            .subsystems
            .into_iter()
            .map(|s| SubsystemStatus {
                name: s.name,
                status: s.status.into(),
                message: s.message,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn load_catalog(client: State<'_, ClientState>) -> Result<Vec<GenreSummary>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let genre_list = c.list_genres().await.map_err(|e| e.to_string())?;

    Ok(genre_list
        .genres
        .into_iter()
        .map(|g| GenreSummary {
            id: g.entity_id,
            display_name: g.display_name,
            description: g.description,
            archetype_count: g.archetype_count,
            profile_count: g.profile_count,
            dynamic_count: g.dynamic_count,
        })
        .collect())
}

#[tauri::command]
pub async fn get_genre_options(
    genre_id: String,
    selected_archetypes: Vec<String>,
    client: State<'_, ClientState>,
) -> Result<GenreOptionsResult, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let opts = c
        .get_genre_options(&genre_id, selected_archetypes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GenreOptionsResult {
        archetypes: opts
            .archetypes
            .into_iter()
            .map(|a| ArchetypeSummary {
                id: a.entity_id,
                display_name: a.display_name,
                description: a.description,
            })
            .collect(),
        profiles: opts
            .profiles
            .into_iter()
            .map(|p| ProfileSummary {
                id: p.entity_id,
                display_name: p.display_name,
                description: p.description,
                scene_type: p.scene_type,
                tension_min: p.tension_min,
                tension_max: p.tension_max,
                cast_size_min: p.cast_size_min,
                cast_size_max: p.cast_size_max,
            })
            .collect(),
        dynamics: opts
            .dynamics
            .into_iter()
            .map(|d| DynamicSummary {
                id: d.entity_id,
                display_name: d.display_name,
                description: d.description,
                role_a: d.role_a,
                role_b: d.role_b,
            })
            .collect(),
        names: opts.names,
        settings: opts
            .settings
            .into_iter()
            .map(|s| SettingSummary {
                id: s.profile_id,
                name: s.name,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn list_sessions(client: State<'_, ClientState>) -> Result<Vec<SessionInfo>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let session_list = c.list_sessions().await.map_err(|e| e.to_string())?;

    Ok(session_list
        .sessions
        .into_iter()
        .map(|s| SessionInfo {
            session_id: s.session_id,
            genre: s.genre,
            profile: s.profile,
            title: s.title,
            cast_names: s.cast_names,
            turn_count: s.turn_count,
            created_at: s.created_at,
        })
        .collect())
}

#[tauri::command]
pub async fn get_scene_state(
    session_id: String,
    client: State<'_, ClientState>,
) -> Result<serde_json::Value, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let state = c
        .get_scene_state(GetSceneStateRequest { session_id })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(SceneStateJson {
        session_id: state.session_id,
        title: state.title,
        setting_description: state.setting_description,
        characters: state
            .characters
            .into_iter()
            .map(|c| CharacterStateJson {
                entity_id: c.entity_id,
                name: c.name,
                role: c.role,
                performance_notes: c.performance_notes,
            })
            .collect(),
        scene_goals_json: state.scene_goals_json,
        intentions_json: state.intentions_json,
        current_turn: state.current_turn,
    })
    .map_err(|e| e.to_string())
}

/// Internal JSON wrapper for SceneState (not exported to TS — returned as Value).
#[derive(serde::Serialize)]
struct SceneStateJson {
    session_id: String,
    title: String,
    setting_description: String,
    characters: Vec<CharacterStateJson>,
    scene_goals_json: Option<String>,
    intentions_json: Option<String>,
    current_turn: u32,
}

#[derive(serde::Serialize)]
struct CharacterStateJson {
    entity_id: String,
    name: String,
    role: String,
    performance_notes: String,
}

#[tauri::command]
pub async fn get_prediction_history(
    session_id: String,
    client: State<'_, ClientState>,
) -> Result<serde_json::Value, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let response = c
        .get_prediction_history(&session_id, None, None)
        .await
        .map_err(|e| e.to_string())?;

    // The raw_json field contains the prediction history as JSON already
    serde_json::from_str(&response.raw_json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Streaming commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn compose_scene(
    selections: SceneSelections,
    app: AppHandle,
    client: State<'_, ClientState>,
) -> Result<SceneInfo, String> {
    let request = ComposeSceneRequest {
        genre_id: selections.genre_id,
        profile_id: selections.profile_id,
        cast: selections
            .cast
            .into_iter()
            .map(|c| CastMember {
                archetype_id: c.archetype_id,
                name: c.name,
                role: c.role,
            })
            .collect(),
        dynamics: selections
            .dynamics
            .into_iter()
            .map(|d| DynamicPairing {
                dynamic_id: d.dynamic_id,
                cast_index_a: d.cast_index_a,
                cast_index_b: d.cast_index_b,
            })
            .collect(),
        title_override: None,
        setting_override: selections.setting_override,
        seed: selections.seed,
    };

    // Acquire lock, start stream, then drop lock before consuming
    let mut stream = {
        let mut guard = client.lock().await;
        let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
        c.compose_scene(request).await.map_err(|e| e.to_string())?
    };

    let mut session_id = String::new();
    let mut title = String::new();
    let mut setting_description = String::new();
    let mut cast = Vec::new();
    let mut opening_prose = String::new();

    while let Some(event) = stream
        .message()
        .await
        .map_err(|e| format!("stream error: {e}"))?
    {
        let turn = event.turn.unwrap_or(0);
        if session_id.is_empty() {
            session_id = event.session_id.clone();
        }

        if let Some(ref payload) = event.payload {
            if let Some(debug_event) = translate_engine_event(turn, payload) {
                let _ = app.emit(DEBUG_CHANNEL, &debug_event);
            }

            match payload {
                engine_event::Payload::SceneComposed(sc) => {
                    title = sc.title.clone();
                    setting_description = sc.setting_description.clone();
                    cast = sc.cast_names.clone();
                }
                engine_event::Payload::NarratorComplete(nc) => {
                    opening_prose = nc.prose.clone();
                }
                _ => {}
            }
        }
    }

    Ok(SceneInfo {
        session_id,
        title,
        setting_description,
        cast,
        opening_prose,
    })
}

#[tauri::command]
pub async fn submit_input(
    session_id: String,
    input: String,
    app: AppHandle,
    client: State<'_, ClientState>,
) -> Result<TurnResult, String> {
    let request = SubmitInputRequest { session_id, input };

    let mut stream = {
        let mut guard = client.lock().await;
        let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
        c.submit_input(request).await.map_err(|e| e.to_string())?
    };

    let mut turn_number = 0u32;
    let mut narrator_prose = String::new();
    let mut prediction_ms = 0u64;
    let mut assembly_ms = 0u64;
    let mut narrator_ms = 0u64;
    let mut context_tokens = ContextTokens {
        preamble: 0,
        journal: 0,
        retrieved: 0,
        total: 0,
    };

    while let Some(event) = stream
        .message()
        .await
        .map_err(|e| format!("stream error: {e}"))?
    {
        let turn = event.turn.unwrap_or(0);

        if let Some(ref payload) = event.payload {
            if let Some(debug_event) = translate_engine_event(turn, payload) {
                let _ = app.emit(DEBUG_CHANNEL, &debug_event);
            }

            match payload {
                engine_event::Payload::Prediction(p) => {
                    prediction_ms = p.timing_ms;
                }
                engine_event::Payload::Context(c) => {
                    assembly_ms = c.timing_ms;
                    context_tokens = ContextTokens {
                        preamble: c.preamble_tokens,
                        journal: c.journal_tokens,
                        retrieved: c.retrieved_tokens,
                        total: c.total_tokens,
                    };
                }
                engine_event::Payload::NarratorComplete(nc) => {
                    narrator_prose = nc.prose.clone();
                    narrator_ms = nc.generation_ms;
                }
                engine_event::Payload::TurnComplete(tc) => {
                    turn_number = tc.turn;
                }
                _ => {}
            }
        }
    }

    Ok(TurnResult {
        turn: turn_number,
        narrator_prose,
        timing: TurnTiming {
            prediction_ms,
            assembly_ms,
            narrator_ms,
        },
        context_tokens,
    })
}

#[tauri::command]
pub async fn resume_session(
    session_id: String,
    app: AppHandle,
    client: State<'_, ClientState>,
) -> Result<ResumeResult, String> {
    let request = ResumeSessionRequest { session_id };

    let mut stream = {
        let mut guard = client.lock().await;
        let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
        c.resume_session(request).await.map_err(|e| e.to_string())?
    };

    let mut scene_session_id = String::new();
    let mut title = String::new();
    let mut setting_description = String::new();
    let mut cast = Vec::new();
    let mut opening_prose = String::new();
    let mut turns: Vec<TurnSummary> = Vec::new();
    // Track the latest narrator output and player input for turn assembly
    let mut latest_narrator_prose = String::new();
    let mut latest_player_input: Option<String> = None;

    while let Some(event) = stream
        .message()
        .await
        .map_err(|e| format!("stream error: {e}"))?
    {
        let turn = event.turn.unwrap_or(0);
        if scene_session_id.is_empty() {
            scene_session_id = event.session_id.clone();
        }

        if let Some(ref payload) = event.payload {
            if let Some(debug_event) = translate_engine_event(turn, payload) {
                let _ = app.emit(DEBUG_CHANNEL, &debug_event);
            }

            match payload {
                engine_event::Payload::SceneComposed(sc) => {
                    title = sc.title.clone();
                    setting_description = sc.setting_description.clone();
                    cast = sc.cast_names.clone();
                }
                engine_event::Payload::NarratorComplete(nc) => {
                    latest_narrator_prose = nc.prose.clone();
                    latest_player_input = if nc.user_message.is_empty() {
                        None
                    } else {
                        Some(nc.user_message.clone())
                    };
                    // The first narrator output (turn 0 or no turn) is the opening
                    if turn == 0 && opening_prose.is_empty() {
                        opening_prose = nc.prose.clone();
                    }
                }
                engine_event::Payload::TurnComplete(tc) => {
                    turns.push(TurnSummary {
                        turn: tc.turn,
                        player_input: latest_player_input.take(),
                        narrator_output: latest_narrator_prose.clone(),
                    });
                }
                _ => {}
            }
        }
    }

    Ok(ResumeResult {
        scene_info: SceneInfo {
            session_id: scene_session_id,
            title,
            setting_description,
            cast,
            opening_prose,
        },
        turns,
    })
}

// ---------------------------------------------------------------------------
// Engine event translation
// ---------------------------------------------------------------------------

/// Translates a proto [`engine_event::Payload`] variant to the corresponding
/// [`DebugEvent`] for emission on the Tauri event channel.
///
/// Returns `None` for payloads that don't need debug forwarding (e.g.,
/// `SceneComposed`, `TurnComplete`, `NarratorToken`).
fn translate_engine_event(turn: u32, payload: &engine_event::Payload) -> Option<DebugEvent> {
    match payload {
        engine_event::Payload::PhaseStarted(ps) => Some(DebugEvent::PhaseStarted {
            turn,
            phase: ps.phase.clone(),
        }),

        engine_event::Payload::Prediction(p) => Some(DebugEvent::PredictionComplete {
            turn,
            raw_json: p.raw_json.clone(),
            timing_ms: p.timing_ms,
            model_loaded: p.model_loaded,
        }),

        engine_event::Payload::Context(c) => Some(DebugEvent::ContextAssembled {
            turn,
            preamble_text: c.preamble_text.clone(),
            journal_text: c.journal_text.clone(),
            retrieved_text: c.retrieved_text.clone(),
            token_counts: ContextTokens {
                preamble: c.preamble_tokens,
                journal: c.journal_tokens,
                retrieved: c.retrieved_tokens,
                total: c.total_tokens,
            },
            timing_ms: c.timing_ms,
        }),

        engine_event::Payload::NarratorComplete(nc) => Some(DebugEvent::NarratorComplete {
            turn,
            system_prompt: nc.system_prompt.clone(),
            user_message: nc.user_message.clone(),
            raw_response: nc.raw_response.clone(),
            model: nc.model.clone(),
            temperature: nc.temperature,
            max_tokens: nc.max_tokens,
            tokens_used: nc.tokens_used,
            timing_ms: nc.generation_ms,
        }),

        engine_event::Payload::Decomposition(d) => Some(DebugEvent::EventDecomposed {
            turn,
            raw_json: if d.raw_json.is_empty() {
                None
            } else {
                Some(d.raw_json.clone())
            },
            timing_ms: d.timing_ms,
            model: d.model.clone(),
            error: d.error.clone(),
        }),

        engine_event::Payload::IntentSynthesis(i) => Some(DebugEvent::IntentSynthesized {
            turn,
            intent_statements: i.intent_statements.clone(),
            timing_ms: i.timing_ms,
        }),

        engine_event::Payload::Arbitration(a) => Some(DebugEvent::ActionArbitrated {
            turn,
            verdict: a.verdict.clone(),
            details: a.details.clone(),
            player_input: a.player_input.clone(),
            timing_ms: a.timing_ms,
        }),

        engine_event::Payload::Goals(g) => Some(DebugEvent::GoalsGenerated {
            turn,
            scene_goals: g.scene_goals.clone(),
            character_goals: g.character_goals.clone(),
            scene_direction: g.scene_direction.clone(),
            character_drives: g.character_drives.clone(),
            player_context: g.player_context.clone(),
            timing_ms: g.timing_ms,
        }),

        engine_event::Payload::Error(e) => Some(DebugEvent::Error {
            turn,
            phase: e.phase.clone(),
            message: e.message.clone(),
        }),

        // These don't need debug forwarding
        engine_event::Payload::SceneComposed(_)
        | engine_event::Payload::TurnComplete(_)
        | engine_event::Payload::NarratorToken(_) => None,
    }
}
