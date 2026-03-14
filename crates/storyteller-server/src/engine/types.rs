//! Server-side engine state types.
//!
//! `Composition` uses `serde_json::Value` for scene/character data to avoid
//! coupling to domain types at the state layer — deserialized on demand by the
//! pipeline. `RuntimeSnapshot` uses typed domain objects so the pipeline can
//! work directly without per-turn deserialization overhead.

use storyteller_core::types::entity::EntityId;
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_core::types::scene::SceneId;
use storyteller_ml::prediction_history::PredictionHistory;

/// Immutable composition data — created once per session.
///
/// Uses `serde_json::Value` for scene/character/goals/intentions while the
/// composition API stabilises. Deserialized to domain types by the pipeline.
#[derive(Debug, Clone)]
pub struct Composition {
    pub scene: serde_json::Value,
    pub characters: Vec<serde_json::Value>,
    pub goals: Option<serde_json::Value>,
    pub intentions: Option<serde_json::Value>,
    pub selections: serde_json::Value,
}

/// Mutable runtime state — published as snapshots via ArcSwap.
///
/// Readers get a cheap `Arc` clone; writers publish a new `Arc` at each
/// pipeline phase boundary. No reader ever blocks a writer.
#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    /// Current turn number (0 = opening, incremented each player turn).
    pub turn_count: u32,
    /// Entity ID of the player's character in this scene, if any.
    pub player_entity_id: Option<EntityId>,
    /// Progressive scene journal — carries compressed turn history for the narrator.
    pub journal: SceneJournal,
    /// Per-character prediction history — ring buffer for ML feature Region 7.
    pub prediction_history: PredictionHistory,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            turn_count: 0,
            player_entity_id: None,
            journal: SceneJournal::new(SceneId::default(), 1200),
            prediction_history: PredictionHistory::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_snapshot_default_is_empty() {
        let snap = RuntimeSnapshot::default();
        assert_eq!(snap.turn_count, 0);
        assert!(snap.journal.entries.is_empty());
        assert!(snap.player_entity_id.is_none());
        assert!(snap.prediction_history.as_map().is_empty());
    }

    #[test]
    fn composition_clone_is_independent() {
        let comp = Composition {
            scene: serde_json::json!({"title": "test"}),
            characters: vec![serde_json::json!({"name": "Alice"})],
            goals: None,
            intentions: None,
            selections: serde_json::json!({}),
        };
        let cloned = comp.clone();
        assert_eq!(comp.scene, cloned.scene);
        assert_eq!(comp.characters.len(), cloned.characters.len());
    }
}
