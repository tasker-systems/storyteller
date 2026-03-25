// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! By-slug queries — fetch a single primitive by genre + entity slug.
//!
//! Each function returns `Option<T>` — `None` when no matching record exists.
//! Joins to `bedrock.genres` on the genre slug and filters to
//! `cluster_id IS NULL` for genre-specific rows only.

use sqlx::PgPool;
use storyteller_core::errors::{StorytellerError, StorytellerResult};
use storyteller_core::types::bedrock::{
    ArchetypeRecord, DynamicRecord, GoalRecord, NarrativeShapeRecord, OntologicalPostureRecord,
    PlaceEntityRecord, ProfileRecord, SettingRecord, SpatialTopologyRecord, TropeRecord,
};

/// Fetch a single archetype by genre slug and entity slug.
pub async fn archetype_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<ArchetypeRecord>> {
    let record = sqlx::query_as::<_, ArchetypeRecord>(
        "SELECT a.id, a.genre_id, a.cluster_id, a.entity_slug, a.name,
                a.payload, a.source_hash, a.created_at, a.updated_at,
                a.archetype_family, a.primary_scale
         FROM bedrock.archetypes a
         JOIN bedrock.genres g ON a.genre_id = g.id
         WHERE g.slug = $1 AND a.entity_slug = $2 AND a.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single dynamic by genre slug and entity slug.
pub async fn dynamic_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<DynamicRecord>> {
    let record = sqlx::query_as::<_, DynamicRecord>(
        "SELECT d.id, d.genre_id, d.cluster_id, d.entity_slug, d.name,
                d.payload, d.source_hash, d.created_at, d.updated_at,
                d.edge_type, d.scale
         FROM bedrock.dynamics d
         JOIN bedrock.genres g ON d.genre_id = g.id
         WHERE g.slug = $1 AND d.entity_slug = $2 AND d.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single setting by genre slug and entity slug.
pub async fn setting_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<SettingRecord>> {
    let record = sqlx::query_as::<_, SettingRecord>(
        "SELECT s.id, s.genre_id, s.cluster_id, s.entity_slug, s.name,
                s.payload, s.source_hash, s.created_at, s.updated_at,
                s.setting_type
         FROM bedrock.settings s
         JOIN bedrock.genres g ON s.genre_id = g.id
         WHERE g.slug = $1 AND s.entity_slug = $2 AND s.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single goal by genre slug and entity slug.
pub async fn goal_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<GoalRecord>> {
    let record = sqlx::query_as::<_, GoalRecord>(
        "SELECT gl.id, gl.genre_id, gl.cluster_id, gl.entity_slug, gl.name,
                gl.payload, gl.source_hash, gl.created_at, gl.updated_at,
                gl.goal_scale
         FROM bedrock.goals gl
         JOIN bedrock.genres g ON gl.genre_id = g.id
         WHERE g.slug = $1 AND gl.entity_slug = $2 AND gl.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single profile by genre slug and entity slug.
pub async fn profile_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<ProfileRecord>> {
    let record = sqlx::query_as::<_, ProfileRecord>(
        "SELECT pr.id, pr.genre_id, pr.cluster_id, pr.entity_slug, pr.name,
                pr.payload, pr.source_hash, pr.created_at, pr.updated_at
         FROM bedrock.profiles pr
         JOIN bedrock.genres g ON pr.genre_id = g.id
         WHERE g.slug = $1 AND pr.entity_slug = $2 AND pr.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single trope by genre slug and entity slug.
pub async fn trope_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<TropeRecord>> {
    let record = sqlx::query_as::<_, TropeRecord>(
        "SELECT t.id, t.genre_id, t.cluster_id, t.entity_slug, t.name,
                t.payload, t.source_hash, t.created_at, t.updated_at,
                t.trope_family_id
         FROM bedrock.tropes t
         JOIN bedrock.genres g ON t.genre_id = g.id
         WHERE g.slug = $1 AND t.entity_slug = $2 AND t.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single narrative shape by genre slug and entity slug.
pub async fn narrative_shape_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<NarrativeShapeRecord>> {
    let record = sqlx::query_as::<_, NarrativeShapeRecord>(
        "SELECT ns.id, ns.genre_id, ns.cluster_id, ns.entity_slug, ns.name,
                ns.payload, ns.source_hash, ns.created_at, ns.updated_at,
                ns.shape_type, ns.beat_count
         FROM bedrock.narrative_shapes ns
         JOIN bedrock.genres g ON ns.genre_id = g.id
         WHERE g.slug = $1 AND ns.entity_slug = $2 AND ns.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single ontological posture record by genre slug and entity slug.
pub async fn ontological_posture_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<OntologicalPostureRecord>> {
    let record = sqlx::query_as::<_, OntologicalPostureRecord>(
        "SELECT op.id, op.genre_id, op.cluster_id, op.entity_slug, op.name,
                op.payload, op.source_hash, op.created_at, op.updated_at,
                op.boundary_stability
         FROM bedrock.ontological_posture op
         JOIN bedrock.genres g ON op.genre_id = g.id
         WHERE g.slug = $1 AND op.entity_slug = $2 AND op.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single spatial topology record by genre slug and entity slug.
pub async fn spatial_topology_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<SpatialTopologyRecord>> {
    let record = sqlx::query_as::<_, SpatialTopologyRecord>(
        "SELECT st.id, st.genre_id, st.cluster_id, st.entity_slug, st.name,
                st.payload, st.source_hash, st.created_at, st.updated_at,
                st.friction_type, st.directionality_type
         FROM bedrock.spatial_topology st
         JOIN bedrock.genres g ON st.genre_id = g.id
         WHERE g.slug = $1 AND st.entity_slug = $2 AND st.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Fetch a single place entity by genre slug and entity slug.
pub async fn place_entity_by_slug(
    pool: &PgPool,
    genre_slug: &str,
    entity_slug: &str,
) -> StorytellerResult<Option<PlaceEntityRecord>> {
    let record = sqlx::query_as::<_, PlaceEntityRecord>(
        "SELECT pe.id, pe.genre_id, pe.cluster_id, pe.entity_slug, pe.name,
                pe.payload, pe.source_hash, pe.created_at, pe.updated_at,
                pe.topological_role
         FROM bedrock.place_entities pe
         JOIN bedrock.genres g ON pe.genre_id = g.id
         WHERE g.slug = $1 AND pe.entity_slug = $2 AND pe.cluster_id IS NULL",
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}
