//! Health check endpoint.
//!
//! Simple liveness probe usable by any deployment target (Kubernetes,
//! Shuttle, load balancers, etc.).

use axum::{routing::get, Router};

use crate::state::AppState;

/// Health check routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

/// Returns 200 OK if the service is alive.
async fn health_check() -> &'static str {
    "ok"
}
