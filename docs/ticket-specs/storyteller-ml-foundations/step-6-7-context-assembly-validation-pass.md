# Steps 6-7: Context Assembly, Enrichment, and End-to-End Validation

## Status: COMPLETE

## What Was Built

The final steps of the Phase 0 ML pipeline: prediction enrichment, Narrator rendering, and wiring the full pipeline into the interactive scene player. This closes the loop from player input through ML inference to rendered narrative prose.

### Step 6: Prediction Enrichment and Rendering

**`storyteller-engine/src/context/prediction.rs`** — two public functions and internal helpers:

- **`enrich_prediction(raw, character, scene, grammar)`** — Deterministic transform from `RawCharacterPrediction` → `CharacterPrediction`. Resolves axis indices to names from the character's tensor BTreeMap, generates templated action/speech/thought descriptions, detects internal conflict from opposing emotional deltas.

- **`render_predictions(predictions)`** — Formats assembled predictions as markdown for the Narrator's context window. Same structural pattern as `preamble::render_preamble()` — structured facts with emotional annotation, not prose.

Enrichment performs five steps, all deterministic Rust (no LLM calls):

1. **Axis resolution** — raw indices → axis names from the character tensor's BTreeMap key ordering
2. **Action description** — templated from ActionType + target name + ActionContext + emotional valence
3. **Speech direction** — templated from SpeechRegister + ActionContext + scene stakes
4. **Emotional subtext** — dominant emotion name + awareness level + character name
5. **Internal conflict detection** — opposing emotional deltas (one rising, one falling) → narrative tension

### Step 7: Pilot Validation and End-to-End Wiring

**`predict_character_behaviors(predictor, characters, scene, player_input, grammar)`** — orchestration function that runs the full pipeline:

1. Classify player input (naive keyword-based)
2. Build scene features from SceneData
3. Build PredictionInput for each character
4. Run batch ONNX inference
5. Enrich each result
6. Return `Vec<CharacterPrediction>`

**`classify_player_input(input, target_count)`** — naive keyword-based event classifier:
- "say"/"tell"/"speak"/"ask" → `EventType::Speech`
- "move"/"walk"/"approach" → `EventType::Movement`
- "look"/"watch"/"examine" → `EventType::Observation`
- "?" → `EventType::Inquiry`
- Default → `EventType::Interaction`
- Emotional register from charged words (anger words → Aggressive, sad words → Vulnerable, etc.)

**Narrator integration** — `build_turn_message()` in `narrator.rs` now renders ML predictions between retrieved context and "This Turn" when `original_predictions` is non-empty.

**Binary wiring** — `play_scene_context.rs` modifications:
- `--no-ml` flag to disable predictions
- `resolve_model_path()` checks `STORYTELLER_MODEL_PATH` then `STORYTELLER_DATA_PATH/models`
- Graceful fallback with clear messages when model unavailable
- Per-turn prediction generation with timing display
- `PredictionsEnriched` observer arm

## Files Modified/Created

### `storyteller-engine/src/context/prediction.rs` (NEW in Step 6, extended in Step 7)

New public API:
- `enrich_prediction()` — raw → assembled prediction
- `render_predictions()` — assembled predictions → markdown
- `estimate_predictions_tokens()` — token budget estimation
- `predict_character_behaviors()` — full orchestration pipeline

Internal helpers: `resolve_axis_names()`, `resolve_target_name()`, `generate_activation_reason()`, `generate_action_description()`, `generate_speech_direction()`, `resolve_primary_name()`, `generate_emotional_subtext()`, `detect_internal_conflict()`, `next_awareness_level()`, `classify_player_input()`, `build_scene_features()`

### `storyteller-engine/src/agents/narrator.rs` (MODIFIED)

Added prediction rendering block in `build_turn_message()` — when `resolver_output.original_predictions` is non-empty, renders them as markdown between Tier 3 (retrieved context) and "This Turn".

### `storyteller-cli/src/bin/play_scene_context.rs` (MODIFIED)

- Added `--no-ml` CLI flag
- Added `resolve_model_path()` helper
- Model loading at startup with graceful fallback
- Per-turn prediction generation replacing static `ResolverOutput`
- `PredictionsEnriched` arm in observer display

### `storyteller-core/src/traits/phase_observer.rs` (MODIFIED)

Added `PredictionsEnriched` variant to `PhaseEventDetail` with `character_count`, `total_actions`, `estimated_tokens`.

### `storyteller-engine/src/context/mod.rs` (MODIFIED)

Added `pub mod prediction;` to expose the new module.

## Design Decisions

### 1. Template-based enrichment, not LLM

All string generation in `enrich_prediction()` uses Rust format strings and match arms. This is deterministic, testable, sub-microsecond, and debuggable. The Narrator LLM does all creative synthesis — the enrichment only provides structured facts.

### 2. Internal conflict detection from opposing deltas

If two emotional deltas move in opposite directions (one positive, one negative), that signals an internal tension. This is a simple heuristic but produces meaningful Narrator input: "joy rising while sadness recedes" gives the Narrator something to render as subtext.

### 3. Naive event classifier is intentionally simple

The keyword-based classifier is a prototype placeholder. It correctly classifies obvious inputs ("I say hello" → Speech, "I walk to the fence" → Movement) and defaults to Interaction for ambiguous cases. Production will use an ML classifier. Good enough to prove the pipeline works.

### 4. Graceful model fallback in the binary

The binary runs with or without the ONNX model. Missing model → clear message + empty predictions. `--no-ml` flag for explicit opt-out. This means the interactive player always works, even without the ML pipeline configured.

### 5. Predictions injected via ResolverOutput, not a new pathway

Rather than adding a new parameter to `assemble_narrator_context()`, predictions flow through `ResolverOutput.original_predictions` — the field that already exists for this purpose. The Narrator reads them from the assembled context.

## Tests

### Unit tests (always run)

| Test | What it verifies |
|------|------------------|
| `enrich_with_workshop_data` | Full enrichment with Bramblehoof + Pyotir scene data |
| `axis_resolution_from_btreemap` | Indices map to correct BTreeMap keys |
| `emotion_index_resolves_to_primary_name` | Plutchik primary indices → names |
| `speech_absent_when_occurs_false` | No speech enrichment when model says silent |
| `internal_conflict_detected_from_opposing_deltas` | Opposing deltas → conflict string |
| `no_conflict_when_deltas_same_direction` | Same-direction deltas → no conflict |
| `render_predictions_has_all_sections` | Rendered markdown contains all expected sections |
| `render_empty_predictions_is_clean` | Empty input → empty string |
| `estimate_tokens_is_reasonable` | Token estimate in expected range |
| `awareness_shift_produces_next_level` | Awareness levels progress correctly |

### Feature-gated tests (`test-ml-model`)

| Test | What it verifies |
|------|------------------|
| `end_to_end_predict_enrich_render` | Real ONNX model → enrich → render → valid markdown with both characters |
| `enriched_predictions_have_valid_fields` | All enriched field values are structurally valid |

### End-to-end manual validation

```
$ STORYTELLER_DATA_PATH=... cargo run --package storyteller-cli --bin play-scene -- --model mistral
[ML model loaded: .../character_predictor.onnx]
[Turn 1: 2 predictions in 0.8ms]
[Turn 1: context assembly 0.5ms | ~1079 tokens]
[Turn 1: LLM 21.3s | total 21.3s | 1 LLM call]
```

The Narrator received structured character behavior and wove it into prose — trust, recognition, guarded silence, creative expression all rendered from ML predictions rather than pure improvisation.

## Verification Results

- `cargo check --all-features` — compiles
- `cargo clippy --all-targets --all-features` — no warnings
- `cargo fmt --check` — formatted
- `cargo test --workspace` — 123 tests pass (35 core + 55 engine + 33 ml)
- End-to-end: player input → classification → ONNX inference (0.8ms) → enrichment → Narrator context → LLM rendering

## Timing Profile (from live run)

| Phase | Time | Notes |
|-------|------|-------|
| ONNX model load | ~7ms | One-time at startup |
| ML prediction (2 characters) | 0.4-0.8ms | Sub-millisecond, as predicted |
| Context assembly | 0.5ms | Deterministic, no LLM |
| Narrator LLM (Mistral 7B) | 10-32s | Dominates total turn time |
| **Total turn** | **10-33s** | Within architecture target on local hardware |

The ML pipeline adds <1ms to each turn — three orders of magnitude below the LLM call that dominates latency. The architecture prediction of 2-8.5s total latency with a production LLM API remains achievable.
