//! Shared application state for API handlers.
//!
//! `AppState` holds references to the engine and any shared resources that
//! handlers need. It is constructed by the deployment layer and passed into
//! [`crate::router()`].

/// Shared state available to all API handlers.
///
/// Constructed by the deployment crate (cli, shuttle, etc.) and threaded
/// through axum's state extraction.
#[derive(Debug, Clone)]
pub struct AppState {
    // Engine references, database pools, and configuration will be added here
    // as the implementation progresses. For now, this is a valid empty state.
    _private: (),
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
