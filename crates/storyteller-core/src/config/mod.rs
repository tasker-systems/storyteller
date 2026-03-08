//! Configuration loading, merging, and validation.
//!
//! Follows the tasker-core pattern: modular dotenv assembly → TOML base + environment
//! overlays → env var substitution → typed config.
//!
//! See: `docs/technical/infrastructure-architecture.md` for configuration strategy.

pub mod config_loader;
pub mod merge;
pub mod merger;
pub mod storyteller;

pub use config_loader::ConfigLoader;
pub use merger::ConfigMerger;
pub use storyteller::StorytellerConfig;
