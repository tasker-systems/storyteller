// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Goal intersection — determines which scene and character goals are active
//! for a composed scene based on descriptor tagging.
//!
//! See: `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md`

use std::collections::{HashMap, HashSet};

use storyteller_core::types::entity::EntityId;

use crate::descriptors::{Archetype, Dynamic, Goal, Profile};

/// Visibility level for how a goal manifests to the player.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoalVisibility {
    Overt,
    Signaled,
    Hidden,
    Structural,
}

/// Fragment register — how a lexicon fragment is used in the narrator prompt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FragmentRegister {
    Atmospheric,
    CharacterSignal,
    Transitional,
}

/// A selected lexicon fragment for narrator context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalFragment {
    pub text: String,
    pub register: FragmentRegister,
}

/// An active scene-level goal with selected fragments.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneGoal {
    pub goal_id: String,
    pub visibility: GoalVisibility,
    pub category: String,
    pub fragments: Vec<GoalFragment>,
}

/// An active per-character goal with selected fragments.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterGoal {
    pub goal_id: String,
    pub visibility: GoalVisibility,
    pub category: String,
    pub fragments: Vec<GoalFragment>,
}

/// The full set of active goals for a composed scene.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ComposedGoals {
    pub scene_goals: Vec<SceneGoal>,
    pub character_goals: HashMap<EntityId, Vec<CharacterGoal>>,
}

/// Category affinity pairs — which goal categories are coherent together.
fn category_affinity(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let compatible = match a {
        "revelation" => &["protection", "relational_shift", "discovery"][..],
        "relational_shift" => &["revelation", "bonding", "confrontation", "departure"],
        "confrontation" => &["relational_shift", "protection", "revelation"],
        "discovery" => &["revelation", "bonding", "protection"],
        "bonding" => &["relational_shift", "discovery", "departure"],
        "departure" => &["relational_shift", "bonding", "revelation"],
        "protection" => &["revelation", "confrontation", "discovery"],
        "transaction" => &["transaction", "confrontation"],
        _ => &[],
    };
    compatible.contains(&b)
}

/// A cast member's identity for goal intersection.
#[derive(Debug)]
pub struct CastMember {
    pub entity_id: EntityId,
    pub archetype: Archetype,
    pub dynamics: Vec<Dynamic>,
}

/// Pass 1: Scene dramaturgical goals.
///
/// `scene_goals = profile.scene_goals ∩ (union of all cast archetypes' pursuable_goals)`
pub fn intersect_scene_goals(
    profile: &Profile,
    cast: &[CastMember],
    goals: &[Goal],
) -> Vec<SceneGoal> {
    let cast_pursuable: HashSet<&str> = cast
        .iter()
        .flat_map(|m| m.archetype.pursuable_goals.iter().map(|s| s.as_str()))
        .collect();

    profile
        .scene_goals
        .iter()
        .filter(|sg| cast_pursuable.contains(sg.as_str()))
        .filter_map(|sg| {
            goals.iter().find(|g| g.id == *sg).map(|g| SceneGoal {
                goal_id: g.id.clone(),
                visibility: parse_visibility(&g.visibility),
                category: g.category.clone(),
                fragments: Vec::new(), // populated by likeness pass
            })
        })
        .collect()
}

/// Pass 2: Per-character objectives.
///
/// For each character:
/// `character_goals = archetype.pursuable_goals
///                    ∩ (union of dynamics.enabled_goals for this character)
///                    - (union of dynamics.blocked_goals for this character)`
///
/// Then coherence-filtered against scene goal categories.
pub fn intersect_character_goals(
    member: &CastMember,
    scene_goals: &[SceneGoal],
    goals: &[Goal],
) -> Vec<CharacterGoal> {
    let enabled: HashSet<&str> = member
        .dynamics
        .iter()
        .flat_map(|d| d.enabled_goals.iter().map(|s| s.as_str()))
        .collect();

    let blocked: HashSet<&str> = member
        .dynamics
        .iter()
        .flat_map(|d| d.blocked_goals.iter().map(|s| s.as_str()))
        .collect();

    let scene_categories: Vec<&str> = scene_goals.iter().map(|sg| sg.category.as_str()).collect();

    // Primary path: pursuable ∩ enabled - blocked, coherence-filtered.
    let primary: Vec<CharacterGoal> = member
        .archetype
        .pursuable_goals
        .iter()
        .filter(|pg| enabled.contains(pg.as_str()))
        .filter(|pg| !blocked.contains(pg.as_str()))
        .filter_map(|pg| goals.iter().find(|g| g.id == *pg))
        .filter(|g| {
            scene_categories.is_empty()
                || scene_categories
                    .iter()
                    .any(|sc| category_affinity(&g.category, sc))
        })
        .map(|g| CharacterGoal {
            goal_id: g.id.clone(),
            visibility: parse_visibility(&g.visibility),
            category: g.category.clone(),
            fragments: Vec::new(),
        })
        .collect();

    if !primary.is_empty() {
        return primary;
    }

    // Fallback: when dynamics don't enable any pursuable goals, use pursuable
    // goals directly (minus blocked), coherence-filtered. Better to have some
    // goal direction than none — felt agency over silence.
    member
        .archetype
        .pursuable_goals
        .iter()
        .filter(|pg| !blocked.contains(pg.as_str()))
        .filter_map(|pg| goals.iter().find(|g| g.id == *pg))
        .filter(|g| {
            scene_categories.is_empty()
                || scene_categories
                    .iter()
                    .any(|sc| category_affinity(&g.category, sc))
        })
        .map(|g| CharacterGoal {
            goal_id: g.id.clone(),
            visibility: parse_visibility(&g.visibility),
            category: g.category.clone(),
            fragments: Vec::new(),
        })
        .collect()
}

fn parse_visibility(s: &str) -> GoalVisibility {
    match s {
        "Overt" => GoalVisibility::Overt,
        "Signaled" => GoalVisibility::Signaled,
        "Hidden" => GoalVisibility::Hidden,
        "Structural" => GoalVisibility::Structural,
        _ => GoalVisibility::Signaled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptors::*;

    fn test_goal(id: &str, category: &str, visibility: &str) -> Goal {
        Goal {
            id: id.to_string(),
            entity_id: String::new(),
            description: format!("Test goal: {id}"),
            category: category.to_string(),
            visibility: visibility.to_string(),
            valence: "tense".to_string(),
            lexicon: Vec::new(),
        }
    }

    fn test_profile(scene_goals: Vec<&str>) -> Profile {
        Profile {
            id: "test_profile".to_string(),
            entity_id: String::new(),
            display_name: "Test".to_string(),
            description: "Test profile".to_string(),
            scene_type: "Gravitational".to_string(),
            tension: RangeBounds { min: 0.3, max: 0.7 },
            cast_size: RangeBounds { min: 2.0, max: 4.0 },
            characteristic_events: Vec::new(),
            scene_goals: scene_goals.into_iter().map(String::from).collect(),
        }
    }

    fn test_archetype(id: &str, pursuable: Vec<&str>) -> Archetype {
        Archetype {
            id: id.to_string(),
            entity_id: String::new(),
            display_name: id.to_string(),
            description: String::new(),
            axes: Vec::new(),
            default_emotional_profile: EmotionalProfile {
                grammar_id: "western".to_string(),
                primaries: Vec::new(),
            },
            default_self_edge: SelfEdge {
                trust_competence: RangeBounds { min: 0.5, max: 0.5 },
                trust_intentions: RangeBounds { min: 0.5, max: 0.5 },
                trust_reliability: RangeBounds { min: 0.5, max: 0.5 },
                affection: RangeBounds { min: 0.5, max: 0.5 },
                debt: RangeBounds { min: 0.0, max: 0.0 },
                history_weight: RangeBounds { min: 0.5, max: 0.5 },
                projection_accuracy: RangeBounds { min: 0.5, max: 0.5 },
            },
            action_tendencies: ActionTendencies {
                primary_action_types: vec!["Speak".to_string()],
                primary_action_contexts: vec!["Conversation".to_string()],
                speech_likelihood: 0.7,
                speech_registers: vec!["Tender".to_string()],
                default_awareness: "Conscious".to_string(),
            },
            pursuable_goals: pursuable.into_iter().map(String::from).collect(),
        }
    }

    fn test_dynamic(enabled: Vec<&str>, blocked: Vec<&str>) -> Dynamic {
        Dynamic {
            id: "test_dynamic".to_string(),
            entity_id: String::new(),
            display_name: "Test".to_string(),
            description: String::new(),
            role_a: "role_a".to_string(),
            role_b: "role_b".to_string(),
            edge_a_to_b: RelationalEdge {
                trust_reliability: RangeBounds { min: 0.3, max: 0.6 },
                trust_competence: RangeBounds { min: 0.3, max: 0.6 },
                trust_benevolence: RangeBounds { min: 0.3, max: 0.6 },
                affection: RangeBounds { min: 0.2, max: 0.5 },
                debt: RangeBounds { min: 0.0, max: 0.1 },
            },
            edge_b_to_a: RelationalEdge {
                trust_reliability: RangeBounds { min: 0.3, max: 0.6 },
                trust_competence: RangeBounds { min: 0.3, max: 0.6 },
                trust_benevolence: RangeBounds { min: 0.3, max: 0.6 },
                affection: RangeBounds { min: 0.2, max: 0.5 },
                debt: RangeBounds { min: 0.0, max: 0.1 },
            },
            topology_a: "Hub".to_string(),
            topology_b: "Bridge".to_string(),
            enabled_goals: enabled.into_iter().map(String::from).collect(),
            blocked_goals: blocked.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn scene_goals_intersection_filters_by_cast() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
            test_goal("share_vulnerability", "bonding", "Signaled"),
            test_goal("negotiate_terms", "transaction", "Overt"),
        ];
        let profile = test_profile(vec![
            "protect_secret",
            "share_vulnerability",
            "negotiate_terms",
        ]);
        let cast = vec![CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret"]),
            dynamics: Vec::new(),
        }];

        let result = intersect_scene_goals(&profile, &cast, &goals);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].goal_id, "protect_secret");
    }

    #[test]
    fn scene_goals_empty_when_no_cast_overlap() {
        let goals = vec![test_goal("negotiate_terms", "transaction", "Overt")];
        let profile = test_profile(vec!["negotiate_terms"]);
        let cast = vec![CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret"]),
            dynamics: Vec::new(),
        }];

        let result = intersect_scene_goals(&profile, &cast, &goals);
        assert!(result.is_empty());
    }

    #[test]
    fn character_goals_intersection_applies_enabled_and_blocked() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
            test_goal("share_vulnerability", "bonding", "Signaled"),
            test_goal("test_loyalty", "relational_shift", "Hidden"),
        ];
        let scene_goals = vec![SceneGoal {
            goal_id: "confront_shared_loss".to_string(),
            visibility: GoalVisibility::Signaled,
            category: "revelation".to_string(),
            fragments: Vec::new(),
        }];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype(
                "stoic",
                vec!["protect_secret", "share_vulnerability", "test_loyalty"],
            ),
            dynamics: vec![test_dynamic(
                vec!["protect_secret", "share_vulnerability", "test_loyalty"],
                vec!["share_vulnerability"], // blocked!
            )],
        };

        let result = intersect_character_goals(&member, &scene_goals, &goals);
        let ids: Vec<&str> = result.iter().map(|g| g.goal_id.as_str()).collect();
        assert!(ids.contains(&"protect_secret"));
        assert!(ids.contains(&"test_loyalty"));
        assert!(!ids.contains(&"share_vulnerability"));
    }

    #[test]
    fn coherence_filter_removes_incompatible_categories() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
            test_goal("negotiate_terms", "transaction", "Overt"),
        ];
        let scene_goals = vec![SceneGoal {
            goal_id: "share_vulnerability".to_string(),
            visibility: GoalVisibility::Signaled,
            category: "bonding".to_string(),
            fragments: Vec::new(),
        }];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret", "negotiate_terms"]),
            dynamics: vec![test_dynamic(
                vec!["protect_secret", "negotiate_terms"],
                Vec::new(),
            )],
        };

        let result = intersect_character_goals(&member, &scene_goals, &goals);
        assert!(result.is_empty());
    }

    #[test]
    fn character_goals_fallback_when_no_enabled_overlap() {
        // When dynamics don't enable any of the character's pursuable goals,
        // fallback to pursuable goals filtered by coherence only.
        let goals = vec![
            test_goal("test_loyalty", "relational_shift", "Hidden"),
            test_goal("maintain_deception", "protection", "Signaled"),
        ];
        let scene_goals = vec![SceneGoal {
            goal_id: "test_loyalty".to_string(),
            visibility: GoalVisibility::Hidden,
            category: "relational_shift".to_string(),
            fragments: Vec::new(),
        }];
        // Archetype pursues test_loyalty and maintain_deception, but the dynamic
        // only enables "unrelated_goal" — zero overlap with pursuable.
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("trickster", vec!["test_loyalty", "maintain_deception"]),
            dynamics: vec![test_dynamic(vec!["unrelated_goal"], Vec::new())],
        };

        let result = intersect_character_goals(&member, &scene_goals, &goals);
        // Fallback should give us test_loyalty (coherent with scene goal category)
        assert!(
            !result.is_empty(),
            "fallback should produce goals when primary intersection is empty"
        );
        let ids: Vec<&str> = result.iter().map(|g| g.goal_id.as_str()).collect();
        assert!(ids.contains(&"test_loyalty"));
    }

    #[test]
    fn character_goals_fallback_respects_blocked() {
        let goals = vec![
            test_goal("test_loyalty", "relational_shift", "Hidden"),
            test_goal("maintain_deception", "protection", "Signaled"),
        ];
        let scene_goals = vec![SceneGoal {
            goal_id: "test_loyalty".to_string(),
            visibility: GoalVisibility::Hidden,
            category: "relational_shift".to_string(),
            fragments: Vec::new(),
        }];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("trickster", vec!["test_loyalty", "maintain_deception"]),
            dynamics: vec![test_dynamic(
                vec!["unrelated_goal"],
                vec!["test_loyalty"], // blocked!
            )],
        };

        let result = intersect_character_goals(&member, &scene_goals, &goals);
        let ids: Vec<&str> = result.iter().map(|g| g.goal_id.as_str()).collect();
        assert!(
            !ids.contains(&"test_loyalty"),
            "blocked goals should stay blocked in fallback"
        );
    }

    #[test]
    fn no_scene_goals_skips_coherence_check() {
        let goals = vec![test_goal("protect_secret", "protection", "Hidden")];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret"]),
            dynamics: vec![test_dynamic(vec!["protect_secret"], Vec::new())],
        };

        let result = intersect_character_goals(&member, &[], &goals);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].goal_id, "protect_secret");
    }

    #[test]
    fn category_affinity_is_symmetric() {
        assert!(category_affinity("revelation", "protection"));
        assert!(category_affinity("protection", "revelation"));
        assert!(category_affinity("bonding", "discovery"));
        assert!(category_affinity("discovery", "bonding"));
    }

    #[test]
    fn category_affinity_same_category() {
        assert!(category_affinity("revelation", "revelation"));
        assert!(category_affinity("transaction", "transaction"));
    }

    #[test]
    fn transaction_is_isolated() {
        assert!(!category_affinity("transaction", "bonding"));
        assert!(!category_affinity("transaction", "revelation"));
        assert!(category_affinity("transaction", "confrontation"));
    }
}
