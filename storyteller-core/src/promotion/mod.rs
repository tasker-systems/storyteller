//! Entity promotion logic — behavioral rules connecting events to entity lifecycle.
//!
//! Events create relationships, and relationships create entities worth tracking.
//! An entity earns its `EntityId` by participating in events that create or modify
//! relationships. This module implements the promotion/demotion logic, entity
//! reference resolution, and retroactive promotion via mention indexing.
//!
//! Design decision: Pure functions with a config struct — no trait abstraction.
//! The promotion logic is deterministic and has no need for runtime polymorphism.

pub mod mention_index;
pub mod resolution;
pub mod tier;
pub mod weight;

/// Configuration for entity promotion decisions.
///
/// All threshold values are initial guesses that need calibration from play data.
#[derive(Debug, Clone)]
pub struct PromotionConfig {
    /// Relational weight threshold for Referenced → Tracked.
    pub tracking_threshold: f32,
    /// Relational weight threshold for Tracked → Persistent.
    pub persistence_threshold: f32,
    /// Minimum distinct events for persistence consideration.
    pub min_persistence_events: u32,
    /// Scenes without events before considering demotion.
    pub demotion_scene_count: u32,
    /// Turns without events before considering scene-local demotion.
    pub demotion_turn_count: u32,
    /// Multiplier for player-interaction weight (player attention
    /// is a strong promotion signal).
    pub player_interaction_multiplier: f32,
}

impl Default for PromotionConfig {
    fn default() -> Self {
        Self {
            tracking_threshold: 0.5,
            persistence_threshold: 2.0,
            min_persistence_events: 3,
            demotion_scene_count: 3,
            demotion_turn_count: 10,
            player_interaction_multiplier: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotion_config_default_values() {
        let config = PromotionConfig::default();
        assert!((config.tracking_threshold - 0.5).abs() < f32::EPSILON);
        assert!((config.persistence_threshold - 2.0).abs() < f32::EPSILON);
        assert_eq!(config.min_persistence_events, 3);
        assert_eq!(config.demotion_scene_count, 3);
        assert_eq!(config.demotion_turn_count, 10);
        assert!((config.player_interaction_multiplier - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn promotion_config_custom_values() {
        let config = PromotionConfig {
            tracking_threshold: 1.0,
            persistence_threshold: 5.0,
            min_persistence_events: 5,
            demotion_scene_count: 5,
            demotion_turn_count: 20,
            player_interaction_multiplier: 3.0,
        };
        assert!((config.tracking_threshold - 1.0).abs() < f32::EPSILON);
        assert_eq!(config.min_persistence_events, 5);
    }
}
