// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

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
