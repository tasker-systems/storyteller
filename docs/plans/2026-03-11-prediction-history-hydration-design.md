# Prediction History Hydration (Region 7)

## Status: Design

## Context

The ML character prediction model accepts 453 input features organized into 7 regions. Region 7 (recent history, 48 features) and Region 4 (relational edges, 120 features) are currently passed as zeros — 168 of 453 features carry no signal. This explains why predictions are static between turns: only 16 event classification features (Region 6) vary, and when the player's actions classify similarly across turns, predictions are identical.

This spec addresses **Region 7 only**. Region 4 (relational edges) requires separate design work on how turn-by-turn interactions become relational shifts — that is a follow-up spec.

### Relationship to Region 4

Regions 4 and 7 together represent the dynamic, situational portion of the prediction input. Region 7 captures "what has happened recently in this scene" (temporal context). Region 4 captures "how these characters relate to each other" (relational context). Both must be hydrated for the model to produce meaningfully dynamic predictions — predictions that respond to the evolving scene rather than reflecting only static character identity.

The feature balance is an intentional design opinion: 263 features encode who the character *is* (tensor, emotions, self-edge), while 168 features encode the situation they're *in* (edges, scene, event, history). Character identity outweighs situation — a character's intent toward action is more heavily influenced by who they are than by what's happening — but the situational features give shape and direction to that intent.

### Relationship to Emergent Agency

The emergent NPC resistance documented in `docs/emergent/agency-from-data.md` currently works through static character data flowing through the intent synthesis pipeline. Hydrating history features should make this behavior more dynamic — an NPC who has been resisting for several turns may show different prediction patterns than one encountering a new situation. Whether this reinforces or modulates the emergent resistance is an empirical question that the integration tests below will help answer.

## Architecture

### Data Flow

```
Turn N predictions complete
    ↓
Caller extracts HistoryEntry per character from CharacterPrediction
    ↓
Push to PredictionHistory ring buffer (depth 3, per-character, most-recent-first)
    ↓
Turn N+1 prediction call
    ↓
predict_character_behaviors() receives history parameter
    ↓
PredictionInput.history populated with character's entries
    ↓
encode_features() writes non-zero values at indices 405-452
    ↓
ONNX model receives history signal → predictions vary turn-over-turn
```

### HistoryEntry Construction

Each `HistoryEntry` is derived from a `CharacterPrediction`:

| HistoryEntry field | Source in CharacterPrediction |
|---|---|
| `action_type` | `actions[0].action_type` (highest confidence action) |
| `speech_register` | `speech.register` if speech predicted, else `Conversational` default |
| `awareness_level` | `thought.awareness_level` |
| `emotional_valence` | Sum of positive `emotional_deltas.intensity_change` minus sum of negative, clamped to `[-1.0, 1.0]` |

### PredictionHistory Type

A `HashMap<EntityId, VecDeque<HistoryEntry>>` with max depth 3 per character. Owned by the caller (workshop `EngineState` or Bevy resource), not by the ML layer.

### predict_character_behaviors() Signature Change

The function gains a `history: &HashMap<EntityId, VecDeque<HistoryEntry>>` parameter. Inside its per-character `.map()` closure, it looks up the character's history by `EntityId` and converts to `&[HistoryEntry]` for `PredictionInput.history`. Characters with no history entry get `&[]` (empty slice, same as current behavior). This keeps the call site clean — callers pass the whole map, the function handles per-character lookup internally.

### Integration Points

**Workshop (`commands.rs`):** After predictions complete, extract `HistoryEntry` per character and push to `EngineState.prediction_history`. Pass to next turn's `predict_character_behaviors()` call.

**Bevy (`turn_cycle.rs`):** Pass empty history for now. Bevy hydration follows the same interface but is out of scope — the Bevy systems don't yet maintain turn-over-turn state in the way the workshop does.

## Future Architecture: Speculative History Reconciliation

The prediction-derived history (this spec) feeds the model's own outputs back as inputs. This is self-consistent but circular — the model never learns what actually happened in the rendered prose, only what it predicted would happen.

### The Architecture

1. Narrator renders prose and emits it to the player
2. Simultaneously, the system sends both the intent directives and the rendered prose to a 3b-instruct model via an async channel
3. The 3b model produces a **provisional history** — structured `HistoryEntry` data reflecting what was actually rendered, not just what was predicted
4. This provisional history sits in a hypothesized state
5. When the player submits their next input (implicitly affirming the turn), the provisional history is promoted to committed and immediately available for the next prediction cycle
6. No additional latency on the critical path — the reconciliation runs during player think time

### Why This Matters

The model would learn from what the narrator chose to render, not just from its own echo. If the narrator softened a character's resistance or amplified it, the history would reflect that. The feedback loop becomes grounded in the actual narrative, not just in predictions.

### Prior Art to Consult

Before implementing this future architecture, review:
- `ProvisionalStatus` enum in `storyteller-core` (Hypothesized → Rendered → Committed lifecycle)
- `docs/technical/event-system.md` — event provenance and commitment model
- `docs/technical/narrator-architecture.md` § Command Sourcing — the append-only event ledger and checkpoint/replay model
- The existing provisional/committed patterns in the turn cycle pipeline

### Not Implemented in This Spec

This is captured as a named future direction. The prediction-derived history (approach A) is the immediate implementation.

## Files

| File | Action | What |
|---|---|---|
| `crates/storyteller-ml/src/prediction_history.rs` | Create | `PredictionHistory` struct, `HistoryEntry` construction from `CharacterPrediction`, emotional valence computation, ring buffer logic |
| `crates/storyteller-ml/src/lib.rs` | Modify | Add `pub mod prediction_history` |
| `crates/storyteller-engine/src/context/prediction.rs` | Modify | Add `history` parameter to `predict_character_behaviors()`, pass through to `PredictionInput` |
| `crates/storyteller-workshop/src-tauri/src/engine_state.rs` | Modify | Add `prediction_history: PredictionHistory` field |
| `crates/storyteller-workshop/src-tauri/src/commands.rs` | Modify | Extract history entries after predictions, push to ring buffer, pass to next turn |
| `crates/storyteller-engine/src/systems/turn_cycle.rs` | Modify | Pass empty history (maintains current behavior) |

## Testing

**Unit tests (in `prediction_history.rs`):**
- `HistoryEntry` construction from `CharacterPrediction` — correct field extraction
- `emotional_valence` computation — positive deltas, negative deltas, mixed, empty
- Ring buffer depth limit (3) and most-recent-first ordering
- Empty history for unknown character ID returns empty slice

**Unit tests (in `feature_schema.rs`):**
- `encode_features()` with non-empty history — verify features at indices 405-452 are non-zero
- Verify encoding matches the expected one-hot positions for known action types / registers

**Integration tests (feature-gated `test-ml-model`):**
- `predict_character_behaviors()` with populated history vs empty history — verify the encoding pipeline is wired correctly (non-zero features at indices 405-452 reach the model)
- Two consecutive simulated turns with the same event input but accumulated history — verify the pipeline assembles and encodes correctly end-to-end

**Note on model sensitivity:** The current ONNX model was trained with Region 7 features at zero. These integration tests validate that the *encoding pipeline* is wired correctly, not that the model produces meaningfully different outputs. The model may need retraining with non-zero history signal before the predictions visibly change — but that retraining is out of scope for this spec.

## Scope Boundaries

- No experiment harness binary or storyteller-data output
- No Region 4 (relational edges) hydration
- No speculative history reconciliation implementation
- No Bevy system changes beyond passing empty history
- No changes to the ONNX model itself or training pipeline
