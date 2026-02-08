//! Game design system trait — pluggable resolution mechanics.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! The `GameDesignSystem` trait defines the interface for genre-specific
//! resolution mechanics. The Resolver uses whichever implementation is
//! configured for the current story.
//!
//! For now, only one implementation will exist (a general-purpose fantasy
//! resolution system). The trait exists to establish the extension point
//! for future genre-specific mechanics (e.g., noir, sci-fi, literary fiction
//! without combat).

use crate::errors::StorytellerResult;
use crate::types::prediction::CharacterPrediction;
use crate::types::resolver::{ResolvedCharacterAction, ResolverOutput};
use crate::types::world_model::WorldModel;

/// Pluggable resolution mechanics for a specific genre or game design.
///
/// Implementations determine how character predictions are resolved into
/// outcomes — what succeeds, what fails, and what consequences follow.
/// The hidden RPG mechanics (attributes, skills, graduated success) live
/// here.
pub trait GameDesignSystem: std::fmt::Debug + Send + Sync {
    /// A human-readable name for this game design system.
    fn name(&self) -> &str;

    /// Resolve a set of character predictions into sequenced outcomes.
    ///
    /// Takes the predictions from the ML models and the world model for
    /// the current scene, and produces a complete `ResolverOutput` with
    /// sequenced actions, conflict resolutions, and state changes.
    fn resolve(
        &self,
        predictions: &[CharacterPrediction],
        world_model: &WorldModel,
    ) -> StorytellerResult<ResolverOutput>;

    /// Resolve a single character's predictions into outcomes.
    ///
    /// Used when only one character needs resolution (e.g., solo scenes).
    fn resolve_single(
        &self,
        prediction: &CharacterPrediction,
        world_model: &WorldModel,
    ) -> StorytellerResult<ResolvedCharacterAction>;
}
