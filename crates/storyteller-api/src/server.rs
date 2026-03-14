//! gRPC server startup and configuration.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tonic::transport::Server;
use tracing::info;

use storyteller_composer::SceneComposer;
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};

use crate::engine::{EngineProviders, EngineStateManager};
use crate::grpc::composer_service::ComposerServiceImpl;
use crate::grpc::engine_service::EngineServiceImpl;
use crate::persistence::SessionStore;
use crate::proto::composer_service_server::ComposerServiceServer;
use crate::proto::storyteller_engine_server::StorytellerEngineServer;

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
/// Serves both [`ComposerService`] and [`StorytellerEngine`] with real LLM
/// providers constructed from [`ServerConfig`].
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading descriptors from {}", config.data_path);
    let composer = Arc::new(
        SceneComposer::load(std::path::Path::new(&config.data_path))
            .map_err(|e| format!("Failed to load descriptors: {e}"))?,
    );

    let state_manager = Arc::new(EngineStateManager::new());

    let session_store = Arc::new(
        SessionStore::new(std::path::Path::new(&config.sessions_dir))
            .map_err(|e| format!("Failed to create session store: {e}"))?,
    );

    // Construct LLM providers from config.
    // Providers are built without requiring Ollama to be running — connections
    // are made lazily on first request.
    let narrator_llm = Arc::new(ExternalServerProvider::new(ExternalServerConfig {
        base_url: config.ollama_url.clone(),
        model: config.narrator_model.clone(),
        timeout: Duration::from_secs(120),
    }));

    let intent_llm = Arc::new(ExternalServerProvider::new(ExternalServerConfig {
        base_url: config.ollama_url.clone(),
        model: config.intent_model.clone(),
        timeout: Duration::from_secs(60),
    }));

    let providers = Arc::new(EngineProviders {
        narrator_llm,
        structured_llm: None, // TODO: construct OllamaStructuredProvider (Task 13+)
        intent_llm: Some(intent_llm),
        predictor_available: false, // TODO: construct CharacterPredictor
    });

    info!(
        narrator_model = %config.narrator_model,
        intent_model = %config.intent_model,
        ollama_url = %config.ollama_url,
        "LLM providers constructed"
    );

    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!("Starting gRPC server on {addr}");

    let composer_service = ComposerServiceImpl::new(composer.clone());
    let engine_service = EngineServiceImpl::new(
        composer.clone(),
        state_manager.clone(),
        session_store.clone(),
        providers.clone(),
    );

    Server::builder()
        .add_service(ComposerServiceServer::new(composer_service))
        .add_service(StorytellerEngineServer::new(engine_service))
        .serve(addr)
        .await?;

    Ok(())
}
