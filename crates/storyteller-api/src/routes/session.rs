//! Session management endpoints.
//!
//! Create, resume, and manage player sessions. Sessions are the container
//! for an active playthrough of a story.
//!
//! See: `docs/technical/infrastructure-architecture.md` (session resilience)

use axum::{routing::post, Router};

use crate::state::AppState;

/// Session management routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/sessions", post(create_session))
}

/// Create a new play session.
async fn create_session() -> &'static str {
    // Placeholder — will initialize engine state and return session ID.
    "session created"
}
