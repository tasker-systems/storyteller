//! Tauri commands — the bridge between the frontend and the storyteller engine.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tokio::sync::Mutex;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::phase_observer::{CollectingObserver, PhaseEventDetail};
use storyteller_core::traits::structured_llm::StructuredLlmConfig;
use storyteller_core::traits::NoopObserver;
use storyteller_core::types::capability_lexicon::CapabilityLexicon;
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::scene::SceneId;
use storyteller_engine::agents::narrator::NarratorAgent;
use storyteller_engine::context::journal::add_turn;
use storyteller_engine::context::prediction::{
    decomposition_to_event_features, predict_character_behaviors,
};
use storyteller_engine::context::{assemble_narrator_context, DEFAULT_TOTAL_TOKEN_BUDGET};
use storyteller_engine::inference::event_decomposition::{
    event_decomposition_schema, event_decomposition_system_prompt, EventDecomposition,
};
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};
use storyteller_engine::inference::frame::CharacterPredictor;
use storyteller_engine::inference::structured::OllamaStructuredProvider;
use storyteller_engine::scene_composer::{
    ComposedGoals, ComposedScene, SceneComposer, SceneSelections,
};
use storyteller_engine::systems::arbitration::check_action_possibility;
use storyteller_ml::prediction_history::PredictionHistory;

use crate::engine_state::EngineState;
use crate::events::{DebugEvent, TokenCounts, DEBUG_EVENT_CHANNEL};
use crate::session::{SessionStore, SessionSummary, TurnRecord};
use crate::session_log::{ContextAssemblyLog, LogEntry, SessionLog, TimingLog};

// ---------------------------------------------------------------------------
// Return types (serialized to JSON for the frontend)
// ---------------------------------------------------------------------------

/// Information about the loaded scene, returned by `start_scene`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInfo {
    /// Scene title.
    pub title: String,
    /// Setting description.
    pub setting_description: String,
    /// Cast member names.
    pub cast: Vec<String>,
    /// The narrator's opening prose.
    pub opening_prose: String,
}

/// Result of resuming a session, returned by `resume_session`.
#[derive(Debug, Clone, Serialize)]
pub struct ResumeResult {
    /// Scene metadata (title, setting, cast, opening prose).
    pub scene_info: SceneInfo,
    /// All turns played so far (for frontend chat hydration).
    pub turns: Vec<TurnSummary>,
}

/// Minimal turn data for frontend chat hydration.
#[derive(Debug, Clone, Serialize)]
pub struct TurnSummary {
    /// Turn number (0 = opening narration).
    pub turn: u32,
    /// Player input text (None for turn 0).
    pub player_input: Option<String>,
    /// Narrator's rendered prose.
    pub narrator_output: String,
}

/// Result of a single player turn, returned by `submit_input`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnResult {
    /// Turn number.
    pub turn: u32,
    /// The narrator's rendered prose.
    pub narrator_prose: String,
    /// Phase timing.
    pub timing: TurnTiming,
    /// Context assembly token breakdown.
    pub context_tokens: ContextTokens,
}

/// Phase timing in milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnTiming {
    /// ML prediction time.
    pub prediction_ms: u64,
    /// Context assembly time.
    pub assembly_ms: u64,
    /// Narrator LLM call time.
    pub narrator_ms: u64,
}

/// Token counts from context assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTokens {
    /// Tier 1 preamble tokens.
    pub preamble: u32,
    /// Tier 2 journal tokens.
    pub journal: u32,
    /// Tier 3 retrieved context tokens.
    pub retrieved: u32,
    /// Total estimated tokens.
    pub total: u32,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Result of an LLM health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStatus {
    /// Whether the server responded to a health probe.
    pub reachable: bool,
    /// Base URL of the configured server.
    pub endpoint: String,
    /// Model name configured.
    pub model: String,
    /// Provider type (e.g., "Ollama").
    pub provider: String,
    /// Available models on the server (if reachable).
    pub available_models: Vec<String>,
    /// Error message if unreachable.
    pub error: Option<String>,
    /// Probe latency in milliseconds.
    pub latency_ms: u64,
}

/// Probe the configured LLM server for reachability before starting a scene.
#[tauri::command]
pub async fn check_llm() -> Result<LlmStatus, String> {
    let config = ExternalServerConfig::default();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let start = Instant::now();
    let tags_url = format!("{}/api/tags", config.base_url);

    match client.get(&tags_url).send().await {
        Ok(response) if response.status().is_success() => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let available_models = match response.json::<serde_json::Value>().await {
                Ok(json) => json
                    .get("models")
                    .and_then(|m| m.as_array())
                    .map(|models| {
                        models
                            .iter()
                            .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default(),
                Err(_) => vec![],
            };

            Ok(LlmStatus {
                reachable: true,
                endpoint: config.base_url,
                model: config.model,
                provider: "Ollama".to_string(),
                available_models,
                error: None,
                latency_ms,
            })
        }
        Ok(response) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status = response.status();
            Ok(LlmStatus {
                reachable: false,
                endpoint: config.base_url,
                model: config.model,
                provider: "Ollama".to_string(),
                available_models: vec![],
                error: Some(format!("Server returned HTTP {status}")),
                latency_ms,
            })
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            Ok(LlmStatus {
                reachable: false,
                endpoint: config.base_url,
                model: config.model,
                provider: "Ollama".to_string(),
                available_models: vec![],
                error: Some(format!("{e}")),
                latency_ms,
            })
        }
    }
}

/// Load the workshop scene — DEPRECATED, use compose_scene instead.
#[tauri::command]
pub async fn start_scene(
    _app: tauri::AppHandle,
    _state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<SceneInfo, String> {
    Err("Classic scene mode has been removed. Use the scene wizard to compose a scene.".to_string())
}

/// Return the genre catalog from the scene composer.
#[tauri::command]
pub async fn load_catalog(
    composer: State<'_, Arc<SceneComposer>>,
) -> Result<serde_json::Value, String> {
    let genres = composer.genres();
    serde_json::to_value(&genres).map_err(|e| format!("Serialize error: {e}"))
}

/// Return filtered profiles, archetypes, dynamics, and names for a genre.
#[tauri::command]
pub async fn get_genre_options(
    genre_id: String,
    selected_archetypes: Vec<String>,
    composer: State<'_, Arc<SceneComposer>>,
) -> Result<serde_json::Value, String> {
    let profiles = composer.profiles_for_genre(&genre_id);
    let archetypes = composer.archetypes_for_genre(&genre_id);
    let dynamics = composer.dynamics_for_genre(&genre_id, &selected_archetypes);
    let names = composer.names_for_genre(&genre_id);
    serde_json::to_value(serde_json::json!({
        "profiles": profiles,
        "archetypes": archetypes,
        "dynamics": dynamics,
        "names": names,
    }))
    .map_err(|e| format!("Serialize error: {e}"))
}

/// Compose a scene from user selections, persist as a session, and render the opening.
#[tauri::command]
pub async fn compose_scene(
    selections: SceneSelections,
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
    composer: State<'_, Arc<SceneComposer>>,
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<SceneInfo, String> {
    let composed: ComposedScene = composer.compose(&selections)?;

    // Goal intersection
    let mut composed_goals = composer.intersect_goals(&selections, &composed);

    // Likeness pass (populate fragments)
    use storyteller_engine::scene_composer::likeness::{
        populate_character_goal_fragments, populate_scene_goal_fragments, LikenessContext,
    };
    {
        let likeness_ctx = LikenessContext {
            genre_id: &selections.genre_id,
            profile_id: &selections.profile_id,
            archetype_ids: selections
                .cast
                .iter()
                .map(|c| c.archetype_id.as_str())
                .collect(),
            dynamic_ids: selections
                .dynamics
                .iter()
                .map(|d| d.dynamic_id.as_str())
                .collect(),
        };
        let mut rng = rand::rng();
        populate_scene_goal_fragments(
            &mut composed_goals.scene_goals,
            composer.goal_defs(),
            &likeness_ctx,
            &mut rng,
        );
        for goals in composed_goals.character_goals.values_mut() {
            populate_character_goal_fragments(goals, composer.goal_defs(), &likeness_ctx, &mut rng);
        }
    }

    // Persist the session before rendering (crash-safe: data is on disk first).
    let session_id =
        session_store.create_session(&selections, &composed.scene, &composed.characters)?;

    // Persist goals
    let goals_json = serde_json::to_value(&composed_goals).map_err(|e| e.to_string())?;
    session_store
        .save_goals(&session_id, &goals_json)
        .map_err(|e| e.to_string())?;

    setup_and_render_opening(
        &app,
        composed.scene,
        composed.characters,
        &state,
        Some(session_id),
        Some(&session_store),
        composed_goals,
    )
    .await
}

/// List all persisted sessions.
#[tauri::command]
pub async fn list_sessions(
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<Vec<SessionSummary>, String> {
    session_store.list_sessions()
}

/// Load a persisted session and return its turn history for chat hydration.
///
/// If the session has saved turns (turns.jsonl), reconstructs engine state from
/// them without re-rendering the opening. Falls back to a fresh opening render
/// for legacy sessions without turn data.
#[tauri::command]
pub async fn resume_session(
    session_id: String,
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<ResumeResult, String> {
    let (_selections, scene, characters) = session_store.load_session(&session_id)?;

    // Try to load saved turns
    let turns = session_store.load_turns(&session_id)?;

    // Load goals from session (if available)
    let loaded_goals = session_store
        .load_goals(&session_id)
        .map_err(|e| e.to_string())?;
    let composed_goals: ComposedGoals = loaded_goals
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    // Load generated intentions from session (if available)
    let loaded_intentions = session_store
        .load_intentions(&session_id)
        .map_err(|e| e.to_string())?;
    if let Some(ref intentions_json) = loaded_intentions {
        tracing::info!("Loaded persisted intentions for session {session_id}");
        // Emit GoalsGenerated debug event so the inspector shows goal data on resume
        use storyteller_engine::inference::intention_generation::GeneratedIntentions;
        if let Ok(intentions) =
            serde_json::from_value::<GeneratedIntentions>(intentions_json.clone())
        {
            emit_debug(
                &app,
                DebugEvent::GoalsGenerated {
                    turn: 0,
                    scene_goals: composed_goals
                        .scene_goals
                        .iter()
                        .map(|g| format!("{} ({})", g.goal_id, g.category))
                        .collect(),
                    character_goals: composed_goals
                        .character_goals
                        .iter()
                        .flat_map(|(eid, goals)| {
                            let name = characters
                                .iter()
                                .find(|c| c.entity_id == *eid)
                                .map(|c| c.name.as_str())
                                .unwrap_or("unknown");
                            goals.iter().map(move |g| {
                                format!(
                                    "{} → {} ({}, {:?})",
                                    name, g.goal_id, g.category, g.visibility
                                )
                            })
                        })
                        .collect(),
                    scene_direction: Some(format!(
                        "{} {}",
                        intentions.scene_intention.dramatic_tension,
                        intentions.scene_intention.trajectory,
                    )),
                    character_drives: intentions
                        .character_intentions
                        .iter()
                        .map(|d| {
                            format!(
                                "{}: {} [constraint: {}] [stance: {}]",
                                d.character, d.objective, d.constraint, d.behavioral_stance
                            )
                        })
                        .collect(),
                    player_context: None,
                    timing_ms: 0,
                },
            );
        }
    }

    if turns.is_empty() {
        // Legacy session or no turns.jsonl — fall back to fresh opening render
        let scene_info = setup_and_render_opening(
            &app,
            scene,
            characters,
            &state,
            Some(session_id),
            Some(&session_store),
            composed_goals,
        )
        .await?;

        return Ok(ResumeResult {
            scene_info,
            turns: vec![],
        });
    }

    // --- Build SceneInfo from scene data + turn 0's narrator output ---
    let opening_prose = turns
        .first()
        .map(|t| t.narrator_output.clone())
        .unwrap_or_default();

    let scene_info = SceneInfo {
        title: scene.title.clone(),
        setting_description: scene.setting.description.clone(),
        cast: scene.cast.iter().map(|c| c.name.clone()).collect(),
        opening_prose,
    };

    // --- Convert to TurnSummary for frontend ---
    let turn_summaries: Vec<TurnSummary> = turns
        .iter()
        .map(|t| TurnSummary {
            turn: t.turn,
            player_input: t.player_input.clone(),
            narrator_output: t.narrator_output.clone(),
        })
        .collect();

    // --- Initialize engine state (same as setup_and_render_opening, minus rendering) ---
    let entity_ids: Vec<_> = characters.iter().map(|c| c.entity_id).collect();

    // Create Ollama LLM provider
    let config = ExternalServerConfig {
        base_url: "http://127.0.0.1:11434".to_string(),
        model: "qwen2.5:14b".to_string(),
        ..Default::default()
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(ExternalServerProvider::new(config));

    // Reconstruct journal from turn narrator outputs
    let mut journal = SceneJournal::new(SceneId::new(), 1200);
    let noop = NoopObserver;
    for turn_record in &turns {
        add_turn(
            &mut journal,
            turn_record.turn,
            &turn_record.narrator_output,
            entity_ids.clone(),
            vec![],
            &noop,
        );
    }

    // Load ML predictor (optional)
    let predictor = resolve_model_path().and_then(|path| {
        CharacterPredictor::load(&path)
            .inspect(|_| tracing::info!("ML model loaded: {}", path.display()))
            .inspect_err(|e| tracing::warn!("ML model failed to load: {e}"))
            .ok()
    });

    // Create structured LLM provider for event decomposition
    let structured_llm: Option<
        Arc<dyn storyteller_core::traits::structured_llm::StructuredLlmProvider>,
    > = {
        let config = StructuredLlmConfig::default();
        let provider = OllamaStructuredProvider::new(config);
        tracing::info!("Structured LLM provider created (qwen2.5:3b-instruct)");
        Some(Arc::new(provider))
    };

    // Intent synthesis LLM
    let intent_llm: Option<Arc<dyn LlmProvider>> = {
        let ollama_url =
            std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("STORYTELLER_INTENT_MODEL")
            .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string());
        let config = ExternalServerConfig {
            base_url: ollama_url,
            model: model.clone(),
            timeout: std::time::Duration::from_secs(60),
        };
        tracing::info!("Intent synthesis LLM provider created ({model})");
        Some(Arc::new(ExternalServerProvider::new(config)))
    };

    let grammar = PlutchikWestern::new();

    // Create session log
    let sessions_dir = PathBuf::from("sessions");
    let session_log = SessionLog::new(&sessions_dir, &scene.title)?;

    // Set turn count to last turn + 1
    let last_turn = turns.last().map(|t| t.turn).unwrap_or(0);

    // Find player character: first cast member whose role contains "protagonist"
    let player_entity_id = scene
        .cast
        .iter()
        .find(|c| c.role.to_lowercase().contains("protagonist"))
        .map(|c| c.entity_id);

    // Hydrate generated intentions from persisted file
    let resume_generated_intentions = session_store
        .load_intentions(&session_id)
        .ok()
        .flatten()
        .and_then(|v| {
            use storyteller_engine::inference::intention_generation::GeneratedIntentions;
            serde_json::from_value::<GeneratedIntentions>(v).ok()
        });

    let engine_state = EngineState {
        scene,
        characters,
        journal,
        llm,
        predictor,
        structured_llm,
        intent_llm,
        grammar,
        session_log,
        turn_count: last_turn,
        session_id: Some(session_id),
        player_entity_id,
        prediction_history: PredictionHistory::default(),
        generated_intentions: resume_generated_intentions,
        composed_goals: Some(composed_goals),
    };

    let mut guard = state.lock().await;
    *guard = Some(engine_state);

    Ok(ResumeResult {
        scene_info,
        turns: turn_summaries,
    })
}

/// Process player input through the engine pipeline and return the narrator's response.
#[tauri::command]
pub async fn submit_input(
    input: String,
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<TurnResult, String> {
    let mut guard = state.lock().await;
    let engine = guard
        .as_mut()
        .ok_or_else(|| "No scene loaded. Call start_scene first.".to_string())?;

    engine.turn_count += 1;
    let turn = engine.turn_count;

    let characters_refs: Vec<&_> = engine.characters.iter().collect();
    let entity_ids: Vec<_> = engine.characters.iter().map(|c| c.entity_id).collect();

    // --- Phase: Event Decomposition (LLM) ---
    // Runs first so decomposition-derived features can inform ML prediction.
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "decomposition".to_string(),
        },
    );

    let decomp_start = Instant::now();
    let mut persisted_decomposition: Option<serde_json::Value> = None;
    let mut event_decomposition: Option<EventDecomposition> = None;

    if let Some(ref structured_llm) = engine.structured_llm {
        // Include the last narrator prose so the 3b model can resolve pronouns
        // and ground the player's actions against established scene context.
        let decomp_input = if let Some(last_entry) = engine.journal.entries.last() {
            format!("[Narrator]\n{}\n\n[Player]\n{}", last_entry.content, input)
        } else {
            input.clone()
        };

        // Call provider directly so we capture raw JSON for debugging
        let request = storyteller_core::traits::structured_llm::StructuredRequest {
            system: event_decomposition_system_prompt(),
            input: decomp_input,
            output_schema: event_decomposition_schema(),
            temperature: 0.1,
        };
        match structured_llm.extract(request).await {
            Ok(raw_json) => {
                let decomp_ms = decomp_start.elapsed().as_millis() as u64;
                tracing::info!(
                    raw = %serde_json::to_string(&raw_json).unwrap_or_default(),
                    "Event decomposition raw LLM response"
                );
                persisted_decomposition = Some(raw_json.clone());
                // Try to parse the raw JSON into our typed decomposition
                let (decomposition, error) = match EventDecomposition::from_json(&raw_json) {
                    Ok(d) => (Some(d), None),
                    Err(e) => {
                        tracing::warn!("Event decomposition parse failed: {e}");
                        (None, Some(format!("{e}")))
                    }
                };
                event_decomposition = decomposition.clone();
                emit_debug(
                    &app,
                    DebugEvent::EventDecomposed {
                        turn,
                        decomposition,
                        raw_llm_json: Some(raw_json),
                        timing_ms: decomp_ms,
                        model: "qwen2.5:3b-instruct".to_string(),
                        error,
                    },
                );
            }
            Err(e) => {
                let decomp_ms = decomp_start.elapsed().as_millis() as u64;
                tracing::warn!("Event decomposition LLM call failed: {e}");
                emit_debug(
                    &app,
                    DebugEvent::EventDecomposed {
                        turn,
                        decomposition: None,
                        raw_llm_json: None,
                        timing_ms: decomp_ms,
                        model: "qwen2.5:3b-instruct".to_string(),
                        error: Some(format!("{e}")),
                    },
                );
            }
        }
    } else {
        emit_debug(
            &app,
            DebugEvent::EventDecomposed {
                turn,
                decomposition: None,
                raw_llm_json: None,
                timing_ms: 0,
                model: "none".to_string(),
                error: Some("No structured LLM provider configured".to_string()),
            },
        );
    }
    let decomposition_ms = decomp_start.elapsed().as_millis() as u64;

    // Derive event features from decomposition for ML prediction input.
    let event_features = if let Some(ref decomposition) = event_decomposition {
        decomposition_to_event_features(decomposition)
    } else {
        // Fallback when no structured LLM available — safe defaults
        storyteller_ml::feature_schema::EventFeatureInput {
            event_type: storyteller_core::types::prediction::EventType::Interaction,
            emotional_register: storyteller_core::types::prediction::EmotionalRegister::Neutral,
            confidence: 0.5,
            target_count: (characters_refs.len().saturating_sub(1)) as u8,
        }
    };

    // --- Phase: ML Predictions ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "prediction".to_string(),
        },
    );

    let predict_start = Instant::now();
    let mut resolver_output = if let Some(ref predictor) = engine.predictor {
        let predictions = predict_character_behaviors(
            predictor,
            &characters_refs,
            &engine.scene,
            &input,
            &engine.grammar,
            event_features,
            engine.prediction_history.as_map(),
        );
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: predictions,
            scene_dynamics: "ML-predicted character behavior".to_string(),
            conflicts: vec![],
            intent_statements: None,
        }
    } else {
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: vec![],
            scene_dynamics: "A quiet arrival — the distance between them is physical and temporal"
                .to_string(),
            conflicts: vec![],
            intent_statements: None,
        }
    };
    let prediction_ms = predict_start.elapsed().as_millis() as u64;

    // Accumulate prediction history for next turn's Region 7 features.
    for pred in &resolver_output.original_predictions {
        engine.prediction_history.push_from_prediction(pred);
    }

    emit_debug(
        &app,
        DebugEvent::PredictionComplete {
            turn,
            resolver_output: resolver_output.clone(),
            timing_ms: prediction_ms,
            model_loaded: engine.predictor.is_some(),
        },
    );

    // --- Phase: Characters ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "characters".to_string(),
        },
    );

    let emotional_markers = extract_emotional_markers(&input);
    emit_debug(
        &app,
        DebugEvent::CharactersUpdated {
            turn,
            characters: engine.characters.clone(),
            emotional_markers: emotional_markers.clone(),
        },
    );

    // --- Phase: Action Arbitration ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "arbitration".to_string(),
        },
    );

    let arb_start = Instant::now();
    let arbitration_result = check_action_possibility(
        &input,
        &[],                       // No genre constraints for workshop scene yet
        &CapabilityLexicon::new(), // Empty lexicon for now
        None,                      // No spatial zone tracking yet
    );
    let arb_ms = arb_start.elapsed().as_millis() as u64;

    let arbitration_json =
        serde_json::to_value(&arbitration_result).unwrap_or(serde_json::Value::Null);

    emit_debug(
        &app,
        DebugEvent::ActionArbitrated {
            turn,
            result: arbitration_result,
            player_input: input.clone(),
            timing_ms: arb_ms,
        },
    );

    // --- Phase: Intent Synthesis ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "intent_synthesis".to_string(),
        },
    );

    let intent_start = Instant::now();
    let journal_tail = engine
        .journal
        .entries
        .iter()
        .rev()
        .take(2)
        .rev()
        .map(|e| e.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let intent_statements = if let Some(ref intent_llm) = engine.intent_llm {
        storyteller_engine::inference::intent_synthesis::synthesize_intents(
            intent_llm.as_ref(),
            &characters_refs,
            &resolver_output.original_predictions,
            &journal_tail,
            &input,
            &engine.scene,
            engine.player_entity_id,
        )
        .await
    } else {
        None
    };
    let intent_ms = intent_start.elapsed().as_millis() as u64;

    resolver_output.intent_statements = intent_statements;

    if let Some(ref intents) = resolver_output.intent_statements {
        emit_debug(
            &app,
            DebugEvent::IntentSynthesized {
                turn,
                intent_statements: intents.clone(),
                timing_ms: intent_ms,
            },
        );
    }

    // --- Phase: Context Assembly ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "context".to_string(),
        },
    );

    let observer = CollectingObserver::new();
    let assembly_start = Instant::now();
    let context = assemble_narrator_context(
        &engine.scene,
        &characters_refs,
        &engine.journal,
        &resolver_output,
        &input,
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
        engine.player_entity_id,
    );
    let assembly_ms = assembly_start.elapsed().as_millis() as u64;

    let token_counts = extract_token_counts(&observer);

    // Render context tiers as text for the debug inspector
    let preamble_text = format!(
        "Narrator: {}\nSetting: {}\nCast: {}\nBoundaries: {}",
        context.preamble.narrator_identity,
        context.preamble.setting_description,
        context
            .preamble
            .cast_descriptions
            .iter()
            .map(|c| format!("{} ({})", c.name, c.role))
            .collect::<Vec<_>>()
            .join(", "),
        context.preamble.boundaries.join("; "),
    );
    let journal_text = context
        .journal
        .entries
        .iter()
        .map(|e| format!("[Turn {}] {}", e.turn_number, e.content))
        .collect::<Vec<_>>()
        .join("\n");
    let retrieved_text = context
        .retrieved
        .iter()
        .map(|r| {
            let mut s = format!("{}: {}", r.subject, r.content);
            if let Some(ref emo) = r.emotional_context {
                s.push_str(&format!(" ({})", emo));
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n");

    emit_debug(
        &app,
        DebugEvent::ContextAssembled {
            turn,
            preamble_text,
            journal_text,
            retrieved_text,
            token_counts: TokenCounts {
                preamble: token_counts.0,
                journal: token_counts.1,
                retrieved: token_counts.2,
                total: token_counts.3,
            },
            timing_ms: assembly_ms,
        },
    );

    // --- Phase: Narrator Rendering ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "narrator".to_string(),
        },
    );

    let narrator = NarratorAgent::new(&context, Arc::clone(&engine.llm)).with_temperature(0.8);
    let llm_start = Instant::now();
    let rendering = match narrator.render(&context, &observer).await {
        Ok(r) => r,
        Err(e) => {
            emit_debug(
                &app,
                DebugEvent::Error {
                    turn,
                    phase: "narrator".to_string(),
                    message: format!("{e}"),
                },
            );
            return Err(format!("Narrator render failed: {e}"));
        }
    };
    let narrator_ms = llm_start.elapsed().as_millis() as u64;

    emit_debug(
        &app,
        DebugEvent::NarratorComplete {
            turn,
            system_prompt: narrator.system_prompt().to_string(),
            user_message: "See context assembly".to_string(),
            raw_response: rendering.text.clone(),
            model: "qwen2.5:14b".to_string(),
            temperature: 0.8,
            max_tokens: 400,
            tokens_used: 0,
            timing_ms: narrator_ms,
        },
    );

    // Update journal
    let noop = NoopObserver;
    add_turn(
        &mut engine.journal,
        turn,
        &rendering.text,
        entity_ids,
        emotional_markers,
        &noop,
    );

    // Append to session log
    let log_entry = LogEntry {
        turn,
        timestamp: chrono::Utc::now(),
        player_input: input,
        narrator_output: rendering.text.clone(),
        context_assembly: ContextAssemblyLog {
            preamble_tokens: token_counts.0,
            journal_tokens: token_counts.1,
            retrieved_tokens: token_counts.2,
            total_tokens: token_counts.3,
        },
        timing: TimingLog {
            prediction_ms,
            assembly_ms,
            narrator_ms,
        },
    };
    engine.session_log.append(&log_entry)?;

    // Persist turn to turns.jsonl (replaces events.jsonl).
    if let Some(ref sid) = engine.session_id {
        let turn_record = TurnRecord {
            turn,
            player_input: Some(log_entry.player_input.clone()),
            narrator_output: rendering.text.clone(),
            predictions: serde_json::to_value(&resolver_output.original_predictions).ok(),
            intent_statements: resolver_output.intent_statements.clone(),
            decomposition: persisted_decomposition,
            arbitration: Some(arbitration_json),
            context_assembly: Some(serde_json::json!({
                "preamble_tokens": token_counts.0,
                "journal_tokens": token_counts.1,
                "retrieved_tokens": token_counts.2,
                "total_tokens": token_counts.3,
            })),
            timing: Some(serde_json::json!({
                "decomposition_ms": decomposition_ms,
                "prediction_ms": prediction_ms,
                "intent_ms": intent_ms,
                "assembly_ms": assembly_ms,
                "narrator_ms": narrator_ms,
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = session_store.append_turn(sid, &turn_record) {
            tracing::warn!(error = %e, "Failed to persist turn {turn}");
        }
    }

    Ok(TurnResult {
        turn,
        narrator_prose: rendering.text,
        timing: TurnTiming {
            prediction_ms,
            assembly_ms,
            narrator_ms,
        },
        context_tokens: ContextTokens {
            preamble: token_counts.0,
            journal: token_counts.1,
            retrieved: token_counts.2,
            total: token_counts.3,
        },
    })
}

/// Return all session log entries for the current session.
#[tauri::command]
pub async fn get_session_log(
    state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<Vec<LogEntry>, String> {
    let guard = state.lock().await;
    let engine = guard
        .as_ref()
        .ok_or_else(|| "No scene loaded. Call start_scene first.".to_string())?;
    engine.session_log.read_all()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Shared setup logic for `start_scene`, `compose_scene`, and `resume_session`.
///
/// Creates LLM providers, loads ML models, emits opening debug events,
/// assembles narrator context, renders the opening, and stores engine state.
async fn setup_and_render_opening(
    app: &tauri::AppHandle,
    scene: SceneData,
    characters: Vec<CharacterSheet>,
    state: &State<'_, Mutex<Option<EngineState>>>,
    session_id: Option<String>,
    session_store: Option<&State<'_, Arc<SessionStore>>>,
    composed_goals: ComposedGoals,
) -> Result<SceneInfo, String> {
    let characters_refs: Vec<&_> = characters.iter().collect();
    let entity_ids: Vec<_> = characters.iter().map(|c| c.entity_id).collect();

    // Create Ollama LLM provider
    let config = ExternalServerConfig {
        base_url: "http://127.0.0.1:11434".to_string(),
        model: "qwen2.5:14b".to_string(),
        ..Default::default()
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(ExternalServerProvider::new(config));

    // Create scene journal
    let journal = SceneJournal::new(SceneId::new(), 1200);

    // Load ML predictor (optional)
    let predictor = resolve_model_path().and_then(|path| {
        CharacterPredictor::load(&path)
            .inspect(|_| tracing::info!("ML model loaded: {}", path.display()))
            .inspect_err(|e| tracing::warn!("ML model failed to load: {e}"))
            .ok()
    });

    // Create structured LLM provider for event decomposition (qwen2.5:3b-instruct)
    let structured_llm: Option<
        Arc<dyn storyteller_core::traits::structured_llm::StructuredLlmProvider>,
    > = {
        let config = StructuredLlmConfig::default();
        let provider = OllamaStructuredProvider::new(config);
        tracing::info!("Structured LLM provider created (qwen2.5:3b-instruct)");
        Some(Arc::new(provider))
    };

    // Intent synthesis LLM — same 3b model, plain completion (not structured)
    let intent_llm: Option<Arc<dyn LlmProvider>> = {
        let ollama_url =
            std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("STORYTELLER_INTENT_MODEL")
            .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string());
        let config = ExternalServerConfig {
            base_url: ollama_url,
            model: model.clone(),
            timeout: std::time::Duration::from_secs(60),
        };
        tracing::info!("Intent synthesis LLM provider created ({model})");
        Some(Arc::new(ExternalServerProvider::new(config)))
    };

    // Intention generation (composition-time LLM call)
    let intention_start = Instant::now();
    let generated_intentions = if !composed_goals.scene_goals.is_empty() {
        use storyteller_engine::inference::intention_generation::generate_intentions;
        let characters_refs_for_ig: Vec<&CharacterSheet> = characters.iter().collect();
        tracing::info!(
            "Generating intentions for {} scene goals, {} characters",
            composed_goals.scene_goals.len(),
            characters_refs_for_ig.len()
        );
        let result = generate_intentions(
            llm.as_ref(),
            &scene,
            &characters_refs_for_ig,
            &composed_goals,
        )
        .await;
        match &result {
            Some(intentions) => tracing::info!(
                "Intention generation succeeded: {} character intentions",
                intentions.character_intentions.len()
            ),
            None => tracing::warn!(
                "Intention generation returned None — LLM call or JSON parsing failed"
            ),
        }
        result
    } else {
        tracing::info!("No scene goals — skipping intention generation");
        None
    };
    let intention_ms = intention_start.elapsed().as_millis() as u64;

    let grammar = PlutchikWestern::new();

    // Create session log
    let sessions_dir = PathBuf::from("sessions");
    let session_log = SessionLog::new(&sessions_dir, &scene.title)?;

    let turn: u32 = 0;

    // --- Phase: ML Predictions (opening — no input to predict on) ---
    emit_debug(
        app,
        DebugEvent::PredictionComplete {
            turn,
            resolver_output: ResolverOutput {
                sequenced_actions: vec![],
                original_predictions: vec![],
                scene_dynamics: "Opening turn — no player input yet".to_string(),
                conflicts: vec![],
                intent_statements: None,
            },
            timing_ms: 0,
            model_loaded: predictor.is_some(),
        },
    );

    // --- Phase: Characters ---
    emit_debug(
        app,
        DebugEvent::CharactersUpdated {
            turn,
            characters: characters.clone(),
            emotional_markers: vec![],
        },
    );

    // --- Phase: Context Assembly ---
    emit_debug(
        app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "context".to_string(),
        },
    );

    // Assemble opening context
    let opening_resolver = ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: vec![],
        scene_dynamics: "A quiet arrival — the distance between them is physical and temporal"
            .to_string(),
        conflicts: vec![],
        intent_statements: None,
    };

    // Find player character: first cast member whose role contains "protagonist"
    let player_entity_id = scene
        .cast
        .iter()
        .find(|c| c.role.to_lowercase().contains("protagonist"))
        .map(|c| c.entity_id);

    let observer = CollectingObserver::new();
    let assembly_start = Instant::now();
    let mut context = assemble_narrator_context(
        &scene,
        &characters_refs,
        &journal,
        &opening_resolver,
        "",
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
        player_entity_id,
    );
    let assembly_ms = assembly_start.elapsed().as_millis() as u64;

    // Inject goal intentions into preamble
    use storyteller_engine::inference::intention_generation::intentions_to_preamble;
    use storyteller_engine::scene_composer::GoalVisibility;

    if let Some(ref intentions) = generated_intentions {
        let (scene_direction, character_drives) = intentions_to_preamble(intentions);
        context.preamble.scene_direction = Some(scene_direction);
        context.preamble.character_drives = character_drives;
    }

    // Player context based on player's character goals
    context.preamble.player_context = player_entity_id
        .and_then(|pid| composed_goals.character_goals.get(&pid))
        .map(|goals| {
            goals
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

    // Emit goals debug event
    emit_debug(
        app,
        DebugEvent::GoalsGenerated {
            turn,
            scene_goals: composed_goals
                .scene_goals
                .iter()
                .map(|g| format!("{} ({})", g.goal_id, g.category))
                .collect(),
            character_goals: composed_goals
                .character_goals
                .iter()
                .flat_map(|(eid, goals)| {
                    let name = characters
                        .iter()
                        .find(|c| c.entity_id == *eid)
                        .map(|c| c.name.as_str())
                        .unwrap_or("unknown");
                    goals.iter().map(move |g| {
                        format!(
                            "{} → {} ({}, {:?})",
                            name, g.goal_id, g.category, g.visibility
                        )
                    })
                })
                .collect(),
            scene_direction: context
                .preamble
                .scene_direction
                .as_ref()
                .map(|d| format!("{} {}", d.dramatic_tension, d.trajectory)),
            character_drives: context
                .preamble
                .character_drives
                .iter()
                .map(|d| {
                    format!(
                        "{}: {} [constraint: {}] [stance: {}]",
                        d.name, d.objective, d.constraint, d.behavioral_stance
                    )
                })
                .collect(),
            player_context: context.preamble.player_context.clone(),
            timing_ms: intention_ms,
        },
    );

    // Re-estimate token counts after goal injection
    let preamble_tokens =
        storyteller_engine::context::preamble::estimate_preamble_tokens(&context.preamble);
    let base_token_counts = extract_token_counts(&observer);
    let token_counts = (
        preamble_tokens,
        base_token_counts.1,
        base_token_counts.2,
        preamble_tokens + base_token_counts.1 + base_token_counts.2,
    );

    // Render context tiers as text for the debug inspector
    // Use the full render_preamble so the inspector shows exactly what the narrator sees
    let opening_preamble_text =
        storyteller_engine::context::preamble::render_preamble(&context.preamble);

    emit_debug(
        app,
        DebugEvent::ContextAssembled {
            turn,
            preamble_text: opening_preamble_text,
            journal_text: String::new(),
            retrieved_text: String::new(),
            token_counts: TokenCounts {
                preamble: token_counts.0,
                journal: token_counts.1,
                retrieved: token_counts.2,
                total: token_counts.3,
            },
            timing_ms: assembly_ms,
        },
    );

    // --- Phase: Narrator Rendering ---
    emit_debug(
        app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "narrator".to_string(),
        },
    );

    // Create narrator and generate opening
    let narrator = NarratorAgent::new(&context, Arc::clone(&llm)).with_temperature(0.8);
    let llm_start = Instant::now();
    let opening = match narrator.render_opening(&observer).await {
        Ok(o) => o,
        Err(e) => {
            emit_debug(
                app,
                DebugEvent::Error {
                    turn,
                    phase: "narrator".to_string(),
                    message: format!("{e}"),
                },
            );
            return Err(format!("Failed to render opening: {e}"));
        }
    };
    let narrator_ms = llm_start.elapsed().as_millis() as u64;

    emit_debug(
        app,
        DebugEvent::NarratorComplete {
            turn,
            system_prompt: narrator.system_prompt().to_string(),
            user_message: "(opening — no player input)".to_string(),
            raw_response: opening.text.clone(),
            model: "qwen2.5:14b".to_string(),
            temperature: 0.8,
            max_tokens: 400,
            tokens_used: 0,
            timing_ms: narrator_ms,
        },
    );

    // Record opening in journal
    let mut journal = journal;
    let noop = NoopObserver;
    add_turn(
        &mut journal,
        0,
        &opening.text,
        entity_ids.to_vec(),
        vec![],
        &noop,
    );

    // Log the opening turn
    let log_entry = LogEntry {
        turn: 0,
        timestamp: chrono::Utc::now(),
        player_input: String::new(),
        narrator_output: opening.text.clone(),
        context_assembly: ContextAssemblyLog {
            preamble_tokens: token_counts.0,
            journal_tokens: token_counts.1,
            retrieved_tokens: token_counts.2,
            total_tokens: token_counts.3,
        },
        timing: TimingLog {
            prediction_ms: 0,
            assembly_ms,
            narrator_ms,
        },
    };
    session_log.append(&log_entry)?;

    // Persist turn 0 (opening narration) to turns.jsonl
    if let (Some(sid), Some(store)) = (&session_id, &session_store) {
        let turn0 = TurnRecord {
            turn: 0,
            player_input: None,
            narrator_output: opening.text.clone(),
            predictions: None,
            intent_statements: None,
            decomposition: None,
            arbitration: None,
            context_assembly: Some(serde_json::json!({
                "preamble_tokens": token_counts.0,
                "journal_tokens": token_counts.1,
                "retrieved_tokens": token_counts.2,
                "total_tokens": token_counts.3,
            })),
            timing: Some(serde_json::json!({
                "assembly_ms": assembly_ms,
                "narrator_ms": narrator_ms,
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = store.append_turn(sid, &turn0) {
            tracing::warn!(error = %e, "Failed to persist turn 0");
        }

        // Persist generated intentions (scene direction + character drives)
        if let Some(ref intentions) = generated_intentions {
            match serde_json::to_value(intentions) {
                Ok(intentions_json) => {
                    if let Err(e) = store.save_intentions(sid, &intentions_json) {
                        tracing::warn!(error = %e, "Failed to persist intentions.json");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to serialize intentions");
                }
            }
        }
    }

    let scene_info = SceneInfo {
        title: scene.title.clone(),
        setting_description: scene.setting.description.clone(),
        cast: scene.cast.iter().map(|c| c.name.clone()).collect(),
        opening_prose: opening.text,
    };

    // Store engine state
    let engine_state = EngineState {
        scene,
        characters,
        journal,
        llm,
        predictor,
        structured_llm,
        intent_llm,
        grammar,
        session_log,
        turn_count: 0,
        session_id,
        player_entity_id,
        prediction_history: PredictionHistory::default(),
        generated_intentions,
        composed_goals: Some(composed_goals),
    };

    let mut guard = state.lock().await;
    *guard = Some(engine_state);

    Ok(scene_info)
}

/// Emit a debug event to the inspector panel. Failures are silently ignored —
/// debug events are best-effort observability, never blocking.
fn emit_debug(app: &tauri::AppHandle, event: DebugEvent) {
    let _ = app.emit(DEBUG_EVENT_CHANNEL, &event);
}

/// Extract rough emotional markers from player input.
///
/// Very naive for the prototype — looks for emotionally charged words.
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

/// Extract token counts from observer events.
///
/// Returns (preamble, journal, retrieved, total).
fn extract_token_counts(observer: &CollectingObserver) -> (u32, u32, u32, u32) {
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

/// Resolve the path to the character predictor ONNX model file.
fn resolve_model_path() -> Option<PathBuf> {
    if let Ok(model_dir) = std::env::var("STORYTELLER_MODEL_PATH") {
        let path = PathBuf::from(model_dir).join("character_predictor.onnx");
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(data_path) = std::env::var("STORYTELLER_DATA_PATH") {
        let path = PathBuf::from(data_path)
            .join("models")
            .join("character_predictor.onnx");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_emotional_markers_finds_sadness() {
        let markers = extract_emotional_markers("I begin to cry softly");
        assert!(markers.contains(&"sadness".to_string()));
    }

    #[test]
    fn extract_emotional_markers_finds_joy() {
        let markers = extract_emotional_markers("She smiled and laughed");
        assert!(markers.contains(&"joy".to_string()));
    }

    #[test]
    fn extract_emotional_markers_finds_recognition() {
        let markers = extract_emotional_markers("I play the flute and remember");
        assert!(markers.contains(&"recognition".to_string()));
        assert_eq!(markers.iter().filter(|m| *m == "recognition").count(), 2);
    }

    #[test]
    fn extract_emotional_markers_returns_empty_for_neutral_input() {
        let markers = extract_emotional_markers("I open the door and walk inside.");
        assert!(markers.is_empty());
    }

    #[test]
    fn extract_emotional_markers_is_case_insensitive() {
        let markers = extract_emotional_markers("I am AFRAID and ANGRY");
        assert!(markers.contains(&"fear".to_string()));
        assert!(markers.contains(&"anger".to_string()));
    }

    #[test]
    fn llm_status_serializes_to_expected_shape() {
        let status = LlmStatus {
            reachable: true,
            endpoint: "http://127.0.0.1:11434".to_string(),
            model: "qwen2.5:14b".to_string(),
            provider: "Ollama".to_string(),
            available_models: vec!["qwen2.5:14b".to_string(), "mistral".to_string()],
            error: None,
            latency_ms: 42,
        };
        let json = serde_json::to_value(&status).expect("serialize");
        assert_eq!(json["reachable"], true);
        assert_eq!(json["provider"], "Ollama");
        assert!(json["error"].is_null());
        assert_eq!(json["available_models"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn scene_info_serializes() {
        let info = SceneInfo {
            title: "The Fair and the Dead".to_string(),
            setting_description: "A twilight meadow".to_string(),
            cast: vec!["Bramblehoof".to_string(), "Pyotir".to_string()],
            opening_prose: "The meadow stretches before you.".to_string(),
        };
        let json = serde_json::to_value(&info).expect("serialize");
        assert_eq!(json["title"], "The Fair and the Dead");
        assert_eq!(json["cast"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn turn_result_serializes_nested_structs() {
        let result = TurnResult {
            turn: 3,
            narrator_prose: "The wind howls.".to_string(),
            timing: TurnTiming {
                prediction_ms: 50,
                assembly_ms: 12,
                narrator_ms: 2400,
            },
            context_tokens: ContextTokens {
                preamble: 600,
                journal: 800,
                retrieved: 400,
                total: 1800,
            },
        };
        let json = serde_json::to_value(&result).expect("serialize");
        assert_eq!(json["turn"], 3);
        assert_eq!(json["timing"]["narrator_ms"], 2400);
        assert_eq!(json["context_tokens"]["total"], 1800);
    }
}
