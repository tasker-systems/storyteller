//! Event types — the two-track classification system.
//!
//! See: `docs/technical/event-system.md`
//!
//! Design decision: Events are classified in two tracks — factual (what happened)
//! and interpretive (what it means). Factual classification is fast and deterministic;
//! interpretive classification may involve LLM inference and runs asynchronously.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unique identifier for an event in the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    /// Create a new random event ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority tier for event processing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum EventPriority {
    /// Must be processed before the current turn completes.
    Immediate,
    /// Processed within the current turn cycle if possible.
    High,
    /// Processed between turns.
    Normal,
    /// Deferred to scene boundaries or background processing.
    Low,
    /// Dispatched to tasker-core for async workflow processing.
    Deferred,
}

/// A narrative event recorded in the event ledger.
///
/// Events are the fundamental unit of change in the storyteller engine.
/// They are persisted to the ledger before processing begins (command sourcing).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeEvent {
    /// Unique event identifier.
    pub id: EventId,
    /// When the event was recorded.
    pub timestamp: DateTime<Utc>,
    /// Processing priority.
    pub priority: EventPriority,
    /// Event payload — the actual content depends on the event type.
    pub payload: serde_json::Value,
}
