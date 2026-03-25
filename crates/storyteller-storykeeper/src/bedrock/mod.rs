// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! PostgreSQL-backed `BedrockQuery` implementation.
//!
//! `PostgresBedrock` wraps a `sqlx::PgPool` and implements the full
//! `BedrockQuery` trait against the `bedrock` schema. Query logic is
//! split into sub-modules by access pattern.
//!
//! See: `docs/technical/storykeeper-api-contract.md`

pub mod queries;

use sqlx::PgPool;
use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::bedrock::BedrockQuery;
use storyteller_core::types::bedrock::{
    ArchetypeDynamicRecord, ArchetypeRecord, DimensionValueRecord, DynamicRecord, GenreContext,
    GenreDimensionRecord, GenreRecord, GoalRecord, NarrativeShapeRecord, OntologicalPostureRecord,
    PlaceEntityRecord, ProfileRecord, SettingRecord, SpatialTopologyRecord,
    StateVariableInteractionRecord, StateVariableRecord, TropeFamilyRecord, TropeRecord,
};

/// PostgreSQL-backed implementation of `BedrockQuery`.
///
/// Wraps a connection pool and delegates each query to the appropriate
/// sub-module in `queries/`. The pool is cheaply cloneable — each
/// `PostgresBedrock` instance shares the same underlying connection pool.
#[derive(Debug, Clone)]
pub struct PostgresBedrock {
    pool: PgPool,
}

impl PostgresBedrock {
    /// Create a new `PostgresBedrock` from an existing connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BedrockQuery for PostgresBedrock {
    // ── Reference (implemented) ───────────────────────────────────────────────

    async fn genres(&self) -> StorytellerResult<Vec<GenreRecord>> {
        queries::reference::genres(&self.pool).await
    }

    async fn trope_families(&self) -> StorytellerResult<Vec<TropeFamilyRecord>> {
        queries::reference::trope_families(&self.pool).await
    }

    // ── Bulk ─────────────────────────────────────────────────────────────────

    async fn genre_context(&self, genre_slug: &str) -> StorytellerResult<GenreContext> {
        queries::genre_context::genre_context(&self.pool, genre_slug).await
    }

    // ── By genre ─────────────────────────────────────────────────────────────

    async fn archetypes_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<ArchetypeRecord>> {
        queries::by_genre::archetypes_by_genre(&self.pool, genre_slug).await
    }

    async fn dynamics_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<DynamicRecord>> {
        queries::by_genre::dynamics_by_genre(&self.pool, genre_slug).await
    }

    async fn settings_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<SettingRecord>> {
        queries::by_genre::settings_by_genre(&self.pool, genre_slug).await
    }

    async fn goals_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<GoalRecord>> {
        queries::by_genre::goals_by_genre(&self.pool, genre_slug).await
    }

    async fn profiles_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<ProfileRecord>> {
        queries::by_genre::profiles_by_genre(&self.pool, genre_slug).await
    }

    async fn tropes_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<TropeRecord>> {
        queries::by_genre::tropes_by_genre(&self.pool, genre_slug).await
    }

    async fn narrative_shapes_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<NarrativeShapeRecord>> {
        queries::by_genre::narrative_shapes_by_genre(&self.pool, genre_slug).await
    }

    async fn ontological_posture_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<OntologicalPostureRecord>> {
        queries::by_genre::ontological_posture_by_genre(&self.pool, genre_slug).await
    }

    async fn spatial_topology_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<SpatialTopologyRecord>> {
        queries::by_genre::spatial_topology_by_genre(&self.pool, genre_slug).await
    }

    async fn place_entities_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<PlaceEntityRecord>> {
        queries::by_genre::place_entities_by_genre(&self.pool, genre_slug).await
    }

    async fn archetype_dynamics_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<ArchetypeDynamicRecord>> {
        queries::by_genre::archetype_dynamics_by_genre(&self.pool, genre_slug).await
    }

    // ── By slug ──────────────────────────────────────────────────────────────

    async fn archetype_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<ArchetypeRecord>> {
        queries::by_slug::archetype_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn dynamic_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<DynamicRecord>> {
        queries::by_slug::dynamic_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn setting_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<SettingRecord>> {
        queries::by_slug::setting_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn goal_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<GoalRecord>> {
        queries::by_slug::goal_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn profile_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<ProfileRecord>> {
        queries::by_slug::profile_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn trope_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<TropeRecord>> {
        queries::by_slug::trope_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn narrative_shape_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<NarrativeShapeRecord>> {
        queries::by_slug::narrative_shape_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn ontological_posture_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<OntologicalPostureRecord>> {
        queries::by_slug::ontological_posture_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn spatial_topology_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<SpatialTopologyRecord>> {
        queries::by_slug::spatial_topology_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    async fn place_entity_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<PlaceEntityRecord>> {
        queries::by_slug::place_entity_by_slug(&self.pool, genre_slug, entity_slug).await
    }

    // ── Dimensional ───────────────────────────────────────────────────────────

    async fn genre_dimensions(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Option<GenreDimensionRecord>> {
        todo!("genre_dimensions: implement in Task 12")
    }

    async fn dimensions_for_entity(
        &self,
        _primitive_table: &str,
        _entity_slug: &str,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<DimensionValueRecord>> {
        todo!("dimensions_for_entity: implement in Task 12")
    }

    async fn entities_by_dimension(
        &self,
        _dimension_slug: &str,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<DimensionValueRecord>> {
        todo!("entities_by_dimension: implement in Task 12")
    }

    async fn dimensional_intersection(
        &self,
        _dimension_slugs: &[&str],
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<DimensionValueRecord>> {
        todo!("dimensional_intersection: implement in Task 12")
    }

    // ── State variables ───────────────────────────────────────────────────────

    async fn state_variables(&self) -> StorytellerResult<Vec<StateVariableRecord>> {
        todo!("state_variables: implement in Task 12")
    }

    async fn state_variable_interactions(
        &self,
        _genre_slug: &str,
        _state_variable_slug: &str,
    ) -> StorytellerResult<Vec<StateVariableInteractionRecord>> {
        todo!("state_variable_interactions: implement in Task 12")
    }
}
