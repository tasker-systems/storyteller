//! Scene types — the fundamental unit of play.
//!
//! See: `docs/technical/scene-model.md`
//!
//! Design decision: The scene is the bounded unit that provides cast, setting,
//! stakes, entity budget, graph position, and warmed data. Scene types determine
//! narrative mass and gravitational behavior.

use uuid::Uuid;

/// Unique identifier for a scene in the narrative graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SceneId(pub Uuid);

impl SceneId {
    /// Create a new random scene ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

/// Fundamental scene classification determining mass and gravitational behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SceneType {
    /// High narrative mass — pivotal moments the story bends toward.
    Gravitational,
    /// Medium mass — travel, exploration, texture. World Agent + Narrator carry these.
    Connective,
    /// Conditional passage — requirements must be met to proceed.
    Gate,
    /// Transformation boundary — the character who enters is not the one who leaves.
    Threshold,
}

/// How a player departs a scene — affects narrative continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DepartureType {
    /// Natural narrative conclusion — the scene has played out.
    Completed,
    /// Player-initiated exit before completion.
    Abandoned,
    /// Scene interrupted by external narrative event.
    Interrupted,
    /// Transition to a connected scene via the narrative graph.
    Traversed,
}
