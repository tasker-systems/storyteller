//! Promotion tier determination — deciding where an entity sits in the lifecycle.
//!
//! Entities progress through tiers based on relational weight:
//! `Unmentioned → Mentioned → Referenced → Tracked → Persistent`.
//!
//! Promotion is monotonic within `determine_promotion_tier` — the function
//! never returns a tier lower than the current one. Demotion is handled
//! separately by `evaluate_demotion`, which checks for sustained inactivity.

use crate::types::entity::{PromotionTier, RelationalWeight};

use super::PromotionConfig;

/// Determine the promotion tier based on accumulated relational weight.
///
/// Respects authored floors — an entity authored at a higher tier is never
/// demoted below that tier. The returned tier is always >= `current_tier`.
pub fn determine_promotion_tier(
    weight: &RelationalWeight,
    current_tier: PromotionTier,
    authored_floor: Option<PromotionTier>,
    config: &PromotionConfig,
) -> PromotionTier {
    let floor = authored_floor.unwrap_or(PromotionTier::Unmentioned);

    let computed = if weight.total_weight >= config.persistence_threshold
        && weight.event_count >= config.min_persistence_events
    {
        PromotionTier::Persistent
    } else if weight.total_weight >= config.tracking_threshold {
        PromotionTier::Tracked
    } else if weight.total_weight > 0.0 {
        PromotionTier::Referenced
    } else if weight.event_count > 0 {
        PromotionTier::Mentioned
    } else {
        PromotionTier::Unmentioned
    };

    // Never below authored floor, never below current tier (promotion is monotonic)
    computed.max(floor).max(current_tier)
}

/// Evaluate whether an entity should be demoted based on inactivity.
///
/// Demotion requires sustained absence from events. Returns the new tier
/// if demotion is warranted, or `None` if the entity retains its current tier.
///
/// Demotion never goes below `authored_floor` or below `Referenced`.
pub fn evaluate_demotion(
    current_tier: PromotionTier,
    authored_floor: Option<PromotionTier>,
    scenes_without_events: u32,
    turns_without_events: u32,
    config: &PromotionConfig,
) -> Option<PromotionTier> {
    let floor = authored_floor.unwrap_or(PromotionTier::Unmentioned);

    let demoted = match current_tier {
        PromotionTier::Persistent if scenes_without_events >= config.demotion_scene_count => {
            Some(PromotionTier::Tracked)
        }
        PromotionTier::Tracked if turns_without_events >= config.demotion_turn_count => {
            Some(PromotionTier::Referenced)
        }
        // Below Referenced: no automatic demotion
        _ => None,
    };

    // Enforce authored floor
    demoted.map(|tier| tier.max(floor))
}

#[cfg(test)]
mod tests {
    use crate::types::entity::EntityRef;

    use super::*;

    fn make_config() -> PromotionConfig {
        PromotionConfig::default()
    }

    fn make_weight(total: f32, events: u32, relationships: u32) -> RelationalWeight {
        RelationalWeight {
            entity: EntityRef::Implicit {
                implied_entity: "test".to_string(),
                implication_source: "test".to_string(),
            },
            total_weight: total,
            event_count: events,
            relationship_count: relationships,
            player_interaction_weight: 0.0,
        }
    }

    // -----------------------------------------------------------------------
    // determine_promotion_tier
    // -----------------------------------------------------------------------

    #[test]
    fn zero_weight_stays_unmentioned() {
        let weight = make_weight(0.0, 0, 0);
        let tier =
            determine_promotion_tier(&weight, PromotionTier::Unmentioned, None, &make_config());
        assert_eq!(tier, PromotionTier::Unmentioned);
    }

    #[test]
    fn event_participation_promotes_to_mentioned() {
        let weight = make_weight(0.0, 1, 0);
        let tier =
            determine_promotion_tier(&weight, PromotionTier::Unmentioned, None, &make_config());
        assert_eq!(tier, PromotionTier::Mentioned);
    }

    #[test]
    fn weight_from_implications_promotes_to_referenced() {
        let weight = make_weight(0.3, 1, 1);
        let tier =
            determine_promotion_tier(&weight, PromotionTier::Unmentioned, None, &make_config());
        assert_eq!(tier, PromotionTier::Referenced);
    }

    #[test]
    fn threshold_crossing_promotes_to_tracked() {
        let config = make_config();
        let weight = make_weight(config.tracking_threshold, 2, 1);
        let tier = determine_promotion_tier(&weight, PromotionTier::Unmentioned, None, &config);
        assert_eq!(tier, PromotionTier::Tracked);
    }

    #[test]
    fn high_weight_and_events_promotes_to_persistent() {
        let config = make_config();
        let weight = make_weight(
            config.persistence_threshold,
            config.min_persistence_events,
            2,
        );
        let tier = determine_promotion_tier(&weight, PromotionTier::Unmentioned, None, &config);
        assert_eq!(tier, PromotionTier::Persistent);
    }

    #[test]
    fn high_weight_insufficient_events_stays_tracked() {
        let config = make_config();
        // Weight exceeds persistence threshold but not enough events
        let weight = make_weight(
            config.persistence_threshold,
            config.min_persistence_events - 1,
            2,
        );
        let tier = determine_promotion_tier(&weight, PromotionTier::Unmentioned, None, &config);
        assert_eq!(tier, PromotionTier::Tracked);
    }

    #[test]
    fn authored_floor_prevents_demotion() {
        let weight = make_weight(0.0, 0, 0);
        let tier = determine_promotion_tier(
            &weight,
            PromotionTier::Unmentioned,
            Some(PromotionTier::Tracked),
            &make_config(),
        );
        assert_eq!(tier, PromotionTier::Tracked);
    }

    #[test]
    fn current_tier_prevents_regression() {
        // Already Tracked, weight only enough for Referenced — stays Tracked
        let weight = make_weight(0.3, 1, 1);
        let tier = determine_promotion_tier(&weight, PromotionTier::Tracked, None, &make_config());
        assert_eq!(tier, PromotionTier::Tracked);
    }

    // -----------------------------------------------------------------------
    // evaluate_demotion
    // -----------------------------------------------------------------------

    #[test]
    fn demotion_after_scene_inactivity() {
        let config = make_config();
        let result = evaluate_demotion(
            PromotionTier::Persistent,
            None,
            config.demotion_scene_count,
            0,
            &config,
        );
        assert_eq!(result, Some(PromotionTier::Tracked));
    }

    #[test]
    fn demotion_after_turn_inactivity() {
        let config = make_config();
        let result = evaluate_demotion(
            PromotionTier::Tracked,
            None,
            0,
            config.demotion_turn_count,
            &config,
        );
        assert_eq!(result, Some(PromotionTier::Referenced));
    }

    #[test]
    fn no_demotion_below_referenced() {
        let config = make_config();
        let result = evaluate_demotion(PromotionTier::Referenced, None, 100, 100, &config);
        assert_eq!(result, None);
    }

    #[test]
    fn authored_floor_prevents_demotion_in_evaluate() {
        let config = make_config();
        // Entity is Persistent, authored at Persistent — should not demote
        let result = evaluate_demotion(
            PromotionTier::Persistent,
            Some(PromotionTier::Persistent),
            config.demotion_scene_count,
            0,
            &config,
        );
        assert_eq!(result, Some(PromotionTier::Persistent));
    }

    #[test]
    fn insufficient_inactivity_no_demotion() {
        let config = make_config();
        let result = evaluate_demotion(
            PromotionTier::Persistent,
            None,
            config.demotion_scene_count - 1,
            0,
            &config,
        );
        assert_eq!(result, None);
    }
}
