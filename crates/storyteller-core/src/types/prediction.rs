//! Character prediction types — two-tier: raw ML output and assembled predictions.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! The prediction pipeline has two stages:
//!
//! 1. **Raw predictions** (`Raw*` types): The ML model (ONNX/ort) outputs only
//!    structured values — enums, floats, indices. No natural language. These are
//!    decoded from the model's output tensor by `storyteller-ml::feature_schema`.
//!
//! 2. **Assembled predictions** (non-`Raw` types): The context assembly system
//!    enriches raw predictions with descriptive annotations by looking up tensor
//!    axes, relational edges, scene affordances, and emotional posture from the
//!    graph. The Narrator receives these assembled predictions as structured
//!    narrative briefings.
//!
//! Downstream consumers (Resolver, GameDesignSystem, Narrator) use the assembled
//! types. The raw types are internal to the prediction → assembly pipeline.

use super::entity::EntityId;
use super::tensor::AwarenessLevel;

// ===========================================================================
// Raw ML output types — structured values only, no natural language
// ===========================================================================

// ---------------------------------------------------------------------------
// Raw action prediction
// ---------------------------------------------------------------------------

/// What context drives a predicted action — helps the assembly system know
/// which graph/tensor data to look up for enrichment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionContext {
    /// The action references shared history between characters.
    SharedHistory,
    /// The action responds to immediate scene circumstances.
    CurrentScene,
    /// The action is driven by the character's emotional state.
    EmotionalReaction,
    /// The action is driven by relationship dynamics.
    RelationalDynamic,
    /// The action responds to the world or environment.
    WorldResponse,
}

/// Raw ML output for a predicted action — structured values only.
///
/// The ML model predicts what *kind* of action, toward whom, in what context,
/// and with what emotional charge. The assembly system generates the
/// natural-language description from these values plus graph/tensor lookups.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawActionPrediction {
    /// The category of action.
    pub action_type: ActionType,
    /// Model confidence in this prediction. Range: \[0.0, 1.0\].
    pub confidence: f32,
    /// Target entity, if the action is directed at someone/something.
    /// Decoded from the model's per-cast-member sigmoid outputs.
    pub target: Option<EntityId>,
    /// Emotional valence of the action. Range: \[-1.0, 1.0\].
    /// Negative = withdrawn/hostile, positive = open/generous.
    pub emotional_valence: f32,
    /// What context drives this action — tells assembly where to look.
    pub action_context: ActionContext,
}

// ---------------------------------------------------------------------------
// Raw speech prediction
// ---------------------------------------------------------------------------

/// Raw ML output for a predicted speech act — structured values only.
///
/// The model predicts *whether* speech occurs, in what register, and with
/// what confidence. It does NOT predict what the character says about —
/// the assembly system infers topic from the action context + relational data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawSpeechPrediction {
    /// Whether the character speaks at all.
    pub occurs: bool,
    /// How the character delivers speech (if it occurs).
    pub register: SpeechRegister,
    /// Model confidence. Range: \[0.0, 1.0\].
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Raw thought prediction
// ---------------------------------------------------------------------------

/// Raw ML output for a predicted internal state — structured values only.
///
/// The model predicts the awareness level and which emotion dominates
/// internally. The assembly system generates the emotional subtext
/// description from the character's tensor profile + relational context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawThoughtPrediction {
    /// How conscious the character is of their internal state.
    pub awareness_level: AwarenessLevel,
    /// Index into the emotional grammar's primaries list.
    /// For Plutchik: 0=joy, 1=trust, 2=fear, 3=surprise,
    /// 4=sadness, 5=disgust, 6=anger, 7=anticipation.
    pub dominant_emotion_index: u8,
}

// ---------------------------------------------------------------------------
// Raw emotional delta
// ---------------------------------------------------------------------------

/// Raw ML output for a predicted shift in a single emotional primary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawEmotionalDelta {
    /// Index into the emotional grammar's primaries list.
    pub primary_index: u8,
    /// How much the intensity changes. Positive = increase, negative = decrease.
    /// Range: \[-1.0, 1.0\] (tanh output).
    pub intensity_change: f32,
    /// Whether the character's awareness of this emotion shifts.
    pub awareness_shifts: bool,
}

// ---------------------------------------------------------------------------
// Raw activated tensor frame
// ---------------------------------------------------------------------------

/// Raw ML output for which tensor axes are activated in this context.
///
/// The frame computation model selects which axes matter for the current
/// scene. Indices refer to the canonical axis ordering defined in
/// `storyteller-ml::feature_schema`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawActivatedTensorFrame {
    /// Indices into the canonical axis ordering.
    pub activated_axis_indices: Vec<u16>,
    /// Model confidence in this frame selection. Range: \[0.0, 1.0\].
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Raw character prediction — the complete ML output for one character
// ---------------------------------------------------------------------------

/// The complete raw ML output for a single character in a single turn.
///
/// Contains only structured values (enums, floats, indices). No natural
/// language. The context assembly system enriches this into a full
/// [`CharacterPrediction`] with descriptive annotations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawCharacterPrediction {
    /// Which character this prediction is for.
    pub character_id: EntityId,
    /// The activated tensor frame used for this prediction.
    pub frame: RawActivatedTensorFrame,
    /// Predicted action (the highest-confidence action from the model).
    pub action: RawActionPrediction,
    /// Predicted speech (may indicate no speech via `occurs: false`).
    pub speech: RawSpeechPrediction,
    /// Internal state prediction.
    pub thought: RawThoughtPrediction,
    /// Emotional shifts predicted for this turn (one per primary that changes).
    pub emotional_deltas: Vec<RawEmotionalDelta>,
}

// ===========================================================================
// Assembled prediction types — enriched with narrative context
// ===========================================================================
//
// These types are produced by the context assembly system from raw ML
// predictions + graph/tensor lookups. They contain natural-language
// annotations that the Resolver and Narrator consume as structured
// narrative briefings.

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

/// Assembled action prediction — raw ML output enriched with a narrative description.
///
/// The `description` field is generated by the context assembly system from the
/// raw prediction's action type, target, emotional valence, and action context,
/// combined with graph lookups (relational edges, tensor axes, scene affordances).
///
/// Consumed by the Resolver (for sequencing) and Narrator (for prose rendering).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionPrediction {
    /// Natural-language description of the intended action.
    /// Generated by context assembly, NOT by the ML model.
    pub description: String,
    /// Model confidence in this prediction. Range: \[0.0, 1.0\].
    pub confidence: f32,
    /// The category of action.
    pub action_type: ActionType,
    /// Target entity, if the action is directed at someone/something.
    pub target: Option<EntityId>,
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

/// Assembled speech prediction — raw ML output enriched with topic direction.
///
/// The `content_direction` field is generated by the context assembly system
/// from the scene context, action context, and relational data — the ML model
/// only predicts whether speech occurs and in what register.
///
/// The Narrator renders actual dialogue from this direction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpeechPrediction {
    /// What the character would speak about — topic and direction, not exact words.
    /// Generated by context assembly, NOT by the ML model.
    pub content_direction: String,
    /// How the character delivers it.
    pub register: SpeechRegister,
    /// Model confidence. Range: \[0.0, 1.0\].
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Thought predictions — internal state visible to the Narrator
// ---------------------------------------------------------------------------

/// Assembled thought prediction — raw ML output enriched with emotional subtext.
///
/// The `emotional_subtext` and `internal_conflict` fields are generated by the
/// context assembly system from the raw prediction's awareness level and dominant
/// emotion, combined with the character's tensor profile and relational edges.
///
/// Visible to the Narrator for rendering subtext and interior states.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThoughtPrediction {
    /// The emotional subtext beneath the character's observable actions.
    /// Generated by context assembly, NOT by the ML model.
    pub emotional_subtext: String,
    /// How conscious the character is of this internal state.
    pub awareness_level: AwarenessLevel,
    /// If present, a tension the character is navigating internally.
    /// Generated by context assembly, NOT by the ML model.
    pub internal_conflict: Option<String>,
}

// ---------------------------------------------------------------------------
// Emotional deltas — how the character's emotional state shifts
// ---------------------------------------------------------------------------

/// Assembled emotional delta — raw ML output enriched with the primary's name.
///
/// The `primary_id` string (e.g., "joy") is resolved by the context assembly
/// system from the raw prediction's `primary_index` using the active emotional
/// grammar. The raw ML model outputs only indices.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionalDelta {
    /// Which emotional primary is shifting (grammar-relative, e.g. "joy").
    /// Resolved from `RawEmotionalDelta::primary_index` by context assembly.
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

/// Assembled tensor frame — raw axis indices resolved to names with reasoning.
///
/// The `activated_axes` names and `activation_reason` are generated by the
/// context assembly system from the raw frame's axis indices, the character's
/// tensor profile, and the scene context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActivatedTensorFrame {
    /// Axis names selected as relevant to the current context.
    /// Resolved from `RawActivatedTensorFrame::activated_axis_indices`.
    pub activated_axes: Vec<String>,
    /// Why these axes were activated — readable by the Narrator for context.
    /// Generated by context assembly, NOT by the ML model.
    pub activation_reason: String,
    /// Model confidence in this frame selection. Range: \[0.0, 1.0\].
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Complete character prediction — the full ML output for one character
// ---------------------------------------------------------------------------

/// Assembled prediction for a single character in a single turn.
///
/// Produced by the context assembly system from a [`RawCharacterPrediction`]
/// enriched with graph/tensor lookups. Consumed by the Resolver (which
/// sequences and resolves conflicts) and the Narrator (which renders prose).
///
/// All `String` fields on this type and its children are generated by context
/// assembly — the ML model outputs only structured values via
/// [`RawCharacterPrediction`].
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
            target: None,
        };
        assert!((0.0..=1.0).contains(&pred.confidence));
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
        let pyotir_id = EntityId::new();
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
                target: Some(pyotir_id),
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

    // -----------------------------------------------------------------------
    // Raw ML output type tests
    // -----------------------------------------------------------------------

    #[test]
    fn raw_action_prediction_is_constructible() {
        let pred = RawActionPrediction {
            action_type: ActionType::Perform,
            confidence: 0.85,
            target: Some(EntityId::new()),
            emotional_valence: 0.6,
            action_context: ActionContext::SharedHistory,
        };
        assert!((0.0..=1.0).contains(&pred.confidence));
        assert!((-1.0..=1.0).contains(&pred.emotional_valence));
        assert_eq!(pred.action_context, ActionContext::SharedHistory);
    }

    #[test]
    fn raw_speech_prediction_is_constructible() {
        let pred = RawSpeechPrediction {
            occurs: true,
            register: SpeechRegister::Conversational,
            confidence: 0.70,
        };
        assert!(pred.occurs);

        let silent = RawSpeechPrediction {
            occurs: false,
            register: SpeechRegister::Internal,
            confidence: 0.90,
        };
        assert!(!silent.occurs);
    }

    #[test]
    fn raw_thought_prediction_is_constructible() {
        let pred = RawThoughtPrediction {
            awareness_level: AwarenessLevel::Defended,
            dominant_emotion_index: 2, // fear in Plutchik ordering
        };
        assert_eq!(pred.awareness_level, AwarenessLevel::Defended);
        assert_eq!(pred.dominant_emotion_index, 2);
    }

    #[test]
    fn raw_emotional_delta_is_constructible() {
        let delta = RawEmotionalDelta {
            primary_index: 0, // joy
            intensity_change: 0.2,
            awareness_shifts: false,
        };
        assert!(!delta.awareness_shifts);

        let shifting = RawEmotionalDelta {
            primary_index: 4, // sadness
            intensity_change: 0.3,
            awareness_shifts: true,
        };
        assert!(shifting.awareness_shifts);
    }

    #[test]
    fn raw_character_prediction_is_constructible() {
        let pyotir_id = EntityId::new();
        let pred = RawCharacterPrediction {
            character_id: EntityId::new(),
            frame: RawActivatedTensorFrame {
                activated_axis_indices: vec![0, 3, 7],
                confidence: 0.80,
            },
            action: RawActionPrediction {
                action_type: ActionType::Perform,
                confidence: 0.85,
                target: Some(pyotir_id),
                emotional_valence: 0.6,
                action_context: ActionContext::SharedHistory,
            },
            speech: RawSpeechPrediction {
                occurs: true,
                register: SpeechRegister::Conversational,
                confidence: 0.70,
            },
            thought: RawThoughtPrediction {
                awareness_level: AwarenessLevel::Recognizable,
                dominant_emotion_index: 0, // joy
            },
            emotional_deltas: vec![
                RawEmotionalDelta {
                    primary_index: 0,
                    intensity_change: 0.2,
                    awareness_shifts: false,
                },
                RawEmotionalDelta {
                    primary_index: 4,
                    intensity_change: -0.1,
                    awareness_shifts: false,
                },
            ],
        };
        assert_eq!(pred.frame.activated_axis_indices.len(), 3);
        assert!(pred.speech.occurs);
        assert_eq!(pred.emotional_deltas.len(), 2);
    }

    #[test]
    fn action_context_covers_all_variants() {
        let contexts = [
            ActionContext::SharedHistory,
            ActionContext::CurrentScene,
            ActionContext::EmotionalReaction,
            ActionContext::RelationalDynamic,
            ActionContext::WorldResponse,
        ];
        for i in 0..contexts.len() {
            for j in (i + 1)..contexts.len() {
                assert_ne!(contexts[i], contexts[j]);
            }
        }
    }
}
