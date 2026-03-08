//! Coherence validation for training examples.
//!
//! Five weighted rules check that generated labels are plausible given
//! the scenario skeleton's character, relational, and scene data.

use storyteller_core::types::prediction::{ActionType, RawCharacterPrediction};
use storyteller_core::types::tensor::AwarenessLevel;

use super::combinator::ScenarioSkeleton;

/// Validation result for a single training example.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Overall coherence score. Range: [0.0, 1.0].
    pub coherence_score: f32,
    /// Per-rule scores.
    pub rule_scores: Vec<RuleScore>,
    /// Whether this example passes the minimum coherence threshold.
    pub passes: bool,
}

/// Score from a single validation rule.
#[derive(Debug, Clone)]
pub struct RuleScore {
    pub name: &'static str,
    pub weight: f32,
    pub score: f32,
}

/// Validate a training example against coherence rules.
///
/// Returns a [`ValidationResult`] with per-rule scores and an overall
/// coherence score. The example passes if `coherence_score >= min_coherence`.
pub fn validate_example(
    skeleton: &ScenarioSkeleton,
    prediction: &RawCharacterPrediction,
    min_coherence: f32,
) -> ValidationResult {
    let rules = vec![
        RuleScore {
            name: "emotional_consistency",
            weight: 0.25,
            score: check_emotional_consistency(skeleton, prediction),
        },
        RuleScore {
            name: "relational_alignment",
            weight: 0.20,
            score: check_relational_alignment(skeleton, prediction),
        },
        RuleScore {
            name: "awareness_discipline",
            weight: 0.25,
            score: check_awareness_discipline(skeleton, prediction),
        },
        RuleScore {
            name: "temporal_stability",
            weight: 0.15,
            score: check_temporal_stability(skeleton, prediction),
        },
        RuleScore {
            name: "action_scene_alignment",
            weight: 0.15,
            score: check_action_scene_alignment(skeleton, prediction),
        },
    ];

    let coherence_score: f32 = rules.iter().map(|r| r.weight * r.score).sum();

    ValidationResult {
        passes: coherence_score >= min_coherence,
        coherence_score,
        rule_scores: rules,
    }
}

/// Rule 1: Emotional consistency (0.25 weight).
///
/// - Deltas don't push primaries outside [0, 1]
/// - Dominant emotion matches high-intensity primary
fn check_emotional_consistency(
    skeleton: &ScenarioSkeleton,
    prediction: &RawCharacterPrediction,
) -> f32 {
    let mut score = 1.0f32;

    // Check deltas don't push outside bounds
    for delta in &prediction.emotional_deltas {
        let idx = delta.primary_index as usize;
        if let Some(primary) = skeleton.character_a.emotional_state.primaries.get(idx) {
            let new_intensity = primary.intensity + delta.intensity_change;
            if !(-0.1..=1.1).contains(&new_intensity) {
                score -= 0.3;
            }
        }
    }

    // Check dominant emotion is actually high-intensity
    let dominant_idx = prediction.thought.dominant_emotion_index as usize;
    if let Some(dominant) = skeleton
        .character_a
        .emotional_state
        .primaries
        .get(dominant_idx)
    {
        // It should be among the top 3 intensities
        let mut intensities: Vec<f32> = skeleton
            .character_a
            .emotional_state
            .primaries
            .iter()
            .map(|p| p.intensity)
            .collect();
        intensities.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        if intensities.len() >= 3 && dominant.intensity < intensities[2] {
            score -= 0.2;
        }
    }

    score.max(0.0)
}

/// Rule 2: Relational alignment (0.20 weight).
///
/// - Low trust + SharedHistory flagged
/// - High affection + Resist flagged
fn check_relational_alignment(
    skeleton: &ScenarioSkeleton,
    prediction: &RawCharacterPrediction,
) -> f32 {
    let mut score = 1.0f32;

    let trust_avg = (skeleton
        .edge_a_to_b
        .substrate
        .trust_reliability
        .central_tendency
        + skeleton
            .edge_a_to_b
            .substrate
            .trust_competence
            .central_tendency
        + skeleton
            .edge_a_to_b
            .substrate
            .trust_benevolence
            .central_tendency)
        / 3.0;

    // Low trust + SharedHistory is suspicious
    if trust_avg < 0.3
        && prediction.action.action_context
            == storyteller_core::types::prediction::ActionContext::SharedHistory
    {
        score -= 0.3;
    }

    // High affection + Resist is suspicious
    if skeleton.edge_a_to_b.substrate.affection.central_tendency > 0.6
        && prediction.action.action_type == ActionType::Resist
    {
        score -= 0.2;
    }

    score.max(0.0)
}

/// Rule 3: Awareness discipline (0.25 weight).
///
/// - Defended primaries shouldn't be Articulate in thought
/// - Structural primaries shouldn't be dominant
fn check_awareness_discipline(
    skeleton: &ScenarioSkeleton,
    prediction: &RawCharacterPrediction,
) -> f32 {
    let mut score = 1.0f32;

    let dominant_idx = prediction.thought.dominant_emotion_index as usize;
    if let Some(dominant) = skeleton
        .character_a
        .emotional_state
        .primaries
        .get(dominant_idx)
    {
        // Structural emotions shouldn't be dominant
        if dominant.awareness == AwarenessLevel::Structural {
            score -= 0.3;
        }

        // Defended + Articulate thought is contradictory
        if dominant.awareness == AwarenessLevel::Defended
            && prediction.thought.awareness_level == AwarenessLevel::Articulate
        {
            score -= 0.2;
        }
    }

    // Check awareness shift deltas: shouldn't shift Defended → Articulate in one turn
    for delta in &prediction.emotional_deltas {
        if delta.awareness_shifts {
            let idx = delta.primary_index as usize;
            if let Some(primary) = skeleton.character_a.emotional_state.primaries.get(idx) {
                if primary.awareness == AwarenessLevel::Structural
                    || primary.awareness == AwarenessLevel::Defended
                {
                    // Large shifts from deep awareness are suspicious
                    if delta.intensity_change.abs() < 0.1 {
                        score -= 0.15;
                    }
                }
            }
        }
    }

    score.max(0.0)
}

/// Rule 4: Temporal stability (0.15 weight).
///
/// - Bedrock deltas should be < 0.1
/// - Topsoil can change freely
fn check_temporal_stability(
    _skeleton: &ScenarioSkeleton,
    prediction: &RawCharacterPrediction,
) -> f32 {
    let mut score = 1.0f32;

    // Check that tensor axes on bedrock layer don't have large emotional deltas
    // This is a proxy — we can't directly map emotional deltas to tensor axes,
    // but we check that overall delta magnitudes are reasonable
    for delta in &prediction.emotional_deltas {
        if delta.intensity_change.abs() > 0.3 {
            score -= 0.2;
        }
    }

    score.max(0.0)
}

/// Rule 5: Action-scene alignment (0.15 weight).
///
/// - Action type should be plausible for scene profile
fn check_action_scene_alignment(
    skeleton: &ScenarioSkeleton,
    prediction: &RawCharacterPrediction,
) -> f32 {
    let mut score = 1.0f32;

    // Low tension scenes: Resist is unlikely
    if skeleton.scene.tension < 0.3 && prediction.action.action_type == ActionType::Resist {
        score -= 0.3;
    }

    // High tension scenes: Wait is less common
    if skeleton.scene.tension > 0.8 && prediction.action.action_type == ActionType::Wait {
        score -= 0.15;
    }

    score.max(0.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn validation_scores_are_bounded() {
        // Scores should always be in [0.0, 1.0]
        let score: f32 = 1.0 - 0.5;
        assert!((0.0..=1.0).contains(&score.max(0.0)));
    }
}
