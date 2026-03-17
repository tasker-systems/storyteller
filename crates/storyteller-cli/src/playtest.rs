//! Playtest subcommand — run an automated multi-turn playtest.
//!
//! Takes a `composition.json` file (from `storyteller-cli compose --output`),
//! re-plays the scene selections to get a fresh session, then runs a turn loop
//! using `PlayerSimulation` to generate player inputs.

use crate::player_simulation::PlayerSimulation;
use std::time::Instant;
use storyteller_client::{engine_event, ClientConfig, StorytellerClient};
use storyteller_core::types::health::HealthStatus;

#[derive(clap::Args)]
pub struct PlaytestArgs {
    /// Path to composition.json file (from `compose --output`)
    #[arg(long, short)]
    file: String,

    /// Number of turns to play
    #[arg(long, default_value = "5")]
    turns: u32,

    /// Player simulation model name (falls back to STORYTELLER_PLAYER_MODEL env var)
    #[arg(long)]
    player_model: Option<String>,
}

pub async fn run(args: PlaytestArgs) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Connect and health check
    let config = ClientConfig::from_env();
    let mut client = StorytellerClient::connect(config).await?;

    let health = client.check_health().await?;
    if let Some(narrator) = health.subsystems.iter().find(|s| s.name == "narrator_llm") {
        if narrator.status == HealthStatus::Unavailable {
            return Err("Narrator LLM is unavailable. Cannot run playtest.".into());
        }
    }
    println!("Connected to server. Health: {:?}", health.status);

    // Read and parse composition file
    let composition_data = std::fs::read_to_string(&args.file)?;
    let composition: serde_json::Value = serde_json::from_str(&composition_data)?;

    // Build ComposeSceneRequest from the composition file's selections
    let request = build_compose_request(&composition)?;

    // Compose scene
    println!("Composing scene...");
    let mut stream = client.compose_scene(request).await?;

    let mut session_id = String::new();
    let mut narrator_output = String::new();
    let mut protagonist_name = String::new();
    let mut composition_json = String::new();

    while let Some(event) = stream.message().await? {
        session_id = event.session_id.clone();
        if let Some(payload) = &event.payload {
            match payload {
                engine_event::Payload::SceneComposed(scene) => {
                    println!("Scene: {}", scene.title);
                    composition_json = scene.composition_json.clone();
                    // Protagonist is the first cast member (index 0)
                    protagonist_name = scene.cast_names.first().cloned().unwrap_or_default();
                }
                engine_event::Payload::NarratorComplete(narrator) => {
                    narrator_output = narrator.prose.clone();
                    println!("\n--- Opening ---\n{}\n", narrator.prose);
                }
                engine_event::Payload::Error(err) => {
                    return Err(format!("Composition error: {}", err.message).into());
                }
                _ => {}
            }
        }
    }

    if session_id.is_empty() {
        return Err("No session created — composition may have failed".into());
    }

    // Set up player simulation
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let player_model = args
        .player_model
        .or_else(|| std::env::var("STORYTELLER_PLAYER_MODEL").ok())
        .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
    let protagonist_context =
        PlayerSimulation::build_protagonist_context(&protagonist_name, &composition_json);
    let player_sim = PlayerSimulation::new(&ollama_url, &player_model, &protagonist_context);

    // Turn loop
    for turn in 1..=args.turns {
        println!("--- Turn {turn}/{} ---", args.turns);

        // Generate player input from the latest narrator output
        let player_input = player_sim.generate_input(&narrator_output).await?;
        println!("[Player]: {player_input}");

        // Submit input to server
        let submit_request = storyteller_client::SubmitInputRequest {
            session_id: session_id.clone(),
            input: player_input,
        };

        let mut response_stream = client.submit_input(submit_request).await?;
        while let Some(event) = response_stream.message().await? {
            if let Some(payload) = &event.payload {
                match payload {
                    engine_event::Payload::NarratorComplete(narrator) => {
                        narrator_output = narrator.prose.clone();
                        println!("[Narrator]: {}\n", narrator.prose);
                    }
                    engine_event::Payload::Error(err) => {
                        eprintln!("Turn error: {}", err.message);
                    }
                    _ => {}
                }
            }
        }
    }

    // Summary
    let elapsed = start.elapsed();
    println!("--- Playtest Complete ---");
    println!("Session:  {session_id}");
    println!("Turns:    {}", args.turns);
    println!("Elapsed:  {:.1}s", elapsed.as_secs_f64());

    Ok(())
}

/// Build a `ComposeSceneRequest` from a `composition.json` file.
///
/// The composition file has a `selections` field (produced by the server when
/// composing or by `SceneComposed.composition_json`).
fn build_compose_request(
    composition: &serde_json::Value,
) -> Result<storyteller_client::ComposeSceneRequest, Box<dyn std::error::Error>> {
    let selections = composition
        .get("selections")
        .ok_or("composition.json missing 'selections' field")?;

    let genre_id = selections
        .get("genre_id")
        .and_then(|v| v.as_str())
        .ok_or("missing genre_id in selections")?
        .to_string();

    let profile_id = selections
        .get("profile_id")
        .and_then(|v| v.as_str())
        .ok_or("missing profile_id in selections")?
        .to_string();

    let cast = selections
        .get("cast")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    Some(storyteller_client::proto::CastMember {
                        archetype_id: c.get("archetype_id")?.as_str()?.to_string(),
                        name: c
                            .get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string()),
                        role: c.get("role")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let dynamics = selections
        .get("dynamics")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    Some(storyteller_client::proto::DynamicPairing {
                        dynamic_id: d.get("dynamic_id")?.as_str()?.to_string(),
                        cast_index_a: d.get("cast_index_a")?.as_u64()? as u32,
                        cast_index_b: d.get("cast_index_b")?.as_u64()? as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(storyteller_client::ComposeSceneRequest {
        genre_id,
        profile_id,
        cast,
        dynamics,
        seed: selections.get("seed").and_then(|v| v.as_u64()),
        title_override: selections
            .get("title_override")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        setting_override: selections
            .get("setting_override")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        player_character: None,
    })
}
