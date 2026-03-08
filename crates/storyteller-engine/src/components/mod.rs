//! Bevy ECS components — the data attached to entities in the world.
//!
//! Components map directly to concepts in the entity model and tensor
//! specifications. Each component is a small, focused piece of data
//! that systems read and write.

pub mod communicability;
pub mod identity;
pub mod persistence;
pub mod scene;
pub mod tensor;
pub mod turn;
