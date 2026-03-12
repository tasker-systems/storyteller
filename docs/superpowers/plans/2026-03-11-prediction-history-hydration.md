# Prediction History Hydration (Region 7) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hydrate the ML model's Region 7 (recent history, 48 features) so character predictions respond to accumulated turn-by-turn context instead of producing static outputs.

**Architecture:** Create a `PredictionHistory` ring buffer type in `storyteller-ml` that accumulates `HistoryEntry` values extracted from enriched `CharacterPrediction` outputs. The workshop wires this into `EngineState` and passes it through `predict_character_behaviors()` on each turn. The Bevy `turn_cycle.rs` passes empty history to preserve current behavior. The encoding layer (`feature_schema.rs`) already handles `HistoryEntry` → feature vector encoding — this plan fills the upstream gap.

**Tech Stack:** Rust (storyteller-ml, storyteller-engine, storyteller-workshop/src-tauri)

**Spec:** `docs/plans/2026-03-11-prediction-history-hydration-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/storyteller-ml/src/prediction_history.rs` | Create | `PredictionHistory` type, `HistoryEntry::from_prediction()`, emotional valence computation, ring buffer push/lookup |
| `crates/storyteller-ml/src/lib.rs` | Modify | Add `pub mod prediction_history` |
| `crates/storyteller-engine/src/context/prediction.rs` | Modify | Add `history` parameter to `predict_character_behaviors()`, look up per-character history |
| `crates/storyteller-workshop/src-tauri/src/engine_state.rs` | Modify | Add `prediction_history` field to `EngineState` |
| `crates/storyteller-workshop/src-tauri/src/commands.rs` | Modify | Extract history entries after predictions, push to ring buffer, pass history to prediction call |
| `crates/storyteller-engine/src/systems/turn_cycle.rs` | Modify | Pass empty history to `predict_character_behaviors()` |

---

## Chunk 1: PredictionHistory Type and HistoryEntry Construction

### Task 1: Create `PredictionHistory` with ring buffer and `HistoryEntry` construction

**Files:**
- Create: `crates/storyteller-ml/src/prediction_history.rs`
- Modify: `crates/storyteller-ml/src/lib.rs`

The `HistoryEntry` struct already exists in `feature_schema.rs` (line 144). This task creates the `PredictionHistory` wrapper and the logic to construct `HistoryEntry` from a `CharacterPrediction`.

- [ ] **Step 1: Write failing tests**

Create `crates/storyteller-ml/src/prediction_history.rs` with the test module first. The tests exercise: construction from `CharacterPrediction`, emotional valence computation (positive, negative, mixed, empty), ring buffer depth limit, and lookup for unknown character.

```rust
//! Prediction history — ring buffer of recent turn outcomes per character.
//!
//! Captures what the ML model predicted each turn so subsequent predictions
//! can reference recent behavior. Feeds Region 7 of the feature vector.
//!
//! See: `docs/plans/2026-03-11-prediction-history-hydration-design.md`

use std::collections::HashMap;

use storyteller_core::types::entity::EntityId;
use storyteller_core::types::prediction::{
    ActionType, CharacterPrediction, SpeechRegister,
};
use storyteller_core::types::tensor::AwarenessLevel;

use crate::feature_schema::{HistoryEntry, HISTORY_DEPTH};

/// Per-character ring buffer of recent prediction outcomes.
///
/// Keyed by `EntityId`. Each character's buffer holds at most [`HISTORY_DEPTH`]
/// entries in most-recent-first order. Uses `Vec` with `insert(0, ..)` instead
/// of `VecDeque` — with max 3 entries the shift cost is negligible and we get
/// a contiguous `&[HistoryEntry]` slice from `.as_slice()`.
#[derive(Debug, Clone, Default)]
pub struct PredictionHistory {
    entries: HashMap<EntityId, Vec<HistoryEntry>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::prediction::{
        ActionPrediction, ActivatedTensorFrame, EmotionalDelta,
        SpeechPrediction, ThoughtPrediction,
    };

    fn mock_prediction(entity_id: EntityId) -> CharacterPrediction {
        CharacterPrediction {
            character_id: entity_id,
            character_name: "TestChar".to_string(),
            frame: ActivatedTensorFrame {
                activated_axes: vec!["trust".to_string()],
                activation_reason: "test".to_string(),
                confidence: 0.8,
            },
            actions: vec![ActionPrediction {
                action_type: ActionType::Speak,
                description: "speaks softly".to_string(),
                confidence: 0.9,
                target: None,
            }],
            speech: Some(SpeechPrediction {
                content_direction: "gentle comfort".to_string(),
                register: SpeechRegister::Whisper,
                confidence: 0.8,
            }),
            thought: ThoughtPrediction {
                emotional_subtext: "warmth beneath guardedness".to_string(),
                awareness_level: AwarenessLevel::Recognizable,
                internal_conflict: None,
            },
            emotional_deltas: vec![
                EmotionalDelta {
                    primary_id: "joy".to_string(),
                    intensity_change: 0.3,
                    awareness_change: None,
                },
                EmotionalDelta {
                    primary_id: "sadness".to_string(),
                    intensity_change: -0.1,
                    awareness_change: None,
                },
            ],
        }
    }

    #[test]
    fn history_entry_from_prediction_extracts_fields() {
        let id = EntityId::new();
        let pred = mock_prediction(id);
        let entry = HistoryEntry::from_prediction(&pred);

        assert_eq!(entry.action_type, ActionType::Speak);
        assert_eq!(entry.speech_register, SpeechRegister::Whisper);
        assert_eq!(entry.awareness_level, AwarenessLevel::Recognizable);
    }

    #[test]
    fn history_entry_defaults_speech_register_when_no_speech() {
        let id = EntityId::new();
        let mut pred = mock_prediction(id);
        pred.speech = None;
        let entry = HistoryEntry::from_prediction(&pred);

        assert_eq!(entry.speech_register, SpeechRegister::Conversational);
    }

    #[test]
    fn history_entry_defaults_action_type_when_no_actions() {
        let id = EntityId::new();
        let mut pred = mock_prediction(id);
        pred.actions.clear();
        let entry = HistoryEntry::from_prediction(&pred);

        assert_eq!(entry.action_type, ActionType::Examine);
    }

    #[test]
    fn emotional_valence_positive_deltas() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.5,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "trust".to_string(),
                intensity_change: 0.3,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn emotional_valence_negative_deltas() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "fear".to_string(),
                intensity_change: -0.4,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "anger".to_string(),
                intensity_change: -0.7,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - (-1.0)).abs() < f32::EPSILON, "should clamp to -1.0");
    }

    #[test]
    fn emotional_valence_mixed_deltas() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.3,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "sadness".to_string(),
                intensity_change: -0.1,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn emotional_valence_empty_deltas() {
        let valence = compute_emotional_valence(&[]);
        assert!(valence.abs() < f32::EPSILON, "empty deltas should be 0.0");
    }

    #[test]
    fn emotional_valence_clamps_to_range() {
        let deltas = vec![
            EmotionalDelta {
                primary_id: "joy".to_string(),
                intensity_change: 0.8,
                awareness_change: None,
            },
            EmotionalDelta {
                primary_id: "trust".to_string(),
                intensity_change: 0.9,
                awareness_change: None,
            },
        ];
        let valence = compute_emotional_valence(&deltas);
        assert!((valence - 1.0).abs() < f32::EPSILON, "should clamp to 1.0");
    }

    #[test]
    fn ring_buffer_push_and_lookup() {
        let id = EntityId::new();
        let pred = mock_prediction(id);
        let mut history = PredictionHistory::default();

        history.push_from_prediction(&pred);
        let entries = history.get(id);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action_type, ActionType::Speak);
    }

    #[test]
    fn ring_buffer_most_recent_first() {
        let id = EntityId::new();
        let mut history = PredictionHistory::default();

        let mut pred1 = mock_prediction(id);
        pred1.actions[0].action_type = ActionType::Perform;
        history.push_from_prediction(&pred1);

        let mut pred2 = mock_prediction(id);
        pred2.actions[0].action_type = ActionType::Speak;
        history.push_from_prediction(&pred2);

        let entries = history.get(id);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action_type, ActionType::Speak, "most recent first");
        assert_eq!(entries[1].action_type, ActionType::Perform);
    }

    #[test]
    fn ring_buffer_respects_depth_limit() {
        let id = EntityId::new();
        let mut history = PredictionHistory::default();

        for _ in 0..5 {
            history.push_from_prediction(&mock_prediction(id));
        }

        let entries = history.get(id);
        assert_eq!(entries.len(), HISTORY_DEPTH, "should cap at HISTORY_DEPTH");
    }

    #[test]
    fn unknown_character_returns_empty() {
        let history = PredictionHistory::default();
        let entries = history.get(EntityId::new());
        assert!(entries.is_empty());
    }

    #[test]
    fn multiple_characters_independent() {
        let id_a = EntityId::new();
        let id_b = EntityId::new();
        let mut history = PredictionHistory::default();

        let mut pred_a = mock_prediction(id_a);
        pred_a.actions[0].action_type = ActionType::Perform;
        history.push_from_prediction(&pred_a);

        let mut pred_b = mock_prediction(id_b);
        pred_b.actions[0].action_type = ActionType::Examine;
        history.push_from_prediction(&pred_b);

        let a_entries = history.get(id_a);
        let b_entries = history.get(id_b);
        assert_eq!(a_entries[0].action_type, ActionType::Perform);
        assert_eq!(b_entries[0].action_type, ActionType::Examine);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-ml prediction_history --all-features`
Expected: Compilation errors — `HistoryEntry::from_prediction`, `compute_emotional_valence`, `PredictionHistory::push_from_prediction`, and `PredictionHistory::get` don't exist yet.

- [ ] **Step 3: Implement `compute_emotional_valence`**

Add above the test module in `prediction_history.rs`:

```rust
/// Compute emotional valence from emotional deltas.
///
/// Sums all `intensity_change` values and clamps to \[-1.0, 1.0\].
/// Returns 0.0 for empty deltas.
pub fn compute_emotional_valence(deltas: &[storyteller_core::types::prediction::EmotionalDelta]) -> f32 {
    deltas
        .iter()
        .map(|d| d.intensity_change)
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}
```

- [ ] **Step 4: Implement `HistoryEntry::from_prediction`**

Add an `impl` block for `HistoryEntry` in `prediction_history.rs`. This extends the type defined in `feature_schema.rs` — Rust allows adding inherent methods in the same crate.

```rust
impl HistoryEntry {
    /// Extract a history entry from an enriched character prediction.
    ///
    /// - `action_type`: highest-confidence action, or `Examine` if no actions.
    /// - `speech_register`: from speech prediction, or `Conversational` if silent.
    /// - `awareness_level`: from thought prediction.
    /// - `emotional_valence`: sum of deltas, clamped to \[-1.0, 1.0\].
    pub fn from_prediction(pred: &CharacterPrediction) -> Self {
        let action_type = pred
            .actions
            .first()
            .map(|a| a.action_type)
            .unwrap_or(ActionType::Examine);

        let speech_register = pred
            .speech
            .as_ref()
            .map(|s| s.register)
            .unwrap_or(SpeechRegister::Conversational);

        Self {
            action_type,
            speech_register,
            awareness_level: pred.thought.awareness_level,
            emotional_valence: compute_emotional_valence(&pred.emotional_deltas),
        }
    }
}
```

- [ ] **Step 5: Implement `PredictionHistory` methods**

Add methods to the `PredictionHistory` struct:

```rust
impl PredictionHistory {
    /// Push a new history entry derived from a character's prediction.
    ///
    /// The entry is inserted at the front (most-recent-first). If the buffer
    /// exceeds [`HISTORY_DEPTH`], the oldest entry is dropped.
    pub fn push_from_prediction(&mut self, prediction: &CharacterPrediction) {
        let entry = HistoryEntry::from_prediction(prediction);
        let buf = self.entries.entry(prediction.character_id).or_default();
        buf.insert(0, entry);
        buf.truncate(HISTORY_DEPTH);
    }

    /// Get history entries for a character.
    ///
    /// Returns an empty slice if the character has no history.
    /// Entries are most-recent-first, at most [`HISTORY_DEPTH`] items.
    pub fn get(&self, character_id: EntityId) -> &[HistoryEntry] {
        match self.entries.get(&character_id) {
            Some(buf) => buf.as_slice(),
            None => &[],
        }
    }

    /// Borrow the underlying map for passing to `predict_character_behaviors()`.
    pub fn as_map(&self) -> &HashMap<EntityId, Vec<HistoryEntry>> {
        &self.entries
    }
}
```

- [ ] **Step 6: Register the module**

In `crates/storyteller-ml/src/lib.rs`, add after line 28 (`pub mod matrix;`):

```rust
pub mod prediction_history;
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p storyteller-ml prediction_history --all-features`
Expected: All 12 tests PASS.

- [ ] **Step 8: Run clippy and fmt**

Run: `cargo clippy -p storyteller-ml --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: Clean.

- [ ] **Step 9: Commit**

```bash
git add crates/storyteller-ml/src/prediction_history.rs crates/storyteller-ml/src/lib.rs
git commit -m "feat(ml): add PredictionHistory ring buffer with HistoryEntry construction"
```

---

## Chunk 2: Wire History Through predict_character_behaviors()

### Task 2: Add `history` parameter to `predict_character_behaviors()`

**Files:**
- Modify: `crates/storyteller-engine/src/context/prediction.rs:42-98`

The function currently hardcodes `history: &[]` on line 71. We add a `HashMap<EntityId, Vec<HistoryEntry>>` parameter and look up per-character history inside the `.map()` closure.

- [ ] **Step 1: Write the failing test**

Add to the test module in `prediction.rs`. This test verifies the function signature accepts history and passes it through. It doesn't require the ONNX model — it just verifies compilation and plumbing. **However**, `predict_character_behaviors()` calls `predictor.predict_batch()` which requires a real model. So this test must be feature-gated with `test-ml-model`.

Add to the existing test module (after the `enriched_predictions_have_valid_fields` test):

```rust
#[test]
fn predict_with_history_parameter() {
    use std::collections::HashMap;
    use storyteller_ml::prediction_history::PredictionHistory;

    let predictor = CharacterPredictor::load(&model_path()).expect("model should load");
    let grammar = PlutchikWestern::new();
    let scene = crate::workshop::the_flute_kept::scene();
    let sheet = test_character_sheet("Bramblehoof");
    let characters: Vec<&CharacterSheet> = vec![&sheet];

    // Turn 1: predict with no history
    let mut history = PredictionHistory::default();
    let (predictions_first, _) = predict_character_behaviors(
        &predictor,
        &characters,
        &scene,
        "Hello there",
        &grammar,
        None,
        &HashMap::new(),
    );

    // Push first turn's predictions into history
    for pred in &predictions_first {
        history.push_from_prediction(pred);
    }

    // Turn 2: predict with accumulated history
    let (predictions_second, _) = predict_character_behaviors(
        &predictor,
        &characters,
        &scene,
        "Hello again",
        &grammar,
        None,
        history.as_map(),
    );

    assert!(
        !predictions_second.is_empty(),
        "Should produce predictions with history"
    );

    // Verify the history was actually populated (pipeline plumbing check).
    // Feature-level encoding assertions (non-zero at indices 405-452) are
    // covered by the encode_with_history test in feature_schema.rs.
    let entries = history.get(sheet.entity_id);
    assert_eq!(entries.len(), 1, "Should have one history entry after turn 1");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine predict_with_history_parameter --all-features`
Expected: FAIL — `predict_character_behaviors` doesn't accept 7 arguments yet.

- [ ] **Step 3: Add the history parameter**

In `crates/storyteller-engine/src/context/prediction.rs`, modify the function signature at line 42:

Change:
```rust
pub fn predict_character_behaviors(
    predictor: &CharacterPredictor,
    characters: &[&CharacterSheet],
    scene: &SceneData,
    player_input: &str,
    grammar: &dyn EmotionalGrammar,
    event_classifier: Option<&EventClassifier>,
) -> (Vec<CharacterPrediction>, Option<ClassificationOutput>) {
```

To:
```rust
pub fn predict_character_behaviors(
    predictor: &CharacterPredictor,
    characters: &[&CharacterSheet],
    scene: &SceneData,
    player_input: &str,
    grammar: &dyn EmotionalGrammar,
    event_classifier: Option<&EventClassifier>,
    history: &std::collections::HashMap<storyteller_core::types::entity::EntityId, Vec<storyteller_ml::feature_schema::HistoryEntry>>,
) -> (Vec<CharacterPrediction>, Option<ClassificationOutput>) {
```

Then inside the `.map()` closure (around line 64-80), change line 71 from:

```rust
                history: &[],
```

To:

```rust
                history: history
                    .get(&sheet.entity_id)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]),
```

- [ ] **Step 4: Fix all call sites**

The function signature changed — all callers must pass the new `history` parameter.

**In `commands.rs`** (line 472), change:

```rust
        let (predictions, _classification) = predict_character_behaviors(
            predictor,
            &characters_refs,
            &engine.scene,
            &input,
            &engine.grammar,
            engine.event_classifier.as_ref(),
        );
```

To:

```rust
        let (predictions, _classification) = predict_character_behaviors(
            predictor,
            &characters_refs,
            &engine.scene,
            &input,
            &engine.grammar,
            engine.event_classifier.as_ref(),
            &std::collections::HashMap::new(),
        );
```

(This is a temporary empty map — Task 4 will replace it with the real history.)

**In `turn_cycle.rs`** (line 145), change:

```rust
    let (predictions, classification) = crate::context::prediction::predict_character_behaviors(
        &predictor.0,
        &characters,
        &scene_res.scene,
        input,
        grammar_res.0.as_ref(),
        classifier_ref,
    );
```

To:

```rust
    let (predictions, classification) = crate::context::prediction::predict_character_behaviors(
        &predictor.0,
        &characters,
        &scene_res.scene,
        input,
        grammar_res.0.as_ref(),
        classifier_ref,
        &std::collections::HashMap::new(),
    );
```

- [ ] **Step 5: Run all tests to verify nothing breaks**

Run: `cargo test --workspace --all-features 2>&1 | tail -20`
Expected: All tests PASS.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: Clean.

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-engine/src/context/prediction.rs crates/storyteller-engine/src/systems/turn_cycle.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(engine): add history parameter to predict_character_behaviors"
```

---

## Chunk 3: Workshop Integration — Accumulate and Pass History

### Task 3: Add `prediction_history` field to `EngineState`

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/engine_state.rs`

- [ ] **Step 1: Add the field**

In `engine_state.rs`, add the import at the top:

```rust
use storyteller_ml::prediction_history::PredictionHistory;
```

Add a new field to `EngineState` after `player_entity_id` (line 47):

```rust
    /// Accumulated prediction history for turn-over-turn ML context.
    pub prediction_history: PredictionHistory,
```

- [ ] **Step 2: Fix all construction sites**

Search for `EngineState {` in the workshop crate and add `prediction_history: PredictionHistory::default(),` to each constructor. These are in `commands.rs` (the `setup_and_render_opening` and `resume_session` functions).

Run: `grep -n "EngineState {" crates/storyteller-workshop/src-tauri/src/commands.rs` to find exact lines.

At each construction site, add:

```rust
            prediction_history: PredictionHistory::default(),
```

You will also need to add the import in `commands.rs`:

```rust
use storyteller_ml::prediction_history::PredictionHistory;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/engine_state.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): add prediction_history field to EngineState"
```

### Task 4: Wire history accumulation and passing in `commands.rs`

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

This replaces the temporary `HashMap::new()` from Task 2 with the real history, and adds accumulation after predictions complete.

- [ ] **Step 1: Pass history to `predict_character_behaviors()`**

In `commands.rs`, at the `predict_character_behaviors` call (around line 472), change the temporary empty map:

```rust
            &std::collections::HashMap::new(),
```

To:

```rust
            engine.prediction_history.as_map(),
```

- [ ] **Step 2: Accumulate history after predictions**

After the prediction block completes (after the `ResolverOutput` is built, around line 496), add history accumulation. Insert this after `let prediction_ms = ...;` (line 497) and before the `emit_debug` call (line 499):

```rust
    // Accumulate prediction history for next turn's Region 7 features.
    for pred in &resolver_output.original_predictions {
        engine.prediction_history.push_from_prediction(pred);
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): accumulate and pass prediction history each turn"
```

### Task 5: Full integration verification

**Files:** None (verification only)

- [ ] **Step 1: Run workspace tests**

Run: `cargo test --workspace --all-features 2>&1 | tail -20`
Expected: All tests PASS (except pre-existing failures requiring `STORYTELLER_DATA_PATH` or Ollama).

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -10`
Expected: Clean.

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --check`
Expected: Clean.

---

## Summary

| Task | What | Commit message |
|------|------|----------------|
| 1 | `PredictionHistory` type + `HistoryEntry::from_prediction` + tests | `feat(ml): add PredictionHistory ring buffer with HistoryEntry construction` |
| 2 | Add `history` param to `predict_character_behaviors()` + fix all callers | `feat(engine): add history parameter to predict_character_behaviors` |
| 3 | Add `prediction_history` field to `EngineState` | `feat(workshop): add prediction_history field to EngineState` |
| 4 | Wire accumulation + passing in workshop `commands.rs` | `feat(workshop): accumulate and pass prediction history each turn` |
| 5 | Full integration verification | (no commit) |
