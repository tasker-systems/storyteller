//! Integration tests for `storyteller-client` against a running server.
//!
//! These tests require both `STORYTELLER_DATA_PATH` to be set and a
//! running `storyteller-server` (and Ollama for LLM tests).
//!
//! Gate behind `--features test-llm` so they are excluded from CI.

#[cfg(feature = "test-llm")]
mod integration {
    use storyteller_client::{ClientConfig, StorytellerClient};
    use storyteller_core::types::health::HealthStatus;

    /// Health check passes and reports at least the narrator_llm subsystem.
    #[tokio::test]
    async fn health_check_reports_subsystems() {
        let config = ClientConfig::from_env();
        let mut client = StorytellerClient::connect(config)
            .await
            .expect("Server should be running for integration tests (set STORYTELLER_SERVER_URL)");

        let health = client
            .check_health()
            .await
            .expect("Health check should succeed");

        assert!(
            !health.subsystems.is_empty(),
            "Should report at least one subsystem"
        );

        let narrator = health
            .subsystems
            .iter()
            .find(|s| s.name == "narrator_llm")
            .expect("Should report narrator_llm subsystem");

        assert_ne!(
            narrator.status,
            HealthStatus::Unavailable,
            "narrator_llm should not be unavailable when server is running"
        );
    }

    /// list_genres returns non-empty genres from the running server.
    #[tokio::test]
    async fn list_genres_returns_results() {
        let config = ClientConfig::from_env();
        let mut client = StorytellerClient::connect(config)
            .await
            .expect("Server should be running");

        let genres = client
            .list_genres()
            .await
            .expect("list_genres should succeed");
        assert!(!genres.genres.is_empty(), "Should have at least one genre");
    }

    /// get_profiles_for_genre returns profiles for a known genre.
    #[tokio::test]
    async fn get_profiles_for_known_genre() {
        let config = ClientConfig::from_env();
        let mut client = StorytellerClient::connect(config)
            .await
            .expect("Server should be running");

        // First get a valid genre slug
        let genres = client.list_genres().await.unwrap();
        let genre_slug = &genres.genres[0].slug;

        let profiles = client
            .get_profiles_for_genre(genre_slug)
            .await
            .expect("get_profiles should succeed");

        assert!(
            !profiles.profiles.is_empty(),
            "Should have at least one profile for genre '{genre_slug}'"
        );
    }
}
