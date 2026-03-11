//! Intent synthesis — dramaturgical bridging layer between ML predictions and narrator.
//!
//! The intent synthesizer receives character data, ML behavior predictions, scene history,
//! and player input, then produces per-character directives that tell the narrator *what
//! each character wants to do this turn and why*.
//!
//! This is a small-LLM call (400 tokens, temperature 0.3) that translates structured
//! prediction data into prose directives the narrator can render.

use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::prediction::{ActionType, CharacterPrediction, SpeechRegister};

/// Returns the system prompt for the intent synthesizer.
pub fn intent_synthesis_system_prompt() -> String {
    "\
You are the Intent Synthesizer — a dramaturgical assistant preparing a briefing for a narrator.

You receive:
- Character data: personality traits, emotional state, relationships
- ML predictions: what a behavior model predicts each character will do
- Recent scene history: what just happened
- Player input: what the player character just did or said

Your job: Write a brief directive for each character. For non-player characters, describe what they WANT to do this turn and WHY. For the player character, describe how the character's nature relates to the directed action.

Rules for non-player characters:
- Be directive: \"Arthur should respond\" not \"Arthur might respond\"
- Be specific about emotional subtext: \"reluctantly, deflecting with humor\" not \"with some emotion\"
- Include speech direction when a character should speak: \"should say something about...\" not prescribing exact words
- Ground in physical behavior: \"his shoulders drop\" not \"he feels sad\"
- One paragraph per character, 2-4 sentences each
- Do NOT write dialogue. The narrator writes all dialogue.
- Do NOT narrate the scene. You are briefing the narrator, not writing prose.

Rules for the player character (marked [PLAYER CHARACTER]):
- The player has directed this character's action. Do NOT override it.
- Describe how this character's personality and emotional state relate to the directed action — whether their nature resists it, inflects it, or suits it. Ground in physical behavior the narrator can render.
- If the directed action is in tension with the character's nature, call this out explicitly — this is how the system nudges players toward authentic characterization without forcing their hand.
- This is advisory — the narrator decides how to weigh it.

Format:

**CharacterName**
[Your directive paragraph for this character.]"
        .to_string()
}

/// Builds the user prompt for intent synthesis from all context components.
pub fn build_intent_user_prompt(
    character_summary: &str,
    predictions_summary: &str,
    journal_tail: &str,
    player_input: &str,
    scene_context: &str,
) -> String {
    format!(
        "\
## Characters
{character_summary}

## ML Predictions
{predictions_summary}

## Recent Scene History
{journal_tail}

## Player Input
{player_input}

## Scene Context
{scene_context}"
    )
}

/// Formats the top `count` tensor axes by central_tendency magnitude as
/// readable English trait names with values.
///
/// Output: `"empathetic 0.78, protective 0.72, grief-stricken 0.65"`
///
/// Axis IDs are converted to readable form: underscores become hyphens.
pub fn format_dominant_axes(sheet: &CharacterSheet, count: usize) -> String {
    let mut axes: Vec<(&String, f32)> = sheet
        .tensor
        .axes
        .iter()
        .map(|(name, entry)| (name, entry.value.central_tendency))
        .collect();

    axes.sort_by(|a, b| {
        b.1.abs()
            .partial_cmp(&a.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    axes.iter()
        .take(count)
        .map(|(name, val)| {
            let readable = axis_id_to_readable(name);
            format!("{readable} {val:.2}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Converts an axis ID to a readable English name. Underscores become hyphens.
fn axis_id_to_readable(id: &str) -> String {
    id.replace('_', "-")
}

/// Converts a `CharacterPrediction` into natural language for the 3b-instruct model.
///
/// Output examples:
/// - `"would most likely examine (85% confidence), unlikely to move (15%)"`
/// - `"would most likely move (70% confidence); likely to speak (conversational, 65% confidence)"`
pub fn format_prediction_readable(pred: &CharacterPrediction) -> String {
    let mut parts = Vec::new();

    let mut actions = pred.actions.clone();
    actions.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (i, action) in actions.iter().enumerate() {
        let action_name = action_type_to_readable(action.action_type);
        let pct = (action.confidence * 100.0).round() as u32;
        if i == 0 {
            parts.push(format!(
                "would most likely {action_name} ({pct}% confidence)"
            ));
        } else if action.confidence < 0.3 {
            parts.push(format!("unlikely to {action_name} ({pct}%)"));
        } else {
            parts.push(format!("might {action_name} ({pct}%)"));
        }
    }

    if let Some(ref speech) = pred.speech {
        let register = speech_register_to_readable(speech.register);
        let pct = (speech.confidence * 100.0).round() as u32;
        parts.push(format!("likely to speak ({register}, {pct}% confidence)"));
    }

    if parts.is_empty() {
        "no strong behavioral prediction".to_string()
    } else {
        parts.join(", ")
    }
}

fn action_type_to_readable(action_type: ActionType) -> &'static str {
    match action_type {
        ActionType::Perform => "act",
        ActionType::Speak => "speak",
        ActionType::Move => "move",
        ActionType::Examine => "examine",
        ActionType::Wait => "wait",
        ActionType::Resist => "resist",
    }
}

fn speech_register_to_readable(register: SpeechRegister) -> &'static str {
    match register {
        SpeechRegister::Whisper => "whispered",
        SpeechRegister::Conversational => "conversational",
        SpeechRegister::Declamatory => "raised voice",
        SpeechRegister::Internal => "internal",
    }
}

/// Produces a compact summary of a character sheet for the intent synthesizer.
///
/// Format: `name | top 4 axes (by central_tendency magnitude) | voice | performance notes`
pub fn summarize_character(sheet: &CharacterSheet) -> String {
    // Sort axes by absolute central_tendency, take top 4
    let mut axes: Vec<(&String, f32)> = sheet
        .tensor
        .axes
        .iter()
        .map(|(name, entry)| (name, entry.value.central_tendency))
        .collect();
    axes.sort_by(|a, b| {
        b.1.abs()
            .partial_cmp(&a.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_axes: Vec<String> = axes
        .iter()
        .take(4)
        .map(|(name, val)| format!("{}={:.2}", name, val))
        .collect();

    format!(
        "{} | {} | voice: {} | {}",
        sheet.name,
        top_axes.join(", "),
        sheet.voice,
        sheet.performance_notes,
    )
}

/// Produces a compact summary of a character prediction for the intent synthesizer.
///
/// Format: `name | actions with confidence | speech (if any) | emotional subtext | emotional shifts`
pub fn summarize_prediction(pred: &CharacterPrediction) -> String {
    let actions: Vec<String> = pred
        .actions
        .iter()
        .map(|a| format!("{:?}({:.2})", a.action_type, a.confidence))
        .collect();
    let actions_str = if actions.is_empty() {
        "no actions".to_string()
    } else {
        actions.join(", ")
    };

    let speech_str = match &pred.speech {
        Some(s) => format!(
            "{:?}({:.2}): {}",
            s.register, s.confidence, s.content_direction
        ),
        None => "silent".to_string(),
    };

    let subtext = &pred.thought.emotional_subtext;

    let shifts: Vec<String> = pred
        .emotional_deltas
        .iter()
        .map(|d| {
            let sign = if d.intensity_change >= 0.0 { "+" } else { "" };
            format!("{}{}{:.2}", d.primary_id, sign, d.intensity_change)
        })
        .collect();
    let shifts_str = if shifts.is_empty() {
        "no shifts".to_string()
    } else {
        shifts.join(", ")
    };

    format!(
        "{} | Actions: {} | Speech: {} | Subtext: {} | Shifts: {}",
        pred.character_name, actions_str, speech_str, subtext, shifts_str,
    )
}

/// Builds character and prediction summaries for all characters in the scene.
///
/// Returns `(character_summary, predictions_summary)` as formatted strings.
/// The player character (identified by `player_entity_id`) is included with a
/// `[PLAYER CHARACTER]` marker, readable dominant traits, and their directed action.
/// NPC predictions are included in the predictions summary; player predictions
/// appear inline in the character summary block.
pub fn build_summaries(
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    player_entity_id: Option<EntityId>,
    player_input: Option<&str>,
) -> (String, String) {
    let mut char_lines: Vec<String> = Vec::new();
    let mut pred_lines: Vec<String> = Vec::new();

    for character in characters {
        let is_player = player_entity_id == Some(character.entity_id);

        if is_player {
            let action_text = player_input.unwrap_or("(no action specified)");
            let traits = format_dominant_axes(character, 5);
            let mut player_block = format!(
                "[PLAYER CHARACTER — directed action: \"{action_text}\"]\n\
                 {} | {} | Dominant traits (0-1 scale): {}",
                character.name, character.performance_notes, traits,
            );

            if let Some(pred) = predictions
                .iter()
                .find(|p| p.character_id == character.entity_id)
            {
                let readable_pred = format_prediction_readable(pred);
                player_block.push_str(&format!("\nML prediction: {readable_pred}"));
            }

            char_lines.push(player_block);
        } else {
            char_lines.push(summarize_character(character));
        }
    }

    for pred in predictions {
        let is_player = player_entity_id == Some(pred.character_id);
        if !is_player {
            pred_lines.push(summarize_prediction(pred));
        }
    }

    let char_summary = if char_lines.is_empty() {
        "No characters in scene.".to_string()
    } else {
        char_lines.join("\n")
    };

    let pred_summary = if pred_lines.is_empty() {
        "No predictions available.".to_string()
    } else {
        pred_lines.join("\n")
    };

    (char_summary, pred_summary)
}

/// Builds scene context from stakes and constraints.
fn build_scene_context(scene: &SceneData) -> String {
    let stakes: Vec<String> = scene.stakes.iter().map(|s| format!("- {s}")).collect();
    let hard: Vec<String> = scene
        .constraints
        .hard
        .iter()
        .map(|c| format!("- {c}"))
        .collect();
    let soft: Vec<String> = scene
        .constraints
        .soft
        .iter()
        .map(|c| format!("- {c}"))
        .collect();

    let mut parts = vec![format!("Stakes:\n{}", stakes.join("\n"))];

    if !hard.is_empty() {
        parts.push(format!("Hard constraints:\n{}", hard.join("\n")));
    }
    if !soft.is_empty() {
        parts.push(format!("Soft constraints:\n{}", soft.join("\n")));
    }

    parts.join("\n\n")
}

/// Synthesizes character intent directives for the narrator.
///
/// Calls the provided LLM with character data, ML predictions, scene history,
/// and player input. Returns `Some(directives)` on success, `None` on failure.
///
/// Uses `max_tokens: 400`, `temperature: 0.3` for concise, deterministic output.
pub async fn synthesize_intents(
    llm: &dyn LlmProvider,
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    journal_tail: &str,
    player_input: &str,
    scene: &SceneData,
    player_entity_id: Option<EntityId>,
) -> Option<String> {
    let (char_summary, pred_summary) = build_summaries(
        characters,
        predictions,
        player_entity_id,
        Some(player_input),
    );
    let scene_context = build_scene_context(scene);

    let user_prompt = build_intent_user_prompt(
        &char_summary,
        &pred_summary,
        journal_tail,
        player_input,
        &scene_context,
    );

    let request = CompletionRequest {
        system_prompt: intent_synthesis_system_prompt(),
        messages: vec![Message {
            role: MessageRole::User,
            content: user_prompt,
        }],
        max_tokens: 400,
        temperature: 0.3,
    };

    match llm.complete(request).await {
        Ok(response) => Some(response.content),
        Err(e) => {
            tracing::warn!("Intent synthesis failed: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_has_key_sections() {
        let prompt = intent_synthesis_system_prompt();
        assert!(prompt.contains("Intent Synthesizer"));
        assert!(prompt.contains("directive"));
        assert!(prompt.contains("Do NOT write dialogue"));
    }

    #[test]
    fn system_prompt_includes_player_character_rules() {
        let prompt = intent_synthesis_system_prompt();
        assert!(
            prompt.contains("PLAYER CHARACTER"),
            "Should mention player character: {prompt}"
        );
        assert!(
            prompt.contains("Do NOT override"),
            "Should instruct not to override player action: {prompt}"
        );
        assert!(
            prompt.contains("advisory"),
            "Should note this is advisory: {prompt}"
        );
    }

    #[test]
    fn user_prompt_includes_all_context() {
        let character_summary = "Arthur: grieving_youth, guarded, empathy=0.6";
        let predictions_summary = "Arthur: Action=Observe(0.85), Speech=Conversational(0.70)";
        let journal_tail = "Margaret arrived at the Old Rectory.";
        let player_input = "Margaret asks how Arthur is holding up.";
        let scene_context = "Stakes: Speech (Tender), Observation (Neutral)";

        let prompt = build_intent_user_prompt(
            character_summary,
            predictions_summary,
            journal_tail,
            player_input,
            scene_context,
        );

        assert!(prompt.contains("Arthur"));
        assert!(prompt.contains("grieving_youth"));
        assert!(prompt.contains("Observe"));
        assert!(prompt.contains("Margaret arrived"));
        assert!(prompt.contains("holding up"));
        assert!(prompt.contains("Speech (Tender)"));
    }

    #[test]
    fn build_summaries_includes_player_with_marker_and_npc_without() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let player_id = bramblehoof.entity_id;
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let predictions = vec![];
        let (char_summary, _) = build_summaries(
            &characters,
            &predictions,
            Some(player_id),
            Some("I approach the fence"),
        );
        assert!(
            char_summary.contains("[PLAYER CHARACTER"),
            "Player should have marker"
        );
        assert!(
            char_summary.contains("Bramblehoof"),
            "Player name should appear"
        );
        assert!(
            char_summary.lines().any(|line| line.starts_with("Pyotir")),
            "NPC should be included in summaries"
        );
    }

    #[test]
    fn build_summaries_includes_player_character_with_marker() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let player_id = bramblehoof.entity_id;
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let predictions = vec![];
        let (char_summary, _) = build_summaries(
            &characters,
            &predictions,
            Some(player_id),
            Some("I charge at the intruder"),
        );
        assert!(
            char_summary.contains("[PLAYER CHARACTER"),
            "Should contain player character marker: {char_summary}"
        );
        assert!(
            char_summary.contains("Bramblehoof"),
            "Player character name should appear: {char_summary}"
        );
        assert!(
            char_summary.contains("directed action"),
            "Should include directed action label: {char_summary}"
        );
        assert!(
            char_summary.contains("charge at the intruder"),
            "Should include the player's input: {char_summary}"
        );
        assert!(
            char_summary.contains("Pyotir"),
            "NPC should be included: {char_summary}"
        );
    }

    #[test]
    fn build_summaries_includes_dominant_traits_for_player() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let player_id = bramblehoof.entity_id;
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let predictions = vec![];
        let (char_summary, _) = build_summaries(
            &characters,
            &predictions,
            Some(player_id),
            Some("I look around"),
        );
        assert!(
            char_summary.contains("Dominant traits (0-1 scale):"),
            "Should label the trait scale: {char_summary}"
        );
    }

    #[test]
    fn build_summaries_backward_compat_no_player() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let predictions = vec![];
        let (char_summary, _) = build_summaries(&characters, &predictions, None, None);
        assert!(
            !char_summary.contains("[PLAYER CHARACTER"),
            "Should not have player marker when no player_entity_id"
        );
        assert!(char_summary.contains("Bramblehoof"));
        assert!(char_summary.contains("Pyotir"));
    }

    #[test]
    fn summarize_character_includes_top_axes() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let summary = summarize_character(&bramblehoof);

        // Should contain the character name and voice info
        assert!(summary.contains("Bramblehoof"));
        assert!(summary.contains("voice:"));
        // Should contain axis values (formatted as name=value)
        assert!(summary.contains('='));
    }

    #[test]
    fn summarize_prediction_formats_all_fields() {
        use storyteller_core::types::prediction::*;
        use storyteller_core::types::tensor::AwarenessLevel;

        let pred = CharacterPrediction {
            character_id: EntityId::new(),
            character_name: "Arthur".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec!["grief".to_string()],
                activation_reason: "loss context".to_string(),
                confidence: 0.8,
            },
            actions: vec![ActionPrediction {
                description: "Observes Margaret carefully".to_string(),
                confidence: 0.85,
                action_type: ActionType::Examine,
                target: None,
            }],
            speech: Some(SpeechPrediction {
                content_direction: "deflecting with humor".to_string(),
                register: SpeechRegister::Conversational,
                confidence: 0.70,
            }),
            thought: ThoughtPrediction {
                emotional_subtext: "guarded hope beneath grief".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: None,
            },
            emotional_deltas: vec![EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.15,
                awareness_change: None,
            }],
        };

        let summary = summarize_prediction(&pred);
        assert!(summary.contains("Arthur"));
        assert!(summary.contains("Examine"));
        assert!(summary.contains("Conversational"));
        assert!(summary.contains("guarded hope"));
        assert!(summary.contains("joy"));
    }

    #[test]
    fn build_scene_context_includes_stakes_and_constraints() {
        let scene = crate::workshop::the_flute_kept::scene();
        let context = build_scene_context(&scene);

        assert!(context.contains("Stakes:"));
        // Scene has constraints, so at least one constraint section should appear
        assert!(context.contains("Hard constraints:") || context.contains("Soft constraints:"));
    }

    #[test]
    fn format_dominant_axes_sorts_by_magnitude_and_formats_readable() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let result = format_dominant_axes(&bramblehoof, 3);
        assert!(!result.is_empty());
        let parts: Vec<&str> = result.split(", ").collect();
        assert_eq!(parts.len(), 3, "Expected 3 axes, got: {result}");
        for part in &parts {
            let tokens: Vec<&str> = part.split_whitespace().collect();
            assert_eq!(tokens.len(), 2, "Expected 'name value', got: {part}");
            tokens[1].parse::<f32>().expect("Value should be a float");
        }
    }

    #[test]
    fn format_dominant_axes_uses_readable_names() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let result = format_dominant_axes(&bramblehoof, 5);
        assert!(
            !result.contains('_'),
            "Should use readable names without underscores: {result}"
        );
    }

    #[test]
    fn format_dominant_axes_respects_count_limit() {
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let result_2 = format_dominant_axes(&bramblehoof, 2);
        let result_5 = format_dominant_axes(&bramblehoof, 5);
        let count_2 = result_2.split(", ").count();
        let count_5 = result_5.split(", ").count();
        assert_eq!(count_2, 2);
        assert!(count_5 <= 5);
        assert!(count_5 > count_2);
    }

    #[test]
    fn format_prediction_readable_includes_action_and_confidence() {
        use storyteller_core::types::prediction::*;
        use storyteller_core::types::tensor::AwarenessLevel;

        let pred = CharacterPrediction {
            character_id: EntityId::new(),
            character_name: "Arthur".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec!["grief".to_string()],
                activation_reason: "loss context".to_string(),
                confidence: 0.8,
            },
            actions: vec![ActionPrediction {
                description: "Observes carefully".to_string(),
                confidence: 0.85,
                action_type: ActionType::Examine,
                target: None,
            }],
            speech: None,
            thought: ThoughtPrediction {
                emotional_subtext: "guarded hope".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: None,
            },
            emotional_deltas: vec![],
        };

        let result = format_prediction_readable(&pred);
        assert!(
            result.contains("examine"),
            "Should mention action type: {result}"
        );
        assert!(
            result.contains("85%"),
            "Should include confidence: {result}"
        );
    }

    #[test]
    fn format_prediction_readable_includes_speech_when_present() {
        use storyteller_core::types::prediction::*;
        use storyteller_core::types::tensor::AwarenessLevel;

        let pred = CharacterPrediction {
            character_id: EntityId::new(),
            character_name: "Arthur".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec![],
                activation_reason: "test".to_string(),
                confidence: 0.8,
            },
            actions: vec![ActionPrediction {
                description: "Approaches".to_string(),
                confidence: 0.70,
                action_type: ActionType::Move,
                target: None,
            }],
            speech: Some(SpeechPrediction {
                content_direction: "deflecting with humor".to_string(),
                register: SpeechRegister::Conversational,
                confidence: 0.65,
            }),
            thought: ThoughtPrediction {
                emotional_subtext: "nervous".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: None,
            },
            emotional_deltas: vec![],
        };

        let result = format_prediction_readable(&pred);
        assert!(result.contains("speak"), "Should mention speech: {result}");
        assert!(
            result.contains("conversational"),
            "Should mention register: {result}"
        );
        assert!(
            result.contains("65%"),
            "Should include speech confidence: {result}"
        );
    }

    #[test]
    fn format_prediction_readable_handles_multiple_actions() {
        use storyteller_core::types::prediction::*;
        use storyteller_core::types::tensor::AwarenessLevel;

        let pred = CharacterPrediction {
            character_id: EntityId::new(),
            character_name: "Arthur".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec![],
                activation_reason: "test".to_string(),
                confidence: 0.8,
            },
            actions: vec![
                ActionPrediction {
                    description: "Observes".to_string(),
                    confidence: 0.85,
                    action_type: ActionType::Examine,
                    target: None,
                },
                ActionPrediction {
                    description: "Moves".to_string(),
                    confidence: 0.15,
                    action_type: ActionType::Move,
                    target: None,
                },
            ],
            speech: None,
            thought: ThoughtPrediction {
                emotional_subtext: "calm".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: None,
            },
            emotional_deltas: vec![],
        };

        let result = format_prediction_readable(&pred);
        assert!(
            result.contains("most likely"),
            "Should identify primary: {result}"
        );
    }
}
