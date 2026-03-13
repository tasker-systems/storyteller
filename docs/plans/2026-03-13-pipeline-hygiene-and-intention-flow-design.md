# Pipeline Hygiene and Intention Flow

## Problem Statement

Playtest session `019ce4c3` revealed five interconnected issues in the per-turn narrative pipeline. While they manifest as separate symptoms — wrong emotion labels, static activation reasons, disappearing context, verbose intent directives, unused ML inference — they share a root cause: the pipeline was designed around DistilBERT's speed assumptions and carries forward heuristics and data gaps that degrade narrative quality after turn 0.

### Symptoms

1. **`unknown_8` emotion label**: The ONNX character prediction model outputs a float for `dominant_emotion_index` that rounds to 8, exceeding the Plutchik grammar's 0-7 range. The fallback `format!("unknown_{index}")` leaks an internal index into narrative-facing text.

2. **Static `activation_reason`**: `generate_activation_reason()` always uses `scene.stakes[0]` — literally the first stake string — regardless of what the character predicted or felt. Every frame across all turns shows `"Active in the context of Speech (Aggressive)"` even during tender moments.

3. **Intentions vanish after turn 0**: Composition-time intentions (scene direction, character drives, player context) are injected into the preamble during `setup_and_render_opening()` but never re-injected in `submit_input()`. The preamble drops from ~473 to ~234 tokens. The narrator loses dramatic orientation and begins echoing its own prior output.

4. **Intent synthesis produces prose, not directives**: The 3b-instruct model writes full scene narrative with dialogue instead of concise behavioral directives. It includes a `**PLAYER CHARACTER (Kael)**` section that references trait names, breaking diegetic framing.

5. **DistilBERT adds no value**: Event kind classifications are sparse or empty. Entity mentions are extracted but never used downstream. The LLM decomposition (3b) already does both jobs better. The classifier is wasted cycles and maintenance overhead.

### Compound Effects

These issues reinforce each other. Without intentions in the per-turn preamble (#3), the narrator has only its own prior rendering as context, causing repetitive descriptions. Without scene objectives flowing to the intent synthesizer (#3 again), NPC directives ignore authored dramatic intentions — Nyx warms to Kael instead of covertly sabotaging him. The synthesizer's verbose prose (#4) then gets echoed too literally by a narrator that lacks its own dramatic compass. Meanwhile, inaccurate emotion labels (#1) and static activation reasons (#2) pollute the prediction data that feeds intent synthesis, and DistilBERT (#5) occupies pipeline time that could go to higher-value stages.

## Design

### Pipeline Reorder

The current pipeline was ordered around DistilBERT providing fast event classification before the LLM stages. With DistilBERT removed, the 3b decomposition becomes the first analytical stage, and its output feeds downstream stages with real semantic understanding instead of keyword heuristics.

**Current pipeline:**
```
Player input
  → DistilBERT classification (event kinds + NER)
  → Keyword fallback → EventFeatureInput
  → ONNX character prediction
  → 3b event decomposition
  → Arbitration
  → 3b intent synthesis
  → Context assembly
  → 14b Narrator
```

**New pipeline:**
```
Player input
  → 3b event decomposition (action types, entities, relational directions)
  → Derive EventFeatureInput from decomposition output
  → ONNX character prediction (with accurate event features)
  → Arbitration (uses decomposition directly)
  → 3b/7b intent synthesis (with composition-time intentions as context)
  → Context assembly (preamble includes scene direction + character drives)
  → 14b Narrator
```

Each stage feeds accurate data to the next. No keyword heuristics as permanent infrastructure.

#### Latency Trade-off

The current pipeline runs DistilBERT (~50ms) and ONNX prediction concurrently with the 3b decomposition call. With the reorder, ONNX prediction waits for the 3b call to complete (~2-5s). However, the 3b decomposition already runs every turn in the current pipeline — the work is just reordered, not added. Total wall-clock time should be roughly the same since the 3b call was previously blocking context assembly anyway. The trade-off is acceptable: slightly later ONNX prediction start in exchange for accurate event features and removal of the DistilBERT overhead.

#### EventFeatureInput Derivation

New function `decomposition_to_event_features()` in `prediction.rs` maps 3b decomposition output to ONNX model input (`EventFeatureInput`). When the decomposition produces multiple events, use the first (highest-priority) event for the mapping.

**Event kind → EventType mapping:**

| DecomposedEvent.kind | EventType |
|---|---|
| `SpeechAct` | `Speech` |
| `InformationTransfer` | `Speech` |
| `ActionOccurrence` | `Action` |
| `SpatialChange` | `Movement` |
| `StateAssertion` | `Observation` |
| `EnvironmentalChange` | `Observation` |
| `EmotionalExpression` | `Emote` |
| `RelationalShift` | `Interaction` |
| (unknown/missing) | `Interaction` (safe default) |

**Entity count → target_count:** Count `DecomposedEntity` items with `category == "CHARACTER"`, excluding the actor. This maps to the number of characters the input is directed at.

**EmotionalRegister derivation from event kind + relational direction:**

| Condition | EmotionalRegister |
|---|---|
| `EmotionalExpression` | `Vulnerable` |
| `SpeechAct` + `RelationalDirection::Directed` | `Inquisitive` |
| `SpeechAct` + `RelationalDirection::Mutual` | `Tender` |
| `ActionOccurrence` + `RelationalDirection::Directed` | `Aggressive` |
| `RelationalShift` | `Guarded` |
| (default) | `Neutral` |

These mappings mirror the existing `event_kind_label_to_event_type()` and `infer_register_from_classification()` logic, adapted for the decomposition output structure. Confidence is set to `0.8` (fixed) since the 3b model's structured output doesn't produce per-field confidence scores.

This replaces both `classify_and_extract()` and the keyword fallback `classify_player_input()`.

### DistilBERT Removal

Clean removal across six areas:

**Model loading**: Remove `resolve_event_classifier_path()` and `EventClassifier::load()` calls in both `start_scene()` and `resume_session()`. Remove `event_classifier: Option<EventClassifier>` from `EngineState`.

**Turn cycle**: Remove `classify_system()` and `ClassifierResource` from Bevy systems. The `Classifying` variant in `TurnPhase` becomes a no-op that immediately advances to the next stage. The workshop (`commands.rs`) doesn't use the Bevy turn cycle systems — it calls functions directly — so this only affects the non-workshop Bevy runtime path, which is not actively used. A rename to `Decomposing` could happen later if the Bevy path is activated.

**Prediction pipeline**: Remove `event_classifier` parameter from `predict_character_behaviors()`. Remove `classify_and_extract()` and `classification_to_event_features()`. Replace with `decomposition_to_event_features()`.

**Workshop debug events**: Remove `EventsClassified` variant from `DebugEvent`. The Events tab already shows LLM decomposition triples — the classification chips at the top are removed.

**Frontend**: Remove classification chip rendering from Events tab. Remove `classifications` field from turn persistence schema entirely (no backwards compatibility — existing sessions can be fixed with `jq` if needed).

**Tests**: Remove `classify_and_extract_*` and `classification_to_features_*` tests. Add tests for `decomposition_to_event_features()`.

### Emotion Index Clamp

Fix at the ML output boundary in `feature_schema.rs`:

```rust
// Before:
let dominant_emotion_index = output[offset].round() as u8;

// After:
let raw_index = output[offset].round() as u8;
if raw_index > 7 {
    tracing::warn!(raw_index, "dominant_emotion_index out of Plutchik range, clamping to 7");
}
let dominant_emotion_index = raw_index.min(7);
```

Clamp at the source rather than downstream. The `unknown_{index}` fallback in `resolve_primary_name()` remains as a defensive guard but should never trigger.

### Remove activation_reason

Remove entirely rather than attempting to compute accurately. The field was a low-confidence inferential label that the narrator never saw — purely debug inspector metadata. The activated axes list already shows *what* activated, which is the useful information.

**prediction.rs**: Delete `generate_activation_reason()` and `truncate_hint()`. Remove `activation_reason` from frame construction.

**Core types**: Remove `activation_reason: String` from `ActivatedTensorFrame` in `storyteller_core::types::prediction`.

**Debug inspector**: Remove activation_reason line from ML Predictions tab rendering.

**Frontend types**: Remove `activation_reason` from the TypeScript prediction interface.

**Turn persistence**: Field disappears from predictions in `turns.jsonl`.

### Wire Intentions into Per-Turn Context

Two injection points serving complementary purposes: the preamble gives the narrator dramatic orientation ("what this scene is about"), while the intent synthesizer gets per-character objectives to ground its turn-specific directives.

#### 5a. Preamble Persistence

Store composition-time data in `EngineState`:
- `generated_intentions: Option<GeneratedIntentions>` — scene direction + character objectives
- `composed_goals: Option<ComposedGoals>` — for player context re-derivation

Extract shared helper from `setup_and_render_opening()`:

```rust
fn inject_goals_into_preamble(
    preamble: &mut PersistentPreamble,
    intentions: Option<&GeneratedIntentions>,
    player_goals: Option<&[CharacterGoal]>,
)
```

Called from both `setup_and_render_opening()` and `submit_input()`. In both cases, called after `assemble_narrator_context()` returns, mutating the `PersistentPreamble` fields on the returned `NarratorContext` struct — matching the existing pattern at lines ~1248-1270 of `commands.rs`. Restores the ~239 tokens that currently drop after turn 0 — scene direction (dramatic tension + trajectory), character drives (per-character objectives/constraints/stances), and player context (overt/signaled goal names).

`resume_session()` must also hydrate both `generated_intentions` and `composed_goals` into `EngineState` from their persisted JSON files (`intentions.json` and `goals.json`). The persistence layer already exists — `save_intentions()` and `load_intentions()` in `session.rs` — so this is wiring the load path into `EngineState` population.

#### 5b. Intent Synthesis Context

Add `intentions: Option<&GeneratedIntentions>` parameter to `synthesize_intents()`. In `build_intent_user_prompt()`, append scene objectives when available:

```
## Scene Objectives
Dramatic tension: {dramatic_tension}
Trajectory: {trajectory}

## Character Objectives
{character}: Objective: {objective} | Constraint: {constraint} | Stance: {behavioral_stance}
```

This grounds per-turn directives in authored dramatic purpose. Nyx's directive becomes "should find an excuse to handle a component Kael just placed" (from her sabotage objective) instead of generic warmth inferred from tensor data alone.

### Tighten Intent Synthesis Prompt

Three changes to the system prompt:

#### 6a. Unify Player/NPC Format

Remove the `[PLAYER CHARACTER]` visible label from output. All characters rendered as `**Name**` with identical formatting. The system prompt still distinguishes behavior — for the player character, the directive describes how the character's nature relates to the player's directed action (friction, alignment, inflection) — but the narrator sees uniform formatting.

The system prompt's "Rules for the player character" section still identifies which character is the player, but instructs the model not to label it in output. The input-side formatting in `build_summaries()` (which currently uses `[PLAYER CHARACTER — directed action: "..."]`) changes to include the player's directed action as context without the meta-label — e.g., `**Kael** (player's action: "...")`. The synthesizer uses this to write a friction-aware directive that reads identically to NPC directives in the output.

This preserves the valuable tension-surfacing behavior (the character's nature may resist or inflect the player's action) while keeping the output diegetic — no trait names, no meta-commentary.

#### 6b. Few-Shot Examples

Add 2-3 examples to the system prompt demonstrating the exact format and length:

```
**Nyx**
Should find an excuse to handle a component Kael just placed, displacing it subtly.
Her body language reads as helpful — leaning in, fingers hovering — but her timing
consistently disrupts his progress. Undercurrent of anticipation, watching for his reaction.

**Kael**
His attention is split between the work and Nyx's proximity. Shoulders tense when she
reaches near his workspace. Should complete the current assembly step with deliberate
focus, grounding himself in the task against the distraction.
```

These demonstrate: directive not prose, 2-3 sentences, physical grounding, no dialogue, no trait names.

#### 6c. Configurable Model

Make the intent synthesis model name configurable via `STORYTELLER_INTENT_MODEL` env var, read in the workshop's LLM provider setup where `intent_llm` (`ExternalServerProvider`) is constructed. Default to `qwen2.5:3b-instruct` with the tightened prompt. If playtesting shows 3b still produces prose instead of directives, swap to 7b by setting `STORYTELLER_INTENT_MODEL=qwen2.5:7b-instruct` without code changes.

Model tier summary:
- **3b-instruct**: event decomposition (structured JSON extraction)
- **3b-instruct (default) or 7b**: intent synthesis (behavioral directives)
- **14b-instruct**: narrator (prose rendering) and intention generation (composition-time)

## Testing Strategy

- **decomposition_to_event_features()**: Unit tests mapping known decomposition outputs to expected EventFeatureInput values, covering all 8 event kinds and the missing/unknown fallback
- **Emotion index clamp**: Unit test confirming indices 0-7 pass through, 8+ clamp to 7 with warning
- **inject_goals_into_preamble()**: Unit test confirming preamble fields populated from intentions, and empty when None
- **Intent synthesis with objectives**: Unit test confirming scene objectives section appears in user prompt when intentions provided, absent when None
- **Existing test updates**: Tests referencing `activation_reason` on `ActivatedTensorFrame` (e.g., in `intent_synthesis.rs` test fixtures) must be updated to remove the field. Tests asserting on `[PLAYER CHARACTER` markers in `build_summaries()` output must be updated for the new format.
- **Pipeline integration**: Playtest via workshop to validate end-to-end narrative quality (followup: automated playtest harness)

## Followup Work (Out of Scope)

- **Automated playtest harness**: Run scenes programmatically, produce `turns.jsonl` for analysis without manual UI interaction. Would enable validating pipeline changes without manual playthroughs.
- **Dynamics descriptor enrichment**: Expand `enabled_goals` in dynamics descriptors to reduce reliance on the goal fallback path, improving narrative coherence when goals are well-aligned with scene/archetype/dynamics affordances.
