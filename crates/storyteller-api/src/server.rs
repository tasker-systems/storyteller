//! gRPC server startup and configuration.

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::Server;
use tracing::info;

use storyteller_composer::SceneComposer;

use crate::engine::EngineStateManager;
use crate::grpc::composer_service::ComposerServiceImpl;
use crate::persistence::SessionStore;
use crate::proto::composer_service_server::ComposerServiceServer;

/// Server configuration from environment variables.
#[derive(Debug)]
pub struct ServerConfig {
    pub grpc_port: u16,
    pub data_path: String,
    pub sessions_dir: String,
    pub narrator_model: String,
    pub decomposition_model: String,
    pub intent_model: String,
    pub ollama_url: String,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_port: std::env::var("STORYTELLER_GRPC_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50051),
            data_path: std::env::var("STORYTELLER_DATA_PATH")
                .unwrap_or_else(|_| "../storyteller-data/training-data/descriptors".to_string()),
            sessions_dir: std::env::var("STORYTELLER_SESSIONS_DIR")
                .unwrap_or_else(|_| ".story/sessions".to_string()),
            narrator_model: std::env::var("STORYTELLER_NARRATOR_MODEL")
                .unwrap_or_else(|_| "qwen2.5:14b".to_string()),
            decomposition_model: std::env::var("STORYTELLER_DECOMPOSITION_MODEL")
                .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string()),
            intent_model: std::env::var("STORYTELLER_INTENT_MODEL")
                .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string()),
            ollama_url: std::env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        }
    }
}

/// Start the gRPC server.
///
/// Currently serves only the [`ComposerService`] — the engine service requires
/// LLM providers which are wired in Task 12.
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading descriptors from {}", config.data_path);
    let composer = Arc::new(
        SceneComposer::load(std::path::Path::new(&config.data_path))
            .map_err(|e| format!("Failed to load descriptors: {e}"))?,
    );

    let _state_manager = Arc::new(EngineStateManager::new());

    let _session_store = Arc::new(
        SessionStore::new(std::path::Path::new(&config.sessions_dir))
            .map_err(|e| format!("Failed to create session store: {e}"))?,
    );

    // TODO: Construct real LLM providers from config.
    // The engine service needs LLM providers — wired in Task 12.

    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!("Starting gRPC server on {addr}");

    let composer_service = ComposerServiceImpl::new(composer.clone());
    // Note: EngineServiceImpl requires EngineProviders which needs LLM.
    // Added incrementally after Task 12 wires LLM providers.
    // .add_service(StorytellerEngineServer::new(engine_service))

    Server::builder()
        .add_service(ComposerServiceServer::new(composer_service))
        .serve(addr)
        .await?;

    Ok(())
}
