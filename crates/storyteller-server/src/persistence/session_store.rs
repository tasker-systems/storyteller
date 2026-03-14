//! Unified session store composing composition, event, and turn writers.

use std::fs;
use std::path::{Path, PathBuf};

use super::composition::CompositionWriter;
use super::events::EventWriter;
use super::turns::TurnWriter;

/// Manages session directories and delegates to specialized writers.
#[derive(Debug, Clone)]
pub struct SessionStore {
    base_dir: PathBuf,
    pub composition: CompositionWriter,
    pub events: EventWriter,
    pub turns: TurnWriter,
}

impl SessionStore {
    pub fn new(base_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(base_dir).map_err(|e| format!("create sessions dir: {e}"))?;

        // Prevent session data from being committed to git
        let gitignore = base_dir.join(".gitignore");
        if !gitignore.exists() {
            let _ = fs::write(&gitignore, "*\n");
        }

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            composition: CompositionWriter::new(base_dir),
            events: EventWriter::new(base_dir),
            turns: TurnWriter::new(base_dir),
        })
    }

    /// List all session IDs (directories in base_dir).
    pub fn list_session_ids(&self) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        let entries =
            fs::read_dir(&self.base_dir).map_err(|e| format!("read sessions dir: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("read entry: {e}"))?;
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    ids.push(name.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Create a new session directory, returning the session_id (UUIDv7).
    pub fn create_session(&self) -> Result<String, String> {
        let session_id = uuid::Uuid::now_v7().to_string();
        let dir = self.base_dir.join(&session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create session dir: {e}"))?;
        Ok(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_creates_base_dir_and_gitignore() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");

        let _store = SessionStore::new(&sessions_dir).unwrap();

        assert!(sessions_dir.exists());
        assert!(sessions_dir.join(".gitignore").exists());
    }

    #[test]
    fn create_session_returns_valid_uuidv7() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();

        let session_id = store.create_session().unwrap();

        // Should be parseable as a UUID
        let parsed = uuid::Uuid::parse_str(&session_id).expect("session_id should be valid UUID");
        assert_eq!(parsed.get_version_num(), 7);
    }

    #[test]
    fn create_session_creates_directory() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();

        let session_id = store.create_session().unwrap();

        assert!(dir.path().join(&session_id).is_dir());
    }

    #[test]
    fn list_session_ids_returns_created_sessions() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();

        let id1 = store.create_session().unwrap();
        let id2 = store.create_session().unwrap();

        let ids = store.list_session_ids().unwrap();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn list_session_ids_excludes_files() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();

        // The .gitignore is a file, not a dir — should not appear in session list
        let ids = store.list_session_ids().unwrap();
        assert!(!ids.contains(&".gitignore".to_string()));
    }

    #[test]
    fn list_session_ids_is_sorted() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();

        store.create_session().unwrap();
        store.create_session().unwrap();
        store.create_session().unwrap();

        let ids = store.list_session_ids().unwrap();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn session_store_delegates_to_composition_writer() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();
        let session_id = store.create_session().unwrap();

        let data = serde_json::json!({"title": "Test Scene"});
        store.composition.write(&session_id, &data).unwrap();

        let read_back = store.composition.read(&session_id).unwrap();
        assert_eq!(read_back["title"], "Test Scene");
    }

    #[test]
    fn session_store_delegates_to_event_writer() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();
        let session_id = store.create_session().unwrap();

        let event_id = store
            .events
            .append(&session_id, "TestEvent", Some(0), &serde_json::json!({}))
            .unwrap();

        let events = store.events.read_all(&session_id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, event_id);
    }

    #[test]
    fn session_store_delegates_to_turn_writer() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();
        let session_id = store.create_session().unwrap();

        let entry = super::super::turns::TurnEntry {
            turn: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            player_input: None,
            event_ids: vec!["e0".to_string()],
        };
        store.turns.append(&session_id, &entry).unwrap();

        assert_eq!(store.turns.turn_count(&session_id).unwrap(), 1);
    }
}
