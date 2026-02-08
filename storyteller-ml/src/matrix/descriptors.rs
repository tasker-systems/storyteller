//! Descriptor types for the combinatorial training data matrix.
//!
//! Descriptors are hand-authored JSON files that define the axes of the
//! combinatorial matrix: archetypes (character templates), dynamics
//! (relational patterns), profiles (scene situations), genres (validity
//! gates), and cross-dimensions (demographic variation).
//!
//! This module provides deserialization types and a validated
//! [`DescriptorSet`] that ensures internal consistency.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

// ===========================================================================
// Common types
// ===========================================================================

/// A range from which values are sampled uniformly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValueRange {
    pub min: f32,
    pub max: f32,
}

impl ValueRange {
    /// Sample a value uniformly within this range.
    pub fn sample(&self, rng: &mut impl rand::Rng) -> f32 {
        rng.random_range(self.min..=self.max)
    }

    /// Clamp a value to this range.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

// ===========================================================================
// Axis vocabulary
// ===========================================================================

/// A single axis definition from the vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisDefinition {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub typical_layer: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// The complete axis vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisVocabulary {
    pub axes: Vec<AxisDefinition>,
}

// ===========================================================================
// Archetype descriptors
// ===========================================================================

/// An axis entry within an archetype — value ranges to sample from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeAxis {
    pub axis_id: String,
    pub central_tendency: ValueRange,
    pub variance: ValueRange,
    pub range_low: ValueRange,
    pub range_high: ValueRange,
    pub layer: String,
    pub provenance: String,
}

/// Emotional primary entry within an archetype descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeEmotionalPrimary {
    pub primary_id: String,
    pub intensity: ValueRange,
    pub awareness: String,
}

/// Default emotional profile for an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeEmotionalProfile {
    pub grammar_id: String,
    pub primaries: Vec<ArchetypeEmotionalPrimary>,
}

/// Self-edge template for an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeSelfEdge {
    pub trust_competence: ValueRange,
    pub trust_intentions: ValueRange,
    pub trust_reliability: ValueRange,
    pub affection: ValueRange,
    pub debt: ValueRange,
    pub history_weight: ValueRange,
    pub projection_accuracy: ValueRange,
}

/// Action tendency descriptors for an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTendencies {
    pub primary_action_types: Vec<String>,
    pub primary_action_contexts: Vec<String>,
    pub speech_likelihood: f32,
    pub speech_registers: Vec<String>,
    pub default_awareness: String,
}

/// A complete archetype descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeDescriptor {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub axes: Vec<ArchetypeAxis>,
    pub default_emotional_profile: ArchetypeEmotionalProfile,
    pub default_self_edge: ArchetypeSelfEdge,
    pub action_tendencies: ActionTendencies,
}

/// The collection of all archetypes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeSet {
    pub archetypes: Vec<ArchetypeDescriptor>,
}

// ===========================================================================
// Dynamic descriptors
// ===========================================================================

/// Substrate template for one direction of a relational edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateTemplate {
    pub trust_reliability: ValueRange,
    pub trust_competence: ValueRange,
    pub trust_benevolence: ValueRange,
    pub affection: ValueRange,
    pub debt: ValueRange,
}

/// A dynamic descriptor — defines both directions of a relational edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicDescriptor {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
    pub edge_a_to_b: SubstrateTemplate,
    pub edge_b_to_a: SubstrateTemplate,
    pub topology_a: String,
    pub topology_b: String,
}

/// The collection of all dynamics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSet {
    pub dynamics: Vec<DynamicDescriptor>,
}

// ===========================================================================
// Profile descriptors
// ===========================================================================

/// A weighted event type that can occur in a scene profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedEvent {
    pub event_type: String,
    pub emotional_register: String,
    pub weight: f32,
}

/// A scene profile descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDescriptor {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub scene_type: String,
    pub tension: ValueRange,
    pub cast_size: ValueRange,
    pub characteristic_events: Vec<WeightedEvent>,
}

/// The collection of all profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSet {
    pub profiles: Vec<ProfileDescriptor>,
}

// ===========================================================================
// Genre descriptors
// ===========================================================================

/// An excluded combination — a specific cell in the matrix that is invalid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedCombination {
    pub archetype: Option<String>,
    pub dynamic: Option<String>,
    pub profile: Option<String>,
    pub reason: String,
}

/// A genre descriptor — validity gates for the combinatorial matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreDescriptor {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub valid_archetypes: Vec<String>,
    pub valid_dynamics: Vec<String>,
    pub valid_profiles: Vec<String>,
    #[serde(default)]
    pub excluded_combinations: Vec<ExcludedCombination>,
}

/// The collection of all genres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreSet {
    pub genres: Vec<GenreDescriptor>,
}

// ===========================================================================
// Cross-dimension descriptors
// ===========================================================================

/// A cross-dimension axis modifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisModifier {
    pub axis_id: String,
    pub additive: f32,
}

/// A single cross-dimension value (e.g., "youth", "human").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDimensionValue {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub axis_modifiers: Vec<AxisModifier>,
    #[serde(default)]
    pub enables_primordial: bool,
}

/// A cross-dimension category (e.g., "age", "species").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDimension {
    pub id: String,
    pub display_name: String,
    pub values: Vec<CrossDimensionValue>,
}

/// The collection of all cross-dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDimensionSet {
    pub dimensions: Vec<CrossDimension>,
}

/// A sampled cross-dimension value set (one value per dimension).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSample {
    /// dimension_id → value_id
    pub values: BTreeMap<String, String>,
}

// ===========================================================================
// DescriptorSet — validated collection of all descriptors
// ===========================================================================

/// The complete set of validated descriptors loaded from JSON files.
#[derive(Debug, Clone)]
pub struct DescriptorSet {
    pub axis_vocabulary: AxisVocabulary,
    pub archetypes: ArchetypeSet,
    pub dynamics: DynamicSet,
    pub profiles: ProfileSet,
    pub genres: GenreSet,
    pub cross_dimensions: CrossDimensionSet,
    /// axis_id → AxisDefinition index for fast lookup.
    axis_index: HashMap<String, usize>,
}

impl DescriptorSet {
    /// Load all descriptor files from the given directory.
    ///
    /// Expects the directory to contain:
    /// - `axis-vocabulary.json`
    /// - `archetypes.json`
    /// - `dynamics.json`
    /// - `profiles.json`
    /// - `genres.json`
    /// - `cross-dimensions.json`
    pub fn load(data_path: &Path) -> Result<Self, DescriptorError> {
        let axis_vocabulary: AxisVocabulary =
            load_json(&data_path.join("axis-vocabulary.json"), "axis-vocabulary")?;
        let archetypes: ArchetypeSet = load_json(&data_path.join("archetypes.json"), "archetypes")?;
        let dynamics: DynamicSet = load_json(&data_path.join("dynamics.json"), "dynamics")?;
        let profiles: ProfileSet = load_json(&data_path.join("profiles.json"), "profiles")?;
        let genres: GenreSet = load_json(&data_path.join("genres.json"), "genres")?;
        let cross_dimensions: CrossDimensionSet =
            load_json(&data_path.join("cross-dimensions.json"), "cross-dimensions")?;

        let axis_index: HashMap<String, usize> = axis_vocabulary
            .axes
            .iter()
            .enumerate()
            .map(|(i, a)| (a.id.clone(), i))
            .collect();

        let set = Self {
            axis_vocabulary,
            archetypes,
            dynamics,
            profiles,
            genres,
            cross_dimensions,
            axis_index,
        };

        set.validate()?;
        Ok(set)
    }

    /// Look up an archetype by ID.
    pub fn archetype(&self, id: &str) -> Option<&ArchetypeDescriptor> {
        self.archetypes.archetypes.iter().find(|a| a.id == id)
    }

    /// Look up a dynamic by ID.
    pub fn dynamic(&self, id: &str) -> Option<&DynamicDescriptor> {
        self.dynamics.dynamics.iter().find(|d| d.id == id)
    }

    /// Look up a profile by ID.
    pub fn profile(&self, id: &str) -> Option<&ProfileDescriptor> {
        self.profiles.profiles.iter().find(|p| p.id == id)
    }

    /// Look up a genre by ID.
    pub fn genre(&self, id: &str) -> Option<&GenreDescriptor> {
        self.genres.genres.iter().find(|g| g.id == id)
    }

    /// Check if an axis ID is in the vocabulary.
    pub fn has_axis(&self, axis_id: &str) -> bool {
        self.axis_index.contains_key(axis_id)
    }

    /// Internal consistency validation.
    fn validate(&self) -> Result<(), DescriptorError> {
        let axis_ids: HashSet<&str> = self
            .axis_vocabulary
            .axes
            .iter()
            .map(|a| a.id.as_str())
            .collect();

        // Validate archetype axis references
        for archetype in &self.archetypes.archetypes {
            for axis in &archetype.axes {
                if !axis_ids.contains(axis.axis_id.as_str()) {
                    return Err(DescriptorError::InvalidReference {
                        context: format!("archetype '{}'", archetype.id),
                        reference: axis.axis_id.clone(),
                        kind: "axis".to_string(),
                    });
                }
                validate_range_order(
                    &axis.central_tendency,
                    &format!("{}/{}", archetype.id, axis.axis_id),
                )?;
                validate_range_order(
                    &axis.variance,
                    &format!("{}/{}/variance", archetype.id, axis.axis_id),
                )?;
            }
        }

        let archetype_ids: HashSet<&str> = self
            .archetypes
            .archetypes
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        let dynamic_ids: HashSet<&str> = self
            .dynamics
            .dynamics
            .iter()
            .map(|d| d.id.as_str())
            .collect();
        let profile_ids: HashSet<&str> = self
            .profiles
            .profiles
            .iter()
            .map(|p| p.id.as_str())
            .collect();

        // Validate genre references
        for genre in &self.genres.genres {
            for arch_id in &genre.valid_archetypes {
                if !archetype_ids.contains(arch_id.as_str()) {
                    return Err(DescriptorError::InvalidReference {
                        context: format!("genre '{}'", genre.id),
                        reference: arch_id.clone(),
                        kind: "archetype".to_string(),
                    });
                }
            }
            for dyn_id in &genre.valid_dynamics {
                if !dynamic_ids.contains(dyn_id.as_str()) {
                    return Err(DescriptorError::InvalidReference {
                        context: format!("genre '{}'", genre.id),
                        reference: dyn_id.clone(),
                        kind: "dynamic".to_string(),
                    });
                }
            }
            for prof_id in &genre.valid_profiles {
                if !profile_ids.contains(prof_id.as_str()) {
                    return Err(DescriptorError::InvalidReference {
                        context: format!("genre '{}'", genre.id),
                        reference: prof_id.clone(),
                        kind: "profile".to_string(),
                    });
                }
            }
        }

        // Validate cross-dimension axis references
        for dim in &self.cross_dimensions.dimensions {
            for val in &dim.values {
                for modifier in &val.axis_modifiers {
                    if !axis_ids.contains(modifier.axis_id.as_str()) {
                        return Err(DescriptorError::InvalidReference {
                            context: format!("cross-dimension '{}/{}'", dim.id, val.id),
                            reference: modifier.axis_id.clone(),
                            kind: "axis".to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

fn load_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    name: &str,
) -> Result<T, DescriptorError> {
    let content = std::fs::read_to_string(path).map_err(|e| DescriptorError::Io {
        file: name.to_string(),
        source: e,
    })?;
    serde_json::from_str(&content).map_err(|e| DescriptorError::Parse {
        file: name.to_string(),
        source: e,
    })
}

fn validate_range_order(range: &ValueRange, context: &str) -> Result<(), DescriptorError> {
    if range.min > range.max {
        return Err(DescriptorError::InvalidRange {
            context: context.to_string(),
            min: range.min,
            max: range.max,
        });
    }
    Ok(())
}

/// Errors that can occur when loading or validating descriptors.
#[derive(Debug)]
pub enum DescriptorError {
    Io {
        file: String,
        source: std::io::Error,
    },
    Parse {
        file: String,
        source: serde_json::Error,
    },
    InvalidReference {
        context: String,
        reference: String,
        kind: String,
    },
    InvalidRange {
        context: String,
        min: f32,
        max: f32,
    },
}

impl std::fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { file, source } => write!(f, "failed to read {file}: {source}"),
            Self::Parse { file, source } => write!(f, "failed to parse {file}: {source}"),
            Self::InvalidReference {
                context,
                reference,
                kind,
            } => write!(f, "invalid {kind} reference '{reference}' in {context}"),
            Self::InvalidRange { context, min, max } => {
                write!(f, "invalid range in {context}: min ({min}) > max ({max})")
            }
        }
    }
}

impl std::error::Error for DescriptorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Resolve the descriptor data path from the `STORYTELLER_DATA_PATH` environment variable.
///
/// Loads `.env` if present, then checks for `STORYTELLER_DATA_PATH` pointing to
/// the root of the `storyteller-data` repository. Returns the path to the
/// `training-data/descriptors/` subdirectory.
pub fn resolve_data_path() -> Result<std::path::PathBuf, DescriptorError> {
    // Try loading .env file (ignore if missing)
    let _ = dotenvy::dotenv();

    if let Ok(path) = std::env::var("STORYTELLER_DATA_PATH") {
        let p = std::path::PathBuf::from(path);
        let descriptors = p.join("training-data/descriptors");
        if descriptors.exists() {
            return Ok(descriptors);
        }
        return Err(DescriptorError::Io {
            file: "descriptor directory".to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "STORYTELLER_DATA_PATH is set but {descriptors:?} does not exist"
                ),
            ),
        });
    }

    Err(DescriptorError::Io {
        file: "descriptor directory".to_string(),
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "STORYTELLER_DATA_PATH not set. See .env.example for configuration.",
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_range_sample_within_bounds() {
        let range = ValueRange { min: 0.2, max: 0.8 };
        let mut rng = rand::rng();
        for _ in 0..100 {
            let v = range.sample(&mut rng);
            assert!(
                (0.2..=0.8).contains(&v),
                "sampled value {v} outside range [0.2, 0.8]"
            );
        }
    }

    #[test]
    fn value_range_clamp() {
        let range = ValueRange {
            min: -1.0,
            max: 1.0,
        };
        assert!((range.clamp(1.5) - 1.0).abs() < f32::EPSILON);
        assert!((range.clamp(-1.5) - -1.0).abs() < f32::EPSILON);
        assert!((range.clamp(0.5) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn validate_range_order_catches_inverted() {
        let bad = ValueRange { min: 0.8, max: 0.2 };
        assert!(validate_range_order(&bad, "test").is_err());

        let good = ValueRange { min: 0.2, max: 0.8 };
        assert!(validate_range_order(&good, "test").is_ok());
    }
}
