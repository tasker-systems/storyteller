//! Communicability profile — how an entity can participate in narrative.
//!
//! See: `docs/foundation/world_design.md` (communicability gradient),
//!      `docs/technical/entity-model.md`
//!
//! Entities exist on a spectrum from full characters to minimal props.
//! These four dimensions determine where an entity falls on that spectrum.

use bevy_ecs::prelude::*;

/// Communicability gradient — determines narrative participation level.
#[derive(Debug, Component)]
pub struct CommunicabilityProfile {
    /// How much narrative surface area this entity has. Range: [0.0, 1.0].
    /// A full character is ~1.0; a prop is ~0.0.
    pub surface_area: f32,

    /// How hard it is to translate this entity's "voice" into narrative.
    /// A person speaking is low friction; a river's "intention" is high friction.
    pub translation_friction: f32,

    /// The timescale at which this entity operates.
    /// A human operates in minutes/hours; a mountain in millennia.
    pub timescale: f32,

    /// Capacity to "turn toward" another entity (Buber's I/Thou).
    /// A character can fully turn toward; a rock has minimal capacity.
    pub turn_toward_capacity: f32,
}
