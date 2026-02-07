//! Character tensor and sheet types — everything needed to instantiate a character agent.
//!
//! See: `docs/technical/tensor-schema-spec.md`, `docs/workshop/character-bramblehoof.md`,
//!      `docs/foundation/emotional-model.md`
//!
//! The `CharacterTensor` is a flexible map from axis names to tensor entries,
//! rather than a fixed struct with 20 fields. This lets each character define
//! whatever axes their sheet needs. `CharacterSheet` bundles the tensor with
//! backstory, triggers, and performance guidance for agent instantiation.
//!
//! The emotional model adds three concepts on top of the personality tensor:
//! - **Emotional state**: Plutchik primary intensities with awareness levels,
//!   separate from personality axes. These are the character's current emotional
//!   condition entering a scene.
//! - **Self-referential edge**: The character's relationship to themselves,
//!   using the same substrate dimensions as inter-entity edges.
//! - **Emotional grammar**: A pluggable vocabulary of primary emotions.
//!   For the prototype, only `plutchik_western` is implemented.

use std::collections::BTreeMap;

use super::entity::EntityId;
use super::scene::SceneId;
use super::tensor::{AwarenessLevel, AxisValue, Provenance, TemporalLayer};

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
///
/// # Prototype approach (intentionally naive)
///
/// In the current prototype, the full tensor is serialized into the LLM system
/// prompt and the model interprets axis names and values based on scene context.
/// This works because axis names are semantically legible ("grief",
/// "righteous_anger") and LLMs are good at contextual interpretation.
///
/// The production architecture will replace this with **frame computation** —
/// an ML inference layer (ONNX/ort) that reads tensor + relational web +
/// scene context to produce a compressed psychological frame (~200-400 tokens)
/// selecting which axes are relevant and how they interact. The Character Agent
/// then receives the frame, not the raw tensor.
///
/// Three operational moments where tensor data matters:
/// 1. **Scene entry**: Full tensor loaded, frame computed
/// 2. **Frame computation**: Activated subset selected based on scene context
/// 3. **Contextual triggers**: Specific axes shifted by classified events
///
/// See: `docs/foundation/emotional-model.md`, `docs/technical/tensor-schema-spec.md`
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

// ---------------------------------------------------------------------------
// Emotional state — Plutchik primary mapping with awareness annotations
// ---------------------------------------------------------------------------

/// A single primary emotion intensity with awareness annotation.
///
/// Part of the emotional grammar system. For `plutchik_western`, the primary_id
/// is one of: joy, sadness, trust, disgust, fear, anger, surprise, anticipation.
/// Intensity is on [0.0, 1.0]. Awareness determines how the frame computation
/// surfaces this state in the LLM prompt.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionalPrimary {
    /// Which primary this is (grammar-relative). E.g. "joy", "sadness".
    pub primary_id: String,
    /// Current intensity entering this scene. Range: [0.0, 1.0].
    pub intensity: f32,
    /// How conscious the character is of this emotional state.
    pub awareness: AwarenessLevel,
}

/// A character's emotional state entering a scene — primary intensities
/// plus authored mood-vector descriptions.
///
/// This is separate from the personality tensor. The tensor captures
/// *who the character is* (axes like "empathy", "practical_focus").
/// The emotional state captures *how the character feels right now*
/// in terms of a specific emotional grammar.
///
/// Mood-vectors are co-activated primaries forming directional vectors.
/// For the prototype, these are authored natural-language descriptions.
/// Later, they could be computed from primary co-activation patterns.
///
/// See: `docs/foundation/emotional-model.md` § Mood-Vectors
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionalState {
    /// Which emotional grammar this state uses. E.g. "plutchik_western".
    pub grammar_id: String,
    /// Primary emotion intensities with awareness levels.
    pub primaries: Vec<EmotionalPrimary>,
    /// Authored descriptions of active mood-vectors — co-activated primaries
    /// and their felt quality. E.g. "sadness + anger → bewildered envy (0.6)".
    pub mood_vector_notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Self-referential edge — character's relationship to themselves
// ---------------------------------------------------------------------------

/// The character's relationship to themselves — a loopback edge using the
/// same conceptual substrate as inter-entity edges.
///
/// Uses simple f32 values rather than AxisValue distributions because
/// the self-edge is a snapshot assessment at scene entry, not a statistical
/// tendency.
///
/// See: `docs/foundation/emotional-model.md` § The Self-Referential Edge
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfEdge {
    /// Self-trust across three dimensions.
    pub trust: SelfEdgeTrust,
    /// How the character feels about themselves.
    pub affection: f32,
    /// What the character owes themselves or others through their self-concept.
    pub debt: f32,
    /// Recurring self-narrative pattern.
    pub history_pattern: String,
    /// How strongly this pattern weighs on the character. Range: [0.0, 1.0].
    pub history_weight: f32,
    /// What the character believes they are — their self-story.
    pub projection_content: String,
    /// How accurate this self-story is. Range: [0.0, 1.0].
    /// The gap between accuracy and 1.0 is where subtext lives.
    pub projection_accuracy: f32,
    /// What the character knows about themselves.
    pub self_knowledge: SelfKnowledge,
}

/// Three dimensions of self-trust.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SelfEdgeTrust {
    /// Do I believe I can do what's needed? Range: [0.0, 1.0].
    pub competence: f32,
    /// Do I trust my own desires and motives? Range: [0.0, 1.0].
    pub intentions: f32,
    /// Can I count on myself to show up? Range: [0.0, 1.0].
    pub reliability: f32,
}

/// What a character knows and doesn't know about themselves.
///
/// The does_not_know items carry implicit awareness annotations —
/// described in natural language for the prototype.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfKnowledge {
    /// What the character recognizes about themselves.
    pub knows: Vec<String>,
    /// What the character doesn't know about themselves.
    /// Each entry may include an awareness annotation in parentheses.
    pub does_not_know: Vec<String>,
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
    /// Emotional grammar identifier. E.g. "plutchik_western".
    pub grammar_id: String,
    /// Emotional state entering this scene — primary intensities and mood-vectors.
    pub emotional_state: EmotionalState,
    /// Self-referential edge — the character's relationship to themselves.
    pub self_edge: SelfEdge,
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
