//! # storyteller-engine
//!
//! Bevy ECS runtime for the storyteller engine. Contains all components, systems,
//! agent implementations, ML inference integration, and messaging.
//!
//! This crate owns the Bevy `App` and `Plugin` infrastructure. All agents run
//! as in-process Bevy systems — not microservices.
//!
//! ## Modules
//!
//! - [`plugin`] — `StorytellerEnginePlugin` (Bevy plugin registration)
//! - [`components`] — Bevy ECS components (identity, communicability, tensors, scene state)
//! - [`systems`] — Bevy systems (turn cycle, scene lifecycle, event pipeline)
//! - [`agents`] — Agent implementations (Narrator, Storykeeper, Character, World, Reconciler)
//! - [`inference`] — ML inference integration (frame computation, LLM providers)
//! - [`messaging`] — RabbitMQ integration for tasker-core workflow dispatch

pub mod agents;
pub mod components;
pub mod inference;
pub mod messaging;
pub mod plugin;
pub mod systems;
pub mod workshop;

pub use plugin::StorytellerEnginePlugin;
