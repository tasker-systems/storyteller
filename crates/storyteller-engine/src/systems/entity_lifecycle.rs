//! Entity promotion, demotion, and decay.
//!
//! See: `docs/technical/entity-model.md`
//!
//! Entities can be promoted (prop → presence → character) or demoted
//! based on narrative relevance. Ephemeral entities decay when no longer
//! needed. Budget management ensures the system doesn't exceed token limits.
