//! Serde types for descriptor JSON files and loading from the storyteller-data directory.
//!
//! The descriptor files define the combinatorial matrix for scene template generation:
//! archetypes, genres, profiles, dynamics, axis vocabulary, and cross-dimensions.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use storyteller_core::StorytellerError;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// A min/max range used throughout the descriptor files for numeric bounds.
#[derive(Debug, Clone, Deserialize)]
pub struct RangeBounds {
    pub min: f64,
    pub max: f64,
}

// ---------------------------------------------------------------------------
// Archetypes
// ---------------------------------------------------------------------------

/// Root wrapper for `archetypes.json`.
#[derive(Debug, Clone, Deserialize)]
struct ArchetypesFile {
    archetypes: Vec<Archetype>,
}

/// A character archetype defining personality axes, emotional profile, self-edge, and action tendencies.
#[derive(Debug, Clone, Deserialize)]
pub struct Archetype {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub axes: Vec<ArchetypeAxis>,
    pub default_emotional_profile: EmotionalProfile,
    pub default_self_edge: SelfEdge,
    pub action_tendencies: ActionTendencies,
}

/// A single axis specification within an archetype, defining the range of values for that axis.
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

/// Default emotional profile for an archetype, using a named grammar (e.g. Plutchik).
#[derive(Debug, Clone, Deserialize)]
pub struct EmotionalProfile {
    pub grammar_id: String,
    pub primaries: Vec<EmotionalPrimary>,
}

/// A single primary emotion with intensity range and awareness level.
#[derive(Debug, Clone, Deserialize)]
pub struct EmotionalPrimary {
    pub primary_id: String,
    pub intensity: RangeBounds,
    pub awareness: String,
}

/// Default self-referential edge for an archetype (how the character relates to themselves).
#[derive(Debug, Clone, Deserialize)]
pub struct SelfEdge {
    pub trust_competence: RangeBounds,
    pub trust_intentions: RangeBounds,
    pub trust_reliability: RangeBounds,
    pub affection: RangeBounds,
    pub debt: RangeBounds,
    pub history_weight: RangeBounds,
    pub projection_accuracy: RangeBounds,
}

/// Behavioral tendencies for an archetype in scenes.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionTendencies {
    pub primary_action_types: Vec<String>,
    pub primary_action_contexts: Vec<String>,
    pub speech_likelihood: f64,
    pub speech_registers: Vec<String>,
    pub default_awareness: String,
}

// ---------------------------------------------------------------------------
// Genres
// ---------------------------------------------------------------------------

/// Root wrapper for `genres.json`.
#[derive(Debug, Clone, Deserialize)]
struct GenresFile {
    genres: Vec<Genre>,
}

/// A genre descriptor defining which archetypes, dynamics, and profiles are valid together.
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

/// A combination of archetype + dynamic + profile that is excluded for narrative reasons.
#[derive(Debug, Clone, Deserialize)]
pub struct ExcludedCombination {
    pub archetype: Option<String>,
    pub dynamic: Option<String>,
    pub profile: Option<String>,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// Root wrapper for `profiles.json`.
#[derive(Debug, Clone, Deserialize)]
struct ProfilesFile {
    profiles: Vec<Profile>,
}

/// A scene profile defining the type, tension range, cast size, and characteristic events.
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

/// An event type + emotional register pair with a weight indicating how characteristic it is.
#[derive(Debug, Clone, Deserialize)]
pub struct CharacteristicEvent {
    pub event_type: String,
    pub emotional_register: String,
    pub weight: f64,
}

// ---------------------------------------------------------------------------
// Dynamics
// ---------------------------------------------------------------------------

/// Root wrapper for `dynamics.json`.
#[derive(Debug, Clone, Deserialize)]
struct DynamicsFile {
    dynamics: Vec<Dynamic>,
}

/// A relational dynamic between two roles, defining edge properties and topology.
#[derive(Debug, Clone, Deserialize)]
pub struct Dynamic {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
    pub edge_a_to_b: RelationalEdge,
    pub edge_b_to_a: RelationalEdge,
    pub topology_a: String,
    pub topology_b: String,
}

/// Trust and relational properties for a directed edge between two characters.
#[derive(Debug, Clone, Deserialize)]
pub struct RelationalEdge {
    pub trust_reliability: RangeBounds,
    pub trust_competence: RangeBounds,
    pub trust_benevolence: RangeBounds,
    pub affection: RangeBounds,
    pub debt: RangeBounds,
}

// ---------------------------------------------------------------------------
// Axis vocabulary
// ---------------------------------------------------------------------------

/// Root wrapper for `axis-vocabulary.json`.
#[derive(Debug, Clone, Deserialize)]
struct AxesFile {
    axes: Vec<AxisDefinition>,
}

/// A single axis in the character tensor vocabulary.
#[derive(Debug, Clone, Deserialize)]
pub struct AxisDefinition {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub typical_layer: String,
    pub category: String,
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Cross-dimensions
// ---------------------------------------------------------------------------

/// Root wrapper for `cross-dimensions.json`.
#[derive(Debug, Clone, Deserialize)]
struct DimensionsFile {
    dimensions: Vec<CrossDimension>,
}

/// A cross-cutting dimension (age, gender, species) with values that can modify axes.
#[derive(Debug, Clone, Deserialize)]
pub struct CrossDimension {
    pub id: String,
    pub display_name: String,
    pub values: Vec<DimensionValue>,
}

/// A single value within a cross-dimension, optionally enabling primordial content.
#[derive(Debug, Clone, Deserialize)]
pub struct DimensionValue {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub enables_primordial: bool,
    #[serde(default)]
    pub axis_modifiers: Vec<AxisModifier>,
}

/// An additive modifier applied to an axis when a dimension value is active.
#[derive(Debug, Clone, Deserialize)]
pub struct AxisModifier {
    pub axis_id: String,
    pub additive: f64,
}

// ---------------------------------------------------------------------------
// Names (optional)
// ---------------------------------------------------------------------------

/// A genre-keyed collection of character names with provenance.
#[derive(Debug, Clone, Deserialize)]
pub struct NameCollection {
    pub names: Vec<String>,
    pub source: String,
}

// ---------------------------------------------------------------------------
// Settings (optional)
// ---------------------------------------------------------------------------

/// A single setting descriptor with location, atmosphere, and sensory details.
#[derive(Debug, Clone, Deserialize)]
pub struct SettingDescriptor {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Genre-keyed setting collection: `default_setting` plus optional profile-keyed overrides.
#[derive(Debug, Clone, Deserialize)]
pub struct SettingCollection {
    #[serde(default)]
    pub default_setting: Option<SettingDescriptor>,
    #[serde(flatten)]
    pub profile_settings: HashMap<String, SettingDescriptor>,
}

// ---------------------------------------------------------------------------
// DescriptorSet — the complete loaded descriptor collection
// ---------------------------------------------------------------------------

/// The complete set of descriptors loaded from disk. This is the entry point for
/// catalog queries and scene composition.
#[derive(Debug, Clone)]
pub struct DescriptorSet {
    pub archetypes: Vec<Archetype>,
    pub genres: Vec<Genre>,
    pub profiles: Vec<Profile>,
    pub dynamics: Vec<Dynamic>,
    pub axes: Vec<AxisDefinition>,
    pub dimensions: Vec<CrossDimension>,
    pub names: HashMap<String, NameCollection>,
    pub settings: HashMap<String, SettingCollection>,
}

impl DescriptorSet {
    /// Load all descriptor files from `base_dir/training-data/descriptors/`.
    ///
    /// Required files (`archetypes.json`, `genres.json`, `profiles.json`, `dynamics.json`,
    /// `axis-vocabulary.json`, `cross-dimensions.json`) must exist and parse successfully.
    /// Optional files (`names.json`, `settings.json`) default to empty collections if missing.
    pub fn load(base_dir: &Path) -> Result<Self, StorytellerError> {
        let desc_dir = base_dir.join("training-data").join("descriptors");

        let archetypes =
            Self::load_required::<ArchetypesFile>(&desc_dir, "archetypes.json")?.archetypes;
        let genres = Self::load_required::<GenresFile>(&desc_dir, "genres.json")?.genres;
        let profiles = Self::load_required::<ProfilesFile>(&desc_dir, "profiles.json")?.profiles;
        let dynamics = Self::load_required::<DynamicsFile>(&desc_dir, "dynamics.json")?.dynamics;
        let axes = Self::load_required::<AxesFile>(&desc_dir, "axis-vocabulary.json")?.axes;
        let dimensions =
            Self::load_required::<DimensionsFile>(&desc_dir, "cross-dimensions.json")?.dimensions;

        let names: HashMap<String, NameCollection> =
            Self::load_optional(&desc_dir, "names.json")?.unwrap_or_default();
        let settings: HashMap<String, SettingCollection> =
            Self::load_optional(&desc_dir, "settings.json")?.unwrap_or_default();

        Ok(Self {
            archetypes,
            genres,
            profiles,
            dynamics,
            axes,
            dimensions,
            names,
            settings,
        })
    }

    /// Load and deserialize a required JSON file. Returns an error if the file is
    /// missing or malformed.
    fn load_required<T: serde::de::DeserializeOwned>(
        dir: &Path,
        filename: &str,
    ) -> Result<T, StorytellerError> {
        let path = dir.join(filename);
        let content = std::fs::read_to_string(&path).map_err(|e| {
            StorytellerError::Config(format!(
                "Failed to read descriptor file {}: {}",
                path.display(),
                e
            ))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            StorytellerError::Config(format!(
                "Failed to parse descriptor file {}: {}",
                path.display(),
                e
            ))
        })
    }

    /// Load and deserialize an optional JSON file. Returns `Ok(None)` if the file
    /// does not exist.
    fn load_optional<T: serde::de::DeserializeOwned>(
        dir: &Path,
        filename: &str,
    ) -> Result<Option<T>, StorytellerError> {
        let path = dir.join(filename);
        if !path.exists() {
            return Ok(None);
        }
        Self::load_required(dir, filename).map(Some)
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

    #[test]
    fn load_descriptors_from_data_path() {
        let Some(base) = data_path() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let set = DescriptorSet::load(&base).expect("descriptor loading should succeed");

        assert!(!set.archetypes.is_empty(), "archetypes should be non-empty");
        assert!(!set.genres.is_empty(), "genres should be non-empty");
        assert!(!set.profiles.is_empty(), "profiles should be non-empty");
        assert!(!set.dynamics.is_empty(), "dynamics should be non-empty");
        assert!(!set.axes.is_empty(), "axes should be non-empty");
        assert!(!set.dimensions.is_empty(), "dimensions should be non-empty");

        println!(
            "Loaded: {} archetypes, {} genres, {} profiles, {} dynamics, {} axes, {} dimensions",
            set.archetypes.len(),
            set.genres.len(),
            set.profiles.len(),
            set.dynamics.len(),
            set.axes.len(),
            set.dimensions.len(),
        );
    }

    #[test]
    fn archetypes_have_axes_and_emotional_profiles() {
        let Some(base) = data_path() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let set = DescriptorSet::load(&base).expect("descriptor loading should succeed");

        for arch in &set.archetypes {
            assert!(
                !arch.axes.is_empty(),
                "archetype '{}' should have axes",
                arch.id
            );
            assert!(
                !arch.default_emotional_profile.primaries.is_empty(),
                "archetype '{}' should have emotional primaries",
                arch.id
            );
            println!(
                "  {} — {} axes, {} primaries",
                arch.id,
                arch.axes.len(),
                arch.default_emotional_profile.primaries.len(),
            );
        }
    }

    #[test]
    fn genres_reference_valid_archetypes() {
        let Some(base) = data_path() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let set = DescriptorSet::load(&base).expect("descriptor loading should succeed");
        let archetype_ids: std::collections::HashSet<&str> =
            set.archetypes.iter().map(|a| a.id.as_str()).collect();

        for genre in &set.genres {
            for arch_ref in &genre.valid_archetypes {
                assert!(
                    archetype_ids.contains(arch_ref.as_str()),
                    "genre '{}' references unknown archetype '{}'",
                    genre.id,
                    arch_ref
                );
            }
        }
    }

    #[test]
    fn genres_reference_valid_dynamics() {
        let Some(base) = data_path() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let set = DescriptorSet::load(&base).expect("descriptor loading should succeed");
        let dynamic_ids: std::collections::HashSet<&str> =
            set.dynamics.iter().map(|d| d.id.as_str()).collect();

        for genre in &set.genres {
            for dyn_ref in &genre.valid_dynamics {
                assert!(
                    dynamic_ids.contains(dyn_ref.as_str()),
                    "genre '{}' references unknown dynamic '{}'",
                    genre.id,
                    dyn_ref
                );
            }
        }
    }

    #[test]
    fn genres_reference_valid_profiles() {
        let Some(base) = data_path() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let set = DescriptorSet::load(&base).expect("descriptor loading should succeed");
        let profile_ids: std::collections::HashSet<&str> =
            set.profiles.iter().map(|p| p.id.as_str()).collect();

        for genre in &set.genres {
            for prof_ref in &genre.valid_profiles {
                assert!(
                    profile_ids.contains(prof_ref.as_str()),
                    "genre '{}' references unknown profile '{}'",
                    genre.id,
                    prof_ref
                );
            }
        }
    }
}
