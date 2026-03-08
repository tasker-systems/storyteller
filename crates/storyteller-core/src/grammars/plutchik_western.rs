//! Plutchik-derived Western emotional grammar — 8 primaries with oppositions.
//!
//! See: `docs/foundation/emotional-model.md` § Plutchik's Wheel of Emotions
//!
//! Primaries: joy, sadness, trust, disgust, fear, anger, surprise, anticipation.
//! Four opposition pairs. Intensity gradients from low (e.g. serenity) to
//! high (e.g. ecstasy).

use crate::traits::emotional_grammar::{EmotionalGrammar, PrimaryDef};
use crate::types::character::EmotionalState;

/// The Plutchik-derived Western emotional grammar.
///
/// Eight primary emotions organized in four opposition pairs, each with
/// a three-level intensity gradient. This is the default grammar and
/// the first one implemented.
///
/// Use the associated constants (`GRAMMAR_ID`, `JOY`, `SADNESS`, etc.)
/// instead of raw strings when constructing emotional states or referencing
/// primaries. This provides compile-time typo protection.
#[derive(Debug)]
pub struct PlutchikWestern {
    primaries: Vec<PrimaryDef>,
}

// --- Vocabulary constants ---
// Use these instead of raw strings for compile-time safety.

impl PlutchikWestern {
    /// Grammar identifier.
    pub const GRAMMAR_ID: &str = "plutchik_western";

    // Opposition pair 1: Joy ↔ Sadness
    /// Primary: joy (serenity → joy → ecstasy).
    pub const JOY: &str = "joy";
    /// Primary: sadness (pensiveness → sadness → grief).
    pub const SADNESS: &str = "sadness";

    // Opposition pair 2: Trust ↔ Disgust
    /// Primary: trust (acceptance → trust → admiration).
    pub const TRUST: &str = "trust";
    /// Primary: disgust (boredom → disgust → loathing).
    pub const DISGUST: &str = "disgust";

    // Opposition pair 3: Fear ↔ Anger
    /// Primary: fear (apprehension → fear → terror).
    pub const FEAR: &str = "fear";
    /// Primary: anger (annoyance → anger → rage).
    pub const ANGER: &str = "anger";

    // Opposition pair 4: Surprise ↔ Anticipation
    /// Primary: surprise (distraction → surprise → amazement).
    pub const SURPRISE: &str = "surprise";
    /// Primary: anticipation (interest → anticipation → vigilance).
    pub const ANTICIPATION: &str = "anticipation";
}

impl PlutchikWestern {
    /// Construct the grammar with all eight primary definitions.
    pub fn new() -> Self {
        Self {
            primaries: vec![
                PrimaryDef {
                    id: Self::JOY.to_string(),
                    name: "Joy".to_string(),
                    opposite_id: Some(Self::SADNESS.to_string()),
                    low_intensity_label: "serenity".to_string(),
                    high_intensity_label: "ecstasy".to_string(),
                },
                PrimaryDef {
                    id: Self::SADNESS.to_string(),
                    name: "Sadness".to_string(),
                    opposite_id: Some(Self::JOY.to_string()),
                    low_intensity_label: "pensiveness".to_string(),
                    high_intensity_label: "grief".to_string(),
                },
                PrimaryDef {
                    id: Self::TRUST.to_string(),
                    name: "Trust".to_string(),
                    opposite_id: Some(Self::DISGUST.to_string()),
                    low_intensity_label: "acceptance".to_string(),
                    high_intensity_label: "admiration".to_string(),
                },
                PrimaryDef {
                    id: Self::DISGUST.to_string(),
                    name: "Disgust".to_string(),
                    opposite_id: Some(Self::TRUST.to_string()),
                    low_intensity_label: "boredom".to_string(),
                    high_intensity_label: "loathing".to_string(),
                },
                PrimaryDef {
                    id: Self::FEAR.to_string(),
                    name: "Fear".to_string(),
                    opposite_id: Some(Self::ANGER.to_string()),
                    low_intensity_label: "apprehension".to_string(),
                    high_intensity_label: "terror".to_string(),
                },
                PrimaryDef {
                    id: Self::ANGER.to_string(),
                    name: "Anger".to_string(),
                    opposite_id: Some(Self::FEAR.to_string()),
                    low_intensity_label: "annoyance".to_string(),
                    high_intensity_label: "rage".to_string(),
                },
                PrimaryDef {
                    id: Self::SURPRISE.to_string(),
                    name: "Surprise".to_string(),
                    opposite_id: Some(Self::ANTICIPATION.to_string()),
                    low_intensity_label: "distraction".to_string(),
                    high_intensity_label: "amazement".to_string(),
                },
                PrimaryDef {
                    id: Self::ANTICIPATION.to_string(),
                    name: "Anticipation".to_string(),
                    opposite_id: Some(Self::SURPRISE.to_string()),
                    low_intensity_label: "interest".to_string(),
                    high_intensity_label: "vigilance".to_string(),
                },
            ],
        }
    }
}

impl Default for PlutchikWestern {
    fn default() -> Self {
        Self::new()
    }
}

impl EmotionalGrammar for PlutchikWestern {
    fn id(&self) -> &str {
        Self::GRAMMAR_ID
    }

    fn name(&self) -> &str {
        "Plutchik-derived Western"
    }

    fn primaries(&self) -> &[PrimaryDef] {
        &self.primaries
    }

    fn intensity_range(&self) -> (f32, f32) {
        (0.0, 1.0)
    }

    fn validate_state(&self, state: &EmotionalState) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Grammar ID must match
        if state.grammar_id != self.id() {
            errors.push(format!(
                "grammar_id mismatch: expected '{}', got '{}'",
                self.id(),
                state.grammar_id
            ));
        }

        // All 8 primaries should be present
        let expected_ids: Vec<&str> = self.primaries.iter().map(|p| p.id.as_str()).collect();
        let present_ids: Vec<&str> = state
            .primaries
            .iter()
            .map(|p| p.primary_id.as_str())
            .collect();

        for expected in &expected_ids {
            if !present_ids.contains(expected) {
                errors.push(format!("missing primary: '{expected}'"));
            }
        }

        // No unknown primaries
        for present in &present_ids {
            if !expected_ids.contains(present) {
                errors.push(format!("unknown primary: '{present}'"));
            }
        }

        // Intensities in range
        let (lo, hi) = self.intensity_range();
        for primary in &state.primaries {
            if primary.intensity < lo || primary.intensity > hi {
                errors.push(format!(
                    "primary '{}' intensity {} out of range [{}, {}]",
                    primary.primary_id, primary.intensity, lo, hi
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::character::EmotionalPrimary;
    use crate::types::tensor::AwarenessLevel;

    #[test]
    fn grammar_has_eight_primaries() {
        let grammar = PlutchikWestern::new();
        assert_eq!(grammar.primaries().len(), 8);
    }

    #[test]
    fn all_primaries_have_opposites() {
        let grammar = PlutchikWestern::new();
        for primary in grammar.primaries() {
            assert!(
                primary.opposite_id.is_some(),
                "primary '{}' should have an opposite",
                primary.id
            );
        }
    }

    #[test]
    fn oppositions_are_symmetric() {
        let grammar = PlutchikWestern::new();
        for primary in grammar.primaries() {
            let opposite_id = primary.opposite_id.as_ref().unwrap();
            let opposite = grammar
                .primaries()
                .iter()
                .find(|p| p.id == *opposite_id)
                .unwrap_or_else(|| {
                    panic!("opposite '{}' not found for '{}'", opposite_id, primary.id)
                });
            assert_eq!(
                opposite.opposite_id.as_ref().unwrap(),
                &primary.id,
                "opposition between '{}' and '{}' is not symmetric",
                primary.id,
                opposite_id
            );
        }
    }

    #[test]
    fn valid_state_passes_validation() {
        let grammar = PlutchikWestern::new();
        let state = make_valid_state();
        assert!(grammar.validate_state(&state).is_ok());
    }

    #[test]
    fn missing_primary_fails_validation() {
        let grammar = PlutchikWestern::new();
        let mut state = make_valid_state();
        state
            .primaries
            .retain(|p| p.primary_id != PlutchikWestern::JOY);
        let result = grammar.validate_state(&state);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.contains(&format!("missing primary: '{}'", PlutchikWestern::JOY))));
    }

    #[test]
    fn unknown_primary_fails_validation() {
        let grammar = PlutchikWestern::new();
        let mut state = make_valid_state();
        state.primaries.push(EmotionalPrimary {
            primary_id: "schadenfreude".to_string(),
            intensity: 0.5,
            awareness: AwarenessLevel::Articulate,
        });
        let result = grammar.validate_state(&state);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.contains("unknown primary: 'schadenfreude'")));
    }

    #[test]
    fn out_of_range_intensity_fails_validation() {
        let grammar = PlutchikWestern::new();
        let mut state = make_valid_state();
        state.primaries[0].intensity = 1.5;
        let result = grammar.validate_state(&state);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("out of range")));
    }

    #[test]
    fn wrong_grammar_id_fails_validation() {
        let grammar = PlutchikWestern::new();
        let mut state = make_valid_state();
        state.grammar_id = "wu_xing".to_string();
        let result = grammar.validate_state(&state);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("grammar_id mismatch")));
    }

    fn make_valid_state() -> EmotionalState {
        EmotionalState {
            grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
            primaries: vec![
                EmotionalPrimary {
                    primary_id: PlutchikWestern::JOY.to_string(),
                    intensity: 0.4,
                    awareness: AwarenessLevel::Articulate,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::SADNESS.to_string(),
                    intensity: 0.5,
                    awareness: AwarenessLevel::Recognizable,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::TRUST.to_string(),
                    intensity: 0.6,
                    awareness: AwarenessLevel::Preconscious,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::DISGUST.to_string(),
                    intensity: 0.3,
                    awareness: AwarenessLevel::Recognizable,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::FEAR.to_string(),
                    intensity: 0.2,
                    awareness: AwarenessLevel::Preconscious,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::ANGER.to_string(),
                    intensity: 0.3,
                    awareness: AwarenessLevel::Recognizable,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::SURPRISE.to_string(),
                    intensity: 0.3,
                    awareness: AwarenessLevel::Articulate,
                },
                EmotionalPrimary {
                    primary_id: PlutchikWestern::ANTICIPATION.to_string(),
                    intensity: 0.6,
                    awareness: AwarenessLevel::Articulate,
                },
            ],
            mood_vector_notes: vec![],
        }
    }
}
