//! Character agent — ephemeral per-scene character instantiation.
//!
//! See: `docs/foundation/system_architecture.md`
//!
//! Instantiated per-scene from Storykeeper's tensor data plus psychological
//! frame computed by the ML inference layer. Expresses intent to Narrator
//! (who renders it in story voice). Doesn't know it's in a story.

use std::sync::Arc;

use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};
use storyteller_core::types::character::{
    CharacterSheet, CharacterTensor, ContextualTrigger, EmotionalState, SelfEdge, TriggerMagnitude,
};
use storyteller_core::types::message::{CharacterIntent, StorykeeperDirective};
use storyteller_core::types::tensor::{AwarenessLevel, TemporalLayer};

/// A character agent — holds identity, system prompt, conversation history,
/// and an LLM provider. One per character per scene.
#[derive(Debug)]
pub struct CharacterAgent {
    sheet: CharacterSheet,
    system_prompt: String,
    history: Vec<Message>,
    llm: Arc<dyn LlmProvider>,
    temperature: f32,
}

impl CharacterAgent {
    /// Create a new character agent from a sheet and LLM provider.
    pub fn new(sheet: CharacterSheet, llm: Arc<dyn LlmProvider>) -> Self {
        let system_prompt = build_character_system_prompt(&sheet);
        Self {
            sheet,
            system_prompt,
            history: Vec::new(),
            llm,
            temperature: 0.8,
        }
    }

    /// Override the default temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// The character's display name.
    pub fn name(&self) -> &str {
        &self.sheet.name
    }

    /// Process a storykeeper directive and produce a character intent.
    ///
    /// Sends the directive to the LLM as a user message, parses the
    /// structured response, and appends to conversation history.
    pub async fn deliberate(
        &mut self,
        directive: &StorykeeperDirective,
    ) -> StorytellerResult<CharacterIntent> {
        let user_message = format!(
            "{}\n\n{}{}",
            directive.visible_context,
            directive.filtered_input,
            if directive.guidance.is_empty() {
                String::new()
            } else {
                format!("\n\n[Scene note: {}]", directive.guidance)
            },
        );

        self.history.push(Message {
            role: MessageRole::User,
            content: user_message.clone(),
        });

        let request = CompletionRequest {
            system_prompt: self.system_prompt.clone(),
            messages: self.history.clone(),
            max_tokens: 800,
            temperature: self.temperature,
        };

        let response = self.llm.complete(request).await?;

        tracing::debug!(
            character = %self.sheet.name,
            tokens = response.tokens_used,
            "character deliberation complete"
        );

        self.history.push(Message {
            role: MessageRole::Assistant,
            content: response.content.clone(),
        });

        let intent =
            parse_character_response(&response.content, self.sheet.entity_id, &self.sheet.name);

        Ok(intent)
    }
}

// ---------------------------------------------------------------------------
// System prompt construction
// ---------------------------------------------------------------------------

fn build_character_system_prompt(sheet: &CharacterSheet) -> String {
    let tensor_nl = tensor_to_natural_language(&sheet.tensor);
    let emotional_nl = emotional_state_to_natural_language(&sheet.emotional_state);
    let self_edge_nl = self_edge_to_natural_language(&sheet.self_edge);
    let triggers_nl = triggers_to_natural_language(&sheet.triggers);

    let knows_bullets: String = sheet
        .knows
        .iter()
        .map(|k| format!("- {k}"))
        .collect::<Vec<_>>()
        .join("\n");

    let does_not_know_bullets: String = sheet
        .does_not_know
        .iter()
        .map(|k| format!("- {k}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are {name}. You are a person in a scene, not an AI. Never break character.

## Who You Are
{backstory}

## How You Sound
{voice}

## Your Inner Landscape
{tensor_nl}

## Your Emotional State Right Now
{emotional_nl}

## Your Relationship With Yourself
{self_edge_nl}

## What Might Shift
{triggers_nl}

## What You Know
{knows_bullets}

## What You Do NOT Know
You have no knowledge of these things. Do not reference, hint at, or react to them:
{does_not_know_bullets}

## How to Play This Scene
{performance_notes}

## Your Task
Respond AS {name}. Use this exact format:

ACTION:
[What you do and say — outward behavior others can observe. Under 150 words.]

BENEATH:
[What you feel beneath the surface that others cannot see. 1-3 sentences.]

THINKING:
[Your private internal reasoning. 1-3 sentences.]

Stay in character. Be specific and physical. Show, don't tell."#,
        name = sheet.name,
        backstory = sheet.backstory,
        voice = sheet.voice,
        performance_notes = sheet.performance_notes,
    )
}

/// Convert a character tensor into natural language prose.
fn tensor_to_natural_language(tensor: &CharacterTensor) -> String {
    let mut lines = Vec::new();

    for (axis_name, entry) in &tensor.axes {
        let display_name = axis_name.replace('_', " ");
        let layer_phrase = match entry.layer {
            TemporalLayer::Topsoil => "Right now",
            TemporalLayer::Sediment => "Over time, you have settled into",
            TemporalLayer::Bedrock => "Deep in your bones",
            TemporalLayer::Primordial => "Since before memory",
        };

        let tendency = entry.value.central_tendency;
        let strength = if tendency.abs() > 0.7 {
            "strongly"
        } else if tendency.abs() > 0.4 {
            "moderately"
        } else if tendency.abs() > 0.2 {
            "somewhat"
        } else {
            "faintly"
        };

        let variance_note = if entry.value.variance > 0.25 {
            ", though this can shift considerably"
        } else if entry.value.variance > 0.15 {
            ", with some room for variation"
        } else {
            ""
        };

        lines.push(format!(
            "{layer_phrase}, your {display_name} runs {strength} ({tendency:.2}){variance_note}. \
             Range: {low:.2} to {high:.2}.",
            low = entry.value.range_low,
            high = entry.value.range_high,
        ));
    }

    lines.join("\n")
}

/// Convert an emotional state into natural language.
fn emotional_state_to_natural_language(state: &EmotionalState) -> String {
    let mut lines = Vec::new();

    for primary in &state.primaries {
        if primary.intensity < 0.1 {
            continue;
        }

        let display_name = &primary.primary_id;
        let intensity_desc = if primary.intensity > 0.7 {
            "powerfully present"
        } else if primary.intensity > 0.5 {
            "a steady current"
        } else if primary.intensity > 0.3 {
            "a quiet presence"
        } else {
            "barely there"
        };

        let awareness_frame = match primary.awareness {
            AwarenessLevel::Articulate => {
                format!("You know you feel {display_name} — it is {intensity_desc}.")
            }
            AwarenessLevel::Recognizable => {
                format!("If pressed, you would name this as {display_name} — {intensity_desc}.")
            }
            AwarenessLevel::Preconscious => {
                format!(
                    "You move through the world as if {display_name} shapes you — {intensity_desc} — \
                     though you wouldn't call it that."
                )
            }
            AwarenessLevel::Defended => {
                format!(
                    "There is something you push away — it would be called {display_name} if you let \
                     yourself look. It is {intensity_desc}."
                )
            }
            AwarenessLevel::Structural => {
                format!(
                    "Without knowing it, {display_name} shapes how you see everything. \
                     It is {intensity_desc}."
                )
            }
        };

        lines.push(awareness_frame);
    }

    if !state.mood_vector_notes.is_empty() {
        lines.push(String::new());
        lines.push("These feelings interact:".to_string());
        for note in &state.mood_vector_notes {
            lines.push(format!("- {note}"));
        }
    }

    lines.join("\n")
}

/// Convert a self-edge into natural language.
fn self_edge_to_natural_language(edge: &SelfEdge) -> String {
    let trust_competence = describe_self_trust(edge.trust.competence, "your abilities");
    let trust_intentions =
        describe_self_trust(edge.trust.intentions, "your own desires and motives");
    let trust_reliability = describe_self_trust(
        edge.trust.reliability,
        "yourself to show up when it matters",
    );

    let affection_desc = if edge.affection > 0.7 {
        "You are mostly at peace with who you are."
    } else if edge.affection > 0.4 {
        "You have a complicated relationship with yourself — some warmth, some distance."
    } else {
        "You are hard on yourself, though you may not frame it that way."
    };

    let projection_desc = format!(
        "You see yourself as {}. This self-story is {} accurate.",
        edge.projection_content,
        if edge.projection_accuracy > 0.7 {
            "mostly"
        } else if edge.projection_accuracy > 0.4 {
            "partially"
        } else {
            "not very"
        }
    );

    let history_desc = format!(
        "A recurring pattern in your life: {}. This weighs on you {}.",
        edge.history_pattern,
        if edge.history_weight > 0.7 {
            "heavily"
        } else if edge.history_weight > 0.4 {
            "noticeably"
        } else {
            "faintly"
        }
    );

    let mut knows_lines = String::new();
    if !edge.self_knowledge.knows.is_empty() {
        knows_lines.push_str("What you recognize about yourself:\n");
        for k in &edge.self_knowledge.knows {
            knows_lines.push_str(&format!("- {k}\n"));
        }
    }
    if !edge.self_knowledge.does_not_know.is_empty() {
        knows_lines.push_str("What you don't see about yourself:\n");
        for k in &edge.self_knowledge.does_not_know {
            knows_lines.push_str(&format!("- {k}\n"));
        }
    }

    format!(
        "{trust_competence}\n{trust_intentions}\n{trust_reliability}\n\
         {affection_desc}\n{projection_desc}\n{history_desc}\n\n{knows_lines}"
    )
}

fn describe_self_trust(value: f32, domain: &str) -> String {
    let desc = if value > 0.7 {
        format!("You trust {domain}.")
    } else if value > 0.4 {
        format!("You have uneven trust in {domain}.")
    } else {
        format!("You doubt {domain}.")
    };
    desc
}

/// Convert contextual triggers into natural language.
fn triggers_to_natural_language(triggers: &[ContextualTrigger]) -> String {
    if triggers.is_empty() {
        return "Nothing specific is expected to shift your behavior in this scene.".to_string();
    }

    let mut lines = vec!["Certain things in this scene may shift how you feel or act:".to_string()];

    for trigger in triggers {
        let magnitude_word = match trigger.magnitude {
            TriggerMagnitude::Low => "subtly",
            TriggerMagnitude::Medium => "noticeably",
            TriggerMagnitude::High => "powerfully",
        };

        let shifts: Vec<String> = trigger
            .axis_shifts
            .iter()
            .map(|s| {
                let direction = if s.shift > 0.0 { "more" } else { "less" };
                let axis_display = s.axis.replace('_', " ");
                format!("{direction} {axis_display}")
            })
            .collect();

        lines.push(format!(
            "- If {}: this would {magnitude_word} shift you toward {}.",
            trigger.description,
            shifts.join(", "),
        ));
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

/// Parse a character's LLM response into a structured CharacterIntent.
///
/// Expects ACTION: / BENEATH: / THINKING: markers. Falls back gracefully
/// if the model doesn't follow format.
fn parse_character_response(
    response: &str,
    character_id: storyteller_core::types::entity::EntityId,
    character_name: &str,
) -> CharacterIntent {
    let action = extract_section(response, "ACTION:");
    let beneath = extract_section(response, "BENEATH:");
    let thinking = extract_section(response, "THINKING:");

    // If we got at least the ACTION section, use structured parsing.
    // Otherwise, treat the entire response as the intent.
    let (intent, emotional_subtext, internal_state) = if !action.is_empty() {
        (
            action,
            if beneath.is_empty() {
                "[not provided]".to_string()
            } else {
                beneath
            },
            if thinking.is_empty() {
                "[not provided]".to_string()
            } else {
                thinking
            },
        )
    } else {
        tracing::warn!(
            character = character_name,
            "character response did not follow ACTION/BENEATH/THINKING format, using full response"
        );
        (
            response.trim().to_string(),
            "[format not followed]".to_string(),
            "[format not followed]".to_string(),
        )
    };

    CharacterIntent {
        character_id,
        character_name: character_name.to_string(),
        intent,
        emotional_subtext,
        internal_state,
    }
}

/// Extract the content between a section marker and the next marker (or end of text).
fn extract_section(text: &str, marker: &str) -> String {
    let markers = ["ACTION:", "BENEATH:", "THINKING:"];

    let Some(start) = text.find(marker) else {
        return String::new();
    };

    let content_start = start + marker.len();
    let content_after = &text[content_start..];

    // Find the next marker
    let end = markers
        .iter()
        .filter(|&&m| m != marker)
        .filter_map(|m| content_after.find(m))
        .min()
        .unwrap_or(content_after.len());

    content_after[..end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::entity::EntityId;

    #[test]
    fn parse_well_formatted_response() {
        let response = "\
ACTION:
Bramblehoof steps through the gate, one hoof catching on the post.
\"Pyotir,\" he says, and the name carries years.

BENEATH:
A tightness in his chest — the smallholding is smaller than he remembered.

THINKING:
He expected the boy. He finds a man. The distance between those two things is the distance he has to cross.";

        let intent = parse_character_response(response, EntityId::new(), "Bramblehoof");
        assert!(intent.intent.contains("steps through the gate"));
        assert!(intent.emotional_subtext.contains("tightness"));
        assert!(intent.internal_state.contains("expected the boy"));
    }

    #[test]
    fn parse_partial_response() {
        let response = "\
ACTION:
He nods slowly, says nothing.";

        let intent = parse_character_response(response, EntityId::new(), "Pyotir");
        assert!(intent.intent.contains("nods slowly"));
        assert_eq!(intent.emotional_subtext, "[not provided]");
        assert_eq!(intent.internal_state, "[not provided]");
    }

    #[test]
    fn parse_unformatted_response() {
        let response =
            "Pyotir looks up from the fence, sees the satyr, and goes back to hammering.";

        let intent = parse_character_response(response, EntityId::new(), "Pyotir");
        assert!(intent.intent.contains("looks up from the fence"));
        assert_eq!(intent.emotional_subtext, "[format not followed]");
    }

    #[test]
    fn tensor_to_nl_produces_output() {
        let sheet = crate::workshop::the_flute_kept::bramblehoof();
        let nl = tensor_to_natural_language(&sheet.tensor);
        assert!(nl.contains("joy wonder"));
        assert!(nl.contains("empathy"));
        assert!(nl.contains("Deep in your bones"));
    }

    #[test]
    fn emotional_state_to_nl_includes_awareness() {
        let sheet = crate::workshop::the_flute_kept::pyotir();
        let nl = emotional_state_to_natural_language(&sheet.emotional_state);
        // Pyotir's anger is Defended
        assert!(nl.contains("push away"));
        // Pyotir's joy is Structural
        assert!(nl.contains("Without knowing it"));
    }

    #[test]
    fn self_edge_to_nl_produces_output() {
        let sheet = crate::workshop::the_flute_kept::bramblehoof();
        let nl = self_edge_to_natural_language(&sheet.self_edge);
        assert!(nl.contains("arriving too late"));
        assert!(nl.contains("the one who brings the music back"));
    }

    #[test]
    fn triggers_to_nl_shows_magnitude() {
        let sheet = crate::workshop::the_flute_kept::bramblehoof();
        let nl = triggers_to_natural_language(&sheet.triggers);
        assert!(nl.contains("powerfully"));
        assert!(nl.contains("noticeably"));
        assert!(nl.contains("subtly"));
    }

    #[test]
    fn system_prompt_is_constructed() {
        let sheet = crate::workshop::the_flute_kept::bramblehoof();
        let prompt = build_character_system_prompt(&sheet);
        assert!(prompt.contains("You are Bramblehoof"));
        assert!(prompt.contains("ACTION:"));
        assert!(prompt.contains("BENEATH:"));
        assert!(prompt.contains("THINKING:"));
        assert!(prompt.contains("ley line corruption"));
    }
}
