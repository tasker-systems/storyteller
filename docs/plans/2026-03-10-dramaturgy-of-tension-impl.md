# Dramaturgy of Tension — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the intent synthesis pipeline to include the player character with tension/alignment notes, so the narrator renders player actions through the lens of character identity.

**Architecture:** The existing intent synthesizer filters out the player character. We stop filtering, add readable trait/prediction formatting helpers, produce a `[PLAYER CHARACTER]` summary for the 3b-instruct model, update the narrator's preamble to mark the player character, and add a tension-rendering instruction to the narrator system prompt. The workshop wires `player_entity_id` through from `EngineState`.

**Tech Stack:** Rust (storyteller-core, storyteller-engine), Tauri (storyteller-workshop), Ollama 3b-instruct

**Spec:** `docs/plans/2026-03-10-dramaturgy-of-tension-design.md`

---

## Chunk 0: Gate the hardcoded scene

The `the_flute_kept` hardcoded scene was a prototype fixture. It has no `player_entity_id` concept and adds a backward-compat edge case we don't want to maintain. Gate it behind `#[cfg(test)]` so tests still work, remove the UI entry point, and `#[cfg(test)]` the `start_scene` command.

### Task 0: Gate `the_flute_kept` and hide classic scene UI

**Files:**
- Modify: `crates/storyteller-engine/src/workshop/mod.rs`
- Modify: `crates/storyteller-engine/src/lib.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`
- Modify: `crates/storyteller-workshop/src/routes/+page.svelte`

- [ ] **Step 1: Gate `the_flute_kept` module behind `#[cfg(test)]`**

In `crates/storyteller-engine/src/workshop/mod.rs`, change:

```rust
// Before:
pub mod the_flute_kept;

// After:
#[cfg(test)]
pub mod the_flute_kept;
```

- [ ] **Step 2: Gate the `start_scene` command**

In `crates/storyteller-workshop/src-tauri/src/commands.rs`:

1. Gate the import:

```rust
// Before:
use storyteller_engine::workshop::the_flute_kept;

// After:
#[cfg(test)]
use storyteller_engine::workshop::the_flute_kept;
```

2. Gate the `start_scene` function with `#[cfg(not(test))]` and provide a stub that returns an error:

```rust
/// Load the workshop scene — DEPRECATED, use compose_scene instead.
#[tauri::command]
pub async fn start_scene(
    _app: tauri::AppHandle,
    _state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<SceneInfo, String> {
    Err("Classic scene mode has been removed. Use the scene wizard to compose a scene.".to_string())
}
```

Note: If `start_scene` is registered as a Tauri command handler in `main.rs` or `lib.rs`, it must remain compilable — hence the stub rather than `#[cfg]` removal.

- [ ] **Step 3: Hide the classic scene button in the UI**

In `crates/storyteller-workshop/src/routes/+page.svelte`, remove or comment out the classic fallback section (lines ~183-192):

```svelte
<!-- Remove this block: -->
<!--
<div class="classic-fallback">
  <span class="fallback-divider">or</span>
  <button class="classic-btn" onclick={handleClassicStart} disabled={loading}>
    {loading ? "Starting..." : "Classic: The Flute Kept"}
  </button>
</div>
-->
```

Also remove the `handleClassicStart` function if it exists and is now unreferenced.

- [ ] **Step 4: Run compilation check**

Run: `cargo check --workspace --all-features`
Expected: PASS (tests can still use `the_flute_kept` via `#[cfg(test)]`)

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace --all-features 2>&1 | tail -5`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/workshop/mod.rs crates/storyteller-engine/src/lib.rs crates/storyteller-workshop/src-tauri/src/commands.rs crates/storyteller-workshop/src/routes/+page.svelte
git commit -m "refactor: gate the_flute_kept behind #[cfg(test)], hide classic scene UI"
```

---

## Chunk 1: Readable Formatting Helpers

These are pure functions with no dependencies on other changes. They format tensor axes and ML predictions into natural language that the 3b-instruct model can understand.

### Task 1: `format_dominant_axes()` helper

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs`

This helper extracts the top N tensor axes by central_tendency magnitude and formats them as readable English trait names with values.

- [ ] **Step 1: Write the failing test for `format_dominant_axes`**

Add to the `#[cfg(test)] mod tests` block in `intent_synthesis.rs`:

```rust
#[test]
fn format_dominant_axes_sorts_by_magnitude_and_formats_readable() {
    let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
    let result = format_dominant_axes(&bramblehoof, 3);

    // Should contain readable trait names with values
    // Bramblehoof's tensor has axes like "empathy", "protective_impulse", etc.
    // Format: "trait_name 0.XX, trait_name 0.XX, trait_name 0.XX"
    assert!(!result.is_empty());

    // Should have exactly 3 entries (comma-separated)
    let parts: Vec<&str> = result.split(", ").collect();
    assert_eq!(parts.len(), 3, "Expected 3 axes, got: {result}");

    // Each entry should have a name and a value
    for part in &parts {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        assert_eq!(tokens.len(), 2, "Expected 'name value', got: {part}");
        // Second token should parse as f32
        tokens[1].parse::<f32>().expect("Value should be a float");
    }
}

#[test]
fn format_dominant_axes_uses_readable_names() {
    let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
    let result = format_dominant_axes(&bramblehoof, 5);

    // Should NOT contain underscores — axis IDs like "protective_impulse"
    // should become "protective-impulse" or similar readable form
    // The mapping converts underscore-separated IDs to hyphenated readable names
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-engine format_dominant_axes --all-features`
Expected: FAIL — `format_dominant_axes` not found

- [ ] **Step 3: Implement `format_dominant_axes`**

Add above `summarize_character()` in `intent_synthesis.rs`:

```rust
/// Formats the top `count` tensor axes by central_tendency magnitude as
/// readable English trait names with values.
///
/// Output: `"empathetic 0.78, protective 0.72, grief-stricken 0.65"`
///
/// Axis IDs are converted to readable form: underscores become hyphens,
/// common suffixes like `_impulse` or `_aversion` are preserved as
/// hyphenated compounds (e.g., `protective_impulse` → `protective-impulse`).
pub fn format_dominant_axes(sheet: &CharacterSheet, count: usize) -> String {
    let mut axes: Vec<(&String, f32)> = sheet
        .tensor
        .axes
        .iter()
        .map(|(name, entry)| (name, entry.value.central_tendency))
        .collect();

    // Sort by absolute magnitude descending
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

/// Converts an axis ID (e.g., `protective_impulse`) to a readable English
/// name (e.g., `protective-impulse`). Simple transformation: replace
/// underscores with hyphens.
fn axis_id_to_readable(id: &str) -> String {
    id.replace('_', "-")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine format_dominant_axes --all-features`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "feat(engine): add format_dominant_axes helper for readable trait formatting"
```

### Task 2: `format_prediction_readable()` helper

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs`

This helper converts a `CharacterPrediction` into natural language the 3b model can reason about.

- [ ] **Step 1: Write the failing tests**

Add to the test module in `intent_synthesis.rs`:

```rust
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
    // Should contain natural language like "would most likely examine (85% confidence)"
    assert!(result.contains("examine"), "Should mention action type: {result}");
    assert!(result.contains("85%"), "Should include confidence: {result}");
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
    assert!(result.contains("conversational"), "Should mention register: {result}");
    assert!(result.contains("65%"), "Should include speech confidence: {result}");
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
    // Should have "would most likely examine" and "unlikely to move"
    assert!(result.contains("most likely"), "Should identify primary: {result}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-engine format_prediction_readable --all-features`
Expected: FAIL — `format_prediction_readable` not found

- [ ] **Step 3: Implement `format_prediction_readable`**

Add after `format_dominant_axes()` in `intent_synthesis.rs`:

```rust
/// Converts a `CharacterPrediction` into natural language for the 3b-instruct model.
///
/// Output examples:
/// - `"would most likely examine (85% confidence), unlikely to move (15%)"`
/// - `"would most likely move (70% confidence); likely to speak (conversational, 65% confidence)"`
pub fn format_prediction_readable(pred: &CharacterPrediction) -> String {
    let mut parts = Vec::new();

    // Sort actions by confidence descending
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
            parts.push(format!("would most likely {action_name} ({pct}% confidence)"));
        } else if action.confidence < 0.3 {
            parts.push(format!("unlikely to {action_name} ({pct}%)"));
        } else {
            parts.push(format!("might {action_name} ({pct}%)"));
        }
    }

    // Speech prediction
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

/// Converts an `ActionType` to a readable verb.
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

/// Converts a `SpeechRegister` to a readable adjective.
fn speech_register_to_readable(register: SpeechRegister) -> &'static str {
    match register {
        SpeechRegister::Whisper => "whispered",
        SpeechRegister::Conversational => "conversational",
        SpeechRegister::Declamatory => "raised voice",
        SpeechRegister::Internal => "internal",
    }
}
```

Note: Add these imports at the top of the file if not already present:

```rust
use storyteller_core::types::prediction::{ActionType, SpeechRegister};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine format_prediction_readable --all-features`
Expected: All 3 tests PASS

- [ ] **Step 5: Run all intent_synthesis tests to check for regressions**

Run: `cargo test -p storyteller-engine intent_synthesis --all-features`
Expected: All existing + new tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "feat(engine): add format_prediction_readable helper for natural language predictions"
```

---

## Chunk 2: Player Character in Intent Synthesis

Modifies `build_summaries()` to include the player character with a different format, updates the system prompt, and adds `player_entity_id` tracking to `EngineState`.

### Task 3: Update `build_summaries()` to include player character

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs`

The function currently filters out the player character. Change it to include the player with a `[PLAYER CHARACTER]` marker, readable traits, and ML prediction summary.

- [ ] **Step 1: Write the failing test**

Add to test module in `intent_synthesis.rs`:

```rust
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

    // Player character should now be INCLUDED with a marker
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
    // NPC should still be present
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
    // When player_entity_id is None, all characters are NPCs (original behavior)
    let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
    let pyotir = crate::workshop::the_flute_kept::pyotir();

    let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
    let predictions = vec![];

    let (char_summary, _) = build_summaries(&characters, &predictions, None, None);

    // No player marker
    assert!(
        !char_summary.contains("[PLAYER CHARACTER"),
        "Should not have player marker when no player_entity_id"
    );
    // Both characters present as NPCs
    assert!(char_summary.contains("Bramblehoof"));
    assert!(char_summary.contains("Pyotir"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-engine build_summaries --all-features`
Expected: FAIL — compilation errors due to new `player_input` parameter

- [ ] **Step 3: Update `build_summaries()` signature and implementation**

Replace the existing `build_summaries()` function:

```rust
/// Builds character and prediction summaries for the intent synthesizer.
///
/// Returns `(character_summary, predictions_summary)` as formatted strings.
///
/// When `player_entity_id` is `Some`, the player character gets a different
/// format with a `[PLAYER CHARACTER]` marker, readable dominant traits, and
/// the directed action. NPCs get the existing compact summary format.
///
/// When `player_entity_id` is `None`, all characters are treated as NPCs
/// (backward-compatible behavior).
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
            // Player character: rich readable format
            let action_text = player_input.unwrap_or("(no action specified)");
            let traits = format_dominant_axes(character, 5);
            let mut player_block = format!(
                "[PLAYER CHARACTER — directed action: \"{action_text}\"]\n\
                 {} | {} | Dominant traits (0-1 scale): {}",
                character.name,
                character.performance_notes,
                traits,
            );

            // Add ML prediction if available
            if let Some(pred) = predictions.iter().find(|p| p.character_id == character.entity_id) {
                let readable_pred = format_prediction_readable(pred);
                player_block.push_str(&format!("\nML prediction: {readable_pred}"));
            }

            char_lines.push(player_block);
        } else {
            // NPC: existing compact format
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
```

- [ ] **Step 4: Fix all call sites of `build_summaries`**

Update `synthesize_intents()` in the same file — pass `player_input` through:

```rust
let (char_summary, pred_summary) = build_summaries(
    characters,
    predictions,
    player_entity_id,
    Some(player_input),
);
```

Update the existing test `build_summaries_skips_player_character` — this test's assertion needs to change since the player is now included (not skipped). Rename and update:

```rust
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

    // Player should be included with marker
    assert!(
        char_summary.contains("[PLAYER CHARACTER"),
        "Player should have marker"
    );
    assert!(
        char_summary.contains("Bramblehoof"),
        "Player name should appear"
    );
    // NPC should be included without marker
    assert!(
        char_summary.lines().any(|line| line.starts_with("Pyotir")),
        "NPC should be included in summaries"
    );
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine intent_synthesis --all-features`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "feat(engine): include player character in build_summaries with tension-ready format"
```

### Task 4: Update intent synthesis system prompt

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs`

Add the player character rules to the system prompt.

- [ ] **Step 1: Write the failing test**

Add to test module:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine system_prompt_includes_player --all-features`
Expected: FAIL — assertions fail

- [ ] **Step 3: Update `intent_synthesis_system_prompt()`**

Update the job description line and append player character rules. Replace the function body:

```rust
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

Format each character section as:
**CharacterName** directive paragraph here."
        .to_string()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine intent_synthesis --all-features`
Expected: All tests PASS (including the existing `system_prompt_has_key_sections` test — verify it still passes since we kept "Intent Synthesizer", "directive", and "Do NOT write dialogue")

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "feat(engine): update intent synthesis system prompt with player character rules"
```

### Task 5: Add `player_entity_id` to `EngineState`

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/engine_state.rs`

- [ ] **Step 1: Add the field**

Add to `EngineState` struct:

```rust
/// Entity ID of the player-controlled character (None for hardcoded scenes).
pub player_entity_id: Option<storyteller_core::types::entity::EntityId>,
```

- [ ] **Step 2: Run compilation check**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: FAIL — all `EngineState` construction sites need the new field

- [ ] **Step 3: Fix all `EngineState` construction sites**

After Task 0 gated the hardcoded scene path, the only production `EngineState` construction is in `setup_and_render_opening()`. The workshop wizard selects a cast with roles — the character with role containing "protagonist" is the player character.

In `setup_and_render_opening()`, before constructing `EngineState`, detect the player character:

```rust
// Find player character: first cast member whose role contains "protagonist"
let player_entity_id = scene.cast.iter()
    .find(|c| c.role.to_lowercase().contains("protagonist"))
    .map(|c| c.entity_id);
```

Add `player_entity_id` to the `EngineState { .. }` construction in `setup_and_render_opening()`.

The gated `start_scene()` stub (from Task 0) doesn't construct `EngineState`, so no change needed there.

- [ ] **Step 4: Run compilation check**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/engine_state.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): add player_entity_id to EngineState"
```

### Task 6: Wire `player_entity_id` through to `synthesize_intents()`

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

Currently the call passes `None` for `player_entity_id`. Wire the real value.

- [ ] **Step 1: Update the `synthesize_intents()` call**

In `commands.rs` around line 695-702, change:

```rust
// Before:
storyteller_engine::inference::intent_synthesis::synthesize_intents(
    intent_llm.as_ref(),
    &characters_refs,
    &resolver_output.original_predictions,
    &journal_tail,
    &input,
    &engine.scene,
    None, // player_entity_id — not tracked yet, None means include all
)

// After:
storyteller_engine::inference::intent_synthesis::synthesize_intents(
    intent_llm.as_ref(),
    &characters_refs,
    &resolver_output.original_predictions,
    &journal_tail,
    &input,
    &engine.scene,
    engine.player_entity_id,
)
```

- [ ] **Step 2: Run compilation check**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): wire player_entity_id through to intent synthesis"
```

---

## Chunk 3: Narrator Prompt Changes

Updates the preamble to identify the player character and adds tension-rendering instruction to the narrator system prompt.

### Task 7: Add `is_player` to `CastDescription`

**Files:**
- Modify: `crates/storyteller-core/src/types/narrator_context.rs`

- [ ] **Step 1: Write the failing test**

Add to test module in `narrator_context.rs`:

```rust
#[test]
fn cast_description_has_is_player_field() {
    let cast = CastDescription {
        entity_id: EntityId::new(),
        name: "Arthur".to_string(),
        role: "protagonist".to_string(),
        voice_note: "Measured, clipped".to_string(),
        is_player: true,
    };
    assert!(cast.is_player);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-core cast_description_has_is_player --all-features`
Expected: FAIL — `is_player` field doesn't exist

- [ ] **Step 3: Add the field**

In `narrator_context.rs`, add to `CastDescription`:

```rust
/// Whether this cast member is controlled by the player.
#[serde(default)]
pub is_player: bool,
```

The `#[serde(default)]` ensures backward compatibility with existing serialized data.

- [ ] **Step 4: Fix all `CastDescription` construction sites**

Search for all places that construct `CastDescription` and add `is_player: false` (default). Key locations:

1. `crates/storyteller-engine/src/context/preamble.rs` — `build_preamble()` (line ~70)
2. `crates/storyteller-engine/src/agents/narrator.rs` — `mock_context()` test helper (line ~319)
3. `crates/storyteller-core/src/types/narrator_context.rs` — `preamble_is_constructible` test

For `build_preamble()`, set `is_player` based on whether the entity_id matches a player entity. This requires a new parameter. Update `build_preamble()` signature:

```rust
pub fn build_preamble(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    observer: &dyn PhaseObserver,
    player_entity_id: Option<EntityId>,
) -> PersistentPreamble {
```

In the cast construction loop, add:

```rust
is_player: player_entity_id == Some(cast_entry.entity_id),
```

- [ ] **Step 5: Fix all `build_preamble()` call sites**

`build_preamble()` is called directly and indirectly through `assemble_narrator_context()`. The full cascade:

**Direct `build_preamble()` calls:**
- `crates/storyteller-engine/src/context/mod.rs` line 54 — inside `assemble_narrator_context()`
- `crates/storyteller-engine/src/context/preamble.rs` tests — pass `None`

**`assemble_narrator_context()` signature must also gain `player_entity_id`:**

In `crates/storyteller-engine/src/context/mod.rs`, add the parameter:

```rust
pub fn assemble_narrator_context(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    journal: &SceneJournal,
    resolver_output: &ResolverOutput,
    player_input: &str,
    referenced_entities: &[EntityId],
    total_budget: u32,
    observer: &dyn PhaseObserver,
    player_entity_id: Option<EntityId>,  // NEW
) -> NarratorContextInput {
```

And forward it to `build_preamble`:

```rust
let preamble = build_preamble(scene, characters, observer, player_entity_id);
```

**`assemble_narrator_context()` call sites (all need the new arg):**
- `crates/storyteller-engine/src/context/mod.rs` tests (2 calls) — pass `None`
- `crates/storyteller-workshop/src-tauri/src/commands.rs` lines 734, 1068 — pass `engine.player_entity_id`
- `crates/storyteller-cli/src/bin/play_scene_context.rs` lines 176, 345 — pass `None`
- `crates/storyteller-engine/src/systems/turn_cycle.rs` line 276 — pass `None` (Bevy system doesn't track player entity yet)

Note: The `StorykeeperQuery` trait in `storyteller-core` has its own `assemble_narrator_context` method with a different signature — it does NOT need to change here (it takes `SessionContext` which already has `player_entity_id`).

- [ ] **Step 6: Run tests**

Run: `cargo test --workspace --all-features 2>&1 | tail -20`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-core/src/types/narrator_context.rs crates/storyteller-engine/src/context/preamble.rs crates/storyteller-engine/src/context/mod.rs crates/storyteller-engine/src/agents/narrator.rs crates/storyteller-engine/src/systems/turn_cycle.rs crates/storyteller-cli/src/bin/play_scene_context.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(core): add is_player field to CastDescription with full cascade"
```

### Task 8: Update `render_preamble()` with player marker

**Files:**
- Modify: `crates/storyteller-engine/src/context/preamble.rs`

- [ ] **Step 1: Write the failing test**

Add to test module in `preamble.rs`:

```rust
#[test]
fn rendered_preamble_marks_player_character() {
    let scene = crate::workshop::the_flute_kept::scene();
    let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
    let pyotir = crate::workshop::the_flute_kept::pyotir();
    let player_id = bramblehoof.entity_id;
    let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

    let observer = storyteller_core::traits::NoopObserver;
    let preamble = build_preamble(&scene, &characters, &observer, Some(player_id));
    let rendered = render_preamble(&preamble);

    // Player character should have "(player)" marker
    assert!(
        rendered.contains("(player)"),
        "Should mark player character: {rendered}"
    );
    // Specifically on Bramblehoof's line
    let bramblehoof_line = rendered
        .lines()
        .find(|l| l.contains("Bramblehoof"))
        .expect("Bramblehoof should be in rendered preamble");
    assert!(
        bramblehoof_line.contains("(player)"),
        "Bramblehoof's line should have (player) marker: {bramblehoof_line}"
    );
    // Pyotir should NOT have the marker
    let pyotir_line = rendered
        .lines()
        .find(|l| l.contains("Pyotir"))
        .expect("Pyotir should be in rendered preamble");
    assert!(
        !pyotir_line.contains("(player)"),
        "NPC should not have (player) marker: {pyotir_line}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine rendered_preamble_marks_player --all-features`
Expected: FAIL — no `(player)` in output

- [ ] **Step 3: Update `render_preamble()`**

In the `## Cast` section of `render_preamble()`, change the header format:

```rust
output.push_str("## Cast\n");
for cast in &preamble.cast_descriptions {
    if cast.is_player {
        output.push_str(&format!("### {} — {} (player)\n", cast.name, cast.role));
    } else {
        output.push_str(&format!("### {} — {}\n", cast.name, cast.role));
    }
    if !cast.voice_note.is_empty() {
        output.push_str(&format!("Voice: {}\n", cast.voice_note));
    }
    output.push('\n');
}
```

- [ ] **Step 4: Add backward-compat test (no player marker when player_entity_id is None)**

Add to test module:

```rust
#[test]
fn rendered_preamble_no_player_marker_when_no_player_id() {
    let scene = crate::workshop::the_flute_kept::scene();
    let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
    let pyotir = crate::workshop::the_flute_kept::pyotir();
    let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

    let observer = storyteller_core::traits::NoopObserver;
    let preamble = build_preamble(&scene, &characters, &observer, None);
    let rendered = render_preamble(&preamble);

    assert!(
        !rendered.contains("(player)"),
        "Should not have player marker when player_entity_id is None: {rendered}"
    );
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine preamble --all-features`
Expected: All preamble tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/context/preamble.rs
git commit -m "feat(engine): render (player) marker in preamble cast section"
```

### Task 9: Add tension-rendering instruction to narrator system prompt

**Files:**
- Modify: `crates/storyteller-engine/src/agents/narrator.rs`

- [ ] **Step 1: Write the failing test**

Add to test module in `narrator.rs`:

```rust
#[test]
fn system_prompt_includes_tension_rendering_instruction() {
    let context = mock_context();
    let prompt = build_system_prompt(&context);
    assert!(
        prompt.contains("player character's paragraph"),
        "Should mention player character intents: {prompt}"
    );
    assert!(
        prompt.contains("Do not block or"),
        "Should instruct not to block player intent"
    );
    assert!(
        prompt.contains("Show it physically"),
        "Should instruct physical rendering"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine system_prompt_includes_tension --all-features`
Expected: FAIL — assertions fail

- [ ] **Step 3: Add the paragraph to `build_system_prompt()`**

In `narrator.rs`, `build_system_prompt()`, add after the existing "Your Task" section (after the "Weave the facts into a single narrative passage" paragraph):

```rust
format!(
    r#"You are the Narrator.

{preamble}

## Your Task
You receive intent statements describing what each character wants to do
this turn. Honor these intents — render them with each character's full
agency. Characters act, speak, and drive the scene. They are not scenery.

When the player character's paragraph appears in the intent statements,
it describes how the character's nature relates to the player's directed
action. The player's action is what happens — but render it through who
the character is. If friction is noted, let the character's body,
hesitation, or instinct show through the action. Do not block or
subvert the player's intent. Do not explain the tension to the reader.
Show it physically.

Render only what is observable — physical actions, speech, gestures.
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
```

- [ ] **Step 4: Run all narrator tests**

Run: `cargo test -p storyteller-engine narrator --all-features`
Expected: All tests PASS. Verify the existing `system_prompt_has_preamble` test still passes (it checks for "Your Voice", "Never Do", "The Scene", character names, "present tense", "intent statements" — all still present).

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/agents/narrator.rs
git commit -m "feat(engine): add tension-rendering instruction to narrator system prompt"
```

### Task 10: Final integration check

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test --workspace --all-features`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues

- [ ] **Step 4: Commit any fixes if needed**

If clippy or fmt found issues, fix and commit:

```bash
git add -A
git commit -m "fix: clippy and fmt cleanup for dramaturgy-of-tension"
```
