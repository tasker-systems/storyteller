# Character Predictions as Intent Statements Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace raw ML prediction markdown in the narrator prompt with natural language intent statements from a 3b-instruct model, and replace events.jsonl with a richer turns.jsonl for session reload.

**Architecture:** Two independent workstreams. Workstream 1 inserts an intent synthesizer (3b-instruct via Ollama) between ML predictions and narrator rendering — it reads predictions + character data and produces per-character behavioral directives. Workstream 2 replaces the overloaded events.jsonl with turns.jsonl (one complete record per turn, including predictions and intents) and updates session reload to return full turn history.

**Tech Stack:** Rust (storyteller-core, storyteller-engine, storyteller-workshop), ONNX Runtime, Ollama (qwen2.5:3b-instruct), Tauri, Svelte 5 / TypeScript

**Spec:** `docs/plans/2026-03-10-character-predictions-as-suggestions-design.md`

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `crates/storyteller-engine/src/inference/intent_synthesis.rs` | Intent synthesis prompt, Ollama call, response handling |

### Modified Files
| File | Changes |
|------|---------|
| `crates/storyteller-core/src/types/resolver.rs` | Add `intent_statements: Option<String>` to `ResolverOutput` |
| `crates/storyteller-engine/src/inference/mod.rs` | Export `intent_synthesis` module |
| `crates/storyteller-engine/src/agents/narrator.rs` | Update system prompt + `build_turn_message()` to use intent statements |
| `crates/storyteller-engine/src/context/mod.rs` | Pass intent statements through context assembly |
| `crates/storyteller-workshop/src-tauri/src/commands.rs` | Wire intent synthesis, write turns.jsonl, update resume_session |
| `crates/storyteller-workshop/src-tauri/src/session.rs` | Add `turns_path()`, TurnRecord serde type |
| `crates/storyteller-workshop/src/lib/types.ts` | Add TurnRecord type, update resumeSession return type |
| `crates/storyteller-workshop/src/lib/api.ts` | Update `resumeSession()` return type |
| `crates/storyteller-workshop/src/lib/PlayScene.svelte` | Hydrate chat from turn history on resume |

---

## Chunk 1: Core Types + Intent Synthesis

### Task 1: Add `intent_statements` to ResolverOutput

**Files:**
- Modify: `crates/storyteller-core/src/types/resolver.rs`

- [ ] **Step 1: Read the current ResolverOutput definition**

Read `crates/storyteller-core/src/types/resolver.rs` to see the current struct and its derives.

- [ ] **Step 2: Add the new field**

Add `intent_statements: Option<String>` to `ResolverOutput`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverOutput {
    pub sequenced_actions: Vec<ResolvedCharacterAction>,
    pub original_predictions: Vec<CharacterPrediction>,
    pub scene_dynamics: String,
    pub conflicts: Vec<ConflictResolution>,
    /// Natural language intent statements from the intent synthesizer.
    /// When `Some`, the narrator uses these instead of raw predictions.
    /// When `None`, falls back to `render_predictions()` on `original_predictions`.
    pub intent_statements: Option<String>,
}
```

- [ ] **Step 3: Fix all construction sites**

Search for `ResolverOutput {` across the workspace. Every construction site needs `intent_statements: None` added. Known locations (~15 sites):
- `crates/storyteller-core/src/types/resolver.rs` (1 test)
- `crates/storyteller-engine/src/agents/narrator.rs` (1 test mock_context)
- `crates/storyteller-engine/src/context/mod.rs` (1 test)
- `crates/storyteller-engine/src/systems/turn_cycle.rs` (3 locations)
- `crates/storyteller-engine/src/systems/rendering.rs` (1 test)
- `crates/storyteller-cli/src/bin/play_scene_context.rs` (3 locations)
- `crates/storyteller-workshop/src-tauri/src/commands.rs` (3 locations)
- `crates/storyteller-storykeeper/src/in_memory.rs` (1 test)

- [ ] **Step 4: Verify compilation**

Run: `cargo check --workspace --all-features`
Expected: Clean compilation with no errors.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace --all-features`
Expected: All existing tests pass (no behavioral change yet).

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(core): add intent_statements field to ResolverOutput"
```

---

### Task 2: Create intent synthesis module

**Files:**
- Create: `crates/storyteller-engine/src/inference/intent_synthesis.rs`
- Modify: `crates/storyteller-engine/src/inference/mod.rs`

- [ ] **Step 1: Write the test for prompt construction**

Add tests at the bottom of the new file. The prompt builder should combine character data, predictions, journal tail, player input, and scene context into a single prompt string.

```rust
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
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-engine intent_synthesis --all-features`
Expected: FAIL — module doesn't exist yet.

- [ ] **Step 3: Add module export**

In `crates/storyteller-engine/src/inference/mod.rs`, add:
```rust
pub mod intent_synthesis;
```

- [ ] **Step 4: Implement prompt construction**

Create `crates/storyteller-engine/src/inference/intent_synthesis.rs`:

```rust
//! Intent synthesis — converts ML predictions into natural language
//! behavioral directives for the narrator.
//!
//! Uses a 3b-instruct model (same as event decomposition) to produce
//! per-character intent statements that tell the narrator what each
//! character wants to do and why.

use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::prediction::CharacterPrediction;

/// System prompt for the intent synthesizer.
pub fn intent_synthesis_system_prompt() -> String {
    String::from(
        "You are the Intent Synthesizer — a dramaturgical assistant preparing \
         a briefing for a narrator.\n\n\
         You receive:\n\
         - Character data: personality traits, emotional state, relationships\n\
         - ML predictions: what a behavior model predicts each character will do\n\
         - Recent scene history: what just happened\n\
         - Player input: what the player character just did or said\n\n\
         Your job: Write a brief directive for each non-player character \
         describing what they WANT to do this turn and WHY.\n\n\
         Rules:\n\
         - Be directive: \"Arthur should respond\" not \"Arthur might respond\"\n\
         - Be specific about emotional subtext: \"reluctantly, deflecting with \
           humor\" not \"with some emotion\"\n\
         - Include speech direction when a character should speak: \"should say \
           something about...\" not prescribing exact words\n\
         - Ground in physical behavior: \"his shoulders drop\" not \"he feels sad\"\n\
         - One paragraph per character, 2-4 sentences each\n\
         - Do NOT write dialogue. The narrator writes all dialogue.\n\
         - Do NOT narrate the scene. You are briefing the narrator, not writing prose.\n\n\
         Format each character section as:\n\
         **CharacterName** directive paragraph here.",
    )
}

/// Build the user prompt for intent synthesis from assembled context.
pub fn build_intent_user_prompt(
    character_summary: &str,
    predictions_summary: &str,
    journal_tail: &str,
    player_input: &str,
    scene_context: &str,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("## Characters\n");
    prompt.push_str(character_summary);
    prompt.push_str("\n\n");

    prompt.push_str("## ML Predictions\n");
    prompt.push_str(predictions_summary);
    prompt.push_str("\n\n");

    prompt.push_str("## Recent Scene History\n");
    prompt.push_str(journal_tail);
    prompt.push_str("\n\n");

    prompt.push_str("## Player Input (this turn)\n");
    prompt.push_str(player_input);
    prompt.push_str("\n\n");

    prompt.push_str("## Scene Context\n");
    prompt.push_str(scene_context);
    prompt.push_str("\n\n");

    prompt.push_str("Write intent directives for each non-player character.");

    prompt
}

/// Summarize a character's tensor data into a compact string for the
/// intent synthesizer. Includes name, key personality axes (from tensor),
/// voice note, and performance notes.
///
/// Note: `CharacterSheet` has no `archetype` field. Role/archetype info
/// comes from the scene's cast entries, not the sheet itself.
/// `TensorEntry.value` is an `AxisValue` with `central_tendency: f32`.
pub fn summarize_character(sheet: &CharacterSheet) -> String {
    let mut parts = vec![sheet.name.clone()];

    // Top personality axes (up to 4, sorted by central_tendency descending)
    let mut axes: Vec<(&String, f32)> = sheet
        .tensor
        .axes
        .iter()
        .map(|(name, entry)| (name, entry.value.central_tendency))
        .collect();
    axes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_axes: Vec<String> = axes
        .iter()
        .take(4)
        .map(|(name, val)| format!("{}={:.1}", name, val))
        .collect();
    if !top_axes.is_empty() {
        parts.push(top_axes.join(", "));
    }

    // Voice
    if !sheet.voice.is_empty() {
        parts.push(format!("voice: {}", sheet.voice));
    }

    // Performance notes (authorial guidance)
    if !sheet.performance_notes.is_empty() {
        parts.push(format!("notes: {}", sheet.performance_notes));
    }

    parts.join(" | ")
}

/// Summarize ML predictions for a character into a compact string.
pub fn summarize_prediction(pred: &CharacterPrediction) -> String {
    let mut parts = vec![pred.character_name.clone()];

    // Actions
    for action in &pred.actions {
        parts.push(format!(
            "Action: {} ({:.0}%)",
            action.description,
            action.confidence * 100.0
        ));
    }

    // Speech
    if let Some(speech) = &pred.speech {
        parts.push(format!(
            "Speech: {:?} — {} ({:.0}%)",
            speech.register,
            speech.content_direction,
            speech.confidence * 100.0
        ));
    } else {
        parts.push("Speech: none predicted".to_string());
    }

    // Emotional state
    parts.push(format!("Internal: {}", pred.thought.emotional_subtext));

    // Emotional shifts
    if !pred.emotional_deltas.is_empty() {
        let shifts: Vec<String> = pred
            .emotional_deltas
            .iter()
            .map(|d| {
                let sign = if d.intensity_change >= 0.0 { "+" } else { "" };
                format!("{} {sign}{:.1}", d.primary_id, d.intensity_change)
            })
            .collect();
        parts.push(format!("Shifts: {}", shifts.join(", ")));
    }

    parts.join(" | ")
}

/// Build the complete character and prediction summaries for all
/// non-player characters in the scene.
pub fn build_summaries(
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    player_entity_id: Option<storyteller_core::types::entity::EntityId>,
) -> (String, String) {
    let mut char_lines = Vec::new();
    let mut pred_lines = Vec::new();

    for sheet in characters {
        // Skip player character
        if let Some(pid) = player_entity_id {
            if sheet.entity_id == pid {
                continue;
            }
        }

        char_lines.push(summarize_character(sheet));

        if let Some(pred) = predictions.iter().find(|p| p.character_id == sheet.entity_id) {
            pred_lines.push(summarize_prediction(pred));
        }
    }

    (char_lines.join("\n"), pred_lines.join("\n"))
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine intent_synthesis --all-features`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs crates/storyteller-engine/src/inference/mod.rs
git commit -m "feat(engine): add intent synthesis prompt construction and summarizers"
```

---

### Task 3: Add async synthesis function

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs`

- [ ] **Step 1: Write the test for the synthesis function**

Add to the test module in `intent_synthesis.rs`:

```rust
    #[test]
    fn build_summaries_skips_player_character() {
        // Use the workshop fixtures which provide fully-constructed CharacterSheets
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();

        let player_id = bramblehoof.entity_id;

        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let predictions = vec![]; // empty predictions are fine for this test

        let (char_summary, _) = build_summaries(&characters, &predictions, Some(player_id));

        assert!(!char_summary.contains("Bramblehoof"), "Player should be excluded");
        assert!(char_summary.contains("Pyotir"), "NPC should be included");
    }
```

Note: Uses `crate::workshop::the_flute_kept` fixtures which provide fully-constructed `CharacterSheet` instances. No `test_sheet()` helper needed.

- [ ] **Step 2: Implement the async `synthesize_intents` function**

This is the main entry point called from `commands.rs`. It takes the Ollama provider, characters, predictions, journal, player input, and scene data, and returns `Option<String>`.

```rust
use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};

/// Synthesize intent statements from ML predictions using a 3b-instruct model.
///
/// Returns `Some(intent_text)` on success, `None` on failure (with warning logged).
/// The caller should fall back to `render_predictions()` when this returns `None`.
pub async fn synthesize_intents(
    llm: &dyn LlmProvider,
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    journal_tail: &str,
    player_input: &str,
    scene: &SceneData,
    player_entity_id: Option<storyteller_core::types::entity::EntityId>,
) -> Option<String> {
    let (char_summary, pred_summary) = build_summaries(characters, predictions, player_entity_id);

    if char_summary.is_empty() {
        tracing::debug!("No non-player characters — skipping intent synthesis");
        return None;
    }

    // Scene context from stakes and constraints
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
        Ok(response) => {
            tracing::debug!(
                tokens = response.tokens_used,
                "intent synthesis complete"
            );
            Some(response.content)
        }
        Err(e) => {
            tracing::warn!(error = %e, "intent synthesis failed, falling back to raw predictions");
            None
        }
    }
}

/// Build scene context string from stakes and constraints.
fn build_scene_context(scene: &SceneData) -> String {
    let mut parts = Vec::new();

    if !scene.stakes.is_empty() {
        parts.push(format!("Stakes: {}", scene.stakes.join(", ")));
    }

    for constraint in &scene.constraints.hard {
        parts.push(format!("Constraint: {}", constraint));
    }

    for constraint in &scene.constraints.soft {
        parts.push(format!("Context: {}", constraint));
    }

    parts.join("\n")
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --workspace --all-features`
Expected: Clean compilation. The `CharacterSheet` may need adjustments for the `archetype` field access — check the actual struct definition and adjust `summarize_character` accordingly.

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace --all-features`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "feat(engine): add async synthesize_intents function with Ollama provider"
```

---

### Task 4: Update narrator to use intent statements

**Files:**
- Modify: `crates/storyteller-engine/src/agents/narrator.rs`

- [ ] **Step 1: Read the current narrator.rs**

Read `crates/storyteller-engine/src/agents/narrator.rs` to see `build_system_prompt()` and `build_turn_message()`.

- [ ] **Step 2: Update system prompt**

In `build_system_prompt()`, change the `## Your Task` section. Replace:

```
You receive structured facts about what characters did, said, and felt.
You render only what is observable — physical actions, speech, gestures.
```

With:

```
You receive intent statements describing what each character wants to do
this turn. Honor these intents — render them with each character's full
agency. Characters act, speak, and drive the scene. They are not scenery.

Render only what is observable — physical actions, speech, gestures.
```

Keep the rest of the system prompt unchanged.

- [ ] **Step 3: Update `build_turn_message()` to prefer intent statements**

Replace the character predictions block (lines ~253-259) with logic that checks for intent statements first:

```rust
    // Character behavioral directives — prefer synthesized intents over raw predictions
    if let Some(intents) = &context.resolver_output.intent_statements {
        message.push_str("## Character Intents\n");
        message.push_str(intents);
        message.push_str("\n\n");
    } else if !context.resolver_output.original_predictions.is_empty() {
        // Fallback: raw ML prediction rendering
        let predictions_md = crate::context::prediction::render_predictions(
            &context.resolver_output.original_predictions,
        );
        message.push_str(&predictions_md);
    }
```

- [ ] **Step 4: Update tests**

Update the `turn_message_has_all_sections` test to verify intent statement rendering. Add a new test:

```rust
    #[test]
    fn turn_message_prefers_intent_statements_over_predictions() {
        let mut context = mock_context();
        context.resolver_output.intent_statements =
            Some("**Pyotir** should greet Bramblehoof warmly.".to_string());

        let message = build_turn_message(&context);

        assert!(message.contains("## Character Intents"));
        assert!(message.contains("should greet Bramblehoof warmly"));
        assert!(!message.contains("## Character Predictions"));
    }

    #[test]
    fn turn_message_falls_back_to_predictions_when_no_intents() {
        use storyteller_core::types::prediction::{
            ActivatedTensorFrame, CharacterPrediction, ThoughtPrediction,
        };
        use storyteller_core::types::tensor::AwarenessLevel;

        let mut context = mock_context();
        context.resolver_output.intent_statements = None;
        // Add a prediction so fallback has something to render.
        // Construct manually — these types do not derive Default.
        context.resolver_output.original_predictions = vec![
            CharacterPrediction {
                character_id: EntityId::new(),
                character_name: "Pyotir".to_string(),
                frame: ActivatedTensorFrame {
                    activated_axes: vec!["stoicism".to_string()],
                    activation_reason: "Active in context".to_string(),
                    confidence: 0.8,
                },
                actions: vec![],
                speech: None,
                thought: ThoughtPrediction {
                    emotional_subtext: "Pyotir senses calm".to_string(),
                    awareness_level: AwarenessLevel::Recognizable,
                    internal_conflict: None,
                },
                emotional_deltas: vec![],
            },
        ];

        let message = build_turn_message(&context);

        // Should fall back to prediction rendering
        assert!(message.contains("## Character Predictions") || message.contains("Pyotir"));
        assert!(!message.contains("## Character Intents"));
    }
```

- [ ] **Step 5: Update system prompt test**

Update `system_prompt_has_preamble` test to check for the new task language:

```rust
    assert!(prompt.contains("intent statements"));
    assert!(prompt.contains("Honor these intents"));
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p storyteller-engine narrator --all-features`
Expected: All narrator tests pass.

- [ ] **Step 7: Run full workspace tests**

Run: `cargo test --workspace --all-features`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/storyteller-engine/src/agents/narrator.rs
git commit -m "feat(engine): narrator prefers intent statements over raw predictions"
```

---

## Chunk 2: turns.jsonl Persistence

### Task 5: Add TurnRecord type and turns_path to SessionStore

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/session.rs`

- [ ] **Step 1: Read current session.rs**

Read `crates/storyteller-workshop/src-tauri/src/session.rs`.

- [ ] **Step 2: Add TurnRecord struct**

Add a serializable turn record struct. This is the shape of each line in `turns.jsonl`:

```rust
use serde::{Deserialize, Serialize};

/// A single turn record persisted to turns.jsonl.
///
/// Turn 0 is the opening narration (player_input is None).
/// Subsequent turns contain the full pipeline output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    pub turn: u32,
    pub player_input: Option<String>,
    pub narrator_output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predictions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_statements: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifications: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decomposition: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arbitration: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_assembly: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<serde_json::Value>,
    pub timestamp: String,
}
```

- [ ] **Step 3: Add `turns_path()` method to SessionStore**

```rust
    /// Path to the turns.jsonl file for a session.
    pub fn turns_path(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id).join("turns.jsonl")
    }
```

- [ ] **Step 4: Add `append_turn()` method**

```rust
    /// Append a turn record to turns.jsonl.
    pub fn append_turn(&self, session_id: &str, record: &TurnRecord) -> Result<(), String> {
        let path = self.turns_path(session_id);
        let line = serde_json::to_string(record).map_err(|e| e.to_string())?;

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("Failed to open turns.jsonl: {e}"))?;

        writeln!(file, "{}", line).map_err(|e| format!("Failed to write turn: {e}"))?;
        Ok(())
    }
```

- [ ] **Step 5: Add `load_turns()` method**

```rust
    /// Load all turn records from turns.jsonl.
    ///
    /// Returns an empty vec if the file doesn't exist yet.
    pub fn load_turns(&self, session_id: &str) -> Result<Vec<TurnRecord>, String> {
        let path = self.turns_path(session_id);
        if !path.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read turns.jsonl: {e}"))?;

        let mut records = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record: TurnRecord = serde_json::from_str(line)
                .map_err(|e| format!("Failed to parse turn at line {}: {e}", i + 1))?;
            records.push(record);
        }

        Ok(records)
    }
```

- [ ] **Step 6: Write roundtrip test**

Note: Use `std::env::temp_dir()` with a unique subdirectory (not `tempfile` crate, which isn't in dev-deps). Follow the existing session test pattern.

```rust
    #[test]
    fn turn_record_roundtrip() {
        let dir = std::env::temp_dir().join(format!("storyteller-test-{}", uuid::Uuid::now_v7()));
        let store = SessionStore::new(&dir).unwrap();

        // Create a minimal session directory
        let session_id = "test-session-001";
        std::fs::create_dir_all(dir.join(".story/sessions").join(session_id)).unwrap();

        let turn0 = TurnRecord {
            turn: 0,
            player_input: None,
            narrator_output: "The old rectory stands quiet.".to_string(),
            predictions: None,
            intent_statements: None,
            classifications: None,
            decomposition: None,
            arbitration: None,
            context_assembly: None,
            timing: None,
            timestamp: "2026-03-10T21:00:00Z".to_string(),
        };

        let turn1 = TurnRecord {
            turn: 1,
            player_input: Some("Margaret enters.".to_string()),
            narrator_output: "Margaret steps through the door.".to_string(),
            predictions: Some(serde_json::json!([{"character_name": "Arthur"}])),
            intent_statements: Some("**Arthur** should look up warily.".to_string()),
            classifications: Some(vec!["SpeechAct: 0.62".to_string()]),
            decomposition: None,
            arbitration: Some(serde_json::json!({"verdict": "Permitted"})),
            context_assembly: None,
            timing: Some(serde_json::json!({"narrator_ms": 5000})),
            timestamp: "2026-03-10T21:01:00Z".to_string(),
        };

        store.append_turn(session_id, &turn0).unwrap();
        store.append_turn(session_id, &turn1).unwrap();

        let loaded = store.load_turns(session_id).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].turn, 0);
        assert!(loaded[0].player_input.is_none());
        assert_eq!(loaded[1].turn, 1);
        assert_eq!(loaded[1].player_input.as_deref(), Some("Margaret enters."));
        assert!(loaded[1].intent_statements.is_some());
    }
```

Note: The `SessionStore::new()` takes a workshop root path and creates `.story/sessions/` under it. Check the actual constructor to ensure the test sets up the right directory structure.

- [ ] **Step 7: Update `list_sessions()` to read `turns.jsonl`**

In `list_sessions()`, the turn count is currently derived from `events.jsonl` line count (lines 134-139). Update to check `turns.jsonl` first, falling back to `events.jsonl`:

```rust
            // Count turns — prefer turns.jsonl, fall back to events.jsonl
            let turns_path = path.join("turns.jsonl");
            let events_path = path.join("events.jsonl");
            let turn_count = if turns_path.exists() {
                match fs::read_to_string(&turns_path) {
                    Ok(s) => s.lines().filter(|l| !l.is_empty()).count(),
                    Err(_) => 0,
                }
            } else {
                match fs::read_to_string(&events_path) {
                    Ok(s) => s.lines().filter(|l| !l.is_empty()).count(),
                    Err(_) => 0,
                }
            };
```

- [ ] **Step 8: Update `create_session()` to not create `events.jsonl`**

In `create_session()`, find where `events.jsonl` is created (empty file) and remove that line. The `append_turn()` method uses `create(true)` so `turns.jsonl` is created on first write.

- [ ] **Step 9: Run tests**

Run: `cargo test -p storyteller-workshop --all-features`
Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/session.rs
git commit -m "feat(workshop): add TurnRecord type and turns.jsonl persistence"
```

---

### Task 6: Write turn 0 in setup_and_render_opening

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Read setup_and_render_opening**

Read `crates/storyteller-workshop/src-tauri/src/commands.rs` lines 725-920 to understand the current flow.

- [ ] **Step 2: Add turn 0 persistence**

After the opening narration is generated and before the function returns `SceneInfo`, persist turn 0 to `turns.jsonl` if a session is active.

**Important code details from reading `setup_and_render_opening()`:**
- The narrator output is in `opening.text` (not `rendering.text`)
- Narrator timing is in `narrator_ms` (not `narrator_elapsed_ms`)
- `setup_and_render_opening` does NOT receive `session_store` as a parameter — it only has `app`, `scene`, `characters`, `state`, `session_id`. You must either: (a) add `session_store: &SessionStore` as a parameter and update all callers (`compose_scene`, `resume_session`), or (b) extract it from `app.state::<SessionStore>()`.

```rust
    // Persist turn 0 (opening narration) to turns.jsonl
    if let Some(sid) = &session_id {
        // Option (a): pass session_store as parameter
        // Option (b): extract from app state
        let turn0 = crate::session::TurnRecord {
            turn: 0,
            player_input: None,
            narrator_output: opening.text.clone(),
            predictions: None,
            intent_statements: None,
            classifications: None,
            decomposition: None,
            arbitration: None,
            context_assembly: None,
            timing: Some(serde_json::json!({
                "narrator_ms": narrator_ms,
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = session_store.append_turn(sid, &turn0) {
            tracing::warn!(error = %e, "Failed to persist turn 0");
        }
    }
```

Read the function signature and choose option (a) or (b) based on what's cleanest. Option (a) is more explicit but requires updating callers.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: Clean.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): persist turn 0 opening narration to turns.jsonl"
```

---

### Task 7: Write turn N in submit_input (replace events.jsonl writes)

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Read the persistence section of submit_input**

Read `crates/storyteller-workshop/src-tauri/src/commands.rs` lines 623-686 to see the current events.jsonl write logic.

- [ ] **Step 2: Replace events.jsonl write with turns.jsonl write**

Replace the `events.jsonl` append block with a `turns.jsonl` append. The key difference: we now include `predictions` and `intent_statements` in the record.

**Important variable name mapping** (read `submit_input` to confirm current names):
- Turn number: `engine.turn_count` (stored as `turn` in the current events block)
- Narrator output: `rendering.text`
- Classifications: `classifications` (a `Vec<String>`, not `persisted_classifications`)
- Arbitration: `arbitration_json` (a `serde_json::Value`)
- Token counts: extracted via `extract_token_counts()` as a tuple `(preamble, journal, retrieved, total)`, not named fields
- Timing: `prediction_ms`, `assembly_ms`, `narrator_ms` — `intent_ms` and `decomposition_ms` are new variables you add in this task

```rust
    // Persist turn to turns.jsonl (replaces events.jsonl)
    if let Some(sid) = &engine.session_id {
        let turn_record = crate::session::TurnRecord {
            turn: engine.turn_count,
            player_input: Some(input.clone()),
            narrator_output: rendering.text.clone(),
            predictions: serde_json::to_value(&resolver_output.original_predictions).ok(),
            intent_statements: resolver_output.intent_statements.clone(),
            classifications: Some(classifications.clone()),
            decomposition: decomposition_json.clone(),
            arbitration: Some(arbitration_json.clone()),
            context_assembly: Some(serde_json::json!({
                "preamble_tokens": token_counts.0,
                "journal_tokens": token_counts.1,
                "retrieved_tokens": token_counts.2,
                "total_tokens": token_counts.3,
            })),
            timing: Some(serde_json::json!({
                "prediction_ms": prediction_ms,
                "intent_ms": intent_ms,
                "assembly_ms": assembly_ms,
                "narrator_ms": narrator_ms,
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = session_store.append_turn(sid, &turn_record) {
            tracing::warn!(error = %e, "Failed to persist turn {}", engine.turn_count);
        }
    }
```

Remove the old `events.jsonl` write block entirely.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: Clean (warnings about unused `events_path` are OK — we'll handle backward compat later).

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): write turns.jsonl instead of events.jsonl for turn persistence"
```

---

## Chunk 3: Workshop Integration + Session Reload

### Task 8: Wire intent synthesis into submit_input

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Read the prediction phase in submit_input**

Read `crates/storyteller-workshop/src-tauri/src/commands.rs` lines 294-340 (prediction phase) and lines 505-579 (context assembly phase).

- [ ] **Step 2: Create a separate LLM provider for intent synthesis**

**Critical:** `OllamaStructuredProvider` implements only `StructuredLlmProvider` (JSON extraction via `extract()`). `synthesize_intents()` needs `LlmProvider` (plain completion via `complete()`). These are different traits.

The solution: create a second `ExternalServerProvider` configured for `qwen2.5:3b-instruct` and store it in `EngineState` as `intent_llm: Option<Arc<dyn LlmProvider>>`.

In `setup_and_render_opening()`, after creating the structured LLM provider, create the intent provider:

```rust
    // Intent synthesis LLM — same 3b model, plain completion (not structured)
    let intent_llm: Option<Arc<dyn LlmProvider>> = if let Ok(ollama_url) = std::env::var("OLLAMA_URL") {
        Some(Arc::new(ExternalServerProvider::new(
            &ollama_url,
            "qwen2.5:3b-instruct",
        )))
    } else {
        None
    };
```

Add to `EngineState`:
```rust
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
```

- [ ] **Step 3: Add intent synthesis after ML predictions**

After the ML prediction phase and before context assembly:

```rust
    // Phase: Intent Synthesis
    let intent_start = std::time::Instant::now();
    let journal_tail = engine
        .journal
        .entries
        .iter()
        .rev()
        .take(2)
        .map(|e| e.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let intent_statements = if let Some(ref intent_llm) = engine.intent_llm {
        crate::inference::intent_synthesis::synthesize_intents(
            intent_llm.as_ref(),
            &characters_refs,
            &predictions,
            &journal_tail,
            &input,
            &engine.scene,
            player_entity_id,
        )
        .await
    } else {
        None
    };
    let intent_ms = intent_start.elapsed().as_millis() as u64;

    // Build resolver output with intent statements
    let resolver_output = ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: predictions,
        scene_dynamics: "ML-predicted character behavior".to_string(),
        conflicts: vec![],
        intent_statements,
    };
```

- [ ] **Step 3: Emit intent debug event**

After intent synthesis, emit a debug event so the workshop inspector can show it:

```rust
    // Emit intent synthesis debug event
    if let Some(ref intents) = resolver_output.intent_statements {
        let _ = app_handle.emit("workshop:debug", serde_json::json!({
            "type": "IntentSynthesized",
            "turn": turn_number,
            "intent_statements": intents,
            "timing_ms": intent_elapsed,
        }));
    }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: Clean.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace --all-features`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): wire intent synthesis into submit_input pipeline"
```

---

### Task 9: Update resume_session to return turn history

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`
- Modify: `crates/storyteller-workshop/src/lib/types.ts`
- Modify: `crates/storyteller-workshop/src/lib/api.ts`

- [ ] **Step 1: Create ResumeResult type in Rust**

Add to `commands.rs` (or a shared types module):

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResumeResult {
    pub scene_info: SceneInfo,
    pub turns: Vec<TurnSummary>,
}

/// Minimal turn data for frontend chat hydration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TurnSummary {
    pub turn: u32,
    pub player_input: Option<String>,
    pub narrator_output: String,
}
```

- [ ] **Step 2: Update resume_session command**

Change `resume_session` to load turns from `turns.jsonl` and return `ResumeResult` instead of `SceneInfo`:

```rust
#[tauri::command]
pub async fn resume_session(
    session_id: String,
    state: tauri::State<'_, std::sync::Mutex<Option<EngineState>>>,
    session_store: tauri::State<'_, SessionStore>,
    app_handle: tauri::AppHandle,
) -> Result<ResumeResult, String> {
    let (selections, scene, characters) = session_store.load_session(&session_id)?;

    // Load turn history from turns.jsonl
    let turn_records = session_store.load_turns(&session_id)?;

    let turns: Vec<TurnSummary> = turn_records
        .iter()
        .map(|r| TurnSummary {
            turn: r.turn,
            player_input: r.player_input.clone(),
            narrator_output: r.narrator_output.clone(),
        })
        .collect();

    if turns.is_empty() {
        // No turns.jsonl — fresh start (legacy session gets fresh render)
        let scene_info = setup_and_render_opening(
            &scene,
            &characters,
            &selections,
            Some(session_id),
            state,
            session_store,
            app_handle,
        )
        .await?;

        return Ok(ResumeResult {
            scene_info,
            turns: vec![],
        });
    }

    // Reconstruct journal from narrator outputs.
    // NOTE: SceneJournal's add method is `add_turn()` from
    // `storyteller_engine::context::journal`, which takes:
    //   (journal, turn_number, content, referenced_entities, emotional_markers, observer)
    // Read the actual function signature before implementing.
    let mut journal = storyteller_core::types::narrator_context::SceneJournal::new(
        scene.scene_id,
        1200, // token budget
    );
    let observer = storyteller_core::traits::NoopObserver;
    for record in &turn_records {
        storyteller_engine::context::journal::add_turn(
            &mut journal,
            record.turn,
            &record.narrator_output,
            vec![],  // referenced_entities
            vec![],  // emotional_markers
            &observer,
        );
    }

    // Initialize engine state WITHOUT re-rendering opening.
    // Extract an `initialize_engine_state()` helper from
    // `setup_and_render_opening()` that does:
    //   1. Create ExternalServerProvider (narrator LLM, 14b model)
    //   2. Create ExternalServerProvider (intent LLM, 3b model)
    //   3. Load CharacterPredictor (ONNX)
    //   4. Load EventClassifier (optional)
    //   5. Create OllamaStructuredProvider (optional)
    //   6. Create EmotionalGrammar
    //   7. Create SessionLog
    //   8. Build EngineState with journal, scene, characters, session_id
    //
    // The refactored setup_and_render_opening() calls
    // initialize_engine_state() then renders the opening.
    // resume_session() calls initialize_engine_state() with the
    // pre-built journal (from turn history) and skips rendering.
    //
    // Set turn_count to last turn + 1.

    let scene_info = SceneInfo {
        title: scene.title.clone(),
        setting_description: scene.setting.description.clone(),
        cast: scene.cast.iter().map(|c| c.name.clone()).collect(),
        opening_prose: turn_records
            .first()
            .map(|t| t.narrator_output.clone())
            .unwrap_or_default(),
    };

    Ok(ResumeResult {
        scene_info,
        turns,
    })
}
```

**Key refactoring note:** Extract `initialize_engine_state()` from `setup_and_render_opening()`. This is the main structural change — the helper encapsulates all the LLM/ML/grammar/journal setup, and both `setup_and_render_opening()` and `resume_session()` call it. The helper accepts an optional pre-built journal (for resume) and a starting turn count.

- [ ] **Step 3: Update TypeScript types**

In `crates/storyteller-workshop/src/lib/types.ts`, add:

```typescript
export interface TurnSummary {
  turn: number;
  player_input: string | null;
  narrator_output: string;
}

export interface ResumeResult {
  scene_info: SceneInfo;
  turns: TurnSummary[];
}

// Add to the DebugEvent discriminated union:
export interface IntentSynthesizedEvent {
  type: "IntentSynthesized";
  turn: number;
  intent_statements: string;
  timing_ms: number;
}
```

Add `IntentSynthesizedEvent` to the `DebugEvent` union type.

- [ ] **Step 4: Update API wrapper**

In `crates/storyteller-workshop/src/lib/api.ts`, change:

```typescript
export async function resumeSession(sessionId: string): Promise<ResumeResult> {
  return invoke<ResumeResult>("resume_session", { sessionId });
}
```

- [ ] **Step 5: Verify Tauri compilation**

Run: `cd crates/storyteller-workshop && cargo tauri build --debug` (or `cargo check -p storyteller-workshop --all-features`)
Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs crates/storyteller-workshop/src/lib/types.ts crates/storyteller-workshop/src/lib/api.ts
git commit -m "feat(workshop): resume_session returns full turn history for chat hydration"
```

---

### Task 10: Update frontend to hydrate from turn history

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/PlayScene.svelte` (or whichever component handles scene play and session resume)

- [ ] **Step 1: Read the current resume flow in the frontend**

Read `crates/storyteller-workshop/src/lib/PlayScene.svelte` (or `+page.svelte`) to understand how `resumeSession()` is called and how the chat state is managed.

- [ ] **Step 2: Update the resume handler**

When `resumeSession()` returns a `ResumeResult`, populate the chat history from `turns`.

**Important:** The actual frontend uses:
- `blocks` (type `StoryBlock[]`), not `messages` — with `{ kind: "narrator" | "player" | "opening", ... }` discriminated union
- `sceneInfo` — matches
- `turnCount`, not `currentTurn`
- The view state machine uses `view` with values like `'setup'` and `'playing'`

Read `+page.svelte` and `PlayScene.svelte` (whichever owns the resume flow) to confirm the exact variable names and `StoryBlock` constructor pattern. The pseudocode below shows the logic:

```typescript
async function handleResume(sessionId: string) {
  const result = await resumeSession(sessionId);

  sceneInfo = result.scene_info;

  // Hydrate chat from turn history using StoryBlock constructors
  blocks = [];
  for (const turn of result.turns) {
    if (turn.turn === 0) {
      blocks.push({ kind: 'opening', text: turn.narrator_output });
    } else {
      if (turn.player_input) {
        blocks.push({ kind: 'player', text: turn.player_input });
      }
      blocks.push({ kind: 'narrator', text: turn.narrator_output });
    }
  }

  // If no turns, fresh start
  if (result.turns.length === 0) {
    blocks.push({ kind: 'opening', text: result.scene_info.opening_prose });
  }

  turnCount = result.turns.length > 0
    ? result.turns[result.turns.length - 1].turn + 1
    : 1;

  view = 'playing';
}
```

Adapt field names and `StoryBlock` shape to match the actual types in the codebase.

- [ ] **Step 3: Test manually**

Run: `cd crates/storyteller-workshop && cargo tauri dev`
1. Compose a scene, play 2-3 turns
2. Quit and relaunch
3. Select the session from the session list
4. Verify: chat history shows all prior turns, can continue playing

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src/lib/
git commit -m "feat(workshop): hydrate chat history from turns.jsonl on session resume"
```

---

### Task 11: Final verification and cleanup

**Files:**
- Various

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace --all-features`
Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues.

- [ ] **Step 4: Manual end-to-end test**

Run: `cd crates/storyteller-workshop && cargo tauri dev`

Test sequence:
1. Compose a new scene (any genre/profile)
2. Play 3 turns — observe narrator output for NPC agency (do characters speak? act proactively?)
3. Check debug inspector — verify Intent tab shows synthesized intents
4. Quit the workshop
5. Relaunch, select the session from the list
6. Verify chat history is restored
7. Play 1 more turn — verify continuity

- [ ] **Step 5: Verify turns.jsonl content**

Read a `turns.jsonl` file from a test session and verify:
- Turn 0 has `player_input: null` and opening narration
- Turn 1+ has predictions, intent_statements, decomposition, etc.
- All fields serialize/deserialize correctly

- [ ] **Step 6: Clean up any dead code**

Remove or deprecate `events_path()` from SessionStore if no longer used. Remove old events.jsonl write code if still present.

- [ ] **Step 7: Commit cleanup**

```bash
git add -A && git commit -m "chore: cleanup dead events.jsonl code and final verification"
```
