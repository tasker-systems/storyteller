//! Bevy systems — the logic that runs each frame/turn.
//!
//! Systems implement the turn cycle, scene lifecycle, event processing,
//! entity promotion/demotion, and observability streaming.

pub mod entity_lifecycle;
pub mod event_pipeline;
pub mod observability;
pub mod scene_lifecycle;
pub mod turn_cycle;
