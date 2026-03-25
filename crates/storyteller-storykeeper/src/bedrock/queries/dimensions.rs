// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Dimensional queries — cross-primitive analysis by narrative dimension.

use sqlx::PgPool;
use storyteller_core::errors::{StorytellerError, StorytellerResult};
use storyteller_core::types::bedrock::{DimensionValueRecord, GenreDimensionRecord};

/// Allowlist of valid primitive table names to prevent SQL injection in
/// `dimensions_for_entity`, which must interpolate the table name.
const VALID_TABLES: &[&str] = &[
    "archetypes",
    "dynamics",
    "settings",
    "goals",
    "profiles",
    "tropes",
    "narrative_shapes",
    "ontological_posture",
    "spatial_topology",
    "place_entities",
    "archetype_dynamics",
];

/// Return the genre-level dimension record for the given genre slug.
pub async fn genre_dimensions(
    pool: &PgPool,
    genre_slug: &str,
) -> StorytellerResult<Option<GenreDimensionRecord>> {
    let record = sqlx::query_as::<_, GenreDimensionRecord>(
        "SELECT gd.id, gd.genre_id, gd.payload, gd.source_hash, gd.created_at, gd.updated_at
         FROM bedrock.genre_dimensions gd
         JOIN bedrock.genres g ON gd.genre_id = g.id
         WHERE g.slug = $1",
    )
    .bind(genre_slug)
    .fetch_optional(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(record)
}

/// Return all dimension values for a specific entity identified by primitive
/// table, entity slug, and genre slug.
///
/// The `primitive_table` argument is validated against an allowlist before
/// interpolation to prevent SQL injection.
pub async fn dimensions_for_entity(
    pool: &PgPool,
    primitive_table: &str,
    entity_slug: &str,
    genre_slug: &str,
) -> StorytellerResult<Vec<DimensionValueRecord>> {
    if !VALID_TABLES.contains(&primitive_table) {
        return Err(StorytellerError::Config(format!(
            "invalid primitive_table '{}': must be one of {:?}",
            primitive_table, VALID_TABLES
        )));
    }

    let sql = format!(
        "SELECT dv.*
         FROM bedrock.dimension_values dv
         JOIN bedrock.genres g ON dv.genre_id = g.id
         WHERE dv.primitive_table = $1
           AND dv.primitive_id = (
             SELECT id FROM bedrock.{primitive_table}
             WHERE entity_slug = $3 AND genre_id = g.id AND cluster_id IS NULL
             LIMIT 1
           )
           AND g.slug = $2",
        primitive_table = primitive_table
    );

    let records = sqlx::query_as::<_, DimensionValueRecord>(&sql)
        .bind(primitive_table)
        .bind(genre_slug)
        .bind(entity_slug)
        .fetch_all(pool)
        .await
        .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all dimension value records for a given dimension slug within a genre.
pub async fn entities_by_dimension(
    pool: &PgPool,
    dimension_slug: &str,
    genre_slug: &str,
) -> StorytellerResult<Vec<DimensionValueRecord>> {
    let records = sqlx::query_as::<_, DimensionValueRecord>(
        "SELECT dv.id, dv.primitive_table, dv.primitive_id, dv.genre_id,
                dv.dimension_slug, dv.dimension_group, dv.value_type,
                dv.numeric_value, dv.categorical_value, dv.complex_value,
                dv.source_path, dv.tier, dv.created_at
         FROM bedrock.dimension_values dv
         JOIN bedrock.genres g ON dv.genre_id = g.id
         WHERE g.slug = $1 AND dv.dimension_slug = $2
         ORDER BY dv.primitive_table, dv.numeric_value DESC NULLS LAST",
    )
    .bind(genre_slug)
    .bind(dimension_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all dimension value records for entities that share ALL of the
/// requested dimension slugs within a genre.
pub async fn dimensional_intersection(
    pool: &PgPool,
    dimension_slugs: &[&str],
    genre_slug: &str,
) -> StorytellerResult<Vec<DimensionValueRecord>> {
    let slugs_owned: Vec<String> = dimension_slugs.iter().map(|s| s.to_string()).collect();
    let slug_count = slugs_owned.len() as i64;

    let records = sqlx::query_as::<_, DimensionValueRecord>(
        "SELECT dv.id, dv.primitive_table, dv.primitive_id, dv.genre_id,
                dv.dimension_slug, dv.dimension_group, dv.value_type,
                dv.numeric_value, dv.categorical_value, dv.complex_value,
                dv.source_path, dv.tier, dv.created_at
         FROM bedrock.dimension_values dv
         JOIN bedrock.genres g ON dv.genre_id = g.id
         WHERE g.slug = $1 AND dv.dimension_slug = ANY($2)
           AND (dv.primitive_table, dv.primitive_id) IN (
             SELECT dv2.primitive_table, dv2.primitive_id
             FROM bedrock.dimension_values dv2
             JOIN bedrock.genres g2 ON dv2.genre_id = g2.id
             WHERE g2.slug = $1 AND dv2.dimension_slug = ANY($2)
             GROUP BY dv2.primitive_table, dv2.primitive_id
             HAVING COUNT(DISTINCT dv2.dimension_slug) = $3
           )
         ORDER BY dv.primitive_table, dv.primitive_id",
    )
    .bind(genre_slug)
    .bind(&slugs_owned)
    .bind(slug_count)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}
