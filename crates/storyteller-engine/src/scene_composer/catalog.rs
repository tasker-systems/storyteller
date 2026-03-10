//! Catalog queries for genre-filtered scene composition options.
//!
//! [`SceneComposer`] wraps a [`DescriptorSet`] and provides filtered views of
//! archetypes, dynamics, profiles, and names scoped to a specific genre. The
//! summary types are lightweight projections designed for UI consumption.

use std::path::Path;

use serde::Serialize;

use storyteller_core::StorytellerError;

use super::descriptors::{Archetype, DescriptorSet, Dynamic, Genre, Profile};

// ---------------------------------------------------------------------------
// Summary types (UI-facing)
// ---------------------------------------------------------------------------

/// Lightweight genre summary for catalog listing.
#[derive(Debug, Clone, Serialize)]
pub struct GenreSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub archetype_count: usize,
    pub profile_count: usize,
    pub dynamic_count: usize,
}

/// Lightweight profile summary for catalog listing.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub scene_type: String,
    pub tension_min: f64,
    pub tension_max: f64,
    pub cast_size_min: f64,
    pub cast_size_max: f64,
}

/// Lightweight archetype summary for catalog listing.
#[derive(Debug, Clone, Serialize)]
pub struct ArchetypeSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
}

/// Lightweight dynamic summary for catalog listing.
#[derive(Debug, Clone, Serialize)]
pub struct DynamicSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
}

// ---------------------------------------------------------------------------
// SceneComposer
// ---------------------------------------------------------------------------

/// Provides genre-filtered catalog queries over the loaded descriptor set.
///
/// The composer is the primary entry point for UI code that needs to present
/// valid scene-building options (archetypes, dynamics, profiles, names) for a
/// given genre, respecting excluded combinations.
#[derive(Debug, Clone)]
pub struct SceneComposer {
    pub(crate) descriptors: DescriptorSet,
}

impl SceneComposer {
    /// Load descriptors from `data_path` and return a ready-to-query composer.
    pub fn load(data_path: &Path) -> Result<Self, StorytellerError> {
        let descriptors = DescriptorSet::load(data_path)?;
        Ok(Self { descriptors })
    }

    // -- catalog queries ---------------------------------------------------

    /// Return summaries of all available genres.
    pub fn genres(&self) -> Vec<GenreSummary> {
        self.descriptors
            .genres
            .iter()
            .map(|g| GenreSummary {
                id: g.id.clone(),
                display_name: g.display_name.clone(),
                description: g.description.clone(),
                archetype_count: g.valid_archetypes.len(),
                profile_count: g.valid_profiles.len(),
                dynamic_count: g.valid_dynamics.len(),
            })
            .collect()
    }

    /// Return profiles valid for the given genre.
    pub fn profiles_for_genre(&self, genre_id: &str) -> Vec<ProfileSummary> {
        let Some(genre) = self.find_genre(genre_id) else {
            return Vec::new();
        };

        genre
            .valid_profiles
            .iter()
            .filter_map(|pid| self.find_profile(pid))
            .map(|p| ProfileSummary {
                id: p.id.clone(),
                display_name: p.display_name.clone(),
                description: p.description.clone(),
                scene_type: p.scene_type.clone(),
                tension_min: p.tension.min,
                tension_max: p.tension.max,
                cast_size_min: p.cast_size.min,
                cast_size_max: p.cast_size.max,
            })
            .collect()
    }

    /// Return archetypes valid for the given genre.
    pub fn archetypes_for_genre(&self, genre_id: &str) -> Vec<ArchetypeSummary> {
        let Some(genre) = self.find_genre(genre_id) else {
            return Vec::new();
        };

        genre
            .valid_archetypes
            .iter()
            .filter_map(|aid| self.find_archetype(aid))
            .map(|a| ArchetypeSummary {
                id: a.id.clone(),
                display_name: a.display_name.clone(),
                description: a.description.clone(),
            })
            .collect()
    }

    /// Return dynamics valid for the given genre, excluding those that conflict
    /// with the `selected_archetypes` per the genre's `excluded_combinations`.
    ///
    /// A dynamic is excluded when an excluded combination has both a matching
    /// dynamic AND a matching archetype from `selected_archetypes`.
    pub fn dynamics_for_genre(
        &self,
        genre_id: &str,
        selected_archetypes: &[String],
    ) -> Vec<DynamicSummary> {
        let Some(genre) = self.find_genre(genre_id) else {
            return Vec::new();
        };

        // Collect dynamic IDs that should be excluded given the selected archetypes.
        let excluded_dynamic_ids: std::collections::HashSet<&str> = genre
            .excluded_combinations
            .iter()
            .filter_map(|ec| {
                let dyn_id = ec.dynamic.as_deref()?;
                let arch_id = ec.archetype.as_deref()?;
                if selected_archetypes.iter().any(|sa| sa == arch_id) {
                    Some(dyn_id)
                } else {
                    None
                }
            })
            .collect();

        genre
            .valid_dynamics
            .iter()
            .filter(|did| !excluded_dynamic_ids.contains(did.as_str()))
            .filter_map(|did| self.find_dynamic(did))
            .map(|d| DynamicSummary {
                id: d.id.clone(),
                display_name: d.display_name.clone(),
                description: d.description.clone(),
                role_a: d.role_a.clone(),
                role_b: d.role_b.clone(),
            })
            .collect()
    }

    /// Return the name pool for the given genre, or an empty vec if the genre
    /// has no associated names.
    pub fn names_for_genre(&self, genre_id: &str) -> Vec<String> {
        self.descriptors
            .names
            .get(genre_id)
            .map(|nc| nc.names.clone())
            .unwrap_or_default()
    }

    // -- internal helpers --------------------------------------------------

    /// Find a genre by id.
    pub(crate) fn find_genre(&self, id: &str) -> Option<&Genre> {
        self.descriptors.genres.iter().find(|g| g.id == id)
    }

    /// Find an archetype by id.
    pub(crate) fn find_archetype(&self, id: &str) -> Option<&Archetype> {
        self.descriptors.archetypes.iter().find(|a| a.id == id)
    }

    /// Find a profile by id.
    pub(crate) fn find_profile(&self, id: &str) -> Option<&Profile> {
        self.descriptors.profiles.iter().find(|p| p.id == id)
    }

    /// Find a dynamic by id.
    pub(crate) fn find_dynamic(&self, id: &str) -> Option<&Dynamic> {
        self.descriptors.dynamics.iter().find(|d| d.id == id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the storyteller-data base path from the environment, or None.
    fn data_path() -> Option<std::path::PathBuf> {
        std::env::var("STORYTELLER_DATA_PATH")
            .ok()
            .map(std::path::PathBuf::from)
    }

    fn load_composer() -> Option<SceneComposer> {
        let base = data_path()?;
        Some(SceneComposer::load(&base).expect("descriptor loading should succeed"))
    }

    #[test]
    fn genres_returns_all_genres() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let genres = composer.genres();
        assert!(!genres.is_empty(), "genres should be non-empty");
        assert!(
            genres.iter().any(|g| g.id == "low_fantasy_folklore"),
            "genres should contain 'low_fantasy_folklore'"
        );

        for g in &genres {
            println!(
                "  {} — {} archetypes, {} profiles, {} dynamics",
                g.id, g.archetype_count, g.profile_count, g.dynamic_count
            );
        }
    }

    #[test]
    fn profiles_filtered_by_genre() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        for genre in &composer.descriptors.genres {
            let profiles = composer.profiles_for_genre(&genre.id);
            for p in &profiles {
                assert!(
                    genre.valid_profiles.contains(&p.id),
                    "profile '{}' returned for genre '{}' but not in valid_profiles",
                    p.id,
                    genre.id
                );
            }
            println!(
                "  genre '{}' — {} profiles returned",
                genre.id,
                profiles.len()
            );
        }
    }

    #[test]
    fn archetypes_filtered_by_genre() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        for genre in &composer.descriptors.genres {
            let archetypes = composer.archetypes_for_genre(&genre.id);
            for a in &archetypes {
                assert!(
                    genre.valid_archetypes.contains(&a.id),
                    "archetype '{}' returned for genre '{}' but not in valid_archetypes",
                    a.id,
                    genre.id
                );
            }
            println!(
                "  genre '{}' — {} archetypes returned",
                genre.id,
                archetypes.len()
            );
        }
    }

    #[test]
    fn dynamics_filtered_by_genre() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        for genre in &composer.descriptors.genres {
            let dynamics = composer.dynamics_for_genre(&genre.id, &[]);
            for d in &dynamics {
                assert!(
                    genre.valid_dynamics.contains(&d.id),
                    "dynamic '{}' returned for genre '{}' but not in valid_dynamics",
                    d.id,
                    genre.id
                );
            }
            println!(
                "  genre '{}' — {} dynamics returned",
                genre.id,
                dynamics.len()
            );
        }
    }

    #[test]
    fn invalid_genre_returns_empty() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        assert!(composer.profiles_for_genre("nonexistent_genre").is_empty());
        assert!(composer
            .archetypes_for_genre("nonexistent_genre")
            .is_empty());
        assert!(composer
            .dynamics_for_genre("nonexistent_genre", &[])
            .is_empty());
        assert!(composer.names_for_genre("nonexistent_genre").is_empty());
    }
}
