//! Standalone binary entry point for the storyteller gRPC server.

use storyteller_server::server::{run_server, ServerConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let config = ServerConfig::from_env();
    run_server(config).await?;

    Ok(())
}
