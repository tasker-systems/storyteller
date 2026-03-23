// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! [`StorytellerClient`] — typed wrapper around generated tonic stubs.

use storyteller_core::types::health::ServerHealth;
use thiserror::Error;

/// Configuration for connecting to the storyteller server.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub endpoint: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50051".to_string(),
        }
    }
}

impl ClientConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("STORYTELLER_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:50051".to_string()),
        }
    }
}

/// Errors from the storyteller client.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to connect to server at {0}")]
    ConnectionFailed(String),

    #[error("RPC error: {0}")]
    RpcError(#[from] tonic::Status),

    #[error("Transport error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    #[error("Subsystem unavailable: {subsystem} - {message}")]
    SubsystemUnavailable { subsystem: String, message: String },
}

/// Typed gRPC client for the storyteller engine server.
///
/// Wraps both `StorytellerEngineClient` and `ComposerServiceClient` behind a
/// single API surface. Use [`StorytellerClient::connect`] to create a client.
#[derive(Debug)]
pub struct StorytellerClient {
    engine:
        crate::proto::storyteller_engine_client::StorytellerEngineClient<tonic::transport::Channel>,
    composer:
        crate::proto::composer_service_client::ComposerServiceClient<tonic::transport::Channel>,
}

impl StorytellerClient {
    /// Connect to the storyteller server at the configured endpoint.
    pub async fn connect(config: ClientConfig) -> Result<Self, ClientError> {
        let channel = tonic::transport::Channel::from_shared(config.endpoint.clone())
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?
            .connect()
            .await
            .map_err(|_| ClientError::ConnectionFailed(config.endpoint))?;

        Ok(Self {
            engine: crate::proto::storyteller_engine_client::StorytellerEngineClient::new(
                channel.clone(),
            ),
            composer: crate::proto::composer_service_client::ComposerServiceClient::new(channel),
        })
    }

    // -------------------------------------------------------------------------
    // Health
    // -------------------------------------------------------------------------

    /// Two-layer health check.
    ///
    /// Layer 1: Can we reach the gRPC server? (`ConnectionFailed` if not)
    /// Layer 2: What's the server's internal health? (`ServerHealth` with subsystems)
    pub async fn check_health(&mut self) -> Result<ServerHealth, ClientError> {
        let response = self.engine.check_health(()).await?.into_inner();

        let subsystems = response
            .subsystems
            .into_iter()
            .map(|s| storyteller_core::types::health::SubsystemHealth {
                name: s.name,
                status: storyteller_core::types::health::HealthStatus::from_str_lossy(&s.status),
                message: s.message,
            })
            .collect();

        Ok(ServerHealth::from_subsystems(subsystems))
    }

    // -------------------------------------------------------------------------
    // Engine RPCs
    // -------------------------------------------------------------------------

    pub async fn compose_scene(
        &mut self,
        request: crate::proto::ComposeSceneRequest,
    ) -> Result<tonic::Streaming<crate::proto::EngineEvent>, ClientError> {
        let response = self.engine.compose_scene(request).await?;
        Ok(response.into_inner())
    }

    pub async fn submit_input(
        &mut self,
        request: crate::proto::SubmitInputRequest,
    ) -> Result<tonic::Streaming<crate::proto::EngineEvent>, ClientError> {
        let response = self.engine.submit_input(request).await?;
        Ok(response.into_inner())
    }

    pub async fn resume_session(
        &mut self,
        request: crate::proto::ResumeSessionRequest,
    ) -> Result<tonic::Streaming<crate::proto::EngineEvent>, ClientError> {
        let response = self.engine.resume_session(request).await?;
        Ok(response.into_inner())
    }

    pub async fn list_sessions(&mut self) -> Result<crate::proto::SessionList, ClientError> {
        let response = self.engine.list_sessions(()).await?;
        Ok(response.into_inner())
    }

    pub async fn get_scene_state(
        &mut self,
        request: crate::proto::GetSceneStateRequest,
    ) -> Result<crate::proto::SceneState, ClientError> {
        let response = self.engine.get_scene_state(request).await?;
        Ok(response.into_inner())
    }

    pub async fn get_prediction_history(
        &mut self,
        session_id: &str,
        from_turn: Option<u32>,
        to_turn: Option<u32>,
    ) -> Result<crate::proto::PredictionHistoryResponse, ClientError> {
        let response = self
            .engine
            .get_prediction_history(crate::proto::PredictionHistoryRequest {
                session_id: session_id.to_string(),
                from_turn,
                to_turn,
            })
            .await?;
        Ok(response.into_inner())
    }

    /// Subscribe to a server-streaming log feed.
    ///
    /// Returns a `tonic::Streaming<LogEntry>` that yields log entries matching
    /// the optional `level` and `target` filters.
    pub async fn stream_logs(
        &mut self,
        level: Option<String>,
        target: Option<String>,
    ) -> Result<tonic::Streaming<crate::proto::LogEntry>, ClientError> {
        let response = self
            .engine
            .stream_logs(crate::proto::LogFilter { level, target })
            .await?;
        Ok(response.into_inner())
    }

    // -------------------------------------------------------------------------
    // Composer RPCs
    // -------------------------------------------------------------------------

    pub async fn list_genres(&mut self) -> Result<crate::proto::GenreList, ClientError> {
        let response = self.composer.list_genres(()).await?;
        Ok(response.into_inner())
    }

    pub async fn get_profiles_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::ProfileList, ClientError> {
        let response = self
            .composer
            .get_profiles_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_archetypes_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::ArchetypeList, ClientError> {
        let response = self
            .composer
            .get_archetypes_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_dynamics_for_genre(
        &mut self,
        genre_id: &str,
        selected_archetype_ids: Vec<String>,
    ) -> Result<crate::proto::DynamicsList, ClientError> {
        let response = self
            .composer
            .get_dynamics_for_genre(crate::proto::DynamicsRequest {
                genre_id: genre_id.to_string(),
                selected_archetype_ids,
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_names_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::NameList, ClientError> {
        let response = self
            .composer
            .get_names_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_settings_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::SettingList, ClientError> {
        let response = self
            .composer
            .get_settings_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_genre_options(
        &mut self,
        genre_id: &str,
        selected_archetype_ids: Vec<String>,
    ) -> Result<crate::proto::GenreOptions, ClientError> {
        let response = self
            .composer
            .get_genre_options(crate::proto::GenreOptionsRequest {
                genre_id: genre_id.to_string(),
                selected_archetype_ids,
            })
            .await?;
        Ok(response.into_inner())
    }
}

// -----------------------------------------------------------------------------
// Unit tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_localhost() {
        let config = ClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50051");
    }

    #[test]
    fn config_from_env_uses_default_when_unset() {
        std::env::remove_var("STORYTELLER_SERVER_URL");
        let config = ClientConfig::from_env();
        assert_eq!(config.endpoint, "http://localhost:50051");
    }

    #[test]
    fn client_error_display() {
        let err = ClientError::ConnectionFailed("http://localhost:50051".to_string());
        assert!(err.to_string().contains("localhost:50051"));

        let err = ClientError::SubsystemUnavailable {
            subsystem: "narrator".to_string(),
            message: "not configured".to_string(),
        };
        assert!(err.to_string().contains("narrator"));
    }

    #[tokio::test]
    async fn connect_fails_with_unreachable_server() {
        // Port 1 is reserved/unreachable on macOS
        let result = StorytellerClient::connect(ClientConfig {
            endpoint: "http://127.0.0.1:1".to_string(),
        })
        .await;
        assert!(result.is_err());
    }
}
