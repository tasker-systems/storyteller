//! # storyteller-api
//!
//! Player-facing HTTP/WebSocket API for the storyteller engine.
//!
//! This crate is **deployment-agnostic** — it produces an [`axum::Router`] that
//! any hosting layer can mount. Both `storyteller-cli` (self-hosted) and a
//! future `storyteller-shuttle` (Shuttle.dev) crate depend on this.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let router = storyteller_api::router(app_state);
//! // Mount `router` in your deployment target's HTTP server.
//! ```
//!
//! ## Modules
//!
//! - [`routes`] — Route definitions (player input, session management, health)
//! - [`middleware`] — Auth, rate limiting, and other middleware
//! - [`state`] — Shared application state passed to handlers

pub mod middleware;
pub mod routes;
pub mod state;

use axum::Router;

/// Build the player-facing API router.
///
/// The returned router is deployment-agnostic — mount it on any axum-compatible
/// server. The `state` parameter carries engine references and configuration.
pub fn router(state: state::AppState) -> Router {
    Router::new()
        .merge(routes::player::routes())
        .merge(routes::session::routes())
        .merge(routes::health::routes())
        .with_state(state)
}
