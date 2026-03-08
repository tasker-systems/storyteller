//! Typed configuration structs for the storyteller engine.

use serde::{Deserialize, Serialize};

/// Top-level configuration for the storyteller engine.
///
/// Only `database` is required. All other sections default to `None`
/// and fall back to sensible defaults when absent.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorytellerConfig {
    pub database: DatabaseConfig,
    pub llm: Option<LlmConfig>,
    pub inference: Option<InferenceConfig>,
    pub context: Option<ContextBudgetConfig>,
}

/// Database connection and pool settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool: Option<PoolConfig>,
}

/// Connection pool tuning.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolConfig {
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub idle_timeout_secs: Option<u64>,
}

/// LLM provider selection and configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub provider: String,
    pub external: Option<ExternalLlmConfig>,
}

/// Configuration for the external LLM server (e.g., Ollama).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalLlmConfig {
    pub base_url: String,
    pub model: String,
}

/// ML inference thread pool settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InferenceConfig {
    pub thread_pool_size: Option<usize>,
}

/// Token budget for the three-tier narrator context assembly.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextBudgetConfig {
    pub preamble_budget_tokens: Option<u32>,
    pub journal_budget_tokens: Option<u32>,
    pub retrieved_budget_tokens: Option<u32>,
}
