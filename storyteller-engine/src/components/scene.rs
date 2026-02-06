//! Scene state components and resources.
//!
//! See: `docs/technical/scene-model.md`
//!
//! Scene state is held as Bevy resources (singleton data) rather than
//! components on entities, since there is exactly one active scene at a time.

use bevy_ecs::prelude::*;
use storyteller_core::types::scene::{SceneId, SceneType};

/// Resource tracking the currently active scene.
#[derive(Debug, Resource)]
pub struct ActiveScene {
    /// Identifier of the current scene.
    pub id: SceneId,
    /// What kind of scene this is.
    pub scene_type: SceneType,
}
