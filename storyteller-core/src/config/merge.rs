//! Deep TOML merge utilities.
//!
//! Adapted from tasker-core's `merge.rs`. Recursively merges TOML tables,
//! with overlay values taking precedence over base values.

use toml::Value;

/// Deep-merges `overlay` into `base`, returning the merged result.
///
/// - Tables are merged recursively.
/// - Scalars and arrays in the overlay replace the base value.
/// - `_docs` keys are stripped from the result.
pub fn deep_merge_toml(base: &Value, overlay: &Value) -> Value {
    let mut merged = base.clone();
    if let (Some(base_table), Some(overlay_table)) = (merged.as_table_mut(), overlay.as_table()) {
        deep_merge_tables(base_table, overlay_table);
    }
    strip_docs_sections(&mut merged);
    merged
}

/// Recursively merges `overlay` table entries into `base` table.
fn deep_merge_tables(
    base: &mut toml::map::Map<String, Value>,
    overlay: &toml::map::Map<String, Value>,
) {
    for (key, overlay_value) in overlay {
        match base.get_mut(key) {
            Some(base_value) if base_value.is_table() && overlay_value.is_table() => {
                // Both are tables — recurse
                if let (Some(bt), Some(ot)) = (base_value.as_table_mut(), overlay_value.as_table())
                {
                    deep_merge_tables(bt, ot);
                }
            }
            _ => {
                // Scalar, array, or new key — overlay wins
                base.insert(key.clone(), overlay_value.clone());
            }
        }
    }
}

/// Removes `_docs` keys from all levels of the TOML tree.
fn strip_docs_sections(value: &mut Value) {
    if let Some(table) = value.as_table_mut() {
        table.retain(|key, _| !key.starts_with("_docs"));
        for (_, v) in table.iter_mut() {
            strip_docs_sections(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        s.parse::<Value>().unwrap()
    }

    #[test]
    fn test_deep_merge_nested_tables() {
        let base = parse(
            r#"
[database]
url = "base_url"
[database.pool]
max_connections = 10
min_connections = 2
"#,
        );
        let overlay = parse(
            r#"
[database.pool]
max_connections = 5
"#,
        );
        let merged = deep_merge_toml(&base, &overlay);

        let pool = merged["database"]["pool"].as_table().unwrap();
        assert_eq!(pool["max_connections"].as_integer(), Some(5));
        assert_eq!(pool["min_connections"].as_integer(), Some(2));
        assert_eq!(merged["database"]["url"].as_str(), Some("base_url"));
    }

    #[test]
    fn test_deep_merge_scalar_override() {
        let base = parse(
            r#"[inference]
thread_pool_size = 4
"#,
        );
        let overlay = parse(
            r#"[inference]
thread_pool_size = 2
"#,
        );
        let merged = deep_merge_toml(&base, &overlay);
        assert_eq!(
            merged["inference"]["thread_pool_size"].as_integer(),
            Some(2)
        );
    }

    #[test]
    fn test_deep_merge_new_keys() {
        let base = parse(
            r#"[database]
url = "base"
"#,
        );
        let overlay = parse(
            r#"[llm]
provider = "external"
"#,
        );
        let merged = deep_merge_toml(&base, &overlay);
        assert_eq!(merged["database"]["url"].as_str(), Some("base"));
        assert_eq!(merged["llm"]["provider"].as_str(), Some("external"));
    }

    #[test]
    fn test_deep_merge_array_replacement() {
        let base = parse(r#"tags = ["a", "b"]"#);
        let overlay = parse(r#"tags = ["c"]"#);
        let merged = deep_merge_toml(&base, &overlay);
        let arr = merged["tags"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_str(), Some("c"));
    }

    #[test]
    fn test_docs_stripping() {
        let base = parse(
            r#"
_docs = "top-level docs to strip"

[database]
url = "test"

[database._docs]
url = "documentation for url"
"#,
        );
        let overlay = parse("");
        let merged = deep_merge_toml(&base, &overlay);
        assert!(merged.get("_docs").is_none());
        assert!(merged["database"].get("_docs").is_none());
        assert_eq!(merged["database"]["url"].as_str(), Some("test"));
    }

    #[test]
    fn test_merge_with_empty_overlay() {
        let base = parse(
            r#"
[database]
url = "original"
[database.pool]
max_connections = 10
"#,
        );
        let overlay = parse("");
        let merged = deep_merge_toml(&base, &overlay);
        assert_eq!(merged["database"]["url"].as_str(), Some("original"));
        assert_eq!(
            merged["database"]["pool"]["max_connections"].as_integer(),
            Some(10)
        );
    }
}
