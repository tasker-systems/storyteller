//! Interactive scene player — run "The Flute Kept" against local Ollama.
//!
//! Usage:
//!   cargo run --bin play-scene -- --model mistral
//!   cargo run --bin play-scene -- --model llama3.1 --temperature 0.7
//!
//! Requires a running Ollama instance at localhost:11434.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use clap::Parser;

use storyteller_engine::agents::character::CharacterAgent;
use storyteller_engine::agents::narrator::NarratorAgent;
use storyteller_engine::agents::{reconciler, storykeeper};
use storyteller_engine::inference::external::ExternalServerProvider;
use storyteller_engine::workshop::the_flute_kept;

use storyteller_core::types::message::PlayerInput;

#[derive(Parser, Debug)]
#[command(
    name = "play-scene",
    about = "Interactive scene player for storyteller prototype"
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (respects RUST_LOG env var)
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

    // Create LLM provider
    let config = storyteller_engine::inference::external::ExternalServerConfig {
        base_url: args.ollama_url,
        model: args.model,
        ..Default::default()
    };
    let llm: Arc<dyn storyteller_core::traits::llm::LlmProvider> =
        Arc::new(ExternalServerProvider::new(config));

    // Create agents
    let mut bramblehoof_agent = CharacterAgent::new(bramblehoof_sheet.clone(), Arc::clone(&llm))
        .with_temperature(args.temperature);
    let mut pyotir_agent = CharacterAgent::new(pyotir_sheet.clone(), Arc::clone(&llm))
        .with_temperature(args.temperature);
    let mut narrator =
        NarratorAgent::new(&scene, Arc::clone(&llm)).with_temperature(args.temperature);

    // Scene opening
    eprintln!("[Generating scene opening...]\n");
    let opening = narrator.render_opening().await?;
    println!("{}\n", opening.text);

    let mut turn: u32 = 0;
    let mut last_narration = opening.text;

    // Turn loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("> ");
        stdout.flush()?;

        let mut line = String::new();
        let bytes = stdin.lock().read_line(&mut line)?;
        if bytes == 0 {
            // EOF
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
        let player_input = PlayerInput {
            text: input.to_string(),
            turn_number: turn,
        };

        // Storykeeper filters
        let directives = storykeeper::produce_directives(
            &player_input,
            &scene,
            &[&bramblehoof_sheet, &pyotir_sheet],
            Some(&last_narration),
        );

        // Character agents deliberate in parallel
        eprintln!("[Turn {turn}: characters deliberating...]");

        let (bramble_result, pyotir_result) = tokio::join!(
            bramblehoof_agent.deliberate(&directives[0]),
            pyotir_agent.deliberate(&directives[1]),
        );

        let bramble_intent = bramble_result?;
        let pyotir_intent = pyotir_result?;

        tracing::debug!(
            bramblehoof_action = %bramble_intent.intent,
            bramblehoof_subtext = %bramble_intent.emotional_subtext,
            "Bramblehoof intent"
        );
        tracing::debug!(
            pyotir_action = %pyotir_intent.intent,
            pyotir_subtext = %pyotir_intent.emotional_subtext,
            "Pyotir intent"
        );

        // Reconcile
        let reconciled = reconciler::reconcile(vec![bramble_intent, pyotir_intent]);

        // Narrator renders
        eprintln!("[Narrator rendering...]");
        let rendering = narrator.render(&reconciled, &player_input).await?;
        last_narration = rendering.text.clone();

        println!("\n{}\n", rendering.text);
    }

    eprintln!("\n--- Scene ended after {turn} turns ---");
    Ok(())
}
