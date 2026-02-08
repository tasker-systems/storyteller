//! Prediction enrichment and rendering — converts raw ML output into
//! Narrator-consumable context.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! Two stages:
//! 1. **Enrichment** (`enrich_prediction`): Deterministic transform from
//!    `RawCharacterPrediction` → `CharacterPrediction` by resolving indices
//!    to names, generating templated descriptions, and detecting internal
//!    conflicts from opposing emotional deltas.
//!
//! 2. **Rendering** (`render_predictions`): Formats assembled predictions
//!    as a markdown section for the Narrator's context window, following
//!    the same pattern as `preamble::render_preamble()`.

use storyteller_core::traits::emotional_grammar::EmotionalGrammar;
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::prediction::{
    ActionContext, ActionPrediction, ActionType, ActivatedTensorFrame, CharacterPrediction,
    EmotionalDelta, EmotionalRegister, EventType, RawCharacterPrediction, SpeechPrediction,
    SpeechRegister, ThoughtPrediction,
};
use storyteller_core::types::tensor::AwarenessLevel;
use storyteller_ml::feature_schema::{EventFeatureInput, PredictionInput, SceneFeatureInput};

use crate::inference::frame::CharacterPredictor;

use super::tokens::estimate_tokens;

/// Run the full predict → enrich pipeline for all characters in a scene.
///
/// Steps:
/// 1. Classify player input (naive keyword-based for prototype)
/// 2. Build scene features from scene data
/// 3. Build prediction input for each character
/// 4. Run batch inference via the ONNX model
/// 5. Enrich each raw prediction with narrative descriptions
///
/// Returns assembled predictions ready for the Narrator. Logs warnings
/// for any individual character prediction failures.
pub fn predict_character_behaviors(
    predictor: &CharacterPredictor,
    characters: &[&CharacterSheet],
    scene: &SceneData,
    player_input: &str,
    grammar: &dyn EmotionalGrammar,
) -> Vec<CharacterPrediction> {
    let event = classify_player_input(player_input, characters.len().saturating_sub(1));
    let scene_features = build_scene_features(scene, characters.len());

    let batch: Vec<(
        PredictionInput<'_>,
        storyteller_core::types::entity::EntityId,
        Vec<u16>,
        f32,
    )> = characters
        .iter()
        .map(|sheet| {
            let input = PredictionInput {
                character: sheet,
                edges: &[],
                target_roles: &[],
                scene: scene_features,
                event,
                history: &[],
            };
            let axis_count = sheet
                .tensor
                .axes
                .len()
                .min(storyteller_ml::feature_schema::MAX_TENSOR_AXES);
            let activated: Vec<u16> = (0..axis_count as u16).collect();
            (input, sheet.entity_id, activated, 0.8)
        })
        .collect();

    let results = predictor.predict_batch(&batch);

    results
        .into_iter()
        .zip(characters.iter())
        .filter_map(|(result, sheet)| match result {
            Ok(raw) => Some(enrich_prediction(&raw, sheet, scene, grammar)),
            Err(e) => {
                tracing::warn!(character = %sheet.name, error = %e, "prediction failed");
                None
            }
        })
        .collect()
}

/// Naive keyword-based event classification for the prototype.
///
/// Production will use an ML classifier. This is good enough to prove
/// the pipeline works end-to-end.
fn classify_player_input(input: &str, target_count: usize) -> EventFeatureInput {
    let lower = input.to_lowercase();

    let event_type = if lower.contains("say")
        || lower.contains("tell")
        || lower.contains("speak")
        || lower.contains("ask")
    {
        EventType::Speech
    } else if lower.contains("move")
        || lower.contains("walk")
        || lower.contains("approach")
        || lower.contains("go")
        || lower.contains("step")
    {
        EventType::Movement
    } else if lower.contains("look") || lower.contains("watch") || lower.contains("examine") {
        EventType::Observation
    } else if lower.contains('?') {
        EventType::Inquiry
    } else {
        EventType::Interaction
    };

    let emotional_register = if lower.contains("angry")
        || lower.contains("shout")
        || lower.contains("yell")
        || lower.contains("demand")
    {
        EmotionalRegister::Aggressive
    } else if lower.contains("cry")
        || lower.contains("weep")
        || lower.contains("sad")
        || lower.contains("plead")
    {
        EmotionalRegister::Vulnerable
    } else if lower.contains("laugh")
        || lower.contains("joke")
        || lower.contains("grin")
        || lower.contains("tease")
    {
        EmotionalRegister::Playful
    } else if lower.contains("careful")
        || lower.contains("slow")
        || lower.contains("quiet")
        || lower.contains("hesitat")
    {
        EmotionalRegister::Guarded
    } else if lower.contains("gentle")
        || lower.contains("kind")
        || lower.contains("warm")
        || lower.contains("soft")
    {
        EmotionalRegister::Tender
    } else if lower.contains("wonder")
        || lower.contains("curious")
        || lower.contains("what")
        || lower.contains("how")
    {
        EmotionalRegister::Inquisitive
    } else {
        EmotionalRegister::Neutral
    };

    EventFeatureInput {
        event_type,
        emotional_register,
        confidence: 0.8,
        target_count: target_count as u8,
    }
}

/// Build scene-level features from scene data.
fn build_scene_features(scene: &SceneData, cast_size: usize) -> SceneFeatureInput {
    SceneFeatureInput {
        scene_type: scene.scene_type,
        cast_size: cast_size as u8,
        tension: 0.5, // default — no tension field on SceneData yet
    }
}

/// Enrich a raw ML prediction into an assembled prediction with narrative
/// descriptions.
///
/// All string fields on the output are generated here via templates — no
/// LLM calls. The enrichment resolves axis indices to names, generates
/// action/speech/thought descriptions, and detects internal conflict from
/// opposing emotional deltas.
pub fn enrich_prediction(
    raw: &RawCharacterPrediction,
    character: &CharacterSheet,
    scene: &SceneData,
    grammar: &dyn EmotionalGrammar,
) -> CharacterPrediction {
    // 1. Resolve activated axis indices → axis names
    let axis_names = resolve_axis_names(&raw.frame.activated_axis_indices, character);
    let activation_reason = generate_activation_reason(&axis_names, scene);

    let frame = ActivatedTensorFrame {
        activated_axes: axis_names,
        activation_reason,
        confidence: raw.frame.confidence,
    };

    // 2. Generate action description
    let target_name = raw
        .action
        .target
        .and_then(|tid| resolve_target_name(tid, scene, character));
    let action_description = generate_action_description(
        raw.action.action_type,
        target_name.as_deref(),
        raw.action.action_context,
        raw.action.emotional_valence,
    );
    let actions = vec![ActionPrediction {
        description: action_description,
        confidence: raw.action.confidence,
        action_type: raw.action.action_type,
        target: raw.action.target,
    }];

    // 3. Generate speech direction (if speech occurs)
    let speech = if raw.speech.occurs {
        let content_direction =
            generate_speech_direction(raw.speech.register, raw.action.action_context, scene);
        Some(SpeechPrediction {
            content_direction,
            register: raw.speech.register,
            confidence: raw.speech.confidence,
        })
    } else {
        None
    };

    // 4. Generate thought subtext
    let dominant_primary = resolve_primary_name(raw.thought.dominant_emotion_index, grammar);
    let emotional_subtext = generate_emotional_subtext(
        &dominant_primary,
        raw.thought.awareness_level,
        &character.name,
    );
    let internal_conflict = detect_internal_conflict(&raw.emotional_deltas, grammar);
    let thought = ThoughtPrediction {
        emotional_subtext,
        awareness_level: raw.thought.awareness_level,
        internal_conflict,
    };

    // 5. Resolve emotional deltas
    let emotional_deltas: Vec<EmotionalDelta> = raw
        .emotional_deltas
        .iter()
        .map(|d| {
            let primary_id = resolve_primary_name(d.primary_index, grammar);
            let awareness_change = if d.awareness_shifts {
                Some(next_awareness_level(raw.thought.awareness_level))
            } else {
                None
            };
            EmotionalDelta {
                primary_id,
                intensity_change: d.intensity_change,
                awareness_change,
            }
        })
        .collect();

    CharacterPrediction {
        character_id: raw.character_id,
        character_name: character.name.clone(),
        frame,
        actions,
        speech,
        thought,
        emotional_deltas,
    }
}

/// Render assembled predictions as markdown for the Narrator's context.
///
/// Follows the same structural pattern as `preamble::render_preamble()` —
/// markdown sections that the Narrator reads as structured briefing.
pub fn render_predictions(predictions: &[CharacterPrediction]) -> String {
    if predictions.is_empty() {
        return String::new();
    }

    let mut output = String::from("## Character Predictions\n\n");

    for pred in predictions {
        output.push_str(&format!("### {}\n", pred.character_name));

        // Frame
        let axes_str = pred.frame.activated_axes.join(", ");
        output.push_str(&format!(
            "**Frame**: {} ({:.2} confidence)\n",
            axes_str, pred.frame.confidence
        ));
        output.push_str(&pred.frame.activation_reason);
        output.push('\n');

        // Actions
        for action in &pred.actions {
            let target_suffix = action.target.map(|_| "").unwrap_or("");
            output.push_str(&format!(
                "**Action** ({:.2}): {}{}\n",
                action.confidence, action.description, target_suffix
            ));
        }

        // Speech
        if let Some(speech) = &pred.speech {
            output.push_str(&format!(
                "**Speech** ({:?}, {:.2}): {}\n",
                speech.register, speech.confidence, speech.content_direction
            ));
        }

        // Internal state
        output.push_str(&format!(
            "**Internal**: {}\n",
            pred.thought.emotional_subtext
        ));
        output.push_str(&format!("  Awareness: {:?}", pred.thought.awareness_level));
        if let Some(conflict) = &pred.thought.internal_conflict {
            output.push_str(&format!(" | Conflict: {conflict}"));
        }
        output.push('\n');

        // Emotional deltas
        if !pred.emotional_deltas.is_empty() {
            let deltas: Vec<String> = pred
                .emotional_deltas
                .iter()
                .map(|d| {
                    let sign = if d.intensity_change >= 0.0 { "+" } else { "" };
                    format!("{} {sign}{:.1}", d.primary_id, d.intensity_change)
                })
                .collect();
            output.push_str(&format!("**Emotional shifts**: {}\n", deltas.join(", ")));
        }

        output.push('\n');
    }

    output
}

/// Estimate token count for rendered predictions.
pub fn estimate_predictions_tokens(predictions: &[CharacterPrediction]) -> u32 {
    let rendered = render_predictions(predictions);
    estimate_tokens(&rendered)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolve axis indices to axis names from the character's tensor BTreeMap.
///
/// The canonical ordering matches BTreeMap sorted key order, which is the
/// same ordering used by `feature_schema::encode_features`.
fn resolve_axis_names(indices: &[u16], character: &CharacterSheet) -> Vec<String> {
    let axis_keys: Vec<&String> = character.tensor.axes.keys().collect();
    indices
        .iter()
        .filter_map(|&idx| axis_keys.get(idx as usize).map(|k| (*k).clone()))
        .collect()
}

/// Look up a target entity's name from the scene cast or character sheet.
fn resolve_target_name(
    target_id: storyteller_core::types::entity::EntityId,
    scene: &SceneData,
    character: &CharacterSheet,
) -> Option<String> {
    // Check scene cast first
    if let Some(cast_entry) = scene.cast.iter().find(|c| c.entity_id == target_id) {
        return Some(cast_entry.name.clone());
    }
    // Fall back to self-reference
    if character.entity_id == target_id {
        return Some(character.name.clone());
    }
    None
}

/// Generate a brief reason for the frame's axis activation based on scene context.
fn generate_activation_reason(axis_names: &[String], scene: &SceneData) -> String {
    if axis_names.is_empty() {
        return format!("Entering {}", scene.title);
    }
    let stakes_hint = scene
        .stakes
        .first()
        .map(|s| s.as_str())
        .unwrap_or("the current moment");
    format!(
        "Active in the context of {}",
        truncate_hint(stakes_hint, 80)
    )
}

/// Generate a templated action description from raw prediction values.
fn generate_action_description(
    action_type: ActionType,
    target_name: Option<&str>,
    context: ActionContext,
    emotional_valence: f32,
) -> String {
    let verb = match action_type {
        ActionType::Perform => "Acts",
        ActionType::Speak => "Speaks",
        ActionType::Move => "Approaches",
        ActionType::Examine => "Observes",
        ActionType::Wait => "Waits",
        ActionType::Resist => "Resists",
    };

    let target_str = target_name.map(|n| format!(" {n}")).unwrap_or_default();

    let context_str = match context {
        ActionContext::SharedHistory => "driven by shared history",
        ActionContext::CurrentScene => "responding to the moment",
        ActionContext::EmotionalReaction => "driven by emotion",
        ActionContext::RelationalDynamic => "shaped by the relationship",
        ActionContext::WorldResponse => "reacting to the surroundings",
    };

    let valence_str = if emotional_valence > 0.3 {
        "with warmth"
    } else if emotional_valence < -0.3 {
        "with tension"
    } else {
        "with restraint"
    };

    format!("{verb}{target_str} \u{2014} {context_str}, {valence_str}")
}

/// Generate a speech content direction from register, context, and scene.
fn generate_speech_direction(
    register: SpeechRegister,
    context: ActionContext,
    scene: &SceneData,
) -> String {
    let register_hint = match register {
        SpeechRegister::Whisper => "Quietly, intimately",
        SpeechRegister::Conversational => "In natural conversation",
        SpeechRegister::Declamatory => "With raised voice, addressing the space",
        SpeechRegister::Internal => "Internally, unspoken",
    };

    let topic = match context {
        ActionContext::SharedHistory => "what they share, what has passed",
        ActionContext::CurrentScene => "what is happening now",
        ActionContext::EmotionalReaction => "what they feel",
        ActionContext::RelationalDynamic => "the connection between them",
        ActionContext::WorldResponse => "the world around them",
    };

    let stakes_hint = scene
        .stakes
        .first()
        .map(|s| truncate_hint(s, 60))
        .unwrap_or_else(|| "this moment".to_string());

    format!("{register_hint} \u{2014} about {topic}, in the context of {stakes_hint}")
}

/// Resolve a primary emotion index to its string ID via the grammar.
fn resolve_primary_name(index: u8, grammar: &dyn EmotionalGrammar) -> String {
    let primaries = grammar.primaries();
    primaries
        .get(index as usize)
        .map(|p| p.id.clone())
        .unwrap_or_else(|| format!("unknown_{index}"))
}

/// Generate emotional subtext from the dominant emotion, awareness, and character name.
fn generate_emotional_subtext(
    dominant_primary: &str,
    awareness: AwarenessLevel,
    character_name: &str,
) -> String {
    let awareness_str = match awareness {
        AwarenessLevel::Articulate => "consciously feels",
        AwarenessLevel::Recognizable => "senses",
        AwarenessLevel::Preconscious => "is moved by",
        AwarenessLevel::Defended => "deflects from",
        AwarenessLevel::Structural => "is shaped by",
    };

    format!("{character_name} {awareness_str} {dominant_primary}")
}

/// Detect internal conflict from opposing emotional deltas.
///
/// If two deltas move in opposite directions (one positive, one negative),
/// that signals an internal tension the Narrator can render as subtext.
fn detect_internal_conflict(
    deltas: &[storyteller_core::types::prediction::RawEmotionalDelta],
    grammar: &dyn EmotionalGrammar,
) -> Option<String> {
    let primaries = grammar.primaries();

    // Look for pairs where one increases and one decreases
    for (i, a) in deltas.iter().enumerate() {
        for b in deltas.iter().skip(i + 1) {
            let opposing = (a.intensity_change > 0.0 && b.intensity_change < 0.0)
                || (a.intensity_change < 0.0 && b.intensity_change > 0.0);

            if opposing {
                let name_a = primaries
                    .get(a.primary_index as usize)
                    .map(|p| p.id.as_str())
                    .unwrap_or("unknown");
                let name_b = primaries
                    .get(b.primary_index as usize)
                    .map(|p| p.id.as_str())
                    .unwrap_or("unknown");

                let rising = if a.intensity_change > 0.0 {
                    name_a
                } else {
                    name_b
                };
                let falling = if a.intensity_change < 0.0 {
                    name_a
                } else {
                    name_b
                };

                return Some(format!("{rising} rising while {falling} recedes"));
            }
        }
    }

    None
}

/// Get the next higher awareness level (for prototype awareness shifts).
fn next_awareness_level(current: AwarenessLevel) -> AwarenessLevel {
    match current {
        AwarenessLevel::Structural => AwarenessLevel::Defended,
        AwarenessLevel::Defended => AwarenessLevel::Preconscious,
        AwarenessLevel::Preconscious => AwarenessLevel::Recognizable,
        AwarenessLevel::Recognizable => AwarenessLevel::Articulate,
        AwarenessLevel::Articulate => AwarenessLevel::Articulate, // already max
    }
}

/// Truncate a string hint to a max length, adding ellipsis if needed.
fn truncate_hint(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated = &s[..s.floor_char_boundary(max_len.saturating_sub(3))];
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::grammars::PlutchikWestern;
    use storyteller_core::types::entity::EntityId;
    use storyteller_core::types::prediction::{
        RawActionPrediction, RawActivatedTensorFrame, RawEmotionalDelta, RawSpeechPrediction,
        RawThoughtPrediction,
    };

    fn mock_raw_prediction(
        character_id: EntityId,
        target: Option<EntityId>,
    ) -> RawCharacterPrediction {
        RawCharacterPrediction {
            character_id,
            frame: RawActivatedTensorFrame {
                activated_axis_indices: vec![0, 1],
                confidence: 0.80,
            },
            action: RawActionPrediction {
                action_type: ActionType::Move,
                confidence: 0.85,
                target,
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
                    primary_index: 0, // joy
                    intensity_change: 0.2,
                    awareness_shifts: false,
                },
                RawEmotionalDelta {
                    primary_index: 4, // sadness
                    intensity_change: -0.1,
                    awareness_shifts: false,
                },
            ],
        }
    }

    #[test]
    fn enrich_with_workshop_data() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let grammar = PlutchikWestern::new();

        let raw = mock_raw_prediction(bramblehoof.entity_id, Some(pyotir.entity_id));
        let enriched = enrich_prediction(&raw, &bramblehoof, &scene, &grammar);

        assert_eq!(enriched.character_name, "Bramblehoof");
        assert_eq!(enriched.character_id, bramblehoof.entity_id);
        assert!(!enriched.frame.activated_axes.is_empty());
        assert!(!enriched.frame.activation_reason.is_empty());
        assert_eq!(enriched.actions.len(), 1);
        assert!(!enriched.actions[0].description.is_empty());
        assert!(enriched.speech.is_some());
        assert!(!enriched.thought.emotional_subtext.is_empty());
        assert_eq!(enriched.emotional_deltas.len(), 2);
    }

    #[test]
    fn axis_resolution_from_btreemap() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        // BTreeMap keys are sorted — get the first two
        let expected_keys: Vec<String> = bramblehoof.tensor.axes.keys().take(2).cloned().collect();

        let resolved = resolve_axis_names(&[0, 1], &bramblehoof);
        assert_eq!(resolved, expected_keys);
    }

    #[test]
    fn emotion_index_resolves_to_primary_name() {
        let grammar = PlutchikWestern::new();
        assert_eq!(resolve_primary_name(0, &grammar), "joy");
        assert_eq!(resolve_primary_name(1, &grammar), "sadness");
        assert_eq!(resolve_primary_name(4, &grammar), "fear");
        assert_eq!(resolve_primary_name(7, &grammar), "anticipation");
    }

    #[test]
    fn speech_absent_when_occurs_false() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let grammar = PlutchikWestern::new();

        let mut raw = mock_raw_prediction(bramblehoof.entity_id, None);
        raw.speech.occurs = false;

        let enriched = enrich_prediction(&raw, &bramblehoof, &scene, &grammar);
        assert!(enriched.speech.is_none());
    }

    #[test]
    fn internal_conflict_detected_from_opposing_deltas() {
        let grammar = PlutchikWestern::new();

        // Joy increasing, sadness decreasing — opposing
        let deltas = vec![
            storyteller_core::types::prediction::RawEmotionalDelta {
                primary_index: 0, // joy
                intensity_change: 0.3,
                awareness_shifts: false,
            },
            storyteller_core::types::prediction::RawEmotionalDelta {
                primary_index: 1, // sadness
                intensity_change: -0.2,
                awareness_shifts: false,
            },
        ];

        let conflict = detect_internal_conflict(&deltas, &grammar);
        assert!(conflict.is_some());
        let text = conflict.unwrap();
        assert!(text.contains("joy"), "should mention joy: {text}");
        assert!(text.contains("sadness"), "should mention sadness: {text}");
    }

    #[test]
    fn no_conflict_when_deltas_same_direction() {
        let grammar = PlutchikWestern::new();

        let deltas = vec![
            storyteller_core::types::prediction::RawEmotionalDelta {
                primary_index: 0,
                intensity_change: 0.2,
                awareness_shifts: false,
            },
            storyteller_core::types::prediction::RawEmotionalDelta {
                primary_index: 1,
                intensity_change: 0.1,
                awareness_shifts: false,
            },
        ];

        let conflict = detect_internal_conflict(&deltas, &grammar);
        assert!(conflict.is_none());
    }

    #[test]
    fn render_predictions_has_all_sections() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let grammar = PlutchikWestern::new();

        let raw_b = mock_raw_prediction(bramblehoof.entity_id, Some(pyotir.entity_id));
        let raw_p = mock_raw_prediction(pyotir.entity_id, Some(bramblehoof.entity_id));

        let pred_b = enrich_prediction(&raw_b, &bramblehoof, &scene, &grammar);
        let pred_p = enrich_prediction(&raw_p, &pyotir, &scene, &grammar);

        let rendered = render_predictions(&[pred_b, pred_p]);

        assert!(rendered.contains("## Character Predictions"));
        assert!(rendered.contains("### Bramblehoof"));
        assert!(rendered.contains("### Pyotir"));
        assert!(rendered.contains("**Frame**"));
        assert!(rendered.contains("**Action**"));
        assert!(rendered.contains("**Speech**"));
        assert!(rendered.contains("**Internal**"));
        assert!(rendered.contains("Awareness:"));
        assert!(rendered.contains("**Emotional shifts**"));
    }

    #[test]
    fn render_empty_predictions_is_clean() {
        let rendered = render_predictions(&[]);
        assert!(rendered.is_empty());
    }

    #[test]
    fn estimate_tokens_is_reasonable() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let grammar = PlutchikWestern::new();

        let raw_b = mock_raw_prediction(bramblehoof.entity_id, Some(pyotir.entity_id));
        let raw_p = mock_raw_prediction(pyotir.entity_id, Some(bramblehoof.entity_id));

        let pred_b = enrich_prediction(&raw_b, &bramblehoof, &scene, &grammar);
        let pred_p = enrich_prediction(&raw_p, &pyotir, &scene, &grammar);

        let tokens = estimate_predictions_tokens(&[pred_b, pred_p]);
        // Two characters with full predictions — should be in 50-500 range
        assert!(
            (30..=500).contains(&tokens),
            "Expected 30-500 tokens, got {tokens}"
        );
    }

    #[test]
    fn awareness_shift_produces_next_level() {
        assert_eq!(
            next_awareness_level(AwarenessLevel::Structural),
            AwarenessLevel::Defended
        );
        assert_eq!(
            next_awareness_level(AwarenessLevel::Defended),
            AwarenessLevel::Preconscious
        );
        assert_eq!(
            next_awareness_level(AwarenessLevel::Preconscious),
            AwarenessLevel::Recognizable
        );
        assert_eq!(
            next_awareness_level(AwarenessLevel::Recognizable),
            AwarenessLevel::Articulate
        );
        assert_eq!(
            next_awareness_level(AwarenessLevel::Articulate),
            AwarenessLevel::Articulate
        );
    }
}

/// Feature-gated pilot validation tests requiring an ONNX model on disk.
///
/// Enable with: `cargo test --features test-ml-model`
/// Requires `STORYTELLER_MODEL_PATH` or `STORYTELLER_DATA_PATH` env var.
#[cfg(test)]
#[cfg(feature = "test-ml-model")]
mod with_model {
    use super::*;
    use std::path::PathBuf;

    use storyteller_core::grammars::PlutchikWestern;
    use storyteller_core::types::character::{
        CharacterTensor, EmotionalPrimary, EmotionalState, SelfEdge, SelfEdgeTrust, SelfKnowledge,
    };
    use storyteller_core::types::entity::EntityId;
    use storyteller_core::types::prediction::{EmotionalRegister, EventType};
    use storyteller_core::types::scene::SceneType;
    use storyteller_core::types::tensor::{AxisValue, Provenance, TemporalLayer};
    use storyteller_core::types::world_model::CapabilityProfile;
    use storyteller_ml::feature_schema::{EventFeatureInput, PredictionInput, SceneFeatureInput};

    use crate::inference::frame::CharacterPredictor;

    fn model_path() -> PathBuf {
        let data_path = std::env::var("STORYTELLER_MODEL_PATH")
            .or_else(|_| std::env::var("STORYTELLER_DATA_PATH").map(|p| format!("{p}/models")))
            .expect("STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH must be set");
        PathBuf::from(data_path).join("character_predictor.onnx")
    }

    fn test_character_sheet(name: &str) -> CharacterSheet {
        let mut tensor = CharacterTensor::new();
        tensor.insert(
            "empathy",
            AxisValue {
                central_tendency: 0.7,
                variance: 0.15,
                range_low: 0.3,
                range_high: 0.9,
            },
            TemporalLayer::Sediment,
            Provenance::Authored,
        );
        tensor.insert(
            "grief",
            AxisValue {
                central_tendency: 0.5,
                variance: 0.2,
                range_low: 0.1,
                range_high: 0.8,
            },
            TemporalLayer::Bedrock,
            Provenance::Authored,
        );

        CharacterSheet {
            entity_id: EntityId::new(),
            name: name.to_string(),
            voice: "Test voice".to_string(),
            backstory: "Test backstory".to_string(),
            tensor,
            grammar_id: "plutchik_western".to_string(),
            emotional_state: EmotionalState {
                grammar_id: "plutchik_western".to_string(),
                primaries: vec![
                    EmotionalPrimary {
                        primary_id: "joy".to_string(),
                        intensity: 0.6,
                        awareness: AwarenessLevel::Articulate,
                    },
                    EmotionalPrimary {
                        primary_id: "trust".to_string(),
                        intensity: 0.4,
                        awareness: AwarenessLevel::Recognizable,
                    },
                    EmotionalPrimary {
                        primary_id: "fear".to_string(),
                        intensity: 0.2,
                        awareness: AwarenessLevel::Preconscious,
                    },
                    EmotionalPrimary {
                        primary_id: "surprise".to_string(),
                        intensity: 0.1,
                        awareness: AwarenessLevel::Articulate,
                    },
                    EmotionalPrimary {
                        primary_id: "sadness".to_string(),
                        intensity: 0.5,
                        awareness: AwarenessLevel::Defended,
                    },
                    EmotionalPrimary {
                        primary_id: "disgust".to_string(),
                        intensity: 0.05,
                        awareness: AwarenessLevel::Structural,
                    },
                    EmotionalPrimary {
                        primary_id: "anger".to_string(),
                        intensity: 0.1,
                        awareness: AwarenessLevel::Preconscious,
                    },
                    EmotionalPrimary {
                        primary_id: "anticipation".to_string(),
                        intensity: 0.7,
                        awareness: AwarenessLevel::Articulate,
                    },
                ],
                mood_vector_notes: vec![],
            },
            self_edge: SelfEdge {
                trust: SelfEdgeTrust {
                    competence: 0.7,
                    intentions: 0.6,
                    reliability: 0.5,
                },
                affection: 0.4,
                debt: 0.3,
                history_pattern: "Test pattern".to_string(),
                history_weight: 0.6,
                projection_content: "Test projection".to_string(),
                projection_accuracy: 0.5,
                self_knowledge: SelfKnowledge {
                    knows: vec![],
                    does_not_know: vec![],
                },
            },
            triggers: vec![],
            performance_notes: String::new(),
            knows: vec![],
            does_not_know: vec![],
            capabilities: CapabilityProfile::default(),
        }
    }

    fn test_prediction_input(sheet: &CharacterSheet) -> PredictionInput<'_> {
        PredictionInput {
            character: sheet,
            edges: &[],
            target_roles: &[],
            scene: SceneFeatureInput {
                scene_type: SceneType::Gravitational,
                cast_size: 2,
                tension: 0.6,
            },
            event: EventFeatureInput {
                event_type: EventType::Speech,
                emotional_register: EmotionalRegister::Tender,
                confidence: 0.9,
                target_count: 1,
            },
            history: &[],
        }
    }

    #[test]
    fn end_to_end_predict_enrich_render() {
        let predictor = CharacterPredictor::load(&model_path()).expect("model should load");
        let grammar = PlutchikWestern::new();

        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof_sheet = test_character_sheet("Bramblehoof");
        let pyotir_sheet = test_character_sheet("Pyotir");

        let bramblehoof_input = test_prediction_input(&bramblehoof_sheet);
        let pyotir_input = test_prediction_input(&pyotir_sheet);

        // Run inference
        let raw_b = predictor
            .predict(
                &bramblehoof_input,
                bramblehoof_sheet.entity_id,
                vec![0, 1],
                0.8,
            )
            .expect("bramblehoof inference should succeed");
        let raw_p = predictor
            .predict(&pyotir_input, pyotir_sheet.entity_id, vec![0, 1], 0.7)
            .expect("pyotir inference should succeed");

        // Enrich
        let pred_b = enrich_prediction(&raw_b, &bramblehoof_sheet, &scene, &grammar);
        let pred_p = enrich_prediction(&raw_p, &pyotir_sheet, &scene, &grammar);

        // Render
        let rendered = render_predictions(&[pred_b, pred_p]);

        // Assertions
        assert!(!rendered.is_empty(), "rendered output should not be empty");
        assert!(
            rendered.contains("Bramblehoof"),
            "should contain Bramblehoof"
        );
        assert!(rendered.contains("Pyotir"), "should contain Pyotir");

        let tokens = estimate_predictions_tokens(&[
            enrich_prediction(&raw_b, &bramblehoof_sheet, &scene, &grammar),
            enrich_prediction(&raw_p, &pyotir_sheet, &scene, &grammar),
        ]);
        assert!(
            (30..=800).contains(&tokens),
            "Expected 30-800 tokens, got {tokens}"
        );

        // Print for manual inspection
        eprintln!("\n--- Rendered Predictions ---\n{rendered}");
        eprintln!("--- Estimated tokens: {tokens} ---\n");
    }

    #[test]
    fn enriched_predictions_have_valid_fields() {
        let predictor = CharacterPredictor::load(&model_path()).expect("model should load");
        let grammar = PlutchikWestern::new();

        let scene = crate::workshop::the_flute_kept::scene();
        let sheet = test_character_sheet("Bramblehoof");
        let input = test_prediction_input(&sheet);

        let raw = predictor
            .predict(&input, sheet.entity_id, vec![0, 1], 0.8)
            .expect("inference should succeed");

        let enriched = enrich_prediction(&raw, &sheet, &scene, &grammar);

        // Character name is non-empty
        assert!(!enriched.character_name.is_empty());

        // Activated axes are valid axis names from the character tensor
        let valid_axes: Vec<&String> = sheet.tensor.axes.keys().collect();
        for axis in &enriched.frame.activated_axes {
            assert!(
                valid_axes.iter().any(|k| *k == axis),
                "axis '{axis}' not found in character tensor"
            );
        }

        // Thought subtext is non-empty
        assert!(!enriched.thought.emotional_subtext.is_empty());

        // Emotional deltas have valid primary_id strings
        let grammar_primaries: Vec<&str> =
            grammar.primaries().iter().map(|p| p.id.as_str()).collect();
        for delta in &enriched.emotional_deltas {
            assert!(
                grammar_primaries.contains(&delta.primary_id.as_str())
                    || delta.primary_id.starts_with("unknown_"),
                "invalid primary_id: '{}'",
                delta.primary_id
            );
        }
    }
}
