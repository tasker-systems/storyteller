//! Composition-time intention generation — transforms active goals + lexicon
//! fragments + scene context into concrete situational intentions via LLM.
//!
//! Called once at scene setup. Uses qwen2.5:14b-instruct (or configurable)
//! via the same Ollama infrastructure as intent synthesis.
//!
//! See: `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md`

use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::narrator_context::{CharacterDrive, SceneDirection};

use crate::scene_composer::goals::ComposedGoals;

/// Generated intentions ready for preamble injection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeneratedIntentions {
    pub scene_intention: SceneIntention,
    pub character_intentions: Vec<CharacterIntention>,
}

/// Scene-level dramatic intention.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneIntention {
    pub dramatic_tension: String,
    pub trajectory: String,
}

/// Per-character concrete intention.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterIntention {
    pub character: String,
    pub objective: String,
    pub constraint: String,
    pub behavioral_stance: String,
}

/// Build the system prompt for intention generation.
pub fn intention_system_prompt() -> String {
    r#"You are a dramaturgical advisor for an interactive narrative engine. Your job is to generate concrete situational intentions for characters in a scene.

Rules:
- Each character's objective MUST reference physical objects, locations, or spatial relationships from the setting
- Objectives should create inter-character tension where characters' pursuits naturally complicate each other
- Constraints should arise from other characters' natural behavior, not arbitrary obstacles
- Behavioral stance describes HOW the character pursues their objective — manner, tactics, observable behavior
- The dramatic tension should describe the specific situation, not abstract themes
- The trajectory should describe the moment the scene is building toward

Respond with valid JSON matching this exact structure:
{
  "scene_intention": {
    "dramatic_tension": "1-3 sentences describing the specific dramatic situation",
    "trajectory": "1-2 sentences describing what moment the scene builds toward"
  },
  "character_intentions": [
    {
      "character": "Character Name",
      "objective": "What they are concretely trying to do, grounded in setting",
      "constraint": "What makes it hard — usually another character's behavior",
      "behavioral_stance": "How they pursue it — manner, tactics, observable behavior"
    }
  ]
}"#
    .to_string()
}

/// Build the user prompt from goals, fragments, and scene context.
pub fn build_intention_prompt(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    composed_goals: &ComposedGoals,
) -> String {
    let mut prompt = String::new();

    // Scene context
    prompt.push_str("Scene Context:\n");
    prompt.push_str(&format!("- Setting: {}\n", scene.setting.description));
    if !scene.setting.affordances.is_empty() {
        prompt.push_str(&format!(
            "- Setting affordances: {}\n",
            scene.setting.affordances.join(", ")
        ));
    }
    if !scene.setting.sensory_details.is_empty() {
        prompt.push_str(&format!(
            "- Setting sensory details: {}\n",
            scene.setting.sensory_details.join(", ")
        ));
    }
    prompt.push('\n');

    // Scene goals
    if !composed_goals.scene_goals.is_empty() {
        prompt.push_str("Scene Goals:\n");
        for sg in &composed_goals.scene_goals {
            prompt.push_str(&format!("- {} ({:?})\n", sg.goal_id, sg.visibility));
            for f in &sg.fragments {
                prompt.push_str(&format!("  \"{}\"\n", f.text));
            }
        }
        prompt.push('\n');
    }

    // Cast and character goals
    prompt.push_str("Cast and Character Goals:\n");
    for character in characters {
        let entity_goals = composed_goals.character_goals.get(&character.entity_id);
        let role = scene
            .cast
            .iter()
            .find(|c| c.entity_id == character.entity_id)
            .map(|c| c.role.as_str())
            .unwrap_or("unknown");
        prompt.push_str(&format!("- {} ({}): ", character.name, role));
        if let Some(goals) = entity_goals {
            let goal_ids: Vec<&str> = goals.iter().map(|g| g.goal_id.as_str()).collect();
            prompt.push_str(&goal_ids.join(", "));
            prompt.push('\n');
            for g in goals {
                for f in &g.fragments {
                    prompt.push_str(&format!("  \"{}\"\n", f.text));
                }
            }
        } else {
            prompt.push_str("(no specific goals)\n");
        }
    }

    prompt
}

/// Generate concrete situational intentions via LLM.
///
/// Returns `None` on LLM failure (graceful degradation — scene proceeds without goals).
pub async fn generate_intentions(
    llm: &dyn LlmProvider,
    scene: &SceneData,
    characters: &[&CharacterSheet],
    composed_goals: &ComposedGoals,
) -> Option<GeneratedIntentions> {
    let system = intention_system_prompt();
    let user = build_intention_prompt(scene, characters, composed_goals);

    let request = CompletionRequest {
        system_prompt: system,
        messages: vec![Message {
            role: MessageRole::User,
            content: user,
        }],
        max_tokens: 800,
        temperature: 0.7,
    };

    let response = match llm.complete(request).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Intention generation LLM call failed: {e}");
            return None;
        }
    };

    parse_intentions(&response.content)
}

/// Parse the LLM JSON output into structured intentions.
fn parse_intentions(text: &str) -> Option<GeneratedIntentions> {
    // Try direct parse first
    if let Ok(intentions) = serde_json::from_str::<GeneratedIntentions>(text) {
        return Some(intentions);
    }

    // Try extracting JSON from markdown code blocks
    let json_str = extract_json_block(text)?;
    match serde_json::from_str::<GeneratedIntentions>(json_str) {
        Ok(intentions) => Some(intentions),
        Err(e) => {
            tracing::warn!(
                "Failed to parse intention JSON: {e}\nRaw LLM output (first 500 chars): {}",
                &text[..text.len().min(500)]
            );
            None
        }
    }
}

/// Extract JSON from markdown code fences if present.
fn extract_json_block(text: &str) -> Option<&str> {
    let start = text.find('{');
    let end = text.rfind('}');
    match (start, end) {
        (Some(s), Some(e)) if s < e => Some(&text[s..=e]),
        _ => None,
    }
}

/// Convert generated intentions into preamble types.
pub fn intentions_to_preamble(
    intentions: &GeneratedIntentions,
) -> (SceneDirection, Vec<CharacterDrive>) {
    let direction = SceneDirection {
        dramatic_tension: intentions.scene_intention.dramatic_tension.clone(),
        trajectory: intentions.scene_intention.trajectory.clone(),
    };

    let drives = intentions
        .character_intentions
        .iter()
        .map(|ci| CharacterDrive {
            name: ci.character.clone(),
            objective: ci.objective.clone(),
            constraint: ci.constraint.clone(),
            behavioral_stance: ci.behavioral_stance.clone(),
        })
        .collect();

    (direction, drives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_json() {
        let json = r#"{
            "scene_intention": {
                "dramatic_tension": "Arthur came for a letter.",
                "trajectory": "Toward trust or betrayal."
            },
            "character_intentions": [
                {
                    "character": "Arthur",
                    "objective": "Get the letter.",
                    "constraint": "Margaret is near the mantel.",
                    "behavioral_stance": "Polite deflection."
                }
            ]
        }"#;

        let result = parse_intentions(json);
        assert!(result.is_some());
        let intentions = result.unwrap();
        assert_eq!(intentions.character_intentions.len(), 1);
        assert_eq!(intentions.character_intentions[0].character, "Arthur");
    }

    #[test]
    fn parse_json_from_code_block() {
        let text = "Here are the intentions:\n```json\n{\"scene_intention\":{\"dramatic_tension\":\"Test.\",\"trajectory\":\"Test.\"},\"character_intentions\":[]}\n```";
        let result = parse_intentions(text);
        assert!(result.is_some());
    }

    #[test]
    fn parse_garbage_returns_none() {
        let result = parse_intentions("This is not JSON at all");
        assert!(result.is_none());
    }

    #[test]
    fn intentions_convert_to_preamble_types() {
        let intentions = GeneratedIntentions {
            scene_intention: SceneIntention {
                dramatic_tension: "Test tension.".to_string(),
                trajectory: "Test trajectory.".to_string(),
            },
            character_intentions: vec![CharacterIntention {
                character: "Arthur".to_string(),
                objective: "Get the letter.".to_string(),
                constraint: "Margaret is near.".to_string(),
                behavioral_stance: "Polite deflection.".to_string(),
            }],
        };

        let (direction, drives) = intentions_to_preamble(&intentions);
        assert_eq!(direction.dramatic_tension, "Test tension.");
        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].name, "Arthur");
    }

    #[test]
    fn system_prompt_contains_json_schema() {
        let prompt = intention_system_prompt();
        assert!(prompt.contains("scene_intention"));
        assert!(prompt.contains("character_intentions"));
        assert!(prompt.contains("objective"));
        assert!(prompt.contains("constraint"));
        assert!(prompt.contains("behavioral_stance"));
    }

    #[test]
    fn user_prompt_includes_scene_and_goals() {
        use storyteller_core::types::character::*;
        use storyteller_core::types::scene::{SceneId, SceneType};

        let scene = SceneData {
            scene_id: SceneId::new(),
            title: "Test Scene".to_string(),
            scene_type: SceneType::Gravitational,
            setting: SceneSetting {
                description: "A quiet rectory".to_string(),
                affordances: vec![
                    "Mantel".to_string(),
                    "Tea caddy".to_string(),
                    "Window".to_string(),
                ],
                sensory_details: vec!["Ticking clock".to_string(), "Lavender".to_string()],
                aesthetic_detail: "Faded wallpaper with roses".to_string(),
            },
            cast: Vec::new(),
            stakes: Vec::new(),
            constraints: SceneConstraints {
                hard: Vec::new(),
                soft: Vec::new(),
                perceptual: Vec::new(),
            },
            emotional_arc: Vec::new(),
            evaluation_criteria: Vec::new(),
        };

        let composed = ComposedGoals::default();
        let characters: Vec<&CharacterSheet> = Vec::new();

        let prompt = build_intention_prompt(&scene, &characters, &composed);
        assert!(prompt.contains("A quiet rectory"));
        assert!(prompt.contains("Mantel"));
    }
}
