//! Session log infrastructure — JSONL-based turn recording.
//!
//! Each session writes to a JSONL file in the `sessions/` directory.
//! Every turn appends a single JSON line with player input, narrator output,
//! context assembly metrics, and timing data.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single turn's record in the session log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Turn number (0 = opening).
    pub turn: u32,
    /// When this turn was recorded.
    pub timestamp: DateTime<Utc>,
    /// What the player typed (empty for opening).
    pub player_input: String,
    /// The narrator's rendered prose.
    pub narrator_output: String,
    /// Context assembly token breakdown.
    pub context_assembly: ContextAssemblyLog,
    /// Phase timing in milliseconds.
    pub timing: TimingLog,
}

/// Token counts from the three-tier context assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssemblyLog {
    /// Tier 1 preamble tokens.
    pub preamble_tokens: u32,
    /// Tier 2 journal tokens.
    pub journal_tokens: u32,
    /// Tier 3 retrieved context tokens.
    pub retrieved_tokens: u32,
    /// Total estimated tokens across all tiers.
    pub total_tokens: u32,
}

/// Timing data for each phase of a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingLog {
    /// ML prediction time in milliseconds.
    pub prediction_ms: u64,
    /// Context assembly time in milliseconds.
    pub assembly_ms: u64,
    /// Narrator LLM call time in milliseconds.
    pub narrator_ms: u64,
}

/// Manages a JSONL session log file.
#[derive(Debug, Clone)]
pub struct SessionLog {
    /// Path to the JSONL file.
    path: PathBuf,
}

impl SessionLog {
    /// Create a new session log. Creates the sessions directory if needed.
    ///
    /// The filename is derived from the scene title and current timestamp,
    /// e.g. `sessions/the-flute-kept-2026-03-08T12-00-00Z.jsonl`.
    pub fn new(sessions_dir: &PathBuf, scene_title: &str) -> Result<Self, String> {
        fs::create_dir_all(sessions_dir)
            .map_err(|e| format!("Failed to create sessions dir: {e}"))?;

        let slug: String = scene_title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ");
        let filename = format!("{slug}-{timestamp}.jsonl");
        let path = sessions_dir.join(filename);

        Ok(Self { path })
    }

    /// Append a single log entry as one JSON line.
    pub fn append(&self, entry: &LogEntry) -> Result<(), String> {
        let json = serde_json::to_string(entry).map_err(|e| format!("Serialize error: {e}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Failed to open log file: {e}"))?;
        writeln!(file, "{json}").map_err(|e| format!("Failed to write log entry: {e}"))?;
        Ok(())
    }

    /// Read all log entries from the JSONL file.
    pub fn read_all(&self) -> Result<Vec<LogEntry>, String> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file =
            fs::File::open(&self.path).map_err(|e| format!("Failed to open log file: {e}"))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Failed to read line {i}: {e}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse line {i}: {e}"))?;
            entries.push(entry);
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_log_entry() {
        let dir = std::env::temp_dir().join(format!("storyteller-test-{}", uuid::Uuid::new_v4()));
        let log = SessionLog::new(&dir, "Test Scene").expect("create log");

        let entry = LogEntry {
            turn: 0,
            timestamp: Utc::now(),
            player_input: String::new(),
            narrator_output: "The scene opens.".to_string(),
            context_assembly: ContextAssemblyLog {
                preamble_tokens: 600,
                journal_tokens: 0,
                retrieved_tokens: 200,
                total_tokens: 800,
            },
            timing: TimingLog {
                prediction_ms: 0,
                assembly_ms: 5,
                narrator_ms: 1200,
            },
        };

        log.append(&entry).expect("append");
        let entries = log.read_all().expect("read_all");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].narrator_output, "The scene opens.");

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }
}
