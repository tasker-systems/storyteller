//! Scene composer — builds playable scenes from training data descriptors.

pub mod catalog;
pub mod descriptors;

pub use catalog::SceneComposer;
pub use descriptors::DescriptorSet;
