// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Reference queries — genres and trope families.
//!
//! These are the simplest bedrock queries: no joins, no genre filtering,
//! just ordered lists of reference data from the `bedrock` schema.

use sqlx::PgPool;
use storyteller_core::errors::{StorytellerError, StorytellerResult};
use storyteller_core::types::bedrock::{GenreRecord, TropeFamilyRecord};

/// Return all genre records ordered by slug.
pub async fn genres(pool: &PgPool) -> StorytellerResult<Vec<GenreRecord>> {
    let records = sqlx::query_as::<_, GenreRecord>(
        "SELECT id, slug, name, description, payload, source_hash, created_at, updated_at \
         FROM bedrock.genres \
         ORDER BY slug",
    )
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}

/// Return all trope family records ordered by slug.
pub async fn trope_families(pool: &PgPool) -> StorytellerResult<Vec<TropeFamilyRecord>> {
    let records = sqlx::query_as::<_, TropeFamilyRecord>(
        "SELECT id, slug, name, description, created_at, updated_at \
         FROM bedrock.trope_families \
         ORDER BY slug",
    )
    .fetch_all(pool)
    .await
    .map_err(StorytellerError::Database)?;
    Ok(records)
}
