//! Game engine server binary.
//!
//! Assembles the Bevy App with the StorytellerEnginePlugin and runs it.
//! This is the main entry point for running the storyteller engine.

use bevy_app::App;
use storyteller_engine::StorytellerEnginePlugin;

fn main() {
    // Initialize tracing subscriber for structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting storyteller engine server");

    App::new().add_plugins(StorytellerEnginePlugin).run();
}
