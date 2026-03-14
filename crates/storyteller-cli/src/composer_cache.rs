//! Local descriptor cache for the CLI.
//!
//! `ComposerCache` syncs genre descriptors from the server and persists them
//! as JSON index files under `.story/composition-cache/`. This allows the CLI
//! to resolve slugs (e.g. `low_fantasy_folklore`) to entity IDs offline.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use storyteller_client::StorytellerClient;

/// A single cache entry: the human-readable slug, the opaque entity ID, and
/// the display name shown in CLI output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub slug: String,
    pub entity_id: String,
    pub display_name: String,
}

/// Local descriptor cache rooted at a directory.
pub struct ComposerCache {
    root: PathBuf,
}

impl ComposerCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Default path relative to the working directory.
    pub fn default_path() -> PathBuf {
        PathBuf::from(".story/composition-cache")
    }

    // -------------------------------------------------------------------------
    // Sync
    // -------------------------------------------------------------------------

    /// Sync all genre and per-genre descriptor indexes from the running server.
    pub async fn sync(
        &self,
        client: &mut StorytellerClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.root)?;

        // Genres
        let genre_list = client.list_genres().await?;
        let genre_entries: Vec<CacheEntry> = genre_list
            .genres
            .iter()
            .map(|g| CacheEntry {
                slug: g.slug.clone(),
                entity_id: g.entity_id.clone(),
                display_name: g.display_name.clone(),
            })
            .collect();
        self.write_index(&self.root.join("genres.json"), &genre_entries)?;

        // Per-genre descriptors — genre_id for server calls is the slug
        for genre in &genre_list.genres {
            let genre_dir = self.root.join(&genre.slug);
            std::fs::create_dir_all(&genre_dir)?;

            // Archetypes
            let archetypes = client.get_archetypes_for_genre(&genre.slug).await?;
            let entries: Vec<CacheEntry> = archetypes
                .archetypes
                .iter()
                .map(|a| CacheEntry {
                    slug: a.slug.clone(),
                    entity_id: a.entity_id.clone(),
                    display_name: a.display_name.clone(),
                })
                .collect();
            self.write_index(&genre_dir.join("archetypes.json"), &entries)?;

            // Profiles
            let profiles = client.get_profiles_for_genre(&genre.slug).await?;
            let entries: Vec<CacheEntry> = profiles
                .profiles
                .iter()
                .map(|p| CacheEntry {
                    slug: p.slug.clone(),
                    entity_id: p.entity_id.clone(),
                    display_name: p.display_name.clone(),
                })
                .collect();
            self.write_index(&genre_dir.join("profiles.json"), &entries)?;

            // Dynamics
            let dynamics = client.get_dynamics_for_genre(&genre.slug, vec![]).await?;
            let entries: Vec<CacheEntry> = dynamics
                .dynamics
                .iter()
                .map(|d| CacheEntry {
                    slug: d.slug.clone(),
                    entity_id: d.entity_id.clone(),
                    display_name: d.display_name.clone(),
                })
                .collect();
            self.write_index(&genre_dir.join("dynamics.json"), &entries)?;

            // Names — stored as CacheEntry with slug = entity_id = display_name = name
            let names = client.get_names_for_genre(&genre.slug).await?;
            let entries: Vec<CacheEntry> = names
                .names
                .iter()
                .map(|n| CacheEntry {
                    slug: n.clone(),
                    entity_id: n.clone(),
                    display_name: n.clone(),
                })
                .collect();
            self.write_index(&genre_dir.join("names.json"), &entries)?;

            // Settings — currently returns empty list from server, but sync gracefully
            let settings = client.get_settings_for_genre(&genre.slug).await?;
            let entries: Vec<CacheEntry> = settings
                .settings
                .iter()
                .map(|s| CacheEntry {
                    slug: s.profile_id.clone(),
                    entity_id: s.profile_id.clone(),
                    display_name: s.name.clone(),
                })
                .collect();
            self.write_index(&genre_dir.join("settings.json"), &entries)?;
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Queries
    // -------------------------------------------------------------------------

    /// Resolve a slug to its entity_id from the cached index.
    ///
    /// `category` is e.g. `"genres"`, `"archetypes"`, `"profiles"`.
    /// `genre_slug` must be `Some(slug)` for per-genre categories.
    pub fn resolve_slug(
        &self,
        category: &str,
        genre_slug: Option<&str>,
        slug: &str,
    ) -> Result<String, String> {
        let entries = self.load_entries(category, genre_slug)?;
        entries
            .iter()
            .find(|e| e.slug == slug)
            .map(|e| e.entity_id.clone())
            .ok_or_else(|| {
                format!(
                    "Slug '{slug}' not found in {category} cache. \
                     Run `storyteller-cli composer sync` to refresh."
                )
            })
    }

    /// List all entries from a cached index.
    pub fn list(
        &self,
        category: &str,
        genre_slug: Option<&str>,
    ) -> Result<Vec<CacheEntry>, String> {
        self.load_entries(category, genre_slug)
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn load_entries(
        &self,
        category: &str,
        genre_slug: Option<&str>,
    ) -> Result<Vec<CacheEntry>, String> {
        let path = match genre_slug {
            Some(g) => self.root.join(g).join(format!("{category}.json")),
            None => self.root.join(format!("{category}.json")),
        };

        if !path.exists() {
            return Err(format!(
                "Cache not found at {}. Run `storyteller-cli composer sync` first.",
                path.display()
            ));
        }

        let data =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read cache: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse cache: {e}"))
    }

    pub(crate) fn write_index(
        &self,
        path: &Path,
        entries: &[CacheEntry],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(entries)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_slug_from_cache() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let entries = vec![CacheEntry {
            slug: "dark_fantasy".into(),
            entity_id: "019ce4c3-0001".into(),
            display_name: "Dark Fantasy".into(),
        }];
        cache
            .write_index(&dir.path().join("genres.json"), &entries)
            .unwrap();

        let id = cache.resolve_slug("genres", None, "dark_fantasy").unwrap();
        assert_eq!(id, "019ce4c3-0001");
    }

    #[test]
    fn resolve_slug_missing_cache_gives_helpful_error() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let err = cache
            .resolve_slug("genres", None, "dark_fantasy")
            .unwrap_err();
        assert!(err.contains("composer sync"), "error was: {err}");
    }

    #[test]
    fn resolve_slug_not_found_gives_helpful_error() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let entries = vec![CacheEntry {
            slug: "low_fantasy".into(),
            entity_id: "019ce4c3-0002".into(),
            display_name: "Low Fantasy".into(),
        }];
        cache
            .write_index(&dir.path().join("genres.json"), &entries)
            .unwrap();

        let err = cache
            .resolve_slug("genres", None, "dark_fantasy")
            .unwrap_err();
        assert!(err.contains("not found"), "error was: {err}");
        assert!(err.contains("composer sync"), "error was: {err}");
    }

    #[test]
    fn list_entries_from_cache() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let entries = vec![
            CacheEntry {
                slug: "a".into(),
                entity_id: "1".into(),
                display_name: "A".into(),
            },
            CacheEntry {
                slug: "b".into(),
                entity_id: "2".into(),
                display_name: "B".into(),
            },
        ];
        cache
            .write_index(&dir.path().join("genres.json"), &entries)
            .unwrap();

        let listed = cache.list("genres", None).unwrap();
        assert_eq!(listed.len(), 2);
    }
}
