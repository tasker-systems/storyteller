//! Shared application state for API handlers.
//!
//! `AppState` holds references to the engine and any shared resources that
//! handlers need. It is constructed by the deployment layer and passed into
//! [`crate::router()`].

use std::sync::Arc;

use storyteller_composer::SceneComposer;

use crate::engine::{EngineProviders, EngineStateManager};
use crate::persistence::SessionStore;

/// Shared state available to all API handlers and gRPC services.
///
/// Constructed by the deployment crate (cli, shuttle, etc.) and threaded
/// through axum's state extraction.
#[derive(Debug, Clone)]
pub struct AppState {
    pub composer: Arc<SceneComposer>,
    pub state_manager: Arc<EngineStateManager>,
    pub session_store: Arc<SessionStore>,
    pub providers: Arc<EngineProviders>,
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        composer: Arc<SceneComposer>,
        state_manager: Arc<EngineStateManager>,
        session_store: Arc<SessionStore>,
        providers: Arc<EngineProviders>,
    ) -> Self {
        Self {
            composer,
            state_manager,
            session_store,
            providers,
        }
    }
}
