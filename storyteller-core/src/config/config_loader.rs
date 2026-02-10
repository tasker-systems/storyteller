//! Configuration loading with environment variable substitution.
//!
//! Adapted from tasker-core's `config_loader.rs`. Loads TOML files, substitutes
//! `${VAR}` and `${VAR:-default}` placeholders from the environment, validates
//! against an allowlist, and parses into [`StorytellerConfig`].

use std::path::Path;

use regex::Regex;

use super::storyteller::StorytellerConfig;
use crate::{StorytellerError, StorytellerResult};

/// Loads and validates storyteller configuration from TOML files.
///
/// All methods are static — `ConfigLoader` carries no state.
#[derive(Debug)]
pub struct ConfigLoader;

impl ConfigLoader {
    /// Reads `STORYTELLER_ENV` from the environment, defaulting to `"development"`.
    pub fn detect_environment() -> String {
        std::env::var("STORYTELLER_ENV").unwrap_or_else(|_| "development".to_string())
    }

    /// Loads `.env` via dotenvy, reads `STORYTELLER_CONFIG_PATH`, and delegates
    /// to [`load_from_path`](Self::load_from_path).
    pub fn load_from_env() -> StorytellerResult<StorytellerConfig> {
        // Best-effort .env loading — missing file is fine.
        let _ = dotenvy::dotenv();

        let config_path = std::env::var("STORYTELLER_CONFIG_PATH").map_err(|_| {
            StorytellerError::Config(
                "STORYTELLER_CONFIG_PATH not set. Run `cargo make setup-env` first.".to_string(),
            )
        })?;

        Self::load_from_path(&config_path)
    }

    /// Reads a TOML file, substitutes env vars, parses, and validates.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> StorytellerResult<StorytellerConfig> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|e| {
            StorytellerError::Config(format!(
                "failed to read config file {}: {e}",
                path.display()
            ))
        })?;

        let substituted = Self::substitute_env_vars(&raw)?;

        let config: StorytellerConfig = toml::from_str(&substituted)
            .map_err(|e| StorytellerError::Config(format!("failed to parse config TOML: {e}")))?;

        Self::validate_config(&config)?;
        Ok(config)
    }

    /// Replaces `${VAR}` and `${VAR:-default}` placeholders with environment values.
    ///
    /// - `${VAR}` — replaced with the env var value; error if unset.
    /// - `${VAR:-default}` — replaced with the env var value, or `default` if unset.
    ///
    /// All variable names are validated against the allowlist. Values are escaped
    /// to prevent TOML injection.
    pub fn substitute_env_vars(input: &str) -> StorytellerResult<String> {
        let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::-((?:[^}]|\\\})*)?)?\}")
            .expect("env var regex is valid");

        let mut result = String::with_capacity(input.len());
        let mut last_end = 0;

        for caps in re.captures_iter(input) {
            let full_match = caps.get(0).expect("match exists");
            result.push_str(&input[last_end..full_match.start()]);

            let var_name = &caps[1];
            let default_value = caps.get(2).map(|m| m.as_str());

            Self::validate_env_var(var_name)?;

            let value = match std::env::var(var_name) {
                Ok(v) => v,
                Err(_) => match default_value {
                    Some(default) => default.to_string(),
                    None => {
                        return Err(StorytellerError::Config(format!(
                            "required environment variable ${{{var_name}}} is not set"
                        )));
                    }
                },
            };

            result.push_str(&Self::escape_toml_string(&value));
            last_end = full_match.end();
        }

        result.push_str(&input[last_end..]);
        Ok(result)
    }

    /// Escapes special characters to prevent TOML injection.
    pub fn escape_toml_string(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for ch in value.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Validates that an env var name is on the allowlist.
    fn validate_env_var(name: &str) -> StorytellerResult<()> {
        let allowlist = Self::get_env_var_allowlist();
        if allowlist.contains(&name) {
            Ok(())
        } else {
            Err(StorytellerError::Config(format!(
                "environment variable '{name}' is not in the config allowlist"
            )))
        }
    }

    /// Returns the set of env var names permitted in TOML config placeholders.
    fn get_env_var_allowlist() -> &'static [&'static str] {
        &[
            "DATABASE_URL",
            "STORYTELLER_ENV",
            "STORYTELLER_CONFIG_PATH",
            "STORYTELLER_DATA_PATH",
            "STORYTELLER_MODEL_PATH",
            "OLLAMA_BASE_URL",
        ]
    }

    /// Validates a parsed config for semantic correctness.
    fn validate_config(config: &StorytellerConfig) -> StorytellerResult<()> {
        if config.database.url.is_empty() {
            return Err(StorytellerError::Config(
                "database.url must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_detect_environment_default() {
        std::env::remove_var("STORYTELLER_ENV");
        assert_eq!(ConfigLoader::detect_environment(), "development");
    }

    #[test]
    #[serial]
    fn test_detect_environment_set() {
        std::env::set_var("STORYTELLER_ENV", "test");
        let result = ConfigLoader::detect_environment();
        std::env::remove_var("STORYTELLER_ENV");
        assert_eq!(result, "test");
    }

    #[test]
    #[serial]
    fn test_env_var_substitution() {
        std::env::set_var("DATABASE_URL", "postgresql://custom:5432/mydb");
        let input = r#"url = "${DATABASE_URL}""#;
        let result = ConfigLoader::substitute_env_vars(input).unwrap();
        std::env::remove_var("DATABASE_URL");
        assert_eq!(result, r#"url = "postgresql://custom:5432/mydb""#);
    }

    #[test]
    #[serial]
    fn test_env_var_substitution_with_default() {
        std::env::remove_var("DATABASE_URL");
        let input = r#"url = "${DATABASE_URL:-postgresql://localhost:5432/fallback}""#;
        let result = ConfigLoader::substitute_env_vars(input).unwrap();
        assert_eq!(result, r#"url = "postgresql://localhost:5432/fallback""#);
    }

    #[test]
    #[serial]
    fn test_env_var_substitution_set_overrides_default() {
        std::env::set_var("DATABASE_URL", "postgresql://real:5432/prod");
        let input = r#"url = "${DATABASE_URL:-postgresql://localhost:5432/fallback}""#;
        let result = ConfigLoader::substitute_env_vars(input).unwrap();
        std::env::remove_var("DATABASE_URL");
        assert_eq!(result, r#"url = "postgresql://real:5432/prod""#);
    }

    #[test]
    fn test_toml_injection_prevention() {
        let value = "value\"\n[malicious]\nhacked = true";
        let escaped = ConfigLoader::escape_toml_string(value);
        assert_eq!(escaped, r#"value\"\n[malicious]\nhacked = true"#);
    }

    #[test]
    fn test_escape_toml_string() {
        assert_eq!(ConfigLoader::escape_toml_string(r#"a\b"c"#), r#"a\\b\"c"#);
        assert_eq!(ConfigLoader::escape_toml_string("a\tb"), r"a\tb");
        assert_eq!(ConfigLoader::escape_toml_string("a\rb"), r"a\rb");
        assert_eq!(ConfigLoader::escape_toml_string("a\nb"), r"a\nb");
    }

    #[test]
    fn test_allowlist_validation_success() {
        let input = r#"url = "${DATABASE_URL:-localhost}""#;
        let result = ConfigLoader::substitute_env_vars(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_allowlist_validation_failure() {
        let input = r#"url = "${SECRET_KEY:-oops}""#;
        let result = ConfigLoader::substitute_env_vars(input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not in the config allowlist"));
    }

    #[test]
    fn test_load_from_path_valid_toml() {
        let dir = std::env::temp_dir().join("storyteller_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("valid.toml");
        std::fs::write(
            &path,
            r#"
[database]
url = "postgresql://localhost:5432/test_db"

[database.pool]
max_connections = 5
"#,
        )
        .unwrap();

        let config = ConfigLoader::load_from_path(&path).unwrap();
        assert_eq!(config.database.url, "postgresql://localhost:5432/test_db");
        assert_eq!(config.database.pool.unwrap().max_connections, Some(5));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_load_from_path_missing_file() {
        let result = ConfigLoader::load_from_path("/nonexistent/path.toml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to read config file"));
    }

    #[test]
    fn test_validate_config_missing_url() {
        let config = StorytellerConfig {
            database: super::super::storyteller::DatabaseConfig {
                url: String::new(),
                pool: None,
            },
            llm: None,
            inference: None,
            context: None,
        };
        let result = ConfigLoader::validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must not be empty"));
    }
}
