//! Character prediction types — ML model outputs replacing LLM character agents.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! In the narrator-centric architecture, character behavior is predicted by ML
//! models (ONNX/ort) rather than individual LLM agents. These types represent
//! the structured output of those predictions, consumed by the Resolver and
//! then by the Narrator's context assembly pipeline.
//!
//! The prediction pipeline: tensor features + emotional state + relational edges
//! + scene context → ML model → structured predictions with confidence scores.

use super::entity::EntityId;
use super::tensor::AwarenessLevel;

// ---------------------------------------------------------------------------
// Action predictions — what the character intends to do
// ---------------------------------------------------------------------------

/// What kind of action a character intends to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    /// A physical or social action (e.g., picking up an object, offering a hand).
    Perform,
    /// Speaking aloud to another character or to the space.
    Speak,
    /// Physical movement within the scene.
    Move,
    /// Observing, noticing, attending to something.
    Examine,
    /// Deliberate inaction — choosing to stay still, remain silent.
    Wait,
    /// Pushing back against an external pressure or another character's action.
    Resist,
}

/// A predicted action for a character — what they intend to do and how confident
/// the model is in this prediction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionPrediction {
    /// Natural-language description of the intended action.
    pub description: String,
    /// Model confidence in this prediction. Range: [0.0, 1.0].
    pub confidence: f32,
    /// The category of action.
    pub action_type: ActionType,
}

// ---------------------------------------------------------------------------
// Speech predictions — what and how the character speaks
// ---------------------------------------------------------------------------

/// How a character delivers speech — volume and register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpeechRegister {
    /// Quiet, intimate, possibly meant for only one listener.
    Whisper,
    /// Normal speech — the default register.
    Conversational,
    /// Raised voice, addressing a group or making a point.
    Declamatory,
    /// Internal monologue — not spoken aloud.
    Internal,
}

/// A predicted speech act — direction rather than exact words.
///
/// The ML model predicts *what the character would say about* and *how*,
/// not the exact dialogue. The Narrator renders actual words.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpeechPrediction {
    /// What the character would speak about — topic and direction, not exact words.
    pub content_direction: String,
    /// How the character delivers it.
    pub register: SpeechRegister,
    /// Model confidence. Range: [0.0, 1.0].
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Thought predictions — internal state visible to the Narrator
// ---------------------------------------------------------------------------

/// A predicted internal state — what the character is thinking/feeling beneath
/// the surface. Visible to the Narrator for rendering subtext.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThoughtPrediction {
    /// The emotional subtext beneath the character's observable actions.
    pub emotional_subtext: String,
    /// How conscious the character is of this internal state.
    pub awareness_level: AwarenessLevel,
    /// If present, a tension the character is navigating internally.
    pub internal_conflict: Option<String>,
}

// ---------------------------------------------------------------------------
// Emotional deltas — how the character's emotional state shifts
// ---------------------------------------------------------------------------

/// A predicted shift in a single emotional primary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionalDelta {
    /// Which emotional primary is shifting (grammar-relative, e.g. "joy").
    pub primary_id: String,
    /// How much the intensity changes. Positive = increase, negative = decrease.
    pub intensity_change: f32,
    /// If awareness shifts, the new level. None means awareness unchanged.
    pub awareness_change: Option<AwarenessLevel>,
}

// ---------------------------------------------------------------------------
// Event classification — structured player input
// ---------------------------------------------------------------------------

/// Classification of a raw player input into typed event data.
///
/// Produced by the event classifier (ML or rule-based) from `PlayerInput`.
/// Consumed by the prediction pipeline and context assembly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassifiedEvent {
    /// What kind of event the player input represents.
    pub event_type: EventType,
    /// Entity IDs targeted or referenced by this event.
    pub targets: Vec<EntityId>,
    /// The classifier's interpretation of player intent.
    pub inferred_intent: String,
    /// The emotional register of the input.
    pub emotional_register: EmotionalRegister,
    /// Classifier confidence. Range: [0.0, 1.0].
    pub confidence: f32,
}

/// What kind of action the player's input represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventType {
    /// Player character speaks to someone.
    Speech,
    /// Player character performs a physical action.
    Action,
    /// Player character moves within or between spaces.
    Movement,
    /// Player character observes or examines something.
    Observation,
    /// Player character interacts with an object or entity.
    Interaction,
    /// Player character expresses emotion or internal state.
    Emote,
    /// Meta-input — player asking about the world or seeking clarification.
    Inquiry,
}

/// The emotional register of a player's input — how it "feels."
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EmotionalRegister {
    /// Forceful, confrontational, assertive.
    Aggressive,
    /// Open, exposed, seeking connection.
    Vulnerable,
    /// Light, humorous, testing boundaries.
    Playful,
    /// Measured, restrained, holding back.
    Guarded,
    /// Flat, factual, emotionally neutral.
    Neutral,
    /// Warm, supportive, caring.
    Tender,
    /// Curious, seeking, exploring.
    Inquisitive,
}

// ---------------------------------------------------------------------------
// Activated tensor frame — the ML model's selection of relevant axes
// ---------------------------------------------------------------------------

/// A subset of a character's tensor axes activated for the current context.
///
/// The frame computation ML model reads the full tensor + scene context and
/// selects which axes are relevant, why, and how confident it is. This replaces
/// the "dump entire tensor into LLM prompt" approach.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActivatedTensorFrame {
    /// Axis names selected as relevant to the current context.
    pub activated_axes: Vec<String>,
    /// Why these axes were activated — readable by the Narrator for context.
    pub activation_reason: String,
    /// Model confidence in this frame selection. Range: [0.0, 1.0].
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Complete character prediction — the full ML output for one character
// ---------------------------------------------------------------------------

/// The complete prediction output for a single character in a single turn.
///
/// Produced by the character prediction ML model, consumed by the Resolver
/// (which sequences and resolves conflicts) and then by the Narrator context
/// assembly pipeline (which renders it into prose).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterPrediction {
    /// Which character this prediction is for.
    pub character_id: EntityId,
    /// The character's name (for rendering convenience).
    pub character_name: String,
    /// The activated tensor frame used for this prediction.
    pub frame: ActivatedTensorFrame,
    /// Predicted actions (may be empty if the character waits).
    pub actions: Vec<ActionPrediction>,
    /// Predicted speech (may be empty if the character is silent).
    pub speech: Option<SpeechPrediction>,
    /// Internal state visible to the Narrator.
    pub thought: ThoughtPrediction,
    /// Emotional shifts predicted for this turn.
    pub emotional_deltas: Vec<EmotionalDelta>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_prediction_is_constructible() {
        let pred = ActionPrediction {
            description: "Bramblehoof approaches the fence slowly".to_string(),
            confidence: 0.85,
            action_type: ActionType::Move,
        };
        assert!(pred.confidence > 0.0 && pred.confidence <= 1.0);
        assert_eq!(pred.action_type, ActionType::Move);
    }

    #[test]
    fn speech_prediction_is_constructible() {
        let pred = SpeechPrediction {
            content_direction: "Greeting, warmth, surprise at seeing an old friend".to_string(),
            register: SpeechRegister::Conversational,
            confidence: 0.75,
        };
        assert_eq!(pred.register, SpeechRegister::Conversational);
    }

    #[test]
    fn character_prediction_is_constructible() {
        let pred = CharacterPrediction {
            character_id: EntityId::new(),
            character_name: "Bramblehoof".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec![
                    "empathy".to_string(),
                    "protective_impulse".to_string(),
                    "grief".to_string(),
                ],
                activation_reason: "Returning to a place of loss with hope".to_string(),
                confidence: 0.80,
            },
            actions: vec![ActionPrediction {
                description: "Approaches the gate, one hand raised in greeting".to_string(),
                confidence: 0.85,
                action_type: ActionType::Move,
            }],
            speech: Some(SpeechPrediction {
                content_direction: "A warm greeting, recognition".to_string(),
                register: SpeechRegister::Conversational,
                confidence: 0.70,
            }),
            thought: ThoughtPrediction {
                emotional_subtext:
                    "Joy at seeing Pyotir alive, dread of what he might find changed".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: Some(
                    "Wants to ask about the music but fears the answer".to_string(),
                ),
            },
            emotional_deltas: vec![EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.2,
                awareness_change: None,
            }],
        };
        assert_eq!(pred.character_name, "Bramblehoof");
        assert_eq!(pred.frame.activated_axes.len(), 3);
        assert_eq!(pred.actions.len(), 1);
        assert!(pred.speech.is_some());
    }

    #[test]
    fn classified_event_is_constructible() {
        let event = ClassifiedEvent {
            event_type: EventType::Speech,
            targets: vec![EntityId::new()],
            inferred_intent: "Player greets the farmer".to_string(),
            emotional_register: EmotionalRegister::Tender,
            confidence: 0.90,
        };
        assert_eq!(event.event_type, EventType::Speech);
        assert_eq!(event.emotional_register, EmotionalRegister::Tender);
    }

    #[test]
    fn emotional_delta_can_shift_awareness() {
        let delta = EmotionalDelta {
            primary_id: "anger".to_string(),
            intensity_change: 0.3,
            awareness_change: Some(AwarenessLevel::Recognizable),
        };
        assert!(delta.awareness_change.is_some());
    }
}
