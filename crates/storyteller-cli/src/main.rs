// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Storyteller CLI — primary entry point.
//!
//! Subcommands:
//!  - `playtest` — run an automated playtest from a composition file
//!  - `compose` — compose a scene and capture its JSON
//!  - `composer sync` — sync descriptor cache from the server
//!  - `composer list <category>` — list cached descriptors

use clap::Parser;

mod compose;
mod composer_cache;
mod player_simulation;
mod playtest;

#[derive(Parser)]
#[command(name = "storyteller-cli", about = "Storyteller engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Run an automated playtest against the engine server
    Playtest(playtest::PlaytestArgs),

    /// Compose a scene via the engine server
    Compose(compose::ComposeArgs),

    /// Manage the local composer descriptor cache
    #[command(subcommand)]
    Composer(ComposerCommands),
}

#[derive(clap::Subcommand)]
enum ComposerCommands {
    /// Sync descriptor cache from the server
    Sync,

    /// List cached descriptors
    #[command(subcommand)]
    List(ListCommands),
}

#[derive(clap::Subcommand)]
enum ListCommands {
    /// List available genres
    Genres,
    /// List archetypes for a genre
    Archetypes {
        #[arg(long)]
        genre: String,
    },
    /// List profiles for a genre
    Profiles {
        #[arg(long)]
        genre: String,
    },
    /// List dynamics for a genre
    Dynamics {
        #[arg(long)]
        genre: String,
    },
    /// List name pools for a genre
    Names {
        #[arg(long)]
        genre: String,
    },
    /// List settings for a genre
    Settings {
        #[arg(long)]
        genre: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Playtest(args) => playtest::run(args).await?,
        Commands::Compose(args) => compose::run(args).await?,
        Commands::Composer(cmd) => match cmd {
            ComposerCommands::Sync => {
                let config = storyteller_client::ClientConfig::from_env();
                let mut client = storyteller_client::StorytellerClient::connect(config).await?;
                let cache = composer_cache::ComposerCache::new(
                    composer_cache::ComposerCache::default_path(),
                );
                cache.sync(&mut client).await?;
                println!("Composer cache synced successfully.");
            }
            ComposerCommands::List(list_cmd) => {
                let cache = composer_cache::ComposerCache::new(
                    composer_cache::ComposerCache::default_path(),
                );
                let entries = match list_cmd {
                    ListCommands::Genres => cache.list("genres", None)?,
                    ListCommands::Archetypes { genre } => cache.list("archetypes", Some(&genre))?,
                    ListCommands::Profiles { genre } => cache.list("profiles", Some(&genre))?,
                    ListCommands::Dynamics { genre } => cache.list("dynamics", Some(&genre))?,
                    ListCommands::Names { genre } => cache.list("names", Some(&genre))?,
                    ListCommands::Settings { genre } => cache.list("settings", Some(&genre))?,
                };
                for entry in &entries {
                    println!(
                        "{:<30} {:<40} {}",
                        entry.slug, entry.entity_id, entry.display_name
                    );
                }
            }
        },
    }

    Ok(())
}
