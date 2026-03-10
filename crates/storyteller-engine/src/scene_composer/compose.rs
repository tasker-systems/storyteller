//! Scene composition — transforms UI selections into playable `SceneData` + `CharacterSheet` pairs.
//!
//! The [`compose`](SceneComposer::compose) method on [`SceneComposer`] is the core
//! entry point. It takes a [`SceneSelections`] struct (genre, profile, cast with
//! archetypes, optional dynamics) and produces a [`ComposedScene`] containing
//! fully-instantiated scene data and character sheets with sampled tensor values.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::types::character::{
    CastEntry, CharacterSheet, CharacterTensor, EmotionalPrimary, EmotionalState, SceneConstraints,
    SceneData, SceneSetting, SelfEdge, SelfEdgeTrust, SelfKnowledge,
};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::scene::{SceneId, SceneType};
use storyteller_core::types::tensor::{AwarenessLevel, AxisValue, Provenance, TemporalLayer};
use storyteller_core::types::world_model::CapabilityProfile;

use super::catalog::SceneComposer;
use super::descriptors::{
    Archetype, EmotionalProfile, RangeBounds, SelfEdge as DescriptorSelfEdge,
};
use super::names::select_names;

// ---------------------------------------------------------------------------
// Selection types (UI → engine boundary)
// ---------------------------------------------------------------------------

/// Complete set of user selections for composing a scene.
///
/// Implements both `Serialize` and `Deserialize` so it can be persisted in
/// session state for replay / undo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSelections {
    /// Genre id from the descriptor set.
    pub genre_id: String,
    /// Profile id — determines scene type, tension, cast size.
    pub profile_id: String,
    /// One entry per cast member, each with an archetype selection.
    pub cast: Vec<CastSelection>,
    /// Optional relational dynamics between cast members.
    #[serde(default)]
    pub dynamics: Vec<DynamicSelection>,
    /// Optional title override (defaults to profile display name).
    #[serde(default)]
    pub title_override: Option<String>,
    /// Optional setting description override.
    #[serde(default)]
    pub setting_override: Option<String>,
    /// Optional RNG seed for deterministic composition.
    #[serde(default)]
    pub seed: Option<u64>,
}

/// A single cast member selection — archetype + optional name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastSelection {
    /// Archetype id for this character.
    pub archetype_id: String,
    /// Optional name override; if absent, picked from the genre name pool.
    #[serde(default)]
    pub name: Option<String>,
    /// Role label for the cast list (e.g. "protagonist", "antagonist").
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "cast".to_string()
}

/// A relational dynamic pairing two cast members by index.
///
/// NOTE: Dynamic edge weights are NOT yet wired into character sheets —
/// the relational web between characters is a future concern. These
/// selections are persisted for forward compatibility and UI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSelection {
    /// Dynamic id from the descriptor set.
    pub dynamic_id: String,
    /// Index into `SceneSelections::cast` for role A.
    pub cast_index_a: usize,
    /// Index into `SceneSelections::cast` for role B.
    pub cast_index_b: usize,
}

// ---------------------------------------------------------------------------
// Composed output
// ---------------------------------------------------------------------------

/// The fully composed scene ready for play, including the selections that
/// produced it (for session persistence and debugging).
#[derive(Debug, Clone, Serialize)]
pub struct ComposedScene {
    /// The scene data with setting, cast list, stakes, constraints.
    pub scene: SceneData,
    /// Character sheets with tensors, emotional states, self-edges.
    pub characters: Vec<CharacterSheet>,
    /// The selections that produced this scene (for persistence / replay).
    pub selections: SceneSelections,
}

// ---------------------------------------------------------------------------
// SceneComposer::compose
// ---------------------------------------------------------------------------

impl SceneComposer {
    /// Compose a playable scene from the given selections.
    ///
    /// Validates that genre, profile, and all archetypes exist in the descriptor
    /// set, samples tensor values within archetype-defined ranges, and builds
    /// the full `SceneData` + `Vec<CharacterSheet>` pair.
    pub fn compose(&self, selections: &SceneSelections) -> Result<ComposedScene, String> {
        // Validate genre
        let genre = self
            .find_genre(&selections.genre_id)
            .ok_or_else(|| format!("unknown genre: '{}'", selections.genre_id))?;

        // Validate profile
        let profile = self
            .find_profile(&selections.profile_id)
            .ok_or_else(|| format!("unknown profile: '{}'", selections.profile_id))?;

        // Validate archetypes
        let archetypes: Vec<&Archetype> = selections
            .cast
            .iter()
            .map(|cs| {
                self.find_archetype(&cs.archetype_id)
                    .ok_or_else(|| format!("unknown archetype: '{}'", cs.archetype_id))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Build RNG — seeded for determinism or fresh.
        let mut rng: StdRng = match selections.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        };

        // Resolve names — use overrides where provided, fill gaps from pool.
        let name_pool = self.names_for_genre(&genre.id);
        let needed_count = selections
            .cast
            .iter()
            .filter(|cs| cs.name.is_none())
            .count();
        let pool_names = select_names(&name_pool, needed_count, &mut rng);
        let mut pool_iter = pool_names.into_iter();

        let names: Vec<String> = selections
            .cast
            .iter()
            .map(|cs| {
                cs.name
                    .clone()
                    .unwrap_or_else(|| pool_iter.next().unwrap_or_else(|| "Unknown".to_string()))
            })
            .collect();

        // Compose character sheets
        let characters: Vec<CharacterSheet> = archetypes
            .iter()
            .zip(names.iter())
            .zip(selections.cast.iter())
            .map(|((archetype, name), cast_sel)| {
                compose_character(archetype, name, &cast_sel.role, &mut rng)
            })
            .collect();

        // Build cast entries for SceneData
        let cast: Vec<CastEntry> = characters
            .iter()
            .zip(selections.cast.iter())
            .map(|(ch, cs)| CastEntry {
                entity_id: ch.entity_id,
                name: ch.name.clone(),
                role: cs.role.clone(),
            })
            .collect();

        // Resolve scene type
        let scene_type = parse_scene_type(&profile.scene_type);

        // Title
        let title = selections
            .title_override
            .clone()
            .unwrap_or_else(|| profile.display_name.clone());

        // Setting
        let setting = compose_setting(
            &self.descriptors.settings,
            &genre.id,
            &profile.id,
            selections.setting_override.as_deref(),
        );

        // Stakes from profile characteristic events
        let stakes = compose_stakes(profile);

        // Constraints from genre + profile
        let constraints = compose_constraints(&genre.description, &profile.description);

        let scene = SceneData {
            scene_id: SceneId::new(),
            title,
            scene_type,
            setting,
            cast,
            stakes,
            constraints,
            emotional_arc: vec![format!(
                "Tension target: {:.1}–{:.1}",
                profile.tension.min, profile.tension.max
            )],
            evaluation_criteria: profile
                .characteristic_events
                .iter()
                .map(|ce| {
                    format!(
                        "{} ({}, weight {:.1})",
                        ce.event_type, ce.emotional_register, ce.weight
                    )
                })
                .collect(),
        };

        Ok(ComposedScene {
            scene,
            characters,
            selections: selections.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a single `CharacterSheet` from an archetype, name, and role.
fn compose_character<R: Rng>(
    archetype: &Archetype,
    name: &str,
    role: &str,
    rng: &mut R,
) -> CharacterSheet {
    // Build tensor from archetype axes
    let mut tensor = CharacterTensor::new();
    for axis in &archetype.axes {
        let value = AxisValue {
            central_tendency: sample_in_range(&axis.central_tendency, rng),
            variance: sample_in_range(&axis.variance, rng),
            range_low: sample_in_range(&axis.range_low, rng),
            range_high: sample_in_range(&axis.range_high, rng),
        };
        let layer = parse_temporal_layer(&axis.layer);
        tensor.insert(&axis.axis_id, value, layer, Provenance::Generated);
    }

    let emotional_state = compose_emotional_state(&archetype.default_emotional_profile, rng);
    let self_edge = compose_self_edge(&archetype.default_self_edge, rng);

    CharacterSheet {
        entity_id: EntityId::new(),
        name: name.to_string(),
        voice: format!(
            "{} ({})",
            archetype
                .action_tendencies
                .speech_registers
                .first()
                .map(|s| s.as_str())
                .unwrap_or("neutral"),
            archetype.action_tendencies.default_awareness
        ),
        backstory: format!(
            "A {} — {}. Role: {}.",
            archetype.display_name, archetype.description, role
        ),
        tensor,
        grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
        emotional_state,
        self_edge,
        triggers: Vec::new(),
        performance_notes: format!(
            "Primary actions: {}. Speech likelihood: {:.0}%.",
            archetype.action_tendencies.primary_action_types.join(", "),
            archetype.action_tendencies.speech_likelihood * 100.0
        ),
        knows: vec![format!("I am {name}")],
        does_not_know: vec!["The full scope of the story".to_string()],
        capabilities: CapabilityProfile::default(),
    }
}

/// Build an `EmotionalState` from a descriptor emotional profile, sampling
/// intensity within each primary's range.
fn compose_emotional_state<R: Rng>(profile: &EmotionalProfile, rng: &mut R) -> EmotionalState {
    let primaries = profile
        .primaries
        .iter()
        .map(|p| EmotionalPrimary {
            primary_id: p.primary_id.clone(),
            intensity: sample_in_range(&p.intensity, rng),
            awareness: parse_awareness(&p.awareness),
        })
        .collect();

    EmotionalState {
        grammar_id: profile.grammar_id.clone(),
        primaries,
        mood_vector_notes: Vec::new(),
    }
}

/// Build a `SelfEdge` from a descriptor self-edge, sampling each dimension.
fn compose_self_edge<R: Rng>(desc: &DescriptorSelfEdge, rng: &mut R) -> SelfEdge {
    SelfEdge {
        trust: SelfEdgeTrust {
            competence: sample_in_range(&desc.trust_competence, rng),
            intentions: sample_in_range(&desc.trust_intentions, rng),
            reliability: sample_in_range(&desc.trust_reliability, rng),
        },
        affection: sample_in_range(&desc.affection, rng),
        debt: sample_in_range(&desc.debt, rng),
        history_pattern: String::new(),
        history_weight: sample_in_range(&desc.history_weight, rng),
        projection_content: String::new(),
        projection_accuracy: sample_in_range(&desc.projection_accuracy, rng),
        self_knowledge: SelfKnowledge {
            knows: Vec::new(),
            does_not_know: Vec::new(),
        },
    }
}

/// Build a `SceneSetting` from the descriptor settings collection, with optional override.
fn compose_setting(
    settings: &std::collections::HashMap<String, super::descriptors::SettingCollection>,
    genre_id: &str,
    profile_id: &str,
    override_text: Option<&str>,
) -> SceneSetting {
    // Check for genre-keyed settings
    let collection = settings.get(genre_id);

    // Try profile-specific setting first, then default_setting
    let descriptor = collection.and_then(|c| {
        c.profile_settings
            .get(profile_id)
            .or(c.default_setting.as_ref())
    });

    let description = override_text
        .map(|s| s.to_string())
        .or_else(|| {
            descriptor.and_then(|d| {
                // Try description field first, then description_templates from extra
                d.description.clone().or_else(|| {
                    d.extra
                        .get("description_templates")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
            })
        })
        .unwrap_or_else(|| "A scene awaiting its setting.".to_string());

    let affordances = descriptor
        .and_then(|d| {
            d.extra
                .get("affordances")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
        })
        .unwrap_or_default();

    let sensory_details = descriptor
        .and_then(|d| {
            d.extra.get("sensory_palette").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            })
        })
        .unwrap_or_default();

    let aesthetic_detail = descriptor.and_then(|d| d.name.clone()).unwrap_or_default();

    SceneSetting {
        description,
        affordances,
        sensory_details,
        aesthetic_detail,
    }
}

/// Generate stakes from the profile's characteristic events.
fn compose_stakes(profile: &super::descriptors::Profile) -> Vec<String> {
    if profile.characteristic_events.is_empty() {
        return vec![format!("A {} scene", profile.scene_type)];
    }
    profile
        .characteristic_events
        .iter()
        .map(|ce| format!("{} ({})", ce.event_type, ce.emotional_register))
        .collect()
}

/// Generate constraints from genre and profile descriptions.
fn compose_constraints(genre_description: &str, profile_description: &str) -> SceneConstraints {
    SceneConstraints {
        hard: vec![format!("Genre: {genre_description}")],
        soft: vec![format!("Profile: {profile_description}")],
        perceptual: vec!["Characters perceive only what is narratively present".to_string()],
    }
}

/// Sample a single `f32` uniformly within a `RangeBounds` (f64 → f32 cast).
fn sample_in_range<R: Rng>(bounds: &RangeBounds, rng: &mut R) -> f32 {
    let min = bounds.min as f32;
    let max = bounds.max as f32;
    if (max - min).abs() < f32::EPSILON {
        return min;
    }
    rng.random_range(min..=max)
}

/// Parse a scene type string into the enum.
fn parse_scene_type(s: &str) -> SceneType {
    match s.to_lowercase().as_str() {
        "gravitational" => SceneType::Gravitational,
        "threshold" => SceneType::Threshold,
        "connective" => SceneType::Connective,
        "gate" => SceneType::Gate,
        _ => SceneType::Connective,
    }
}

/// Parse a temporal layer string into the enum.
fn parse_temporal_layer(s: &str) -> TemporalLayer {
    match s.to_lowercase().as_str() {
        "bedrock" => TemporalLayer::Bedrock,
        "sediment" => TemporalLayer::Sediment,
        "topsoil" => TemporalLayer::Topsoil,
        "primordial" => TemporalLayer::Primordial,
        _ => TemporalLayer::Sediment,
    }
}

/// Parse an awareness level string into the enum.
fn parse_awareness(s: &str) -> AwarenessLevel {
    match s.to_lowercase().as_str() {
        "articulate" => AwarenessLevel::Articulate,
        "recognizable" => AwarenessLevel::Recognizable,
        "preconscious" => AwarenessLevel::Preconscious,
        "defended" => AwarenessLevel::Defended,
        "structural" => AwarenessLevel::Structural,
        _ => AwarenessLevel::Recognizable,
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

    /// Build a minimal SceneSelections using the first genre, profile, and two archetypes.
    fn make_selections(composer: &SceneComposer, seed: Option<u64>) -> SceneSelections {
        let genre = &composer.descriptors.genres[0];
        let profile_id = genre.valid_profiles.first().unwrap().clone();
        let arch_a = genre.valid_archetypes[0].clone();
        let arch_b = genre
            .valid_archetypes
            .get(1)
            .cloned()
            .unwrap_or_else(|| arch_a.clone());

        SceneSelections {
            genre_id: genre.id.clone(),
            profile_id,
            cast: vec![
                CastSelection {
                    archetype_id: arch_a,
                    name: None,
                    role: "protagonist".to_string(),
                },
                CastSelection {
                    archetype_id: arch_b,
                    name: None,
                    role: "deuteragonist".to_string(),
                },
            ],
            dynamics: Vec::new(),
            title_override: None,
            setting_override: None,
            seed,
        }
    }

    #[test]
    fn compose_basic_scene() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let selections = make_selections(&composer, Some(42));
        let composed = composer
            .compose(&selections)
            .expect("composition should succeed");

        // Two characters produced
        assert_eq!(composed.characters.len(), 2);
        // Cast matches
        assert_eq!(composed.scene.cast.len(), 2);
        // Names are non-empty
        for ch in &composed.characters {
            assert!(!ch.name.is_empty(), "character name should be non-empty");
        }
        // Title defaults to profile display name
        assert!(!composed.scene.title.is_empty());

        println!("Scene: {}", composed.scene.title);
        for ch in &composed.characters {
            println!("  {} — {} axes", ch.name, ch.tensor.axes.len());
        }
    }

    #[test]
    fn compose_is_deterministic_with_seed() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let selections = make_selections(&composer, Some(12345));
        let a = composer.compose(&selections).expect("first compose");
        let b = composer.compose(&selections).expect("second compose");

        // Same names (pool selection is deterministic)
        assert_eq!(a.characters[0].name, b.characters[0].name);
        assert_eq!(a.characters[1].name, b.characters[1].name);

        // Same tensor values (RNG is deterministic)
        for (ca, cb) in a.characters.iter().zip(b.characters.iter()) {
            for (key, entry_a) in &ca.tensor.axes {
                let entry_b = cb.tensor.axes.get(key).expect("same axes");
                assert!(
                    (entry_a.value.central_tendency - entry_b.value.central_tendency).abs()
                        < f32::EPSILON,
                    "central_tendency for axis '{key}' should match"
                );
            }
        }
    }

    #[test]
    fn compose_with_setting_override() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let mut selections = make_selections(&composer, Some(42));
        selections.setting_override = Some("A moonlit glade beside a frozen river".to_string());

        let composed = composer
            .compose(&selections)
            .expect("composition should succeed");
        assert_eq!(
            composed.scene.setting.description,
            "A moonlit glade beside a frozen river"
        );
    }

    #[test]
    fn compose_with_name_override() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let mut selections = make_selections(&composer, Some(42));
        selections.cast[0].name = Some("Gwendolyn".to_string());

        let composed = composer
            .compose(&selections)
            .expect("composition should succeed");
        assert_eq!(composed.characters[0].name, "Gwendolyn");
    }

    #[test]
    fn compose_unknown_genre_returns_error() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let selections = SceneSelections {
            genre_id: "nonexistent_genre".to_string(),
            profile_id: "whatever".to_string(),
            cast: Vec::new(),
            dynamics: Vec::new(),
            title_override: None,
            setting_override: None,
            seed: None,
        };
        let result = composer.compose(&selections);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown genre"));
    }

    #[test]
    fn compose_unknown_archetype_returns_error() {
        let Some(composer) = load_composer() else {
            eprintln!("STORYTELLER_DATA_PATH not set — skipping");
            return;
        };

        let genre = &composer.descriptors.genres[0];
        let profile_id = genre.valid_profiles.first().unwrap().clone();

        let selections = SceneSelections {
            genre_id: genre.id.clone(),
            profile_id,
            cast: vec![CastSelection {
                archetype_id: "nonexistent_archetype".to_string(),
                name: None,
                role: "cast".to_string(),
            }],
            dynamics: Vec::new(),
            title_override: None,
            setting_override: None,
            seed: None,
        };
        let result = composer.compose(&selections);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown archetype"));
    }

    #[test]
    fn sample_in_range_equal_bounds() {
        let bounds = RangeBounds { min: 0.5, max: 0.5 };
        let mut rng = StdRng::seed_from_u64(42);
        let val = sample_in_range(&bounds, &mut rng);
        assert!((val - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_scene_type_variants() {
        assert_eq!(parse_scene_type("gravitational"), SceneType::Gravitational);
        assert_eq!(parse_scene_type("Threshold"), SceneType::Threshold);
        assert_eq!(parse_scene_type("CONNECTIVE"), SceneType::Connective);
        assert_eq!(parse_scene_type("gate"), SceneType::Gate);
        assert_eq!(parse_scene_type("unknown"), SceneType::Connective);
    }

    #[test]
    fn parse_awareness_variants() {
        assert_eq!(parse_awareness("articulate"), AwarenessLevel::Articulate);
        assert_eq!(
            parse_awareness("Recognizable"),
            AwarenessLevel::Recognizable
        );
        assert_eq!(
            parse_awareness("preconscious"),
            AwarenessLevel::Preconscious
        );
        assert_eq!(parse_awareness("defended"), AwarenessLevel::Defended);
        assert_eq!(parse_awareness("structural"), AwarenessLevel::Structural);
        assert_eq!(parse_awareness("unknown"), AwarenessLevel::Recognizable);
    }

    #[test]
    fn parse_temporal_layer_variants() {
        assert_eq!(parse_temporal_layer("bedrock"), TemporalLayer::Bedrock);
        assert_eq!(parse_temporal_layer("Sediment"), TemporalLayer::Sediment);
        assert_eq!(parse_temporal_layer("topsoil"), TemporalLayer::Topsoil);
        assert_eq!(
            parse_temporal_layer("primordial"),
            TemporalLayer::Primordial
        );
        assert_eq!(parse_temporal_layer("unknown"), TemporalLayer::Sediment);
    }

    #[test]
    fn selections_roundtrip_serde() {
        let selections = SceneSelections {
            genre_id: "low_fantasy_folklore".to_string(),
            profile_id: "confrontation".to_string(),
            cast: vec![CastSelection {
                archetype_id: "the_reluctant_hero".to_string(),
                name: Some("Aelwyn".to_string()),
                role: "protagonist".to_string(),
            }],
            dynamics: vec![DynamicSelection {
                dynamic_id: "mentor_student".to_string(),
                cast_index_a: 0,
                cast_index_b: 1,
            }],
            title_override: Some("The Frozen Path".to_string()),
            setting_override: None,
            seed: Some(42),
        };

        let json = serde_json::to_string(&selections).expect("serialize");
        let back: SceneSelections = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.genre_id, selections.genre_id);
        assert_eq!(back.profile_id, selections.profile_id);
        assert_eq!(back.cast.len(), 1);
        assert_eq!(back.dynamics.len(), 1);
        assert_eq!(back.seed, Some(42));
    }
}
