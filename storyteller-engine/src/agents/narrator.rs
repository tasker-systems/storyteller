//! Narrator agent — the player-facing voice.
//!
//! See: `docs/foundation/system_architecture.md`, `docs/technical/narrator-architecture.md`
//!
//! In the narrator-centric architecture, the Narrator is the ONLY LLM agent.
//! It receives structured context from the three-tier assembly system and
//! renders literary prose.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;

use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};
use storyteller_core::traits::phase_observer::{PhaseEvent, PhaseEventDetail, PhaseObserver};
use storyteller_core::types::message::NarratorRendering;
use storyteller_core::types::narrator_context::NarratorContextInput;
use storyteller_core::types::turn_cycle::TurnCycleStage;

use crate::context::journal::render_journal;
use crate::context::preamble::render_preamble;

/// The narrator agent — renders character intents into literary prose
/// for the player. Each turn is a one-shot LLM call; the three-tier
/// context assembly provides all continuity. The narrator doesn't
/// remember — the system remembers and provides on demand.
#[derive(Debug)]
pub struct NarratorAgent {
    system_prompt: String,
    llm: Arc<dyn LlmProvider>,
    temperature: f32,
}

impl NarratorAgent {
    /// Create a narrator from structured context (narrator-centric architecture).
    ///
    /// Uses the pre-assembled `PersistentPreamble` from the context assembly
    /// pipeline. The preamble is rendered into the system prompt.
    pub fn new(context: &NarratorContextInput, llm: Arc<dyn LlmProvider>) -> Self {
        let system_prompt = build_system_prompt(context);
        Self {
            system_prompt,
            llm,
            temperature: 0.8,
        }
    }

    /// Override the default temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Render a turn from assembled three-tier context.
    ///
    /// Each turn is a one-shot LLM call. The Narrator receives structured
    /// facts with emotional annotation, not prose to parrot. All continuity
    /// comes from the three-tier context assembly:
    ///
    /// 1. System prompt (from Tier 1 preamble)
    /// 2. Scene journal (Tier 2, rendered as structured narrative record)
    /// 3. Retrieved context (Tier 3, rendered as annotated facts)
    /// 4. Current turn data (resolver output + player input)
    pub async fn render(
        &self,
        context: &NarratorContextInput,
        observer: &dyn PhaseObserver,
    ) -> StorytellerResult<NarratorRendering> {
        let start = Instant::now();

        let user_message = build_turn_message(context);

        let system_prompt_len = self.system_prompt.len();
        let user_message_len = user_message.len();

        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: context.journal.entries.last().map_or(0, |e| e.turn_number),
            stage: TurnCycleStage::Rendering,
            detail: PhaseEventDetail::NarratorPromptBuilt {
                system_prompt_chars: system_prompt_len,
                user_message_chars: user_message_len,
            },
        });

        let messages = vec![Message {
            role: MessageRole::User,
            content: user_message,
        }];

        let request = CompletionRequest {
            system_prompt: self.system_prompt.clone(),
            messages,
            max_tokens: 400,
            temperature: self.temperature,
        };

        let response = self.llm.complete(request).await?;
        let elapsed = start.elapsed();

        tracing::debug!(
            tokens = response.tokens_used,
            elapsed_ms = elapsed.as_millis() as u64,
            "narrator turn rendered"
        );

        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: context.journal.entries.last().map_or(0, |e| e.turn_number),
            stage: TurnCycleStage::Rendering,
            detail: PhaseEventDetail::NarratorRenderingComplete {
                tokens_used: Some(response.tokens_used),
                elapsed_ms: elapsed.as_millis() as u64,
            },
        });

        Ok(NarratorRendering {
            text: response.content,
            stage_directions: Some(context.resolver_output.scene_dynamics.clone()),
        })
    }

    /// Render a scene opening from assembled context.
    pub async fn render_opening(
        &self,
        observer: &dyn PhaseObserver,
    ) -> StorytellerResult<NarratorRendering> {
        let start = Instant::now();

        let user_message = "Open the scene. Establish the setting and mood. \
            The characters have not yet interacted. Under 200 words."
            .to_string();

        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: 0,
            stage: TurnCycleStage::Rendering,
            detail: PhaseEventDetail::NarratorPromptBuilt {
                system_prompt_chars: self.system_prompt.len(),
                user_message_chars: user_message.len(),
            },
        });

        let messages = vec![Message {
            role: MessageRole::User,
            content: user_message,
        }];

        let request = CompletionRequest {
            system_prompt: self.system_prompt.clone(),
            messages,
            max_tokens: 600,
            temperature: self.temperature,
        };

        let response = self.llm.complete(request).await?;
        let elapsed = start.elapsed();

        tracing::debug!(
            tokens = response.tokens_used,
            elapsed_ms = elapsed.as_millis() as u64,
            "narrator opening rendered"
        );

        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: 0,
            stage: TurnCycleStage::Rendering,
            detail: PhaseEventDetail::NarratorRenderingComplete {
                tokens_used: Some(response.tokens_used),
                elapsed_ms: elapsed.as_millis() as u64,
            },
        });

        Ok(NarratorRendering {
            text: response.content,
            stage_directions: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Prompt construction
// ---------------------------------------------------------------------------

/// Build the system prompt from assembled context (preamble + task instructions).
fn build_system_prompt(context: &NarratorContextInput) -> String {
    let preamble = render_preamble(&context.preamble);

    format!(
        r#"You are the Narrator.

{preamble}

## Your Task
You receive structured facts about what characters did, said, and felt.
You render only what is observable — physical actions, speech, gestures.
Never state what a character thinks, feels, or realizes. Show it through
the body. Trust the reader to infer.

Weave the facts into a single narrative passage. Use physical detail to
carry emotional weight.

## Already Presented
Each turn includes a record of what the player has already read. This
record is context for continuity — not material to re-render. Your job
is to advance the scene, not summarize it. If a detail from a previous
turn is relevant, reference it obliquely through a character's gesture
or awareness, never by restating it. Assume the reader remembers.

## Scope
Render ONLY the actions and events described in "This Turn." Do not
invent departures, goodbyes, or scene resolutions. Do not write beyond
the moment. The scene continues after your passage ends.

Write in present tense, third person. HARD LIMIT: under 200 words."#
    )
}

/// Build the user message for a turn from assembled context.
fn build_turn_message(context: &NarratorContextInput) -> String {
    let mut message = String::new();

    // Tier 2: Scene journal — framed as "already presented" to discourage
    // the narrator from re-rendering previous content (D.4 deduplication).
    let journal = render_journal(&context.journal);
    if !journal.is_empty() {
        message.push_str("## Already Presented to the Player\n");
        message.push_str("(For continuity reference only — do not re-render.)\n");
        message.push_str(&journal);
        message.push('\n');
    }

    // Tier 3: Retrieved context
    if !context.retrieved.is_empty() {
        message.push_str("## Relevant Context\n");
        for item in &context.retrieved {
            message.push_str(&format!("- **{}**: {}", item.subject, item.content));
            if let Some(emotional) = &item.emotional_context {
                message.push_str(&format!(" _{emotional}_"));
            }
            message.push('\n');
        }
        message.push('\n');
    }

    // Character predictions from ML pipeline
    if !context.resolver_output.original_predictions.is_empty() {
        let predictions_md = crate::context::prediction::render_predictions(
            &context.resolver_output.original_predictions,
        );
        message.push_str(&predictions_md);
    }

    // Current turn: resolver output
    message.push_str("## This Turn\n");
    message.push_str(&format!("Player: {}\n\n", context.player_input_summary));

    if !context.resolver_output.sequenced_actions.is_empty() {
        message.push_str("Character actions:\n");
        for action in &context.resolver_output.sequenced_actions {
            for outcome in &action.outcomes {
                message.push_str(&format!(
                    "- **{}**: {} ({:?})\n",
                    action.character_name, outcome.action.description, outcome.success,
                ));
                for consequence in &outcome.consequences {
                    message.push_str(&format!("  - Consequence: {consequence}\n"));
                }
            }
        }
        message.push('\n');
    }

    if !context.resolver_output.scene_dynamics.is_empty() {
        message.push_str(&format!(
            "Scene dynamics: {}\n\n",
            context.resolver_output.scene_dynamics
        ));
    }

    message.push_str(
        "Render ONLY this moment. Do not resolve the scene. Under 200 words.",
    );

    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::traits::phase_observer::NoopObserver;
    use storyteller_core::types::entity::EntityId;
    use storyteller_core::types::narrator_context::{
        CastDescription, PersistentPreamble, RetrievedContext, SceneJournal,
    };
    use storyteller_core::types::resolver::ResolverOutput;
    use storyteller_core::types::scene::SceneId;

    fn mock_context() -> NarratorContextInput {
        NarratorContextInput {
            preamble: PersistentPreamble {
                narrator_identity: "Literary fiction, present tense".to_string(),
                anti_patterns: vec!["Exclamation marks".to_string()],
                setting_description: "A smallholding outside Svyoritch".to_string(),
                cast_descriptions: vec![
                    CastDescription {
                        entity_id: EntityId::new(),
                        name: "Bramblehoof".to_string(),
                        role: "Visitor, catalyst".to_string(),
                        voice_note: "Warm, reaches for metaphor".to_string(),
                    },
                    CastDescription {
                        entity_id: EntityId::new(),
                        name: "Pyotir".to_string(),
                        role: "Resident, ground truth".to_string(),
                        voice_note: "Measured, practical".to_string(),
                    },
                ],
                boundaries: vec!["Pyotir cannot leave".to_string()],
            },
            journal: SceneJournal::new(SceneId::new(), 1200),
            retrieved: vec![RetrievedContext {
                subject: "Bramblehoof — backstory".to_string(),
                content: "A satyr bard who gave a boy a flute years ago".to_string(),
                revealed: false,
                emotional_context: Some("Hope mixed with guilt".to_string()),
                source_entities: vec![EntityId::new()],
            }],
            resolver_output: ResolverOutput {
                sequenced_actions: vec![],
                original_predictions: vec![],
                scene_dynamics: "Quiet tension between recognition and distance".to_string(),
                conflicts: vec![],
            },
            player_input_summary: "I approach the fence slowly.".to_string(),
            estimated_tokens: 1500,
        }
    }

    #[test]
    fn system_prompt_has_preamble() {
        let context = mock_context();
        let prompt = build_system_prompt(&context);
        assert!(prompt.contains("Your Voice"));
        assert!(prompt.contains("Never Do"));
        assert!(prompt.contains("The Scene"));
        assert!(prompt.contains("Bramblehoof"));
        assert!(prompt.contains("Pyotir"));
        assert!(prompt.contains("present tense"));
    }

    #[test]
    fn turn_message_has_all_sections() {
        let context = mock_context();
        let message = build_turn_message(&context);

        // Journal section (empty journal → "no prior turns")
        assert!(message.contains("Already Presented to the Player"));

        // Retrieved context section
        assert!(message.contains("Relevant Context"));
        assert!(message.contains("backstory"));
        assert!(message.contains("Hope mixed with guilt"));

        // Current turn
        assert!(message.contains("This Turn"));
        assert!(message.contains("I approach the fence slowly"));

        // Scene dynamics
        assert!(message.contains("Quiet tension"));

        // Rendering instruction
        assert!(message.contains("Under 200 words"));
    }

    #[test]
    fn new_creates_narrator() {
        use async_trait::async_trait;
        use storyteller_core::traits::llm::CompletionResponse;

        #[derive(Debug)]
        struct MockLlm;

        #[async_trait]
        impl LlmProvider for MockLlm {
            async fn complete(
                &self,
                _request: CompletionRequest,
            ) -> StorytellerResult<CompletionResponse> {
                Ok(CompletionResponse {
                    content: "Mock narrator output".to_string(),
                    tokens_used: 10,
                })
            }
        }

        let context = mock_context();
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let narrator = NarratorAgent::new(&context, llm);

        assert!(narrator.system_prompt.contains("Bramblehoof"));
        assert!(narrator.system_prompt.contains("Pyotir"));
        assert!(narrator.system_prompt.contains("Your Voice"));
    }

    #[tokio::test]
    async fn render_produces_output() {
        use async_trait::async_trait;
        use storyteller_core::traits::llm::CompletionResponse;

        #[derive(Debug)]
        struct MockLlm;

        #[async_trait]
        impl LlmProvider for MockLlm {
            async fn complete(
                &self,
                _request: CompletionRequest,
            ) -> StorytellerResult<CompletionResponse> {
                Ok(CompletionResponse {
                    content: "The hooves leave shallow prints in the turned earth.".to_string(),
                    tokens_used: 12,
                })
            }
        }

        let context = mock_context();
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let narrator = NarratorAgent::new(&context, llm);

        let observer = NoopObserver;
        let rendering = narrator
            .render(&context, &observer)
            .await
            .expect("render should succeed");

        assert!(rendering.text.contains("hooves"));
        assert!(rendering.stage_directions.is_some());
    }

    #[tokio::test]
    async fn render_emits_observer_events() {
        use async_trait::async_trait;
        use storyteller_core::traits::llm::CompletionResponse;
        use storyteller_core::traits::phase_observer::CollectingObserver;

        #[derive(Debug)]
        struct MockLlm;

        #[async_trait]
        impl LlmProvider for MockLlm {
            async fn complete(
                &self,
                _request: CompletionRequest,
            ) -> StorytellerResult<CompletionResponse> {
                Ok(CompletionResponse {
                    content: "Mock output.".to_string(),
                    tokens_used: 5,
                })
            }
        }

        let context = mock_context();
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let narrator = NarratorAgent::new(&context, llm);

        let observer = CollectingObserver::new();
        let _rendering = narrator
            .render(&context, &observer)
            .await
            .expect("render should succeed");

        let events = observer.take_events();
        assert_eq!(
            events.len(),
            2,
            "Expected NarratorPromptBuilt + NarratorRenderingComplete"
        );

        assert!(matches!(
            events[0].detail,
            PhaseEventDetail::NarratorPromptBuilt { .. }
        ));
        assert!(matches!(
            events[1].detail,
            PhaseEventDetail::NarratorRenderingComplete { .. }
        ));
    }
}
