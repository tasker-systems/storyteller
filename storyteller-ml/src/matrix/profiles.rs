//! Scene profile instantiation — descriptor → SceneFeatureInput + EventFeatureInput.

use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use rand::Rng;

use storyteller_core::types::prediction::{EmotionalRegister, EventType};
use storyteller_core::types::scene::SceneType;

use crate::feature_schema::{EventFeatureInput, SceneFeatureInput};

use super::descriptors::ProfileDescriptor;

/// Instantiate scene and event feature inputs from a profile descriptor.
pub fn instantiate_scene_and_event(
    profile: &ProfileDescriptor,
    rng: &mut impl Rng,
) -> (SceneFeatureInput, EventFeatureInput) {
    let scene_type = parse_scene_type(&profile.scene_type);
    let tension = profile.tension.sample(rng).clamp(0.0, 1.0);
    let cast_size = profile.cast_size.sample(rng).round().max(2.0) as u8;

    let scene = SceneFeatureInput {
        scene_type,
        cast_size,
        tension,
    };

    // Select characteristic event by weighted random
    let weights: Vec<f32> = profile
        .characteristic_events
        .iter()
        .map(|e| e.weight)
        .collect();

    let event = if let Ok(dist) = WeightedIndex::new(&weights) {
        let idx = dist.sample(rng);
        let selected = &profile.characteristic_events[idx];
        EventFeatureInput {
            event_type: parse_event_type(&selected.event_type),
            emotional_register: parse_emotional_register(&selected.emotional_register),
            confidence: rng.random_range(0.7..=0.95),
            target_count: 1,
        }
    } else {
        // Fallback if weights are all zero
        EventFeatureInput {
            event_type: EventType::Speech,
            emotional_register: EmotionalRegister::Neutral,
            confidence: 0.8,
            target_count: 1,
        }
    };

    (scene, event)
}

fn parse_scene_type(s: &str) -> SceneType {
    match s {
        "Gravitational" => SceneType::Gravitational,
        "Connective" => SceneType::Connective,
        "Gate" => SceneType::Gate,
        "Threshold" => SceneType::Threshold,
        _ => SceneType::Connective,
    }
}

pub fn parse_event_type(s: &str) -> EventType {
    match s {
        "Speech" => EventType::Speech,
        "Action" => EventType::Action,
        "Movement" => EventType::Movement,
        "Observation" => EventType::Observation,
        "Interaction" => EventType::Interaction,
        "Emote" => EventType::Emote,
        "Inquiry" => EventType::Inquiry,
        _ => EventType::Speech,
    }
}

pub fn parse_emotional_register(s: &str) -> EmotionalRegister {
    match s {
        "Aggressive" => EmotionalRegister::Aggressive,
        "Vulnerable" => EmotionalRegister::Vulnerable,
        "Playful" => EmotionalRegister::Playful,
        "Guarded" => EmotionalRegister::Guarded,
        "Neutral" => EmotionalRegister::Neutral,
        "Tender" => EmotionalRegister::Tender,
        "Inquisitive" => EmotionalRegister::Inquisitive,
        _ => EmotionalRegister::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::super::descriptors::{ValueRange, WeightedEvent};
    use super::*;

    fn test_profile() -> ProfileDescriptor {
        ProfileDescriptor {
            id: "test".to_string(),
            display_name: "Test".to_string(),
            description: "Test profile".to_string(),
            scene_type: "Gravitational".to_string(),
            tension: ValueRange { min: 0.3, max: 0.7 },
            cast_size: ValueRange { min: 2.0, max: 3.0 },
            characteristic_events: vec![
                WeightedEvent {
                    event_type: "Speech".to_string(),
                    emotional_register: "Tender".to_string(),
                    weight: 0.5,
                },
                WeightedEvent {
                    event_type: "Observation".to_string(),
                    emotional_register: "Neutral".to_string(),
                    weight: 0.5,
                },
            ],
        }
    }

    #[test]
    fn instantiate_produces_valid_scene() {
        let profile = test_profile();
        let mut rng = rand::rng();
        let (scene, event) = instantiate_scene_and_event(&profile, &mut rng);
        assert_eq!(scene.scene_type, SceneType::Gravitational);
        assert!((0.0..=1.0).contains(&scene.tension));
        assert!(scene.cast_size >= 2);
        assert!((0.0..=1.0).contains(&event.confidence));
    }

    #[test]
    fn parse_scene_types() {
        assert_eq!(parse_scene_type("Gravitational"), SceneType::Gravitational);
        assert_eq!(parse_scene_type("Gate"), SceneType::Gate);
        assert_eq!(parse_scene_type("Threshold"), SceneType::Threshold);
        assert_eq!(parse_scene_type("unknown"), SceneType::Connective);
    }

    #[test]
    fn parse_event_types_and_registers() {
        assert_eq!(parse_event_type("Speech"), EventType::Speech);
        assert_eq!(parse_event_type("Action"), EventType::Action);
        assert_eq!(
            parse_emotional_register("Aggressive"),
            EmotionalRegister::Aggressive
        );
        assert_eq!(
            parse_emotional_register("Tender"),
            EmotionalRegister::Tender
        );
    }
}
