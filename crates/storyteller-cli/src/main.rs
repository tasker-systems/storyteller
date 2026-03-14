//! Storyteller CLI — primary entry point.
//!
//! Provides CLI subcommands for running the gRPC server and development utilities.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "storyteller-cli", about = "Storyteller engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gRPC engine server.
    Serve,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Load .env if available
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    match cli.command {
        Commands::Serve => {
            let config = storyteller_server::server::ServerConfig::from_env();
            storyteller_server::server::run_server(config).await?;
        }
    }

    Ok(())
}
