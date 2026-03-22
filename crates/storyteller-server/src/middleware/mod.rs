// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Middleware for the player-facing API.
//!
//! Auth, rate limiting, and other cross-cutting concerns. These are
//! deployment-agnostic — the deployment crate decides whether to apply
//! them (e.g., Shuttle may handle auth at the platform level).

pub mod auth;
