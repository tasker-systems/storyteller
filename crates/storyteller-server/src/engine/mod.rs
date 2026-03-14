//! Server-side engine state and provider management.
//!
//! ## Modules
//!
//! - [`types`] — `Composition` (immutable per-session) and `RuntimeSnapshot` (SWMR mutable)
//! - [`state_manager`] — `EngineStateManager`: session registry with lock-free reads
//! - [`providers`] — `EngineProviders`: shared LLM/ML resources

pub mod providers;
pub mod state_manager;
pub mod types;

pub use providers::EngineProviders;
pub use state_manager::EngineStateManager;
pub use types::{Composition, RuntimeSnapshot};
