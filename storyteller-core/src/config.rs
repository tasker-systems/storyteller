//! Configuration loading and validation.
//!
//! See: `docs/technical/infrastructure-architecture.md` for configuration strategy.

/// Top-level configuration for the storyteller engine.
///
/// Loaded from TOML files with environment-specific overrides,
/// following the same pattern as tasker-core.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StorytellerConfig {
    /// Database connection URL.
    pub database_url: String,

    /// RabbitMQ connection URL (optional — only needed for tasker-core integration).
    pub rabbitmq_url: Option<String>,
}
