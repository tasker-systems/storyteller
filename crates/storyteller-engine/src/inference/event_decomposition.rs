//! Event decomposition via small-LLM structured JSON extraction.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! Maps narrative text to entity→action→entity triples using a small,
//! fast model (e.g., Qwen2.5:3b-instruct) with constrained JSON output.
//! The output converts to [`super::event_classifier::ClassificationOutput`]
//! for compatibility with the existing ML classification pipeline.

use serde::{Deserialize, Serialize};

use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::structured_llm::{StructuredLlmProvider, StructuredRequest};
use storyteller_core::types::event_grammar::RelationalDirection;
use storyteller_ml::event_templates::NerCategory;

use super::event_classifier::{ClassificationOutput, ExtractedEntity};

/// Default confidence score assigned to LLM-extracted events and entities.
///
/// The small LLM provides structured extraction without numeric confidence
/// scores. This constant represents the baseline trust level for LLM
/// extraction outputs — lower than high-confidence ML model predictions
/// but sufficient for downstream processing.
const LLM_DEFAULT_CONFIDENCE: f32 = 0.85;

// ===========================================================================
// Deserialized types — map to the JSON schema sent to the LLM
// ===========================================================================

/// An entity mention extracted by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedEntity {
    /// The text mention as it appears in the narrative.
    pub mention: String,
    /// Entity category as a string (e.g., "CHARACTER", "OBJECT").
    pub category: String,
}

/// A single decomposed event from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedEvent {
    /// Event kind label (e.g., "SpeechAct", "ActionOccurrence").
    pub kind: String,
    /// The entity performing the action, if identified.
    pub actor: Option<DecomposedEntity>,
    /// The action verb phrase.
    pub action: String,
    /// The entity receiving the action, if applicable.
    pub target: Option<DecomposedEntity>,
    /// How the event's relational vector flows.
    #[serde(
        deserialize_with = "deserialize_relational_direction",
        serialize_with = "serialize_relational_direction"
    )]
    pub relational_direction: RelationalDirection,
    /// Optional note from the LLM explaining its confidence reasoning.
    pub confidence_note: Option<String>,
}

/// The full decomposition result from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDecomposition {
    /// Discrete events extracted from the text.
    pub events: Vec<DecomposedEvent>,
    /// All entity mentions, including those not part of any event.
    pub entities: Vec<DecomposedEntity>,
}

// ===========================================================================
// Custom deserialization for RelationalDirection
// ===========================================================================

/// Serialize `RelationalDirection` to lowercase strings matching the LLM schema.
fn serialize_relational_direction<S>(
    value: &RelationalDirection,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = match value {
        RelationalDirection::Directed => "directed",
        RelationalDirection::Mutual => "mutual",
        RelationalDirection::SelfDirected => "self",
        RelationalDirection::Diffuse => "diffuse",
    };
    serializer.serialize_str(s)
}

/// Deserialize `RelationalDirection` from lowercase LLM output strings.
///
/// Maps: "directed" → Directed, "mutual" → Mutual, "self" → SelfDirected,
/// "diffuse" → Diffuse.
fn deserialize_relational_direction<'de, D>(
    deserializer: D,
) -> Result<RelationalDirection, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "directed" => Ok(RelationalDirection::Directed),
        "mutual" => Ok(RelationalDirection::Mutual),
        "self" => Ok(RelationalDirection::SelfDirected),
        "diffuse" => Ok(RelationalDirection::Diffuse),
        other => Err(serde::de::Error::custom(format!(
            "unknown relational_direction: {other}"
        ))),
    }
}

// ===========================================================================
// NerCategory string parsing
// ===========================================================================

/// Parse a NER category string (as output by the LLM) to a `NerCategory`.
///
/// Accepts uppercase labels matching the enum variant names.
fn parse_ner_category(s: &str) -> Option<NerCategory> {
    match s {
        "CHARACTER" => Some(NerCategory::Character),
        "OBJECT" => Some(NerCategory::Object),
        "LOCATION" => Some(NerCategory::Location),
        "GESTURE" => Some(NerCategory::Gesture),
        "SENSORY" => Some(NerCategory::Sensory),
        "ABSTRACT" => Some(NerCategory::Abstract),
        "COLLECTIVE" => Some(NerCategory::Collective),
        _ => None,
    }
}

// ===========================================================================
// EventDecomposition methods
// ===========================================================================

impl EventDecomposition {
    /// Parse an `EventDecomposition` from a JSON value produced by the LLM.
    ///
    /// Small models sometimes nest their output inside the schema structure
    /// (e.g., under a `"properties"` key). This method detects that pattern
    /// and unwraps the actual data before parsing.
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` if the JSON does not match
    /// the expected schema.
    pub fn from_json(value: &serde_json::Value) -> StorytellerResult<Self> {
        // Try direct parse first.
        if let Ok(decomp) = serde_json::from_value::<Self>(value.clone()) {
            return Ok(decomp);
        }

        // Small models sometimes wrap the response in the schema structure,
        // nesting actual data under the "properties" key.
        if let Some(props) = value.get("properties") {
            if let Ok(decomp) = serde_json::from_value::<Self>(props.clone()) {
                return Ok(decomp);
            }
        }

        // Neither attempt succeeded — report the direct-parse error.
        serde_json::from_value(value.clone()).map_err(|e| {
            storyteller_core::StorytellerError::Inference(format!(
                "failed to parse event decomposition: {e}"
            ))
        })
    }

    /// Convert to the existing classification output contract.
    ///
    /// Each event's `kind` becomes an `(String, 0.85)` tuple. Each entity
    /// becomes an `ExtractedEntity` with `start: 0, end: 0` (the LLM does
    /// not provide character offsets) and `confidence: 0.85`.
    pub fn to_classification_output(&self) -> ClassificationOutput {
        let event_kinds: Vec<(String, f32)> = self
            .events
            .iter()
            .map(|e| (e.kind.clone(), LLM_DEFAULT_CONFIDENCE))
            .collect();

        let entity_mentions: Vec<ExtractedEntity> = self
            .entities
            .iter()
            .filter_map(|e| {
                let category = parse_ner_category(&e.category)?;
                Some(ExtractedEntity {
                    text: e.mention.clone(),
                    start: 0,
                    end: 0,
                    category,
                    confidence: LLM_DEFAULT_CONFIDENCE,
                })
            })
            .collect();

        ClassificationOutput {
            event_kinds,
            entity_mentions,
        }
    }
}

// ===========================================================================
// System prompt and schema
// ===========================================================================

/// Returns the JSON schema for the event decomposition output.
pub fn event_decomposition_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["events", "entities"],
        "properties": {
            "events": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["kind", "action", "relational_direction"],
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": [
                                "StateAssertion", "ActionOccurrence", "SpatialChange",
                                "EmotionalExpression", "InformationTransfer", "SpeechAct",
                                "RelationalShift", "EnvironmentalChange"
                            ]
                        },
                        "actor": {
                            "type": "object",
                            "properties": {
                                "mention": { "type": "string" },
                                "category": {
                                    "type": "string",
                                    "enum": ["CHARACTER", "OBJECT", "LOCATION", "GESTURE", "SENSORY", "ABSTRACT", "COLLECTIVE"]
                                }
                            }
                        },
                        "action": { "type": "string" },
                        "target": {
                            "type": "object",
                            "properties": {
                                "mention": { "type": "string" },
                                "category": {
                                    "type": "string",
                                    "enum": ["CHARACTER", "OBJECT", "LOCATION", "GESTURE", "SENSORY", "ABSTRACT", "COLLECTIVE"]
                                }
                            }
                        },
                        "relational_direction": {
                            "type": "string",
                            "enum": ["directed", "mutual", "self", "diffuse"]
                        },
                        "confidence_note": { "type": "string" }
                    }
                }
            },
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["mention", "category"],
                    "properties": {
                        "mention": { "type": "string" },
                        "category": {
                            "type": "string",
                            "enum": ["CHARACTER", "OBJECT", "LOCATION", "GESTURE", "SENSORY", "ABSTRACT", "COLLECTIVE"]
                        }
                    }
                }
            }
        }
    })
}

/// Returns the system prompt for event decomposition.
///
/// Describes the extraction task and rules. The output schema is passed
/// separately via `StructuredRequest::output_schema` — the provider
/// injects it into the prompt so the model sees it exactly once.
pub fn event_decomposition_system_prompt() -> String {
    "You are an event extractor for interactive fiction. Given narrative text, \
identify discrete events as entity→action→entity triples.

Rules:
- Every event needs at minimum an actor and an action
- A target is required for directed actions, optional for self/diffuse actions
- Use entity categories: CHARACTER, OBJECT, LOCATION, GESTURE, SENSORY, ABSTRACT, COLLECTIVE
- Use event kinds: StateAssertion, ActionOccurrence, SpatialChange, EmotionalExpression, \
InformationTransfer, SpeechAct, RelationalShift, EnvironmentalChange
- relational_direction must be one of: \"directed\", \"mutual\", \"self\", \"diffuse\"
- When a character acts without a clear target, set relational_direction to \"self\"
- When an action affects the general situation, set relational_direction to \"diffuse\"
- Extract ALL entities mentioned, even those not in events"
        .to_string()
}

// ===========================================================================
// Async decomposition function
// ===========================================================================

/// Decompose narrative text into structured events using a small LLM.
///
/// Sends the text to the structured LLM provider with the event decomposition
/// system prompt and schema, then parses the JSON response.
///
/// # Errors
///
/// Returns `StorytellerError::Inference` if the provider call fails or the
/// response cannot be parsed.
pub async fn decompose_events(
    provider: &dyn StructuredLlmProvider,
    text: &str,
) -> StorytellerResult<EventDecomposition> {
    let request = StructuredRequest {
        system: event_decomposition_system_prompt(),
        input: text.to_string(),
        output_schema: event_decomposition_schema(),
        temperature: 0.1,
    };

    let json = provider.extract(request).await?;
    EventDecomposition::from_json(&json)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_decomposition_from_valid_json() {
        let json = serde_json::json!({
            "events": [{
                "kind": "EmotionalExpression",
                "actor": { "mention": "the child", "category": "CHARACTER" },
                "action": "laughs at",
                "target": { "mention": "the small sprite", "category": "CHARACTER" },
                "relational_direction": "directed",
                "confidence_note": "clear directed emotional expression"
            }],
            "entities": [
                { "mention": "the child", "category": "CHARACTER" },
                { "mention": "the small sprite", "category": "CHARACTER" },
                { "mention": "the corner", "category": "LOCATION" }
            ]
        });
        let result = EventDecomposition::from_json(&json);
        assert!(result.is_ok(), "Parse failed: {result:?}");
        let decomp = result.unwrap();
        assert_eq!(decomp.events.len(), 1);
        assert_eq!(decomp.entities.len(), 3);
        assert_eq!(decomp.events[0].kind, "EmotionalExpression");
        assert!(matches!(
            decomp.events[0].relational_direction,
            RelationalDirection::Directed
        ));
    }

    #[test]
    fn parse_self_directed_event() {
        let json = serde_json::json!({
            "events": [{
                "kind": "EmotionalExpression",
                "actor": { "mention": "the wanderer", "category": "CHARACTER" },
                "action": "laughs aloud",
                "relational_direction": "self",
                "confidence_note": "no target, self-directed joy"
            }],
            "entities": [
                { "mention": "the wanderer", "category": "CHARACTER" }
            ]
        });
        let decomp = EventDecomposition::from_json(&json).unwrap();
        assert!(matches!(
            decomp.events[0].relational_direction,
            RelationalDirection::SelfDirected
        ));
        assert!(decomp.events[0].target.is_none());
    }

    #[test]
    fn to_classification_output_maps_correctly() {
        let json = serde_json::json!({
            "events": [{
                "kind": "SpeechAct",
                "actor": { "mention": "I", "category": "CHARACTER" },
                "action": "say hello to",
                "target": { "mention": "Pyotir", "category": "CHARACTER" },
                "relational_direction": "directed",
                "confidence_note": "explicit speech act"
            }],
            "entities": [
                { "mention": "I", "category": "CHARACTER" },
                { "mention": "Pyotir", "category": "CHARACTER" }
            ]
        });
        let decomp = EventDecomposition::from_json(&json).unwrap();
        let output = decomp.to_classification_output();
        assert_eq!(output.event_kinds.len(), 1);
        assert_eq!(output.event_kinds[0].0, "SpeechAct");
        assert_eq!(output.entity_mentions.len(), 2);
    }

    #[test]
    fn system_prompt_is_well_formed() {
        let prompt = event_decomposition_system_prompt();
        assert!(prompt.contains("entity"));
        assert!(prompt.contains("CHARACTER"));
        assert!(prompt.contains("SpeechAct"));
        assert!(prompt.contains("relational_direction"));
    }

    #[test]
    fn empty_events_parses_cleanly() {
        let json = serde_json::json!({
            "events": [],
            "entities": []
        });
        let decomp = EventDecomposition::from_json(&json).unwrap();
        assert!(decomp.events.is_empty());
        assert!(decomp.entities.is_empty());
        let output = decomp.to_classification_output();
        assert!(output.event_kinds.is_empty());
    }

    #[test]
    fn schema_contains_required_fields() {
        let schema = event_decomposition_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("events")));
        assert!(required.contains(&serde_json::json!("entities")));
    }

    #[test]
    fn to_classification_output_skips_unknown_categories() {
        let json = serde_json::json!({
            "events": [],
            "entities": [
                { "mention": "the thing", "category": "UNKNOWN_CATEGORY" },
                { "mention": "Sarah", "category": "CHARACTER" }
            ]
        });
        let decomp = EventDecomposition::from_json(&json).unwrap();
        let output = decomp.to_classification_output();
        // Unknown category filtered out, only CHARACTER remains.
        assert_eq!(output.entity_mentions.len(), 1);
        assert_eq!(output.entity_mentions[0].text, "Sarah");
    }

    #[test]
    fn parse_all_relational_directions() {
        for (input, expected) in [
            ("directed", RelationalDirection::Directed),
            ("mutual", RelationalDirection::Mutual),
            ("self", RelationalDirection::SelfDirected),
            ("diffuse", RelationalDirection::Diffuse),
        ] {
            let json = serde_json::json!({
                "events": [{
                    "kind": "ActionOccurrence",
                    "action": "test",
                    "relational_direction": input
                }],
                "entities": []
            });
            let decomp = EventDecomposition::from_json(&json).unwrap();
            assert_eq!(
                decomp.events[0].relational_direction, expected,
                "failed for input: {input}"
            );
        }
    }

    #[test]
    fn parse_schema_wrapped_response() {
        // Small models sometimes nest data inside the schema structure.
        let json = serde_json::json!({
            "type": "object",
            "required": ["events", "entities"],
            "properties": {
                "events": [{
                    "kind": "SpeechAct",
                    "actor": { "mention": "I", "category": "CHARACTER" },
                    "action": "say hello",
                    "target": { "mention": "Pyotir", "category": "CHARACTER" },
                    "relational_direction": "directed"
                }],
                "entities": [
                    { "mention": "I", "category": "CHARACTER" },
                    { "mention": "Pyotir", "category": "CHARACTER" }
                ]
            }
        });
        let result = EventDecomposition::from_json(&json);
        assert!(
            result.is_ok(),
            "Should unwrap schema-nested response: {result:?}"
        );
        let decomp = result.unwrap();
        assert_eq!(decomp.events.len(), 1);
        assert_eq!(decomp.events[0].kind, "SpeechAct");
        assert_eq!(decomp.entities.len(), 2);
    }

    #[test]
    fn invalid_relational_direction_fails() {
        let json = serde_json::json!({
            "events": [{
                "kind": "ActionOccurrence",
                "action": "test",
                "relational_direction": "unknown_direction"
            }],
            "entities": []
        });
        let result = EventDecomposition::from_json(&json);
        assert!(result.is_err());
    }

    #[cfg(feature = "test-llm")]
    mod integration {
        use super::super::*;
        use crate::inference::structured::OllamaStructuredProvider;
        use storyteller_core::traits::structured_llm::StructuredLlmConfig;

        fn provider() -> OllamaStructuredProvider {
            OllamaStructuredProvider::new(StructuredLlmConfig::default())
        }

        #[tokio::test]
        async fn decompose_speech_act() {
            let result = decompose_events(
                &provider(),
                "I say to Pyotir, 'Do you remember this melody?'",
            )
            .await;
            assert!(result.is_ok(), "Decomposition failed: {result:?}");
            let decomp = result.unwrap();
            assert!(!decomp.events.is_empty(), "No events extracted");
            assert!(
                decomp
                    .events
                    .iter()
                    .any(|e| e.kind == "SpeechAct" || e.kind == "InformationTransfer"),
                "Expected SpeechAct or InformationTransfer, got: {:?}",
                decomp.events.iter().map(|e| &e.kind).collect::<Vec<_>>()
            );
        }

        #[tokio::test]
        async fn decompose_directed_emotional_expression() {
            let result = decompose_events(
                &provider(),
                "The child laughs at the small sprite in the corner.",
            )
            .await;
            assert!(result.is_ok(), "Decomposition failed: {result:?}");
            let decomp = result.unwrap();
            assert!(!decomp.events.is_empty(), "No events extracted");
            let event = &decomp.events[0];
            assert!(event.actor.is_some(), "Expected actor, got none");
            // The LLM should identify this as directed (at the sprite)
            assert!(
                event.target.is_some()
                    || matches!(event.relational_direction, RelationalDirection::Directed),
                "Expected directed action with target"
            );
        }

        #[tokio::test]
        async fn decompose_self_directed_action() {
            let result = decompose_events(
                &provider(),
                "Walking through the sunlit meadow, she laughs aloud with pure joy.",
            )
            .await;
            assert!(result.is_ok(), "Decomposition failed: {result:?}");
            let decomp = result.unwrap();
            assert!(!decomp.events.is_empty(), "No events extracted");
            // Should be self-directed or diffuse — no specific target
        }

        #[tokio::test]
        async fn decompose_multi_entity_action() {
            let result = decompose_events(
                &provider(),
                "I dive for the rapier, roll across the floor, and slash the weapon from his hand.",
            )
            .await;
            assert!(result.is_ok(), "Decomposition failed: {result:?}");
            let decomp = result.unwrap();
            assert!(
                decomp.entities.len() >= 2,
                "Expected at least 2 entities, got {}: {:?}",
                decomp.entities.len(),
                decomp.entities
            );
            assert!(!decomp.events.is_empty(), "No events extracted");
        }

        #[tokio::test]
        async fn decompose_produces_valid_classification_output() {
            let result = decompose_events(
                &provider(),
                "I pick up the ancient stone from the riverbed.",
            )
            .await;
            assert!(result.is_ok(), "Decomposition failed: {result:?}");
            let decomp = result.unwrap();
            let output = decomp.to_classification_output();
            // Should produce at least one event kind
            assert!(
                !output.event_kinds.is_empty(),
                "No event kinds in classification output"
            );
            // Confidence should be 0.85 (default)
            assert!((output.event_kinds[0].1 - 0.85).abs() < 0.01);
        }
    }
}
