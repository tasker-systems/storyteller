//! Persistence profile — how an entity survives scene boundaries.
//!
//! See: `docs/technical/entity-model.md`

use bevy_ecs::prelude::*;
use storyteller_core::types::entity::PersistenceMode;

/// Controls how an entity persists across scene transitions.
#[derive(Debug, Component)]
pub struct PersistenceProfile {
    /// Whether this entity is permanent, scene-local, or ephemeral.
    pub mode: PersistenceMode,
}
