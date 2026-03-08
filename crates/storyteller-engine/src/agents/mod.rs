//! Agent implementations.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! In the narrator-centric architecture, the Narrator is the ONLY LLM agent.
//! Character behavior is predicted by ML models, and world constraints are
//! enforced by a deterministic rules engine (Resolver). Agents are Bevy
//! systems — lightweight functions, not microservices.

pub mod classifier;
pub mod narrator;
pub mod world;

pub use narrator::NarratorAgent;
