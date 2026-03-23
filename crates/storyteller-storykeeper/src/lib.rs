// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Storyteller Storykeeper — persistence layer for the storytelling engine.
//!
//! This crate owns all database access: migrations, the `MIGRATOR` static,
//! and Storykeeper trait implementations.
//!
//! Two implementations:
//! - [`InMemoryStorykeeper`] — in-process state for tests and prototype
//! - `PostgresStorykeeper` — sqlx queries against the real schema (future)
//!
//! See: `docs/technical/storykeeper-api-contract.md`
//! See: `docs/technical/postgresql-schema-design.md`

pub mod database;
pub mod in_memory;

pub use in_memory::InMemoryStorykeeper;
