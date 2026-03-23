// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! PostgreSQL database operations.
//!
//! See: `docs/technical/infrastructure-architecture.md`
//!
//! Handles the event ledger, checkpoint read/write, and session state management.
//! All database access flows through sqlx with the PostgreSQL driver.

pub mod checkpoint;
pub mod ledger;
pub mod session;
