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
//! - [`PostgresBedrock`] — sqlx queries against the `bedrock` schema
//!
//! See: `docs/technical/storykeeper-api-contract.md`
//! See: `docs/technical/postgresql-schema-design.md`

pub mod bedrock;
pub mod database;
pub mod in_memory;

pub use bedrock::PostgresBedrock;
pub use in_memory::InMemoryStorykeeper;
