// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Append-only event stream persistence.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

/// A single persisted event record.
/// Named `PersistedEvent` to avoid collision with the proto-generated `StoredEvent`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedEvent {
    pub event_id: String,
    pub event_type: String,
    pub session_id: String,
    pub turn: Option<u32>,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

/// Appends events to events.jsonl for a session.
#[derive(Debug, Clone)]
pub struct EventWriter {
    base_dir: PathBuf,
}

impl EventWriter {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    /// Append an event and return its assigned event_id.
    pub fn append(
        &self,
        session_id: &str,
        event_type: &str,
        turn: Option<u32>,
        payload: &serde_json::Value,
    ) -> Result<String, String> {
        let dir = self.base_dir.join(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

        let event_id = Uuid::now_v7().to_string();
        let record = PersistedEvent {
            event_id: event_id.clone(),
            event_type: event_type.to_string(),
            session_id: session_id.to_string(),
            turn,
            timestamp: Utc::now().to_rfc3339(),
            payload: payload.clone(),
        };

        let mut line =
            serde_json::to_string(&record).map_err(|e| format!("serialize event: {e}"))?;
        line.push('\n');

        let path = dir.join("events.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open events.jsonl: {e}"))?;
        file.write_all(line.as_bytes())
            .map_err(|e| format!("write event: {e}"))?;

        Ok(event_id)
    }

    /// Read all events for a session.
    pub fn read_all(&self, session_id: &str) -> Result<Vec<PersistedEvent>, String> {
        let path = self.base_dir.join(session_id).join("events.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path).map_err(|e| format!("read events: {e}"))?;
        contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("parse event: {e}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn append_and_read_events_in_order() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());

        let id1 = writer
            .append(
                "session-1",
                "PlayerInput",
                Some(1),
                &serde_json::json!({"text": "look around"}),
            )
            .unwrap();
        let id2 = writer
            .append(
                "session-1",
                "NarratorOutput",
                Some(1),
                &serde_json::json!({"text": "You see a forest."}),
            )
            .unwrap();

        let events = writer.read_all("session-1").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, id1);
        assert_eq!(events[1].event_id, id2);
        assert_eq!(events[0].event_type, "PlayerInput");
        assert_eq!(events[1].event_type, "NarratorOutput");
    }

    #[test]
    fn event_has_uuidv7_id() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());

        let event_id = writer
            .append("session-1", "TestEvent", None, &serde_json::json!({}))
            .unwrap();

        // UUIDv7 is a valid UUID
        let parsed = Uuid::parse_str(&event_id).expect("event_id should be valid UUID");
        // Version 7 has the version bits set to 0111 (7)
        assert_eq!(parsed.get_version_num(), 7);
    }

    #[test]
    fn event_has_rfc3339_timestamp() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());

        writer
            .append("session-1", "TestEvent", None, &serde_json::json!({}))
            .unwrap();

        let events = writer.read_all("session-1").unwrap();
        assert_eq!(events.len(), 1);

        // Should parse as a valid RFC3339 timestamp
        chrono::DateTime::parse_from_rfc3339(&events[0].timestamp)
            .expect("timestamp should be valid RFC3339");
    }

    #[test]
    fn read_empty_session_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());
        let events = writer.read_all("no-such-session").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn events_for_different_sessions_are_isolated() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());

        writer
            .append("session-a", "EventA", Some(0), &serde_json::json!({"x": 1}))
            .unwrap();
        writer
            .append("session-b", "EventB", Some(0), &serde_json::json!({"x": 2}))
            .unwrap();

        let events_a = writer.read_all("session-a").unwrap();
        let events_b = writer.read_all("session-b").unwrap();

        assert_eq!(events_a.len(), 1);
        assert_eq!(events_b.len(), 1);
        assert_eq!(events_a[0].event_type, "EventA");
        assert_eq!(events_b[0].event_type, "EventB");
    }

    #[test]
    fn event_turn_can_be_none() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());

        writer
            .append("session-1", "SessionStarted", None, &serde_json::json!({}))
            .unwrap();

        let events = writer.read_all("session-1").unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].turn.is_none());
    }

    #[test]
    fn payload_is_preserved_exactly() {
        let dir = TempDir::new().unwrap();
        let writer = EventWriter::new(dir.path());

        let payload = serde_json::json!({
            "text": "I attack the dragon",
            "confidence": 0.95,
            "entities": ["dragon"],
            "nested": {"key": "value"}
        });

        writer
            .append("session-1", "PlayerInput", Some(3), &payload)
            .unwrap();

        let events = writer.read_all("session-1").unwrap();
        assert_eq!(events[0].payload["text"], "I attack the dragon");
        assert_eq!(events[0].payload["confidence"], 0.95);
        assert_eq!(events[0].payload["nested"]["key"], "value");
    }
}
