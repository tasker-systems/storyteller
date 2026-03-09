//! Tauri commands — the bridge between the frontend and the storyteller engine.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::phase_observer::{CollectingObserver, PhaseEventDetail};
use storyteller_core::traits::NoopObserver;
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::scene::SceneId;

use storyteller_engine::agents::narrator::NarratorAgent;
use storyteller_engine::context::journal::add_turn;
use storyteller_engine::context::prediction::predict_character_behaviors;
use storyteller_engine::context::{assemble_narrator_context, DEFAULT_TOTAL_TOKEN_BUDGET};
use storyteller_engine::inference::event_classifier::EventClassifier;
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};
use storyteller_engine::inference::frame::CharacterPredictor;
use storyteller_engine::workshop::the_flute_kept;

use crate::engine_state::EngineState;
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

/// Load the workshop scene, create the LLM provider, and generate the opening.
#[tauri::command]
pub async fn start_scene(
    state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<SceneInfo, String> {
    let scene = the_flute_kept::scene();
    let bramblehoof_sheet = the_flute_kept::bramblehoof();
    let pyotir_sheet = the_flute_kept::pyotir();
    let characters_refs: Vec<&_> = [&bramblehoof_sheet, &pyotir_sheet].to_vec();
    let entity_ids = [bramblehoof_sheet.entity_id, pyotir_sheet.entity_id];

    // Create Ollama LLM provider
    let config = ExternalServerConfig {
        base_url: "http://localhost:11434".to_string(),
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

    let grammar = PlutchikWestern::new();

    // Create session log
    let sessions_dir = PathBuf::from("sessions");
    let session_log = SessionLog::new(&sessions_dir, &scene.title)?;

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

    // Create narrator and generate opening
    let narrator = NarratorAgent::new(&context, Arc::clone(&llm)).with_temperature(0.8);
    let llm_start = Instant::now();
    let opening = narrator
        .render_opening(&observer)
        .await
        .map_err(|e| format!("Failed to render opening: {e}"))?;
    let narrator_ms = llm_start.elapsed().as_millis() as u64;

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
    let characters = vec![bramblehoof_sheet, pyotir_sheet];
    let engine_state = EngineState {
        scene,
        characters,
        journal,
        llm,
        predictor,
        event_classifier,
        grammar,
        session_log,
        turn_count: 0,
    };

    let mut guard = state.lock().await;
    *guard = Some(engine_state);

    Ok(scene_info)
}

/// Process player input through the engine pipeline and return the narrator's response.
#[tauri::command]
pub async fn submit_input(
    input: String,
    state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<TurnResult, String> {
    let mut guard = state.lock().await;
    let engine = guard
        .as_mut()
        .ok_or_else(|| "No scene loaded. Call start_scene first.".to_string())?;

    engine.turn_count += 1;
    let turn = engine.turn_count;

    let characters_refs: Vec<&_> = engine.characters.iter().collect();
    let entity_ids: Vec<_> = engine.characters.iter().map(|c| c.entity_id).collect();

    // ML predictions (if model available)
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

    // Context assembly
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

    // Narrator renders
    let narrator = NarratorAgent::new(&context, Arc::clone(&engine.llm)).with_temperature(0.8);
    let llm_start = Instant::now();
    let rendering = narrator
        .render(&context, &observer)
        .await
        .map_err(|e| format!("Narrator render failed: {e}"))?;
    let narrator_ms = llm_start.elapsed().as_millis() as u64;

    // Update journal
    let noop = NoopObserver;
    add_turn(
        &mut engine.journal,
        turn,
        &rendering.text,
        entity_ids,
        extract_emotional_markers(&input),
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
