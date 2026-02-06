//! Middleware for the player-facing API.
//!
//! Auth, rate limiting, and other cross-cutting concerns. These are
//! deployment-agnostic — the deployment crate decides whether to apply
//! them (e.g., Shuttle may handle auth at the platform level).

pub mod auth;
