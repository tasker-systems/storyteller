//! Scene composition and descriptor catalog for the storyteller engine.
//!
//! This crate owns the creative assembly layer: descriptor catalogs,
//! genre/archetype/setting/dynamics selection, character generation,
//! scene composition, and goal intersection. Hydrated from JSON descriptor
//! files today, eventually database-backed.

pub mod catalog;
pub mod compose;
pub mod descriptors;
pub mod goals;
pub mod likeness;
pub mod names;

pub use catalog::SceneComposer;
pub use compose::{CastSelection, ComposedScene, DynamicSelection, SceneSelections};
pub use descriptors::DescriptorSet;
pub use goals::{CastMember, CharacterGoal, ComposedGoals, GoalVisibility, SceneGoal};
