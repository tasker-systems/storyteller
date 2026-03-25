// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! The `BedrockQuery` trait — read-only access to the bedrock narrative data layer.
//!
//! Implementations provide access to narrative primitives (archetypes, dynamics,
//! settings, goals, profiles, tropes, narrative shapes, ontological postures,
//! spatial topologies, place entities, archetype dynamics) along with dimensional
//! analysis, state variables, and reference data.

use crate::errors::StorytellerResult;
use crate::types::bedrock::{
    ArchetypeDynamicRecord, ArchetypeRecord, DimensionValueRecord, DynamicRecord, GenreContext,
    GenreDimensionRecord, GenreRecord, GoalRecord, NarrativeShapeRecord, OntologicalPostureRecord,
    PlaceEntityRecord, ProfileRecord, SettingRecord, SpatialTopologyRecord,
    StateVariableInteractionRecord, StateVariableRecord, TropeFamilyRecord, TropeRecord,
};

/// Read-only access to the bedrock narrative data layer.
///
/// All methods take genre slugs and entity slugs as string references — no
/// Uuid lookups are exposed at this boundary. Implementations are responsible
/// for resolving slugs to database identifiers.
///
/// The trait is object-safe: all async methods use `async_trait`, which
/// rewrites them to `Pin<Box<dyn Future + Send>>` under the hood.
#[async_trait::async_trait]
pub trait BedrockQuery: Send + Sync + std::fmt::Debug {
    // ── Bulk ──────────────────────────────────────────────────────────────────

    /// Return all narrative primitives for a genre in a single assembled context.
    async fn genre_context(&self, genre_slug: &str) -> StorytellerResult<GenreContext>;

    // ── By genre (11 types) ──────────────────────────────────────────────────

    /// Return all archetypes for the given genre.
    async fn archetypes_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<ArchetypeRecord>>;

    /// Return all dynamics for the given genre.
    async fn dynamics_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<DynamicRecord>>;

    /// Return all settings for the given genre.
    async fn settings_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<SettingRecord>>;

    /// Return all goals for the given genre.
    async fn goals_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<GoalRecord>>;

    /// Return all profiles for the given genre.
    async fn profiles_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<ProfileRecord>>;

    /// Return all tropes for the given genre.
    async fn tropes_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<TropeRecord>>;

    /// Return all narrative shapes for the given genre.
    async fn narrative_shapes_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<NarrativeShapeRecord>>;

    /// Return all ontological postures for the given genre.
    async fn ontological_posture_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<OntologicalPostureRecord>>;

    /// Return all spatial topologies for the given genre.
    async fn spatial_topology_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<SpatialTopologyRecord>>;

    /// Return all place entities for the given genre.
    async fn place_entities_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<PlaceEntityRecord>>;

    /// Return all archetype dynamics for the given genre.
    async fn archetype_dynamics_by_genre(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<ArchetypeDynamicRecord>>;

    // ── By slug (10 types) ───────────────────────────────────────────────────

    /// Return a single archetype by genre + entity slug, or `None` if not found.
    async fn archetype_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<ArchetypeRecord>>;

    /// Return a single dynamic by genre + entity slug, or `None` if not found.
    async fn dynamic_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<DynamicRecord>>;

    /// Return a single setting by genre + entity slug, or `None` if not found.
    async fn setting_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<SettingRecord>>;

    /// Return a single goal by genre + entity slug, or `None` if not found.
    async fn goal_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<GoalRecord>>;

    /// Return a single profile by genre + entity slug, or `None` if not found.
    async fn profile_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<ProfileRecord>>;

    /// Return a single trope by genre + entity slug, or `None` if not found.
    async fn trope_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<TropeRecord>>;

    /// Return a single narrative shape by genre + entity slug, or `None` if not found.
    async fn narrative_shape_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<NarrativeShapeRecord>>;

    /// Return a single ontological posture by genre + entity slug, or `None` if not found.
    async fn ontological_posture_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<OntologicalPostureRecord>>;

    /// Return a single spatial topology by genre + entity slug, or `None` if not found.
    async fn spatial_topology_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<SpatialTopologyRecord>>;

    /// Return a single place entity by genre + entity slug, or `None` if not found.
    async fn place_entity_by_slug(
        &self,
        genre_slug: &str,
        entity_slug: &str,
    ) -> StorytellerResult<Option<PlaceEntityRecord>>;

    // ── Dimensional ───────────────────────────────────────────────────────────

    /// Return the genre-level dimensional analysis record, or `None` if not present.
    async fn genre_dimensions(
        &self,
        genre_slug: &str,
    ) -> StorytellerResult<Option<GenreDimensionRecord>>;

    /// Return all dimension values for a specific entity within a genre.
    async fn dimensions_for_entity(
        &self,
        primitive_table: &str,
        entity_slug: &str,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<DimensionValueRecord>>;

    /// Return all entities in a genre that have a value for the given dimension.
    async fn entities_by_dimension(
        &self,
        dimension_slug: &str,
        genre_slug: &str,
    ) -> StorytellerResult<Vec<DimensionValueRecord>>;

    /// Return entities in a genre that have values for ALL of the given dimensions.
    async fn dimensional_intersection(
        &self,
        dimension_slugs: &[&str],
        genre_slug: &str,
    ) -> StorytellerResult<Vec<DimensionValueRecord>>;

    // ── State variables ───────────────────────────────────────────────────────

    /// Return all state variable definitions.
    async fn state_variables(&self) -> StorytellerResult<Vec<StateVariableRecord>>;

    /// Return interactions between a specific state variable and primitives in a genre.
    async fn state_variable_interactions(
        &self,
        genre_slug: &str,
        state_variable_slug: &str,
    ) -> StorytellerResult<Vec<StateVariableInteractionRecord>>;

    // ── Reference ─────────────────────────────────────────────────────────────

    /// Return all genre records.
    async fn genres(&self) -> StorytellerResult<Vec<GenreRecord>>;

    /// Return all trope family records.
    async fn trope_families(&self) -> StorytellerResult<Vec<TropeFamilyRecord>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _assert_object_safe(_: &dyn BedrockQuery) {}

    fn _assert_send_sync<T: Send + Sync>() {}

    fn _assert_bounds() {
        _assert_send_sync::<Box<dyn BedrockQuery>>();
    }
}
