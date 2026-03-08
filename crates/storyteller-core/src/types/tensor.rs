//! Character tensor types — multidimensional personality representation.
//!
//! See: `docs/technical/tensor-schema-spec.md`, `docs/technical/tensor-case-study-sarah.md`,
//!      `docs/foundation/emotional-model.md`
//!
//! Design decision: Tensor values are `[central_tendency, variance, range_low, range_high]`
//! tuples on a [-1.0, 1.0] scale. Contextual triggers shift axes conditionally.
//! Three temporal layers (topsoil, sediment, bedrock) plus primordial for
//! ancient/non-human entities.
//!
//! Awareness levels are orthogonal to temporal layers — a bedrock trait can be
//! Articulate, a topsoil emotion can be Defended. This determines how the
//! frame computation surfaces emotional data in LLM prompts.

/// A single tensor axis value with statistical distribution.
///
/// Represents not a point but a *tendency* — how a character tends to behave
/// along this axis, how variable they are, and what their observed range has been.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AxisValue {
    /// Where the character typically falls on this axis. Range: [-1.0, 1.0].
    pub central_tendency: f32,
    /// How much the character varies from their tendency. Range: [0.0, 1.0].
    pub variance: f32,
    /// Lowest observed/plausible value. Range: [-1.0, 1.0].
    pub range_low: f32,
    /// Highest observed/plausible value. Range: [-1.0, 1.0].
    pub range_high: f32,
}

/// Temporal layer for tensor values — geological model of identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TemporalLayer {
    /// Recent, volatile — changes within a scene. Half-life: one scene.
    Topsoil,
    /// Settled patterns — changes over months/years of story time.
    Sediment,
    /// Core identity — rarely changes, and only through profound transformation.
    Bedrock,
    /// Deep time — for ancient or non-human entities with geological/mythic timescales.
    Primordial,
}

/// Provenance tracking for any authored, inferred, or generated value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Provenance {
    /// Explicitly set by the story designer.
    Authored,
    /// Inferred from other authored data.
    Inferred,
    /// Procedurally generated.
    Generated,
    /// Reviewed and confirmed by a human.
    Confirmed,
    /// Overridden from a previous value.
    Overridden,
}

/// How conscious a character is of a given emotional state or trait.
///
/// Orthogonal to temporal layer — a bedrock value can be Articulate,
/// a topsoil emotion can be Defended. Determines how frame computation
/// surfaces data in LLM prompts: Articulate states can be directly
/// stated; Structural ones shape the frame without appearing in content.
///
/// See: `docs/foundation/emotional-model.md` § Awareness as Orthogonal Dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AwarenessLevel {
    /// Character can name and discuss this state. Frame may include direct statement.
    Articulate,
    /// Character would recognize this if pointed out. Frame includes it but not foregrounded.
    Recognizable,
    /// Manifests as behavioral tendency without conscious recognition.
    /// Frame encodes as behavioral direction, not named emotion.
    Preconscious,
    /// Character actively avoids recognizing this state. Appears as absence,
    /// compensation, or redirection. Frame must not make character more
    /// self-aware than they are.
    Defended,
    /// Shapes perception and behavior without being available to introspection.
    /// Frame encodes structurally (what the character notices, how they interpret)
    /// without appearing as emotional content.
    Structural,
}
