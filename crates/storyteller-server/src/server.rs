//! gRPC server startup and configuration.

use std::net::SocketAddr;
use std::time::Duration;

use tonic::transport::Server;
use tracing::info;

use std::sync::Arc;

use storyteller_composer::SceneComposer;
use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::structured_llm::StructuredLlmConfig;
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};
use storyteller_engine::inference::frame::CharacterPredictor;
use storyteller_engine::inference::structured::OllamaStructuredProvider;

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
    /// Optional path to the ONNX character predictor model file.
    pub model_path: Option<String>,
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
            model_path: std::env::var("STORYTELLER_MODEL_PATH").ok(),
        }
    }
}

/// Start the gRPC server.
///
/// Serves both [`ComposerServiceImpl`] and [`EngineServiceImpl`] with real LLM
/// providers constructed from [`ServerConfig`].
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading descriptors from {}", config.data_path);
    let composer = Arc::new(SceneComposer::load(std::path::Path::new(
        &config.data_path,
    ))?);

    let state_manager = Arc::new(EngineStateManager::new());

    let session_store = Arc::new(SessionStore::new(std::path::Path::new(
        &config.sessions_dir,
    ))?);

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

    // Structured LLM provider for event decomposition
    let structured_llm: Option<
        Arc<dyn storyteller_core::traits::structured_llm::StructuredLlmProvider>,
    > = {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig {
            base_url: config.ollama_url.clone(),
            model: config.decomposition_model.clone(),
            ..Default::default()
        });
        tracing::info!(model = %config.decomposition_model, "Structured LLM provider created");
        Some(Arc::new(provider))
    };

    // Character predictor (optional — degrades gracefully when model not on disk)
    let predictor: Option<Arc<CharacterPredictor>> = config.model_path.as_ref().and_then(|path| {
        let model_path = std::path::Path::new(path);
        match CharacterPredictor::load(model_path) {
            Ok(p) => {
                tracing::info!(%path, "Character predictor loaded");
                Some(Arc::new(p))
            }
            Err(e) => {
                tracing::warn!(%e, "Character predictor not available");
                None
            }
        }
    });

    let grammar = Arc::new(PlutchikWestern::new());

    let providers = Arc::new(EngineProviders {
        narrator_llm,
        structured_llm,
        intent_llm: Some(intent_llm),
        predictor,
        grammar,
        narrator_model: config.narrator_model.clone(),
        decomposition_model: config.decomposition_model.clone(),
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
