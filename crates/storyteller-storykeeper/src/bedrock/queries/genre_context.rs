// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Bulk genre context query — assembles all primitives for a genre in one pass.
//!
//! Calls the `bedrock.genre_context(slug)` SQL function which returns a single
//! JSONB value containing the genre record and all eleven primitive type arrays.
//! A NULL return means the genre slug was not found.

use sqlx::PgPool;
use storyteller_core::errors::{StorytellerError, StorytellerResult};
use storyteller_core::types::bedrock::GenreContext;

/// Fetch all bedrock primitives for one genre in a single SQL call.
///
/// Calls `bedrock.genre_context($1)` which returns NULL when the genre slug
/// is not found. Each primitive array in the returned JSONB maps 1:1 to the
/// corresponding `*Record` struct via serde deserialization.
///
/// The `genre_dimensions` key may be a bare object or null — the `GenreContext`
/// field is `Option<GenreDimensionRecord>` which handles both.
pub async fn genre_context(pool: &PgPool, genre_slug: &str) -> StorytellerResult<GenreContext> {
    let json: Option<serde_json::Value> = sqlx::query_scalar("SELECT bedrock.genre_context($1)")
        .bind(genre_slug)
        .fetch_one(pool)
        .await
        .map_err(StorytellerError::Database)?;

    let json = json.ok_or_else(|| {
        StorytellerError::EntityNotFound(format!("genre not found: {genre_slug}"))
    })?;

    // The SQL function uses the key "genre_dimensions"; GenreContext uses "dimensions".
    // Re-map the key before deserializing.
    let json = remap_dimensions_key(json);

    serde_json::from_value::<GenreContext>(json).map_err(StorytellerError::Serialization)
}

/// Rename the `genre_dimensions` key returned by the SQL function to `dimensions`
/// so it matches the `GenreContext` struct field.
///
/// Also normalises null primitive arrays to empty vecs by replacing JSON null
/// with an empty array `[]` for all array fields. The SQL function returns null
/// for a `jsonb_agg(...)` when there are no rows, but serde will fail to
/// deserialize null as `Vec<T>`.
fn remap_dimensions_key(mut json: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = json.as_object_mut() {
        // Move genre_dimensions → dimensions
        if let Some(dims) = obj.remove("genre_dimensions") {
            obj.insert("dimensions".to_string(), dims);
        }

        // Null jsonb_agg → empty array for every array field
        for key in &[
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
        ] {
            let entry = obj.entry(*key).or_insert(serde_json::Value::Null);
            if entry.is_null() {
                *entry = serde_json::Value::Array(vec![]);
            }
        }
    }
    json
}
