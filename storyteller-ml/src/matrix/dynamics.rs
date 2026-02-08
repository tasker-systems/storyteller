//! Relational dynamic instantiation — descriptor → DirectedEdge pair.

use rand::Rng;

use storyteller_core::types::entity::EntityId;
use storyteller_core::types::relational::{
    DirectedEdge, InformationState, RelationalSubstrate, TopologicalRole,
};
use storyteller_core::types::tensor::AxisValue;

use super::descriptors::{DynamicDescriptor, SubstrateTemplate};

/// Instantiate a pair of directed edges from a dynamic descriptor.
///
/// Returns `(edge_a_to_b, edge_b_to_a)`.
pub fn instantiate_edges(
    dynamic: &DynamicDescriptor,
    source_a: EntityId,
    source_b: EntityId,
    rng: &mut impl Rng,
) -> (DirectedEdge, DirectedEdge) {
    let edge_ab = make_edge(
        source_a,
        source_b,
        &dynamic.edge_a_to_b,
        &dynamic.topology_a,
        rng,
    );
    let edge_ba = make_edge(
        source_b,
        source_a,
        &dynamic.edge_b_to_a,
        &dynamic.topology_b,
        rng,
    );
    (edge_ab, edge_ba)
}

fn make_edge(
    source: EntityId,
    target: EntityId,
    template: &SubstrateTemplate,
    _topology: &str,
    rng: &mut impl Rng,
) -> DirectedEdge {
    DirectedEdge {
        source,
        target,
        substrate: sample_substrate(template, rng),
        information_state: InformationState {
            known_facts: vec![],
            beliefs: vec![],
            blind_spots: vec![],
        },
    }
}

fn sample_substrate(template: &SubstrateTemplate, rng: &mut impl Rng) -> RelationalSubstrate {
    RelationalSubstrate {
        trust_reliability: sample_axis_value(template.trust_reliability.sample(rng)),
        trust_competence: sample_axis_value(template.trust_competence.sample(rng)),
        trust_benevolence: sample_axis_value(template.trust_benevolence.sample(rng)),
        affection: sample_axis_value(template.affection.sample(rng)),
        debt: sample_axis_value(template.debt.sample(rng)),
    }
}

fn sample_axis_value(central: f32) -> AxisValue {
    let variance = 0.15;
    AxisValue {
        central_tendency: central,
        variance,
        range_low: (central - variance * 1.5).max(-1.0),
        range_high: (central + variance * 1.5).min(1.0),
    }
}

/// Parse a topology string to a [`TopologicalRole`].
pub fn parse_topology(s: &str) -> TopologicalRole {
    match s {
        "Gate" => TopologicalRole::Gate,
        "Bridge" => TopologicalRole::Bridge,
        "Hub" => TopologicalRole::Hub,
        _ => TopologicalRole::Periphery,
    }
}

#[cfg(test)]
mod tests {
    use super::super::descriptors::ValueRange;
    use super::*;

    #[test]
    fn parse_topologies() {
        assert_eq!(parse_topology("Gate"), TopologicalRole::Gate);
        assert_eq!(parse_topology("Bridge"), TopologicalRole::Bridge);
        assert_eq!(parse_topology("Hub"), TopologicalRole::Hub);
        assert_eq!(parse_topology("Periphery"), TopologicalRole::Periphery);
        assert_eq!(parse_topology("unknown"), TopologicalRole::Periphery);
    }

    #[test]
    fn sample_axis_value_produces_valid_ranges() {
        let av = sample_axis_value(0.5);
        assert!(av.range_low <= av.central_tendency);
        assert!(av.range_high >= av.central_tendency);
        assert!(av.range_low >= -1.0);
        assert!(av.range_high <= 1.0);
    }

    fn test_dynamic() -> DynamicDescriptor {
        DynamicDescriptor {
            id: "test".to_string(),
            display_name: "Test".to_string(),
            description: "Test dynamic".to_string(),
            role_a: "role_a".to_string(),
            role_b: "role_b".to_string(),
            edge_a_to_b: SubstrateTemplate {
                trust_reliability: ValueRange { min: 0.3, max: 0.6 },
                trust_competence: ValueRange { min: 0.3, max: 0.6 },
                trust_benevolence: ValueRange { min: 0.3, max: 0.6 },
                affection: ValueRange { min: 0.3, max: 0.6 },
                debt: ValueRange { min: 0.1, max: 0.3 },
            },
            edge_b_to_a: SubstrateTemplate {
                trust_reliability: ValueRange { min: 0.4, max: 0.7 },
                trust_competence: ValueRange { min: 0.4, max: 0.7 },
                trust_benevolence: ValueRange { min: 0.4, max: 0.7 },
                affection: ValueRange { min: 0.3, max: 0.5 },
                debt: ValueRange { min: 0.2, max: 0.4 },
            },
            topology_a: "Hub".to_string(),
            topology_b: "Periphery".to_string(),
        }
    }

    #[test]
    fn instantiate_edges_produces_two_edges() {
        let dynamic = test_dynamic();
        let a = EntityId::new();
        let b = EntityId::new();
        let mut rng = rand::rng();
        let (edge_ab, edge_ba) = instantiate_edges(&dynamic, a, b, &mut rng);
        assert_eq!(edge_ab.source, a);
        assert_eq!(edge_ab.target, b);
        assert_eq!(edge_ba.source, b);
        assert_eq!(edge_ba.target, a);
    }
}
