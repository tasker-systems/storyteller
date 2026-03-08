//! Turn-level event composition detection (Phase E).
//!
//! Bridges `ClassificationOutput` → `EventAtom`s, then detects compound events
//! from sequential atoms within committed turns. Composition detection runs
//! after D.3 committed-turn classification inside `commit_previous_system`.
//!
//! Two composition types are detected:
//! - **Causal**: ordered pairs matching known causal patterns AND sharing participant overlap
//! - **Temporal**: atoms within the same turn that share at least one Actor/Target participant
//!
//! Causal compositions have priority — when atoms match both, causal wins.

use chrono::Utc;

use storyteller_core::promotion::weight::entity_matches;
use storyteller_core::types::entity::{EntityRef, ReferentialContext};
use storyteller_core::types::event::{EventId, EventPriority, TurnId};
use storyteller_core::types::event_grammar::{
    ClassifierRef, CompositionType, CompoundEvent, ConfidenceEvidence, EventAtom, EventConfidence,
    EventKind, EventSource, ParticipantRole, RelationalImplication,
};
use storyteller_core::types::implication::{
    assign_participant_roles, infer_implications_heuristic,
};
use storyteller_core::types::prediction::{ActionType, SpeechRegister};
use storyteller_core::types::scene::SceneId;
use storyteller_core::types::turn_cycle::EntityCategory;

use crate::inference::event_classifier::{ClassificationOutput, ExtractedEntity};
use storyteller_ml::event_templates::NerCategory;

// ===========================================================================
// Bridge: ClassificationOutput → EventAtom
// ===========================================================================

/// Returns a `&str` discriminant for `EventKind` matching `EVENT_KIND_LABELS`.
fn event_kind_discriminant(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::StateAssertion { .. } => "StateAssertion",
        EventKind::ActionOccurrence { .. } => "ActionOccurrence",
        EventKind::SpatialChange { .. } => "SpatialChange",
        EventKind::EmotionalExpression { .. } => "EmotionalExpression",
        EventKind::InformationTransfer { .. } => "InformationTransfer",
        EventKind::SpeechAct { .. } => "SpeechAct",
        EventKind::RelationalShift { .. } => "RelationalShift",
        EventKind::EnvironmentalChange { .. } => "EnvironmentalChange",
        EventKind::SceneLifecycle { .. } => "SceneLifecycle",
        EventKind::EntityLifecycle { .. } => "EntityLifecycle",
    }
}

/// Convert classifier label string → `EventKind` with default sub-type values.
///
/// Unknown labels fall back to `StateAssertion`. The classifier cannot
/// distinguish sub-types (e.g., `ActionType::Examine` vs `Perform`), so
/// defaults are intentionally naive — production will use ML frames.
fn label_to_event_kind(label: &str) -> EventKind {
    match label {
        "StateAssertion" => EventKind::StateAssertion {
            assertion: String::new(),
        },
        "ActionOccurrence" => EventKind::ActionOccurrence {
            action_type: ActionType::Perform,
        },
        "SpatialChange" => EventKind::SpatialChange {
            from: None,
            to: None,
        },
        "EmotionalExpression" => EventKind::EmotionalExpression {
            emotion_hint: None,
            intensity: 0.5,
        },
        "InformationTransfer" => EventKind::InformationTransfer {
            content_summary: String::new(),
        },
        "SpeechAct" => EventKind::SpeechAct {
            register: SpeechRegister::Conversational,
        },
        "RelationalShift" => EventKind::RelationalShift {
            dimension: String::new(),
            delta: 0.0,
        },
        "EnvironmentalChange" => EventKind::EnvironmentalChange {
            description: String::new(),
        },
        _ => EventKind::StateAssertion {
            assertion: String::new(),
        },
    }
}

/// Convert `ExtractedEntity` → `(EntityRef::Unresolved, EntityCategory)`.
///
/// Maps `NerCategory` → `EntityCategory`: Character→Character, Object→Object,
/// Location→Location, all others→Other. Creates `EntityRef::Unresolved` with
/// the entity mention text and minimal `ReferentialContext`.
fn extracted_to_participant_input(
    entity: &ExtractedEntity,
    scene_id: SceneId,
    turn_id: TurnId,
) -> (EntityRef, EntityCategory) {
    let entity_ref = EntityRef::Unresolved {
        mention: entity.text.clone(),
        context: ReferentialContext {
            descriptors: vec![],
            spatial_context: None,
            possessor: None,
            prior_mentions: vec![],
            first_mentioned_scene: scene_id,
            first_mentioned_turn: turn_id,
        },
    };

    let category = match entity.category {
        NerCategory::Character => EntityCategory::Character,
        NerCategory::Object => EntityCategory::Object,
        NerCategory::Location => EntityCategory::Location,
        NerCategory::Gesture
        | NerCategory::Sensory
        | NerCategory::Abstract
        | NerCategory::Collective => EntityCategory::Other,
    };

    (entity_ref, category)
}

/// Convert `ClassificationOutput` → `Vec<EventAtom>`.
///
/// For each (label, confidence) in the classification:
/// 1. Map label → `EventKind` via `label_to_event_kind()`
/// 2. Convert entity mentions → `(EntityRef, EntityCategory)` tuples
/// 3. Call `assign_participant_roles()` from `implication.rs`
/// 4. Call `infer_implications_heuristic()` with the kind and participants
/// 5. Construct `EventAtom` with `TurnExtraction` source
pub fn build_event_atoms(
    classification: &ClassificationOutput,
    scene_id: SceneId,
    turn_id: TurnId,
) -> Vec<EventAtom> {
    let entity_inputs: Vec<(EntityRef, EntityCategory)> = classification
        .entity_mentions
        .iter()
        .map(|e| extracted_to_participant_input(e, scene_id, turn_id))
        .collect();

    classification
        .event_kinds
        .iter()
        .map(|(label, confidence)| {
            let kind = label_to_event_kind(label);
            let participants = assign_participant_roles(&entity_inputs, &kind);
            let relational_implications =
                infer_implications_heuristic(&kind, &participants, *confidence);

            EventAtom {
                id: EventId::new(),
                timestamp: Utc::now(),
                kind,
                participants,
                relational_implications,
                source: EventSource::TurnExtraction {
                    turn_id,
                    classifier: ClassifierRef {
                        name: "event_classifier".to_string(),
                        version: "0.1".to_string(),
                    },
                },
                confidence: EventConfidence {
                    value: *confidence,
                    evidence: ConfidenceEvidence::ClassifierOutput {
                        classifier: "event_classifier".to_string(),
                        latency_ms: 0,
                    },
                },
                priority: EventPriority::Normal,
                scene_id,
                turn_id: Some(turn_id),
            }
        })
        .collect()
}

// ===========================================================================
// Participant overlap check
// ===========================================================================

/// Check whether two atoms share a participant in Actor or Target role.
///
/// Uses `entity_matches()` from `promotion::weight` for entity comparison
/// (handles Resolved UUID equality, Unresolved normalized mention matching).
fn has_participant_overlap(a: &EventAtom, b: &EventAtom) -> bool {
    let a_actors_targets: Vec<&EntityRef> = a
        .participants
        .iter()
        .filter(|p| p.role == ParticipantRole::Actor || p.role == ParticipantRole::Target)
        .map(|p| &p.entity)
        .collect();

    let b_actors_targets: Vec<&EntityRef> = b
        .participants
        .iter()
        .filter(|p| p.role == ParticipantRole::Actor || p.role == ParticipantRole::Target)
        .map(|p| &p.entity)
        .collect();

    for a_entity in &a_actors_targets {
        for b_entity in &b_actors_targets {
            if entity_matches(a_entity, b_entity) {
                return true;
            }
        }
    }

    false
}

// ===========================================================================
// Causal composition detection (E.2)
// ===========================================================================

/// A known causal pattern between event kinds.
#[derive(Debug, Clone)]
struct CausalPattern {
    cause: &'static str,
    effect: &'static str,
    mechanism: &'static str,
}

/// The initial causal pattern library — data-driven causal rules.
///
/// Uses discriminant strings since the classifier doesn't distinguish sub-types.
fn causal_pattern_library() -> &'static [CausalPattern] {
    static PATTERNS: &[CausalPattern] = &[
        CausalPattern {
            cause: "ActionOccurrence",
            effect: "EmotionalExpression",
            mechanism: "action triggered emotional response",
        },
        CausalPattern {
            cause: "SpeechAct",
            effect: "RelationalShift",
            mechanism: "speech shifted relationship",
        },
        CausalPattern {
            cause: "ActionOccurrence",
            effect: "SpatialChange",
            mechanism: "action caused movement",
        },
        CausalPattern {
            cause: "InformationTransfer",
            effect: "EmotionalExpression",
            mechanism: "revelation triggered emotional response",
        },
        CausalPattern {
            cause: "SpeechAct",
            effect: "ActionOccurrence",
            mechanism: "speech prompted action",
        },
    ];
    PATTERNS
}

/// Detect causal compositions: ordered pairs matching known causal patterns
/// AND sharing participant overlap.
///
/// Returns `(compounds, consumed_pairs)` — consumed pairs are excluded from
/// temporal detection to avoid double-counting.
fn detect_causal_compositions(
    atoms: &[EventAtom],
) -> (Vec<CompoundEvent>, Vec<(EventId, EventId)>) {
    let mut compounds = Vec::new();
    let mut consumed = Vec::new();
    let patterns = causal_pattern_library();

    for i in 0..atoms.len() {
        for j in (i + 1)..atoms.len() {
            let cause_disc = event_kind_discriminant(&atoms[i].kind);
            let effect_disc = event_kind_discriminant(&atoms[j].kind);

            let matching_pattern = patterns
                .iter()
                .find(|p| p.cause == cause_disc && p.effect == effect_disc);

            if let Some(pattern) = matching_pattern {
                if has_participant_overlap(&atoms[i], &atoms[j]) {
                    let priority = atoms[i].priority.min(atoms[j].priority);
                    let avg_conf = (atoms[i].confidence.value + atoms[j].confidence.value) / 2.0;

                    compounds.push(CompoundEvent {
                        id: EventId::new(),
                        atoms: vec![atoms[i].id, atoms[j].id],
                        composition_type: CompositionType::Causal {
                            mechanism: pattern.mechanism.to_string(),
                        },
                        emergent_implications: vec![],
                        composition_confidence: avg_conf,
                        priority,
                    });

                    consumed.push((atoms[i].id, atoms[j].id));
                }
            }
        }
    }

    (compounds, consumed)
}

// ===========================================================================
// Temporal composition detection (E.1)
// ===========================================================================

/// Detect temporal compositions: atoms within the same set that share
/// at least one Actor/Target participant.
///
/// `excluded_pairs`: atom ID pairs already consumed by causal detection.
fn detect_temporal_compositions(
    atoms: &[EventAtom],
    excluded_pairs: &[(EventId, EventId)],
) -> Vec<CompoundEvent> {
    let mut compounds = Vec::new();

    for i in 0..atoms.len() {
        for j in (i + 1)..atoms.len() {
            let pair = (atoms[i].id, atoms[j].id);
            let is_excluded = excluded_pairs
                .iter()
                .any(|&(a, b)| (a == pair.0 && b == pair.1) || (a == pair.1 && b == pair.0));

            if is_excluded {
                continue;
            }

            if has_participant_overlap(&atoms[i], &atoms[j]) {
                let priority = atoms[i].priority.min(atoms[j].priority);
                let avg_conf = (atoms[i].confidence.value + atoms[j].confidence.value) / 2.0;

                compounds.push(CompoundEvent {
                    id: EventId::new(),
                    atoms: vec![atoms[i].id, atoms[j].id],
                    composition_type: CompositionType::Temporal,
                    emergent_implications: vec![],
                    composition_confidence: avg_conf,
                    priority,
                });
            }
        }
    }

    compounds
}

// ===========================================================================
// Emergent weight calculation (E.3)
// ===========================================================================

/// Compute emergent relational implications for a compound event.
///
/// Merges constituent atoms' implications with a multiplier:
/// - Causal: 1.5x
/// - Temporal: 1.0x (no amplification)
///
/// Deduplicates by (source, target) keeping max weight. Clamps to \[0.0, 1.0\].
fn compute_emergent_implications(
    atoms: &[EventAtom],
    atom_ids: &[EventId],
    composition_type: &CompositionType,
) -> Vec<RelationalImplication> {
    let multiplier = match composition_type {
        CompositionType::Causal { .. } => 1.5,
        _ => 1.0,
    };

    let mut merged: Vec<RelationalImplication> = Vec::new();

    for atom in atoms {
        if !atom_ids.contains(&atom.id) {
            continue;
        }
        for imp in &atom.relational_implications {
            let scaled_weight = (imp.weight * multiplier).clamp(0.0, 1.0);

            // Deduplicate by (source, target): keep max weight
            let existing = merged.iter_mut().find(|existing| {
                entity_matches(&existing.source, &imp.source)
                    && entity_matches(&existing.target, &imp.target)
            });

            match existing {
                Some(e) => {
                    if scaled_weight > e.weight {
                        e.weight = scaled_weight;
                        e.implication_type = imp.implication_type;
                    }
                }
                None => {
                    merged.push(RelationalImplication {
                        source: imp.source.clone(),
                        target: imp.target.clone(),
                        implication_type: imp.implication_type,
                        weight: scaled_weight,
                    });
                }
            }
        }
    }

    merged
}

// ===========================================================================
// Top-level pipeline orchestrator
// ===========================================================================

/// Run full composition detection: causal first (higher priority),
/// then temporal (excluding causal-consumed pairs), with emergent
/// implications computed for all compounds.
pub fn detect_compositions(atoms: &[EventAtom]) -> Vec<CompoundEvent> {
    if atoms.len() < 2 {
        return Vec::new();
    }

    // Causal first (higher priority)
    let (mut causal_compounds, consumed_pairs) = detect_causal_compositions(atoms);

    // Temporal second (excluding consumed pairs)
    let mut temporal_compounds = detect_temporal_compositions(atoms, &consumed_pairs);

    // Compute emergent implications for all compounds
    for compound in causal_compounds
        .iter_mut()
        .chain(temporal_compounds.iter_mut())
    {
        compound.emergent_implications =
            compute_emergent_implications(atoms, &compound.atoms, &compound.composition_type);
    }

    causal_compounds.append(&mut temporal_compounds);
    causal_compounds
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::event_grammar::Participant;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn make_actor(mention: &str) -> Participant {
        Participant {
            entity: EntityRef::Unresolved {
                mention: mention.to_string(),
                context: ReferentialContext {
                    descriptors: vec![],
                    spatial_context: None,
                    possessor: None,
                    prior_mentions: vec![],
                    first_mentioned_scene: SceneId::new(),
                    first_mentioned_turn: TurnId::new(),
                },
            },
            role: ParticipantRole::Actor,
        }
    }

    fn make_target(mention: &str) -> Participant {
        Participant {
            entity: EntityRef::Unresolved {
                mention: mention.to_string(),
                context: ReferentialContext {
                    descriptors: vec![],
                    spatial_context: None,
                    possessor: None,
                    prior_mentions: vec![],
                    first_mentioned_scene: SceneId::new(),
                    first_mentioned_turn: TurnId::new(),
                },
            },
            role: ParticipantRole::Target,
        }
    }

    fn make_test_atom(kind: EventKind, participants: Vec<Participant>) -> EventAtom {
        let implications = infer_implications_heuristic(&kind, &participants, 0.8);
        EventAtom {
            id: EventId::new(),
            timestamp: Utc::now(),
            kind,
            participants,
            relational_implications: implications,
            source: EventSource::TurnExtraction {
                turn_id: TurnId::new(),
                classifier: ClassifierRef {
                    name: "test".to_string(),
                    version: "0.1".to_string(),
                },
            },
            confidence: EventConfidence {
                value: 0.8,
                evidence: ConfidenceEvidence::SystemProduced,
            },
            priority: EventPriority::Normal,
            scene_id: SceneId::new(),
            turn_id: Some(TurnId::new()),
        }
    }

    // -----------------------------------------------------------------------
    // Bridge tests
    // -----------------------------------------------------------------------

    #[test]
    fn label_to_event_kind_known_labels() {
        assert!(matches!(
            label_to_event_kind("StateAssertion"),
            EventKind::StateAssertion { .. }
        ));
        assert!(matches!(
            label_to_event_kind("ActionOccurrence"),
            EventKind::ActionOccurrence { .. }
        ));
        assert!(matches!(
            label_to_event_kind("SpatialChange"),
            EventKind::SpatialChange { .. }
        ));
        assert!(matches!(
            label_to_event_kind("EmotionalExpression"),
            EventKind::EmotionalExpression { .. }
        ));
        assert!(matches!(
            label_to_event_kind("InformationTransfer"),
            EventKind::InformationTransfer { .. }
        ));
        assert!(matches!(
            label_to_event_kind("SpeechAct"),
            EventKind::SpeechAct { .. }
        ));
        assert!(matches!(
            label_to_event_kind("RelationalShift"),
            EventKind::RelationalShift { .. }
        ));
        assert!(matches!(
            label_to_event_kind("EnvironmentalChange"),
            EventKind::EnvironmentalChange { .. }
        ));
    }

    #[test]
    fn label_to_event_kind_unknown_returns_state_assertion() {
        assert!(matches!(
            label_to_event_kind("UnknownLabel"),
            EventKind::StateAssertion { .. }
        ));
        assert!(matches!(
            label_to_event_kind(""),
            EventKind::StateAssertion { .. }
        ));
    }

    #[test]
    fn build_event_atoms_from_single_kind() {
        let classification = ClassificationOutput {
            event_kinds: vec![("ActionOccurrence".to_string(), 0.9)],
            entity_mentions: vec![
                ExtractedEntity {
                    text: "Sarah".to_string(),
                    start: 0,
                    end: 5,
                    category: NerCategory::Character,
                    confidence: 0.95,
                },
                ExtractedEntity {
                    text: "the stone".to_string(),
                    start: 14,
                    end: 23,
                    category: NerCategory::Object,
                    confidence: 0.88,
                },
            ],
        };

        let atoms = build_event_atoms(&classification, SceneId::new(), TurnId::new());
        assert_eq!(atoms.len(), 1);
        assert!(matches!(atoms[0].kind, EventKind::ActionOccurrence { .. }));
        // Sarah → Actor, the stone → Instrument
        assert_eq!(atoms[0].participants.len(), 2);
        assert_eq!(atoms[0].participants[0].role, ParticipantRole::Actor);
        assert_eq!(atoms[0].participants[1].role, ParticipantRole::Instrument);
        // ActionOccurrence has 3 base implications (Attention, Possession, Proximity)
        // but only Actor→Instrument pairs (no Target), so 3 implications
        assert!(!atoms[0].relational_implications.is_empty());
    }

    #[test]
    fn build_event_atoms_multiple_kinds() {
        let classification = ClassificationOutput {
            event_kinds: vec![
                ("SpeechAct".to_string(), 0.85),
                ("EmotionalExpression".to_string(), 0.7),
            ],
            entity_mentions: vec![ExtractedEntity {
                text: "Sarah".to_string(),
                start: 0,
                end: 5,
                category: NerCategory::Character,
                confidence: 0.95,
            }],
        };

        let atoms = build_event_atoms(&classification, SceneId::new(), TurnId::new());
        assert_eq!(atoms.len(), 2);
        assert!(matches!(atoms[0].kind, EventKind::SpeechAct { .. }));
        assert!(matches!(
            atoms[1].kind,
            EventKind::EmotionalExpression { .. }
        ));
    }

    #[test]
    fn build_event_atoms_empty_classification() {
        let classification = ClassificationOutput {
            event_kinds: vec![],
            entity_mentions: vec![],
        };

        let atoms = build_event_atoms(&classification, SceneId::new(), TurnId::new());
        assert!(atoms.is_empty());
    }

    // -----------------------------------------------------------------------
    // Participant overlap tests
    // -----------------------------------------------------------------------

    #[test]
    fn participant_overlap_shared_actor() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Sarah")],
        );
        assert!(has_participant_overlap(&a, &b));
    }

    #[test]
    fn participant_overlap_actor_target_cross() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::EmotionalExpression {
                emotion_hint: None,
                intensity: 0.5,
            },
            vec![make_target("Sarah")],
        );
        assert!(has_participant_overlap(&a, &b));
    }

    #[test]
    fn participant_overlap_no_match() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Tom")],
        );
        assert!(!has_participant_overlap(&a, &b));
    }

    // -----------------------------------------------------------------------
    // Temporal composition tests
    // -----------------------------------------------------------------------

    #[test]
    fn temporal_shared_participants_compose() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );
        let b = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Sarah")],
        );

        let compounds = detect_temporal_compositions(&[a, b], &[]);
        assert_eq!(compounds.len(), 1);
        assert!(matches!(
            compounds[0].composition_type,
            CompositionType::Temporal
        ));
    }

    #[test]
    fn temporal_no_overlap_skips() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Tom")],
        );

        let compounds = detect_temporal_compositions(&[a, b], &[]);
        assert!(compounds.is_empty());
    }

    #[test]
    fn temporal_respects_excluded_pairs() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Sarah")],
        );

        let excluded = vec![(a.id, b.id)];
        let compounds = detect_temporal_compositions(&[a, b], &excluded);
        assert!(
            compounds.is_empty(),
            "excluded pair should not produce temporal compound"
        );
    }

    // -----------------------------------------------------------------------
    // Causal composition tests
    // -----------------------------------------------------------------------

    #[test]
    fn causal_action_then_emotional() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );
        let b = make_test_atom(
            EventKind::EmotionalExpression {
                emotion_hint: Some("grief".to_string()),
                intensity: 0.8,
            },
            vec![make_actor("Tom")],
        );

        let (compounds, consumed) = detect_causal_compositions(&[a, b]);
        assert_eq!(compounds.len(), 1);
        assert!(matches!(
            compounds[0].composition_type,
            CompositionType::Causal { .. }
        ));
        if let CompositionType::Causal { ref mechanism } = compounds[0].composition_type {
            assert_eq!(mechanism, "action triggered emotional response");
        }
        assert_eq!(consumed.len(), 1);
    }

    #[test]
    fn causal_speech_then_relational_shift() {
        let a = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Sarah"), make_target("Adam")],
        );
        let b = make_test_atom(
            EventKind::RelationalShift {
                dimension: "trust".to_string(),
                delta: -0.3,
            },
            vec![make_actor("Sarah"), make_target("Adam")],
        );

        let (compounds, _) = detect_causal_compositions(&[a, b]);
        assert_eq!(compounds.len(), 1);
        if let CompositionType::Causal { ref mechanism } = compounds[0].composition_type {
            assert_eq!(mechanism, "speech shifted relationship");
        }
    }

    #[test]
    fn causal_requires_participant_overlap() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::EmotionalExpression {
                emotion_hint: None,
                intensity: 0.5,
            },
            vec![make_actor("Tom")],
        );

        let (compounds, _) = detect_causal_compositions(&[a, b]);
        assert!(
            compounds.is_empty(),
            "causal should require participant overlap"
        );
    }

    #[test]
    fn causal_wrong_order_skips() {
        // EmotionalExpression before ActionOccurrence — not a matching pattern
        let a = make_test_atom(
            EventKind::EmotionalExpression {
                emotion_hint: None,
                intensity: 0.5,
            },
            vec![make_actor("Sarah")],
        );
        let b = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );

        let (compounds, _) = detect_causal_compositions(&[a, b]);
        assert!(compounds.is_empty(), "wrong causal order should not match");
    }

    // -----------------------------------------------------------------------
    // Emergent weight tests
    // -----------------------------------------------------------------------

    #[test]
    fn emergent_causal_multiplier_applied() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );

        // Get the original weight for comparison
        let original_max_weight = a
            .relational_implications
            .iter()
            .map(|i| i.weight)
            .fold(0.0_f32, f32::max);

        let emergent = compute_emergent_implications(
            &[a.clone()],
            &[a.id],
            &CompositionType::Causal {
                mechanism: "test".to_string(),
            },
        );

        assert!(!emergent.is_empty());
        // Causal multiplier = 1.5x, should increase weights (clamped to 1.0)
        let emergent_max = emergent.iter().map(|i| i.weight).fold(0.0_f32, f32::max);
        let expected = (original_max_weight * 1.5).clamp(0.0, 1.0);
        assert!(
            (emergent_max - expected).abs() < f32::EPSILON,
            "expected {expected}, got {emergent_max}"
        );
    }

    #[test]
    fn emergent_temporal_no_amplification() {
        let a = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );

        let original_max_weight = a
            .relational_implications
            .iter()
            .map(|i| i.weight)
            .fold(0.0_f32, f32::max);

        let emergent =
            compute_emergent_implications(&[a.clone()], &[a.id], &CompositionType::Temporal);

        assert!(!emergent.is_empty());
        let emergent_max = emergent.iter().map(|i| i.weight).fold(0.0_f32, f32::max);
        assert!(
            (emergent_max - original_max_weight).abs() < f32::EPSILON,
            "temporal should not amplify: expected {original_max_weight}, got {emergent_max}"
        );
    }

    // -----------------------------------------------------------------------
    // Pipeline tests
    // -----------------------------------------------------------------------

    #[test]
    fn detect_compositions_prefers_causal_over_temporal() {
        // ActionOccurrence → EmotionalExpression with shared participant
        // should produce causal, not temporal
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );
        let b = make_test_atom(
            EventKind::EmotionalExpression {
                emotion_hint: None,
                intensity: 0.5,
            },
            vec![make_actor("Tom")],
        );

        let compounds = detect_compositions(&[a, b]);
        assert_eq!(compounds.len(), 1, "should produce exactly one compound");
        assert!(
            matches!(
                compounds[0].composition_type,
                CompositionType::Causal { .. }
            ),
            "should be causal, not temporal"
        );
        // Emergent implications should be populated
        assert!(
            !compounds[0].emergent_implications.is_empty(),
            "emergent implications should be computed"
        );
    }

    #[test]
    fn detect_compositions_mixed_atoms() {
        // 3 atoms: A (ActionOccurrence, Sarah→Tom), B (EmotionalExpression, Tom),
        // C (SpeechAct, Sarah→Tom)
        // A→B should be causal, A-C and B-C should be temporal (if overlap)
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );
        let b = make_test_atom(
            EventKind::EmotionalExpression {
                emotion_hint: None,
                intensity: 0.5,
            },
            vec![make_actor("Tom")],
        );
        let c = make_test_atom(
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            vec![make_actor("Sarah"), make_target("Tom")],
        );

        let compounds = detect_compositions(&[a, b, c]);

        let causal_count = compounds
            .iter()
            .filter(|c| matches!(c.composition_type, CompositionType::Causal { .. }))
            .count();
        let temporal_count = compounds
            .iter()
            .filter(|c| matches!(c.composition_type, CompositionType::Temporal))
            .count();

        assert!(
            causal_count >= 1,
            "should have at least 1 causal compound (A→B)"
        );
        assert!(
            temporal_count >= 1,
            "should have at least 1 temporal compound"
        );
        assert!(
            compounds.len() >= 2,
            "expected at least 2 compounds, got {}",
            compounds.len()
        );
    }

    #[test]
    fn detect_compositions_empty_atoms() {
        let compounds = detect_compositions(&[]);
        assert!(compounds.is_empty());
    }

    #[test]
    fn detect_compositions_single_atom() {
        let a = make_test_atom(
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            vec![make_actor("Sarah")],
        );
        let compounds = detect_compositions(&[a]);
        assert!(compounds.is_empty(), "single atom cannot compose");
    }

    // -----------------------------------------------------------------------
    // Discriminant test
    // -----------------------------------------------------------------------

    #[test]
    fn event_kind_discriminant_all_variants() {
        assert_eq!(
            event_kind_discriminant(&EventKind::StateAssertion {
                assertion: String::new()
            }),
            "StateAssertion"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::ActionOccurrence {
                action_type: ActionType::Perform
            }),
            "ActionOccurrence"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::SpatialChange {
                from: None,
                to: None
            }),
            "SpatialChange"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::EmotionalExpression {
                emotion_hint: None,
                intensity: 0.5
            }),
            "EmotionalExpression"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::InformationTransfer {
                content_summary: String::new()
            }),
            "InformationTransfer"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::SpeechAct {
                register: SpeechRegister::Conversational
            }),
            "SpeechAct"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::RelationalShift {
                dimension: String::new(),
                delta: 0.0
            }),
            "RelationalShift"
        );
        assert_eq!(
            event_kind_discriminant(&EventKind::EnvironmentalChange {
                description: String::new()
            }),
            "EnvironmentalChange"
        );
    }

    // -----------------------------------------------------------------------
    // Entity conversion test
    // -----------------------------------------------------------------------

    #[test]
    fn extracted_to_participant_input_maps_categories() {
        let scene_id = SceneId::new();
        let turn_id = TurnId::new();

        let char_entity = ExtractedEntity {
            text: "Sarah".to_string(),
            start: 0,
            end: 5,
            category: NerCategory::Character,
            confidence: 0.9,
        };
        let (_, cat) = extracted_to_participant_input(&char_entity, scene_id, turn_id);
        assert_eq!(cat, EntityCategory::Character);

        let obj_entity = ExtractedEntity {
            text: "the stone".to_string(),
            start: 0,
            end: 9,
            category: NerCategory::Object,
            confidence: 0.8,
        };
        let (_, cat) = extracted_to_participant_input(&obj_entity, scene_id, turn_id);
        assert_eq!(cat, EntityCategory::Object);

        let loc_entity = ExtractedEntity {
            text: "the clearing".to_string(),
            start: 0,
            end: 12,
            category: NerCategory::Location,
            confidence: 0.85,
        };
        let (_, cat) = extracted_to_participant_input(&loc_entity, scene_id, turn_id);
        assert_eq!(cat, EntityCategory::Location);

        let gesture_entity = ExtractedEntity {
            text: "clenched fists".to_string(),
            start: 0,
            end: 14,
            category: NerCategory::Gesture,
            confidence: 0.7,
        };
        let (_, cat) = extracted_to_participant_input(&gesture_entity, scene_id, turn_id);
        assert_eq!(cat, EntityCategory::Other);
    }
}
