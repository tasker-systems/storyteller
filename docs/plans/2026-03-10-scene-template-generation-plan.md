# Scene Template Generation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a descriptor-driven scene composition system that produces playable scenes from the existing training data matrix, with session persistence and a workshop UI for scene setup.

**Architecture:** A `SceneComposer` in storyteller-engine loads JSON descriptors from `STORYTELLER_DATA_PATH`, exposes catalog queries for cascading UI selection, and composes `SceneData` + `Vec<CharacterSheet>` from user selections. Sessions persist as flat JSON files in `.story/sessions/`. Workshop Tauri commands bridge composer to a new Svelte scene-setup wizard.

**Tech Stack:** Rust (serde, rand), Tauri 2 commands, SvelteKit 5 frontend, Python (httpx, beautifulsoup4) for name harvesting

**Spec:** `docs/plans/2026-03-10-scene-template-generation-design.md`

---

## Chunk 1: Descriptor Types & Loading

### Task 1: Descriptor serde types

**Files:**
- Create: `crates/storyteller-engine/src/scene_composer/mod.rs`
- Create: `crates/storyteller-engine/src/scene_composer/descriptors.rs`
- Modify: `crates/storyteller-engine/src/lib.rs:19-26` (add module)
- Modify: `crates/storyteller-engine/Cargo.toml:12` (add rand dependency)

These types mirror the JSON structure in `storyteller-data/training-data/descriptors/`. They are deserialization-only — we don't write these files, just read them.

- [ ] **Step 1: Add rand dependency to storyteller-engine**

In `crates/storyteller-engine/Cargo.toml`, add under `[dependencies]`:
```toml
rand = { workspace = true }
```

- [ ] **Step 2: Create mod.rs with module declarations**

```rust
// crates/storyteller-engine/src/scene_composer/mod.rs
//! Scene composer — builds playable scenes from training data descriptors.
//!
//! Loads archetype, genre, profile, dynamic, and name descriptors from
//! `STORYTELLER_DATA_PATH/training-data/descriptors/` and composes
//! `SceneData` + `Vec<CharacterSheet>` from user selections.

pub mod descriptors;

pub use descriptors::DescriptorSet;
```

- [ ] **Step 3: Register module in engine lib.rs**

Add `pub mod scene_composer;` to `crates/storyteller-engine/src/lib.rs` after the `workshop` line (line 26).

- [ ] **Step 4: Write descriptor types in descriptors.rs**

Create `crates/storyteller-engine/src/scene_composer/descriptors.rs` with serde structs matching every descriptor JSON file. Key types:

```rust
//! Descriptor types — serde structs for the training data descriptor JSONs.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

// ---- Shared ----

/// A numeric range with min/max bounds, used throughout descriptors.
#[derive(Debug, Clone, Deserialize)]
pub struct RangeBounds {
    pub min: f64,
    pub max: f64,
}

// ---- Archetypes ----

#[derive(Debug, Clone, Deserialize)]
pub struct ArchetypesFile {
    pub archetypes: Vec<Archetype>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Archetype {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub axes: Vec<ArchetypeAxis>,
    pub default_emotional_profile: DefaultEmotionalProfile,
    pub default_self_edge: DefaultSelfEdge,
    pub action_tendencies: ActionTendencies,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchetypeAxis {
    pub axis_id: String,
    pub central_tendency: RangeBounds,
    pub variance: RangeBounds,
    pub range_low: RangeBounds,
    pub range_high: RangeBounds,
    pub layer: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultEmotionalProfile {
    pub grammar_id: String,
    pub primaries: Vec<EmotionalPrimaryRange>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmotionalPrimaryRange {
    pub primary_id: String,
    pub intensity: RangeBounds,
    pub awareness: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultSelfEdge {
    pub trust_competence: RangeBounds,
    pub trust_intentions: RangeBounds,
    pub trust_reliability: RangeBounds,
    pub affection: RangeBounds,
    pub debt: RangeBounds,
    pub history_weight: RangeBounds,
    pub projection_accuracy: RangeBounds,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionTendencies {
    pub primary_action_types: Vec<String>,
    pub primary_action_contexts: Vec<String>,
    pub speech_likelihood: f64,
    pub speech_registers: Vec<String>,
    pub default_awareness: String,
}

// ---- Genres ----

#[derive(Debug, Clone, Deserialize)]
pub struct GenresFile {
    pub genres: Vec<Genre>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Genre {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub valid_archetypes: Vec<String>,
    pub valid_dynamics: Vec<String>,
    pub valid_profiles: Vec<String>,
    pub excluded_combinations: Vec<ExcludedCombination>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExcludedCombination {
    pub archetype: Option<String>,
    pub dynamic: Option<String>,
    pub profile: Option<String>,
    pub reason: String,
}

// ---- Profiles ----

#[derive(Debug, Clone, Deserialize)]
pub struct ProfilesFile {
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub scene_type: String,
    pub tension: RangeBounds,
    pub cast_size: RangeBounds,
    pub characteristic_events: Vec<CharacteristicEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharacteristicEvent {
    pub event_type: String,
    pub emotional_register: String,
    pub weight: f64,
}

// ---- Dynamics ----

#[derive(Debug, Clone, Deserialize)]
pub struct DynamicsFile {
    pub dynamics: Vec<Dynamic>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dynamic {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
    pub edge_a_to_b: EdgeWeights,
    pub edge_b_to_a: EdgeWeights,
    pub topology_a: String,
    pub topology_b: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EdgeWeights {
    pub trust_reliability: RangeBounds,
    pub trust_competence: RangeBounds,
    pub trust_benevolence: RangeBounds,
    pub affection: RangeBounds,
    pub debt: RangeBounds,
}

// ---- Axis Vocabulary ----

#[derive(Debug, Clone, Deserialize)]
pub struct AxisVocabularyFile {
    pub axes: Vec<AxisDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AxisDefinition {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub typical_layer: String,
    pub category: String,
    pub tags: Vec<String>,
}

// ---- Cross-Dimensions ----

#[derive(Debug, Clone, Deserialize)]
pub struct CrossDimensionsFile {
    pub dimensions: Vec<CrossDimension>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrossDimension {
    pub id: String,
    pub display_name: String,
    pub values: Vec<DimensionValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DimensionValue {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub enables_primordial: bool,
    #[serde(default)]
    pub axis_modifiers: Vec<AxisModifier>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AxisModifier {
    pub axis_id: String,
    pub additive: f64,
}

// ---- Names ----

/// Genre-keyed name pools. File may not exist yet (names.json).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NamesFile {
    #[serde(flatten)]
    pub genres: BTreeMap<String, NamePool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NamePool {
    pub names: Vec<String>,
    #[serde(default)]
    pub source: String,
}

// ---- Settings ----

/// Genre-keyed setting templates. File may not exist yet (settings.json).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SettingsFile {
    #[serde(flatten)]
    pub genres: BTreeMap<String, GenreSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenreSettings {
    #[serde(default)]
    pub default_setting: Option<DefaultSetting>,
    #[serde(flatten)]
    pub profiles: BTreeMap<String, ProfileSetting>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultSetting {
    #[serde(default)]
    pub sensory_palette: Vec<String>,
    #[serde(default)]
    pub time_options: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileSetting {
    #[serde(default)]
    pub description_templates: Vec<String>,
    #[serde(default)]
    pub affordances: Vec<String>,
}

// ---- Unified Descriptor Set ----

/// All descriptors loaded from STORYTELLER_DATA_PATH.
#[derive(Debug, Clone)]
pub struct DescriptorSet {
    pub archetypes: Vec<Archetype>,
    pub genres: Vec<Genre>,
    pub profiles: Vec<Profile>,
    pub dynamics: Vec<Dynamic>,
    pub axes: Vec<AxisDefinition>,
    pub cross_dimensions: Vec<CrossDimension>,
    pub names: NamesFile,
    pub settings: SettingsFile,
}

impl DescriptorSet {
    /// Load all descriptors from the given base directory.
    ///
    /// Expects `base_dir/training-data/descriptors/` to contain:
    /// - archetypes.json (required)
    /// - genres.json (required)
    /// - profiles.json (required)
    /// - dynamics.json (required)
    /// - axis-vocabulary.json (required)
    /// - cross-dimensions.json (required)
    /// - names.json (optional — empty if missing)
    /// - settings.json (optional — empty if missing)
    pub fn load(base_dir: &Path) -> Result<Self, String> {
        let desc_dir = base_dir.join("training-data").join("descriptors");

        let load_required = |filename: &str| -> Result<String, String> {
            let path = desc_dir.join(filename);
            std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {e}", path.display()))
        };

        let load_optional = |filename: &str| -> Option<String> {
            let path = desc_dir.join(filename);
            std::fs::read_to_string(&path).ok()
        };

        let archetypes: ArchetypesFile = serde_json::from_str(&load_required("archetypes.json")?)
            .map_err(|e| format!("Failed to parse archetypes.json: {e}"))?;
        let genres: GenresFile = serde_json::from_str(&load_required("genres.json")?)
            .map_err(|e| format!("Failed to parse genres.json: {e}"))?;
        let profiles: ProfilesFile = serde_json::from_str(&load_required("profiles.json")?)
            .map_err(|e| format!("Failed to parse profiles.json: {e}"))?;
        let dynamics: DynamicsFile = serde_json::from_str(&load_required("dynamics.json")?)
            .map_err(|e| format!("Failed to parse dynamics.json: {e}"))?;
        let axes: AxisVocabularyFile =
            serde_json::from_str(&load_required("axis-vocabulary.json")?)
                .map_err(|e| format!("Failed to parse axis-vocabulary.json: {e}"))?;
        let cross_dimensions: CrossDimensionsFile =
            serde_json::from_str(&load_required("cross-dimensions.json")?)
                .map_err(|e| format!("Failed to parse cross-dimensions.json: {e}"))?;

        let names: NamesFile = load_optional("names.json")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let settings: SettingsFile = load_optional("settings.json")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Self {
            archetypes: archetypes.archetypes,
            genres: genres.genres,
            profiles: profiles.profiles,
            dynamics: dynamics.dynamics,
            axes: axes.axes,
            cross_dimensions: cross_dimensions.dimensions,
            names,
            settings,
        })
    }
}
```

- [ ] **Step 5: Write tests for descriptor loading**

Add to the bottom of `descriptors.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn data_path() -> Option<std::path::PathBuf> {
        std::env::var("STORYTELLER_DATA_PATH").ok().map(std::path::PathBuf::from)
    }

    #[test]
    fn load_descriptors_from_data_path() {
        let Some(path) = data_path() else {
            eprintln!("STORYTELLER_DATA_PATH not set, skipping");
            return;
        };
        let set = DescriptorSet::load(&path).expect("should load descriptors");
        assert!(!set.archetypes.is_empty(), "should have archetypes");
        assert!(!set.genres.is_empty(), "should have genres");
        assert!(!set.profiles.is_empty(), "should have profiles");
        assert!(!set.dynamics.is_empty(), "should have dynamics");
        assert!(!set.axes.is_empty(), "should have axes");
        assert!(!set.cross_dimensions.is_empty(), "should have cross_dimensions");
    }

    #[test]
    fn archetypes_have_axes_and_emotional_profiles() {
        let Some(path) = data_path() else { return };
        let set = DescriptorSet::load(&path).unwrap();
        for arch in &set.archetypes {
            assert!(!arch.axes.is_empty(), "{} should have axes", arch.id);
            assert!(
                !arch.default_emotional_profile.primaries.is_empty(),
                "{} should have emotional primaries",
                arch.id,
            );
        }
    }

    #[test]
    fn genres_reference_valid_archetypes() {
        let Some(path) = data_path() else { return };
        let set = DescriptorSet::load(&path).unwrap();
        let arch_ids: Vec<&str> = set.archetypes.iter().map(|a| a.id.as_str()).collect();
        for genre in &set.genres {
            for va in &genre.valid_archetypes {
                assert!(
                    arch_ids.contains(&va.as_str()),
                    "genre {} references unknown archetype {}",
                    genre.id,
                    va,
                );
            }
        }
    }

    #[test]
    fn genres_reference_valid_dynamics() {
        let Some(path) = data_path() else { return };
        let set = DescriptorSet::load(&path).unwrap();
        let dyn_ids: Vec<&str> = set.dynamics.iter().map(|d| d.id.as_str()).collect();
        for genre in &set.genres {
            for vd in &genre.valid_dynamics {
                assert!(
                    dyn_ids.contains(&vd.as_str()),
                    "genre {} references unknown dynamic {}",
                    genre.id,
                    vd,
                );
            }
        }
    }

    #[test]
    fn genres_reference_valid_profiles() {
        let Some(path) = data_path() else { return };
        let set = DescriptorSet::load(&path).unwrap();
        let prof_ids: Vec<&str> = set.profiles.iter().map(|p| p.id.as_str()).collect();
        for genre in &set.genres {
            for vp in &genre.valid_profiles {
                assert!(
                    prof_ids.contains(&vp.as_str()),
                    "genre {} references unknown profile {}",
                    genre.id,
                    vp,
                );
            }
        }
    }
}
```

- [ ] **Step 6: Run tests to verify descriptor loading**

Run: `cd /Users/petetaylor/projects/tasker-systems/storyteller && cargo test -p storyteller-engine scene_composer --all-features -- --nocapture`

Expected: All tests PASS (assuming `STORYTELLER_DATA_PATH` is set in `.env`).

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/ crates/storyteller-engine/src/lib.rs crates/storyteller-engine/Cargo.toml
git commit -m "feat(engine): add scene_composer module with descriptor loading"
```

### Task 2: Catalog queries

**Files:**
- Create: `crates/storyteller-engine/src/scene_composer/catalog.rs`
- Modify: `crates/storyteller-engine/src/scene_composer/mod.rs`

The catalog exposes filtered option queries that drive the frontend's cascading dropdowns.

- [ ] **Step 1: Create catalog.rs with SceneComposer struct and genre query**

```rust
// crates/storyteller-engine/src/scene_composer/catalog.rs
//! Catalog queries — filtered option space for scene setup UI.

use serde::Serialize;

use super::descriptors::{Archetype, DescriptorSet, Dynamic, Genre, Profile};

/// Holds loaded descriptors and provides filtered catalog queries.
#[derive(Debug, Clone)]
pub struct SceneComposer {
    pub(crate) descriptors: DescriptorSet,
}

/// Summary info about a genre for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct GenreSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub archetype_count: usize,
    pub profile_count: usize,
    pub dynamic_count: usize,
}

/// Summary info about a profile for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub scene_type: String,
    pub tension_min: f64,
    pub tension_max: f64,
    pub cast_size_min: usize,
    pub cast_size_max: usize,
}

/// Summary info about an archetype for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ArchetypeSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
}

/// Summary info about a relational dynamic for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct DynamicSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
}

impl SceneComposer {
    /// Create a new SceneComposer by loading descriptors from the given path.
    pub fn load(data_path: &std::path::Path) -> Result<Self, String> {
        let descriptors = DescriptorSet::load(data_path)?;
        Ok(Self { descriptors })
    }

    /// List all available genres.
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

    /// Get valid profiles for a genre.
    pub fn profiles_for_genre(&self, genre_id: &str) -> Vec<ProfileSummary> {
        let Some(genre) = self.find_genre(genre_id) else {
            return vec![];
        };
        self.descriptors
            .profiles
            .iter()
            .filter(|p| genre.valid_profiles.contains(&p.id))
            .map(|p| ProfileSummary {
                id: p.id.clone(),
                display_name: p.display_name.clone(),
                description: p.description.clone(),
                scene_type: p.scene_type.clone(),
                tension_min: p.tension.min,
                tension_max: p.tension.max,
                cast_size_min: p.cast_size.min as usize,
                cast_size_max: p.cast_size.max as usize,
            })
            .collect()
    }

    /// Get valid archetypes for a genre.
    pub fn archetypes_for_genre(&self, genre_id: &str) -> Vec<ArchetypeSummary> {
        let Some(genre) = self.find_genre(genre_id) else {
            return vec![];
        };
        self.descriptors
            .archetypes
            .iter()
            .filter(|a| genre.valid_archetypes.contains(&a.id))
            .map(|a| ArchetypeSummary {
                id: a.id.clone(),
                display_name: a.display_name.clone(),
                description: a.description.clone(),
            })
            .collect()
    }

    /// Get valid dynamics for a genre, optionally filtered by selected archetypes.
    ///
    /// If `selected_archetypes` is non-empty, excludes dynamics that are
    /// in the genre's excluded_combinations for those archetypes.
    pub fn dynamics_for_genre(
        &self,
        genre_id: &str,
        selected_archetypes: &[String],
    ) -> Vec<DynamicSummary> {
        let Some(genre) = self.find_genre(genre_id) else {
            return vec![];
        };
        self.descriptors
            .dynamics
            .iter()
            .filter(|d| genre.valid_dynamics.contains(&d.id))
            .filter(|d| {
                // Check excluded combinations
                !genre.excluded_combinations.iter().any(|exc| {
                    let dynamic_matches =
                        exc.dynamic.as_ref().is_some_and(|ed| ed == &d.id);
                    let archetype_matches = exc.archetype.as_ref().is_some_and(|ea| {
                        selected_archetypes.contains(ea)
                    });
                    dynamic_matches && archetype_matches
                })
            })
            .map(|d| DynamicSummary {
                id: d.id.clone(),
                display_name: d.display_name.clone(),
                description: d.description.clone(),
                role_a: d.role_a.clone(),
                role_b: d.role_b.clone(),
            })
            .collect()
    }

    /// Get available names for a genre.
    pub fn names_for_genre(&self, genre_id: &str) -> Vec<String> {
        self.descriptors
            .names
            .genres
            .get(genre_id)
            .map(|pool| pool.names.clone())
            .unwrap_or_default()
    }

    fn find_genre(&self, genre_id: &str) -> Option<&Genre> {
        self.descriptors.genres.iter().find(|g| g.id == genre_id)
    }

    pub(crate) fn find_archetype(&self, id: &str) -> Option<&Archetype> {
        self.descriptors.archetypes.iter().find(|a| a.id == id)
    }

    pub(crate) fn find_profile(&self, id: &str) -> Option<&Profile> {
        self.descriptors.profiles.iter().find(|p| p.id == id)
    }

    pub(crate) fn find_dynamic(&self, id: &str) -> Option<&Dynamic> {
        self.descriptors.dynamics.iter().find(|d| d.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn composer() -> Option<SceneComposer> {
        let path = std::env::var("STORYTELLER_DATA_PATH").ok()?;
        SceneComposer::load(std::path::Path::new(&path)).ok()
    }

    #[test]
    fn genres_returns_all_genres() {
        let Some(c) = composer() else { return };
        let genres = c.genres();
        assert!(!genres.is_empty());
        // low_fantasy_folklore should exist
        assert!(genres.iter().any(|g| g.id == "low_fantasy_folklore"));
    }

    #[test]
    fn profiles_filtered_by_genre() {
        let Some(c) = composer() else { return };
        let profiles = c.profiles_for_genre("low_fantasy_folklore");
        assert!(!profiles.is_empty());
        // All returned profiles should be in the genre's valid list
        let genre = c.find_genre("low_fantasy_folklore").unwrap();
        for p in &profiles {
            assert!(genre.valid_profiles.contains(&p.id));
        }
    }

    #[test]
    fn archetypes_filtered_by_genre() {
        let Some(c) = composer() else { return };
        let archetypes = c.archetypes_for_genre("low_fantasy_folklore");
        assert!(!archetypes.is_empty());
        let genre = c.find_genre("low_fantasy_folklore").unwrap();
        for a in &archetypes {
            assert!(genre.valid_archetypes.contains(&a.id));
        }
    }

    #[test]
    fn dynamics_filtered_by_genre() {
        let Some(c) = composer() else { return };
        let dynamics = c.dynamics_for_genre("low_fantasy_folklore", &[]);
        assert!(!dynamics.is_empty());
        let genre = c.find_genre("low_fantasy_folklore").unwrap();
        for d in &dynamics {
            assert!(genre.valid_dynamics.contains(&d.id));
        }
    }

    #[test]
    fn invalid_genre_returns_empty() {
        let Some(c) = composer() else { return };
        assert!(c.profiles_for_genre("nonexistent").is_empty());
        assert!(c.archetypes_for_genre("nonexistent").is_empty());
        assert!(c.dynamics_for_genre("nonexistent", &[]).is_empty());
    }
}
```

- [ ] **Step 2: Update mod.rs to include catalog**

Add to `crates/storyteller-engine/src/scene_composer/mod.rs`:

```rust
pub mod catalog;

pub use catalog::SceneComposer;
// (keep existing pub use descriptors::DescriptorSet)
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p storyteller-engine scene_composer --all-features -- --nocapture`

Expected: All catalog tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/
git commit -m "feat(engine): catalog queries for genre-filtered scene options"
```

### Task 3: Scene composition

**Files:**
- Create: `crates/storyteller-engine/src/scene_composer/compose.rs`
- Create: `crates/storyteller-engine/src/scene_composer/names.rs`
- Modify: `crates/storyteller-engine/src/scene_composer/mod.rs`

This is the core logic — takes `SceneSelections` and produces `(SceneData, Vec<CharacterSheet>)`.

- [ ] **Step 1: Create names.rs for name pool management**

```rust
// crates/storyteller-engine/src/scene_composer/names.rs
//! Name pool — genre-appropriate name selection with deduplication.

use rand::seq::SliceRandom;
use rand::Rng;

/// Select `count` unique names from the pool, using fallback names if pool is too small.
pub fn select_names<R: Rng>(pool: &[String], count: usize, rng: &mut R) -> Vec<String> {
    if pool.len() >= count {
        let mut names = pool.to_vec();
        names.shuffle(rng);
        names.truncate(count);
        names
    } else {
        // Use what we have, then generate fallbacks
        let mut names = pool.to_vec();
        for i in names.len()..count {
            names.push(format!("Character {}", i + 1));
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_names_from_pool() {
        let pool: Vec<String> = (1..=10).map(|i| format!("Name{i}")).collect();
        let mut rng = rand::rng();
        let names = select_names(&pool, 3, &mut rng);
        assert_eq!(names.len(), 3);
        // All unique
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn select_names_with_fallback() {
        let pool: Vec<String> = vec!["Alice".to_string()];
        let mut rng = rand::rng();
        let names = select_names(&pool, 3, &mut rng);
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "Alice");
    }

    #[test]
    fn select_names_deterministic_with_seed() {
        let pool: Vec<String> = (1..=20).map(|i| format!("Name{i}")).collect();
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let names1 = select_names(&pool, 3, &mut rng1);
        let names2 = select_names(&pool, 3, &mut rng2);
        assert_eq!(names1, names2);
    }
}
```

- [ ] **Step 2: Create compose.rs with SceneSelections and composition logic**

```rust
// crates/storyteller-engine/src/scene_composer/compose.rs
//! Scene composition — transforms selections into SceneData + CharacterSheets.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::types::character::{
    CastEntry, CharacterSheet, CharacterTensor, EmotionalPrimary, EmotionalState,
    SceneConstraints, SceneData, SceneSetting, SelfEdge, SelfEdgeTrust, SelfKnowledge,
};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::scene::{SceneId, SceneType};
use storyteller_core::types::tensor::{AwarenessLevel, AxisValue, Provenance, TemporalLayer};
use storyteller_core::types::world_model::{Attribute, CapabilityProfile, Skill};

use super::catalog::SceneComposer;
use super::descriptors::{Archetype, Dynamic, Profile, RangeBounds};
use super::names::select_names;

/// User's scene setup selections — needs both Serialize (for session persistence)
/// and Deserialize (for loading from JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSelections {
    pub genre: String,
    pub profile: String,
    pub cast: Vec<CastSelection>,
    pub dynamics: Vec<DynamicSelection>,
    pub setting_override: Option<String>,
    pub seed: Option<u64>,
}

/// A single cast member's selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastSelection {
    pub archetype: String,
    pub name: String,
    pub is_player_perspective: bool,
}

/// A relational dynamic between two cast members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSelection {
    pub character_a_index: usize,
    pub character_b_index: usize,
    pub dynamic: String,
}

/// Output of scene composition.
#[derive(Debug, Clone)]
pub struct ComposedScene {
    pub scene: SceneData,
    pub characters: Vec<CharacterSheet>,
    pub selections: SceneSelections,
}

impl SceneComposer {
    /// Compose a playable scene from user selections.
    pub fn compose(&self, selections: &SceneSelections) -> Result<ComposedScene, String> {
        let mut rng: Box<dyn rand::RngCore> = match selections.seed {
            Some(seed) => Box::new(StdRng::seed_from_u64(seed)),
            None => Box::new(rand::rng()),
        };

        let profile = self
            .find_profile(&selections.profile)
            .ok_or_else(|| format!("Unknown profile: {}", selections.profile))?;

        // Build entity IDs and character sheets
        let mut characters = Vec::new();
        let mut cast_entries = Vec::new();

        for cs in &selections.cast {
            let archetype = self
                .find_archetype(&cs.archetype)
                .ok_or_else(|| format!("Unknown archetype: {}", cs.archetype))?;

            let entity_id = EntityId::new();
            let sheet = compose_character(archetype, &cs.name, entity_id, &mut *rng);
            cast_entries.push(CastEntry {
                entity_id,
                name: cs.name.clone(),
                role: archetype.description.clone(),
            });
            characters.push(sheet);
        }

        // Build scene data
        let scene_type = match profile.scene_type.as_str() {
            "Gravitational" => SceneType::Gravitational,
            "Threshold" => SceneType::Threshold,
            "Connective" => SceneType::Connective,
            "Gate" => SceneType::Gate,
            _ => SceneType::Gravitational,
        };

        let setting = compose_setting(
            &self.descriptors,
            &selections.genre,
            &selections.profile,
            selections.setting_override.as_deref(),
            &mut *rng,
        );

        let scene = SceneData {
            scene_id: SceneId::new(),
            title: format!(
                "{} — {}",
                profile.display_name,
                selections
                    .cast
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" & ")
            ),
            scene_type,
            setting,
            cast: cast_entries,
            stakes: compose_stakes(profile, &selections.cast),
            constraints: compose_constraints(profile),
            emotional_arc: vec![
                "1. Opening — establishing the dynamic".to_string(),
                "2. Development — tension emerges from the interaction".to_string(),
                "3. Pivot — something shifts in understanding".to_string(),
                "4. Resolution or departure — the scene finds its shape".to_string(),
            ],
            evaluation_criteria: vec![
                "Tone consistency with genre and profile".to_string(),
                "Character voices remain distinct".to_string(),
                "Information boundaries respected".to_string(),
                "Subtext carries more weight than surface dialogue".to_string(),
            ],
        };

        Ok(ComposedScene {
            scene,
            characters,
            selections: selections.clone(),
        })
    }
}

fn compose_character<R: Rng>(
    archetype: &Archetype,
    name: &str,
    entity_id: EntityId,
    rng: &mut R,
) -> CharacterSheet {
    let mut tensor = CharacterTensor::new();

    for axis in &archetype.axes {
        let ct = sample_in_range(&axis.central_tendency, rng);
        let var = sample_in_range(&axis.variance, rng);
        let rl = sample_in_range(&axis.range_low, rng);
        let rh = sample_in_range(&axis.range_high, rng);

        let layer = match axis.layer.as_str() {
            "bedrock" => TemporalLayer::Bedrock,
            "sediment" => TemporalLayer::Sediment,
            "topsoil" => TemporalLayer::Topsoil,
            _ => TemporalLayer::Sediment,
        };

        tensor.insert(
            &axis.axis_id,
            AxisValue {
                central_tendency: ct as f32,
                variance: var as f32,
                range_low: rl as f32,
                range_high: rh as f32,
            },
            layer,
            Provenance::Generated,
        );
    }

    let emotional_state = compose_emotional_state(archetype, rng);
    let self_edge = compose_self_edge(archetype, rng);

    CharacterSheet {
        entity_id,
        name: name.to_string(),
        voice: format!(
            "Voice shaped by the {} archetype — {}",
            archetype.display_name,
            archetype.description.chars().take(100).collect::<String>()
        ),
        backstory: format!(
            "A character of the {} archetype. {}",
            archetype.display_name, archetype.description,
        ),
        tensor,
        grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
        emotional_state,
        self_edge,
        triggers: vec![], // Triggers are scene-specific, not archetype-derived
        performance_notes: format!(
            "Play this character as a {} — {}.",
            archetype.display_name, archetype.description,
        ),
        knows: vec!["Their own circumstances and immediate context".to_string()],
        does_not_know: vec!["The other characters' inner states and motivations".to_string()],
        capabilities: CapabilityProfile::default(),
    }
}

fn compose_emotional_state<R: Rng>(archetype: &Archetype, rng: &mut R) -> EmotionalState {
    let primaries = archetype
        .default_emotional_profile
        .primaries
        .iter()
        .map(|p| {
            let intensity = sample_in_range(&p.intensity, rng) as f32;
            let awareness = match p.awareness.as_str() {
                "articulate" => AwarenessLevel::Articulate,
                "recognizable" => AwarenessLevel::Recognizable,
                "preconscious" => AwarenessLevel::Preconscious,
                "defended" => AwarenessLevel::Defended,
                "structural" => AwarenessLevel::Structural,
                _ => AwarenessLevel::Recognizable,
            };
            EmotionalPrimary {
                primary_id: p.primary_id.clone(),
                intensity,
                awareness,
            }
        })
        .collect();

    EmotionalState {
        grammar_id: archetype.default_emotional_profile.grammar_id.clone(),
        primaries,
        mood_vector_notes: vec![],
    }
}

fn compose_self_edge<R: Rng>(archetype: &Archetype, rng: &mut R) -> SelfEdge {
    let se = &archetype.default_self_edge;
    SelfEdge {
        trust: SelfEdgeTrust {
            competence: sample_in_range(&se.trust_competence, rng) as f32,
            intentions: sample_in_range(&se.trust_intentions, rng) as f32,
            reliability: sample_in_range(&se.trust_reliability, rng) as f32,
        },
        affection: sample_in_range(&se.affection, rng) as f32,
        debt: sample_in_range(&se.debt, rng) as f32,
        history_pattern: String::new(),
        history_weight: sample_in_range(&se.history_weight, rng) as f32,
        projection_content: String::new(),
        projection_accuracy: sample_in_range(&se.projection_accuracy, rng) as f32,
        self_knowledge: SelfKnowledge {
            knows: vec![],
            does_not_know: vec![],
        },
    }
}

fn compose_setting(
    descriptors: &super::descriptors::DescriptorSet,
    genre_id: &str,
    profile_id: &str,
    override_text: Option<&str>,
    rng: &mut dyn Rng,
) -> SceneSetting {
    if let Some(text) = override_text {
        return SceneSetting {
            description: text.to_string(),
            affordances: vec![],
            sensory_details: vec![],
            aesthetic_detail: String::new(),
        };
    }

    // Try to find genre-specific setting templates
    let genre_settings = descriptors.settings.genres.get(genre_id);

    let description = genre_settings
        .and_then(|gs| gs.profiles.get(profile_id))
        .and_then(|ps| {
            if ps.description_templates.is_empty() {
                None
            } else {
                let idx = rng.random_range(0..ps.description_templates.len());
                Some(ps.description_templates[idx].clone())
            }
        })
        .unwrap_or_else(|| "A place where two paths cross.".to_string());

    let affordances = genre_settings
        .and_then(|gs| gs.profiles.get(profile_id))
        .map(|ps| ps.affordances.clone())
        .unwrap_or_default();

    let sensory_details = genre_settings
        .and_then(|gs| gs.default_setting.as_ref())
        .map(|ds| ds.sensory_palette.clone())
        .unwrap_or_default();

    SceneSetting {
        description,
        affordances,
        sensory_details,
        aesthetic_detail: String::new(),
    }
}

fn compose_stakes(profile: &Profile, cast: &[CastSelection]) -> Vec<String> {
    let mut stakes = Vec::new();
    for cs in cast {
        stakes.push(format!(
            "For {}: what this {} scene means for a {}",
            cs.name, profile.display_name, cs.archetype
        ));
    }
    stakes
}

fn compose_constraints(profile: &Profile) -> SceneConstraints {
    SceneConstraints {
        hard: vec![format!(
            "Scene tension range: {:.1} - {:.1}",
            profile.tension.min, profile.tension.max
        )],
        soft: vec!["Characters respond authentically to each other's actions".to_string()],
        perceptual: vec!["Each character perceives the scene from their own vantage".to_string()],
    }
}

fn sample_in_range<R: Rng>(bounds: &RangeBounds, rng: &mut R) -> f64 {
    if bounds.min >= bounds.max {
        return bounds.min;
    }
    rng.random_range(bounds.min..=bounds.max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn composer() -> Option<SceneComposer> {
        let path = std::env::var("STORYTELLER_DATA_PATH").ok()?;
        SceneComposer::load(std::path::Path::new(&path)).ok()
    }

    #[test]
    fn compose_basic_scene() {
        let Some(c) = composer() else { return };
        let selections = SceneSelections {
            genre: "low_fantasy_folklore".to_string(),
            profile: "quiet_reunion".to_string(),
            cast: vec![
                CastSelection {
                    archetype: "wandering_artist".to_string(),
                    name: "Aelric".to_string(),
                    is_player_perspective: true,
                },
                CastSelection {
                    archetype: "stoic_survivor".to_string(),
                    name: "Maren".to_string(),
                    is_player_perspective: false,
                },
            ],
            dynamics: vec![DynamicSelection {
                character_a_index: 0,
                character_b_index: 1,
                dynamic: "former_friends_estranged".to_string(),
            }],
            setting_override: None,
            seed: Some(42),
        };

        let result = c.compose(&selections).unwrap();
        assert_eq!(result.characters.len(), 2);
        assert_eq!(result.characters[0].name, "Aelric");
        assert_eq!(result.characters[1].name, "Maren");
        assert!(!result.scene.title.is_empty());
        assert_eq!(result.scene.cast.len(), 2);
    }

    #[test]
    fn compose_is_deterministic_with_seed() {
        let Some(c) = composer() else { return };
        let selections = SceneSelections {
            genre: "low_fantasy_folklore".to_string(),
            profile: "quiet_reunion".to_string(),
            cast: vec![
                CastSelection {
                    archetype: "wandering_artist".to_string(),
                    name: "Aelric".to_string(),
                    is_player_perspective: true,
                },
                CastSelection {
                    archetype: "stoic_survivor".to_string(),
                    name: "Maren".to_string(),
                    is_player_perspective: false,
                },
            ],
            dynamics: vec![],
            setting_override: None,
            seed: Some(42),
        };

        let r1 = c.compose(&selections).unwrap();
        let r2 = c.compose(&selections).unwrap();

        // Same seed → same tensor values
        let t1 = r1.characters[0].tensor.get("joy_wonder");
        let t2 = r2.characters[0].tensor.get("joy_wonder");
        assert_eq!(
            t1.map(|t| t.value.central_tendency),
            t2.map(|t| t.value.central_tendency),
        );
    }

    #[test]
    fn compose_with_setting_override() {
        let Some(c) = composer() else { return };
        let selections = SceneSelections {
            genre: "low_fantasy_folklore".to_string(),
            profile: "quiet_reunion".to_string(),
            cast: vec![CastSelection {
                archetype: "wandering_artist".to_string(),
                name: "Aelric".to_string(),
                is_player_perspective: true,
            }],
            dynamics: vec![],
            setting_override: Some("A moonlit clearing in the forest.".to_string()),
            seed: Some(42),
        };

        let result = c.compose(&selections).unwrap();
        assert_eq!(
            result.scene.setting.description,
            "A moonlit clearing in the forest."
        );
    }
}
```

- [ ] **Step 3: Update mod.rs**

```rust
// crates/storyteller-engine/src/scene_composer/mod.rs
//! Scene composer — builds playable scenes from training data descriptors.

pub mod catalog;
pub mod compose;
pub mod descriptors;
pub mod names;

pub use catalog::SceneComposer;
pub use compose::{CastSelection, ComposedScene, DynamicSelection, SceneSelections};
pub use descriptors::DescriptorSet;
```

- [ ] **Step 4: Run all scene_composer tests**

Run: `cargo test -p storyteller-engine scene_composer --all-features -- --nocapture`

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/
git commit -m "feat(engine): scene composition from archetype + profile + genre selections"
```

## Chunk 2: Session Persistence & Workshop Integration

### Task 4: Session persistence

**Files:**
- Create: `crates/storyteller-workshop/src-tauri/src/session.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs`

Session persistence to `.story/sessions/` using flat JSON files.

- [ ] **Step 1: Create session.rs**

```rust
// crates/storyteller-workshop/src-tauri/src/session.rs
//! Session persistence — flat-file JSON storage in .story/sessions/.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_engine::scene_composer::SceneSelections;

/// Summary of a persisted session, for listing in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub genre: String,
    pub profile: String,
    pub title: String,
    pub cast_names: Vec<String>,
    pub turn_count: usize,
}

/// Manages session storage in .story/sessions/.
#[derive(Debug, Clone)]
pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    /// Create a new session store. Creates .story/sessions/ if needed.
    pub fn new(workshop_root: &Path) -> Result<Self, String> {
        let base_dir = workshop_root.join(".story").join("sessions");
        fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create sessions dir: {e}"))?;

        // Ensure .gitignore exists
        let gitignore = workshop_root.join(".story").join(".gitignore");
        if !gitignore.exists() {
            fs::write(&gitignore, "*\n")
                .map_err(|e| format!("Failed to create .gitignore: {e}"))?;
        }

        Ok(Self { base_dir })
    }

    /// Create a new session directory and persist initial data.
    pub fn create_session(
        &self,
        selections: &SceneSelections,
        scene: &SceneData,
        characters: &[CharacterSheet],
    ) -> Result<String, String> {
        let session_id = uuid::Uuid::now_v7().to_string();
        let session_dir = self.base_dir.join(&session_id);
        fs::create_dir_all(&session_dir)
            .map_err(|e| format!("Failed to create session dir: {e}"))?;

        // Write scene-selections.json
        let selections_json = serde_json::to_string_pretty(selections)
            .map_err(|e| format!("Failed to serialize selections: {e}"))?;
        fs::write(session_dir.join("scene-selections.json"), selections_json)
            .map_err(|e| format!("Failed to write scene-selections.json: {e}"))?;

        // Write scene.json
        let scene_json = serde_json::to_string_pretty(scene)
            .map_err(|e| format!("Failed to serialize scene: {e}"))?;
        fs::write(session_dir.join("scene.json"), scene_json)
            .map_err(|e| format!("Failed to write scene.json: {e}"))?;

        // Write characters.json
        let chars_json = serde_json::to_string_pretty(characters)
            .map_err(|e| format!("Failed to serialize characters: {e}"))?;
        fs::write(session_dir.join("characters.json"), chars_json)
            .map_err(|e| format!("Failed to write characters.json: {e}"))?;

        // Create empty events.jsonl
        fs::write(session_dir.join("events.jsonl"), "")
            .map_err(|e| format!("Failed to create events.jsonl: {e}"))?;

        Ok(session_id)
    }

    /// List all sessions, most recent first (UUIDv7 sorts chronologically).
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, String> {
        let mut sessions = Vec::new();

        let entries = fs::read_dir(&self.base_dir)
            .map_err(|e| format!("Failed to read sessions dir: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let session_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Try to read scene-selections.json for summary info
            let selections_path = path.join("scene-selections.json");
            let Ok(selections_str) = fs::read_to_string(&selections_path) else {
                continue;
            };
            let Ok(selections) = serde_json::from_str::<SceneSelections>(&selections_str) else {
                continue;
            };

            // Count events
            let events_path = path.join("events.jsonl");
            let turn_count = fs::read_to_string(&events_path)
                .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0);

            // Read scene title
            let title = fs::read_to_string(path.join("scene.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("title").and_then(|t| t.as_str()).map(String::from))
                .unwrap_or_else(|| "Untitled".to_string());

            sessions.push(SessionSummary {
                session_id,
                genre: selections.genre,
                profile: selections.profile,
                title,
                cast_names: selections.cast.iter().map(|c| c.name.clone()).collect(),
                turn_count,
            });
        }

        // Sort by session_id descending (UUIDv7 = chronological)
        sessions.sort_by(|a, b| b.session_id.cmp(&a.session_id));
        Ok(sessions)
    }

    /// Load a session's scene and characters for resuming.
    pub fn load_session(
        &self,
        session_id: &str,
    ) -> Result<(SceneSelections, SceneData, Vec<CharacterSheet>), String> {
        let session_dir = self.base_dir.join(session_id);
        if !session_dir.exists() {
            return Err(format!("Session not found: {session_id}"));
        }

        let selections: SceneSelections =
            serde_json::from_str(&fs::read_to_string(session_dir.join("scene-selections.json"))
                .map_err(|e| format!("Failed to read scene-selections.json: {e}"))?)
            .map_err(|e| format!("Failed to parse scene-selections.json: {e}"))?;

        let scene: SceneData =
            serde_json::from_str(&fs::read_to_string(session_dir.join("scene.json"))
                .map_err(|e| format!("Failed to read scene.json: {e}"))?)
            .map_err(|e| format!("Failed to parse scene.json: {e}"))?;

        let characters: Vec<CharacterSheet> =
            serde_json::from_str(&fs::read_to_string(session_dir.join("characters.json"))
                .map_err(|e| format!("Failed to read characters.json: {e}"))?)
            .map_err(|e| format!("Failed to parse characters.json: {e}"))?;

        Ok((selections, scene, characters))
    }

    /// Get the path to a session's events.jsonl for appending.
    pub fn events_path(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id).join("events.jsonl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list_sessions() {
        let tmp = std::env::temp_dir().join(format!("storyteller-session-test-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&tmp).unwrap();

        let selections = SceneSelections {
            genre: "low_fantasy_folklore".to_string(),
            profile: "quiet_reunion".to_string(),
            cast: vec![storyteller_engine::scene_composer::CastSelection {
                archetype: "wandering_artist".to_string(),
                name: "Aelric".to_string(),
                is_player_perspective: true,
            }],
            dynamics: vec![],
            setting_override: None,
            seed: Some(42),
        };

        // Minimal scene for testing
        let scene = SceneData {
            scene_id: storyteller_core::types::scene::SceneId::new(),
            title: "Test Scene".to_string(),
            scene_type: storyteller_core::types::scene::SceneType::Gravitational,
            setting: storyteller_core::types::character::SceneSetting {
                description: "A test setting".to_string(),
                affordances: vec![],
                sensory_details: vec![],
                aesthetic_detail: String::new(),
            },
            cast: vec![],
            stakes: vec![],
            constraints: storyteller_core::types::character::SceneConstraints {
                hard: vec![],
                soft: vec![],
                perceptual: vec![],
            },
            emotional_arc: vec![],
            evaluation_criteria: vec![],
        };

        let id = store.create_session(&selections, &scene, &[]).unwrap();
        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, id);
        assert_eq!(sessions[0].genre, "low_fantasy_folklore");

        // Load it back
        let (loaded_sel, loaded_scene, loaded_chars) = store.load_session(&id).unwrap();
        assert_eq!(loaded_sel.genre, "low_fantasy_folklore");
        assert_eq!(loaded_scene.title, "Test Scene");
        assert!(loaded_chars.is_empty());

        // Clean up
        let _ = fs::remove_dir_all(&tmp);
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add `mod session;` to `crates/storyteller-workshop/src-tauri/src/lib.rs` after the existing module declarations (line 4).

- [ ] **Step 3: Run tests**

Run: `cargo test -p storyteller-workshop session --all-features -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/session.rs crates/storyteller-workshop/src-tauri/src/lib.rs
git commit -m "feat(workshop): session persistence to .story/sessions/"
```

### Task 5: Workshop Tauri commands for catalog and composition

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/engine_state.rs`

Wire the SceneComposer and SessionStore into Tauri commands.

- [ ] **Step 1: Add SceneComposer as managed state in lib.rs**

Update `crates/storyteller-workshop/src-tauri/src/lib.rs` to load the SceneComposer at startup and manage it alongside EngineState. Add a `SessionStore` managed state too.

After `dotenvy::dotenv()` (line 18), add composer and session store initialization:

```rust
use crate::session::SessionStore;
use storyteller_engine::scene_composer::SceneComposer;
use std::path::PathBuf;
use std::sync::Arc;
```

In the builder chain, before `.manage(Mutex::new(None::<EngineState>))`, add:

```rust
.manage({
    let data_path = std::env::var("STORYTELLER_DATA_PATH")
        .map(PathBuf::from)
        .expect("STORYTELLER_DATA_PATH must be set");
    let composer = SceneComposer::load(&data_path)
        .expect("Failed to load scene descriptors");
    Arc::new(composer)
})
.manage({
    // Workshop root is the src-tauri directory's parent
    let workshop_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()
        .expect("workshop root")
        .to_path_buf();
    let store = SessionStore::new(&workshop_root)
        .expect("Failed to initialize session store");
    Arc::new(store)
})
```

- [ ] **Step 2: Add new commands to commands.rs**

Add at the bottom of `crates/storyteller-workshop/src-tauri/src/commands.rs`:

The new commands follow the exact same pattern as `start_scene` in the existing `commands.rs`. The key API calls are:

1. `assemble_narrator_context()` — builds `NarratorContextInput` from scene + characters + journal + resolver
2. `NarratorAgent::new(&context, Arc::clone(&llm))` — creates narrator from assembled context
3. `narrator.render_opening(&observer)` — generates opening prose (one-shot LLM call)

Debug events must be emitted for each phase (predictions, characters, events, context, narrator) using the same `emit_debug()` helper as `start_scene`.

**Important:** The implementer should extract the common setup logic (LLM creation, ML model loading, structured LLM creation, context assembly, narrator rendering, debug emission) from the existing `start_scene` into a shared helper function, then call it from both `start_scene`, `compose_scene`, and `resume_session`. This avoids tripling the code.

```rust
use storyteller_engine::scene_composer::{
    SceneComposer, SceneSelections,
};
use crate::session::{SessionStore, SessionSummary};

// ---------------------------------------------------------------------------
// Scene template commands
// ---------------------------------------------------------------------------

/// Load the catalog — returns list of available genres.
#[tauri::command]
pub async fn load_catalog(
    composer: State<'_, Arc<SceneComposer>>,
) -> Result<serde_json::Value, String> {
    let genres = composer.genres();
    serde_json::to_value(&genres).map_err(|e| format!("Serialize error: {e}"))
}

/// Get filtered options for a genre — profiles, archetypes, dynamics, names.
#[tauri::command]
pub async fn get_genre_options(
    genre_id: String,
    selected_archetypes: Vec<String>,
    composer: State<'_, Arc<SceneComposer>>,
) -> Result<serde_json::Value, String> {
    let profiles = composer.profiles_for_genre(&genre_id);
    let archetypes = composer.archetypes_for_genre(&genre_id);
    let dynamics = composer.dynamics_for_genre(&genre_id, &selected_archetypes);
    let names = composer.names_for_genre(&genre_id);

    serde_json::to_value(&serde_json::json!({
        "profiles": profiles,
        "archetypes": archetypes,
        "dynamics": dynamics,
        "names": names,
    }))
    .map_err(|e| format!("Serialize error: {e}"))
}

/// Compose a scene from selections, persist to session, and start playing.
///
/// Follows the same pattern as `start_scene`:
/// 1. Compose scene + characters from selections
/// 2. Persist to .story/sessions/
/// 3. Create LLM, ML, structured LLM providers
/// 4. Assemble narrator context via `assemble_narrator_context()`
/// 5. Create `NarratorAgent::new(&context, llm)` and call `render_opening(&observer)`
/// 6. Emit debug events for each phase
/// 7. Store EngineState
#[tauri::command]
pub async fn compose_scene(
    app: tauri::AppHandle,
    selections: SceneSelections,
    state: State<'_, Mutex<Option<EngineState>>>,
    composer: State<'_, Arc<SceneComposer>>,
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<SceneInfo, String> {
    let composed = composer.compose(&selections)?;

    // Persist to .story/sessions/
    let session_id = session_store.create_session(
        &composed.selections,
        &composed.scene,
        &composed.characters,
    )?;

    tracing::info!("Session created: {} ({})", session_id, composed.scene.title);

    // --- Setup (same pattern as start_scene) ---
    let characters_refs: Vec<&_> = composed.characters.iter().collect();
    let entity_ids: Vec<_> = composed.characters.iter().map(|c| c.entity_id).collect();

    let config = ExternalServerConfig {
        base_url: "http://127.0.0.1:11434".to_string(),
        model: "qwen2.5:14b".to_string(),
        ..Default::default()
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(ExternalServerProvider::new(config));
    let journal = SceneJournal::new(composed.scene.scene_id, 1200);

    let predictor = resolve_model_path().and_then(|path| CharacterPredictor::load(&path).ok());
    let event_classifier =
        resolve_event_classifier_path().and_then(|path| EventClassifier::load(&path).ok());
    let structured_llm: Option<Arc<dyn storyteller_core::traits::structured_llm::StructuredLlmProvider>> = {
        let config = StructuredLlmConfig::default();
        Some(Arc::new(OllamaStructuredProvider::new(config)))
    };
    let grammar = PlutchikWestern::new();

    let turn: u32 = 0;

    // --- Debug events (same as start_scene opening) ---
    emit_debug(&app, DebugEvent::PredictionComplete {
        turn,
        resolver_output: ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: vec![],
            scene_dynamics: "Opening turn — no player input yet".to_string(),
            conflicts: vec![],
        },
        timing_ms: 0,
        model_loaded: predictor.is_some(),
    });

    emit_debug(&app, DebugEvent::CharactersUpdated {
        turn,
        characters: composed.characters.clone(),
        emotional_markers: vec![],
    });

    emit_debug(&app, DebugEvent::EventsClassified {
        turn,
        classifications: vec![],
        classifier_loaded: event_classifier.is_some(),
    });

    // --- Context assembly (same pattern as start_scene) ---
    let opening_resolver = ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: vec![],
        scene_dynamics: "Opening — characters have not yet interacted".to_string(),
        conflicts: vec![],
    };

    let observer = CollectingObserver::new();
    let assembly_start = Instant::now();
    let context = assemble_narrator_context(
        &composed.scene,
        &characters_refs,
        &journal,
        &opening_resolver,
        "",
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
    );
    let assembly_ms = assembly_start.elapsed().as_millis() as u64;
    let token_counts = extract_token_counts(&observer);

    emit_debug(&app, DebugEvent::ContextAssembled {
        turn,
        preamble_text: format!("Narrator context for: {}", composed.scene.title),
        journal_text: String::new(),
        retrieved_text: String::new(),
        token_counts: TokenCounts {
            preamble: token_counts.0,
            journal: token_counts.1,
            retrieved: token_counts.2,
            total: token_counts.3,
        },
        timing_ms: assembly_ms,
    });

    // --- Narrator rendering (same pattern as start_scene) ---
    emit_debug(&app, DebugEvent::PhaseStarted {
        turn,
        phase: "narrator".to_string(),
    });

    let narrator = NarratorAgent::new(&context, Arc::clone(&llm)).with_temperature(0.8);
    let llm_start = Instant::now();
    let opening = narrator.render_opening(&observer).await
        .map_err(|e| format!("Failed to render opening: {e}"))?;
    let narrator_ms = llm_start.elapsed().as_millis() as u64;

    emit_debug(&app, DebugEvent::NarratorComplete {
        turn,
        prose_length: opening.text.len(),
        timing_ms: narrator_ms,
    });

    // --- Session log (use session store events path) ---
    // Note: The existing SessionLog writes to a flat `sessions/` dir.
    // For composed scenes, we log to .story/sessions/{id}/events.jsonl.
    // TODO: Unify SessionLog to use SessionStore paths once session
    // persistence is validated. For now, keep the old SessionLog for
    // compatibility with submit_input's logging.
    let sessions_dir = PathBuf::from("sessions");
    let session_log = SessionLog::new(&sessions_dir, &composed.scene.title)?;

    let scene_info = SceneInfo {
        title: composed.scene.title.clone(),
        setting_description: composed.scene.setting.description.clone(),
        cast: composed.scene.cast.iter().map(|c| c.name.clone()).collect(),
        opening_prose: opening.text.clone(),
    };

    let mut guard = state.lock().await;
    *guard = Some(EngineState {
        scene: composed.scene,
        characters: composed.characters,
        journal,
        llm,
        predictor,
        event_classifier,
        structured_llm,
        grammar,
        session_log,
        turn_count: 0,
    });

    Ok(scene_info)
}

/// List all persisted sessions.
#[tauri::command]
pub async fn list_sessions(
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<Vec<SessionSummary>, String> {
    session_store.list_sessions()
}

/// Resume a session by ID.
///
/// Same setup pattern as compose_scene but loads from persisted files.
/// NOTE: Fork session is deferred — not implemented in this pass.
/// TODO: Replay events.jsonl to rebuild journal state instead of
/// generating a fresh opening.
#[tauri::command]
pub async fn resume_session(
    session_id: String,
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
    session_store: State<'_, Arc<SessionStore>>,
) -> Result<SceneInfo, String> {
    let (_selections, scene, characters) = session_store.load_session(&session_id)?;

    // Follow same pattern as compose_scene for setup, context assembly,
    // narrator rendering, and debug event emission.
    // See compose_scene above — the implementer should extract shared
    // setup logic into a helper function used by start_scene,
    // compose_scene, and resume_session.
    //
    // Key difference: resume_session should eventually replay events.jsonl
    // to rebuild journal state. For now, it generates a fresh opening.

    // ... (same setup as compose_scene: LLM, ML, context assembly,
    //  NarratorAgent::new(&context, Arc::clone(&llm)),
    //  narrator.render_opening(&observer), debug events) ...

    todo!("Implementer: extract shared setup from compose_scene into helper, call it here")
}
```

- [ ] **Step 3: Register new commands in lib.rs**

Update the `generate_handler!` macro in `crates/storyteller-workshop/src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    commands::check_llm,
    commands::start_scene,
    commands::submit_input,
    commands::get_session_log,
    commands::load_catalog,
    commands::get_genre_options,
    commands::compose_scene,
    commands::list_sessions,
    commands::resume_session,
])
```

- [ ] **Step 4: Run compilation check**

Run: `cargo check -p storyteller-workshop --all-features`

Expected: Compiles without errors. (Full testing requires Tauri runtime, so compilation check is sufficient here.)

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/
git commit -m "feat(workshop): Tauri commands for catalog, composition, and session management"
```

## Chunk 3: Workshop Frontend

### Task 6: TypeScript types and API wrappers

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/types.ts`
- Modify: `crates/storyteller-workshop/src/lib/api.ts`

- [ ] **Step 1: Add scene setup types to types.ts**

Append to `crates/storyteller-workshop/src/lib/types.ts`:

```typescript
// ---------------------------------------------------------------------------
// Scene Template Types
// ---------------------------------------------------------------------------

export interface GenreSummary {
  id: string;
  display_name: string;
  description: string;
  archetype_count: number;
  profile_count: number;
  dynamic_count: number;
}

export interface ProfileSummary {
  id: string;
  display_name: string;
  description: string;
  scene_type: string;
  tension_min: number;
  tension_max: number;
  cast_size_min: number;
  cast_size_max: number;
}

export interface ArchetypeSummary {
  id: string;
  display_name: string;
  description: string;
}

export interface DynamicSummary {
  id: string;
  display_name: string;
  description: string;
  role_a: string;
  role_b: string;
}

export interface GenreOptions {
  profiles: ProfileSummary[];
  archetypes: ArchetypeSummary[];
  dynamics: DynamicSummary[];
  names: string[];
}

export interface CastSelection {
  archetype: string;
  name: string;
  is_player_perspective: boolean;
}

export interface DynamicSelection {
  character_a_index: number;
  character_b_index: number;
  dynamic: string;
}

export interface SceneSelections {
  genre: string;
  profile: string;
  cast: CastSelection[];
  dynamics: DynamicSelection[];
  setting_override: string | null;
  seed: number | null;
}

export interface SessionSummary {
  session_id: string;
  genre: string;
  profile: string;
  title: string;
  cast_names: string[];
  turn_count: number;
}
```

- [ ] **Step 2: Add API wrappers to api.ts**

Append to `crates/storyteller-workshop/src/lib/api.ts`:

```typescript
import type {
  GenreSummary,
  GenreOptions,
  SceneSelections,
  SessionSummary,
} from "./types";

export async function loadCatalog(): Promise<GenreSummary[]> {
  return invoke<GenreSummary[]>("load_catalog");
}

export async function getGenreOptions(
  genreId: string,
  selectedArchetypes: string[] = []
): Promise<GenreOptions> {
  return invoke<GenreOptions>("get_genre_options", {
    genreId,
    selectedArchetypes,
  });
}

export async function composeScene(
  selections: SceneSelections
): Promise<SceneInfo> {
  return invoke<SceneInfo>("compose_scene", { selections });
}

export async function listSessions(): Promise<SessionSummary[]> {
  return invoke<SessionSummary[]>("list_sessions");
}

export async function resumeSession(sessionId: string): Promise<SceneInfo> {
  return invoke<SceneInfo>("resume_session", { sessionId });
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/lib/types.ts crates/storyteller-workshop/src/lib/api.ts
git commit -m "feat(workshop): TypeScript types and API wrappers for scene setup"
```

### Task 7: Scene setup wizard component

**Files:**
- Create: `crates/storyteller-workshop/src/lib/SceneSetup.svelte`

This is the linear wizard: Genre → Profile → Cast → Setting → Launch.

- [ ] **Step 1: Create SceneSetup.svelte**

Create `crates/storyteller-workshop/src/lib/SceneSetup.svelte` — a multi-step wizard component. Each step renders conditionally based on a `step` state variable. The component accepts an `onlaunch` callback prop that receives `SceneInfo` when composition completes.

The wizard should:
1. Load genres on mount via `loadCatalog()`
2. On genre select, call `getGenreOptions()` to populate profiles/archetypes/dynamics
3. On profile select, show cast builder with `cast_size_min`..`cast_size_max` slots
4. Each cast slot: archetype dropdown + name field (pre-filled from genre names) + player radio
5. Once 2+ cast members, show dynamics assignment between pairs
6. Setting preview with optional text override
7. Launch button calls `composeScene()` and passes result to parent

Use the existing app styling conventions: `var(--bg)`, `var(--text-primary)`, `var(--accent)`, `var(--border)`, `var(--font-mono)`, Georgia serif for headings.

**Note:** This component will require iteration during implementation. The exact layout and styling should follow the existing workshop aesthetic. The key requirement is the data flow: cascading selections that constrain downstream options.

- [ ] **Step 2: Commit**

```bash
git add crates/storyteller-workshop/src/lib/SceneSetup.svelte
git commit -m "feat(workshop): scene setup wizard component"
```

### Task 8: Session panel component

**Files:**
- Create: `crates/storyteller-workshop/src/lib/SessionPanel.svelte`

Left-panel session navigator.

- [ ] **Step 1: Create SessionPanel.svelte**

Create `crates/storyteller-workshop/src/lib/SessionPanel.svelte` — a collapsible left panel that shows:
- "New Session" button (triggers scene setup wizard)
- List of recent sessions from `listSessions()`, showing title, genre, cast names, turn count
- Each session row has a "Resume" action
- Fork action is deferred to a future pass (noted in design doc Section 6)

The panel should:
- Accept `onNewSession` and `onResumeSession(sessionId)` callback props
- Load sessions on mount
- Refresh session list when a new session is created
- Be collapsible (similar pattern to DebugPanel)

- [ ] **Step 2: Commit**

```bash
git add crates/storyteller-workshop/src/lib/SessionPanel.svelte
git commit -m "feat(workshop): session panel for new/resume session management"
```

### Task 9: Page integration

**Files:**
- Modify: `crates/storyteller-workshop/src/routes/+page.svelte`

Wire the setup flow into the main page. The page state machine becomes:
- `"setup"` — show SceneSetup wizard (no scene loaded)
- `"playing"` — show StoryPane + InputBar + DebugPanel (current behavior)

- [ ] **Step 1: Update +page.svelte**

Modify `crates/storyteller-workshop/src/routes/+page.svelte` to:
1. Add `SessionPanel` on the left (collapsible)
2. Add `SceneSetup` as the initial view instead of auto-starting `the_flute_kept`
3. When `SceneSetup` completes (or a session is resumed), transition to play view
4. Keep `start_scene()` as a fallback path (header menu item "Classic: The Flute Kept")

The page state: `let view: "setup" | "playing" = $state("setup");`

On `SceneSetup.onlaunch`: set `sceneInfo`, create opening block, set `view = "playing"`.
On `SessionPanel.onResumeSession`: call `resumeSession()`, same transition.

- [ ] **Step 2: Run svelte-check**

Run: `cd crates/storyteller-workshop && npx svelte-check`

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/
git commit -m "feat(workshop): integrate scene setup wizard and session panel into main page"
```

## Chunk 4: Name Harvesting Tool & Descriptor Authoring

### Task 10: Name harvester Python tool

**Files:**
- Create: `tools/name-harvester/pyproject.toml`
- Create: `tools/name-harvester/src/name_harvester/__init__.py`
- Create: `tools/name-harvester/src/name_harvester/harvester.py`
- Create: `tools/name-harvester/src/name_harvester/cli.py`
- Create: `tools/name-harvester/tests/test_harvester.py`

- [ ] **Step 1: Create pyproject.toml**

```toml
[project]
name = "name-harvester"
version = "0.1.0"
description = "Harvest genre-appropriate character names from fantasynamegenerators.com"
requires-python = ">=3.11"
dependencies = [
    "httpx>=0.27",
    "beautifulsoup4>=4.12",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "ruff>=0.3",
]

[project.scripts]
harvest-names = "name_harvester.cli:main"

[tool.ruff]
line-length = 100
target-version = "py311"
```

- [ ] **Step 2: Create harvester.py**

```python
"""Polite name harvesting from fantasynamegenerators.com."""

import time
from dataclasses import dataclass

import httpx
from bs4 import BeautifulSoup


@dataclass
class GenreNameMapping:
    """Maps a storyteller genre to a fantasynamegenerators.com URL."""
    genre_id: str
    url: str
    source_description: str


# Editorial mapping — which name generators fit which genre aesthetic
GENRE_MAPPINGS: list[GenreNameMapping] = [
    GenreNameMapping(
        genre_id="low_fantasy_folklore",
        url="https://www.fantasynamegenerators.com/slavic-names.php",
        source_description="fantasynamegenerators.com/slavic-names",
    ),
    GenreNameMapping(
        genre_id="sci_fi_noir",
        url="https://www.fantasynamegenerators.com/cyberpunk-names.php",
        source_description="fantasynamegenerators.com/cyberpunk-names",
    ),
    GenreNameMapping(
        genre_id="cozy_ghost_story",
        url="https://www.fantasynamegenerators.com/english-names.php",
        source_description="fantasynamegenerators.com/english-names",
    ),
]


def harvest_names(url: str, delay: float = 2.5) -> list[str]:
    """Fetch names from a fantasynamegenerators.com page.

    Submits the generator form and parses the result.
    Rate-limited by `delay` seconds.
    """
    time.sleep(delay)

    client = httpx.Client(
        timeout=15.0,
        headers={"User-Agent": "storyteller-name-harvester/0.1 (research tool)"},
    )

    # The site uses a form POST to generate names
    response = client.post(url, data={})
    response.raise_for_status()

    return parse_names(response.text)


def parse_names(html: str) -> list[str]:
    """Extract names from the generator result HTML."""
    soup = BeautifulSoup(html, "html.parser")
    # Names typically appear in a div with id "result" or class "nameList"
    result_div = soup.find("div", id="result") or soup.find("div", class_="nameList")
    if not result_div:
        return []

    names = []
    for item in result_div.stripped_strings:
        name = item.strip()
        if name and len(name) > 1:
            names.append(name)
    return names
```

- [ ] **Step 3: Create cli.py**

```python
"""CLI for name harvesting."""

import json
import sys
from pathlib import Path

from .harvester import GENRE_MAPPINGS, harvest_names


def main() -> None:
    """Harvest names for all mapped genres and write to names.json."""
    output_path = None
    if len(sys.argv) > 1:
        output_path = Path(sys.argv[1])
    else:
        # Try STORYTELLER_DATA_PATH
        import os
        data_path = os.environ.get("STORYTELLER_DATA_PATH")
        if data_path:
            output_path = Path(data_path) / "training-data" / "descriptors" / "names.json"

    if not output_path:
        print("Usage: harvest-names [output_path]")
        print("Or set STORYTELLER_DATA_PATH environment variable")
        sys.exit(1)

    # Load existing if present
    existing: dict = {}
    if output_path.exists():
        existing = json.loads(output_path.read_text())

    for mapping in GENRE_MAPPINGS:
        print(f"Harvesting {mapping.genre_id} from {mapping.url}...")
        try:
            names = harvest_names(mapping.url)
            print(f"  Got {len(names)} names")
            existing[mapping.genre_id] = {
                "names": sorted(set(names)),
                "source": mapping.source_description,
            }
        except Exception as e:
            print(f"  Error: {e}")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(existing, indent=2, ensure_ascii=False) + "\n")
    print(f"Written to {output_path}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Create __init__.py and test**

`src/name_harvester/__init__.py`: empty file.

`tests/test_harvester.py`:
```python
"""Tests for name parsing logic (not network calls)."""

from name_harvester.harvester import parse_names


def test_parse_names_from_result_div():
    html = '''
    <html><body>
    <div id="result">
    Aelric<br>Maren<br>Vasil<br>Ilyana<br>Korin
    </div>
    </body></html>
    '''
    names = parse_names(html)
    assert len(names) == 5
    assert "Aelric" in names
    assert "Maren" in names


def test_parse_names_empty():
    html = "<html><body><p>No names here</p></body></html>"
    names = parse_names(html)
    assert names == []
```

- [ ] **Step 5: Run tests**

Run: `cd tools/name-harvester && uv sync --dev && uv run pytest -v`

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tools/name-harvester/
git commit -m "feat(tools): name harvester for genre-appropriate character names"
```

### Task 11: Author stub genres and setting templates

**Files:**
- Modify: `../storyteller-data/training-data/descriptors/genres.json` (add 2 stub genres)
- Create: `../storyteller-data/training-data/descriptors/settings.json` (setting templates)

This is authorial work — the implementer should:

- [ ] **Step 1: Add stub genres to genres.json**

Add two new entries to the `genres` array in `genres.json`:

1. `sci_fi_noir` — valid archetypes: byronic_hero, clever_trickster, withdrawn_scholar, reluctant_leader, loyal_soldier. Valid dynamics: rivals_with_respect, suspicious_allies, authority_subject, companions_on_road. Valid profiles: confrontation_over_betrayal, first_meeting_hidden_agendas, negotiation_under_pressure, discovery_of_truth.

2. `cozy_ghost_story` — valid archetypes: grieving_youth, wise_elder, protective_parent, withdrawn_scholar, fey_outsider. Valid dynamics: guardian_ward, mentor_student, strangers_shared_grief, patron_protege. Valid profiles: quiet_reunion, vulnerable_admission, quiet_aftermath, farewell_scene, discovery_of_truth.

Each with `excluded_combinations` as appropriate and a brief description.

- [ ] **Step 2: Create settings.json**

Author setting templates for `low_fantasy_folklore` (at minimum `quiet_reunion` and `confrontation_over_betrayal` profiles). Include `default_setting` with sensory palette and time options.

Stub entries for `sci_fi_noir` and `cozy_ghost_story` with default settings only.

- [ ] **Step 3: Commit (in storyteller-data repo)**

```bash
cd ../storyteller-data
git add training-data/descriptors/genres.json training-data/descriptors/settings.json
git commit -m "feat: add stub genres and setting templates for scene composition"
```

### Task 12: Verify end-to-end

- [ ] **Step 1: Run all Rust tests**

Run: `cd /Users/petetaylor/projects/tasker-systems/storyteller && cargo test --all-features`

Expected: All existing tests still pass + new scene_composer and session tests pass.

- [ ] **Step 2: Run Tauri dev build**

Run: `cd crates/storyteller-workshop && bun tauri dev`

Expected: Workshop launches. Scene setup wizard appears instead of auto-starting The Flute Kept. Genre selection shows low_fantasy_folklore + stub genres. Cascading selection works. Composing a scene persists to `.story/sessions/` and transitions to play view.

- [ ] **Step 3: Verify session persistence**

After composing and playing a few turns, check:
```bash
ls crates/storyteller-workshop/.story/sessions/
cat crates/storyteller-workshop/.story/sessions/*/scene-selections.json
```

Expected: Session directory exists with scene-selections.json, scene.json, characters.json, events.jsonl.

- [ ] **Step 4: Final commit (if any fixups needed)**

```bash
git add -A
git commit -m "fix: end-to-end integration fixups for scene template generation"
```
