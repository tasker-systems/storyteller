// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Record types for the bedrock narrative data layer.
//!
//! These structs map directly to database rows returned by `BedrockQuery`
//! implementations. Primitive records share a common shape (id, genre_id,
//! cluster_id, entity_slug, name, payload, source_hash, timestamps) with
//! type-specific promoted columns added per record.

use chrono::{DateTime, Utc};
use uuid::Uuid;

// ── Reference Records ────────────────────────────────────────────────────────

/// A narrative genre (e.g. "epic-fantasy", "noir-crime").
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct GenreRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A cluster grouping related genres.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct GenreClusterRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

/// A narrative state variable (e.g. "tension", "trust").
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct StateVariableRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub default_range: Option<serde_json::Value>,
    pub payload: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A family grouping related tropes.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct TropeFamilyRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A narrative dimension used for cross-primitive analysis.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct DimensionRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub dimension_group: String,
    pub description: Option<String>,
}

// ── Primitive Records ────────────────────────────────────────────────────────
//
// All primitive records share the common fields:
//   id, genre_id, cluster_id (Option), entity_slug, name,
//   payload (serde_json::Value), source_hash, created_at, updated_at
//
// Type-specific promoted columns are listed per struct.

/// A character archetype primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ArchetypeRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub archetype_family: Option<String>,
    pub primary_scale: Option<String>,
}

/// An interpersonal or narrative dynamic primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct DynamicRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub edge_type: Option<String>,
    pub scale: Option<String>,
}

/// A narrative setting primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct SettingRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub setting_type: Option<String>,
}

/// A character or story goal primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct GoalRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub goal_scale: Option<String>,
}

/// A character profile primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ProfileRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A narrative trope primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct TropeRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub trope_family_id: Option<Uuid>,
}

/// A narrative shape (story structure) primitive.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct NarrativeShapeRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub shape_type: Option<String>,
    pub beat_count: Option<i32>,
}

/// An ontological posture primitive (how entities relate to reality/existence).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct OntologicalPostureRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub boundary_stability: Option<String>,
}

/// A spatial topology primitive (how space is structured in the narrative).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct SpatialTopologyRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub friction_type: Option<String>,
    pub directionality_type: Option<String>,
}

/// A place entity primitive (a location with narrative presence).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct PlaceEntityRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub topological_role: Option<String>,
}

/// A cross-archetype dynamic (relationship pattern between two archetypes).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ArchetypeDynamicRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // promoted columns
    pub archetype_a: Option<String>,
    pub archetype_b: Option<String>,
}

/// Dimensional analysis data for an entire genre (special shape — no entity_slug/cluster_id/name).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct GenreDimensionRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Query Result Records ─────────────────────────────────────────────────────

/// A single dimension value attached to a primitive entity.
///
/// Returned by dimensional query methods; maps across multiple tables via
/// a shared view or union query.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct DimensionValueRecord {
    pub id: Uuid,
    pub primitive_table: String,
    pub primitive_id: Uuid,
    pub genre_id: Uuid,
    pub dimension_slug: String,
    pub dimension_group: String,
    pub value_type: String,
    pub numeric_value: Option<f32>,
    pub categorical_value: Option<String>,
    pub complex_value: Option<serde_json::Value>,
    pub source_path: Option<String>,
    pub tier: String,
    pub created_at: DateTime<Utc>,
}

/// The interaction between a state variable and a primitive entity.
///
/// This is a query result from a JOIN, not a direct table mapping.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct StateVariableInteractionRecord {
    pub state_variable_slug: String,
    pub state_variable_name: String,
    pub operation: Option<String>,
    pub context: Option<serde_json::Value>,
    pub primitive_table: String,
    pub primitive_id: Uuid,
}

// ── Envelope and Composite ───────────────────────────────────────────────────

/// Generic envelope wrapping a bedrock primitive with its identifying context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BedrockEntity<T: std::fmt::Debug + Clone> {
    pub id: uuid::Uuid,
    pub genre_slug: String,
    pub entity_slug: String,
    pub record: T,
}

/// All narrative primitives for a single genre, assembled for context injection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenreContext {
    pub genre: GenreRecord,
    pub archetypes: Vec<ArchetypeRecord>,
    pub dynamics: Vec<DynamicRecord>,
    pub settings: Vec<SettingRecord>,
    pub goals: Vec<GoalRecord>,
    pub profiles: Vec<ProfileRecord>,
    pub tropes: Vec<TropeRecord>,
    pub narrative_shapes: Vec<NarrativeShapeRecord>,
    pub ontological_posture: Vec<OntologicalPostureRecord>,
    pub spatial_topology: Vec<SpatialTopologyRecord>,
    pub place_entities: Vec<PlaceEntityRecord>,
    pub archetype_dynamics: Vec<ArchetypeDynamicRecord>,
    pub dimensions: Option<GenreDimensionRecord>,
}
