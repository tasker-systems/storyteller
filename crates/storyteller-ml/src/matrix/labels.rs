//! Heuristic label generation — produces ground-truth predictions from scenario skeletons.
//!
//! This is the core algorithmic challenge: generating plausible training labels
//! from archetype action tendencies, emotional profiles, relational dynamics,
//! and scene context. No LLM involved — pure heuristic.

use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use rand::Rng;

use storyteller_core::types::prediction::{
    ActionContext, ActionType, EmotionalRegister, EventType, RawActionPrediction,
    RawActivatedTensorFrame, RawCharacterPrediction, RawEmotionalDelta, RawSpeechPrediction,
    RawThoughtPrediction, SpeechRegister,
};
use storyteller_core::types::tensor::AwarenessLevel;

use super::combinator::ScenarioSkeleton;

/// Generate heuristic labels for a scenario skeleton.
pub fn generate_labels(skeleton: &ScenarioSkeleton, rng: &mut impl Rng) -> RawCharacterPrediction {
    // Extract archetype action tendencies from the character sheet's performance_notes
    // We need the archetype descriptor, but we only have the skeleton. Parse from known data.
    let action_type = select_action_type(skeleton, rng);
    let action_context = select_action_context(skeleton, rng);
    let emotional_valence = compute_emotional_valence(skeleton, rng);
    let speech = generate_speech(skeleton, rng);
    let thought = generate_thought(skeleton, rng);
    let emotional_deltas = generate_emotional_deltas(skeleton, rng);

    // Activated axes: pick the first N axes from the character's tensor
    let activated_axis_indices: Vec<u16> = (0..skeleton.character_a.tensor.axes.len().min(8))
        .map(|i| i as u16)
        .collect();

    RawCharacterPrediction {
        character_id: skeleton.character_a.entity_id,
        frame: RawActivatedTensorFrame {
            activated_axis_indices,
            confidence: rng.random_range(0.6..=0.9),
        },
        action: RawActionPrediction {
            action_type,
            confidence: rng.random_range(0.5..=0.9),
            target: Some(skeleton.character_b.entity_id),
            emotional_valence,
            action_context,
        },
        speech,
        thought,
        emotional_deltas,
    }
}

fn select_action_type(skeleton: &ScenarioSkeleton, rng: &mut impl Rng) -> ActionType {
    // Base weights from scene event type resonance
    let mut weights = [1.0f32; 6]; // Perform, Speak, Move, Examine, Wait, Resist

    // Boost based on event type
    match skeleton.event.event_type {
        EventType::Speech => weights[1] += 2.0,      // Speak
        EventType::Action => weights[0] += 2.0,      // Perform
        EventType::Movement => weights[2] += 2.0,    // Move
        EventType::Observation => weights[3] += 2.0, // Examine
        EventType::Interaction => weights[0] += 1.0, // Perform
        EventType::Emote => weights[4] += 1.0,       // Wait (internal)
        EventType::Inquiry => weights[3] += 1.5,     // Examine
    }

    // Boost based on emotional register
    match skeleton.event.emotional_register {
        EmotionalRegister::Aggressive => {
            weights[5] += 1.5; // Resist
            weights[1] += 1.0; // Speak
        }
        EmotionalRegister::Vulnerable => {
            weights[1] += 1.0; // Speak
            weights[4] += 0.5; // Wait
        }
        EmotionalRegister::Tender => {
            weights[1] += 1.5; // Speak
            weights[0] += 0.5; // Perform (gesture)
        }
        EmotionalRegister::Guarded => {
            weights[4] += 1.0; // Wait
            weights[3] += 1.0; // Examine
        }
        EmotionalRegister::Inquisitive => {
            weights[3] += 1.5; // Examine
            weights[1] += 1.0; // Speak
        }
        _ => {}
    }

    // Tension modifiers
    if skeleton.scene.tension > 0.7 {
        weights[5] += 1.0; // Resist
        weights[2] += 0.5; // Move
    } else if skeleton.scene.tension < 0.3 {
        weights[4] += 1.0; // Wait
        weights[3] += 0.5; // Examine
    }

    // Empathy boost from character tensor
    if let Some(emp) = skeleton.character_a.tensor.get("empathy") {
        if emp.value.central_tendency > 0.6 {
            weights[1] += 1.0; // Speak
        }
    }
    if let Some(dist) = skeleton.character_a.tensor.get("distance_management") {
        if dist.value.central_tendency > 0.6 {
            weights[4] += 1.0; // Wait
        }
    }

    weighted_select(
        &weights,
        &[
            ActionType::Perform,
            ActionType::Speak,
            ActionType::Move,
            ActionType::Examine,
            ActionType::Wait,
            ActionType::Resist,
        ],
        rng,
    )
}

fn select_action_context(skeleton: &ScenarioSkeleton, rng: &mut impl Rng) -> ActionContext {
    let mut weights = [1.0f32; 5];

    // SharedHistory, CurrentScene, EmotionalReaction, RelationalDynamic, WorldResponse
    match skeleton.event.emotional_register {
        EmotionalRegister::Aggressive => weights[2] += 2.0,
        EmotionalRegister::Vulnerable => weights[2] += 1.5,
        EmotionalRegister::Tender => weights[3] += 1.5,
        EmotionalRegister::Inquisitive => weights[1] += 1.5,
        _ => weights[1] += 1.0,
    }

    // High trust = SharedHistory more likely
    if skeleton
        .edge_a_to_b
        .substrate
        .trust_benevolence
        .central_tendency
        > 0.5
    {
        weights[0] += 1.0;
    }

    weighted_select(
        &weights,
        &[
            ActionContext::SharedHistory,
            ActionContext::CurrentScene,
            ActionContext::EmotionalReaction,
            ActionContext::RelationalDynamic,
            ActionContext::WorldResponse,
        ],
        rng,
    )
}

fn compute_emotional_valence(skeleton: &ScenarioSkeleton, rng: &mut impl Rng) -> f32 {
    let base = match skeleton.event.emotional_register {
        EmotionalRegister::Aggressive => -0.3,
        EmotionalRegister::Vulnerable => 0.1,
        EmotionalRegister::Playful => 0.3,
        EmotionalRegister::Guarded => -0.1,
        EmotionalRegister::Neutral => 0.0,
        EmotionalRegister::Tender => 0.4,
        EmotionalRegister::Inquisitive => 0.1,
    };

    // Modify based on character warmth
    let warmth_mod = skeleton
        .character_a
        .tensor
        .get("warmth_openness")
        .map(|e| e.value.central_tendency * 0.2)
        .unwrap_or(0.0);

    let noise: f32 = rng.random_range(-0.1..=0.1);
    (base + warmth_mod + noise).clamp(-1.0, 1.0)
}

fn generate_speech(skeleton: &ScenarioSkeleton, rng: &mut impl Rng) -> RawSpeechPrediction {
    // Base speech likelihood: depends on event type and character traits
    let base_likelihood = match skeleton.event.event_type {
        EventType::Speech => 0.8,
        EventType::Emote => 0.3,
        EventType::Observation => 0.4,
        _ => 0.5,
    };

    // Modify by expression/silence axis
    let expression_mod = skeleton
        .character_a
        .tensor
        .get("expression_silence")
        .map(|e| e.value.central_tendency * 0.2)
        .unwrap_or(0.0);

    let occurs = rng.random_range(0.0..=1.0) < (base_likelihood + expression_mod);

    let register = if skeleton.scene.tension > 0.7 {
        if rng.random_bool(0.4) {
            SpeechRegister::Declamatory
        } else {
            SpeechRegister::Conversational
        }
    } else if skeleton.scene.tension < 0.3 {
        if rng.random_bool(0.4) {
            SpeechRegister::Whisper
        } else {
            SpeechRegister::Conversational
        }
    } else {
        SpeechRegister::Conversational
    };

    RawSpeechPrediction {
        occurs,
        register,
        confidence: rng.random_range(0.5..=0.9),
    }
}

fn generate_thought(skeleton: &ScenarioSkeleton, _rng: &mut impl Rng) -> RawThoughtPrediction {
    // Awareness from self_awareness axis
    let self_awareness = skeleton
        .character_a
        .tensor
        .get("self_awareness")
        .map(|e| e.value.central_tendency)
        .unwrap_or(0.5);

    let awareness_level = if self_awareness > 0.7 {
        AwarenessLevel::Articulate
    } else if self_awareness > 0.5 {
        AwarenessLevel::Recognizable
    } else if self_awareness > 0.3 {
        AwarenessLevel::Preconscious
    } else {
        AwarenessLevel::Defended
    };

    // Dominant emotion = highest intensity primary
    let dominant_emotion_index = skeleton
        .character_a
        .emotional_state
        .primaries
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.intensity
                .partial_cmp(&b.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i as u8)
        .unwrap_or(0);

    RawThoughtPrediction {
        awareness_level,
        dominant_emotion_index,
    }
}

fn generate_emotional_deltas(
    skeleton: &ScenarioSkeleton,
    rng: &mut impl Rng,
) -> Vec<RawEmotionalDelta> {
    let mut deltas = Vec::new();

    // Player register drives delta direction
    let (boost_indices, suppress_indices) = match skeleton.event.emotional_register {
        EmotionalRegister::Aggressive => (vec![6u8], vec![1u8]), // anger+, trust-
        EmotionalRegister::Vulnerable => (vec![1u8], vec![2u8]), // trust+, fear-
        EmotionalRegister::Tender => (vec![0u8, 1u8], vec![]),   // joy+, trust+
        EmotionalRegister::Playful => (vec![0u8, 3u8], vec![]),  // joy+, surprise+
        EmotionalRegister::Guarded => (vec![2u8], vec![0u8]),    // fear+, joy-
        EmotionalRegister::Inquisitive => (vec![3u8, 7u8], vec![]), // surprise+, anticipation+
        EmotionalRegister::Neutral => (vec![], vec![]),
    };

    for idx in boost_indices {
        let magnitude = rng.random_range(0.05..=0.25);
        let awareness_shifts = magnitude > 0.15 && rng.random_bool(0.2);
        deltas.push(RawEmotionalDelta {
            primary_index: idx,
            intensity_change: magnitude,
            awareness_shifts,
        });
    }

    for idx in suppress_indices {
        let magnitude = rng.random_range(0.05..=0.15);
        deltas.push(RawEmotionalDelta {
            primary_index: idx,
            intensity_change: -magnitude,
            awareness_shifts: false,
        });
    }

    deltas
}

fn weighted_select<T: Copy>(weights: &[f32], items: &[T], rng: &mut impl Rng) -> T {
    if let Ok(dist) = WeightedIndex::new(weights) {
        items[dist.sample(rng)]
    } else {
        items[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_select_returns_valid_item() {
        let weights = [1.0, 2.0, 3.0];
        let items = [10, 20, 30];
        let mut rng = rand::rng();
        for _ in 0..100 {
            let result = weighted_select(&weights, &items, &mut rng);
            assert!([10, 20, 30].contains(&result));
        }
    }
}
