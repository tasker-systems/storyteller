//! Character tensor and sheet types — everything needed to instantiate a character agent.
//!
//! See: `docs/technical/tensor-schema-spec.md`, `docs/workshop/character-bramblehoof.md`
//!
//! The `CharacterTensor` is a flexible map from axis names to tensor entries,
//! rather than a fixed struct with 20 fields. This lets each character define
//! whatever axes their sheet needs. `CharacterSheet` bundles the tensor with
//! backstory, triggers, and performance guidance for agent instantiation.

use std::collections::BTreeMap;

use super::entity::EntityId;
use super::scene::SceneId;
use super::tensor::{AxisValue, Provenance, TemporalLayer};

/// A single entry in a character tensor — value, temporal layer, and provenance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TensorEntry {
    /// The statistical distribution for this axis.
    pub value: AxisValue,
    /// Which temporal layer this value belongs to.
    pub layer: TemporalLayer,
    /// How this value was established.
    pub provenance: Provenance,
}

/// A character's full personality tensor — flexible map from axis names to entries.
///
/// Not a fixed struct: each character defines whatever axes their sheet needs.
/// The tensor is the "who this character is" data that gets translated into
/// LLM system prompts by the agent architecture.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterTensor {
    /// Axis name → tensor entry. Uses BTreeMap for deterministic ordering.
    pub axes: BTreeMap<String, TensorEntry>,
}

impl CharacterTensor {
    /// Create an empty tensor.
    pub fn new() -> Self {
        Self {
            axes: BTreeMap::new(),
        }
    }

    /// Insert an axis entry.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: AxisValue,
        layer: TemporalLayer,
        provenance: Provenance,
    ) {
        self.axes.insert(
            name.into(),
            TensorEntry {
                value,
                layer,
                provenance,
            },
        );
    }

    /// Look up an axis by name.
    pub fn get(&self, name: &str) -> Option<&TensorEntry> {
        self.axes.get(name)
    }
}

impl Default for CharacterTensor {
    fn default() -> Self {
        Self::new()
    }
}

/// A contextual trigger that shifts tensor axes when conditions are met.
///
/// This is the simple prototype version — a human-readable trigger description
/// paired with axis shifts. The full set-theoretic composition (All/Any/Not)
/// is deferred.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextualTrigger {
    /// Human-readable description of what activates this trigger.
    pub description: String,
    /// Which axes shift and by how much when triggered.
    pub axis_shifts: Vec<AxisShift>,
    /// Rough magnitude category for the overall trigger.
    pub magnitude: TriggerMagnitude,
}

/// A single axis shift within a trigger.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxisShift {
    /// Name of the axis to shift (must match a key in CharacterTensor).
    pub axis: String,
    /// How much to shift the central tendency. Positive = increase.
    pub shift: f32,
}

/// Rough magnitude category for contextual triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TriggerMagnitude {
    /// Subtle shift, background influence.
    Low,
    /// Noticeable but not scene-defining.
    Medium,
    /// Defines or redirects the scene's emotional arc.
    High,
}

/// Everything a character agent needs at scene entry.
///
/// Bundles identity, tensor, backstory, triggers, and performance guidance.
/// For the prototype, these are hardcoded as Rust struct literals.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterSheet {
    /// Entity identity.
    pub entity_id: EntityId,
    /// Display name.
    pub name: String,
    /// Voice register description — how this character sounds.
    pub voice: String,
    /// Full backstory text provided to the agent.
    pub backstory: String,
    /// The character's personality tensor.
    pub tensor: CharacterTensor,
    /// Scene-specific contextual triggers.
    pub triggers: Vec<ContextualTrigger>,
    /// Performance notes — guidance for the LLM on *how* to play this character.
    pub performance_notes: String,
    /// Information boundary: what this character knows.
    pub knows: Vec<String>,
    /// Information boundary: what this character must NOT know.
    pub does_not_know: Vec<String>,
}

/// Extended scene representation with everything needed to run a scene.
///
/// For the prototype, a single struct holding setting, cast, stakes,
/// constraints, and emotional arc. Hardcoded for "The Flute Kept".
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneData {
    /// Scene identifier.
    pub scene_id: SceneId,
    /// Human-readable scene title.
    pub title: String,
    /// Scene classification.
    pub scene_type: super::scene::SceneType,
    /// Setting description for the World Agent / Narrator.
    pub setting: SceneSetting,
    /// Characters in this scene.
    pub cast: Vec<CastEntry>,
    /// What is at stake in this scene.
    pub stakes: Vec<String>,
    /// Hard, soft, and perceptual constraints.
    pub constraints: SceneConstraints,
    /// Emotional arc notes — guidance for the overall shape of the scene.
    pub emotional_arc: Vec<String>,
    /// Evaluation criteria — what "success" looks like for this scene.
    pub evaluation_criteria: Vec<String>,
}

/// Physical and sensory setting for a scene.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneSetting {
    /// Where and when.
    pub description: String,
    /// Physical affordances — what the space allows.
    pub affordances: Vec<String>,
    /// Sensory details for the World Agent.
    pub sensory_details: Vec<String>,
    /// One aesthetic detail that matters.
    pub aesthetic_detail: String,
}

/// A character in the scene's cast list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CastEntry {
    /// Entity ID of this cast member.
    pub entity_id: EntityId,
    /// Name for reference.
    pub name: String,
    /// Role in this scene.
    pub role: String,
}

/// Scene constraints organized by type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneConstraints {
    /// Hard constraints — enforced by the World Agent, cannot be violated.
    pub hard: Vec<String>,
    /// Soft constraints — character capacity, may bend under pressure.
    pub soft: Vec<String>,
    /// Perceptual constraints — what can be sensed or inferred.
    pub perceptual: Vec<String>,
}
