//! Tensor components — personality, motivation, and emotional state as ECS data.
//!
//! See: `docs/technical/tensor-schema-spec.md`, `docs/technical/tensor-case-study-sarah.md`
//!
//! These components attach tensor data to Bevy entities. The full character
//! tensor is decomposed into focused components that systems can query
//! independently.

use bevy_ecs::prelude::*;

/// Marker component indicating this entity has a full character tensor.
///
/// Entities with this marker have personality axes, motivations, and
/// emotional state components attached. Entities without it may still
/// have partial tensor data (e.g., a presence with just emotional state).
#[derive(Debug, Component)]
pub struct FullTensorMarker;
