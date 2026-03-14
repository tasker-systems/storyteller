//! Server-side engine state types.
//!
//! Uses `serde_json::Value` for dynamic fields while the turn pipeline matures.
//! As integration deepens, these will be replaced with strongly-typed domain
//! objects from `storyteller-core`.

/// Immutable composition data — created once per session.
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
#[derive(Debug, Clone, Default)]
pub struct RuntimeSnapshot {
    pub journal_entries: Vec<String>,
    pub turn_count: u32,
    pub player_entity_id: Option<String>,
    pub prediction_history: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_snapshot_default_is_empty() {
        let snap = RuntimeSnapshot::default();
        assert_eq!(snap.turn_count, 0);
        assert!(snap.journal_entries.is_empty());
        assert!(snap.player_entity_id.is_none());
        assert!(snap.prediction_history.is_empty());
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
