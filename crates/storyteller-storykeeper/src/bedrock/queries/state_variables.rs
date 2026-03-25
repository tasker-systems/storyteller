// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! State variable queries — definitions and genre interactions.

use sqlx::PgPool;
use storyteller_core::errors::{StorytellerError, StorytellerResult};
use storyteller_core::types::bedrock::{StateVariableInteractionRecord, StateVariableRecord};

/// Return all state variable definitions, ordered by slug.
pub async fn state_variables(pool: &PgPool) -> StorytellerResult<Vec<StateVariableRecord>> {
    let records = sqlx::query_as::<_, StateVariableRecord>(
        "SELECT id, slug, name, description, default_range, payload, created_at, updated_at
         FROM bedrock.state_variables ORDER BY slug",
    )
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return state variable interactions for a given genre and state variable slug.
///
/// Uses a polymorphic filter across all primitive tables that may carry
/// state variable interactions. Verbose but avoids dynamic SQL — PostgreSQL
/// will optimize the repeated genre subquery.
pub async fn state_variable_interactions(
    pool: &PgPool,
    genre_slug: &str,
    state_variable_slug: &str,
) -> StorytellerResult<Vec<StateVariableInteractionRecord>> {
    let records = sqlx::query_as::<_, StateVariableInteractionRecord>(
        "SELECT sv.slug AS state_variable_slug, sv.name AS state_variable_name,
                psvi.operation, psvi.context, psvi.primitive_table, psvi.primitive_id
         FROM bedrock.primitive_state_variable_interactions psvi
         JOIN bedrock.state_variables sv ON psvi.state_variable_id = sv.id
         WHERE sv.slug = $2
           AND (
             (psvi.primitive_table = 'archetypes' AND psvi.primitive_id IN (SELECT id FROM bedrock.archetypes WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'dynamics' AND psvi.primitive_id IN (SELECT id FROM bedrock.dynamics WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'goals' AND psvi.primitive_id IN (SELECT id FROM bedrock.goals WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'tropes' AND psvi.primitive_id IN (SELECT id FROM bedrock.tropes WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'settings' AND psvi.primitive_id IN (SELECT id FROM bedrock.settings WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'profiles' AND psvi.primitive_id IN (SELECT id FROM bedrock.profiles WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'spatial_topology' AND psvi.primitive_id IN (SELECT id FROM bedrock.spatial_topology WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'narrative_shapes' AND psvi.primitive_id IN (SELECT id FROM bedrock.narrative_shapes WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'ontological_posture' AND psvi.primitive_id IN (SELECT id FROM bedrock.ontological_posture WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'place_entities' AND psvi.primitive_id IN (SELECT id FROM bedrock.place_entities WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
             OR (psvi.primitive_table = 'archetype_dynamics' AND psvi.primitive_id IN (SELECT id FROM bedrock.archetype_dynamics WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1) AND cluster_id IS NULL))
           )
         ORDER BY psvi.primitive_table",
    )
    .bind(genre_slug)
    .bind(state_variable_slug)
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}
