//! Player input/output endpoints.
//!
//! These routes handle the core gameplay loop: accepting player input,
//! streaming narrative responses, and providing turn-cycle progress events.
//!
//! See: `docs/technical/infrastructure-architecture.md` (player-facing protocol),
//!      `docs/technical/scene-model.md` (ludic contract)

use axum::{routing::post, Router};

use crate::state::AppState;

/// Player interaction routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/input", post(submit_input))
}

/// Accept player input for the current scene.
async fn submit_input() -> &'static str {
    // Placeholder — will dispatch to the turn cycle pipeline.
    "input received"
}
