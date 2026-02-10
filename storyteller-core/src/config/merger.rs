//! Configuration merger: loads base + environment overlay and produces merged TOML.
//!
//! Simplified from tasker-core's `merger.rs` — storyteller has a single config
//! context (`storyteller`) rather than tasker's multi-context setup.

use std::path::{Path, PathBuf};

use crate::{StorytellerError, StorytellerResult};

use super::merge::deep_merge_toml;

/// Merges base and environment-specific TOML config files.
#[derive(Debug)]
pub struct ConfigMerger {
    source_dir: PathBuf,
    environment: String,
}

impl ConfigMerger {
    /// Creates a new merger for the given source directory and environment.
    ///
    /// - `source_dir` — path to `config/storyteller/` (contains `base/` and `environments/`)
    /// - `environment` — e.g., `"development"`, `"test"`
    pub fn new<P: AsRef<Path>>(source_dir: P, environment: &str) -> Self {
        Self {
            source_dir: source_dir.as_ref().to_path_buf(),
            environment: environment.to_string(),
        }
    }

    /// Loads `base/storyteller.toml`, optionally overlays `environments/{env}/storyteller.toml`,
    /// and returns the merged TOML string.
    pub fn merge(&self) -> StorytellerResult<String> {
        let base_path = self.source_dir.join("base/storyteller.toml");

        let base_content = std::fs::read_to_string(&base_path).map_err(|e| {
            StorytellerError::Config(format!(
                "failed to read base config {}: {e}",
                base_path.display()
            ))
        })?;

        let base: toml::Value = base_content.parse().map_err(|e| {
            StorytellerError::Config(format!("failed to parse base config TOML: {e}"))
        })?;

        // Environment overlay is optional
        let env_path = self.source_dir.join(format!(
            "environments/{}/storyteller.toml",
            self.environment
        ));

        let merged = if env_path.exists() {
            let env_content = std::fs::read_to_string(&env_path).map_err(|e| {
                StorytellerError::Config(format!(
                    "failed to read environment config {}: {e}",
                    env_path.display()
                ))
            })?;
            let env_overlay: toml::Value = env_content.parse().map_err(|e| {
                StorytellerError::Config(format!("failed to parse environment config TOML: {e}"))
            })?;
            deep_merge_toml(&base, &env_overlay)
        } else {
            base
        };

        toml::to_string_pretty(&merged).map_err(|e| {
            StorytellerError::Config(format!("failed to serialize merged config: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dir(name: &str) -> PathBuf {
        let source = std::env::temp_dir()
            .join("storyteller_merger_test")
            .join(name);
        // Clean up any previous run
        let _ = std::fs::remove_dir_all(&source);
        std::fs::create_dir_all(source.join("base")).unwrap();
        std::fs::create_dir_all(source.join("environments/test")).unwrap();
        source
    }

    #[test]
    fn test_merger_creation() {
        let merger = ConfigMerger::new("/tmp/config", "test");
        assert_eq!(merger.source_dir, PathBuf::from("/tmp/config"));
        assert_eq!(merger.environment, "test");
    }

    #[test]
    fn test_merger_merge_base_only() {
        let source = create_test_dir("base_only");
        std::fs::write(
            source.join("base/storyteller.toml"),
            r#"[database]
url = "postgresql://localhost/dev"
"#,
        )
        .unwrap();

        let merger = ConfigMerger::new(&source, "production");
        let result = merger.merge().unwrap();
        assert!(result.contains("postgresql://localhost/dev"));

        let _ = std::fs::remove_dir_all(&source);
    }

    #[test]
    fn test_merger_merge_with_env_overlay() {
        let source = create_test_dir("env_overlay");
        std::fs::write(
            source.join("base/storyteller.toml"),
            r#"[database]
url = "postgresql://localhost/dev"

[database.pool]
max_connections = 10
"#,
        )
        .unwrap();
        std::fs::write(
            source.join("environments/test/storyteller.toml"),
            r#"[database]
url = "postgresql://localhost/test"

[database.pool]
max_connections = 5
"#,
        )
        .unwrap();

        let merger = ConfigMerger::new(&source, "test");
        let result = merger.merge().unwrap();
        assert!(result.contains("postgresql://localhost/test"));
        assert!(result.contains("5"));

        let _ = std::fs::remove_dir_all(&source);
    }

    #[test]
    fn test_merger_missing_base_dir() {
        let merger = ConfigMerger::new("/nonexistent/config", "test");
        let result = merger.merge();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to read base config"));
    }
}
