//! Standalone binary entry point for the storyteller gRPC server.

use storyteller_server::server::{run_server, ServerConfig};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let log_broadcast = storyteller_server::create_log_broadcast();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(EnvFilter::from_default_env())
        .with(storyteller_server::BroadcastTracingLayer::new(
            log_broadcast.clone(),
        ))
        .init();

    let config = ServerConfig::from_env();
    run_server(config, Some(log_broadcast)).await?;

    Ok(())
}
