//! Turn index persistence — references into the event stream.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// A turn index entry referencing events by ID.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnEntry {
    pub turn: u32,
    pub timestamp: String,
    pub player_input: Option<String>,
    pub event_ids: Vec<String>,
}

/// Appends turn index entries to turns.jsonl.
#[derive(Debug, Clone)]
pub struct TurnWriter {
    base_dir: PathBuf,
}

impl TurnWriter {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn append(&self, session_id: &str, entry: &TurnEntry) -> Result<(), String> {
        let dir = self.base_dir.join(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

        let mut line = serde_json::to_string(entry).map_err(|e| format!("serialize turn: {e}"))?;
        line.push('\n');

        let path = dir.join("turns.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open turns.jsonl: {e}"))?;
        file.write_all(line.as_bytes())
            .map_err(|e| format!("write turn: {e}"))?;
        Ok(())
    }

    pub fn read_all(&self, session_id: &str) -> Result<Vec<TurnEntry>, String> {
        let path = self.base_dir.join(session_id).join("turns.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path).map_err(|e| format!("read turns: {e}"))?;
        contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("parse turn: {e}")))
            .collect()
    }

    pub fn turn_count(&self, session_id: &str) -> Result<usize, String> {
        let path = self.base_dir.join(session_id).join("turns.jsonl");
        if !path.exists() {
            return Ok(0);
        }
        let contents = std::fs::read_to_string(&path).map_err(|e| format!("read turns: {e}"))?;
        Ok(contents.lines().filter(|l| !l.trim().is_empty()).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_entry(turn: u32, player_input: Option<&str>, event_ids: Vec<&str>) -> TurnEntry {
        TurnEntry {
            turn,
            timestamp: Utc::now().to_rfc3339(),
            player_input: player_input.map(|s| s.to_string()),
            event_ids: event_ids.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn append_turns_and_read_back_in_order() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());

        let entry0 = make_entry(0, None, vec!["event-uuid-opening"]);
        let entry1 = make_entry(
            1,
            Some("look around"),
            vec!["event-uuid-1a", "event-uuid-1b"],
        );
        let entry2 = make_entry(2, Some("go north"), vec!["event-uuid-2a"]);

        writer.append("session-1", &entry0).unwrap();
        writer.append("session-1", &entry1).unwrap();
        writer.append("session-1", &entry2).unwrap();

        let turns = writer.read_all("session-1").unwrap();
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].turn, 0);
        assert_eq!(turns[1].turn, 1);
        assert_eq!(turns[2].turn, 2);
    }

    #[test]
    fn turn_numbers_and_event_ids_are_preserved() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());

        let entry = make_entry(5, Some("cast spell"), vec!["uuid-a", "uuid-b", "uuid-c"]);
        writer.append("session-1", &entry).unwrap();

        let turns = writer.read_all("session-1").unwrap();
        assert_eq!(turns[0].turn, 5);
        assert_eq!(turns[0].event_ids.len(), 3);
        assert_eq!(turns[0].event_ids[0], "uuid-a");
        assert_eq!(turns[0].event_ids[2], "uuid-c");
        assert_eq!(turns[0].player_input.as_deref(), Some("cast spell"));
    }

    #[test]
    fn turn_zero_has_no_player_input() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());

        let entry = make_entry(0, None, vec!["opening-event-id"]);
        writer.append("session-1", &entry).unwrap();

        let turns = writer.read_all("session-1").unwrap();
        assert_eq!(turns.len(), 1);
        assert!(turns[0].player_input.is_none());
    }

    #[test]
    fn turn_count_returns_correct_count() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());

        assert_eq!(writer.turn_count("session-1").unwrap(), 0);

        writer
            .append("session-1", &make_entry(0, None, vec!["e0"]))
            .unwrap();
        assert_eq!(writer.turn_count("session-1").unwrap(), 1);

        writer
            .append("session-1", &make_entry(1, Some("hello"), vec!["e1"]))
            .unwrap();
        assert_eq!(writer.turn_count("session-1").unwrap(), 2);
    }

    #[test]
    fn read_empty_session_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());
        let turns = writer.read_all("no-such-session").unwrap();
        assert!(turns.is_empty());
    }

    #[test]
    fn turns_for_different_sessions_are_isolated() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());

        writer
            .append("session-a", &make_entry(0, None, vec!["evt-a"]))
            .unwrap();
        writer
            .append("session-b", &make_entry(0, None, vec!["evt-b"]))
            .unwrap();
        writer
            .append("session-b", &make_entry(1, Some("go"), vec!["evt-b2"]))
            .unwrap();

        assert_eq!(writer.turn_count("session-a").unwrap(), 1);
        assert_eq!(writer.turn_count("session-b").unwrap(), 2);
    }

    #[test]
    fn timestamp_is_valid_rfc3339() {
        let dir = TempDir::new().unwrap();
        let writer = TurnWriter::new(dir.path());

        let entry = make_entry(0, None, vec!["e0"]);
        writer.append("session-1", &entry).unwrap();

        let turns = writer.read_all("session-1").unwrap();
        chrono::DateTime::parse_from_rfc3339(&turns[0].timestamp)
            .expect("timestamp should be valid RFC3339");
    }
}
