//! Agent system implementations.
//!
//! See: `docs/technical/agent-message-catalog.md`, `docs/foundation/system_architecture.md`
//!
//! Agents are Bevy systems — lightweight functions that query ECS state,
//! call LLM providers, and emit events. They are NOT microservices.
//! Each agent has a specific role, perspective, and information boundary.

pub mod character;
pub mod classifier;
pub mod narrator;
pub mod reconciler;
pub mod storykeeper;
pub mod world;
