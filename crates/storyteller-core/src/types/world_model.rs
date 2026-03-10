//! World model types — spatial zones, environmental constraints, and character capabilities.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! The world model provides the Resolver with the physical and social rules
//! that constrain character actions. Narrative distance zones determine what
//! actions are available at each spatial/social proximity. Skills and attributes
//! feed the hidden RPG mechanics of the Resolver.

use std::collections::BTreeMap;

use super::entity::EntityId;
use super::tensor::Provenance;

// ---------------------------------------------------------------------------
// Narrative distance zones — spatial/social proximity model
// ---------------------------------------------------------------------------

/// Spatial and social proximity between entities within a scene.
///
/// Determines what actions are possible. A character in the Peripheral zone
/// cannot whisper to someone in the Intimate zone. The zones are not strictly
/// physical — they encode social proximity too.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum NarrativeDistanceZone {
    /// Close enough to touch — physical contact, whispered speech.
    Intimate,
    /// Normal conversation distance — face-to-face, full social interaction.
    Conversational,
    /// Aware of each other — can see, hear raised voices, wave.
    Awareness,
    /// At the edge of perception — might notice if looking, otherwise unaware.
    Peripheral,
    /// Not present in the scene — cannot interact at all.
    Absent,
}

impl NarrativeDistanceZone {
    /// Whether speech at the given register can reach from this zone.
    pub fn can_hear_speech(&self) -> bool {
        matches!(
            self,
            Self::Intimate | Self::Conversational | Self::Awareness
        )
    }

    /// Whether physical interaction is possible at this zone.
    pub fn can_touch(&self) -> bool {
        matches!(self, Self::Intimate)
    }

    /// Whether the entity is perceptible at all.
    pub fn is_perceptible(&self) -> bool {
        !matches!(self, Self::Absent)
    }
}

/// The distance between two entities in a scene.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DistanceEntry {
    /// One entity.
    pub entity_a: EntityId,
    /// Another entity.
    pub entity_b: EntityId,
    /// Current distance zone between them.
    pub zone: NarrativeDistanceZone,
}

// ---------------------------------------------------------------------------
// Environmental constraints
// ---------------------------------------------------------------------------

/// A constraint on what can happen in a scene, derived from the physical
/// and social environment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvironmentalConstraint {
    /// Human-readable name for this constraint.
    pub name: String,
    /// What this constraint means narratively.
    pub description: String,
    /// Which action types are affected. Empty = all actions.
    pub affected_action_types: Vec<String>,
}

// ---------------------------------------------------------------------------
// Genre constraints — structured rules for the arbitration system
// ---------------------------------------------------------------------------

/// A typed genre constraint — replaces freeform genre_physics strings
/// with structured rules the arbitration system can evaluate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GenreConstraint {
    /// A capability that does not exist in this world.
    Forbidden { capability: String, reason: String },
    /// A capability that exists with conditions.
    Conditional {
        capability: String,
        requires: Vec<String>,
    },
    /// A physical law override (e.g., low gravity, no sound in vacuum).
    PhysicsOverride { property: String, value: String },
}

/// A constraint violation explaining why an action is impossible.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstraintViolation {
    /// Which constraint was violated.
    pub constraint_name: String,
    /// Human-readable explanation for narrator context injection.
    pub description: String,
}

/// A condition that modifies a permitted action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionCondition {
    /// What must happen for the action to succeed.
    pub requirement: String,
    /// How this affects the graduated outcome.
    pub impact: String,
}

/// Result of an action possibility check.
///
/// The arbitration system returns this before action resolution.
/// `Permitted` and `Impossible` come from deterministic rules.
/// `Ambiguous` triggers the small LLM fallback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verdict")]
pub enum ActionPossibility {
    /// Action is permitted, possibly with conditions.
    Permitted { conditions: Vec<ActionCondition> },
    /// Action is impossible due to a constraint violation.
    Impossible { reason: ConstraintViolation },
    /// Rules engine cannot determine — needs LLM analysis.
    Ambiguous {
        known_constraints: Vec<EnvironmentalConstraint>,
        uncertainty: String,
    },
}

impl ActionPossibility {
    /// Returns true if the action is permitted.
    pub fn is_permitted(&self) -> bool {
        matches!(self, Self::Permitted { .. })
    }

    /// Returns true if the action is impossible.
    pub fn is_impossible(&self) -> bool {
        matches!(self, Self::Impossible { .. })
    }

    /// Returns true if the result is ambiguous.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, Self::Ambiguous { .. })
    }
}

// ---------------------------------------------------------------------------
// World model — the scene's physical and social rules
// ---------------------------------------------------------------------------

/// The world model for a scene — everything the Resolver needs to enforce
/// physical and social reality.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldModel {
    /// Genre physics constraints (e.g., "magic exists but is subtle").
    pub genre_physics: Vec<String>,
    /// Current distance zones between entities in the scene.
    pub spatial_zones: Vec<DistanceEntry>,
    /// Environmental constraints that affect action resolution.
    pub environmental_constraints: Vec<EnvironmentalConstraint>,
}

// ---------------------------------------------------------------------------
// Character capabilities — attributes and skills for the Resolver
// ---------------------------------------------------------------------------

/// A character attribute — a broad capability dimension.
///
/// Attributes are the foundation of the hidden RPG mechanics. They are
/// not exposed to the player or the Narrator — only the Resolver uses them
/// to determine action outcomes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Attribute {
    /// Attribute name (e.g., "Presence", "Insight", "Resilience").
    pub name: String,
    /// Base value. Range: [0.0, 1.0].
    pub base_value: f32,
    /// How this value was established.
    pub provenance: Provenance,
}

/// A character skill — a specific learned capability.
///
/// Skills are narrower than attributes. They have a primary attribute
/// that governs them and optional secondary attributes that contribute.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    /// Skill name (e.g., "Persuasion", "Music", "Herbalism").
    pub name: String,
    /// Brief description of what this skill covers.
    pub description: String,
    /// The primary attribute this skill draws from.
    pub primary_attribute: String,
    /// Secondary attributes that contribute at reduced weight.
    pub secondary_attributes: Vec<String>,
    /// Base skill value. Range: [0.0, 1.0].
    pub base_value: f32,
}

/// A character's complete capability profile — attributes and skills.
///
/// Stored on `CharacterSheet`, consumed by the Resolver.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CapabilityProfile {
    /// Broad capability dimensions.
    pub attributes: BTreeMap<String, Attribute>,
    /// Specific learned capabilities.
    pub skills: BTreeMap<String, Skill>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_zones_have_correct_affordances() {
        assert!(NarrativeDistanceZone::Intimate.can_touch());
        assert!(NarrativeDistanceZone::Intimate.can_hear_speech());
        assert!(!NarrativeDistanceZone::Conversational.can_touch());
        assert!(NarrativeDistanceZone::Conversational.can_hear_speech());
        assert!(!NarrativeDistanceZone::Awareness.can_touch());
        assert!(NarrativeDistanceZone::Awareness.can_hear_speech());
        assert!(!NarrativeDistanceZone::Peripheral.can_hear_speech());
        assert!(NarrativeDistanceZone::Peripheral.is_perceptible());
        assert!(!NarrativeDistanceZone::Absent.is_perceptible());
    }

    #[test]
    fn distance_zones_are_ordered() {
        assert!(NarrativeDistanceZone::Intimate < NarrativeDistanceZone::Conversational);
        assert!(NarrativeDistanceZone::Conversational < NarrativeDistanceZone::Awareness);
        assert!(NarrativeDistanceZone::Awareness < NarrativeDistanceZone::Peripheral);
        assert!(NarrativeDistanceZone::Peripheral < NarrativeDistanceZone::Absent);
    }

    #[test]
    fn world_model_is_constructible() {
        let model = WorldModel {
            genre_physics: vec![
                "Magic exists but is subtle — felt, not seen".to_string(),
                "Ley lines carry spiritual resonance".to_string(),
            ],
            spatial_zones: vec![DistanceEntry {
                entity_a: EntityId::new(),
                entity_b: EntityId::new(),
                zone: NarrativeDistanceZone::Conversational,
            }],
            environmental_constraints: vec![EnvironmentalConstraint {
                name: "Failing light".to_string(),
                description: "Late afternoon — visibility decreasing, scene has natural end"
                    .to_string(),
                affected_action_types: vec!["Examine".to_string()],
            }],
        };
        assert_eq!(model.genre_physics.len(), 2);
        assert_eq!(model.spatial_zones.len(), 1);
    }

    #[test]
    fn genre_constraint_forbidden_variant() {
        let constraint = GenreConstraint::Forbidden {
            capability: "telekinesis".to_string(),
            reason: "Magic does not exist in this world".to_string(),
        };
        assert!(matches!(constraint, GenreConstraint::Forbidden { .. }));
    }

    #[test]
    fn genre_constraint_conditional_variant() {
        let constraint = GenreConstraint::Conditional {
            capability: "flight".to_string(),
            requires: vec!["wings".to_string(), "open sky".to_string()],
        };
        if let GenreConstraint::Conditional { requires, .. } = &constraint {
            assert_eq!(requires.len(), 2);
        }
    }

    #[test]
    fn genre_constraint_physics_override() {
        let constraint = GenreConstraint::PhysicsOverride {
            property: "gravity".to_string(),
            value: "0.3g".to_string(),
        };
        assert!(matches!(
            constraint,
            GenreConstraint::PhysicsOverride { .. }
        ));
    }

    #[test]
    fn action_possibility_permitted() {
        let result = ActionPossibility::Permitted { conditions: vec![] };
        assert!(result.is_permitted());
        assert!(!result.is_impossible());
        assert!(!result.is_ambiguous());
    }

    #[test]
    fn action_possibility_impossible() {
        let result = ActionPossibility::Impossible {
            reason: ConstraintViolation {
                constraint_name: "genre_physics".to_string(),
                description: "Magic does not exist".to_string(),
            },
        };
        assert!(result.is_impossible());
        assert!(!result.is_permitted());
    }

    #[test]
    fn action_possibility_ambiguous() {
        let result = ActionPossibility::Ambiguous {
            known_constraints: vec![],
            uncertainty: "Low gravity leap height unclear".to_string(),
        };
        assert!(result.is_ambiguous());
        assert!(!result.is_permitted());
    }

    #[test]
    fn genre_constraint_serializes() {
        let constraint = GenreConstraint::Forbidden {
            capability: "telekinesis".to_string(),
            reason: "No magic".to_string(),
        };
        let json = serde_json::to_string(&constraint).unwrap();
        let roundtrip: GenreConstraint = serde_json::from_str(&json).unwrap();
        assert!(matches!(roundtrip, GenreConstraint::Forbidden { .. }));
    }

    #[test]
    fn capability_profile_is_constructible() {
        let mut profile = CapabilityProfile::default();
        profile.attributes.insert(
            "presence".to_string(),
            Attribute {
                name: "Presence".to_string(),
                base_value: 0.8,
                provenance: Provenance::Authored,
            },
        );
        profile.skills.insert(
            "music".to_string(),
            Skill {
                name: "Music".to_string(),
                description: "Performance, composition, and musical improvisation".to_string(),
                primary_attribute: "presence".to_string(),
                secondary_attributes: vec!["insight".to_string()],
                base_value: 0.9,
            },
        );
        assert_eq!(profile.attributes.len(), 1);
        assert_eq!(profile.skills.len(), 1);
    }
}
