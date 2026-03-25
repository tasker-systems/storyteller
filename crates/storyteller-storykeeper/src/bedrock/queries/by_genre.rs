// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! By-genre queries — return all primitives of a given type for one genre.
//!
//! Each function joins to `bedrock.genres` on the slug and filters to
//! `cluster_id IS NULL` so only genre-specific (non-cross-genre) rows are
//! returned.

use sqlx::PgPool;
use storyteller_core::errors::{StorytellerError, StorytellerResult};
use storyteller_core::types::bedrock::{
    ArchetypeDynamicRecord, ArchetypeRecord, DynamicRecord, GoalRecord, NarrativeShapeRecord,
    OntologicalPostureRecord, PlaceEntityRecord, ProfileRecord, SettingRecord,
    SpatialTopologyRecord, TropeRecord,
};

/// Return all archetypes for the given genre slug.
pub async fn archetypes_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<ArchetypeRecord>> {
    let records = sqlx::query_as::<_, ArchetypeRecord>(
        "SELECT a.id, a.genre_id, a.cluster_id, a.entity_slug, a.name,
                a.payload, a.source_hash, a.created_at, a.updated_at,
                a.archetype_family, a.primary_scale
         FROM bedrock.archetypes a
         JOIN bedrock.genres g ON a.genre_id = g.id
         WHERE g.slug = $1 AND a.cluster_id IS NULL
         ORDER BY a.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all dynamics for the given genre slug.
pub async fn dynamics_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<DynamicRecord>> {
    let records = sqlx::query_as::<_, DynamicRecord>(
        "SELECT d.id, d.genre_id, d.cluster_id, d.entity_slug, d.name,
                d.payload, d.source_hash, d.created_at, d.updated_at,
                d.edge_type, d.scale
         FROM bedrock.dynamics d
         JOIN bedrock.genres g ON d.genre_id = g.id
         WHERE g.slug = $1 AND d.cluster_id IS NULL
         ORDER BY d.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all settings for the given genre slug.
pub async fn settings_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<SettingRecord>> {
    let records = sqlx::query_as::<_, SettingRecord>(
        "SELECT s.id, s.genre_id, s.cluster_id, s.entity_slug, s.name,
                s.payload, s.source_hash, s.created_at, s.updated_at,
                s.setting_type
         FROM bedrock.settings s
         JOIN bedrock.genres g ON s.genre_id = g.id
         WHERE g.slug = $1 AND s.cluster_id IS NULL
         ORDER BY s.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all goals for the given genre slug.
pub async fn goals_by_genre(pool: &PgPool, genre_slug: &str) -> StorytellerResult<Vec<GoalRecord>> {
    let records = sqlx::query_as::<_, GoalRecord>(
        "SELECT gl.id, gl.genre_id, gl.cluster_id, gl.entity_slug, gl.name,
                gl.payload, gl.source_hash, gl.created_at, gl.updated_at,
                gl.goal_scale
         FROM bedrock.goals gl
         JOIN bedrock.genres g ON gl.genre_id = g.id
         WHERE g.slug = $1 AND gl.cluster_id IS NULL
         ORDER BY gl.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all profiles for the given genre slug.
pub async fn profiles_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<ProfileRecord>> {
    let records = sqlx::query_as::<_, ProfileRecord>(
        "SELECT pr.id, pr.genre_id, pr.cluster_id, pr.entity_slug, pr.name,
                pr.payload, pr.source_hash, pr.created_at, pr.updated_at
         FROM bedrock.profiles pr
         JOIN bedrock.genres g ON pr.genre_id = g.id
         WHERE g.slug = $1 AND pr.cluster_id IS NULL
         ORDER BY pr.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all tropes for the given genre slug.
pub async fn tropes_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<TropeRecord>> {
    let records = sqlx::query_as::<_, TropeRecord>(
        "SELECT t.id, t.genre_id, t.cluster_id, t.entity_slug, t.name,
                t.payload, t.source_hash, t.created_at, t.updated_at,
                t.trope_family_id
         FROM bedrock.tropes t
         JOIN bedrock.genres g ON t.genre_id = g.id
         WHERE g.slug = $1 AND t.cluster_id IS NULL
         ORDER BY t.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all narrative shapes for the given genre slug.
pub async fn narrative_shapes_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<NarrativeShapeRecord>> {
    let records = sqlx::query_as::<_, NarrativeShapeRecord>(
        "SELECT ns.id, ns.genre_id, ns.cluster_id, ns.entity_slug, ns.name,
                ns.payload, ns.source_hash, ns.created_at, ns.updated_at,
                ns.shape_type, ns.beat_count
         FROM bedrock.narrative_shapes ns
         JOIN bedrock.genres g ON ns.genre_id = g.id
         WHERE g.slug = $1 AND ns.cluster_id IS NULL
         ORDER BY ns.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all ontological posture records for the given genre slug.
pub async fn ontological_posture_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<OntologicalPostureRecord>> {
    let records = sqlx::query_as::<_, OntologicalPostureRecord>(
        "SELECT op.id, op.genre_id, op.cluster_id, op.entity_slug, op.name,
                op.payload, op.source_hash, op.created_at, op.updated_at,
                op.boundary_stability
         FROM bedrock.ontological_posture op
         JOIN bedrock.genres g ON op.genre_id = g.id
         WHERE g.slug = $1 AND op.cluster_id IS NULL
         ORDER BY op.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all spatial topology records for the given genre slug.
pub async fn spatial_topology_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<SpatialTopologyRecord>> {
    let records = sqlx::query_as::<_, SpatialTopologyRecord>(
        "SELECT st.id, st.genre_id, st.cluster_id, st.entity_slug, st.name,
                st.payload, st.source_hash, st.created_at, st.updated_at,
                st.friction_type, st.directionality_type
         FROM bedrock.spatial_topology st
         JOIN bedrock.genres g ON st.genre_id = g.id
         WHERE g.slug = $1 AND st.cluster_id IS NULL
         ORDER BY st.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all place entities for the given genre slug.
pub async fn place_entities_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<PlaceEntityRecord>> {
    let records = sqlx::query_as::<_, PlaceEntityRecord>(
        "SELECT pe.id, pe.genre_id, pe.cluster_id, pe.entity_slug, pe.name,
                pe.payload, pe.source_hash, pe.created_at, pe.updated_at,
                pe.topological_role
         FROM bedrock.place_entities pe
         JOIN bedrock.genres g ON pe.genre_id = g.id
         WHERE g.slug = $1 AND pe.cluster_id IS NULL
         ORDER BY pe.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all archetype dynamics for the given genre slug.
pub async fn archetype_dynamics_by_genre(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Vec<ArchetypeDynamicRecord>> {
    let records = sqlx::query_as::<_, ArchetypeDynamicRecord>(
        "SELECT ad.id, ad.genre_id, ad.cluster_id, ad.entity_slug, ad.name,
                ad.payload, ad.source_hash, ad.created_at, ad.updated_at,
                ad.archetype_a, ad.archetype_b
         FROM bedrock.archetype_dynamics ad
         JOIN bedrock.genres g ON ad.genre_id = g.id
         WHERE g.slug = $1 AND ad.cluster_id IS NULL
         ORDER BY ad.entity_slug",
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}
