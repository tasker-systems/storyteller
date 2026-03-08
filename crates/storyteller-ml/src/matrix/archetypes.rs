//! Character archetype instantiation — descriptor → CharacterSheet.
//!
//! Each archetype descriptor defines value ranges. This module samples
//! concrete values within those ranges and applies cross-dimension
//! modifiers to produce a complete [`CharacterSheet`].

use rand::Rng;

use storyteller_core::types::character::{
    CharacterSheet, CharacterTensor, EmotionalPrimary, EmotionalState, SelfEdge, SelfEdgeTrust,
    SelfKnowledge,
};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::tensor::{AwarenessLevel, AxisValue, Provenance, TemporalLayer};
use storyteller_core::types::world_model::CapabilityProfile;

use super::descriptors::{ArchetypeDescriptor, CrossSample, DescriptorSet};

/// Instantiate a concrete [`CharacterSheet`] from an archetype descriptor,
/// cross-dimension sample, and RNG.
pub fn instantiate_character(
    archetype: &ArchetypeDescriptor,
    cross_sample: &CrossSample,
    descriptors: &DescriptorSet,
    rng: &mut impl Rng,
) -> CharacterSheet {
    let mut tensor = CharacterTensor::new();

    // Sample each axis within descriptor ranges
    for axis in &archetype.axes {
        let ct = axis.central_tendency.sample(rng);
        let var = axis.variance.sample(rng);
        let rl = axis.range_low.sample(rng);
        let rh = axis.range_high.sample(rng);

        tensor.insert(
            &axis.axis_id,
            AxisValue {
                central_tendency: ct,
                variance: var,
                range_low: rl.min(ct),
                range_high: rh.max(ct),
            },
            parse_temporal_layer(&axis.layer),
            parse_provenance(&axis.provenance),
        );
    }

    // Apply cross-dimension axis modifiers
    for dim in &descriptors.cross_dimensions.dimensions {
        if let Some(value_id) = cross_sample.values.get(&dim.id) {
            if let Some(value) = dim.values.iter().find(|v| v.id == *value_id) {
                for modifier in &value.axis_modifiers {
                    if let Some(entry) = tensor.axes.get_mut(&modifier.axis_id) {
                        entry.value.central_tendency =
                            (entry.value.central_tendency + modifier.additive).clamp(-1.0, 1.0);
                    }
                }
            }
        }
    }

    // Sample emotional state
    let primaries: Vec<EmotionalPrimary> = archetype
        .default_emotional_profile
        .primaries
        .iter()
        .map(|p| EmotionalPrimary {
            primary_id: p.primary_id.clone(),
            intensity: p.intensity.sample(rng).clamp(0.0, 1.0),
            awareness: parse_awareness(&p.awareness),
        })
        .collect();

    let emotional_state = EmotionalState {
        grammar_id: archetype.default_emotional_profile.grammar_id.clone(),
        primaries,
        mood_vector_notes: vec![],
    };

    // Sample self-edge
    let se = &archetype.default_self_edge;
    let self_edge = SelfEdge {
        trust: SelfEdgeTrust {
            competence: se.trust_competence.sample(rng).clamp(0.0, 1.0),
            intentions: se.trust_intentions.sample(rng).clamp(0.0, 1.0),
            reliability: se.trust_reliability.sample(rng).clamp(0.0, 1.0),
        },
        affection: se.affection.sample(rng).clamp(0.0, 1.0),
        debt: se.debt.sample(rng).clamp(0.0, 1.0),
        history_pattern: format!("{} pattern", archetype.display_name),
        history_weight: se.history_weight.sample(rng).clamp(0.0, 1.0),
        projection_content: format!("{} self-image", archetype.display_name),
        projection_accuracy: se.projection_accuracy.sample(rng).clamp(0.0, 1.0),
        self_knowledge: SelfKnowledge {
            knows: vec![],
            does_not_know: vec![],
        },
    };

    CharacterSheet {
        entity_id: EntityId::new(),
        name: format!("{} instance", archetype.display_name),
        voice: format!("{} voice register", archetype.display_name),
        backstory: archetype.description.clone(),
        tensor,
        grammar_id: archetype.default_emotional_profile.grammar_id.clone(),
        emotional_state,
        self_edge,
        triggers: vec![],
        performance_notes: format!(
            "Archetype: {}. Primary actions: {:?}",
            archetype.display_name, archetype.action_tendencies.primary_action_types,
        ),
        knows: vec![],
        does_not_know: vec![],
        capabilities: CapabilityProfile::default(),
    }
}

fn parse_temporal_layer(s: &str) -> TemporalLayer {
    match s.to_lowercase().as_str() {
        "topsoil" => TemporalLayer::Topsoil,
        "sediment" => TemporalLayer::Sediment,
        "bedrock" => TemporalLayer::Bedrock,
        "primordial" => TemporalLayer::Primordial,
        _ => TemporalLayer::Sediment,
    }
}

fn parse_provenance(s: &str) -> Provenance {
    match s.to_lowercase().as_str() {
        "authored" => Provenance::Authored,
        "inferred" => Provenance::Inferred,
        "generated" => Provenance::Generated,
        "confirmed" => Provenance::Confirmed,
        "overridden" => Provenance::Overridden,
        _ => Provenance::Generated,
    }
}

pub fn parse_awareness(s: &str) -> AwarenessLevel {
    match s.to_lowercase().as_str() {
        "articulate" => AwarenessLevel::Articulate,
        "recognizable" => AwarenessLevel::Recognizable,
        "preconscious" => AwarenessLevel::Preconscious,
        "defended" => AwarenessLevel::Defended,
        "structural" => AwarenessLevel::Structural,
        _ => AwarenessLevel::Recognizable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_temporal_layers() {
        assert_eq!(parse_temporal_layer("topsoil"), TemporalLayer::Topsoil);
        assert_eq!(parse_temporal_layer("Bedrock"), TemporalLayer::Bedrock);
        assert_eq!(
            parse_temporal_layer("primordial"),
            TemporalLayer::Primordial
        );
        assert_eq!(parse_temporal_layer("unknown"), TemporalLayer::Sediment);
    }

    #[test]
    fn parse_provenances() {
        assert_eq!(parse_provenance("authored"), Provenance::Authored);
        assert_eq!(parse_provenance("Generated"), Provenance::Generated);
        assert_eq!(parse_provenance("unknown"), Provenance::Generated);
    }

    #[test]
    fn parse_awareness_levels() {
        assert_eq!(parse_awareness("articulate"), AwarenessLevel::Articulate);
        assert_eq!(parse_awareness("Defended"), AwarenessLevel::Defended);
        assert_eq!(parse_awareness("structural"), AwarenessLevel::Structural);
        assert_eq!(parse_awareness("unknown"), AwarenessLevel::Recognizable);
    }
}
