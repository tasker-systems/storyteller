//! Event grammar types — the formal specification of events in the storyteller system.
//!
//! See: `docs/ticket-specs/event-system-foundations/event-grammar.md`
//!
//! Design decision: Events create relationships, and relationships create entities
//! worth tracking. An `EventAtom` is the minimal unit. Everything the event system
//! processes is either an atom or a composition of atoms.
//!
//! The event grammar extends (not replaces) existing types:
//! - `EventType` (in prediction.rs) remains the classifier's output vocabulary
//! - `EventPriority` (in event.rs) applies directly to atoms and compounds
//! - `ClassifiedEvent` (in prediction.rs) is mapped to `EventAtom` by the pipeline

use chrono::{DateTime, Utc};

use super::entity::{EntityId, EntityRef};
use super::event::{EventId, EventPriority, TurnId};
use super::prediction::{ActionType, SpeechRegister};
use super::scene::SceneId;

// ===========================================================================
// Event Kind — the semantic taxonomy
// ===========================================================================

/// What kind of event occurred — the semantic category.
///
/// This taxonomy covers events from all sources: player input, character
/// predictions, narrator output, system events, and composed events.
/// The existing `EventType` (Speech, Action, Movement, etc.) maps to a
/// subset of these kinds for player-input events specifically.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    /// An assertion about current state — "Tanya is sitting at the table."
    /// Establishes facts in the truth set but often carries no relational
    /// implications on its own.
    StateAssertion {
        /// What is being asserted.
        assertion: String,
    },

    /// An action that occurred — "Sarah picked up the stone."
    /// Most actions carry relational implications (actor->target, actor->instrument).
    ActionOccurrence {
        /// The type of action (maps to existing `ActionType` where applicable).
        action_type: ActionType,
    },

    /// A spatial change — "The Wolf crossed the stream."
    /// May carry relational implications if the movement is toward or away
    /// from another entity.
    SpatialChange {
        /// Where from (if known).
        from: Option<String>,
        /// Where to (if known).
        to: Option<String>,
    },

    /// An emotional expression — visible to others in the scene.
    /// "Tanya begins to cry." Creates relational implications with witnesses.
    EmotionalExpression {
        /// Which emotion, if identifiable.
        emotion_hint: Option<String>,
        /// Intensity of expression. Range: \[0.0, 1.0\].
        intensity: f32,
    },

    /// Information transfer — one entity reveals something to another.
    /// "Adam tells Sarah about the path." Always relational.
    InformationTransfer {
        /// What was communicated (summary, not content).
        content_summary: String,
    },

    /// A speech act — someone speaks.
    SpeechAct {
        /// The register of speech delivery.
        register: SpeechRegister,
    },

    /// A relational shift — the relationship between entities changes.
    /// Typically produced by the interpretive track, not the factual track.
    RelationalShift {
        /// Which substrate dimension shifted.
        dimension: String,
        /// Direction and magnitude.
        delta: f32,
    },

    /// An environmental or world-state change — "Rain begins to fall."
    EnvironmentalChange {
        /// What changed.
        description: String,
    },

    /// A scene lifecycle event — entry, exit, transition.
    SceneLifecycle {
        /// What happened in the lifecycle.
        lifecycle_type: SceneLifecycleType,
    },

    /// An entity lifecycle event — promotion, demotion, dissolution.
    EntityLifecycle {
        /// What happened to the entity.
        lifecycle_type: EntityLifecycleType,
    },
}

/// Scene lifecycle subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SceneLifecycleType {
    /// The scene was entered.
    Entered,
    /// The scene was exited.
    Exited,
    /// A character arrived in the scene.
    CharacterArrival,
    /// A character departed the scene.
    CharacterDeparture,
}

/// Entity lifecycle subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntityLifecycleType {
    /// The entity was promoted to a higher tier.
    Promoted,
    /// The entity was demoted to a lower tier.
    Demoted,
    /// The entity was dissolved (removed from tracking).
    Dissolved,
}

// ===========================================================================
// Participants — who or what is involved in an event
// ===========================================================================

/// An entity participating in an event, with its role.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Participant {
    /// Reference to the entity — may be resolved (has `EntityId`) or
    /// unresolved (mentioned by name/description, identity pending).
    pub entity: EntityRef,
    /// What role this entity plays in the event.
    pub role: ParticipantRole,
}

/// The role an entity plays in an event.
///
/// Roles determine how the event's relational implications flow. An Actor
/// has agency; a Target receives the action; a Witness observes but is
/// not directly involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParticipantRole {
    /// The entity performing the action.
    Actor,
    /// The entity receiving the action or speech.
    Target,
    /// An object or tool used in the action.
    Instrument,
    /// Where the event takes place.
    Location,
    /// An entity that observes the event without direct participation.
    Witness,
    /// The entity being discussed or referenced without being present.
    Subject,
}

// ===========================================================================
// Relational implications — how events create relationships
// ===========================================================================

/// A relational implication of an event — what relationship it creates
/// or modifies between participants.
///
/// An event may have zero or many relational implications.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationalImplication {
    /// The source entity in the directed relationship.
    pub source: EntityRef,
    /// The target entity in the directed relationship.
    pub target: EntityRef,
    /// What kind of relational change this implies.
    pub implication_type: ImplicationType,
    /// How strong this relational implication is. Range: \[0.0, 1.0\].
    pub weight: f32,
}

/// What kind of relational change an event implies.
///
/// These map to (but are not identical to) the substrate dimensions in
/// `RelationalSubstrate`. An implication is a *signal* that a substrate
/// dimension may need updating; the Storykeeper decides whether and how
/// much to actually shift the substrate values.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImplicationType {
    /// Possession or ownership — "picked up the cup."
    Possession,
    /// Spatial proximity — "approached the fence."
    Proximity,
    /// Attention or focus — "looked at the flowers."
    Attention,
    /// Emotional connection (positive or negative) — "cried because of the chip."
    EmotionalConnection {
        /// Positive or negative valence.
        valence: f32,
    },
    /// Trust signal — "told her the truth" or "lied about the path."
    TrustSignal {
        /// Direction of trust change.
        direction: f32,
    },
    /// Information sharing — "revealed the secret."
    InformationSharing,
    /// Conflict or opposition — "resisted the Wolf's guidance."
    Conflict,
    /// Care or protection — "shielded the child."
    Care,
    /// Debt or obligation — "saved his life" or "borrowed the flute."
    Obligation {
        /// Positive = debt owed to source, negative = debt owed to target.
        direction: f32,
    },
}

// ===========================================================================
// Event source and confidence — provenance tracking
// ===========================================================================

/// Where an event originated — for provenance and confidence calibration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventSource {
    /// Classified from player input text.
    PlayerInput {
        /// The original player text (or a reference to it).
        raw_input: String,
        /// Which classifier produced this event.
        classifier: ClassifierRef,
    },
    /// Extracted from committed turn content — Narrator prose + player
    /// response processed together as a turn unit.
    TurnExtraction {
        /// The turn this event was extracted from.
        turn_id: TurnId,
        /// Which classifier processed the turn unit.
        classifier: ClassifierRef,
    },
    /// Originally hypothesized by an ML character prediction, confirmed
    /// by turn extraction.
    ConfirmedPrediction {
        /// Which character's prediction originally hypothesized this event.
        character_id: EntityId,
        /// The confidence of the original ML prediction.
        prediction_confidence: f32,
    },
    /// Produced by the engine's deterministic systems.
    System {
        /// Which system component produced this.
        component: String,
    },
    /// Derived from composition of other atoms.
    Composed {
        /// The atoms this was composed from.
        source_atom_ids: Vec<EventId>,
    },
}

/// Reference to a classifier model.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassifierRef {
    /// Name of the classifier.
    pub name: String,
    /// Version or model identifier.
    pub version: String,
}

/// How confident the system is that this event occurred, with provenance.
///
/// Confidence is not a single number — it carries the reasoning that
/// produced it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventConfidence {
    /// Scalar confidence. Range: \[0.0, 1.0\].
    pub value: f32,
    /// What evidence supports this confidence.
    pub evidence: ConfidenceEvidence,
}

/// What supports the confidence assessment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConfidenceEvidence {
    /// The classifier produced this confidence directly.
    ClassifierOutput {
        /// Classifier model name.
        classifier: String,
        /// Latency of classification in milliseconds.
        latency_ms: u32,
    },
    /// System-produced events have inherent confidence.
    SystemProduced,
    /// Sensitivity map keyword match.
    SensitivityMatch {
        /// Which sensitivity entry matched.
        entry_id: String,
        /// The baseline confidence from the sensitivity entry.
        baseline: f32,
    },
    /// Refined by deep interpretation (Stage 4).
    DeepInterpretation {
        /// The provisional confidence before refinement.
        provisional: f32,
        /// Supporting event references.
        supporting_events: Vec<EventId>,
    },
    /// Composed from child atom confidences.
    ComposedConfidence {
        /// The individual atom confidences.
        atom_confidences: Vec<f32>,
    },
}

// ===========================================================================
// The Event Atom — minimal unit of event
// ===========================================================================

/// The minimal unit of event in the system.
///
/// Every event — player action, character prediction, narrator description,
/// system lifecycle — is represented as one or more `EventAtom`s. Atoms are
/// the entries in the event ledger and the inputs to trigger evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventAtom {
    /// Unique identifier for this atom in the ledger.
    pub id: EventId,
    /// When this event was recorded.
    pub timestamp: DateTime<Utc>,
    /// What kind of event this is.
    pub kind: EventKind,
    /// Who or what is involved.
    pub participants: Vec<Participant>,
    /// What relationships this event creates, modifies, or implies.
    pub relational_implications: Vec<RelationalImplication>,
    /// Where this event came from.
    pub source: EventSource,
    /// How confident we are that this event occurred.
    pub confidence: EventConfidence,
    /// Processing priority.
    pub priority: EventPriority,
    /// The scene in which this event occurred.
    pub scene_id: SceneId,
    /// The turn in which this event occurred (if applicable).
    /// System events (scene lifecycle) may not belong to a specific turn.
    pub turn_id: Option<TurnId>,
}

// ===========================================================================
// Composition — compound events from atom patterns
// ===========================================================================

/// A compound event — an ordered set of atoms with a composition type
/// that together carry more narrative weight than the sum of their parts.
///
/// Compound events are derived, not observed. The composition detector
/// identifies them from sequential atoms based on temporal proximity,
/// participant overlap, and causal/thematic patterns.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompoundEvent {
    /// Unique identifier for this compound event.
    pub id: EventId,
    /// The atom IDs that compose this event, in order.
    pub atoms: Vec<EventId>,
    /// How the atoms are related.
    pub composition_type: CompositionType,
    /// The emergent relational implications — may differ from or exceed
    /// the sum of the individual atoms' implications.
    pub emergent_implications: Vec<RelationalImplication>,
    /// Confidence in the composition itself (not in the individual atoms).
    /// Range: \[0.0, 1.0\].
    pub composition_confidence: f32,
    /// Processing priority — inherited from the highest-priority atom,
    /// or elevated if the composition has narrative urgency.
    pub priority: EventPriority,
}

/// How atoms in a compound event are related.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CompositionType {
    /// A caused B — "she saw the chip and began to cry."
    Causal {
        /// Brief description of the causal relationship.
        mechanism: String,
    },
    /// A then B — temporal sequence without explicit causation.
    Temporal,
    /// A enabled B — A created conditions for B to occur.
    Conditional {
        /// What condition was created.
        condition: String,
    },
    /// A echoes B — thematic resonance without causal or temporal link.
    Thematic {
        /// What theme connects the atoms.
        theme: String,
    },
}

// ===========================================================================
// Turn types — the atomic unit of event extraction
// ===========================================================================

/// The lifecycle state of a turn.
///
/// Ordering is meaningful: `Hypothesized < Rendered < Committed`.
/// Events progress through these states and are only committed to the
/// event ledger at the `Committed` stage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum TurnState {
    /// ML predictions computed. Working state only — informs Narrator
    /// context assembly.
    Hypothesized,
    /// Narrator has produced prose. Player may or may not have seen it.
    /// Provisional — can be rejected/re-rendered.
    Rendered,
    /// Player has responded. Turn is complete. Events are extracted,
    /// ledger updated, truth set modified.
    Committed,
}

/// A turn — the unit of player interaction within a scene.
///
/// A turn captures the full cycle: system generates content (predictions +
/// Narrator rendering), player witnesses and responds, events are extracted.
/// The turn is the atomic unit of event extraction — nothing is committed
/// to the event ledger until a turn completes.
///
/// In Bevy, the active turn is a Resource. Scene entry creates the first
/// turn; each player response commits the current turn and creates the next.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Turn {
    /// Unique identifier.
    pub id: TurnId,
    /// The scene this turn belongs to.
    pub scene_id: SceneId,
    /// Ordinal position within the scene (1-indexed).
    pub turn_number: u32,
    /// Current lifecycle state.
    pub state: TurnState,
    /// When the turn was created (predictions begin).
    pub created_at: DateTime<Utc>,
    /// When the Narrator's rendering was completed.
    pub rendered_at: Option<DateTime<Utc>>,
    /// When the player responded (turn committed).
    pub committed_at: Option<DateTime<Utc>>,
}

/// A committed turn — the atomic unit of event extraction.
///
/// Contains everything needed to extract events: what was rendered,
/// what the player did, and what the ML models predicted. Once a
/// turn is committed, its events are extracted exactly once
/// (idempotency guarantee).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommittedTurn {
    /// Unique identifier for this turn.
    pub turn_id: TurnId,
    /// The scene this turn occurred in.
    pub scene_id: SceneId,
    /// Ordinal position within the scene.
    pub turn_number: u32,
    /// The Narrator's rendered prose for this turn.
    pub narrator_prose: String,
    /// The player's response text.
    pub player_response: String,
    /// ML prediction metadata — not events themselves, but evidence
    /// that supports extraction confidence.
    pub prediction_metadata: Vec<PredictionMetadata>,
    /// Whether events have already been extracted for this turn.
    /// Once true, the turn is not re-processed (idempotency).
    pub events_extracted: bool,
}

/// Metadata from an ML character prediction, preserved as evidence
/// for turn extraction. Not an event source — evidence for one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PredictionMetadata {
    /// Which character this prediction was for.
    pub character_id: EntityId,
    /// What the model predicted (action type).
    pub predicted_action_type: Option<ActionType>,
    /// Whether the model predicted speech.
    pub predicted_speech: bool,
    /// Whether the model predicted an emotional shift.
    pub predicted_emotional_shift: bool,
    /// The model's confidence in its predictions. Range: \[0.0, 1.0\].
    pub confidence: f32,
}

// ===========================================================================
// Event payload — typed wrapper for NarrativeEvent
// ===========================================================================

/// The typed payload of a narrative event.
///
/// Replaces the untyped `serde_json::Value` payload on `NarrativeEvent`
/// with structured event data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventPayload {
    /// A single event atom.
    Atom(EventAtom),
    /// A compound event (composition of atoms).
    Compound(CompoundEvent),
    /// Legacy untyped payload — for migration compatibility.
    Untyped(serde_json::Value),
}

#[cfg(test)]
mod tests {
    use super::super::entity::ReferentialContext;
    use super::*;

    // -----------------------------------------------------------------------
    // EventKind variants
    // -----------------------------------------------------------------------

    #[test]
    fn event_kind_all_variants_constructible() {
        let kinds = vec![
            EventKind::StateAssertion {
                assertion: "Tanya is sitting at the table".to_string(),
            },
            EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            EventKind::SpatialChange {
                from: Some("the clearing".to_string()),
                to: Some("the stream".to_string()),
            },
            EventKind::EmotionalExpression {
                emotion_hint: Some("grief".to_string()),
                intensity: 0.8,
            },
            EventKind::InformationTransfer {
                content_summary: "Adam reveals the hidden path".to_string(),
            },
            EventKind::SpeechAct {
                register: SpeechRegister::Conversational,
            },
            EventKind::RelationalShift {
                dimension: "trust_reliability".to_string(),
                delta: -0.3,
            },
            EventKind::EnvironmentalChange {
                description: "Rain begins to fall".to_string(),
            },
            EventKind::SceneLifecycle {
                lifecycle_type: SceneLifecycleType::Entered,
            },
            EventKind::EntityLifecycle {
                lifecycle_type: EntityLifecycleType::Promoted,
            },
        ];
        assert_eq!(kinds.len(), 10);
    }

    #[test]
    fn event_kind_serde_roundtrip() {
        let kind = EventKind::EmotionalExpression {
            emotion_hint: Some("joy".to_string()),
            intensity: 0.6,
        };
        let json = serde_json::to_string(&kind).expect("serialize");
        let deserialized: EventKind = serde_json::from_str(&json).expect("deserialize");
        if let EventKind::EmotionalExpression {
            emotion_hint,
            intensity,
        } = deserialized
        {
            assert_eq!(emotion_hint, Some("joy".to_string()));
            assert!((intensity - 0.6).abs() < f32::EPSILON);
        } else {
            panic!("expected EmotionalExpression variant");
        }
    }

    // -----------------------------------------------------------------------
    // ParticipantRole
    // -----------------------------------------------------------------------

    #[test]
    fn participant_role_all_variants_distinct() {
        let roles = [
            ParticipantRole::Actor,
            ParticipantRole::Target,
            ParticipantRole::Instrument,
            ParticipantRole::Location,
            ParticipantRole::Witness,
            ParticipantRole::Subject,
        ];
        for i in 0..roles.len() {
            for j in (i + 1)..roles.len() {
                assert_ne!(roles[i], roles[j]);
            }
        }
    }

    // -----------------------------------------------------------------------
    // ImplicationType
    // -----------------------------------------------------------------------

    #[test]
    fn implication_type_all_variants_constructible() {
        let types = vec![
            ImplicationType::Possession,
            ImplicationType::Proximity,
            ImplicationType::Attention,
            ImplicationType::EmotionalConnection { valence: 0.5 },
            ImplicationType::TrustSignal { direction: 0.3 },
            ImplicationType::InformationSharing,
            ImplicationType::Conflict,
            ImplicationType::Care,
            ImplicationType::Obligation { direction: -0.2 },
        ];
        assert_eq!(types.len(), 9);
    }

    #[test]
    fn implication_type_is_copy() {
        let a = ImplicationType::Care;
        let b = a;
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // EventSource
    // -----------------------------------------------------------------------

    #[test]
    fn event_source_all_variants_constructible() {
        let sources: Vec<EventSource> = vec![
            EventSource::PlayerInput {
                raw_input: "I pick up the stone".to_string(),
                classifier: ClassifierRef {
                    name: "keyword".to_string(),
                    version: "0.1".to_string(),
                },
            },
            EventSource::TurnExtraction {
                turn_id: TurnId::new(),
                classifier: ClassifierRef {
                    name: "deberta".to_string(),
                    version: "1.0".to_string(),
                },
            },
            EventSource::ConfirmedPrediction {
                character_id: EntityId::new(),
                prediction_confidence: 0.85,
            },
            EventSource::System {
                component: "scene_lifecycle".to_string(),
            },
            EventSource::Composed {
                source_atom_ids: vec![EventId::new(), EventId::new()],
            },
        ];
        assert_eq!(sources.len(), 5);
    }

    // -----------------------------------------------------------------------
    // EventConfidence
    // -----------------------------------------------------------------------

    #[test]
    fn event_confidence_range() {
        let conf = EventConfidence {
            value: 0.85,
            evidence: ConfidenceEvidence::SystemProduced,
        };
        assert!((0.0..=1.0).contains(&conf.value));
    }

    // -----------------------------------------------------------------------
    // EventAtom
    // -----------------------------------------------------------------------

    #[test]
    fn event_atom_full_construction() {
        let sarah_id = EntityId::new();
        let stone_ref = EntityRef::Unresolved {
            mention: "the stone".to_string(),
            context: ReferentialContext {
                descriptors: vec!["smooth".to_string()],
                spatial_context: Some("by the stream".to_string()),
                possessor: None,
                prior_mentions: vec![],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
        };

        let atom = EventAtom {
            id: EventId::new(),
            timestamp: Utc::now(),
            kind: EventKind::ActionOccurrence {
                action_type: ActionType::Perform,
            },
            participants: vec![
                Participant {
                    entity: EntityRef::Resolved(sarah_id),
                    role: ParticipantRole::Actor,
                },
                Participant {
                    entity: stone_ref,
                    role: ParticipantRole::Instrument,
                },
            ],
            relational_implications: vec![RelationalImplication {
                source: EntityRef::Resolved(sarah_id),
                target: EntityRef::Unresolved {
                    mention: "the stone".to_string(),
                    context: ReferentialContext {
                        descriptors: vec![],
                        spatial_context: None,
                        possessor: None,
                        prior_mentions: vec![],
                        first_mentioned_scene: SceneId::new(),
                        first_mentioned_turn: TurnId::new(),
                    },
                },
                implication_type: ImplicationType::Possession,
                weight: 0.3,
            }],
            source: EventSource::PlayerInput {
                raw_input: "I pick up the stone".to_string(),
                classifier: ClassifierRef {
                    name: "keyword".to_string(),
                    version: "0.1".to_string(),
                },
            },
            confidence: EventConfidence {
                value: 0.8,
                evidence: ConfidenceEvidence::ClassifierOutput {
                    classifier: "keyword".to_string(),
                    latency_ms: 2,
                },
            },
            priority: EventPriority::High,
            scene_id: SceneId::new(),
            turn_id: Some(TurnId::new()),
        };
        assert_eq!(atom.participants.len(), 2);
        assert_eq!(atom.relational_implications.len(), 1);
    }

    #[test]
    fn event_atom_serde_roundtrip() {
        let atom = EventAtom {
            id: EventId::new(),
            timestamp: Utc::now(),
            kind: EventKind::SceneLifecycle {
                lifecycle_type: SceneLifecycleType::Entered,
            },
            participants: vec![],
            relational_implications: vec![],
            source: EventSource::System {
                component: "scene_lifecycle".to_string(),
            },
            confidence: EventConfidence {
                value: 1.0,
                evidence: ConfidenceEvidence::SystemProduced,
            },
            priority: EventPriority::Immediate,
            scene_id: SceneId::new(),
            turn_id: None,
        };
        let json = serde_json::to_string(&atom).expect("serialize");
        let deserialized: EventAtom = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(atom.id, deserialized.id);
        assert_eq!(atom.priority, deserialized.priority);
    }

    // -----------------------------------------------------------------------
    // CompoundEvent
    // -----------------------------------------------------------------------

    #[test]
    fn compound_event_construction() {
        let compound = CompoundEvent {
            id: EventId::new(),
            atoms: vec![EventId::new(), EventId::new()],
            composition_type: CompositionType::Causal {
                mechanism: "seeing the chip triggered grief".to_string(),
            },
            emergent_implications: vec![],
            composition_confidence: 0.7,
            priority: EventPriority::High,
        };
        assert_eq!(compound.atoms.len(), 2);
    }

    #[test]
    fn composition_type_all_variants_constructible() {
        let types: Vec<CompositionType> = vec![
            CompositionType::Causal {
                mechanism: "cause".to_string(),
            },
            CompositionType::Temporal,
            CompositionType::Conditional {
                condition: "door opened".to_string(),
            },
            CompositionType::Thematic {
                theme: "loss".to_string(),
            },
        ];
        assert_eq!(types.len(), 4);
    }

    // -----------------------------------------------------------------------
    // TurnState
    // -----------------------------------------------------------------------

    #[test]
    fn turn_state_ordering() {
        assert!(TurnState::Hypothesized < TurnState::Rendered);
        assert!(TurnState::Rendered < TurnState::Committed);
    }

    // -----------------------------------------------------------------------
    // Turn
    // -----------------------------------------------------------------------

    #[test]
    fn turn_construction() {
        let turn = Turn {
            id: TurnId::new(),
            scene_id: SceneId::new(),
            turn_number: 1,
            state: TurnState::Hypothesized,
            created_at: Utc::now(),
            rendered_at: None,
            committed_at: None,
        };
        assert_eq!(turn.turn_number, 1);
        assert_eq!(turn.state, TurnState::Hypothesized);
    }

    // -----------------------------------------------------------------------
    // CommittedTurn
    // -----------------------------------------------------------------------

    #[test]
    fn committed_turn_construction() {
        let committed = CommittedTurn {
            turn_id: TurnId::new(),
            scene_id: SceneId::new(),
            turn_number: 3,
            narrator_prose: "The wolf watches from the ridge.".to_string(),
            player_response: "I look back at the wolf.".to_string(),
            prediction_metadata: vec![PredictionMetadata {
                character_id: EntityId::new(),
                predicted_action_type: Some(ActionType::Examine),
                predicted_speech: false,
                predicted_emotional_shift: true,
                confidence: 0.75,
            }],
            events_extracted: false,
        };
        assert!(!committed.events_extracted);
        assert_eq!(committed.prediction_metadata.len(), 1);
    }

    // -----------------------------------------------------------------------
    // PredictionMetadata
    // -----------------------------------------------------------------------

    #[test]
    fn prediction_metadata_without_action_type() {
        let meta = PredictionMetadata {
            character_id: EntityId::new(),
            predicted_action_type: None,
            predicted_speech: true,
            predicted_emotional_shift: false,
            confidence: 0.6,
        };
        assert!(meta.predicted_action_type.is_none());
        assert!(meta.predicted_speech);
    }

    // -----------------------------------------------------------------------
    // EventPayload
    // -----------------------------------------------------------------------

    #[test]
    fn event_payload_atom_variant() {
        let atom = EventAtom {
            id: EventId::new(),
            timestamp: Utc::now(),
            kind: EventKind::StateAssertion {
                assertion: "test".to_string(),
            },
            participants: vec![],
            relational_implications: vec![],
            source: EventSource::System {
                component: "test".to_string(),
            },
            confidence: EventConfidence {
                value: 1.0,
                evidence: ConfidenceEvidence::SystemProduced,
            },
            priority: EventPriority::Normal,
            scene_id: SceneId::new(),
            turn_id: None,
        };
        let payload = EventPayload::Atom(atom);
        assert!(matches!(payload, EventPayload::Atom(_)));
    }

    #[test]
    fn scene_lifecycle_type_all_variants_distinct() {
        let types = [
            SceneLifecycleType::Entered,
            SceneLifecycleType::Exited,
            SceneLifecycleType::CharacterArrival,
            SceneLifecycleType::CharacterDeparture,
        ];
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j]);
            }
        }
    }

    #[test]
    fn entity_lifecycle_type_all_variants_distinct() {
        let types = [
            EntityLifecycleType::Promoted,
            EntityLifecycleType::Demoted,
            EntityLifecycleType::Dissolved,
        ];
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j]);
            }
        }
    }
}
