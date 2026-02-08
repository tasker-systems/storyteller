//! Interactive scene player — narrator-centric architecture.
//!
//! Runs workshop scenes using the three-tier context assembly pipeline
//! with a single Narrator LLM call per turn.
//!
//! Usage:
//!   cargo run --bin play-scene -- --model mistral
//!   cargo run --bin play-scene -- --model qwen2.5:32b --temperature 0.7
//!
//! Requires a running Ollama instance at localhost:11434.

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;

use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::phase_observer::{CollectingObserver, PhaseEventDetail};
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::scene::SceneId;

use storyteller_engine::agents::narrator::NarratorAgent;
use storyteller_engine::context::journal::add_turn;
use storyteller_engine::context::{assemble_narrator_context, DEFAULT_TOTAL_TOKEN_BUDGET};
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};
use storyteller_engine::workshop::the_flute_kept;

#[derive(Parser, Debug)]
#[command(
    name = "play-scene",
    about = "Interactive scene player — single Narrator LLM call per turn"
)]
struct Args {
    /// Ollama model name
    #[arg(long, default_value = "mistral")]
    model: String,

    /// LLM sampling temperature
    #[arg(long, default_value_t = 0.8)]
    temperature: f32,

    /// Ollama base URL
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Show timing details for each phase
    #[arg(long, default_value_t = true)]
    timing: bool,
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

    // Create LLM provider
    let config = ExternalServerConfig {
        base_url: args.ollama_url,
        model: args.model,
        ..Default::default()
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(ExternalServerProvider::new(config));

    // Scene journal — rolling compressed history
    let mut journal = SceneJournal::new(SceneId::new(), 1200);

    // Resolver output — no ML predictions yet, so the Narrator infers
    // character behavior from the rich context it receives.
    let resolver_output = ResolverOutput {
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
        &resolver_output,
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
    let mut narrator =
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

    // Turn loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("> ");
        stdout.flush()?;

        let mut line = String::new();
        let bytes = stdin.lock().read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" || input == "/q" {
            break;
        }

        turn += 1;
        let turn_start = Instant::now();

        // Context assembly (deterministic — no LLM calls)
        let observer = CollectingObserver::new();
        let assembly_start = Instant::now();
        let context = assemble_narrator_context(
            &scene,
            &characters,
            &journal,
            &resolver_output,
            input,
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
            extract_emotional_markers(input),
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
            _ => {}
        }
    }
}
