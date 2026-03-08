//! Event types — the two-track classification system.
//!
//! See: `docs/technical/event-system.md`
//!
//! Design decision: Events are classified in two tracks — factual (what happened)
//! and interpretive (what it means). Factual classification is fast and deterministic;
//! interpretive classification may involve LLM inference and runs asynchronously.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::event_grammar::EventPayload;

/// Unique identifier for an event in the ledger.
///
/// Uses UUID v7 (time-ordered) for efficient BTree indexing and natural
/// temporal ordering in the event ledger.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct EventId(pub Uuid);

impl EventId {
    /// Create a new time-ordered event ID (UUID v7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a turn within a scene.
///
/// Uses UUID v7 (time-ordered) for efficient BTree indexing and natural
/// temporal ordering, following the same pattern as `EventId` and `SceneId`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TurnId(pub Uuid);

impl TurnId {
    /// Create a new time-ordered turn ID (UUID v7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for TurnId {
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
    /// Event payload — typed event data.
    ///
    /// Uses `EventPayload::Untyped` for migration compatibility with
    /// existing untyped payloads.
    pub payload: EventPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_id_generation_and_ordering() {
        let first = TurnId::new();
        let second = TurnId::new();
        // UUID v7 is time-ordered — second should sort after first.
        assert!(second >= first);
        assert_ne!(first, second);
    }

    #[test]
    fn turn_id_default_creates_new_id() {
        let id = TurnId::default();
        // Verify it's a valid UUID (not nil).
        assert_ne!(id.0, Uuid::nil());
    }

    #[test]
    fn event_payload_untyped_construction() {
        let payload = EventPayload::Untyped(serde_json::json!({"type": "legacy"}));
        assert!(matches!(payload, EventPayload::Untyped(_)));
    }

    #[test]
    fn narrative_event_with_untyped_payload_serde_roundtrip() {
        let event = NarrativeEvent {
            id: EventId::new(),
            timestamp: Utc::now(),
            priority: EventPriority::Normal,
            payload: EventPayload::Untyped(serde_json::json!({"action": "walk"})),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let deserialized: NarrativeEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event.id, deserialized.id);
        assert_eq!(event.priority, deserialized.priority);
    }
}
