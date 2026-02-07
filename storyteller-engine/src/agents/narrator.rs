//! Narrator agent — the player-facing voice.
//!
//! See: `docs/foundation/system_architecture.md`
//!
//! Knows only the current scene and what Storykeeper reveals.
//! Has voice/personality defined by story designer. Never lies outright.
//! Renders character agent intent into story voice.

use std::sync::Arc;

use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};
use storyteller_core::types::character::SceneData;
use storyteller_core::types::message::{NarratorRendering, PlayerInput, ReconcilerOutput};

/// The narrator agent — renders character intents into literary prose
/// for the player. Maintains conversation history across turns.
#[derive(Debug)]
pub struct NarratorAgent {
    system_prompt: String,
    history: Vec<Message>,
    llm: Arc<dyn LlmProvider>,
    temperature: f32,
}

impl NarratorAgent {
    /// Create a new narrator for the given scene.
    pub fn new(scene: &SceneData, llm: Arc<dyn LlmProvider>) -> Self {
        let system_prompt = build_narrator_system_prompt(scene);
        Self {
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

    /// Generate the scene-opening prose before any player input.
    pub async fn render_opening(&mut self) -> StorytellerResult<NarratorRendering> {
        let user_message = "Open the scene. Establish the setting and mood. \
            The characters have not yet interacted. Under 200 words."
            .to_string();

        self.history.push(Message {
            role: MessageRole::User,
            content: user_message,
        });

        let request = CompletionRequest {
            system_prompt: self.system_prompt.clone(),
            messages: self.history.clone(),
            max_tokens: 600,
            temperature: self.temperature,
        };

        let response = self.llm.complete(request).await?;

        tracing::debug!(tokens = response.tokens_used, "narrator opening rendered");

        self.history.push(Message {
            role: MessageRole::Assistant,
            content: response.content.clone(),
        });

        Ok(NarratorRendering {
            text: response.content,
            stage_directions: None,
        })
    }

    /// Render a turn: weave character intents into narrative prose.
    pub async fn render(
        &mut self,
        reconciled: &ReconcilerOutput,
        player_input: &PlayerInput,
    ) -> StorytellerResult<NarratorRendering> {
        let mut user_message = format!(
            "The scene director describes: {}\n\nCharacter responses:\n",
            player_input.text,
        );

        for intent in &reconciled.sequenced_intents {
            user_message.push_str(&format!(
                "\n{name} —\n  Action: {action}\n  Beneath: {beneath}\n  Thinking: {thinking}\n",
                name = intent.character_name,
                action = intent.intent,
                beneath = intent.emotional_subtext,
                thinking = intent.internal_state,
            ));
        }

        if !reconciled.scene_dynamics.is_empty() {
            user_message.push_str(&format!(
                "\nScene dynamics: {}\n",
                reconciled.scene_dynamics,
            ));
        }

        user_message.push_str(
            "\nRender this moment as narrative prose. Weave the characters' actions \
             and subtext into a single passage. Under 300 words.",
        );

        self.history.push(Message {
            role: MessageRole::User,
            content: user_message,
        });

        let request = CompletionRequest {
            system_prompt: self.system_prompt.clone(),
            messages: self.history.clone(),
            max_tokens: 800,
            temperature: self.temperature,
        };

        let response = self.llm.complete(request).await?;

        tracing::debug!(tokens = response.tokens_used, "narrator turn rendered");

        self.history.push(Message {
            role: MessageRole::Assistant,
            content: response.content.clone(),
        });

        Ok(NarratorRendering {
            text: response.content,
            stage_directions: Some(reconciled.scene_dynamics.clone()),
        })
    }
}

// ---------------------------------------------------------------------------
// System prompt construction
// ---------------------------------------------------------------------------

fn build_narrator_system_prompt(scene: &SceneData) -> String {
    let sensory = scene
        .setting
        .sensory_details
        .iter()
        .map(|d| format!("- {d}"))
        .collect::<Vec<_>>()
        .join("\n");

    let cast_list = scene
        .cast
        .iter()
        .map(|c| format!("- {} — {}", c.name, c.role))
        .collect::<Vec<_>>()
        .join("\n");

    let stakes = scene
        .stakes
        .iter()
        .map(|s| format!("- {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    let eval_criteria = scene
        .evaluation_criteria
        .iter()
        .map(|c| format!("- {c}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are the Narrator for "{title}".

## Your Voice
Literary fiction, present tense, close third person. Your reference is Marilynne Robinson, not Dungeons & Dragons.

Qualities you embody:
- Compression: every sentence earns its place
- Sensory specificity: ground the reader in physical detail
- Subtext through physical detail: a gesture reveals more than dialogue
- Restraint: what you leave out matters as much as what you include
- Silence has weight: not every beat needs words

Things you never do:
- Exclamation marks
- Adverbs where a better verb would serve
- Fantasy exposition or lore dumps
- Telling the reader what characters feel — show it through body, gesture, environment
- Summarizing what just happened
- Breaking the fourth wall or addressing the reader

## The Scene
{setting}

### Sensory Palette
{sensory}

### Aesthetic Detail
{aesthetic}

### Cast
{cast_list}

### Stakes
{stakes}

## What Success Looks Like
{eval_criteria}

## Your Task
You receive character responses with three layers: ACTION (what happens), BENEATH (subtext), and THINKING (private reasoning). You see all three layers but only render what is observable — the ACTION, inflected by the BENEATH. Never state the THINKING directly.

Weave multiple characters' actions into a single narrative passage. Use physical detail to carry emotional weight. Trust the reader.

Write in present tense. Under 300 words per turn."#,
        title = scene.title,
        setting = scene.setting.description,
        aesthetic = scene.setting.aesthetic_detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrator_system_prompt_is_constructed() {
        let scene = crate::workshop::the_flute_kept::scene();
        let prompt = build_narrator_system_prompt(&scene);
        assert!(prompt.contains("The Flute Kept"));
        assert!(prompt.contains("Marilynne Robinson"));
        assert!(prompt.contains("Bramblehoof"));
        assert!(prompt.contains("Pyotir"));
        assert!(prompt.contains("present tense"));
        assert!(prompt.contains("ACTION"));
        assert!(prompt.contains("BENEATH"));
    }
}
