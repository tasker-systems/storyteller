//! Character tensor types — multidimensional personality representation.
//!
//! See: `docs/technical/tensor-schema-spec.md`, `docs/technical/tensor-case-study-sarah.md`
//!
//! Design decision: Tensor values are `[central_tendency, variance, range_low, range_high]`
//! tuples on a [-1.0, 1.0] scale. Contextual triggers shift axes conditionally.
//! Three temporal layers (topsoil, sediment, bedrock) plus primordial for
//! ancient/non-human entities.

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
