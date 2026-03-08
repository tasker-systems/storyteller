//! Relational implication inference — heuristic mapping from events to relationships.
//!
//! See: `docs/ticket-specs/event-system-foundations/phase-c-ml-classification-pipeline.md`
//!
//! Phase C.4: infer relational implications from an event's kind and participants.
//! This is a pure function on core types — no engine, no ML, no Bevy dependencies.
//!
//! The heuristic lookup table maps `EventKind × ParticipantRole → Vec<ImplicationType>`
//! with base weights scaled by event confidence. The Storykeeper decides whether
//! and how much to actually shift substrate values — implications are signals, not
//! commands.

use super::entity::EntityRef;
use super::event_grammar::{
    EventKind, ImplicationType, Participant, ParticipantRole, RelationalImplication,
};
use super::turn_cycle::EntityCategory;

/// Infer relational implications from an event's kind and participants.
///
/// Heuristic lookup: `EventKind` → base implications with weights, scaled
/// by `event_confidence`. Implications are emitted for Actor→Target pairs
/// in the participant list. When no explicit Target exists, Actor→self
/// implications are generated for emotional/internal events.
pub fn infer_implications_heuristic(
    kind: &EventKind,
    participants: &[Participant],
    event_confidence: f32,
) -> Vec<RelationalImplication> {
    let base_implications = base_implications_for_kind(kind);
    if base_implications.is_empty() {
        return Vec::new();
    }

    let actors: Vec<&Participant> = participants
        .iter()
        .filter(|p| p.role == ParticipantRole::Actor)
        .collect();
    let targets: Vec<&Participant> = participants
        .iter()
        .filter(|p| p.role == ParticipantRole::Target)
        .collect();

    let mut implications = Vec::new();

    for actor in &actors {
        if targets.is_empty() {
            // Actor→self for emotional/internal events
            for &(ref imp_type, base_weight) in &base_implications {
                implications.push(RelationalImplication {
                    source: actor.entity.clone(),
                    target: actor.entity.clone(),
                    implication_type: *imp_type,
                    weight: base_weight * event_confidence,
                });
            }
        } else {
            for target in &targets {
                for &(ref imp_type, base_weight) in &base_implications {
                    implications.push(RelationalImplication {
                        source: actor.entity.clone(),
                        target: target.entity.clone(),
                        implication_type: *imp_type,
                        weight: base_weight * event_confidence,
                    });
                }
            }
        }
    }

    implications
}

/// Look up the base relational implications for an event kind.
///
/// Returns `(ImplicationType, base_weight)` pairs. The caller scales
/// weights by event confidence.
fn base_implications_for_kind(kind: &EventKind) -> Vec<(ImplicationType, f32)> {
    match kind {
        EventKind::SpeechAct { .. } => vec![
            (ImplicationType::Attention, 0.3),
            (ImplicationType::InformationSharing, 0.5),
        ],
        EventKind::ActionOccurrence { .. } => vec![
            (ImplicationType::Attention, 0.3),
            (ImplicationType::Possession, 0.5),
            (ImplicationType::Proximity, 0.2),
        ],
        EventKind::SpatialChange { .. } => vec![(ImplicationType::Proximity, 0.2)],
        EventKind::EmotionalExpression { .. } => {
            vec![(ImplicationType::EmotionalConnection { valence: 0.0 }, 0.6)]
        }
        EventKind::InformationTransfer { .. } => vec![
            (ImplicationType::InformationSharing, 0.5),
            (ImplicationType::TrustSignal { direction: 0.5 }, 0.7),
        ],
        EventKind::RelationalShift { delta, .. } => {
            vec![(ImplicationType::TrustSignal { direction: *delta }, 1.0)]
        }
        EventKind::StateAssertion { .. }
        | EventKind::EnvironmentalChange { .. }
        | EventKind::SceneLifecycle { .. }
        | EventKind::EntityLifecycle { .. } => Vec::new(),
    }
}

/// Assign participant roles to entities based on position and event kind.
///
/// Heuristic: first Character → Actor, second Character → Target,
/// Object → Instrument, Location → Location, others → Witness.
///
/// Accepts `(EntityRef, EntityCategory)` tuples to avoid depending on
/// engine types. The caller converts `ExtractedEntity` → `(EntityRef, EntityCategory)`
/// before calling.
pub fn assign_participant_roles(
    entities: &[(EntityRef, EntityCategory)],
    _kind: &EventKind,
) -> Vec<Participant> {
    let mut participants = Vec::new();
    let mut first_character = true;

    for (entity_ref, category) in entities {
        let role = match category {
            EntityCategory::Character => {
                if first_character {
                    first_character = false;
                    ParticipantRole::Actor
                } else {
                    ParticipantRole::Target
                }
            }
            EntityCategory::Object => ParticipantRole::Instrument,
            EntityCategory::Location => ParticipantRole::Location,
            EntityCategory::Other => ParticipantRole::Witness,
        };

        participants.push(Participant {
            entity: entity_ref.clone(),
            role,
        });
    }

    participants
}

#[cfg(test)]
mod tests {
    use super::super::entity::{EntityId, ReferentialContext};
    use super::super::event::TurnId;
    use super::super::prediction::{ActionType, SpeechRegister};
    use super::super::scene::SceneId;
    use super::*;

    fn resolved(id: EntityId) -> EntityRef {
        EntityRef::Resolved(id)
    }

    fn actor(id: EntityId) -> Participant {
        Participant {
            entity: resolved(id),
            role: ParticipantRole::Actor,
        }
    }

    fn target(id: EntityId) -> Participant {
        Participant {
            entity: resolved(id),
            role: ParticipantRole::Target,
        }
    }

    // -----------------------------------------------------------------------
    // infer_implications_heuristic
    // -----------------------------------------------------------------------

    #[test]
    fn speech_act_produces_attention_and_info_sharing() {
        let sarah = EntityId::new();
        let tom = EntityId::new();
        let kind = EventKind::SpeechAct {
            register: SpeechRegister::Conversational,
        };
        let participants = vec![actor(sarah), target(tom)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        assert_eq!(implications.len(), 2);

        let types: Vec<_> = implications.iter().map(|i| &i.implication_type).collect();
        assert!(types
            .iter()
            .any(|t| matches!(t, ImplicationType::Attention)));
        assert!(types
            .iter()
            .any(|t| matches!(t, ImplicationType::InformationSharing)));
    }

    #[test]
    fn action_occurrence_produces_three_implications() {
        let sarah = EntityId::new();
        let stone = EntityId::new();
        let kind = EventKind::ActionOccurrence {
            action_type: ActionType::Perform,
        };
        let participants = vec![actor(sarah), target(stone)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        assert_eq!(implications.len(), 3);
    }

    #[test]
    fn spatial_change_produces_proximity() {
        let wolf = EntityId::new();
        let kind = EventKind::SpatialChange {
            from: Some("the clearing".to_string()),
            to: Some("the stream".to_string()),
        };
        let participants = vec![actor(wolf)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        assert_eq!(implications.len(), 1);
        assert!(matches!(
            implications[0].implication_type,
            ImplicationType::Proximity
        ));
    }

    #[test]
    fn emotional_expression_produces_emotional_connection() {
        let sarah = EntityId::new();
        let kind = EventKind::EmotionalExpression {
            emotion_hint: Some("grief".to_string()),
            intensity: 0.8,
        };
        let participants = vec![actor(sarah)];

        let implications = infer_implications_heuristic(&kind, &participants, 0.9);
        assert_eq!(implications.len(), 1);
        assert!(matches!(
            implications[0].implication_type,
            ImplicationType::EmotionalConnection { .. }
        ));
        // Actor→self when no target
        assert_eq!(
            implications[0].source.entity_id(),
            implications[0].target.entity_id()
        );
    }

    #[test]
    fn information_transfer_produces_info_and_trust() {
        let adam = EntityId::new();
        let sarah = EntityId::new();
        let kind = EventKind::InformationTransfer {
            content_summary: "Adam tells Sarah about the path".to_string(),
        };
        let participants = vec![actor(adam), target(sarah)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        assert_eq!(implications.len(), 2);

        let types: Vec<_> = implications.iter().map(|i| &i.implication_type).collect();
        assert!(types
            .iter()
            .any(|t| matches!(t, ImplicationType::InformationSharing)));
        assert!(types
            .iter()
            .any(|t| matches!(t, ImplicationType::TrustSignal { .. })));
    }

    #[test]
    fn relational_shift_uses_delta() {
        let sarah = EntityId::new();
        let adam = EntityId::new();
        let kind = EventKind::RelationalShift {
            dimension: "trust_reliability".to_string(),
            delta: -0.3,
        };
        let participants = vec![actor(sarah), target(adam)];

        let implications = infer_implications_heuristic(&kind, &participants, 0.8);
        assert_eq!(implications.len(), 1);
        if let ImplicationType::TrustSignal { direction } = implications[0].implication_type {
            assert!((direction - (-0.3)).abs() < f32::EPSILON);
        } else {
            panic!(
                "expected TrustSignal, got {:?}",
                implications[0].implication_type
            );
        }
    }

    #[test]
    fn state_assertion_produces_no_implications() {
        let sarah = EntityId::new();
        let kind = EventKind::StateAssertion {
            assertion: "Sarah sits at the table".to_string(),
        };
        let participants = vec![actor(sarah)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        assert!(implications.is_empty());
    }

    #[test]
    fn environmental_change_produces_no_implications() {
        let kind = EventKind::EnvironmentalChange {
            description: "Rain begins to fall".to_string(),
        };
        let implications = infer_implications_heuristic(&kind, &[], 1.0);
        assert!(implications.is_empty());
    }

    #[test]
    fn scene_lifecycle_produces_no_implications() {
        let kind = EventKind::SceneLifecycle {
            lifecycle_type: super::super::event_grammar::SceneLifecycleType::Entered,
        };
        let implications = infer_implications_heuristic(&kind, &[], 1.0);
        assert!(implications.is_empty());
    }

    #[test]
    fn entity_lifecycle_produces_no_implications() {
        let kind = EventKind::EntityLifecycle {
            lifecycle_type: super::super::event_grammar::EntityLifecycleType::Promoted,
        };
        let implications = infer_implications_heuristic(&kind, &[], 1.0);
        assert!(implications.is_empty());
    }

    #[test]
    fn confidence_scales_weights() {
        let sarah = EntityId::new();
        let tom = EntityId::new();
        let kind = EventKind::SpeechAct {
            register: SpeechRegister::Conversational,
        };
        let participants = vec![actor(sarah), target(tom)];

        let full = infer_implications_heuristic(&kind, &participants, 1.0);
        let half = infer_implications_heuristic(&kind, &participants, 0.5);

        for (f, h) in full.iter().zip(half.iter()) {
            assert!(
                (h.weight - f.weight * 0.5).abs() < f32::EPSILON,
                "expected {} * 0.5 = {}, got {}",
                f.weight,
                f.weight * 0.5,
                h.weight
            );
        }
    }

    #[test]
    fn empty_participants_produces_no_implications() {
        let kind = EventKind::SpeechAct {
            register: SpeechRegister::Conversational,
        };
        let implications = infer_implications_heuristic(&kind, &[], 1.0);
        assert!(implications.is_empty());
    }

    #[test]
    fn actor_self_for_emotional_with_no_target() {
        let sarah = EntityId::new();
        let kind = EventKind::EmotionalExpression {
            emotion_hint: Some("joy".to_string()),
            intensity: 0.5,
        };
        let participants = vec![actor(sarah)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        assert_eq!(implications.len(), 1);
        // Source and target are the same entity
        assert_eq!(implications[0].source.entity_id(), Some(sarah));
        assert_eq!(implications[0].target.entity_id(), Some(sarah));
    }

    #[test]
    fn multiple_actors_multiple_targets() {
        let a1 = EntityId::new();
        let a2 = EntityId::new();
        let t1 = EntityId::new();
        let kind = EventKind::SpeechAct {
            register: SpeechRegister::Conversational,
        };
        let participants = vec![actor(a1), actor(a2), target(t1)];

        let implications = infer_implications_heuristic(&kind, &participants, 1.0);
        // 2 actors × 1 target × 2 implication types = 4
        assert_eq!(implications.len(), 4);
    }

    // -----------------------------------------------------------------------
    // assign_participant_roles
    // -----------------------------------------------------------------------

    #[test]
    fn first_character_is_actor() {
        let sarah = EntityRef::Resolved(EntityId::new());
        let entities = vec![(sarah.clone(), EntityCategory::Character)];
        let kind = EventKind::SpeechAct {
            register: SpeechRegister::Conversational,
        };

        let participants = assign_participant_roles(&entities, &kind);
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].role, ParticipantRole::Actor);
    }

    #[test]
    fn second_character_is_target() {
        let sarah = EntityRef::Resolved(EntityId::new());
        let tom = EntityRef::Resolved(EntityId::new());
        let entities = vec![
            (sarah.clone(), EntityCategory::Character),
            (tom.clone(), EntityCategory::Character),
        ];
        let kind = EventKind::ActionOccurrence {
            action_type: ActionType::Perform,
        };

        let participants = assign_participant_roles(&entities, &kind);
        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].role, ParticipantRole::Actor);
        assert_eq!(participants[1].role, ParticipantRole::Target);
    }

    #[test]
    fn object_is_instrument() {
        let stone = EntityRef::Unresolved {
            mention: "the stone".to_string(),
            context: ReferentialContext {
                descriptors: vec![],
                spatial_context: None,
                possessor: None,
                prior_mentions: vec![],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
        };
        let entities = vec![(stone, EntityCategory::Object)];
        let kind = EventKind::ActionOccurrence {
            action_type: ActionType::Perform,
        };

        let participants = assign_participant_roles(&entities, &kind);
        assert_eq!(participants[0].role, ParticipantRole::Instrument);
    }

    #[test]
    fn location_is_location_role() {
        let clearing = EntityRef::Resolved(EntityId::new());
        let entities = vec![(clearing, EntityCategory::Location)];
        let kind = EventKind::SpatialChange {
            from: None,
            to: None,
        };

        let participants = assign_participant_roles(&entities, &kind);
        assert_eq!(participants[0].role, ParticipantRole::Location);
    }

    #[test]
    fn other_is_witness() {
        let sound = EntityRef::Resolved(EntityId::new());
        let entities = vec![(sound, EntityCategory::Other)];
        let kind = EventKind::EnvironmentalChange {
            description: "A sound echoes".to_string(),
        };

        let participants = assign_participant_roles(&entities, &kind);
        assert_eq!(participants[0].role, ParticipantRole::Witness);
    }

    #[test]
    fn mixed_entity_types() {
        let sarah = EntityRef::Resolved(EntityId::new());
        let stone = EntityRef::Resolved(EntityId::new());
        let clearing = EntityRef::Resolved(EntityId::new());
        let tom = EntityRef::Resolved(EntityId::new());

        let entities = vec![
            (sarah, EntityCategory::Character),
            (stone, EntityCategory::Object),
            (clearing, EntityCategory::Location),
            (tom, EntityCategory::Character),
        ];
        let kind = EventKind::ActionOccurrence {
            action_type: ActionType::Perform,
        };

        let participants = assign_participant_roles(&entities, &kind);
        assert_eq!(participants.len(), 4);
        assert_eq!(participants[0].role, ParticipantRole::Actor);
        assert_eq!(participants[1].role, ParticipantRole::Instrument);
        assert_eq!(participants[2].role, ParticipantRole::Location);
        assert_eq!(participants[3].role, ParticipantRole::Target);
    }

    #[test]
    fn empty_entities_produces_no_participants() {
        let kind = EventKind::StateAssertion {
            assertion: "test".to_string(),
        };
        let participants = assign_participant_roles(&[], &kind);
        assert!(participants.is_empty());
    }
}
