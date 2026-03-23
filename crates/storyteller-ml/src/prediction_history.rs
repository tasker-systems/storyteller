// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Prediction history — ring buffer of recent turn outcomes per character.
//!
//! Captures what the ML model predicted each turn so subsequent predictions
//! can reference recent behavior. Feeds Region 7 of the feature vector.
//!
//! See: `docs/plans/2026-03-11-prediction-history-hydration-design.md`

use std::collections::HashMap;

use storyteller_core::types::entity::EntityId;
use storyteller_core::types::prediction::{ActionType, CharacterPrediction, SpeechRegister};

use crate::feature_schema::{HistoryEntry, HISTORY_DEPTH};

/// Per-character ring buffer of recent prediction outcomes.
///
/// Keyed by `EntityId`. Each character's buffer holds at most [`HISTORY_DEPTH`]
/// entries in most-recent-first order. Uses `Vec` with `insert(0, ..)` instead
/// of `VecDeque` — with max 3 entries the shift cost is negligible and we get
/// a contiguous `&[HistoryEntry]` slice from `.as_slice()`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PredictionHistory {
    entries: HashMap<EntityId, Vec<HistoryEntry>>,
}

/// Compute emotional valence from emotional deltas.
///
/// Sums all `intensity_change` values and clamps to \[-1.0, 1.0\].
/// Returns 0.0 for empty deltas.
pub fn compute_emotional_valence(
    deltas: &[storyteller_core::types::prediction::EmotionalDelta],
) -> f32 {
    deltas
        .iter()
        .map(|d| d.intensity_change)
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}

impl HistoryEntry {
    /// Extract a history entry from an enriched character prediction.
    pub fn from_prediction(pred: &CharacterPrediction) -> Self {
        let action_type = pred
            .actions
            .first()
            .map(|a| a.action_type)
            .unwrap_or(ActionType::Examine);

        let speech_register = pred
            .speech
            .as_ref()
            .map(|s| s.register)
            .unwrap_or(SpeechRegister::Conversational);

        Self {
            action_type,
            speech_register,
            awareness_level: pred.thought.awareness_level,
            emotional_valence: compute_emotional_valence(&pred.emotional_deltas),
        }
    }
}

impl PredictionHistory {
    /// Push a new prediction into the ring buffer for the character.
    pub fn push_from_prediction(&mut self, prediction: &CharacterPrediction) {
        let entry = HistoryEntry::from_prediction(prediction);
        let buf = self.entries.entry(prediction.character_id).or_default();
        buf.insert(0, entry);
        buf.truncate(HISTORY_DEPTH);
    }

    /// Get the recent history for a character, most-recent-first.
    pub fn get(&self, character_id: EntityId) -> &[HistoryEntry] {
        match self.entries.get(&character_id) {
            Some(buf) => buf.as_slice(),
            None => &[],
        }
    }

    /// Access the full internal map (for encoding all characters at once).
    pub fn as_map(&self) -> &HashMap<EntityId, Vec<HistoryEntry>> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::prediction::{
        ActionPrediction, ActivatedTensorFrame, EmotionalDelta, SpeechPrediction, ThoughtPrediction,
    };
    use storyteller_core::types::tensor::AwarenessLevel;

    fn mock_prediction(entity_id: EntityId) -> CharacterPrediction {
        CharacterPrediction {
            character_id: entity_id,
            character_name: "TestChar".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec!["trust".to_string()],
                confidence: 0.8,
            },
            actions: vec![ActionPrediction {
                action_type: ActionType::Speak,
                description: "speaks softly".to_string(),
                confidence: 0.9,
                target: None,
            }],
            speech: Some(SpeechPrediction {
                content_direction: "gentle comfort".to_string(),
                register: SpeechRegister::Whisper,
                confidence: 0.8,
            }),
            thought: ThoughtPrediction {
                emotional_subtext: "warmth beneath guardedness".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: None,
            },
            emotional_deltas: vec![
                EmotionalDelta {
                    primary_id: "joy".to_string(),
                    intensity_change: 0.3,
                    awareness_change: None,
                },
                EmotionalDelta {
                    primary_id: "sadness".to_string(),
                    intensity_change: -0.1,
                    awareness_change: None,
                },
            ],
        }
    }

    #[test]
    fn history_entry_from_prediction_extracts_fields() {
        let id = EntityId::new();
        let pred = mock_prediction(id);
        let entry = HistoryEntry::from_prediction(&pred);

        assert_eq!(entry.action_type, ActionType::Speak);
        assert_eq!(entry.speech_register, SpeechRegister::Whisper);
        assert_eq!(entry.awareness_level, AwarenessLevel::Recognizable);
    }

    #[test]
    fn history_entry_defaults_speech_register_when_no_speech() {
        let id = EntityId::new();
        let mut pred = mock_prediction(id);
        pred.speech = None;
        let entry = HistoryEntry::from_prediction(&pred);

        assert_eq!(entry.speech_register, SpeechRegister::Conversational);
    }

    #[test]
    fn history_entry_defaults_action_type_when_no_actions() {
        let id = EntityId::new();
        let mut pred = mock_prediction(id);
        pred.actions.clear();
        let entry = HistoryEntry::from_prediction(&pred);

        assert_eq!(entry.action_type, ActionType::Examine);
    }

    #[test]
    fn emotional_valence_positive_deltas() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.5,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "trust".to_string(),
                intensity_change: 0.3,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn emotional_valence_negative_deltas() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "fear".to_string(),
                intensity_change: -0.4,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "anger".to_string(),
                intensity_change: -0.7,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!(
            (valence - (-1.0)).abs() < f32::EPSILON,
            "should clamp to -1.0"
        );
    }

    #[test]
    fn emotional_valence_mixed_deltas() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.3,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "sadness".to_string(),
                intensity_change: -0.1,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn emotional_valence_empty_deltas() {
        let valence = compute_emotional_valence(&[]);
        assert!(valence.abs() < f32::EPSILON, "empty deltas should be 0.0");
    }

    #[test]
    fn emotional_valence_clamps_to_range() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.8,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "trust".to_string(),
                intensity_change: 0.9,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - 1.0).abs() < f32::EPSILON, "should clamp to 1.0");
    }

    #[test]
    fn ring_buffer_push_and_lookup() {
        let id = EntityId::new();
        let pred = mock_prediction(id);
        let mut history = PredictionHistory::default();

        history.push_from_prediction(&pred);
        let entries = history.get(id);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action_type, ActionType::Speak);
    }

    #[test]
    fn ring_buffer_most_recent_first() {
        let id = EntityId::new();
        let mut history = PredictionHistory::default();

        let mut pred1 = mock_prediction(id);
        pred1.actions[0].action_type = ActionType::Perform;
        history.push_from_prediction(&pred1);

        let mut pred2 = mock_prediction(id);
        pred2.actions[0].action_type = ActionType::Speak;
        history.push_from_prediction(&pred2);

        let entries = history.get(id);
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].action_type,
            ActionType::Speak,
            "most recent first"
        );
        assert_eq!(entries[1].action_type, ActionType::Perform);
    }

    #[test]
    fn ring_buffer_respects_depth_limit() {
        let id = EntityId::new();
        let mut history = PredictionHistory::default();

        for _ in 0..5 {
            history.push_from_prediction(&mock_prediction(id));
        }

        let entries = history.get(id);
        assert_eq!(entries.len(), HISTORY_DEPTH, "should cap at HISTORY_DEPTH");
    }

    #[test]
    fn unknown_character_returns_empty() {
        let history = PredictionHistory::default();
        let entries = history.get(EntityId::new());
        assert!(entries.is_empty());
    }

    #[test]
    fn multiple_characters_independent() {
        let id_a = EntityId::new();
        let id_b = EntityId::new();
        let mut history = PredictionHistory::default();

        let mut pred_a = mock_prediction(id_a);
        pred_a.actions[0].action_type = ActionType::Perform;
        history.push_from_prediction(&pred_a);

        let mut pred_b = mock_prediction(id_b);
        pred_b.actions[0].action_type = ActionType::Examine;
        history.push_from_prediction(&pred_b);

        let a_entries = history.get(id_a);
        let b_entries = history.get(id_b);
        assert_eq!(a_entries[0].action_type, ActionType::Perform);
        assert_eq!(b_entries[0].action_type, ActionType::Examine);
    }
}
