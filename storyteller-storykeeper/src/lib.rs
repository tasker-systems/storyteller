//! Storyteller Storykeeper — persistence layer for the storytelling engine.
//!
//! This crate owns all database access: migrations, the `MIGRATOR` static,
//! and (eventually) the `StorykeeperQuery`, `StorykeeperCommit`, and
//! `StorykeeperLifecycle` trait implementations backed by PostgreSQL.
//!
//! See: `docs/technical/storykeeper-api-contract.md`
//! See: `docs/technical/postgresql-schema-design.md`

pub mod database;
