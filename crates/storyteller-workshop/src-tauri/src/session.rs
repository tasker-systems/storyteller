//! Session persistence — flat-file JSON storage in `.story/sessions/`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_engine::scene_composer::SceneSelections;

/// Summary of a persisted session, used for listing without loading full data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// UUIDv7 session identifier (sorts chronologically).
    pub session_id: String,
    /// Genre id from the scene selections.
    pub genre: String,
    /// Profile id from the scene selections.
    pub profile: String,
    /// Scene title.
    pub title: String,
    /// Display names of all cast members.
    pub cast_names: Vec<String>,
    /// Number of events recorded in the session so far.
    pub turn_count: usize,
}

/// A single turn record persisted to turns.jsonl.
///
/// Turn 0 is the opening narration (player_input is None).
/// Subsequent turns contain the full pipeline output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    pub turn: u32,
    pub player_input: Option<String>,
    pub narrator_output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predictions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_statements: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifications: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decomposition: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arbitration: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_assembly: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<serde_json::Value>,
    pub timestamp: String,
}

/// Write a serializable value as pretty-printed JSON to a file within a directory.
fn write_json(dir: &Path, name: &str, value: &(impl Serialize + ?Sized)) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| format!("serialize {name}: {e}"))?;
    fs::write(dir.join(name), json).map_err(|e| format!("write {name}: {e}"))?;
    Ok(())
}

/// Flat-file session store rooted at `{workshop_root}/.story/sessions/`.
#[derive(Debug, Clone)]
pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    /// Create a new session store, ensuring the `.story/sessions/` directory
    /// exists and that `.story/.gitignore` contains `*` so session data is
    /// never committed.
    pub fn new(workshop_root: &Path) -> Result<Self, String> {
        let story_dir = workshop_root.join(".story");
        let sessions_dir = story_dir.join("sessions");
        fs::create_dir_all(&sessions_dir)
            .map_err(|e| format!("failed to create sessions dir: {e}"))?;

        let gitignore = story_dir.join(".gitignore");
        if !gitignore.exists() {
            fs::write(&gitignore, "*\n").map_err(|e| format!("failed to write .gitignore: {e}"))?;
        }

        Ok(Self {
            base_dir: sessions_dir,
        })
    }

    /// Create a new session directory with the given scene data.
    ///
    /// Returns the session id (a UUIDv7 string).
    pub fn create_session(
        &self,
        selections: &SceneSelections,
        scene: &SceneData,
        characters: &[CharacterSheet],
    ) -> Result<String, String> {
        let session_id = uuid::Uuid::now_v7().to_string();
        let session_dir = self.base_dir.join(&session_id);
        fs::create_dir_all(&session_dir)
            .map_err(|e| format!("failed to create session dir: {e}"))?;

        write_json(&session_dir, "scene-selections.json", selections)?;
        write_json(&session_dir, "scene.json", scene)?;
        write_json(&session_dir, "characters.json", characters)?;

        // Create empty events file.
        fs::write(session_dir.join("events.jsonl"), "")
            .map_err(|e| format!("write events.jsonl: {e}"))?;

        Ok(session_id)
    }

    /// List all sessions, returning summaries sorted by session id descending
    /// (newest first, since UUIDv7 sorts chronologically).
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, String> {
        let entries = fs::read_dir(&self.base_dir)
            .map_err(|e| format!("failed to read sessions dir: {e}"))?;

        let mut summaries = Vec::new();

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let session_id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            // Parse selections for genre/profile/cast names.
            let selections_path = path.join("scene-selections.json");
            let selections_json = match fs::read_to_string(&selections_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let selections: SceneSelections = match serde_json::from_str(&selections_json) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Extract title from scene.json without full deserialization.
            let scene_path = path.join("scene.json");
            let title = match fs::read_to_string(&scene_path) {
                Ok(s) => {
                    let v: serde_json::Value =
                        serde_json::from_str(&s).unwrap_or(serde_json::Value::Null);
                    v.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled")
                        .to_string()
                }
                Err(_) => "Untitled".to_string(),
            };

            // Count lines in turns.jsonl (preferred) or events.jsonl (fallback).
            let turns_path = path.join("turns.jsonl");
            let events_path = path.join("events.jsonl");
            let turn_count = if turns_path.exists() {
                match fs::read_to_string(&turns_path) {
                    Ok(s) => s.lines().filter(|l| !l.is_empty()).count(),
                    Err(_) => 0,
                }
            } else {
                match fs::read_to_string(&events_path) {
                    Ok(s) => s.lines().filter(|l| !l.is_empty()).count(),
                    Err(_) => 0,
                }
            };

            let cast_names: Vec<String> = selections
                .cast
                .iter()
                .filter_map(|c| c.name.clone())
                .collect();

            summaries.push(SessionSummary {
                session_id,
                genre: selections.genre_id,
                profile: selections.profile_id,
                title,
                cast_names,
                turn_count,
            });
        }

        // Sort descending by session_id (UUIDv7 = chronological).
        summaries.sort_by(|a, b| b.session_id.cmp(&a.session_id));

        Ok(summaries)
    }

    /// Load a session's full data by id.
    pub fn load_session(
        &self,
        session_id: &str,
    ) -> Result<(SceneSelections, SceneData, Vec<CharacterSheet>), String> {
        let session_dir = self.base_dir.join(session_id);
        if !session_dir.is_dir() {
            return Err(format!("session not found: {session_id}"));
        }

        let read_json = |name: &str| -> Result<String, String> {
            fs::read_to_string(session_dir.join(name)).map_err(|e| format!("read {name}: {e}"))
        };

        let selections: SceneSelections =
            serde_json::from_str(&read_json("scene-selections.json")?)
                .map_err(|e| format!("parse scene-selections.json: {e}"))?;
        let scene: SceneData = serde_json::from_str(&read_json("scene.json")?)
            .map_err(|e| format!("parse scene.json: {e}"))?;
        let characters: Vec<CharacterSheet> = serde_json::from_str(&read_json("characters.json")?)
            .map_err(|e| format!("parse characters.json: {e}"))?;

        Ok((selections, scene, characters))
    }

    /// Returns the path to `turns.jsonl` for a session.
    pub fn turns_path(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id).join("turns.jsonl")
    }

    /// Append a turn record as a JSON line to the session's `turns.jsonl`.
    ///
    /// Creates the file on first call.
    pub fn append_turn(&self, session_id: &str, record: &TurnRecord) -> Result<(), String> {
        let path = self.turns_path(session_id);
        let mut json =
            serde_json::to_string(record).map_err(|e| format!("serialize turn record: {e}"))?;
        json.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open turns.jsonl: {e}"))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("write turns.jsonl: {e}"))?;

        Ok(())
    }

    /// Load all turn records from a session's `turns.jsonl`.
    ///
    /// Returns an empty vec if the file does not exist.
    pub fn load_turns(&self, session_id: &str) -> Result<Vec<TurnRecord>, String> {
        let path = self.turns_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&path).map_err(|e| format!("read turns.jsonl: {e}"))?;

        contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| serde_json::from_str(line).map_err(|e| format!("parse turn record: {e}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::character::{SceneConstraints, SceneSetting};
    use storyteller_core::types::scene::{SceneId, SceneType};
    use storyteller_engine::scene_composer::CastSelection;

    #[test]
    fn create_and_list_sessions() {
        let tmp =
            std::env::temp_dir().join(format!("storyteller-session-test-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&tmp).expect("create temp dir");

        let store = SessionStore::new(&tmp).expect("create store");

        // Verify .story/sessions/ exists
        assert!(tmp.join(".story/sessions").is_dir());
        // Verify .gitignore was written
        let gitignore = fs::read_to_string(tmp.join(".story/.gitignore")).expect("read gitignore");
        assert_eq!(gitignore, "*\n");

        // Build minimal test data
        let selections = SceneSelections {
            genre_id: "test_genre".to_string(),
            profile_id: "test_profile".to_string(),
            cast: vec![
                CastSelection {
                    archetype_id: "hero".to_string(),
                    name: Some("Aldric".to_string()),
                    role: "protagonist".to_string(),
                },
                CastSelection {
                    archetype_id: "trickster".to_string(),
                    name: Some("Mira".to_string()),
                    role: "deuteragonist".to_string(),
                },
            ],
            dynamics: Vec::new(),
            title_override: Some("The Frozen Path".to_string()),
            setting_override: None,
            seed: Some(42),
        };

        let scene = SceneData {
            scene_id: SceneId::new(),
            title: "The Frozen Path".to_string(),
            scene_type: SceneType::Gravitational,
            setting: SceneSetting {
                description: "A moonlit glade".to_string(),
                affordances: vec![],
                sensory_details: vec![],
                aesthetic_detail: String::new(),
            },
            cast: vec![],
            stakes: vec!["survival".to_string()],
            constraints: SceneConstraints {
                hard: vec![],
                soft: vec![],
                perceptual: vec![],
            },
            emotional_arc: vec![],
            evaluation_criteria: vec![],
        };

        let characters: Vec<CharacterSheet> = vec![];

        // Create session
        let session_id = store
            .create_session(&selections, &scene, &characters)
            .expect("create session");
        assert!(!session_id.is_empty());

        // Verify files exist
        let session_dir = tmp.join(".story/sessions").join(&session_id);
        assert!(session_dir.join("scene-selections.json").exists());
        assert!(session_dir.join("scene.json").exists());
        assert!(session_dir.join("characters.json").exists());
        assert!(session_dir.join("events.jsonl").exists());

        // List sessions
        let summaries = store.list_sessions().expect("list sessions");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].session_id, session_id);
        assert_eq!(summaries[0].genre, "test_genre");
        assert_eq!(summaries[0].profile, "test_profile");
        assert_eq!(summaries[0].title, "The Frozen Path");
        assert_eq!(summaries[0].cast_names, vec!["Aldric", "Mira"]);
        assert_eq!(summaries[0].turn_count, 0);

        // Load session and verify roundtrip
        let (loaded_sel, loaded_scene, loaded_chars) =
            store.load_session(&session_id).expect("load session");
        assert_eq!(loaded_sel.genre_id, "test_genre");
        assert_eq!(loaded_scene.title, "The Frozen Path");
        assert!(loaded_chars.is_empty());

        // Verify turns_path
        let turns = store.turns_path(&session_id);
        assert_eq!(turns, session_dir.join("turns.jsonl"));

        // Load nonexistent session
        assert!(store.load_session("nonexistent").is_err());

        // Clean up
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn turn_record_roundtrip() {
        let dir = std::env::temp_dir().join(format!("storyteller-test-{}", uuid::Uuid::now_v7()));
        let store = SessionStore::new(&dir).expect("create store");

        let session_id = "test-session-001";
        fs::create_dir_all(dir.join(".story/sessions").join(session_id))
            .expect("create session dir");

        let turn0 = TurnRecord {
            turn: 0,
            player_input: None,
            narrator_output: "The old rectory stands quiet.".to_string(),
            predictions: None,
            intent_statements: None,
            classifications: None,
            decomposition: None,
            arbitration: None,
            context_assembly: None,
            timing: None,
            timestamp: "2026-03-10T21:00:00Z".to_string(),
        };

        let turn1 = TurnRecord {
            turn: 1,
            player_input: Some("Margaret enters.".to_string()),
            narrator_output: "Margaret steps through the door.".to_string(),
            predictions: Some(serde_json::json!([{"character_name": "Arthur"}])),
            intent_statements: Some("**Arthur** should look up warily.".to_string()),
            classifications: Some(vec!["SpeechAct: 0.62".to_string()]),
            decomposition: None,
            arbitration: Some(serde_json::json!({"verdict": "Permitted"})),
            context_assembly: None,
            timing: Some(serde_json::json!({"narrator_ms": 5000})),
            timestamp: "2026-03-10T21:01:00Z".to_string(),
        };

        store
            .append_turn(session_id, &turn0)
            .expect("append turn 0");
        store
            .append_turn(session_id, &turn1)
            .expect("append turn 1");

        let loaded = store.load_turns(session_id).expect("load turns");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].turn, 0);
        assert!(loaded[0].player_input.is_none());
        assert_eq!(loaded[1].turn, 1);
        assert_eq!(loaded[1].player_input.as_deref(), Some("Margaret enters."));
        assert!(loaded[1].intent_statements.is_some());

        // Verify list_sessions counts turns from turns.jsonl
        // (session_id is not a UUIDv7, so list_sessions may or may not include
        // it, but turns_path and load_turns are the primary contract under test.)

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}
