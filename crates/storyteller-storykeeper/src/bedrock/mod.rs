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

    async fn genre_context(&self, _genre_slug: &str) -> StorytellerResult<GenreContext> {
        todo!("genre_context: implement in Task 9")
    }

    // ── By genre ─────────────────────────────────────────────────────────────

    async fn archetypes_by_genre(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<ArchetypeRecord>> {
        todo!("archetypes_by_genre: implement in Task 10")
    }

    async fn dynamics_by_genre(&self, _genre_slug: &str) -> StorytellerResult<Vec<DynamicRecord>> {
        todo!("dynamics_by_genre: implement in Task 10")
    }

    async fn settings_by_genre(&self, _genre_slug: &str) -> StorytellerResult<Vec<SettingRecord>> {
        todo!("settings_by_genre: implement in Task 10")
    }

    async fn goals_by_genre(&self, _genre_slug: &str) -> StorytellerResult<Vec<GoalRecord>> {
        todo!("goals_by_genre: implement in Task 10")
    }

    async fn profiles_by_genre(&self, _genre_slug: &str) -> StorytellerResult<Vec<ProfileRecord>> {
        todo!("profiles_by_genre: implement in Task 10")
    }

    async fn tropes_by_genre(&self, _genre_slug: &str) -> StorytellerResult<Vec<TropeRecord>> {
        todo!("tropes_by_genre: implement in Task 10")
    }

    async fn narrative_shapes_by_genre(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<NarrativeShapeRecord>> {
        todo!("narrative_shapes_by_genre: implement in Task 10")
    }

    async fn ontological_posture_by_genre(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<OntologicalPostureRecord>> {
        todo!("ontological_posture_by_genre: implement in Task 10")
    }

    async fn spatial_topology_by_genre(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<SpatialTopologyRecord>> {
        todo!("spatial_topology_by_genre: implement in Task 10")
    }

    async fn place_entities_by_genre(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<PlaceEntityRecord>> {
        todo!("place_entities_by_genre: implement in Task 10")
    }

    async fn archetype_dynamics_by_genre(
        &self,
        _genre_slug: &str,
    ) -> StorytellerResult<Vec<ArchetypeDynamicRecord>> {
        todo!("archetype_dynamics_by_genre: implement in Task 10")
    }

    // ── By slug ──────────────────────────────────────────────────────────────

    async fn archetype_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<ArchetypeRecord>> {
        todo!("archetype_by_slug: implement in Task 11")
    }

    async fn dynamic_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<DynamicRecord>> {
        todo!("dynamic_by_slug: implement in Task 11")
    }

    async fn setting_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<SettingRecord>> {
        todo!("setting_by_slug: implement in Task 11")
    }

    async fn goal_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<GoalRecord>> {
        todo!("goal_by_slug: implement in Task 11")
    }

    async fn profile_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<ProfileRecord>> {
        todo!("profile_by_slug: implement in Task 11")
    }

    async fn trope_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<TropeRecord>> {
        todo!("trope_by_slug: implement in Task 11")
    }

    async fn narrative_shape_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<NarrativeShapeRecord>> {
        todo!("narrative_shape_by_slug: implement in Task 11")
    }

    async fn ontological_posture_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<OntologicalPostureRecord>> {
        todo!("ontological_posture_by_slug: implement in Task 11")
    }

    async fn spatial_topology_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<SpatialTopologyRecord>> {
        todo!("spatial_topology_by_slug: implement in Task 11")
    }

    async fn place_entity_by_slug(
        &self,
        _genre_slug: &str,
        _entity_slug: &str,
    ) -> StorytellerResult<Option<PlaceEntityRecord>> {
        todo!("place_entity_by_slug: implement in Task 11")
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
