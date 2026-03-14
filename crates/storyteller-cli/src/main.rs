//! Storyteller CLI — primary entry point.
//!
//! Subcommands: playtest, compose, composer (added in later tasks).

use clap::Parser;

#[derive(Parser)]
#[command(name = "storyteller-cli", about = "Storyteller engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    // Subcommands will be added in later tasks
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {}
}
