//! Interactive scene player — narrator-centric architecture.
//!
//! Runs workshop scenes using the three-tier context assembly pipeline
//! with a single Narrator LLM call per turn.
//!
//! Usage:
//!   cargo run --bin play-scene -- --model qwen2.5:14b
//!   cargo run --bin play-scene -- --model mistral --temperature 0.7
//!   cargo run --bin play-scene -- --inputs test_inputs.txt --no-ml
//!
//! Requires a running Ollama instance at localhost:11434.

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::phase_observer::{CollectingObserver, PhaseEventDetail};
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

#[derive(Parser, Debug)]
#[command(
    name = "play-scene",
    about = "Interactive scene player — single Narrator LLM call per turn"
)]
struct Args {
    /// Ollama model name
    #[arg(long, default_value = "qwen2.5:14b")]
    model: String,

    /// LLM sampling temperature
    #[arg(long, default_value_t = 0.8)]
    temperature: f32,

    /// Ollama base URL
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Disable ML predictions (run without ONNX model)
    #[arg(long)]
    no_ml: bool,

    /// Show timing details for each phase
    #[arg(long, default_value_t = true)]
    timing: bool,

    /// Read player inputs from a file (one per line) instead of stdin.
    /// Useful for scripted evaluation sessions.
    #[arg(long)]
    inputs: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    eprintln!("--- The Flute Kept ---");
    eprintln!(
        "Model: {}  Temperature: {}  Ollama: {}",
        args.model, args.temperature, args.ollama_url
    );
    eprintln!("Type scene directions. /quit to exit.\n");

    // Load scene and character data
    let scene = the_flute_kept::scene();
    let bramblehoof_sheet = the_flute_kept::bramblehoof();
    let pyotir_sheet = the_flute_kept::pyotir();
    let characters = vec![&bramblehoof_sheet, &pyotir_sheet];

    // Load ML predictor (optional — graceful fallback)
    let grammar = PlutchikWestern::new();
    let predictor = if args.no_ml {
        eprintln!("[ML predictions disabled via --no-ml]");
        None
    } else {
        match resolve_model_path() {
            Some(path) => match CharacterPredictor::load(&path) {
                Ok(p) => {
                    eprintln!("[ML model loaded: {}]", path.display());
                    Some(p)
                }
                Err(e) => {
                    eprintln!(
                        "[Warning: ML model failed to load: {e}. Running without predictions.]"
                    );
                    None
                }
            },
            None => {
                eprintln!(
                    "[No ML model found. Set STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH. \
                     Running without predictions.]"
                );
                None
            }
        }
    };

    // Load event classifier (optional — graceful fallback)
    let event_classifier = if args.no_ml {
        None
    } else {
        match resolve_event_classifier_path() {
            Some(path) => match EventClassifier::load(&path) {
                Ok(c) => {
                    eprintln!("[Event classifier loaded: {}]", path.display());
                    Some(c)
                }
                Err(e) => {
                    eprintln!(
                        "[Warning: Event classifier failed to load: {e}. \
                         Using keyword fallback.]"
                    );
                    None
                }
            },
            None => {
                eprintln!(
                    "[No event classifier found. Using keyword fallback for event classification.]"
                );
                None
            }
        }
    };

    // Create LLM provider
    let config = ExternalServerConfig {
        base_url: args.ollama_url,
        model: args.model,
        ..Default::default()
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(ExternalServerProvider::new(config));

    // Scene journal — rolling compressed history
    let mut journal = SceneJournal::new(SceneId::new(), 1200);

    // Opening resolver output — no player input yet, so no ML predictions.
    let opening_resolver = ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: vec![],
        scene_dynamics: "A quiet arrival — the distance between them is physical and temporal"
            .to_string(),
        conflicts: vec![],
    };

    // Assemble initial context for the opening
    let observer = CollectingObserver::new();
    let total_start = Instant::now();

    let assembly_start = Instant::now();
    let context = assemble_narrator_context(
        &scene,
        &characters,
        &journal,
        &opening_resolver,
        "",
        &[bramblehoof_sheet.entity_id, pyotir_sheet.entity_id],
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
    );
    let assembly_elapsed = assembly_start.elapsed();

    if args.timing {
        eprintln!(
            "[Context assembly: {:.1}ms | ~{} tokens]",
            assembly_elapsed.as_secs_f64() * 1000.0,
            context.estimated_tokens
        );
        print_observer_summary(&observer);
    }

    // Create narrator from assembled context
    let narrator =
        NarratorAgent::new(&context, Arc::clone(&llm)).with_temperature(args.temperature);

    // Scene opening — single LLM call
    eprintln!("[Generating scene opening (1 LLM call)...]\n");
    let llm_start = Instant::now();
    let opening = narrator.render_opening(&observer).await?;
    let llm_elapsed = llm_start.elapsed();

    if args.timing {
        eprintln!(
            "[Opening rendered: {:.1}s | 1 LLM call]",
            llm_elapsed.as_secs_f64()
        );
        print_observer_summary(&observer);
    }

    println!("{}\n", opening.text);

    // Record the opening in the journal
    let noop = storyteller_core::traits::NoopObserver;
    add_turn(
        &mut journal,
        0,
        &opening.text,
        vec![bramblehoof_sheet.entity_id, pyotir_sheet.entity_id],
        vec![],
        &noop,
    );

    let mut turn: u32 = 0;

    // Build input source: scripted file or interactive stdin.
    let scripted_inputs: Option<Vec<String>> = if let Some(ref path) = args.inputs {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && l != "/quit" && l != "/q")
            .collect();
        eprintln!(
            "[Scripted mode: {} inputs from {}]",
            lines.len(),
            path.display()
        );
        Some(lines)
    } else {
        None
    };

    let mut script_iter = scripted_inputs.as_ref().map(|v| v.iter());

    // Turn loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        let input: String = if let Some(ref mut iter) = script_iter {
            // Scripted: take next line, or end the scene.
            match iter.next() {
                Some(line) => {
                    println!("> {line}");
                    line.clone()
                }
                None => break,
            }
        } else {
            // Interactive: read from stdin.
            print!("> ");
            stdout.flush()?;
            let mut line = String::new();
            let bytes = stdin.lock().read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed == "/quit" || trimmed == "/q" {
                break;
            }
            trimmed
        };

        turn += 1;
        let turn_start = Instant::now();

        // ML predictions (if model available)
        let resolver_output = if let Some(ref predictor) = predictor {
            let predict_start = Instant::now();
            let (predictions, classification) = predict_character_behaviors(
                predictor,
                &characters,
                &scene,
                &input,
                &grammar,
                event_classifier.as_ref(),
            );
            let predict_elapsed = predict_start.elapsed();

            // Display classification results
            if let Some(ref output) = classification {
                let kinds: Vec<_> = output
                    .event_kinds
                    .iter()
                    .map(|(k, c)| format!("{k}({c:.0}%)"))
                    .collect();
                let entities: Vec<_> = output
                    .entity_mentions
                    .iter()
                    .map(|e| format!("{}:{:?}", e.text, e.category))
                    .collect();
                eprintln!(
                    "[Turn {turn}: ML classification: [{}] entities: [{}]]",
                    kinds.join(", "),
                    entities.join(", ")
                );
            }

            eprintln!(
                "[Turn {turn}: {} predictions in {:.1}ms]",
                predictions.len(),
                predict_elapsed.as_secs_f64() * 1000.0
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
                scene_dynamics:
                    "A quiet arrival — the distance between them is physical and temporal"
                        .to_string(),
                conflicts: vec![],
            }
        };

        // Context assembly (deterministic — no LLM calls)
        let observer = CollectingObserver::new();
        let assembly_start = Instant::now();
        let context = assemble_narrator_context(
            &scene,
            &characters,
            &journal,
            &resolver_output,
            &input,
            &[bramblehoof_sheet.entity_id, pyotir_sheet.entity_id],
            DEFAULT_TOTAL_TOKEN_BUDGET,
            &observer,
        );
        let assembly_elapsed = assembly_start.elapsed();

        if args.timing {
            eprintln!(
                "[Turn {turn}: context assembly {:.1}ms | ~{} tokens]",
                assembly_elapsed.as_secs_f64() * 1000.0,
                context.estimated_tokens
            );
        }

        // Narrator renders — single LLM call
        eprintln!("[Turn {turn}: narrator rendering (1 LLM call)...]");
        let llm_start = Instant::now();
        let rendering = narrator.render(&context, &observer).await?;
        let llm_elapsed = llm_start.elapsed();

        let turn_total = turn_start.elapsed();

        if args.timing {
            eprintln!(
                "[Turn {turn}: LLM {:.1}s | total {:.1}s | 1 LLM call]",
                llm_elapsed.as_secs_f64(),
                turn_total.as_secs_f64()
            );
            print_observer_summary(&observer);
        }

        println!("\n{}\n", rendering.text);

        // Record this turn in the journal
        add_turn(
            &mut journal,
            turn,
            &rendering.text,
            vec![bramblehoof_sheet.entity_id, pyotir_sheet.entity_id],
            extract_emotional_markers(&input),
            &noop,
        );
    }

    let total_elapsed = total_start.elapsed();
    eprintln!(
        "\n--- Scene ended after {turn} turns ({:.1}s total) ---",
        total_elapsed.as_secs_f64()
    );

    Ok(())
}

/// Extract rough emotional markers from player input.
///
/// Very naive for the prototype — looks for emotionally charged words.
/// Production would use the event classifier.
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

/// Print a summary of observer events (for timing display).
fn print_observer_summary(observer: &CollectingObserver) {
    let events = observer.take_events();
    for event in &events {
        match &event.detail {
            PhaseEventDetail::ContextAssembled {
                preamble_tokens,
                journal_tokens,
                retrieved_tokens,
                total_tokens,
                trimmed,
            } => {
                eprintln!(
                    "  Tokens: preamble={preamble_tokens} journal={journal_tokens} \
                     retrieved={retrieved_tokens} total={total_tokens}{}",
                    if *trimmed { " [TRIMMED]" } else { "" }
                );
            }
            PhaseEventDetail::NarratorRenderingComplete {
                tokens_used: Some(tokens),
                elapsed_ms,
            } => {
                eprintln!("  Narrator: {tokens} tokens in {elapsed_ms}ms");
            }
            PhaseEventDetail::InformationBoundaryApplied {
                available,
                permitted,
                ..
            } => {
                eprintln!("  Boundary: {permitted}/{available} items permitted");
            }
            PhaseEventDetail::PredictionsEnriched {
                character_count,
                total_actions,
                estimated_tokens,
            } => {
                eprintln!(
                    "  Predictions: {character_count} characters, {total_actions} actions, ~{estimated_tokens} tokens"
                );
            }
            _ => {}
        }
    }
}

/// Resolve the path to the character predictor ONNX model file.
///
/// Checks in order:
/// 1. `STORYTELLER_MODEL_PATH` env var (directory containing model files)
/// 2. `STORYTELLER_DATA_PATH/models` (sibling data repo convention)
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
///
/// Expects a directory containing `event_classifier.onnx`, `ner_classifier.onnx`,
/// and `tokenizer.json`.
///
/// Checks in order:
/// 1. `STORYTELLER_MODEL_PATH/event_classifier/`
/// 2. `STORYTELLER_DATA_PATH/models/event_classifier/`
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
