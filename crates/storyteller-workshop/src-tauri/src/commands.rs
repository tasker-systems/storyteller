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
use storyteller_engine::context::prediction::predict_character_behaviors;
use storyteller_engine::context::{assemble_narrator_context, DEFAULT_TOTAL_TOKEN_BUDGET};
use storyteller_engine::inference::event_classifier::EventClassifier;
use storyteller_engine::inference::event_decomposition::{
    event_decomposition_schema, event_decomposition_system_prompt, EventDecomposition,
};
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};
use storyteller_engine::inference::frame::CharacterPredictor;
use storyteller_engine::inference::structured::OllamaStructuredProvider;
use storyteller_engine::scene_composer::{ComposedScene, SceneComposer, SceneSelections};
use storyteller_engine::systems::arbitration::check_action_possibility;
use storyteller_engine::workshop::the_flute_kept;

use crate::engine_state::EngineState;
use crate::events::{DebugEvent, TokenCounts, DEBUG_EVENT_CHANNEL};
use crate::session::{SessionStore, SessionSummary};
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

/// Load the workshop scene, create the LLM provider, and generate the opening.
#[tauri::command]
pub async fn start_scene(
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<SceneInfo, String> {
    let scene = the_flute_kept::scene();
    let bramblehoof = the_flute_kept::bramblehoof();
    let pyotir = the_flute_kept::pyotir();
    let characters = vec![bramblehoof, pyotir];

    setup_and_render_opening(&app, scene, characters, &state, None).await
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

    // Persist the session before rendering (crash-safe: data is on disk first).
    let session_id =
        session_store.create_session(&selections, &composed.scene, &composed.characters)?;

    setup_and_render_opening(
        &app,
        composed.scene,
        composed.characters,
        &state,
        Some(session_id),
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

/// Load a persisted session and render its opening.
#[tauri::command]
pub async fn resume_session(
    session_id: String,
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<SceneInfo, String> {
    let (_selections, scene, characters) = session_store.load_session(&session_id)?;
    setup_and_render_opening(&app, scene, characters, &state, Some(session_id)).await
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

    // --- Phase: ML Predictions ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "prediction".to_string(),
        },
    );

    let predict_start = Instant::now();
    let resolver_output = if let Some(ref predictor) = engine.predictor {
        let (predictions, _classification) = predict_character_behaviors(
            predictor,
            &characters_refs,
            &engine.scene,
            &input,
            &engine.grammar,
            engine.event_classifier.as_ref(),
        );
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: predictions,
            scene_dynamics: "ML-predicted character behavior".to_string(),
            conflicts: vec![],
        }
    } else {
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: vec![],
            scene_dynamics: "A quiet arrival — the distance between them is physical and temporal"
                .to_string(),
            conflicts: vec![],
        }
    };
    let prediction_ms = predict_start.elapsed().as_millis() as u64;

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

    // --- Phase: Event Classification ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "events".to_string(),
        },
    );

    let classifications: Vec<String> = if let Some(ref classifier) = engine.event_classifier {
        match classifier.classify_text(&input) {
            Ok(output) => output
                .event_kinds
                .iter()
                .map(|(label, score)| format!("{label}: {score:.2}"))
                .collect(),
            Err(e) => vec![format!("Classification error: {e}")],
        }
    } else {
        vec![]
    };

    emit_debug(
        &app,
        DebugEvent::EventsClassified {
            turn,
            classifications,
            classifier_loaded: engine.event_classifier.is_some(),
        },
    );

    // --- Phase: Event Decomposition (LLM) ---
    emit_debug(
        &app,
        DebugEvent::PhaseStarted {
            turn,
            phase: "decomposition".to_string(),
        },
    );

    let decomp_start = Instant::now();
    if let Some(ref structured_llm) = engine.structured_llm {
        // Call provider directly so we capture raw JSON for debugging
        let request = storyteller_core::traits::structured_llm::StructuredRequest {
            system: event_decomposition_system_prompt(),
            input: input.clone(),
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
                // Try to parse the raw JSON into our typed decomposition
                let (decomposition, error) = match EventDecomposition::from_json(&raw_json) {
                    Ok(d) => (Some(d), None),
                    Err(e) => {
                        tracing::warn!("Event decomposition parse failed: {e}");
                        (None, Some(format!("{e}")))
                    }
                };
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

    emit_debug(
        &app,
        DebugEvent::ActionArbitrated {
            turn,
            result: arbitration_result,
            player_input: input.clone(),
            timing_ms: arb_ms,
        },
    );

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

    // Append to persisted session events.jsonl if this is a persisted session.
    if let Some(ref sid) = engine.session_id {
        let events_path = session_store.events_path(sid);
        let event_line = serde_json::json!({
            "turn": turn,
            "timestamp": log_entry.timestamp.to_rfc3339(),
            "player_input": log_entry.player_input,
            "narrator_output": log_entry.narrator_output,
        });
        let mut line =
            serde_json::to_string(&event_line).map_err(|e| format!("serialize event: {e}"))?;
        line.push('\n');
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
            .map_err(|e| format!("append events.jsonl: {e}"))?;
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

    // Load event classifier (optional)
    let event_classifier = resolve_event_classifier_path().and_then(|path| {
        EventClassifier::load(&path)
            .inspect(|_| tracing::info!("Event classifier loaded: {}", path.display()))
            .inspect_err(|e| tracing::warn!("Event classifier failed to load: {e}"))
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

    // --- Phase: Events (opening — no input to classify) ---
    emit_debug(
        app,
        DebugEvent::EventsClassified {
            turn,
            classifications: vec![],
            classifier_loaded: event_classifier.is_some(),
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
    };

    let observer = CollectingObserver::new();
    let assembly_start = Instant::now();
    let context = assemble_narrator_context(
        &scene,
        &characters_refs,
        &journal,
        &opening_resolver,
        "",
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
    );
    let assembly_ms = assembly_start.elapsed().as_millis() as u64;

    // Extract token counts from observer
    let token_counts = extract_token_counts(&observer);

    // Render context tiers as text for the debug inspector
    let opening_preamble_text = format!(
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
        event_classifier,
        structured_llm,
        grammar,
        session_log,
        turn_count: 0,
        session_id,
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

/// Resolve the path to the event classifier model directory.
fn resolve_event_classifier_path() -> Option<PathBuf> {
    if let Ok(model_dir) = std::env::var("STORYTELLER_MODEL_PATH") {
        let path = PathBuf::from(model_dir).join("event_classifier");
        if path.join("event_classifier.onnx").exists() {
            return Some(path);
        }
    }
    if let Ok(data_path) = std::env::var("STORYTELLER_DATA_PATH") {
        let path = PathBuf::from(data_path).join("models/event_classifier");
        if path.join("event_classifier.onnx").exists() {
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
