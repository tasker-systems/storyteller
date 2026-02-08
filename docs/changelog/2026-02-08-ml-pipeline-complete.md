# Phase 0 ML Pipeline Complete — Character Prediction End-to-End

**Date**: 2026-02-08
**Phase**: Phase 0 — ML Pipeline Validation
**Branch**: `jcoletaylor/agent-prototyping`

## What Happened

We built the complete character prediction ML pipeline in a single branch, from architectural redesign through to a working interactive scene where an ML model predicts character behavior and a single LLM Narrator renders it as prose. The branch validates the core bet of the narrator-centric architecture: that ML prediction + deterministic enrichment + single LLM call can replace multiple LLM agent calls while being faster, cheaper, and more controllable.

The pipeline runs end-to-end: player types "I approach the fence slowly" → naive keyword classifier identifies Movement event → ONNX model predicts both characters' actions, speech, thought, and emotional shifts in 0.8ms → enrichment system resolves indices to names and generates structured briefings → Narrator LLM receives character behavior as structured fact and renders literary prose.

## The Branch: 8 Commits, ~12,700 Lines

| Commit | What |
|--------|------|
| `fb0807e` | Phase 1 core type restructuring — prediction, resolver, world model, narrator context types |
| `763361a` | Three-tier context assembly, architecture validation (parity test), legacy agent removal |
| `9609381` | storyteller-ml crate — feature schema (453→42), combinatorial training data pipeline (15K examples) |
| `896832e` | Replace symlink with `STORYTELLER_DATA_PATH` env var |
| `b099bdd` | Python training package — 407K-param multi-head MLP, cell-stratified splits, ONNX export |
| `e40bf49` | Full training run — val_loss 1.68, awareness 90% accuracy, 38KB ONNX model |
| `abda4b1` | Feature-gated test tiers (`test-ml-model`, `test-llm`), model fixtures for CI |
| `62fb8e2` | Formatting cleanup |

Plus the final session (uncommitted at time of writing): prediction enrichment, Narrator wiring, interactive scene player integration.

## What Was Built

### storyteller-core (foundation types)

| Module | What |
|--------|------|
| `types/prediction.rs` | Two-tier prediction types: `Raw*` (ML output, enums + floats only) and assembled (enriched with narrative descriptions) |
| `types/resolver.rs` | `ResolverOutput`, `SuccessDegree`, `ActionOutcome`, `ConflictResolution` — deterministic resolution types |
| `types/world_model.rs` | `NarrativeDistanceZone`, `WorldModel`, `CapabilityProfile` — world constraint types |
| `types/narrator_context.rs` | `NarratorContextInput`, `PersistentPreamble`, `SceneJournal`, `RetrievedContext` — three-tier context |
| `traits/phase_observer.rs` | `PhaseObserver` trait with 10+ event variants for pipeline observability |

### storyteller-ml (ML pipeline crate — NEW)

| Module | What |
|--------|------|
| `feature_schema.rs` | 453-feature input / 42-feature output encoding — the Rust↔Python contract |
| `matrix/archetypes.rs` | 12 character archetype templates with stochastic variation |
| `matrix/dynamics.rs` | 10 relational dynamic templates generating edge configurations |
| `matrix/profiles.rs` | 10 scene profile templates with affordances and constraints |
| `matrix/combinator.rs` | Combinatorial matrix iterator with exclusion rules |
| `matrix/labels.rs` | Heuristic label generation from archetype × dynamic × profile |
| `matrix/validation.rs` | 5-rule coherence validation (action, speech, awareness, emotion, frame consistency) |
| `matrix/export.rs` | JSONL export with content-hash deduplication and manifests |
| `bin/generate_training_data` | CLI: matrix → labeled examples → validated JSONL |
| `bin/validate_dataset` | CLI: coherence check existing datasets |

### training/ (Python package — NEW)

| Module | What |
|--------|------|
| `model.py` | `CharacterPredictor` — 407K-param multi-head MLP (shared trunk → 4 heads) |
| `dataset.py` | JSONL dataset loader with cell-stratified train/val split |
| `losses.py` | Per-head loss decomposition: CrossEntropy + BCE + MSE, weighted |
| `train.py` | Training loop with Adam, early stopping, per-head metrics |
| `export.py` | `torch.onnx.export` with named I/O and round-trip validation |
| `cli.py` | `train-character-model` CLI with full hyperparameter control |

### storyteller-engine (inference + context)

| Module | What |
|--------|------|
| `inference/frame.rs` | `CharacterPredictor` — ONNX inference via `ort`, dedicated rayon pool, batch prediction |
| `context/prediction.rs` | `enrich_prediction()`, `render_predictions()`, `predict_character_behaviors()` |
| `context/preamble.rs` | Tier 1: narrator identity, scene, cast, boundaries |
| `context/journal.rs` | Tier 2: progressive compression by recency/weight/redundancy |
| `context/retrieval.rs` | Tier 3: information boundary enforcement, relevance ranking |
| `agents/narrator.rs` | Modified: renders ML predictions in turn message |

### storyteller-cli (interactive player)

| Module | What |
|--------|------|
| `bin/play_scene_context.rs` | Modified: `--no-ml` flag, model loading, per-turn prediction, graceful fallback |

## Pipeline Architecture

```
Player: "I approach the fence slowly"
    │
    ▼
classify_player_input()          →  EventFeatureInput { Movement, Guarded, 0.8 }
    │
    ▼
build_scene_features()           →  SceneFeatureInput { Gravitational, cast=2, tension=0.5 }
    │
    ▼
For each character:
  encode_features()              →  Vec<f32> [453]
  ort::Session::run()            →  4 tensors → Vec<f32> [42]
  decode_outputs()               →  RawCharacterPrediction
  enrich_prediction()            →  CharacterPrediction (with narrative descriptions)
    │
    ▼
render_predictions()             →  Markdown briefing for Narrator
    │
    ▼
assemble_narrator_context()      →  Three-tier context (preamble + journal + predictions + retrieved)
    │
    ▼
Narrator LLM (single call)      →  Literary prose
```

## Timing Profile

| Phase | Time | % of Turn |
|-------|------|-----------|
| Event classification | <0.01ms | ~0% |
| ML prediction (2 chars) | 0.4-0.8ms | ~0% |
| Context assembly | 0.5ms | ~0% |
| Narrator LLM (Mistral 7B, local) | 10-32s | ~100% |
| **Total turn** | **10-33s** | |

The entire ML + enrichment + assembly pipeline adds <2ms to each turn. The LLM call dominates completely. With a cloud API (Sonnet/Haiku at 1-3s), total turn time would be 1-4s — well within the 2-8.5s architecture target.

## Test Counts

| Crate | Tests | Notes |
|-------|-------|-------|
| storyteller-core | 35 | Prediction types, resolver types, grammar validation, observer |
| storyteller-engine | 55 | Context assembly (20), narrator (5), prediction enrichment (10), inference (3), workshop (18) |
| storyteller-ml | 33 | Feature schema (14), matrix pipeline (19) |
| **Total** | **123** | All passing, clippy clean |

Plus 31 Python tests (model, dataset, export, losses, feature schema).

## Training Results

- **Data**: 15,000 examples from 5,000 combinatorial cells × 3 stochastic variations
- **Architecture**: 407K-param multi-head MLP (453 → 384 → 256 → 256 → 4 heads)
- **Training**: 100 epochs, Adam, dropout 0.3, cell-stratified 80/20 split
- **Validation loss**: 1.68 (converged)
- **Awareness accuracy**: 90% (vs. 20% random baseline)
- **Action type accuracy**: 32% (vs. 17% random baseline — 6 classes, limited by heuristic labels)
- **Model size**: 38KB ONNX graph + 1.6MB weights
- **Inference**: <1ms per character on CPU

The action type accuracy is modest but expected — the heuristic label generator produces reasonable but not sharp labels. Real improvement requires LLM-generated labels (Phase 3) or human-reviewed training data. The model demonstrates that the architecture works; the training data quality is the bottleneck.

## Key Observations

### What worked

1. **The architectural bet is validated.** A single LLM call with rich structured context produces output at parity with three LLM calls from the legacy multi-agent approach, at 38% less wall time.

2. **ML prediction is effectively free.** Sub-millisecond inference means we can predict every character every turn without meaningful cost. The budget concern is LLM tokens, not ML compute.

3. **Template enrichment is surprisingly effective.** Simple templates like "Acts — driven by shared history, with warmth" give the Narrator enough to work with. The Narrator's creative synthesis fills in what the templates leave out.

4. **The three-tier context system works.** Preamble (identity) + journal (compressed history) + retrieved (backstory) + predictions (character behavior) gives the Narrator everything it needs in ~1000-1500 tokens.

5. **Graceful degradation works.** The system runs with or without the ML model, with or without a running Ollama instance. Each component fails independently with clear messages.

### What's rough

1. **Narrator verbosity.** Mistral 7B ignores word limits (300 words requested, 400-800 produced). This is a model instruction-following issue, not an architecture issue.

2. **Template leakage.** Enrichment templates occasionally show through in the Narrator output ("in the context of For Bramblehoof: whether he can see past his own narrative"). The templates need refinement, or the Narrator prompt needs explicit instruction to paraphrase structured input.

3. **Heuristic training labels.** The combinatorial matrix generates structurally valid but shallow labels. Action type accuracy at 32% reflects this ceiling. LLM-generated labels (with coherence validation) are the next improvement.

4. **No relational data wired.** The prediction pipeline passes empty edge arrays and empty topology roles. Relational edges are defined in the type system but not yet populated from scene data.

## What Comes Next

This branch is ready to merge. Phase 0 is complete — the character prediction ML pipeline works end-to-end.

Likely next work:

1. **Coherence validation pass** — programmatic checks that ML predictions are consistent with character tensors (high-empathy characters should prefer Speak/Examine over Resist)
2. **LLM-generated training labels** — replace heuristic labels with Ollama-generated intents, validated by the coherence pipeline
3. **Relational data wiring** — populate edge arrays from workshop scene data so predictions account for character relationships
4. **Resolver rules engine** — deterministic action resolution with narrative distance zones and graduated success
5. **Cloud LLM provider** — Anthropic API support for quality/speed comparison
6. **Prompt engineering** — iterate on narrator system prompt to address verbosity and template leakage
