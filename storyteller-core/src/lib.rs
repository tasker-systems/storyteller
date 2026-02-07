//! # storyteller-core
//!
//! Foundation crate for the storyteller engine. Contains all core types, shared
//! traits, error definitions, database operations, and graph queries.
//!
//! This crate has **no Bevy dependency** — it is the "headless" layer that can
//! be tested, inspected, and reused independently of the ECS runtime.
//!
//! ## Modules
//!
//! - [`types`] — Domain types: entities, tensors, events, scenes, relationships, narrative
//! - [`traits`] — Shared traits (e.g., `LlmProvider`, `EmotionalGrammar`)
//! - [`grammars`] — Emotional grammar implementations (e.g., `PlutchikWestern`)
//! - [`errors`] — `StorytellerError` and `StorytellerResult`
//! - [`config`] — Configuration loading and validation
//! - [`database`] — PostgreSQL operations (event ledger, checkpoints, sessions)
//! - [`graph`] — Apache AGE graph queries (relational web, narrative graph, settings)

pub mod config;
pub mod database;
pub mod errors;
pub mod grammars;
pub mod graph;
pub mod traits;
pub mod types;

pub use errors::{StorytellerError, StorytellerResult};
