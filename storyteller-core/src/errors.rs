//! Error types for the storyteller engine.
//!
//! See: `docs/technical/event-system.md` for event processing error semantics.

/// Unified error type for storyteller operations.
#[derive(Debug, thiserror::Error)]
pub enum StorytellerError {
    /// Database operation failed.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration is invalid or missing.
    #[error("configuration error: {0}")]
    Config(String),

    /// An entity was not found.
    #[error("entity not found: {0}")]
    EntityNotFound(String),

    /// A scene operation failed.
    #[error("scene error: {0}")]
    Scene(String),

    /// An agent produced an invalid response.
    #[error("agent error: {0}")]
    Agent(String),

    /// LLM provider returned an error.
    #[error("llm error: {0}")]
    Llm(String),

    /// Graph query failed.
    #[error("graph error: {0}")]
    Graph(String),

    /// Messaging (RabbitMQ) operation failed.
    #[error("messaging error: {0}")]
    Messaging(String),

    /// ML inference error (ort/ONNX Runtime).
    #[error("inference error: {0}")]
    Inference(String),

    /// Catch-all for unexpected errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Convenience alias used throughout the storyteller crates.
pub type StorytellerResult<T> = Result<T, StorytellerError>;
