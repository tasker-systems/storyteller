// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Action arbitration engine — deterministic rules for checking player actions
//! against world constraints.
//!
//! The arbitration system evaluates player input against genre constraints,
//! spatial zones, and environmental rules before action resolution. It returns
//! `ActionPossibility` — either `Permitted`, `Impossible`, or `Ambiguous`.
//!
//! `Permitted` and `Impossible` are deterministic. `Ambiguous` triggers the
//! small LLM fallback for edge cases the rules engine cannot resolve.

use storyteller_core::types::capability_lexicon::CapabilityLexicon;
use storyteller_core::types::world_model::{
    ActionPossibility, ConstraintViolation, EnvironmentalConstraint, GenreConstraint,
    NarrativeDistanceZone,
};

/// Keywords indicating physical touch/contact actions.
const TOUCH_KEYWORDS: &[&str] = &[
    "touch", "grab", "embrace", "hand", "hold", "push", "shove", "punch", "kick", "hug", "slap",
    "pat", "stroke", "caress",
];

/// Keywords indicating speech/verbal actions.
const SPEECH_KEYWORDS: &[&str] = &[
    "whisper", "say", "tell", "speak", "ask", "shout", "yell", "call", "murmur", "mutter",
    "exclaim",
];

/// Check player input against genre constraints using the capability lexicon.
///
/// Returns `Impossible` if any `Forbidden` constraint matches the input.
/// Returns `Ambiguous` if a `Conditional` constraint matches (since we cannot
/// deterministically verify whether the conditions are met).
/// Returns `Permitted` if no constraints match.
pub fn check_genre_constraints(
    player_input: &str,
    constraints: &[GenreConstraint],
    lexicon: &CapabilityLexicon,
) -> ActionPossibility {
    let matched_capabilities = lexicon.match_text(player_input);

    let mut ambiguous_result: Option<ActionPossibility> = None;

    for constraint in constraints {
        match constraint {
            GenreConstraint::Forbidden {
                capability, reason, ..
            } => {
                if matched_capabilities.iter().any(|c| c == capability) {
                    return ActionPossibility::Impossible {
                        reason: ConstraintViolation {
                            constraint_name: format!("genre:forbidden:{capability}"),
                            description: reason.clone(),
                        },
                    };
                }
            }
            GenreConstraint::Conditional {
                capability,
                requires,
            } => {
                if matched_capabilities.iter().any(|c| c == capability)
                    && ambiguous_result.is_none()
                {
                    ambiguous_result = Some(ActionPossibility::Ambiguous {
                        known_constraints: requires
                            .iter()
                            .map(|r| EnvironmentalConstraint {
                                name: format!("conditional:{capability}"),
                                description: r.clone(),
                                affected_action_types: vec![capability.clone()],
                            })
                            .collect(),
                        uncertainty: format!(
                            "Capability '{capability}' requires conditions that cannot be verified deterministically: {}",
                            requires.join(", ")
                        ),
                    });
                }
            }
            GenreConstraint::PhysicsOverride { .. } => {
                // Physics overrides affect resolution, not possibility.
                // They are passed through to the resolver as conditions.
            }
        }
    }

    if let Some(ambiguous) = ambiguous_result {
        return ambiguous;
    }

    ActionPossibility::Permitted {
        conditions: Vec::new(),
    }
}

/// Check player input against spatial distance constraints.
///
/// Detects touch-related and speech-related keywords in the input and
/// checks whether the actor's distance zone permits those actions.
/// Returns `Impossible` with a violation if the zone blocks the action,
/// or `Permitted` if all detected actions are allowed.
pub fn check_spatial_constraints(
    player_input: &str,
    zone: NarrativeDistanceZone,
) -> ActionPossibility {
    let lower = player_input.to_lowercase();

    // Check touch keywords against zone affordance.
    if !zone.can_touch() {
        for keyword in TOUCH_KEYWORDS {
            if lower.contains(keyword) {
                return ActionPossibility::Impossible {
                    reason: ConstraintViolation {
                        constraint_name: "spatial:touch".to_string(),
                        description: format!(
                            "Physical contact ('{keyword}') is not possible at {zone:?} distance"
                        ),
                    },
                };
            }
        }
    }

    // Check speech keywords against zone affordance.
    if !zone.can_hear_speech() {
        for keyword in SPEECH_KEYWORDS {
            if lower.contains(keyword) {
                return ActionPossibility::Impossible {
                    reason: ConstraintViolation {
                        constraint_name: "spatial:speech".to_string(),
                        description: format!(
                            "Speech ('{keyword}') cannot be heard at {zone:?} distance"
                        ),
                    },
                };
            }
        }
    }

    ActionPossibility::Permitted {
        conditions: Vec::new(),
    }
}

/// Orchestrate all action possibility checks in priority order.
///
/// Runs genre constraints first, then spatial constraints (if a zone is
/// provided). Returns the first `Impossible` found. If any check returns
/// `Ambiguous`, returns that. Returns `Permitted` only if all checks pass.
pub fn check_action_possibility(
    player_input: &str,
    genre_constraints: &[GenreConstraint],
    lexicon: &CapabilityLexicon,
    actor_zone: Option<NarrativeDistanceZone>,
) -> ActionPossibility {
    // 1. Genre constraints (highest priority — world-level rules).
    let genre_result = check_genre_constraints(player_input, genre_constraints, lexicon);
    if genre_result.is_impossible() {
        return genre_result;
    }

    // 2. Spatial constraints (if zone provided).
    let spatial_result = actor_zone.map(|zone| check_spatial_constraints(player_input, zone));

    if let Some(ref result) = spatial_result {
        if result.is_impossible() {
            return spatial_result.expect("checked Some above");
        }
    }

    // 3. Return Ambiguous if genre check was ambiguous.
    if genre_result.is_ambiguous() {
        return genre_result;
    }

    // All checks passed.
    ActionPossibility::Permitted {
        conditions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::capability_lexicon::*;

    fn swordsmanship_lexicon() -> CapabilityLexicon {
        let mut l = CapabilityLexicon::new();
        l.add(LexiconEntry {
            capability: "swordsmanship".to_string(),
            synonyms: vec!["fencing".to_string()],
            action_verbs: vec![
                "slash".to_string(),
                "parry".to_string(),
                "thrust".to_string(),
            ],
            implied_objects: vec![
                "rapier".to_string(),
                "sword".to_string(),
                "blade".to_string(),
            ],
            idiomatic_phrases: vec!["crossed swords".to_string()],
        });
        l
    }

    fn telekinesis_forbidden() -> Vec<GenreConstraint> {
        vec![GenreConstraint::Forbidden {
            capability: "telekinesis".to_string(),
            reason: "Magic does not exist in this world".to_string(),
        }]
    }

    fn telekinesis_lexicon() -> CapabilityLexicon {
        let mut l = CapabilityLexicon::new();
        l.add(LexiconEntry {
            capability: "telekinesis".to_string(),
            synonyms: vec!["telekinesis".to_string()],
            action_verbs: vec!["levitate".to_string(), "move with mind".to_string()],
            implied_objects: vec![],
            idiomatic_phrases: vec![],
        });
        l
    }

    #[test]
    fn forbidden_capability_returns_impossible() {
        let constraints = telekinesis_forbidden();
        let lexicon = telekinesis_lexicon();
        let result =
            check_genre_constraints("I levitate the stone with my mind", &constraints, &lexicon);
        assert!(result.is_impossible());
    }

    #[test]
    fn permitted_action_passes_genre_check() {
        let constraints = telekinesis_forbidden();
        let lexicon = telekinesis_lexicon();
        let result = check_genre_constraints("I pick up the stone", &constraints, &lexicon);
        assert!(result.is_permitted());
    }

    #[test]
    fn conditional_constraint_returns_ambiguous() {
        let constraints = vec![GenreConstraint::Conditional {
            capability: "swordsmanship".to_string(),
            requires: vec!["weapon equipped".to_string()],
        }];
        let lexicon = swordsmanship_lexicon();
        let result = check_genre_constraints(
            "I slash at the guard with my rapier",
            &constraints,
            &lexicon,
        );
        assert!(result.is_ambiguous());
    }

    #[test]
    fn spatial_zone_blocks_distant_touch() {
        let result = check_spatial_constraints(
            "I reach out and touch her hand",
            NarrativeDistanceZone::Peripheral,
        );
        assert!(result.is_impossible());
    }

    #[test]
    fn spatial_zone_allows_intimate_touch() {
        let result = check_spatial_constraints(
            "I reach out and touch her hand",
            NarrativeDistanceZone::Intimate,
        );
        assert!(result.is_permitted());
    }

    #[test]
    fn spatial_zone_blocks_distant_whisper() {
        let result = check_spatial_constraints(
            "I whisper a secret to her",
            NarrativeDistanceZone::Peripheral,
        );
        assert!(result.is_impossible());
    }

    #[test]
    fn full_check_returns_first_impossible() {
        let constraints = telekinesis_forbidden();
        let lexicon = telekinesis_lexicon();
        let result = check_action_possibility(
            "I levitate the stone",
            &constraints,
            &lexicon,
            Some(NarrativeDistanceZone::Conversational),
        );
        assert!(result.is_impossible());
    }

    #[test]
    fn full_check_permits_normal_action() {
        let constraints = telekinesis_forbidden();
        let lexicon = telekinesis_lexicon();
        let result = check_action_possibility(
            "I walk through the meadow",
            &constraints,
            &lexicon,
            Some(NarrativeDistanceZone::Conversational),
        );
        assert!(result.is_permitted());
    }

    #[test]
    fn full_check_without_zone_skips_spatial() {
        let result =
            check_action_possibility("I touch her hand", &[], &CapabilityLexicon::new(), None);
        assert!(result.is_permitted());
    }
}
