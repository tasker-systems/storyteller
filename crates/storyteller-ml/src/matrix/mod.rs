//! Combinatorial matrix for training data generation.
//!
//! See: `docs/technical/narrator-architecture.md` § Training Data
//!
//! Three axes of variation produce diverse training scenarios:
//! - **Archetypes**: character personality templates → tensor profiles
//! - **Dynamics**: relational pattern templates → edge configurations
//! - **Profiles**: scene situation templates → constraint/affordance sets
//!
//! The [`descriptors`] module loads hand-authored JSON templates.
//! [`combinator`] iterates valid matrix cells and produces scenario skeletons.
//! [`labels`] generates heuristic ground-truth labels.
//! [`validation`] checks coherence.
//! [`export`] writes JSONL training data.

pub mod archetypes;
pub mod combinator;
pub mod descriptors;
pub mod dynamics;
pub mod export;
pub mod labels;
pub mod profiles;
pub mod validation;
