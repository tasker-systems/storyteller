//! Integration tests for gRPC services.
//!
//! These tests start a real gRPC server and verify RPCs via tonic client stubs.
//!
//! Tier 1 (no feature flags): Requires STORYTELLER_DATA_PATH.
//! Tier 2 (test-llm feature): Also requires a running Ollama server.

use storyteller_server::engine::{EngineProviders, EngineStateManager};
use storyteller_server::grpc::composer_service::ComposerServiceImpl;
use storyteller_server::grpc::engine_service::EngineServiceImpl;
use storyteller_server::persistence::SessionStore;
use storyteller_server::proto::composer_service_client::ComposerServiceClient;
use storyteller_server::proto::composer_service_server::ComposerServiceServer;
use storyteller_server::proto::storyteller_engine_client::StorytellerEngineClient;
use storyteller_server::proto::storyteller_engine_server::StorytellerEngineServer;
use storyteller_server::proto::*;

use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
#[cfg(feature = "test-llm")]
use tokio_stream::StreamExt as _;
use tonic::transport::{Channel, Server};

// ---------------------------------------------------------------------------
// Mock LLM provider for tests that don’t require Ollama
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MockLlm;

#[async_trait::async_trait]
impl storyteller_core::traits::llm::LlmProvider for MockLlm {
    async fn complete(
        &self,
        _req: storyteller_core::traits::llm::CompletionRequest,
    ) -> storyteller_core::errors::StorytellerResult<
        storyteller_core::traits::llm::CompletionResponse,
    > {
        Ok(storyteller_core::traits::llm::CompletionResponse {
            content: "[mock narrator response]".to_string(),
            tokens_used: 10,
        })
    }
}

// ---------------------------------------------------------------------------
// Test server helpers
// ---------------------------------------------------------------------------

/// Starts a ComposerService-only test server.
/// Returns the server URL or `None` if STORYTELLER_DATA_PATH is not set.
async fn start_test_server() -> Option<String> {
    let data_path = std::env::var("STORYTELLER_DATA_PATH").ok()?;
    let composer = Arc::new(
        storyteller_composer::SceneComposer::load(std::path::Path::new(&data_path))
            .expect("load descriptors"),
    );

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    let service = ComposerServiceImpl::new(composer);

    tokio::spawn(async move {
        Server::builder()
            .add_service(ComposerServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Some(url)
}

/// Result of starting an engine test server.
struct EngineTestServer {
    url: String,
    _temp_dir: TempDir, // keep alive for session store lifetime
}

/// Starts an EngineService test server with mock providers and (optionally) real LLM.
/// Returns `None` if STORYTELLER_DATA_PATH is not set.
async fn start_engine_test_server(
    narrator_llm: Arc<dyn storyteller_core::traits::llm::LlmProvider>,
) -> Option<EngineTestServer> {
    let data_path = std::env::var("STORYTELLER_DATA_PATH").ok()?;
    let composer = Arc::new(
        storyteller_composer::SceneComposer::load(std::path::Path::new(&data_path))
            .expect("load descriptors"),
    );

    let temp_dir = TempDir::new().unwrap();
    let session_store = Arc::new(SessionStore::new(temp_dir.path()).expect("create session store"));
    let state_manager = Arc::new(EngineStateManager::new());
    let providers = Arc::new(EngineProviders {
        narrator_llm,
        structured_llm: None,
        intent_llm: None,
        predictor: None,
        grammar: Arc::new(storyteller_core::grammars::PlutchikWestern::new()),
        narrator_model: "test-model".to_string(),
        decomposition_model: String::new(),
    });

    let engine_service = EngineServiceImpl::new(composer, state_manager, session_store, providers);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    tokio::spawn(async move {
        Server::builder()
            .add_service(StorytellerEngineServer::new(engine_service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Some(EngineTestServer {
        url,
        _temp_dir: temp_dir,
    })
}

// ---------------------------------------------------------------------------
// Engine service tests
// ---------------------------------------------------------------------------

/// CheckHealth with mock LLM: narrator_llm is healthy; optional subsystems
/// (structured_llm, intent_llm, predictor) are unavailable → overall degraded.
#[tokio::test]
async fn check_health_reports_degraded_with_mock_llm() {
    let Some(server) = start_engine_test_server(Arc::new(MockLlm)).await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(server.url)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = StorytellerEngineClient::new(channel);

    let response = client.check_health(()).await.unwrap();
    let health = response.into_inner();

    assert_eq!(
        health.status, "degraded",
        "overall status should be degraded when optional subsystems are missing"
    );

    // Four subsystems are always emitted.
    assert_eq!(
        health.subsystems.len(),
        4,
        "should have exactly 4 subsystems"
    );

    let narrator = health
        .subsystems
        .iter()
        .find(|s| s.name == "narrator_llm")
        .expect("narrator_llm subsystem should be present");
    assert_eq!(narrator.status, "healthy", "narrator_llm should be healthy");
    assert!(
        narrator.message.is_none(),
        "healthy narrator_llm should have no message"
    );

    for name in &["structured_llm", "intent_llm", "predictor"] {
        let sub = health
            .subsystems
            .iter()
            .find(|s| s.name.as_str() == *name)
            .unwrap_or_else(|| panic!("{name} subsystem should be present"));
        assert_eq!(
            sub.status, "unavailable",
            "{name} should be unavailable when not configured"
        );
        assert!(
            sub.message.is_some(),
            "{name} should have an explanatory message"
        );
    }
}

/// ComposeScene end-to-end: verify event stream order and non-empty narrator prose.
///
/// Requires: STORYTELLER_DATA_PATH env var + a running Ollama server.
#[cfg(feature = "test-llm")]
#[tokio::test(flavor = "multi_thread")]
async fn compose_scene_integration() {
    use std::time::Duration;
    use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};

    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let narrator_model =
        std::env::var("STORYTELLER_NARRATOR_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());

    let narrator_llm = Arc::new(ExternalServerProvider::new(ExternalServerConfig {
        base_url: ollama_url,
        model: narrator_model,
        timeout: Duration::from_secs(180),
    }));

    let Some(server) = start_engine_test_server(narrator_llm).await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(server.url)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = StorytellerEngineClient::new(channel);

    let request = ComposeSceneRequest {
        genre_id: "low_fantasy_folklore".to_string(),
        profile_id: "quiet_reunion".to_string(),
        cast: vec![
            CastMember {
                archetype_id: "wandering_artist".to_string(),
                name: Some("Mira".to_string()),
                role: "protagonist".to_string(),
            },
            CastMember {
                archetype_id: "stoic_survivor".to_string(),
                name: Some("Aldric".to_string()),
                role: "antagonist".to_string(),
            },
        ],
        dynamics: vec![],
        title_override: None,
        setting_override: None,
        seed: None,
    };

    let response = client.compose_scene(request).await.unwrap();
    let mut stream = response.into_inner();

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        let event = event.expect("stream should not error");
        events.push(event);
    }

    // Verify minimum event count: composition + scene_composed + goals + narrator_phase + narrator_complete + turn_complete
    assert!(
        events.len() >= 6,
        "expected at least 6 events, got {}",
        events.len()
    );

    // Check event ordering by payload type.
    let payloads: Vec<&str> = events
        .iter()
        .filter_map(|e| match &e.payload {
            Some(engine_event::Payload::PhaseStarted(p)) => Some(p.phase.as_str()),
            Some(engine_event::Payload::SceneComposed(_)) => Some("SceneComposed"),
            Some(engine_event::Payload::Goals(_)) => Some("Goals"),
            Some(engine_event::Payload::NarratorComplete(_)) => Some("NarratorComplete"),
            Some(engine_event::Payload::TurnComplete(_)) => Some("TurnComplete"),
            _ => None,
        })
        .collect();

    assert!(
        payloads.contains(&"composition"),
        "expected PhaseStarted(composition) event"
    );
    assert!(
        payloads.contains(&"SceneComposed"),
        "expected SceneComposed event"
    );
    assert!(payloads.contains(&"Goals"), "expected GoalsGenerated event");
    assert!(
        payloads.contains(&"narrator"),
        "expected PhaseStarted(narrator) event"
    );
    assert!(
        payloads.contains(&"NarratorComplete"),
        "expected NarratorComplete event"
    );
    assert!(
        payloads.contains(&"TurnComplete"),
        "expected TurnComplete event"
    );

    // composition phase must precede narrator phase
    let composition_idx = payloads.iter().position(|&p| p == "composition").unwrap();
    let narrator_idx = payloads.iter().position(|&p| p == "narrator").unwrap();
    assert!(
        composition_idx < narrator_idx,
        "composition phase should precede narrator phase"
    );

    // Narrator prose must be non-empty
    let narrator_complete = events
        .iter()
        .find_map(|e| {
            if let Some(engine_event::Payload::NarratorComplete(n)) = &e.payload {
                Some(n)
            } else {
                None
            }
        })
        .expect("NarratorComplete event should be present");
    assert!(
        !narrator_complete.prose.is_empty(),
        "narrator prose should not be empty"
    );
}

// ---------------------------------------------------------------------------
// ComposerService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_genres_returns_non_empty() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client.list_genres(()).await.unwrap();
    let genres = response.into_inner().genres;

    assert!(!genres.is_empty(), "should return at least one genre");
    assert!(
        genres.iter().any(|g| g.slug == "low_fantasy_folklore"),
        "should contain low_fantasy_folklore"
    );
}

#[tokio::test]
async fn profiles_for_genre_returns_results() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client
        .get_profiles_for_genre(GenreRequest {
            genre_id: "low_fantasy_folklore".to_string(),
        })
        .await
        .unwrap();

    let profiles = response.into_inner().profiles;
    assert!(!profiles.is_empty(), "should return profiles for genre");
}

#[tokio::test]
async fn invalid_genre_returns_empty_profiles() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client
        .get_profiles_for_genre(GenreRequest {
            genre_id: "nonexistent_genre".to_string(),
        })
        .await
        .unwrap();

    assert!(response.into_inner().profiles.is_empty());
}
