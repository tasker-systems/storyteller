//! Scene composer — builds playable scenes from training data descriptors.

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
