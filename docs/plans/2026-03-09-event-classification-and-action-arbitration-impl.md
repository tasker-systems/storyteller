# Event Classification and Action Arbitration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace DistilBERT as the authoritative event decomposer with a small LLM (Qwen2.5:3b-instruct) for entity→action→entity extraction, and add action arbitration with deterministic rules + LLM fallback.

**Architecture:** A `StructuredLlmProvider` trait with Ollama implementation handles JSON extraction for both event decomposition (D.3 committed-turn phase) and action arbitration (pre-resolution). DistilBERT remains for D.2 fast pass. Typed `GenreConstraint` and `ActionPossibility` enums replace string-based genre physics. Capability lexicon pre-seeding enables natural language → authored capability matching.

**Tech Stack:** Rust (storyteller-core types, storyteller-engine systems), Bevy ECS (Resources, Systems), reqwest (HTTP to Ollama), serde_json (structured output parsing), Ollama with qwen2.5:3b-instruct.

**Design Doc:** `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`

---

### Task 1: RelationalDirection Enum in storyteller-core

**Files:**
- Modify: `crates/storyteller-core/src/types/event_grammar.rs`
- Modify: `crates/storyteller-core/src/types/mod.rs` (if needed for re-export)

**Step 1: Write the failing test**

Add to the test module in `event_grammar.rs`:

```rust
#[test]
fn relational_direction_variants_exist() {
    let directed = RelationalDirection::Directed;
    let mutual = RelationalDirection::Mutual;
    let self_directed = RelationalDirection::SelfDirected;
    let diffuse = RelationalDirection::Diffuse;
    assert_ne!(format!("{directed:?}"), format!("{mutual:?}"));
    assert_ne!(format!("{self_directed:?}"), format!("{diffuse:?}"));
}

#[test]
fn relational_direction_serializes() {
    let directed = RelationalDirection::Directed;
    let json = serde_json::to_string(&directed).unwrap();
    assert_eq!(json, "\"Directed\"");
    let roundtrip: RelationalDirection = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtrip, directed);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-core relational_direction --all-features`
Expected: FAIL — `RelationalDirection` not found

**Step 3: Write the enum**

Add to `event_grammar.rs` near the other event-related enums:

```rust
/// Directionality of an event's relational vector.
///
/// Captures whether an event flows from actor→target, is mutual,
/// is self-directed (internal/reflexive), or is diffuse (directed
/// at the situation rather than a specific entity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelationalDirection {
    /// Actor acts upon target: "laughs at", "strikes", "tells"
    Directed,
    /// Both parties involved mutually: "embrace", "argue", "negotiate"
    Mutual,
    /// Action directed at self: laughing alone, sighing, internal realization
    SelfDirected,
    /// Action directed at situation/world, not a specific entity:
    /// joyful laughter in a field, despair at circumstance
    Diffuse,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p storyteller-core relational_direction --all-features`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/storyteller-core/src/types/event_grammar.rs
git commit -m "feat(core): add RelationalDirection enum for event directionality"
```

---

### Task 2: StructuredLlmProvider Trait in storyteller-core

**Files:**
- Create: `crates/storyteller-core/src/traits/structured_llm.rs`
- Modify: `crates/storyteller-core/src/traits/mod.rs`

**Step 1: Write the failing test**

Add a test in the new file that exercises the trait types:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_request_builds_with_defaults() {
        let req = StructuredRequest {
            system: "Extract events".to_string(),
            input: "The child laughs at the sprite".to_string(),
            output_schema: serde_json::json!({"type": "object"}),
            temperature: 0.1,
        };
        assert_eq!(req.temperature, 0.1);
        assert!(!req.input.is_empty());
    }

    #[test]
    fn structured_llm_config_has_sensible_defaults() {
        let config = StructuredLlmConfig::default();
        assert_eq!(config.base_url, "http://127.0.0.1:11434");
        assert_eq!(config.model, "qwen2.5:3b-instruct");
        assert_eq!(config.temperature, 0.1);
        assert_eq!(config.timeout, std::time::Duration::from_secs(10));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-core structured_llm --all-features`
Expected: FAIL — module not found

**Step 3: Write the trait and types**

Create `crates/storyteller-core/src/traits/structured_llm.rs`:

```rust
//! Structured LLM provider — fast JSON extraction for event decomposition
//! and action arbitration.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! Distinct from the narrator's `LlmProvider`. The narrator uses a capable
//! model for prose generation. This provider uses a small, fast model
//! (e.g., Qwen2.5:3b-instruct) for structured extraction tasks with
//! constrained JSON output.

use std::time::Duration;

use crate::errors::StorytellerResult;

/// A request for structured JSON extraction from a small LLM.
#[derive(Debug, Clone)]
pub struct StructuredRequest {
    /// System prompt establishing the extraction task.
    pub system: String,
    /// The text to analyze.
    pub input: String,
    /// JSON schema the output must conform to.
    pub output_schema: serde_json::Value,
    /// Temperature (low — extraction, not creativity).
    pub temperature: f32,
}

/// Configuration for the structured LLM service.
#[derive(Debug, Clone)]
pub struct StructuredLlmConfig {
    /// Base URL of the server (e.g., "http://127.0.0.1:11434").
    pub base_url: String,
    /// Model name — distinct from narrator model.
    pub model: String,
    /// Temperature for extraction tasks.
    pub temperature: f32,
    /// Request timeout — these calls should be fast.
    pub timeout: Duration,
}

impl Default for StructuredLlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            model: "qwen2.5:3b-instruct".to_string(),
            temperature: 0.1,
            timeout: Duration::from_secs(10),
        }
    }
}

/// Provider for fast, structured-output LLM calls.
///
/// Used by both event decomposition (D.3) and action arbitration.
/// Implementations connect to Ollama or similar inference servers.
#[async_trait::async_trait]
pub trait StructuredLlmProvider: std::fmt::Debug + Send + Sync {
    /// Send a structured extraction request and receive JSON output.
    async fn extract(&self, request: StructuredRequest) -> StorytellerResult<serde_json::Value>;
}
```

Add to `crates/storyteller-core/src/traits/mod.rs`:

```rust
pub mod structured_llm;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p storyteller-core structured_llm --all-features`
Expected: PASS

**Step 5: Run full core tests**

Run: `cargo test -p storyteller-core --all-features`
Expected: All pass

**Step 6: Commit**

```bash
git add crates/storyteller-core/src/traits/structured_llm.rs crates/storyteller-core/src/traits/mod.rs
git commit -m "feat(core): add StructuredLlmProvider trait for JSON extraction"
```

---

### Task 3: Ollama Structured LLM Provider Implementation

**Files:**
- Create: `crates/storyteller-engine/src/inference/structured.rs`
- Modify: `crates/storyteller-engine/src/inference/mod.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::traits::structured_llm::StructuredLlmConfig;

    #[test]
    fn provider_builds_from_config() {
        let config = StructuredLlmConfig::default();
        let provider = OllamaStructuredProvider::new(config.clone());
        assert_eq!(provider.config.model, "qwen2.5:3b-instruct");
    }

    #[test]
    fn provider_implements_debug() {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let debug = format!("{provider:?}");
        assert!(debug.contains("OllamaStructuredProvider"));
    }

    #[test]
    fn parse_llm_json_extracts_from_markdown_fences() {
        let raw = "```json\n{\"events\": []}\n```";
        let parsed = extract_json_from_response(raw).unwrap();
        assert_eq!(parsed, serde_json::json!({"events": []}));
    }

    #[test]
    fn parse_llm_json_handles_bare_json() {
        let raw = "{\"events\": []}";
        let parsed = extract_json_from_response(raw).unwrap();
        assert_eq!(parsed, serde_json::json!({"events": []}));
    }

    #[test]
    fn parse_llm_json_handles_text_around_json() {
        let raw = "Here is the result:\n{\"events\": []}\nDone.";
        let parsed = extract_json_from_response(raw).unwrap();
        assert_eq!(parsed, serde_json::json!({"events": []}));
    }

    // Integration test — requires Ollama with qwen2.5:3b-instruct
    #[cfg(feature = "test-llm")]
    #[tokio::test]
    async fn ollama_structured_extraction() {
        use storyteller_core::traits::structured_llm::StructuredRequest;

        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let request = StructuredRequest {
            system: "Return a JSON object with a single key 'greeting' containing 'hello'.".to_string(),
            input: "Say hello.".to_string(),
            output_schema: serde_json::json!({"type": "object", "properties": {"greeting": {"type": "string"}}}),
            temperature: 0.1,
        };
        let result = provider.extract(request).await;
        assert!(result.is_ok(), "Structured extraction failed: {result:?}");
        let json = result.unwrap();
        assert!(json.get("greeting").is_some(), "Missing 'greeting' key: {json}");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine structured --all-features`
Expected: FAIL — module not found

**Step 3: Write the implementation**

Create `crates/storyteller-engine/src/inference/structured.rs`:

```rust
//! Ollama implementation of StructuredLlmProvider for JSON extraction.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! Connects to Ollama's /api/chat endpoint with format: "json" to request
//! structured output. Used for event decomposition and action arbitration.

use storyteller_core::errors::StorytellerError;
use storyteller_core::traits::structured_llm::{
    StructuredLlmConfig, StructuredLlmProvider, StructuredRequest,
};
use tracing::instrument;

/// Ollama-backed structured LLM provider.
#[derive(Debug)]
pub struct OllamaStructuredProvider {
    config: StructuredLlmConfig,
    client: reqwest::Client,
}

impl OllamaStructuredProvider {
    /// Create a new provider with the given configuration.
    pub fn new(config: StructuredLlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("failed to build HTTP client");
        Self { config, client }
    }
}

#[derive(Debug, serde::Serialize)]
struct OllamaJsonRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    format: String,
    options: OllamaOptions,
}

#[derive(Debug, serde::Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, serde::Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, serde::Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Debug, serde::Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

/// Extract JSON from an LLM response that may contain markdown fences or surrounding text.
pub fn extract_json_from_response(raw: &str) -> Result<serde_json::Value, StorytellerError> {
    let trimmed = raw.trim();

    // Try direct parse first
    if let Ok(v) = serde_json::from_str(trimmed) {
        return Ok(v);
    }

    // Try extracting from markdown code fence
    if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.find("```") {
            let json_str = after_fence[..end].trim();
            if let Ok(v) = serde_json::from_str(json_str) {
                return Ok(v);
            }
        }
    }

    // Try finding first { and last } for bare JSON in surrounding text
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            let json_str = &trimmed[start..=end];
            if let Ok(v) = serde_json::from_str(json_str) {
                return Ok(v);
            }
        }
    }

    Err(StorytellerError::Inference(format!(
        "failed to extract JSON from LLM response: {trimmed}"
    )))
}

#[async_trait::async_trait]
impl StructuredLlmProvider for OllamaStructuredProvider {
    #[instrument(skip(self, request), fields(model = %self.config.model))]
    async fn extract(
        &self,
        request: StructuredRequest,
    ) -> storyteller_core::StorytellerResult<serde_json::Value> {
        let url = format!("{}/api/chat", self.config.base_url);

        let ollama_request = OllamaJsonRequest {
            model: self.config.model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: request.system,
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: request.input,
                },
            ],
            stream: false,
            format: "json".to_string(),
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: 1024,
            },
        };

        tracing::debug!("sending structured extraction request to Ollama: {url}");

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| StorytellerError::Inference(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "failed to read body".to_string());
            return Err(StorytellerError::Inference(format!(
                "Ollama returned {status}: {body}"
            )));
        }

        let ollama_response: OllamaChatResponse = response
            .json()
            .await
            .map_err(|e| {
                StorytellerError::Inference(format!("failed to parse Ollama response: {e}"))
            })?;

        tracing::debug!(
            content_len = ollama_response.message.content.len(),
            "structured extraction response received"
        );

        extract_json_from_response(&ollama_response.message.content)
    }
}
```

Add to `crates/storyteller-engine/src/inference/mod.rs`:

```rust
pub mod structured;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine structured --all-features`
Expected: Unit tests PASS (integration test skipped without `test-llm` feature)

**Step 5: Commit**

```bash
git add crates/storyteller-engine/src/inference/structured.rs crates/storyteller-engine/src/inference/mod.rs
git commit -m "feat(engine): add OllamaStructuredProvider for JSON extraction via Ollama"
```

---

### Task 4: Event Decomposition Types and Parsing

**Files:**
- Create: `crates/storyteller-engine/src/inference/event_decomposition.rs`
- Modify: `crates/storyteller-engine/src/inference/mod.rs`

This task defines the Rust types that map to the JSON extraction schema and the parsing logic that converts LLM JSON output into `ClassificationOutput` (the existing contract).

**Step 1: Write the failing tests**

```rust
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
        assert_eq!(decomp.events[0].relational_direction, RelationalDirection::Directed);
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
        assert_eq!(decomp.events[0].relational_direction, RelationalDirection::SelfDirected);
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
        assert!(prompt.contains("entity→action→entity"));
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
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine event_decomposition --all-features`
Expected: FAIL — module not found

**Step 3: Write the implementation**

Create `crates/storyteller-engine/src/inference/event_decomposition.rs`:

This file should contain:
- `EventDecomposition` struct (mirrors the JSON schema) with `from_json()` parser
- `DecomposedEvent` struct with actor, action, target (optional), kind, relational_direction, confidence_note
- `DecomposedEntity` struct with mention and category
- `to_classification_output()` — maps the LLM output into the existing `ClassificationOutput` contract so downstream `build_event_atoms()` works unchanged
- `event_decomposition_system_prompt()` — returns the system prompt string
- `event_decomposition_schema()` — returns the JSON schema for the output

The `RelationalDirection` enum comes from `storyteller_core::types::event_grammar::RelationalDirection` (Task 1). The `to_classification_output()` method should:
- Map each `DecomposedEvent.kind` string → `(String, f32)` tuple with confidence 0.85 (LLM extraction default)
- Map each `DecomposedEntity` → `ExtractedEntity` with `category` mapped via `storyteller_ml::event_labels::NerCategory`
- Character offsets set to 0 for LLM-extracted entities (the LLM doesn't provide token offsets)

**Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine event_decomposition --all-features`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/storyteller-engine/src/inference/event_decomposition.rs crates/storyteller-engine/src/inference/mod.rs
git commit -m "feat(engine): add event decomposition types and LLM output parsing"
```

---

### Task 5: Typed GenreConstraint and ActionPossibility in storyteller-core

**Files:**
- Modify: `crates/storyteller-core/src/types/world_model.rs`

**Step 1: Write the failing tests**

Add to the test module in `world_model.rs`:

```rust
#[test]
fn genre_constraint_forbidden_variant() {
    let constraint = GenreConstraint::Forbidden {
        capability: "telekinesis".to_string(),
        reason: "Magic does not exist in this world".to_string(),
    };
    assert!(matches!(constraint, GenreConstraint::Forbidden { .. }));
}

#[test]
fn genre_constraint_conditional_variant() {
    let constraint = GenreConstraint::Conditional {
        capability: "flight".to_string(),
        requires: vec!["wings".to_string(), "open sky".to_string()],
    };
    if let GenreConstraint::Conditional { requires, .. } = &constraint {
        assert_eq!(requires.len(), 2);
    }
}

#[test]
fn genre_constraint_physics_override() {
    let constraint = GenreConstraint::PhysicsOverride {
        property: "gravity".to_string(),
        value: "0.3g".to_string(),
    };
    assert!(matches!(constraint, GenreConstraint::PhysicsOverride { .. }));
}

#[test]
fn action_possibility_permitted() {
    let result = ActionPossibility::Permitted { conditions: vec![] };
    assert!(result.is_permitted());
}

#[test]
fn action_possibility_impossible() {
    let result = ActionPossibility::Impossible {
        reason: ConstraintViolation {
            constraint_name: "genre_physics".to_string(),
            description: "Magic does not exist".to_string(),
        },
    };
    assert!(result.is_impossible());
}

#[test]
fn action_possibility_ambiguous() {
    let result = ActionPossibility::Ambiguous {
        known_constraints: vec![],
        uncertainty: "Low gravity leap height unclear".to_string(),
    };
    assert!(result.is_ambiguous());
}

#[test]
fn genre_constraint_serializes() {
    let constraint = GenreConstraint::Forbidden {
        capability: "telekinesis".to_string(),
        reason: "No magic".to_string(),
    };
    let json = serde_json::to_string(&constraint).unwrap();
    let roundtrip: GenreConstraint = serde_json::from_str(&json).unwrap();
    assert!(matches!(roundtrip, GenreConstraint::Forbidden { .. }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-core genre_constraint --all-features && cargo test -p storyteller-core action_possibility --all-features`
Expected: FAIL — types not found

**Step 3: Write the types**

Add to `world_model.rs`:

```rust
/// A typed genre constraint — replaces freeform genre_physics strings
/// with structured rules the arbitration system can evaluate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GenreConstraint {
    /// A capability that does not exist in this world.
    Forbidden { capability: String, reason: String },
    /// A capability that exists with conditions.
    Conditional {
        capability: String,
        requires: Vec<String>,
    },
    /// A physical law override (e.g., low gravity, no sound in vacuum).
    PhysicsOverride { property: String, value: String },
}

/// A constraint violation explaining why an action is impossible.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstraintViolation {
    /// Which constraint was violated.
    pub constraint_name: String,
    /// Human-readable explanation for narrator context injection.
    pub description: String,
}

/// A condition that modifies a permitted action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionCondition {
    /// What must happen for the action to succeed.
    pub requirement: String,
    /// How this affects the graduated outcome.
    pub impact: String,
}

/// Result of an action possibility check.
///
/// The arbitration system returns this before action resolution.
/// `Permitted` and `Impossible` come from deterministic rules.
/// `Ambiguous` triggers the small LLM fallback.
#[derive(Debug, Clone)]
pub enum ActionPossibility {
    /// Action is permitted, possibly with conditions.
    Permitted { conditions: Vec<ActionCondition> },
    /// Action is impossible due to a constraint violation.
    Impossible { reason: ConstraintViolation },
    /// Rules engine cannot determine — needs LLM analysis.
    Ambiguous {
        known_constraints: Vec<EnvironmentalConstraint>,
        uncertainty: String,
    },
}

impl ActionPossibility {
    /// Returns true if the action is permitted.
    pub fn is_permitted(&self) -> bool {
        matches!(self, Self::Permitted { .. })
    }

    /// Returns true if the action is impossible.
    pub fn is_impossible(&self) -> bool {
        matches!(self, Self::Impossible { .. })
    }

    /// Returns true if the result is ambiguous.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, Self::Ambiguous { .. })
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-core genre_constraint --all-features && cargo test -p storyteller-core action_possibility --all-features`
Expected: PASS

**Step 5: Run full core tests**

Run: `cargo test -p storyteller-core --all-features`
Expected: All pass

**Step 6: Commit**

```bash
git add crates/storyteller-core/src/types/world_model.rs
git commit -m "feat(core): add GenreConstraint, ActionPossibility, and ConstraintViolation types"
```

---

### Task 6: Capability Lexicon Types

**Files:**
- Create: `crates/storyteller-core/src/types/capability_lexicon.rs`
- Modify: `crates/storyteller-core/src/types/mod.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexicon_entry_matches_synonym() {
        let entry = LexiconEntry {
            capability: "swordsmanship".to_string(),
            synonyms: vec!["fencing".to_string(), "blade work".to_string()],
            action_verbs: vec!["slash".to_string(), "parry".to_string()],
            implied_objects: vec!["rapier".to_string(), "sword".to_string()],
            idiomatic_phrases: vec!["crossed swords".to_string()],
        };
        assert!(entry.matches_token("fencing"));
        assert!(entry.matches_token("slash"));
        assert!(entry.matches_token("rapier"));
        assert!(!entry.matches_token("cooking"));
    }

    #[test]
    fn lexicon_matches_against_text() {
        let mut lexicon = CapabilityLexicon::new();
        lexicon.add(LexiconEntry {
            capability: "swordsmanship".to_string(),
            synonyms: vec![],
            action_verbs: vec!["slash".to_string()],
            implied_objects: vec!["rapier".to_string()],
            idiomatic_phrases: vec![],
        });
        let matches = lexicon.match_text("I dive for the rapier and slash at his hand");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "swordsmanship");
    }

    #[test]
    fn lexicon_returns_empty_for_no_match() {
        let lexicon = CapabilityLexicon::new();
        let matches = lexicon.match_text("I walk through the meadow");
        assert!(matches.is_empty());
    }

    #[test]
    fn lexicon_serializes() {
        let mut lexicon = CapabilityLexicon::new();
        lexicon.add(LexiconEntry {
            capability: "archery".to_string(),
            synonyms: vec!["bowmanship".to_string()],
            action_verbs: vec!["shoot".to_string(), "aim".to_string()],
            implied_objects: vec!["bow".to_string(), "arrow".to_string()],
            idiomatic_phrases: vec![],
        });
        let json = serde_json::to_string(&lexicon).unwrap();
        let roundtrip: CapabilityLexicon = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.entries.len(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-core capability_lexicon --all-features`
Expected: FAIL — module not found

**Step 3: Write the implementation**

Create `crates/storyteller-core/src/types/capability_lexicon.rs`:

```rust
//! Capability lexicon — pre-seeded natural language mappings for authored capabilities.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! At story authoring time, each authored capability (e.g., "swordsmanship")
//! is expanded into synonyms, action verbs, implied objects, and idiomatic
//! phrases. At runtime, capability matching is fast string/token lookup
//! against these pre-computed sets.

use std::collections::BTreeMap;

/// A pre-seeded mapping from authored capability to natural language terms.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LexiconEntry {
    /// The authored capability name.
    pub capability: String,
    /// Direct synonyms: "swordsmanship" → ["fencing", "blade work"].
    pub synonyms: Vec<String>,
    /// Action verbs: "swordsmanship" → ["slash", "parry", "thrust"].
    pub action_verbs: Vec<String>,
    /// Implied objects: "swordsmanship" → ["rapier", "sword", "blade"].
    pub implied_objects: Vec<String>,
    /// Multi-hop phrases: "swordsmanship" → ["crossed swords", "steel rang"].
    pub idiomatic_phrases: Vec<String>,
}

impl LexiconEntry {
    /// Check if a single token matches any term in this entry.
    pub fn matches_token(&self, token: &str) -> bool {
        let lower = token.to_lowercase();
        self.synonyms.iter().any(|s| s.to_lowercase() == lower)
            || self.action_verbs.iter().any(|v| v.to_lowercase() == lower)
            || self.implied_objects.iter().any(|o| o.to_lowercase() == lower)
    }

    /// Check if any term from this entry appears in the given text.
    pub fn matches_text(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.synonyms.iter().any(|s| lower.contains(&s.to_lowercase()))
            || self.action_verbs.iter().any(|v| lower.contains(&v.to_lowercase()))
            || self.implied_objects.iter().any(|o| lower.contains(&o.to_lowercase()))
            || self.idiomatic_phrases.iter().any(|p| lower.contains(&p.to_lowercase()))
    }
}

/// Collection of capability lexicon entries for a story's game design system.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CapabilityLexicon {
    /// Maps capability name to its lexicon entry.
    pub entries: BTreeMap<String, LexiconEntry>,
}

impl CapabilityLexicon {
    /// Create an empty lexicon.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a lexicon entry.
    pub fn add(&mut self, entry: LexiconEntry) {
        self.entries.insert(entry.capability.clone(), entry);
    }

    /// Find all capabilities that match tokens in the given text.
    pub fn match_text(&self, text: &str) -> Vec<String> {
        self.entries
            .values()
            .filter(|entry| entry.matches_text(text))
            .map(|entry| entry.capability.clone())
            .collect()
    }
}
```

Add to `crates/storyteller-core/src/types/mod.rs`:

```rust
pub mod capability_lexicon;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-core capability_lexicon --all-features`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/storyteller-core/src/types/capability_lexicon.rs crates/storyteller-core/src/types/mod.rs
git commit -m "feat(core): add CapabilityLexicon for natural language capability matching"
```

---

### Task 7: Action Arbitration Engine

**Files:**
- Create: `crates/storyteller-engine/src/systems/arbitration.rs`
- Modify: `crates/storyteller-engine/src/systems/mod.rs`

This task implements the deterministic rules engine that checks genre constraints, spatial zones, capabilities, and environmental constraints. Returns `ActionPossibility`.

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::world_model::*;
    use storyteller_core::types::capability_lexicon::*;

    fn test_world_model() -> WorldModel {
        WorldModel {
            genre_physics: vec!["No magic in this world".to_string()],
            spatial_zones: vec![],
            environmental_constraints: vec![],
        }
    }

    fn test_genre_constraints() -> Vec<GenreConstraint> {
        vec![GenreConstraint::Forbidden {
            capability: "telekinesis".to_string(),
            reason: "Magic does not exist".to_string(),
        }]
    }

    #[test]
    fn forbidden_capability_returns_impossible() {
        let constraints = test_genre_constraints();
        let lexicon = {
            let mut l = CapabilityLexicon::new();
            l.add(LexiconEntry {
                capability: "telekinesis".to_string(),
                synonyms: vec!["telekinesis".to_string()],
                action_verbs: vec!["levitate".to_string(), "move with mind".to_string()],
                implied_objects: vec![],
                idiomatic_phrases: vec![],
            });
            l
        };
        let result = check_genre_constraints("I levitate the stone", &constraints, &lexicon);
        assert!(result.is_impossible());
    }

    #[test]
    fn permitted_action_returns_permitted() {
        let constraints = test_genre_constraints();
        let lexicon = CapabilityLexicon::new();
        let result = check_genre_constraints("I pick up the stone", &constraints, &lexicon);
        assert!(result.is_permitted());
    }

    #[test]
    fn spatial_zone_check_blocks_distant_touch() {
        let result = check_spatial_constraints(
            "I reach out and touch her hand",
            NarrativeDistanceZone::Peripheral,
        );
        assert!(result.is_impossible());
    }

    #[test]
    fn spatial_zone_allows_intimate_touch() {
        let result = check_spatial_constraints(
            "I reach out and touch her hand",
            NarrativeDistanceZone::Intimate,
        );
        assert!(result.is_permitted());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine arbitration --all-features`
Expected: FAIL — module not found

**Step 3: Write the implementation**

Create `crates/storyteller-engine/src/systems/arbitration.rs` with:

- `check_genre_constraints(player_input, constraints, lexicon) → ActionPossibility` — checks player input against `GenreConstraint::Forbidden` entries via lexicon matching
- `check_spatial_constraints(player_input, zone) → ActionPossibility` — checks for touch/speech keywords against the narrative distance zone
- `check_action_possibility(player_input, world_model, genre_constraints, lexicon, actor_zone) → ActionPossibility` — orchestrates all checks in order, returns first `Impossible` or `Ambiguous`, else `Permitted`

This is the deterministic layer only — the LLM fallback for `Ambiguous` is wired in Task 8.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine arbitration --all-features`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/storyteller-engine/src/systems/arbitration.rs crates/storyteller-engine/src/systems/mod.rs
git commit -m "feat(engine): add deterministic action arbitration rules engine"
```

---

### Task 8: Wire Event Decomposition into D.3 Committed-Turn Phase

**Files:**
- Modify: `crates/storyteller-engine/src/components/turn.rs` (add `StructuredLlmResource`)
- Modify: `crates/storyteller-engine/src/systems/turn_cycle.rs` (modify `commit_previous_system`)

This task integrates the small LLM event decomposition into the existing D.3 committed-turn classification. When `StructuredLlmResource` is available, use LLM decomposition instead of DistilBERT for the committed-turn pass. Falls back to DistilBERT when unavailable.

**Step 1: Add the Bevy Resource**

In `crates/storyteller-engine/src/components/turn.rs`, add:

```rust
use std::sync::Arc;
use storyteller_core::traits::structured_llm::StructuredLlmProvider;

/// Bevy Resource: structured LLM provider for event decomposition and action arbitration.
///
/// Optional — when absent, event decomposition falls back to DistilBERT
/// and action arbitration skips the LLM fallback for ambiguous cases.
#[derive(Resource, Clone)]
pub struct StructuredLlmResource(pub Arc<dyn StructuredLlmProvider>);

impl std::fmt::Debug for StructuredLlmResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuredLlmResource").finish()
    }
}
```

**Step 2: Modify commit_previous_system to accept StructuredLlmResource**

In `turn_cycle.rs`, update the `commit_previous_system` signature to accept the optional resource and use it for D.3 when available.

The key change in `commit_previous_system` (around lines 326-348): when `StructuredLlmResource` is present AND a `TokioRuntime` handle is available, dispatch the combined text to the small LLM for decomposition instead of using DistilBERT. Since this is async and the system is sync, use the same oneshot pattern as the narrator task — but since we accept a one-turn delay, we can block on the result here (the commit system runs before the current turn's narrator call, so blocking briefly is acceptable).

Alternatively, spawn the LLM decomposition as a background task and collect results in the *next* turn's commit phase. This avoids blocking but requires tracking the pending decomposition.

**Recommendation:** Start with the simpler blocking approach. The small LLM call should complete in ~500ms-1s, which is acceptable in the commit phase before the narrator's ~2-8s call. If latency becomes an issue, switch to background task later.

**Step 3: Write tests**

Add tests to `turn_cycle.rs` that verify:
- When `StructuredLlmResource` is absent, behavior is unchanged (DistilBERT fallback)
- The system function signature accepts the new optional resource
- CompletedTurn still gets populated correctly

**Step 4: Run full turn cycle tests**

Run: `cargo test -p storyteller-engine turn_cycle --all-features`
Expected: All existing tests pass, new tests pass

**Step 5: Commit**

```bash
git add crates/storyteller-engine/src/components/turn.rs crates/storyteller-engine/src/systems/turn_cycle.rs
git commit -m "feat(engine): wire event decomposition LLM into D.3 committed-turn phase"
```

---

### Task 9: Wire Arbitration into Resolution Phase

**Files:**
- Modify: `crates/storyteller-engine/src/systems/turn_cycle.rs` (modify `resolve_system`)

Update `resolve_system` to run the arbitration check before wrapping predictions into `ResolverOutput`. When an action is `Impossible`, inject the constraint violation into the resolver output so the narrator can narrate it. When `Ambiguous` and `StructuredLlmResource` is available, call the small LLM for a ruling.

**Step 1: Update resolve_system**

The current `resolve_system` (lines 201-215) is a pass-through. Add arbitration check before it:

```rust
pub fn resolve_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut turn_ctx: ResMut<TurnContext>,
    // New optional resources for arbitration:
    structured_llm: Option<Res<StructuredLlmResource>>,
    runtime: Option<Res<TokioRuntime>>,
) {
    // If we have player input, run arbitration check
    if let Some(ref input) = turn_ctx.player_input {
        // Run deterministic arbitration (genre constraints, spatial zones, etc.)
        // If Ambiguous and structured_llm available, call LLM fallback
        // Store result in turn_ctx for context assembly
    }

    // Existing pass-through logic
    let predictions = turn_ctx.predictions.clone().unwrap_or_default();
    let resolver_output = ResolverOutput { /* ... */ };
    turn_ctx.resolver_output = Some(resolver_output);
    stage.0 = stage.0.next();
}
```

**Step 2: Add arbitration_result to TurnContext**

In `components/turn.rs`, add:

```rust
pub struct TurnContext {
    // ... existing fields ...
    /// Action arbitration result (if arbitration was run).
    pub arbitration: Option<ActionPossibility>,
}
```

**Step 3: Write tests**

Test that:
- resolve_system still advances stage correctly
- When no arbitration resources present, behavior unchanged
- arbitration field populated when resources available

**Step 4: Run tests**

Run: `cargo test -p storyteller-engine turn_cycle --all-features`
Expected: All pass

**Step 5: Commit**

```bash
git add crates/storyteller-engine/src/systems/turn_cycle.rs crates/storyteller-engine/src/components/turn.rs
git commit -m "feat(engine): wire action arbitration into resolution phase"
```

---

### Task 10: Integration Test with Ollama

**Files:**
- Create: `crates/storyteller-engine/tests/event_decomposition_integration.rs` (or add to existing integration test file)

**Step 1: Write integration test (feature-gated)**

```rust
//! Integration test for event decomposition via small LLM.
//! Requires: Ollama running with qwen2.5:3b-instruct model.
//! Run with: cargo test --features test-llm -p storyteller-engine event_decomposition_integration

#[cfg(feature = "test-llm")]
mod event_decomposition_integration {
    use storyteller_core::traits::structured_llm::StructuredLlmConfig;
    use storyteller_engine::inference::structured::OllamaStructuredProvider;
    use storyteller_engine::inference::event_decomposition::*;

    #[tokio::test]
    async fn decompose_speech_act() {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let result = decompose_events(&provider, "I say to Pyotir, 'Do you remember this melody?'").await;
        assert!(result.is_ok(), "Decomposition failed: {result:?}");
        let decomp = result.unwrap();
        assert!(!decomp.events.is_empty(), "No events extracted");
        // Should identify at least a SpeechAct
        assert!(
            decomp.events.iter().any(|e| e.kind == "SpeechAct"),
            "Expected SpeechAct, got: {:?}", decomp.events
        );
    }

    #[tokio::test]
    async fn decompose_directed_emotional_expression() {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let result = decompose_events(&provider, "The child laughs at the small sprite in the corner.").await;
        assert!(result.is_ok());
        let decomp = result.unwrap();
        assert!(!decomp.events.is_empty());
        // Should identify directed action with actor and target
        let event = &decomp.events[0];
        assert!(event.actor.is_some(), "Expected actor");
        assert!(event.target.is_some(), "Expected target for directed action");
    }

    #[tokio::test]
    async fn decompose_self_directed_action() {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let result = decompose_events(&provider, "Walking through the sunlit meadow, she laughs aloud with pure joy.").await;
        assert!(result.is_ok());
        let decomp = result.unwrap();
        assert!(!decomp.events.is_empty());
    }

    #[tokio::test]
    async fn decompose_multi_entity_action() {
        let provider = OllamaStructuredProvider::new(StructuredLlmConfig::default());
        let result = decompose_events(
            &provider,
            "I dive for the rapier, roll across the floor, and slash the weapon from his hand.",
        ).await;
        assert!(result.is_ok());
        let decomp = result.unwrap();
        assert!(decomp.entities.len() >= 2, "Expected at least rapier and hand/weapon");
    }
}
```

**Step 2: Run integration tests**

Run: `cargo test --features test-llm -p storyteller-engine event_decomposition_integration`
Expected: PASS (requires Ollama with qwen2.5:3b-instruct running)

**Step 3: Iterate on system prompt if tests reveal issues**

If the LLM produces unexpected output, adjust the system prompt in `event_decomposition.rs` and re-run. This is the prompt engineering iteration loop.

**Step 4: Commit**

```bash
git add crates/storyteller-engine/tests/
git commit -m "test(engine): add integration tests for event decomposition via small LLM"
```

---

### Task 11: Workspace Verification and Cleanup

**Step 1: Run full workspace tests**

Run: `cargo test --workspace --all-features`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: Clean

**Step 4: Commit any fixes**

If clippy or fmt caught anything, fix and commit.

**Step 5: Push branch**

```bash
git push
```

---

## Task Dependency Graph

```
Task 1 (RelationalDirection) ──→ Task 4 (Event Decomposition Types)
Task 2 (StructuredLlmProvider trait) ──→ Task 3 (Ollama Implementation)
Task 3 ──→ Task 4
Task 4 ──→ Task 8 (Wire into D.3)
Task 5 (GenreConstraint + ActionPossibility) ──→ Task 7 (Arbitration Engine)
Task 6 (Capability Lexicon) ──→ Task 7
Task 7 ──→ Task 9 (Wire into Resolution)
Task 8 ──→ Task 10 (Integration Tests)
Task 9 ──→ Task 10
Task 10 ──→ Task 11 (Verification)
```

Tasks 1-2 can run in parallel. Tasks 5-6 can run in parallel with 1-4.
