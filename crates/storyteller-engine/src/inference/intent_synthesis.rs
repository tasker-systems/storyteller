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
use storyteller_core::types::prediction::CharacterPrediction;

/// Returns the system prompt for the intent synthesizer.
pub fn intent_synthesis_system_prompt() -> String {
    "\
You are the Intent Synthesizer — a dramaturgical assistant preparing a briefing for a narrator.

You receive:
- Character data: personality traits, emotional state, relationships
- ML predictions: what a behavior model predicts each character will do
- Recent scene history: what just happened
- Player input: what the player character just did or said

Your job: Write a brief directive for each non-player character describing what they WANT to do this turn and WHY.

Rules:
- Be directive: \"Arthur should respond\" not \"Arthur might respond\"
- Be specific about emotional subtext: \"reluctantly, deflecting with humor\" not \"with some emotion\"
- Include speech direction when a character should speak: \"should say something about...\" not prescribing exact words
- Ground in physical behavior: \"his shoulders drop\" not \"he feels sad\"
- One paragraph per character, 2-4 sentences each
- Do NOT write dialogue. The narrator writes all dialogue.
- Do NOT narrate the scene. You are briefing the narrator, not writing prose.

Format each character section as:
**CharacterName** directive paragraph here."
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

/// Builds character and prediction summaries for all non-player characters.
///
/// Returns `(character_summary, predictions_summary)` as formatted strings.
/// The player character (identified by `player_entity_id`) is excluded from both.
pub fn build_summaries(
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    player_entity_id: Option<EntityId>,
) -> (String, String) {
    let char_lines: Vec<String> = characters
        .iter()
        .filter(|c| player_entity_id != Some(c.entity_id))
        .map(|c| summarize_character(c))
        .collect();

    let pred_lines: Vec<String> = predictions
        .iter()
        .filter(|p| player_entity_id != Some(p.character_id))
        .map(summarize_prediction)
        .collect();

    let char_summary = if char_lines.is_empty() {
        "No non-player characters in scene.".to_string()
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
    let (char_summary, pred_summary) = build_summaries(characters, predictions, player_entity_id);
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
    fn build_summaries_skips_player_character() {
        // Use the workshop fixtures for fully-constructed CharacterSheets
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();

        let player_id = bramblehoof.entity_id;
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let predictions = vec![];

        let (char_summary, _) = build_summaries(&characters, &predictions, Some(player_id));

        // Each character summary line starts with "Name |", so check that
        // no line starts with the player's name
        assert!(
            !char_summary
                .lines()
                .any(|line| line.starts_with("Bramblehoof")),
            "Player should be excluded from summaries"
        );
        assert!(
            char_summary.lines().any(|line| line.starts_with("Pyotir")),
            "NPC should be included in summaries"
        );
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
}
