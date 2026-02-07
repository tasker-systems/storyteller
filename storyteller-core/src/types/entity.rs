//! Entity types — the unified representation for all story elements.
//!
//! See: `docs/technical/entity-model.md`
//!
//! Design decision: Everything is an Entity. Characters, presences, conditions,
//! and props share a single type with component configuration determining
//! their capabilities. Promotion and demotion between tiers is a lifecycle
//! operation, not a type change.

use uuid::Uuid;

/// Unique identifier for an entity within a story session.
///
/// Uses UUID v7 (time-ordered) for efficient BTree indexing both in-process
/// and in PostgreSQL. Temporal ordering means IDs sort by creation time.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new time-ordered entity ID (UUID v7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

/// How an entity came into existence in the story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntityOrigin {
    /// Authored by the story designer before play begins.
    Authored,
    /// Promoted from a lower tier during play (e.g., prop → presence).
    Promoted,
    /// Generated procedurally by the engine.
    Generated,
}

/// How an entity persists across scene boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PersistenceMode {
    /// Persists across all scenes (main characters, key locations).
    Permanent,
    /// Persists within the current scene only.
    SceneLocal,
    /// Created on demand, not persisted.
    Ephemeral,
}
