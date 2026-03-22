// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Write-once composition persistence.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes and reads composition.json files (write-once per session).
#[derive(Debug, Clone)]
pub struct CompositionWriter {
    base_dir: PathBuf,
}

impl CompositionWriter {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id)
    }

    pub fn write(&self, session_id: &str, composition: &serde_json::Value) -> Result<(), String> {
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create session dir: {e}"))?;

        let path = dir.join("composition.json");
        let json = serde_json::to_string_pretty(composition)
            .map_err(|e| format!("serialize composition: {e}"))?;

        // Atomic write-once: create_new(true) fails if file already exists (no TOCTOU race)
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                format!(
                    "composition.json already exists or write failed for session {session_id}: {e}"
                )
            })?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("write composition: {e}"))?;
        Ok(())
    }

    pub fn read(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let path = self.session_dir(session_id).join("composition.json");
        let contents = fs::read_to_string(&path).map_err(|e| format!("read composition: {e}"))?;
        serde_json::from_str(&contents).map_err(|e| format!("parse composition: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_composition() {
        let dir = TempDir::new().unwrap();
        let writer = CompositionWriter::new(dir.path());

        let composition = serde_json::json!({
            "selections": {"genre_id": "test"},
            "scene": {"title": "Test Scene"},
            "characters": [],
            "goals": null,
            "intentions": null
        });

        writer.write("session-1", &composition).unwrap();
        let read_back = writer.read("session-1").unwrap();
        assert_eq!(read_back["selections"]["genre_id"], "test");
    }

    #[test]
    fn write_composition_twice_fails() {
        let dir = TempDir::new().unwrap();
        let writer = CompositionWriter::new(dir.path());
        let data = serde_json::json!({"test": true});

        writer.write("session-1", &data).unwrap();
        let result = writer.write("session-1", &data);
        assert!(result.is_err());
    }

    #[test]
    fn read_nonexistent_session_fails() {
        let dir = TempDir::new().unwrap();
        let writer = CompositionWriter::new(dir.path());
        let result = writer.read("no-such-session");
        assert!(result.is_err());
    }

    #[test]
    fn write_preserves_all_fields() {
        let dir = TempDir::new().unwrap();
        let writer = CompositionWriter::new(dir.path());

        let composition = serde_json::json!({
            "selections": {"genre_id": "fantasy", "archetype_id": "hero"},
            "scene": {"title": "The Beginning", "setting": "forest"},
            "characters": [{"id": "char-1", "name": "Aria"}],
            "goals": {"primary": "rescue the artifact"},
            "intentions": {"char-1": "protect the party"}
        });

        writer.write("session-2", &composition).unwrap();
        let read_back = writer.read("session-2").unwrap();

        assert_eq!(read_back["selections"]["archetype_id"], "hero");
        assert_eq!(read_back["scene"]["setting"], "forest");
        assert_eq!(read_back["characters"][0]["name"], "Aria");
        assert_eq!(read_back["goals"]["primary"], "rescue the artifact");
    }
}
