//! Combinatorial matrix generation — iterates valid cells and produces scenario skeletons.

use std::collections::BTreeMap;

use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;
use serde::{Deserialize, Serialize};

use storyteller_core::types::character::CharacterSheet;
use storyteller_core::types::relational::{DirectedEdge, TopologicalRole};

use crate::feature_schema::{EventFeatureInput, SceneFeatureInput};

use super::archetypes::instantiate_character;
use super::descriptors::{CrossSample, DescriptorSet, GenreDescriptor};
use super::dynamics::{instantiate_edges, parse_topology};
use super::profiles::instantiate_scene_and_event;

/// A single cell in the combinatorial matrix — identifies which descriptors were combined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixCell {
    pub archetype_a: String,
    pub archetype_b: String,
    pub dynamic: String,
    /// Whether character A takes role_a (true) or role_b (false) in the dynamic.
    pub a_is_role_a: bool,
    pub profile: String,
    pub genre: String,
}

/// A fully instantiated scenario ready for label generation.
#[derive(Debug, Clone)]
pub struct ScenarioSkeleton {
    pub cell: MatrixCell,
    pub character_a: CharacterSheet,
    pub character_b: CharacterSheet,
    pub edge_a_to_b: DirectedEdge,
    pub edge_b_to_a: DirectedEdge,
    pub topology_a: TopologicalRole,
    pub topology_b: TopologicalRole,
    pub scene: SceneFeatureInput,
    pub event: EventFeatureInput,
    pub cross_sample: CrossSample,
    pub variation_index: u32,
}

/// Generate the combinatorial matrix, sampling `count` valid cells.
///
/// Iterates archetype_a × archetype_b (a ≠ b) × dynamic (both role assignments) × profile,
/// filtering by genre validity gates. Samples down to `count` from the valid cells.
pub fn generate_matrix(
    descriptors: &DescriptorSet,
    genre: &GenreDescriptor,
    count: usize,
    variations: u32,
    rng: &mut impl Rng,
) -> Vec<ScenarioSkeleton> {
    // Collect all valid cells
    let mut valid_cells: Vec<MatrixCell> = Vec::new();

    for arch_a in &genre.valid_archetypes {
        for arch_b in &genre.valid_archetypes {
            if arch_a == arch_b {
                continue;
            }
            for dyn_id in &genre.valid_dynamics {
                for prof_id in &genre.valid_profiles {
                    // Check excluded combinations
                    if is_excluded(genre, arch_a, dyn_id, prof_id)
                        || is_excluded(genre, arch_b, dyn_id, prof_id)
                    {
                        continue;
                    }

                    // Both role assignments
                    for a_is_role_a in [true, false] {
                        valid_cells.push(MatrixCell {
                            archetype_a: arch_a.clone(),
                            archetype_b: arch_b.clone(),
                            dynamic: dyn_id.clone(),
                            a_is_role_a,
                            profile: prof_id.clone(),
                            genre: genre.id.clone(),
                        });
                    }
                }
            }
        }
    }

    // Sample down to count
    valid_cells.shuffle(rng);
    valid_cells.truncate(count);

    // Instantiate skeletons with variations
    let mut skeletons = Vec::new();
    for cell in &valid_cells {
        for var_idx in 0..variations {
            let cross_sample = sample_cross_dimensions(descriptors, rng);

            let arch_a = descriptors.archetype(&cell.archetype_a).unwrap();
            let arch_b = descriptors.archetype(&cell.archetype_b).unwrap();
            let dynamic = descriptors.dynamic(&cell.dynamic).unwrap();
            let profile = descriptors.profile(&cell.profile).unwrap();

            let character_a = instantiate_character(arch_a, &cross_sample, descriptors, rng);
            let character_b = instantiate_character(arch_b, &cross_sample, descriptors, rng);

            let (edge_ab, edge_ba) =
                instantiate_edges(dynamic, character_a.entity_id, character_b.entity_id, rng);

            let (topology_a, topology_b) = if cell.a_is_role_a {
                (
                    parse_topology(&dynamic.topology_a),
                    parse_topology(&dynamic.topology_b),
                )
            } else {
                (
                    parse_topology(&dynamic.topology_b),
                    parse_topology(&dynamic.topology_a),
                )
            };

            let (scene, event) = instantiate_scene_and_event(profile, rng);

            skeletons.push(ScenarioSkeleton {
                cell: cell.clone(),
                character_a,
                character_b,
                edge_a_to_b: edge_ab,
                edge_b_to_a: edge_ba,
                topology_a,
                topology_b,
                scene,
                event,
                cross_sample: cross_sample.clone(),
                variation_index: var_idx,
            });
        }
    }

    skeletons
}

fn is_excluded(genre: &GenreDescriptor, archetype: &str, dynamic: &str, profile: &str) -> bool {
    genre.excluded_combinations.iter().any(|exc| {
        let arch_match = exc.archetype.as_deref().is_none_or(|a| a == archetype);
        let dyn_match = exc.dynamic.as_deref().is_none_or(|d| d == dynamic);
        let prof_match = exc.profile.as_deref().is_none_or(|p| p == profile);
        arch_match && dyn_match && prof_match
    })
}

fn sample_cross_dimensions(descriptors: &DescriptorSet, rng: &mut impl Rng) -> CrossSample {
    let mut values = BTreeMap::new();
    for dim in &descriptors.cross_dimensions.dimensions {
        if let Some(val) = dim.values.choose(rng) {
            values.insert(dim.id.clone(), val.id.clone());
        }
    }
    CrossSample { values }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_excluded_matches_archetype_profile() {
        use super::super::descriptors::{ExcludedCombination, GenreDescriptor};
        let genre = GenreDescriptor {
            id: "test".to_string(),
            display_name: "Test".to_string(),
            description: "Test".to_string(),
            valid_archetypes: vec![],
            valid_dynamics: vec![],
            valid_profiles: vec![],
            excluded_combinations: vec![ExcludedCombination {
                archetype: Some("fey_outsider".to_string()),
                dynamic: None,
                profile: Some("physical_challenge_cooperation".to_string()),
                reason: "test".to_string(),
            }],
        };

        assert!(is_excluded(
            &genre,
            "fey_outsider",
            "any_dynamic",
            "physical_challenge_cooperation"
        ));
        assert!(!is_excluded(
            &genre,
            "fey_outsider",
            "any_dynamic",
            "quiet_reunion"
        ));
        assert!(!is_excluded(
            &genre,
            "wandering_artist",
            "any_dynamic",
            "physical_challenge_cooperation"
        ));
    }
}
