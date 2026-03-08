//! Relational weight computation — how much relational significance an entity has accumulated.
//!
//! Weight is computed from the events an entity participates in. Each relational
//! implication in an event contributes to the entity's weight. Player-initiated
//! events receive a configurable multiplier because player attention is a strong
//! promotion signal.

use std::collections::BTreeSet;

use crate::types::entity::{EntityId, EntityRef, RelationalWeight};
use crate::types::event_grammar::{EventAtom, EventSource, Participant};

use super::PromotionConfig;

/// Normalize a mention string for comparison: lowercase, trim whitespace,
/// strip leading articles ("the", "a", "an").
pub fn normalize_mention(mention: &str) -> String {
    let trimmed = mention.trim().to_lowercase();
    for article in &["the ", "a ", "an "] {
        if let Some(rest) = trimmed.strip_prefix(article) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    trimmed
}

/// Determine whether two `EntityRef` values refer to the same entity.
///
/// - Resolved vs Resolved: compare `EntityId`.
/// - Unresolved vs Unresolved: compare normalized mention text.
/// - Implicit vs Implicit: compare normalized `implied_entity` text.
/// - Mixed variants: no match (resolution hasn't happened yet).
pub fn entity_matches(needle: &EntityRef, haystack: &EntityRef) -> bool {
    match (needle, haystack) {
        (EntityRef::Resolved(a), EntityRef::Resolved(b)) => a == b,
        (EntityRef::Unresolved { mention: a, .. }, EntityRef::Unresolved { mention: b, .. }) => {
            normalize_mention(a) == normalize_mention(b)
        }
        (
            EntityRef::Implicit {
                implied_entity: a, ..
            },
            EntityRef::Implicit {
                implied_entity: b, ..
            },
        ) => normalize_mention(a) == normalize_mention(b),
        _ => false,
    }
}

/// Returns true if the given event was initiated by the player.
fn is_player_event(event: &EventAtom) -> bool {
    matches!(event.source, EventSource::PlayerInput { .. })
}

/// Returns true if the given entity participates in the event as a participant.
fn entity_is_participant(entity: &EntityRef, event: &EventAtom) -> bool {
    event
        .participants
        .iter()
        .any(|p| entity_matches(entity, &p.entity))
}

/// Returns true if the given entity appears in any relational implication
/// of the event (as source or target).
fn entity_in_implications(entity: &EntityRef, event: &EventAtom) -> bool {
    event
        .relational_implications
        .iter()
        .any(|imp| entity_matches(entity, &imp.source) || entity_matches(entity, &imp.target))
}

/// Collect entity IDs of other entities in relational implications involving `entity`.
///
/// Returns mention strings for unresolved entities and UUIDs for resolved ones,
/// deduplicated. Used to count distinct relationship partners.
fn collect_relationship_partners(entity: &EntityRef, events: &[EventAtom]) -> BTreeSet<String> {
    let mut partners = BTreeSet::new();
    for event in events {
        for imp in &event.relational_implications {
            if entity_matches(entity, &imp.source) {
                partners.insert(entity_ref_key(&imp.target));
            }
            if entity_matches(entity, &imp.target) {
                partners.insert(entity_ref_key(&imp.source));
            }
        }
    }
    partners
}

/// Produce a deduplication key for an EntityRef.
fn entity_ref_key(entity_ref: &EntityRef) -> String {
    match entity_ref {
        EntityRef::Resolved(id) => id.0.to_string(),
        EntityRef::Unresolved { mention, .. } => {
            format!("unresolved:{}", normalize_mention(mention))
        }
        EntityRef::Implicit { implied_entity, .. } => {
            format!("implicit:{}", normalize_mention(implied_entity))
        }
    }
}

/// Compute the relational weight of an entity from the events it participates in.
///
/// The weight is the sum of relational implication weights where this entity
/// appears as source or target. Player-initiated events receive a multiplier.
///
/// The `player_entity_id` identifies the player character so that events
/// involving the player can be weighted more heavily.
pub fn compute_relational_weight(
    entity: &EntityRef,
    events: &[EventAtom],
    player_entity_id: EntityId,
    config: &PromotionConfig,
) -> RelationalWeight {
    let player_ref = EntityRef::Resolved(player_entity_id);

    let mut total_weight: f32 = 0.0;
    let mut player_interaction_weight: f32 = 0.0;
    let mut event_count: u32 = 0;

    // Track which events we've already counted (by EventId)
    let mut counted_events = BTreeSet::new();

    for event in events {
        // Check if this entity is involved in this event at all
        let in_participants = entity_is_participant(entity, event);
        let in_implications = entity_in_implications(entity, event);

        if !in_participants && !in_implications {
            continue;
        }

        // Count distinct events (deduplicate by EventId)
        if !counted_events.insert(event.id) {
            continue;
        }

        event_count += 1;

        // Sum implication weights where this entity is source or target
        let mut event_weight: f32 = 0.0;
        for imp in &event.relational_implications {
            if entity_matches(entity, &imp.source) || entity_matches(entity, &imp.target) {
                event_weight += imp.weight;
            }
        }

        // Check if player is also a participant in this event
        let player_involved = event
            .participants
            .iter()
            .any(|p: &Participant| entity_matches(&player_ref, &p.entity))
            || is_player_event(event);

        if player_involved {
            player_interaction_weight += event_weight * config.player_interaction_multiplier;
        }

        total_weight += event_weight;
    }

    let relationship_count = collect_relationship_partners(entity, events).len() as u32;

    RelationalWeight {
        entity: entity.clone(),
        total_weight: total_weight + player_interaction_weight,
        event_count,
        relationship_count,
        player_interaction_weight,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::types::entity::ReferentialContext;
    use crate::types::event::{EventId, EventPriority, TurnId};
    use crate::types::event_grammar::*;
    use crate::types::prediction::ActionType;
    use crate::types::scene::SceneId;

    use super::*;

    fn make_config() -> PromotionConfig {
        PromotionConfig::default()
    }

    fn make_resolved_ref(id: EntityId) -> EntityRef {
        EntityRef::Resolved(id)
    }

    fn make_unresolved_ref(mention: &str) -> EntityRef {
        EntityRef::Unresolved {
            mention: mention.to_string(),
            context: ReferentialContext {
                descriptors: vec![],
                spatial_context: None,
                possessor: None,
                prior_mentions: vec![],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
        }
    }

    fn make_atom(
        participants: Vec<Participant>,
        implications: Vec<RelationalImplication>,
        source: EventSource,
    ) -> EventAtom {
        EventAtom {
            id: EventId::new(),
            timestamp: Utc::now(),
            kind: EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            participants,
            relational_implications: implications,
            source,
            confidence: EventConfidence {
                value: 0.8,
                evidence: ConfidenceEvidence::SystemProduced,
            },
            priority: EventPriority::Normal,
            scene_id: SceneId::new(),
            turn_id: Some(TurnId::new()),
        }
    }

    fn system_source() -> EventSource {
        EventSource::System {
            component: "test".to_string(),
        }
    }

    fn player_source() -> EventSource {
        EventSource::PlayerInput {
            raw_input: "test".to_string(),
            classifier: ClassifierRef {
                name: "test".to_string(),
                version: "0.1".to_string(),
            },
        }
    }

    // -----------------------------------------------------------------------
    // normalize_mention
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_strips_articles_and_lowercases() {
        assert_eq!(normalize_mention("The Cup"), "cup");
        assert_eq!(normalize_mention("a stone"), "stone");
        assert_eq!(normalize_mention("An Old Tree"), "old tree");
        assert_eq!(normalize_mention("  flowers  "), "flowers");
    }

    #[test]
    fn normalize_preserves_non_article_text() {
        assert_eq!(normalize_mention("Sarah"), "sarah");
        assert_eq!(normalize_mention("there"), "there");
    }

    // -----------------------------------------------------------------------
    // entity_matches
    // -----------------------------------------------------------------------

    #[test]
    fn entity_matches_resolved_same_id() {
        let id = EntityId::new();
        assert!(entity_matches(
            &EntityRef::Resolved(id),
            &EntityRef::Resolved(id)
        ));
    }

    #[test]
    fn entity_matches_resolved_different_id() {
        assert!(!entity_matches(
            &EntityRef::Resolved(EntityId::new()),
            &EntityRef::Resolved(EntityId::new())
        ));
    }

    #[test]
    fn entity_matches_unresolved_same_mention() {
        let a = make_unresolved_ref("the Cup");
        let b = make_unresolved_ref("cup");
        assert!(entity_matches(&a, &b));
    }

    #[test]
    fn entity_matches_mixed_variants_no_match() {
        let resolved = EntityRef::Resolved(EntityId::new());
        let unresolved = make_unresolved_ref("cup");
        assert!(!entity_matches(&resolved, &unresolved));
        assert!(!entity_matches(&unresolved, &resolved));
    }

    // -----------------------------------------------------------------------
    // compute_relational_weight
    // -----------------------------------------------------------------------

    #[test]
    fn zero_events_zero_weight() {
        let entity = make_resolved_ref(EntityId::new());
        let weight = compute_relational_weight(&entity, &[], EntityId::new(), &make_config());
        assert!((weight.total_weight).abs() < f32::EPSILON);
        assert_eq!(weight.event_count, 0);
        assert_eq!(weight.relationship_count, 0);
    }

    #[test]
    fn single_event_one_implication() {
        let entity_id = EntityId::new();
        let other_id = EntityId::new();
        let entity = make_resolved_ref(entity_id);

        let atom = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Actor,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(entity_id),
                target: EntityRef::Resolved(other_id),
                implication_type: ImplicationType::Attention,
                weight: 0.3,
            }],
            system_source(),
        );

        let weight = compute_relational_weight(
            &entity,
            &[atom],
            EntityId::new(), // different from entity_id — not player
            &make_config(),
        );
        assert!((weight.total_weight - 0.3).abs() < f32::EPSILON);
        assert_eq!(weight.event_count, 1);
        assert_eq!(weight.relationship_count, 1);
    }

    #[test]
    fn multiple_events_accumulate_weight() {
        let entity_id = EntityId::new();
        let other_id = EntityId::new();
        let entity = make_resolved_ref(entity_id);

        let atom1 = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Actor,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(entity_id),
                target: EntityRef::Resolved(other_id),
                implication_type: ImplicationType::Attention,
                weight: 0.3,
            }],
            system_source(),
        );

        let atom2 = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Target,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(other_id),
                target: EntityRef::Resolved(entity_id),
                implication_type: ImplicationType::Care,
                weight: 0.5,
            }],
            system_source(),
        );

        let weight =
            compute_relational_weight(&entity, &[atom1, atom2], EntityId::new(), &make_config());
        assert!((weight.total_weight - 0.8).abs() < f32::EPSILON);
        assert_eq!(weight.event_count, 2);
    }

    #[test]
    fn player_involved_events_have_multiplied_weight() {
        let entity_id = EntityId::new();
        let player_id = EntityId::new();
        let entity = make_resolved_ref(entity_id);

        let atom = make_atom(
            vec![
                Participant {
                    entity: EntityRef::Resolved(entity_id),
                    role: ParticipantRole::Target,
                },
                Participant {
                    entity: EntityRef::Resolved(player_id),
                    role: ParticipantRole::Actor,
                },
            ],
            vec![RelationalImplication {
                source: EntityRef::Resolved(player_id),
                target: EntityRef::Resolved(entity_id),
                implication_type: ImplicationType::Attention,
                weight: 0.4,
            }],
            player_source(),
        );

        let config = make_config();
        let weight = compute_relational_weight(&entity, &[atom], player_id, &config);

        // Base weight 0.4 + player interaction weight 0.4 * 2.0 = 0.4 + 0.8 = 1.2
        assert!((weight.total_weight - 1.2).abs() < f32::EPSILON);
        assert!((weight.player_interaction_weight - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn entity_as_source_and_target_both_count() {
        let entity_id = EntityId::new();
        let other_id = EntityId::new();
        let entity = make_resolved_ref(entity_id);

        let atom = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Actor,
            }],
            vec![
                RelationalImplication {
                    source: EntityRef::Resolved(entity_id),
                    target: EntityRef::Resolved(other_id),
                    implication_type: ImplicationType::Attention,
                    weight: 0.2,
                },
                RelationalImplication {
                    source: EntityRef::Resolved(other_id),
                    target: EntityRef::Resolved(entity_id),
                    implication_type: ImplicationType::Care,
                    weight: 0.3,
                },
            ],
            system_source(),
        );

        let weight = compute_relational_weight(&entity, &[atom], EntityId::new(), &make_config());
        assert!((weight.total_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn distinct_relationship_partner_counting() {
        let entity_id = EntityId::new();
        let other1 = EntityId::new();
        let other2 = EntityId::new();
        let entity = make_resolved_ref(entity_id);

        let atom1 = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Actor,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(entity_id),
                target: EntityRef::Resolved(other1),
                implication_type: ImplicationType::Attention,
                weight: 0.2,
            }],
            system_source(),
        );

        let atom2 = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Actor,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(entity_id),
                target: EntityRef::Resolved(other2),
                implication_type: ImplicationType::Care,
                weight: 0.3,
            }],
            system_source(),
        );

        // Third event with same partner as first — should not increase partner count
        let atom3 = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(entity_id),
                role: ParticipantRole::Actor,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(entity_id),
                target: EntityRef::Resolved(other1),
                implication_type: ImplicationType::Conflict,
                weight: 0.1,
            }],
            system_source(),
        );

        let weight = compute_relational_weight(
            &entity,
            &[atom1, atom2, atom3],
            EntityId::new(),
            &make_config(),
        );
        assert_eq!(weight.relationship_count, 2);
        assert_eq!(weight.event_count, 3);
    }

    #[test]
    fn entity_not_in_event_ignored() {
        let entity_id = EntityId::new();
        let other1 = EntityId::new();
        let other2 = EntityId::new();
        let entity = make_resolved_ref(entity_id);

        // Event that doesn't involve our entity at all
        let atom = make_atom(
            vec![Participant {
                entity: EntityRef::Resolved(other1),
                role: ParticipantRole::Actor,
            }],
            vec![RelationalImplication {
                source: EntityRef::Resolved(other1),
                target: EntityRef::Resolved(other2),
                implication_type: ImplicationType::Attention,
                weight: 0.5,
            }],
            system_source(),
        );

        let weight = compute_relational_weight(&entity, &[atom], EntityId::new(), &make_config());
        assert!((weight.total_weight).abs() < f32::EPSILON);
        assert_eq!(weight.event_count, 0);
    }
}
