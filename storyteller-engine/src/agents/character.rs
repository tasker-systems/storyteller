//! Character agent — ephemeral per-scene character instantiation.
//!
//! See: `docs/foundation/system_architecture.md`
//!
//! Instantiated per-scene from Storykeeper's tensor data plus psychological
//! frame computed by the ML inference layer. Expresses intent to Narrator
//! (who renders it in story voice). Doesn't know it's in a story.
