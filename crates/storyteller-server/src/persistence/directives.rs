// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Append-only directive store for async agent outputs.
//!
//! Writers: future dramaturge (Tier C.1), world agent (Tier C.2).
//! Reader: context assembly at assembly time.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// A directive entry from an async agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectiveEntry {
    pub id: String,
    pub agency: String,
    #[serde(rename = "type")]
    pub directive_type: String,
    pub applicable_turns: Vec<u32>,
    pub based_on_turns: Vec<u32>,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// Append-only store for directives in `directives.jsonl`.
///
/// Follows the same pattern as `TurnWriter` and `EventWriter`:
/// takes `base_dir` at construction, accepts `session_id` per method call.
#[derive(Debug, Clone)]
pub struct DirectiveStore {
    base_dir: PathBuf,
}

impl DirectiveStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    fn path_for(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id).join("directives.jsonl")
    }

    /// Append a directive entry.
    pub fn append(&self, session_id: &str, entry: &DirectiveEntry) -> Result<(), String> {
        let dir = self.base_dir.join(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

        let path = self.path_for(session_id);
        let json = serde_json::to_string(entry).map_err(|e| format!("serialize directive: {e}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open directives.jsonl: {e}"))?;
        writeln!(file, "{json}").map_err(|e| format!("write directive: {e}"))
    }

    /// Read all directives for a session.
    fn read_all(&self, session_id: &str) -> Result<Vec<DirectiveEntry>, String> {
        let path = self.path_for(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path).map_err(|e| format!("open directives.jsonl: {e}"))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| format!("read line: {e}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: DirectiveEntry =
                serde_json::from_str(&line).map_err(|e| format!("parse directive: {e}"))?;
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Latest directive by agency.
    pub fn latest_by_agency(
        &self,
        session_id: &str,
        agency: &str,
    ) -> Result<Option<DirectiveEntry>, String> {
        let entries = self.read_all(session_id)?;
        Ok(entries.into_iter().rev().find(|e| e.agency == agency))
    }

    /// All directives applicable to a given turn.
    pub fn applicable_for_turn(
        &self,
        session_id: &str,
        turn: u32,
    ) -> Result<Vec<DirectiveEntry>, String> {
        let entries = self.read_all(session_id)?;
        Ok(entries
            .into_iter()
            .filter(|e| e.applicable_turns.contains(&turn))
            .collect())
    }

    /// Last N directives across all agencies.
    pub fn last_n(&self, session_id: &str, n: usize) -> Result<Vec<DirectiveEntry>, String> {
        let entries = self.read_all(session_id)?;
        let start = entries.len().saturating_sub(n);
        Ok(entries[start..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_entry(agency: &str, turns: Vec<u32>) -> DirectiveEntry {
        DirectiveEntry {
            id: uuid::Uuid::new_v4().to_string(),
            agency: agency.to_string(),
            directive_type: "test".to_string(),
            applicable_turns: turns,
            based_on_turns: vec![1],
            payload: serde_json::json!({"note": "test"}),
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    fn setup_store() -> (TempDir, DirectiveStore, String) {
        let dir = TempDir::new().unwrap();
        let store = DirectiveStore::new(dir.path());
        let session_id = "test-session";
        std::fs::create_dir_all(dir.path().join(session_id)).unwrap();
        (dir, store, session_id.to_string())
    }

    #[test]
    fn append_and_read_all() {
        let (_dir, store, sid) = setup_store();
        let entry = make_entry("dramaturge", vec![3, 4]);
        store.append(&sid, &entry).unwrap();
        let all = store.read_all(&sid).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].agency, "dramaturge");
    }

    #[test]
    fn latest_by_agency() {
        let (_dir, store, sid) = setup_store();
        store
            .append(&sid, &make_entry("dramaturge", vec![1]))
            .unwrap();
        store
            .append(&sid, &make_entry("world_agent", vec![2]))
            .unwrap();
        store
            .append(&sid, &make_entry("dramaturge", vec![3]))
            .unwrap();
        let latest = store.latest_by_agency(&sid, "dramaturge").unwrap().unwrap();
        assert_eq!(latest.applicable_turns, vec![3]);
    }

    #[test]
    fn applicable_for_turn() {
        let (_dir, store, sid) = setup_store();
        store
            .append(&sid, &make_entry("dramaturge", vec![2, 3]))
            .unwrap();
        store
            .append(&sid, &make_entry("world_agent", vec![3, 4]))
            .unwrap();
        let turn3 = store.applicable_for_turn(&sid, 3).unwrap();
        assert_eq!(turn3.len(), 2);
        let turn4 = store.applicable_for_turn(&sid, 4).unwrap();
        assert_eq!(turn4.len(), 1);
    }

    #[test]
    fn empty_store_returns_empty() {
        let (_dir, store, sid) = setup_store();
        assert!(store.read_all(&sid).unwrap().is_empty());
        assert!(store
            .latest_by_agency(&sid, "dramaturge")
            .unwrap()
            .is_none());
        assert!(store.applicable_for_turn(&sid, 1).unwrap().is_empty());
    }

    #[test]
    fn last_n() {
        let (_dir, store, sid) = setup_store();
        for i in 0..5 {
            store
                .append(&sid, &make_entry("dramaturge", vec![i]))
                .unwrap();
        }
        let last2 = store.last_n(&sid, 2).unwrap();
        assert_eq!(last2.len(), 2);
        assert_eq!(last2[0].applicable_turns, vec![3]);
        assert_eq!(last2[1].applicable_turns, vec![4]);
    }
}
