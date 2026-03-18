//! Standalone binary entry point for the storyteller gRPC server.

use storyteller_server::server::{run_server, ServerConfig};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let log_broadcast = storyteller_server::create_log_broadcast();

    // Apply EnvFilter only to the console layer so the broadcast layer
    // receives all events regardless of RUST_LOG setting. The workshop's
    // Logs tab applies its own filtering client-side.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_filter(env_filter),
        )
        .with(storyteller_server::BroadcastTracingLayer::new(
            log_broadcast.clone(),
        ))
        .init();

    let config = ServerConfig::from_env();
    run_server(config, Some(log_broadcast)).await?;

    Ok(())
}
