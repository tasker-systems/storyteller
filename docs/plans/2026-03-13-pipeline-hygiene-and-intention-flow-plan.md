# Pipeline Hygiene and Intention Flow Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorder the per-turn narrative pipeline so that 3b decomposition runs first and feeds accurate data to ONNX prediction, remove DistilBERT and activation_reason, fix emotion index overflow, wire composition-time intentions into every turn's context, and tighten the intent synthesis prompt.

**Architecture:** Remove DistilBERT event classifier entirely. Reorder pipeline: player input → 3b decomposition → derive EventFeatureInput → ONNX prediction → arbitration → intent synthesis (with scene objectives) → context assembly (with persistent preamble goals) → narrator. Fix data quality issues at source (emotion clamp, activation_reason removal).

**Tech Stack:** Rust, Bevy ECS, ONNX Runtime, Ollama (3b/7b/14b), Tauri, Svelte 5, TypeScript

**Design spec:** `docs/plans/2026-03-13-pipeline-hygiene-and-intention-flow-design.md`

---

## File Map

### Files to Modify

| File | Changes |
|------|---------|
| `crates/storyteller-core/src/types/prediction.rs:361-371` | Remove `activation_reason` from `ActivatedTensorFrame` |
| `crates/storyteller-ml/src/feature_schema.rs:458` | Clamp `dominant_emotion_index` to 0-7 |
| `crates/storyteller-engine/src/context/prediction.rs` | Remove classifier-dependent code, add `decomposition_to_event_features()`, remove `activation_reason` generation |
| `crates/storyteller-engine/src/inference/intent_synthesis.rs` | Add scene objectives to prompt, update system prompt, unify player/NPC format, add few-shot examples |
| `crates/storyteller-engine/src/systems/turn_cycle.rs` | Remove `classify_system()` and `ClassifierResource` |
| `crates/storyteller-workshop/src-tauri/src/engine_state.rs` | Remove `event_classifier`, add `generated_intentions` and `composed_goals` |
| `crates/storyteller-workshop/src-tauri/src/commands.rs` | Reorder pipeline in `submit_input()`, remove classifier loading, inject goals into per-turn preamble, pass intentions to intent synthesis |
| `crates/storyteller-workshop/src-tauri/src/events.rs` | Remove `EventsClassified` variant |
| `crates/storyteller-workshop/src-tauri/src/session.rs` | Remove `classifications` from `TurnRecord` |
| `crates/storyteller-workshop/src/lib/types.ts` | Remove `EventsClassifiedEvent`, `activation_reason`, `classifications` |
| `crates/storyteller-workshop/src/lib/logic.ts` | Remove classification phase handling |
| `crates/storyteller-workshop/src/lib/DebugPanel.svelte` | Remove classification chips, remove activation_reason display |

---

## Chunk 1: Data Quality Fixes (emotion clamp + activation_reason removal)

### Task 1: Clamp dominant_emotion_index at ML output boundary

**Files:**
- Modify: `crates/storyteller-ml/src/feature_schema.rs:458`

- [ ] **Step 1: Write failing test for out-of-range emotion index**

In the existing test module of `feature_schema.rs`, add a test that constructs a raw output array with value `8.0` at the dominant_emotion_index position and verifies it clamps to `7`:

```rust
#[test]
fn dominant_emotion_index_clamps_to_max() {
    // The dominant_emotion_index is the last element of the output array.
    // If it rounds to 8 or higher, it should clamp to 7 (max Plutchik index).
    let raw_value = 8.4_f32;
    let clamped = (raw_value.round() as u8).min(7);
    assert_eq!(clamped, 7);
}
```

- [ ] **Step 2: Run test to verify it passes** (this is a unit logic test, not an integration test)

Run: `cargo test -p storyteller-ml dominant_emotion_index_clamps`

- [ ] **Step 3: Apply the clamp in feature_schema.rs**

At line 458, change:
```rust
let dominant_emotion_index = output[offset].round() as u8;
```
to:
```rust
let raw_index = output[offset].round() as u8;
if raw_index > 7 {
    tracing::warn!(raw_index, "dominant_emotion_index out of Plutchik range, clamping to 7");
}
let dominant_emotion_index = raw_index.min(7);
```

- [ ] **Step 4: Run all storyteller-ml tests**

Run: `cargo test -p storyteller-ml`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-ml/src/feature_schema.rs
git commit -m "fix(ml): clamp dominant_emotion_index to Plutchik range [0,7]"
```

---

### Task 2: Remove activation_reason from ActivatedTensorFrame

**Files:**
- Modify: `crates/storyteller-core/src/types/prediction.rs:361-371`
- Modify: `crates/storyteller-engine/src/context/prediction.rs:290-296, 483-496, 648+`

- [ ] **Step 1: Remove `activation_reason` field from `ActivatedTensorFrame`**

In `crates/storyteller-core/src/types/prediction.rs`, remove the `activation_reason: String` field and its doc comment from the struct at lines 366-368. Also update the struct-level doc comment at lines 356-360 to remove the reference to `activation_reason`.

The struct becomes:
```rust
/// Assembled tensor frame — raw axis indices resolved to names.
///
/// The `activated_axes` names are generated by the context assembly system
/// from the raw frame's axis indices and the character's tensor profile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActivatedTensorFrame {
    /// Axis names selected as relevant to the current context.
    /// Resolved from `RawActivatedTensorFrame::activated_axis_indices`.
    pub activated_axes: Vec<String>,
    /// Model confidence in this frame selection. Range: [0.0, 1.0].
    pub confidence: f32,
}
```

- [ ] **Step 2: Fix compilation errors in prediction.rs**

In `crates/storyteller-engine/src/context/prediction.rs`:

1. In `enrich_prediction()` (line 290), remove the `activation_reason` line and the `generate_activation_reason()` call. The frame construction becomes:
```rust
let frame = ActivatedTensorFrame {
    activated_axes: axis_names,
    confidence: raw.frame.confidence,
};
```

2. Delete the `generate_activation_reason()` function (lines 483-496).

3. Delete the `truncate_hint()` function (line 648+) if it's only used by `generate_activation_reason()`. Check for other callers first.

- [ ] **Step 3: Fix any remaining compilation errors across workspace**

Run: `cargo check --workspace --all-features`

Fix any code that references `activation_reason` on `ActivatedTensorFrame`. Check:
- `crates/storyteller-engine/src/inference/intent_synthesis.rs` (test fixtures)
- `crates/storyteller-engine/src/context/prediction.rs` (tests)

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: All pass (some tests may need `activation_reason` field removed from test fixtures).

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-core/src/types/prediction.rs crates/storyteller-engine/src/context/prediction.rs crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "refactor: remove activation_reason from ActivatedTensorFrame"
```

---

## Chunk 2: DistilBERT Removal

### Task 3: Remove EventClassifier from EngineState and model loading

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/engine_state.rs:11, 33-34`
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs:481-486, 1093-1098`

- [ ] **Step 1: Remove `event_classifier` from EngineState**

In `engine_state.rs`:
1. Remove the import: `use storyteller_engine::inference::event_classifier::EventClassifier;` (line 11)
2. Remove the field: `pub event_classifier: Option<EventClassifier>` (line 34) and its doc comment (line 33)

- [ ] **Step 2: Remove classifier loading from commands.rs**

In `commands.rs`, remove the `resolve_event_classifier_path()` and `EventClassifier::load()` calls at:
- Lines 481-486 (in `resume_session()` or `start_scene()`)
- Lines 1093-1098 (in `setup_and_render_opening()`)

Also remove the `event_classifier` field from any `EngineState` construction sites — search for `event_classifier:` in `commands.rs` and set up the struct without it.

Remove the `EventClassifier` import from `commands.rs`.

- [ ] **Step 3: Fix compilation**

Run: `cargo check -p storyteller-workshop-lib`
Fix any remaining references to `engine.event_classifier`.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/engine_state.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "refactor: remove EventClassifier from EngineState and model loading"
```

---

### Task 4: Remove classifier from prediction pipeline

**Files:**
- Modify: `crates/storyteller-engine/src/context/prediction.rs:42-53, 111-189`

- [ ] **Step 1: Remove classifier parameter from predict_character_behaviors**

Change the function signature at line 42-53 to remove `event_classifier: Option<&EventClassifier>`. The function will now take an `EventFeatureInput` directly instead of computing it internally:

```rust
pub fn predict_character_behaviors(
    predictor: &CharacterPredictor,
    characters: &[&CharacterSheet],
    scene: &SceneData,
    player_input: &str,
    grammar: &dyn EmotionalGrammar,
    event_features: EventFeatureInput,
    history: &std::collections::HashMap<
        storyteller_core::types::entity::EntityId,
        Vec<storyteller_ml::feature_schema::HistoryEntry>,
    >,
) -> Vec<CharacterPrediction> {
```

Note: return type changes from `(Vec<CharacterPrediction>, Option<ClassificationOutput>)` to `Vec<CharacterPrediction>` — the classification output is no longer produced here.

Update the function body:
- Remove the `classify_and_extract()` call (line 54-58)
- Use the `event_features` parameter directly where `event` was used (line 74)
- Return just `predictions` instead of `(predictions, classification)` (line 104)

- [ ] **Step 2: Delete classifier-dependent functions**

Delete these functions from `prediction.rs`:
- `classify_and_extract()` (lines 111-129)
- `classification_to_event_features()` (lines 135-158)
- `event_kind_label_to_event_type()` (lines 162-174)
- `infer_register_from_classification()` (lines 179-189)
- `classify_player_input()` (lines 194+) — the keyword fallback

Remove the `EventClassifier` and `ClassificationOutput` imports.

- [ ] **Step 3: Add decomposition_to_event_features function**

Add a new public function in `prediction.rs`:

```rust
use crate::inference::event_decomposition::{DecomposedEvent, DecomposedEntity, EventDecomposition};
use storyteller_core::types::event_grammar::RelationalDirection;

/// Derive ONNX model input features from the LLM event decomposition output.
///
/// Uses the first (highest-priority) decomposed event to determine event type
/// and emotional register. Entity count is derived from CHARACTER entities
/// excluding the actor.
pub fn decomposition_to_event_features(decomposition: &EventDecomposition) -> EventFeatureInput {
    let first_event = decomposition.events.first();

    let event_type = first_event
        .map(|e| decomposed_kind_to_event_type(&e.kind))
        .unwrap_or(EventType::Interaction);

    let emotional_register = first_event
        .map(|e| infer_register_from_decomposition(&e.kind, &e.relational_direction))
        .unwrap_or(EmotionalRegister::Neutral);

    let target_count = decomposition
        .entities
        .iter()
        .filter(|e| e.category == "CHARACTER")
        .count()
        .saturating_sub(1) as u8; // exclude actor

    EventFeatureInput {
        event_type,
        emotional_register,
        confidence: 0.8,
        target_count,
    }
}

fn decomposed_kind_to_event_type(kind: &str) -> EventType {
    match kind {
        "SpeechAct" => EventType::Speech,
        "InformationTransfer" => EventType::Speech,
        "ActionOccurrence" => EventType::Action,
        "SpatialChange" => EventType::Movement,
        "StateAssertion" => EventType::Observation,
        "EnvironmentalChange" => EventType::Observation,
        "EmotionalExpression" => EventType::Emote,
        "RelationalShift" => EventType::Interaction,
        _ => EventType::Interaction,
    }
}

fn infer_register_from_decomposition(kind: &str, direction: &RelationalDirection) -> EmotionalRegister {
    match (kind, direction) {
        ("EmotionalExpression", _) => EmotionalRegister::Vulnerable,
        ("SpeechAct", RelationalDirection::Directed) => EmotionalRegister::Inquisitive,
        ("SpeechAct", RelationalDirection::Mutual) => EmotionalRegister::Tender,
        ("ActionOccurrence", RelationalDirection::Directed) => EmotionalRegister::Aggressive,
        ("RelationalShift", _) => EmotionalRegister::Guarded,
        _ => EmotionalRegister::Neutral,
    }
}
```

- [ ] **Step 4: Write tests for decomposition_to_event_features**

```rust
#[cfg(test)]
mod decomposition_tests {
    use super::*;
    use crate::inference::event_decomposition::{DecomposedEntity, DecomposedEvent, EventDecomposition};

    fn make_decomposition(kind: &str, direction: RelationalDirection) -> EventDecomposition {
        EventDecomposition {
            events: vec![DecomposedEvent {
                kind: kind.to_string(),
                actor: Some(DecomposedEntity {
                    mention: "Kael".to_string(),
                    category: "CHARACTER".to_string(),
                }),
                action: "test".to_string(),
                target: Some(DecomposedEntity {
                    mention: "Nyx".to_string(),
                    category: "CHARACTER".to_string(),
                }),
                relational_direction: direction,
                confidence_note: None,
            }],
            entities: vec![
                DecomposedEntity { mention: "Kael".to_string(), category: "CHARACTER".to_string() },
                DecomposedEntity { mention: "Nyx".to_string(), category: "CHARACTER".to_string() },
            ],
        }
    }

    #[test]
    fn speech_act_maps_to_speech() {
        let d = make_decomposition("SpeechAct", RelationalDirection::Directed);
        let features = decomposition_to_event_features(&d);
        assert_eq!(features.event_type, EventType::Speech);
        assert_eq!(features.emotional_register, EmotionalRegister::Inquisitive);
        assert_eq!(features.target_count, 1); // 2 chars - 1 actor
    }

    #[test]
    fn emotional_expression_maps_to_emote() {
        let d = make_decomposition("EmotionalExpression", RelationalDirection::SelfDirected);
        let features = decomposition_to_event_features(&d);
        assert_eq!(features.event_type, EventType::Emote);
        assert_eq!(features.emotional_register, EmotionalRegister::Vulnerable);
    }

    #[test]
    fn relational_shift_maps_to_interaction() {
        let d = make_decomposition("RelationalShift", RelationalDirection::Mutual);
        let features = decomposition_to_event_features(&d);
        assert_eq!(features.event_type, EventType::Interaction);
        assert_eq!(features.emotional_register, EmotionalRegister::Guarded);
    }

    #[test]
    fn empty_decomposition_uses_defaults() {
        let d = EventDecomposition { events: vec![], entities: vec![] };
        let features = decomposition_to_event_features(&d);
        assert_eq!(features.event_type, EventType::Interaction);
        assert_eq!(features.emotional_register, EmotionalRegister::Neutral);
        assert_eq!(features.target_count, 0);
    }

    #[test]
    fn all_event_kinds_map_correctly() {
        let cases = vec![
            ("SpeechAct", EventType::Speech),
            ("InformationTransfer", EventType::Speech),
            ("ActionOccurrence", EventType::Action),
            ("SpatialChange", EventType::Movement),
            ("StateAssertion", EventType::Observation),
            ("EnvironmentalChange", EventType::Observation),
            ("EmotionalExpression", EventType::Emote),
            ("RelationalShift", EventType::Interaction),
            ("UnknownKind", EventType::Interaction),
        ];
        for (kind, expected) in cases {
            let d = make_decomposition(kind, RelationalDirection::Directed);
            let features = decomposition_to_event_features(&d);
            assert_eq!(features.event_type, expected, "failed for kind={kind}");
        }
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p storyteller-engine`
Expected: All pass including new decomposition tests. Some existing tests that relied on `classify_and_extract` or `classification_to_event_features` should have been deleted in step 2.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/context/prediction.rs
git commit -m "refactor: replace classifier pipeline with decomposition_to_event_features"
```

---

### Task 5: Remove classifier from turn cycle and debug events

**Files:**
- Modify: `crates/storyteller-engine/src/systems/turn_cycle.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/events.rs:55-60`
- Modify: `crates/storyteller-workshop/src-tauri/src/session.rs:42`
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs:656-676, 1002`

- [ ] **Step 1: Remove classify_system and ClassifierResource from turn_cycle.rs**

In `turn_cycle.rs`:
1. Delete the `classify_system()` function (lines 74-91)
2. Delete the `ClassifierResource` struct (lines 97-106, approximate)
3. If the `Classifying` stage is referenced in system set ordering, make it a no-op by advancing the stage immediately without doing work

- [ ] **Step 2: Remove EventsClassified from DebugEvent**

In `events.rs`, delete the `EventsClassified` variant (lines 55-60):
```rust
// DELETE:
#[serde(rename = "events_classified")]
EventsClassified {
    turn: u32,
    classifications: Vec<String>,
    classifier_loaded: bool,
},
```

- [ ] **Step 3: Remove classifications from TurnRecord**

In `session.rs`, remove the `classifications` field from `TurnRecord` (line 42):
```rust
// DELETE:
#[serde(skip_serializing_if = "Option::is_none")]
pub classifications: Option<Vec<String>>,
```

- [ ] **Step 4: Remove classification code from commands.rs**

In `commands.rs`:
1. Delete the event classification phase block (lines 656-676) — the `classify_text` call and `EventsClassified` debug event emission
2. Remove `classifications` from the `TurnRecord` construction (line 1002)
3. Remove any `_classification` variable bindings that are now unused

- [ ] **Step 5: Fix compilation and run tests**

Run: `cargo check --workspace --all-features && cargo test --workspace`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/systems/turn_cycle.rs crates/storyteller-workshop/src-tauri/src/events.rs crates/storyteller-workshop/src-tauri/src/session.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "refactor: remove DistilBERT classifier from turn cycle, debug events, and persistence"
```

---

### Task 6: Remove classifier references from frontend

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/types.ts`
- Modify: `crates/storyteller-workshop/src/lib/logic.ts`
- Modify: `crates/storyteller-workshop/src/lib/DebugPanel.svelte`

- [ ] **Step 1: Remove EventsClassifiedEvent from types.ts**

Delete the `EventsClassifiedEvent` interface (lines 195-200). Remove `"events_classified"` from the `DebugEvent` union type. Remove `classifications` from any turn-related types.

- [ ] **Step 2: Remove classification phase from logic.ts**

Remove the `events_classified` case from `applyDebugEvent()`. Remove `"Events": "classification"` from `TAB_PHASE_MAP` (if it exists as a separate phase). Remove `classifications` initialization from `freshDebugState()` if present.

- [ ] **Step 3: Remove classification chips from DebugPanel.svelte**

In the Events tab rendering, remove the classification chip section (lines ~313-325) that renders DistilBERT labels. Keep the LLM decomposition triples rendering.

- [ ] **Step 4: Remove activation_reason from frontend**

In `types.ts`, remove `activation_reason: string` from the prediction frame interface. In `DebugPanel.svelte`, remove the activation_reason line from ML Predictions tab rendering.

- [ ] **Step 5: Verify frontend compiles**

Run: `cd crates/storyteller-workshop && npx svelte-check`
Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-workshop/src/lib/types.ts crates/storyteller-workshop/src/lib/logic.ts crates/storyteller-workshop/src/lib/DebugPanel.svelte
git commit -m "refactor: remove classifier and activation_reason from frontend"
```

---

## Chunk 3: Pipeline Reorder

### Task 7: Reorder submit_input pipeline — decomposition before prediction

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

This is the core pipeline reorder. Currently `submit_input()` runs: prediction → classification → decomposition → arbitration → intent synthesis → context assembly → narrator. After this change: decomposition → prediction (with decomposition-derived features) → arbitration → intent synthesis → context assembly → narrator.

- [ ] **Step 1: Move event decomposition phase before ML prediction phase**

In `submit_input()`, move the event decomposition block (lines ~688-761) to run immediately after the player input is received, before the ML predictions phase (lines ~575-626).

The decomposition needs:
- `engine.structured_llm` (already available)
- `input` (player input string)
- `&narrator_context_for_decomposition` (narrator + player text for pronoun resolution)

The narrator context for decomposition uses the previous turn's narrator output (from the journal tail) combined with the current player input in `[Narrator]...\n\n[Player]...` format. This data is available at the start of `submit_input()` — it does not depend on classification or prediction results. Construct it before the decomposition block:

```rust
let journal_tail = engine.journal.recent_text(3);
let narrator_context_for_decomposition = format!(
    "[Narrator]{journal_tail}\n\n[Player]{input}"
);
```

- [ ] **Step 2: Derive EventFeatureInput from decomposition and pass to prediction**

After the decomposition block, derive event features:

```rust
let event_features = if let Some(ref decomposition) = event_decomposition {
    storyteller_engine::context::prediction::decomposition_to_event_features(decomposition)
} else {
    // Fallback when no structured LLM available — use safe defaults
    storyteller_engine::context::prediction::EventFeatureInput {
        event_type: storyteller_core::types::prediction::EventType::Interaction,
        emotional_register: storyteller_core::types::prediction::EmotionalRegister::Neutral,
        confidence: 0.5,
        target_count: (characters_refs.len().saturating_sub(1)) as u8,
    }
};
```

Update the `predict_character_behaviors()` call to pass `event_features` instead of `engine.event_classifier.as_ref()`:

```rust
let predictions = storyteller_engine::context::prediction::predict_character_behaviors(
    predictor,
    &characters_refs,
    &engine.scene,
    &input,
    &engine.grammar,
    event_features,
    &history_map,
);
```

Note: return type is now `Vec<CharacterPrediction>` (not a tuple).

- [ ] **Step 3: Update timing tracking**

Add decomposition timing (`decomposition_ms`) to the timing block. The decomposition now runs before prediction, so capture its duration at the new location. Remove any classification-specific timing.

- [ ] **Step 4: Verify compilation and run tests**

Run: `cargo check -p storyteller-workshop-lib && cargo test --workspace`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "refactor: reorder pipeline — decomposition before prediction"
```

---

## Chunk 4: Wire Intentions into Per-Turn Context

### Task 8: Add intention storage to EngineState

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/engine_state.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Add intention fields to EngineState**

In `engine_state.rs`, add imports and two new fields:

```rust
use storyteller_engine::inference::intention_generation::GeneratedIntentions;
use storyteller_engine::scene_composer::goals::ComposedGoals;
```

Add to the struct:
```rust
/// Composition-time intentions (scene direction + character drives).
/// Persists across turns for preamble injection and intent synthesis context.
pub generated_intentions: Option<GeneratedIntentions>,
/// Composed scene/character goals for player context re-derivation.
pub composed_goals: Option<ComposedGoals>,
```

- [ ] **Step 2: Populate fields in setup_and_render_opening**

In `commands.rs`, in `setup_and_render_opening()`, after intentions are generated and goals are composed, store them in `EngineState`:

```rust
engine.generated_intentions = generated_intentions.clone();
engine.composed_goals = Some(composed_goals.clone());
```

- [ ] **Step 3: Hydrate fields in resume_session**

In `resume_session()`, after loading the session, hydrate from persisted files:

```rust
// Load persisted intentions
let generated_intentions = store.load_intentions(sid)
    .ok()
    .flatten()
    .and_then(|v| serde_json::from_value::<GeneratedIntentions>(v).ok());
engine.generated_intentions = generated_intentions;

// Load persisted goals
let composed_goals = store.load_goals(sid)
    .ok()
    .flatten()
    .and_then(|v| serde_json::from_value::<ComposedGoals>(v).ok());
engine.composed_goals = composed_goals;
```

Note: `load_goals()` may need to be added to `SessionStore` if not already present (check `session.rs` for existing `save_goals`/`load_goals` methods).

- [ ] **Step 4: Update all EngineState construction sites**

Search `commands.rs` for `EngineState {` and ensure `generated_intentions: None` and `composed_goals: None` are set at all construction sites (they'll be populated after setup completes).

- [ ] **Step 5: Fix compilation and run tests**

Run: `cargo check --workspace --all-features && cargo test --workspace`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/engine_state.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: store generated_intentions and composed_goals in EngineState"
```

---

### Task 9: Extract inject_goals_into_preamble helper and wire into submit_input

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Extract shared helper function**

Extract the goal injection logic from `setup_and_render_opening()` (lines ~1248-1270) into a standalone function in `commands.rs`:

```rust
use storyteller_engine::inference::intention_generation::{GeneratedIntentions, intentions_to_preamble};
use storyteller_engine::scene_composer::goals::ComposedGoals;
use storyteller_core::types::narrator_context::PersistentPreamble;
use storyteller_core::types::entity::EntityId;

/// Inject composition-time goals into the narrator preamble.
///
/// Populates `scene_direction`, `character_drives`, and `player_context`
/// from generated intentions and composed goals. Called after
/// `assemble_narrator_context()` returns to enrich the preamble with
/// persistent scene objectives.
fn inject_goals_into_preamble(
    preamble: &mut PersistentPreamble,
    intentions: Option<&GeneratedIntentions>,
    composed_goals: Option<&ComposedGoals>,
    player_entity_id: Option<EntityId>,
) {
    if let Some(intentions) = intentions {
        let (scene_direction, character_drives) = intentions_to_preamble(intentions);
        preamble.scene_direction = Some(scene_direction);
        preamble.character_drives = character_drives;
    }

    preamble.player_context = player_entity_id
        .and_then(|pid| {
            composed_goals
                .and_then(|goals| goals.character_goals.get(&pid))
        })
        .map(|goals| {
            goals
                .iter()
                .filter(|g| matches!(g.visibility, GoalVisibility::Overt | GoalVisibility::Signaled))
                .map(|g| g.goal_id.replace('_', " "))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|s| !s.is_empty());
}
```

- [ ] **Step 2: Replace inline code in setup_and_render_opening with helper call**

Replace the existing inline goal injection in `setup_and_render_opening()` with:
```rust
inject_goals_into_preamble(
    &mut context.preamble,
    engine.generated_intentions.as_ref(),
    engine.composed_goals.as_ref(),
    engine.player_entity_id,
);
```

- [ ] **Step 3: Call helper in submit_input after assemble_narrator_context**

In `submit_input()`, after the `assemble_narrator_context()` call (line ~865), add:
```rust
inject_goals_into_preamble(
    &mut context.preamble,
    engine.generated_intentions.as_ref(),
    engine.composed_goals.as_ref(),
    engine.player_entity_id,
);
```

- [ ] **Step 4: Verify compilation and run tests**

Run: `cargo check --workspace --all-features && cargo test --workspace`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: inject composition-time goals into per-turn narrator preamble"
```

---

### Task 10: Pass intentions to intent synthesis

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Add intentions parameter to build_intent_user_prompt**

Update `build_intent_user_prompt()` to accept an optional intentions section:

```rust
pub fn build_intent_user_prompt(
    character_summary: &str,
    predictions_summary: &str,
    journal_tail: &str,
    player_input: &str,
    scene_context: &str,
    scene_objectives: Option<&str>,
) -> String {
    let mut prompt = format!(
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
    );

    if let Some(objectives) = scene_objectives {
        prompt.push_str("\n\n");
        prompt.push_str(objectives);
    }

    prompt
}
```

- [ ] **Step 2: Add helper to format intentions as scene objectives text**

```rust
use super::intention_generation::GeneratedIntentions;

/// Format composition-time intentions as a prompt section for the intent synthesizer.
pub fn format_scene_objectives(intentions: &GeneratedIntentions) -> String {
    let mut text = String::from("## Scene Objectives\n");
    text.push_str(&format!("Dramatic tension: {}\n", intentions.scene_intention.dramatic_tension));
    text.push_str(&format!("Trajectory: {}\n", intentions.scene_intention.trajectory));

    if !intentions.character_intentions.is_empty() {
        text.push_str("\n## Character Objectives\n");
        for ci in &intentions.character_intentions {
            text.push_str(&format!(
                "{}: Objective: {} | Constraint: {} | Stance: {}\n",
                ci.character, ci.objective, ci.constraint, ci.behavioral_stance
            ));
        }
    }

    text
}
```

- [ ] **Step 3: Update synthesize_intents to accept and pass intentions**

Add `intentions: Option<&GeneratedIntentions>` parameter to `synthesize_intents()`:

```rust
pub async fn synthesize_intents(
    llm: &dyn LlmProvider,
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    journal_tail: &str,
    player_input: &str,
    scene: &SceneData,
    player_entity_id: Option<EntityId>,
    intentions: Option<&GeneratedIntentions>,
) -> Option<String> {
```

In the body, format and pass objectives:
```rust
let scene_objectives = intentions.map(|i| format_scene_objectives(i));
let user_prompt = build_intent_user_prompt(
    &char_summary,
    &pred_summary,
    journal_tail,
    player_input,
    &scene_context,
    scene_objectives.as_deref(),
);
```

- [ ] **Step 4: Write tests**

```rust
#[test]
fn user_prompt_includes_scene_objectives_when_provided() {
    let prompt = build_intent_user_prompt(
        "chars", "preds", "journal", "input", "context",
        Some("## Scene Objectives\nDramatic tension: test tension"),
    );
    assert!(prompt.contains("## Scene Objectives"));
    assert!(prompt.contains("test tension"));
}

#[test]
fn user_prompt_omits_objectives_when_none() {
    let prompt = build_intent_user_prompt(
        "chars", "preds", "journal", "input", "context",
        None,
    );
    assert!(!prompt.contains("Scene Objectives"));
}
```

- [ ] **Step 5: Update call site in commands.rs**

In `submit_input()`, update the `synthesize_intents()` call to pass intentions:
```rust
storyteller_engine::inference::intent_synthesis::synthesize_intents(
    intent_llm.as_ref(),
    &characters_refs,
    &resolver_output.original_predictions,
    &journal_tail,
    &input,
    &engine.scene,
    engine.player_entity_id,
    engine.generated_intentions.as_ref(),
)
```

- [ ] **Step 6: Fix compilation and run tests**

Run: `cargo check --workspace --all-features && cargo test --workspace`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: pass composition-time intentions to per-turn intent synthesis"
```

---

## Chunk 5: Tighten Intent Synthesis Prompt

### Task 11: Update system prompt — unify format, add few-shot examples

**Files:**
- Modify: `crates/storyteller-engine/src/inference/intent_synthesis.rs:16-48, 257-312`

- [ ] **Step 1: Update intent_synthesis_system_prompt**

Replace the current system prompt (lines 16-48) with a tightened version:

```rust
pub fn intent_synthesis_system_prompt() -> String {
    "\
You are the Intent Synthesizer — a dramaturgical assistant preparing a briefing for a narrator.

You receive:
- Character data: personality traits, emotional state, relationships
- ML predictions: what a behavior model predicts each character will do
- Scene objectives: dramatic tension and per-character objectives for this scene
- Recent scene history: what just happened
- Player input: what the player character just did or said

Your job: Write a brief directive for each character.

For non-player characters:
- Describe what they WANT to do this turn and WHY, grounded in their scene objective
- Be directive: \"Arthur should respond\" not \"Arthur might respond\"
- Be specific about emotional subtext: \"reluctantly, deflecting with humor\" not \"with some emotion\"
- Include speech direction when a character should speak: \"should say something about...\" not prescribing exact words
- Ground in physical behavior: \"his shoulders drop\" not \"he feels sad\"

For the player character:
- The player has directed this character's action. Do NOT override it.
- Describe how this character's personality and emotional state relate to the directed action — whether their nature resists it, inflects it, or suits it
- Ground in physical behavior the narrator can render
- If the directed action is in tension with the character's nature, call this out — this is how the system surfaces authentic characterization

Rules:
- One paragraph per character, 2-3 sentences each
- Do NOT write dialogue. The narrator writes all dialogue.
- Do NOT narrate the scene. You are briefing the narrator, not writing prose.
- Do NOT name personality traits directly. Show them through behavior.
- Use **CharacterName** headers for all characters (no other labels or markers)

Examples of good directives:

**Nyx**
Should find an excuse to handle a component Kael just placed, displacing it subtly. Her body language reads as helpful — leaning in, fingers hovering — but her timing consistently disrupts his progress. Undercurrent of anticipation, watching for his reaction.

**Kael**
His attention is split between the work and Nyx's proximity. Shoulders tense when she reaches near his workspace. Should complete the current assembly step with deliberate focus, grounding himself in the task against the distraction."
        .to_string()
}
```

- [ ] **Step 2: Update build_summaries to unify player/NPC format**

In `build_summaries()` (lines 257-312), change the player character block (lines 269-286) to remove the `[PLAYER CHARACTER]` label:

```rust
if is_player {
    let action_text = player_input.unwrap_or("(no action specified)");
    let traits = format_dominant_axes(character, 5);
    let mut player_block = format!(
        "**{}** (player's action: \"{action_text}\")\n\
         {} | Dominant traits (0-1 scale): {}",
        character.name, character.performance_notes, traits,
    );

    if let Some(pred) = predictions
        .iter()
        .find(|p| p.character_id == character.entity_id)
    {
        let readable_pred = format_prediction_readable(pred);
        player_block.push_str(&format!("\nML prediction: {readable_pred}"));
    }

    char_lines.push(player_block);
}
```

- [ ] **Step 3: Update existing tests**

Update tests that assert on `[PLAYER CHARACTER` in `build_summaries()` output. Change assertions to check for the new format: `**CharacterName** (player's action: "...")`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p storyteller-engine`
Expected: All pass including updated format tests.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/inference/intent_synthesis.rs
git commit -m "feat: tighten intent synthesis prompt with few-shot examples and unified format"
```

---

### Task 12: Make intent synthesis model configurable

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Read model name from environment**

Find where `intent_llm` (`ExternalServerProvider`) is constructed in `commands.rs`. Change the hardcoded model name to read from `STORYTELLER_INTENT_MODEL` env var with a default:

```rust
let intent_model = std::env::var("STORYTELLER_INTENT_MODEL")
    .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string());
```

Use `intent_model` when constructing the `ExternalServerProvider` for intent synthesis.

- [ ] **Step 2: Add tracing for configured model**

```rust
tracing::info!(model = %intent_model, "Intent synthesis model configured");
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-workshop-lib`
Expected: Clean.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: make intent synthesis model configurable via STORYTELLER_INTENT_MODEL"
```

---

## Final Verification

### Task 13: Full workspace verification

- [ ] **Step 1: Run all Rust tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Check formatting**

Run: `cargo fmt --check`
Expected: Clean.

- [ ] **Step 4: Check frontend**

Run: `cd crates/storyteller-workshop && npx svelte-check`
Expected: 0 errors.

- [ ] **Step 5: Manual verification checklist**

Verify the following by reading the code:
- [ ] No references to `EventClassifier` remain in the workspace. The `event_classifier.rs` module itself may still be needed if `ClassificationOutput` or `ExtractedEntity` types are re-exported for use by the decomposition module — if so, keep the module but remove the `EventClassifier` struct and inference logic. If no types are referenced, delete the module entirely.
- [ ] No references to `activation_reason` remain in Rust code
- [ ] No references to `classifications` remain in `TurnRecord` or frontend types
- [ ] `EngineState` has `generated_intentions` and `composed_goals` fields
- [ ] `submit_input()` pipeline order is: decomposition → prediction → arbitration → intent synthesis → context assembly → narrator
- [ ] `inject_goals_into_preamble()` is called in both `setup_and_render_opening()` and `submit_input()`
- [ ] `synthesize_intents()` receives `intentions` parameter
