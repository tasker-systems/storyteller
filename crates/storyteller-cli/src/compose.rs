//! Compose subcommand — compose a scene via the engine server.
//!
//! Resolves slugs from the local composer cache, sends a `ComposeSceneRequest`
//! to the server, and emits the composition JSON (suitable for use with
//! `storyteller-cli playtest --file`).

use crate::composer_cache::ComposerCache;
use storyteller_client::{ClientConfig, StorytellerClient};

#[derive(clap::Args)]
pub struct ComposeArgs {
    /// Genre slug (from composer cache)
    #[arg(long)]
    genre: String,

    /// Profile slug (from composer cache)
    #[arg(long)]
    profile: String,

    /// Cast selections as "archetype_slug:role" pairs, comma-separated
    /// e.g. --cast wandering_artist:protagonist,stoic_survivor:antagonist
    #[arg(long, value_delimiter = ',')]
    cast: Vec<String>,

    /// Dynamic pairings as "slug:idx_a-idx_b" entries, semicolon-separated
    /// e.g. --dynamics bitter_rivals:0-1
    #[arg(long, value_delimiter = ';')]
    dynamics: Vec<String>,

    /// Output file path (stdout if not specified)
    #[arg(long, short)]
    output: Option<String>,

    /// Random seed for deterministic composition
    #[arg(long)]
    seed: Option<u64>,
}

pub async fn run(args: ComposeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let cache = ComposerCache::new(ComposerCache::default_path());

    // Resolve genre and profile slugs
    let genre_id = cache.resolve_slug("genres", None, &args.genre)?;
    let profile_id = cache.resolve_slug("profiles", Some(&args.genre), &args.profile)?;

    // Parse and resolve cast selections
    let mut cast = Vec::new();
    for entry in &args.cast {
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(
                format!("Invalid cast format: '{entry}'. Use 'archetype_slug:role'").into(),
            );
        }
        let archetype_id = cache.resolve_slug("archetypes", Some(&args.genre), parts[0])?;
        cast.push(storyteller_client::proto::CastMember {
            archetype_id,
            name: None, // server assigns from name pool
            role: parts[1].to_string(),
        });
    }

    // Parse and resolve dynamic pairings
    let mut dynamics = Vec::new();
    for entry in &args.dynamics {
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(
                format!("Invalid dynamics format: '{entry}'. Use 'slug:idx_a-idx_b'").into(),
            );
        }
        let dynamic_id = cache.resolve_slug("dynamics", Some(&args.genre), parts[0])?;
        let indices: Vec<&str> = parts[1].splitn(2, '-').collect();
        if indices.len() != 2 {
            return Err(format!(
                "Invalid dynamics indices: '{}'. Use 'idx_a-idx_b'",
                parts[1]
            )
            .into());
        }
        dynamics.push(storyteller_client::proto::DynamicPairing {
            dynamic_id,
            cast_index_a: indices[0].parse()?,
            cast_index_b: indices[1].parse()?,
        });
    }

    // Connect to server and compose
    let config = ClientConfig::from_env();
    let mut client = StorytellerClient::connect(config).await?;

    let request = storyteller_client::ComposeSceneRequest {
        genre_id,
        profile_id,
        cast,
        dynamics,
        seed: args.seed,
        title_override: None,
        setting_override: None,
        player_character: None,
    };

    let mut stream = client.compose_scene(request).await?;

    let mut composition_json_str: Option<String> = None;
    while let Some(event) = stream.message().await? {
        if let Some(payload) = &event.payload {
            match payload {
                storyteller_client::engine_event::Payload::SceneComposed(scene) => {
                    composition_json_str = Some(scene.composition_json.clone());
                    println!("Scene composed: {}", scene.title);
                }
                storyteller_client::engine_event::Payload::NarratorComplete(narrator) => {
                    println!("\n--- Opening ---\n{}\n", narrator.prose);
                }
                storyteller_client::engine_event::Payload::Error(err) => {
                    return Err(format!("Composition failed: {}", err.message).into());
                }
                _ => {}
            }
        }
    }

    let json = composition_json_str.ok_or("No SceneComposed event received")?;
    // Pretty-print if valid JSON
    let json = serde_json::from_str::<serde_json::Value>(&json)
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or(json.clone()))
        .unwrap_or(json);

    match args.output {
        Some(path) => {
            std::fs::write(&path, &json)?;
            println!("Composition written to {path}");
        }
        None => println!("{json}"),
    }

    Ok(())
}
