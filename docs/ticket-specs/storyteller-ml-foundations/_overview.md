# Phase 0: Character Prediction ML Pipeline

## Context

The narrator-centric architecture validated in Phase 2 replaces LLM-based character agents with ML prediction models. Before building the full pipeline (Phase 3), we need to validate the approach: can an ML model trained on combinatorially-generated examples predict coherent character behavior from tensor features?

This plan covers:
1. Framework decision (Burn vs. PyTorch+ONNX)
2. A critical architectural decision about text generation
3. Training data generation pipeline
4. Model architecture
5. Workspace structure

---

## Decision 1: Framework — PyTorch for Training, ort for Inference

### The Honest Assessment

**Burn (v0.20.0)** has the layers we need (Linear, ReLU/GELU, dropout, embeddings, attention), real training infrastructure (Learner, Adam/AdamW, LR schedulers, DataLoader), and works on Apple Silicon via WGPU/Metal. The single-language appeal is genuine.

**But Burn is not ready for comfortable experimentation:**
- **No ONNX export** (open issue since Oct 2023, no timeline). Train-in-Burn means deploy-with-Burn, no escape hatch.
- **API instability** — breaking changes between minor versions (TensorData struct, scatter/slice APIs broke in 0.20).
- **No production users at scale**. Largest cited deployment: a 24MB plant disease classifier.
- **Limited community examples** beyond MNIST/CIFAR.
- **No operation fusion** for common activations — performance gap vs. PyTorch on GPU.
- **During the iterative model design phase** — where we figure out what architecture predicts character behavior well — PyTorch's ecosystem makes experimentation faster. Burn makes it expensive.

**The decision**: PyTorch for training, ONNX Runtime (`ort`) for inference.

- `ort` is already declared in the workspace (`2.0.0-rc.11`), unused but ready.
- Python toolchain already exists (`doc-tools/` with `uv`).
- The model architecture (multi-head MLP, see below) is the simplest possible ONNX export case.
- Training is a development-time activity, not runtime. The game engine stays pure Rust.
- When Burn matures (likely 2027+), we can reimplement training in Rust. The hybrid path doesn't preclude this.

### What Burn Could Still Do

If we reach a point where we want on-device fine-tuning (adapting to a specific player's patterns), Burn or ort's Trainer API could serve that need. But that's a future consideration, not Phase 0.

---

## Decision 2: The ML Model Produces ONLY Structured Data — No Text

This is the most important architectural decision in the analysis.

### The Problem

The current `CharacterPrediction` types have string fields:
- `ActionPrediction.description`: "Plays a melody from their shared past"
- `SpeechPrediction.content_direction`: "References the melody's origin"
- `ThoughtPrediction.emotional_subtext`: "Longing, defended by performance"
- `ThoughtPrediction.internal_conflict`: "Wants to be recognized but fears rejection"

### Why the ML Model Should NOT Generate These

Text generation requires a sequence-to-sequence architecture (~125M+ parameters minimum). At 2K-7.5K training examples, this dramatically overfits. It also reintroduces the exact problem the architectural pivot solved: "using an LLM as an expensive lookup table." Inference jumps from <1ms to 50-500ms per character (autoregressive decoding).

### The Answer: Context Assembly Renders Structured Narrative Briefings

The ML model outputs **only structured values**: `ActionType` enum, confidence float, `SpeechRegister` enum, `AwarenessLevel` enum, emotional delta floats. The **context assembly system** (Storykeeper) takes these structured predictions plus the character's tensor profile plus relational graph context and **renders structured narrative briefings** — readable facts with emotional annotation, not prose. The Narrator does ALL creative synthesis.

This is the same pattern the context assembly already uses for Tier 3 retrieved context:
```
- **Bramblehoof — backstory**: A satyr bard who gave a boy a flute years ago _Hope mixed with guilt_
```

For ML predictions, the assembly renders a structured briefing by enriching raw predictions with graph/tensor lookups:

```
### Bramblehoof (confidence: 0.85)
- Action: Performs [creative expression — relevant axes: creative_expression(0.9),
  empathy(0.8); afforded by: flute in scene]
- Target: Pyotir (history: high, trust: moderate, affection: defended)
- Emotional posture: Sadness (Defended, 0.6), Joy (Preconscious, 0.3)
- Subtext: High empathy toward target + defended grief = acts from feelings he cannot name
- Speech: None this turn
- Internal: Awareness=Defended, dominant=Sadness, tension=high
```

The Narrator reads this briefing and synthesizes: "Bramblehoof plays a melody from their shared past — testing whether Pyotir remembers." The assembly doesn't compose that prose. It provides the facts the Narrator needs to compose it.

#### How the Enrichment Works

The assembly function `render_prediction(raw_prediction, character_sheet, edges, scene)` performs five deterministic steps:

1. **Resolve entities** — target entity ID → "Pyotir" from the scene cast list
2. **Look up relevant tensor axes** — for `ActionType::Perform`, find axes tagged as creative/physical/social in the character tensor. Bramblehoof's highest: `creative_expression: 0.9`
3. **Look up scene affordances** — what objects/conditions in the rendered space are relevant to this action type? The flute is present in `SceneData.setting`
4. **Describe relational context** — from the `DirectedEdge` substrate dimensions between these two characters: "history: high, trust: moderate, affection: defended"
5. **Render emotional posture** — combine the ML's emotional deltas with the character's current `EmotionalState` and awareness levels, using the emotional grammar (Plutchik) to name the primaries

This is deterministic, testable, debuggable Rust code — a `render_prediction()` function in the context module alongside the existing `render_preamble()`, `render_journal()`, and `render_retrieved()`. Each layer of the system does exactly one job:

- **ML model**: predicts structured behavior from tensor features (enums + floats)
- **Context assembly**: enriches predictions with graph/tensor facts into readable briefings
- **Narrator LLM**: transforms structured briefings into literary prose

The same briefing pattern works for knowledge graph information, world system bounds, and any other structured data source — the assembly renders facts, the Narrator renders prose. No layer tries to do another layer's job.

This keeps text generation in exactly one place (the Narrator LLM), keeps the ML model purely structured (MLP works), and produces richer briefings than any ML model could (because the assembly draws on the full graph context the ML model never sees).

### Type Changes Required

The prediction types need revision to separate ML-output fields from assembly-generated fields. The string fields stay on the types (the Resolver and Narrator still consume them), but they are populated by the context assembly system, not by the ML model. We'll add a `RawPrediction` type for what the ML model actually outputs, and the existing `CharacterPrediction` becomes the assembled version.

---

## Model Architecture: Multi-Head MLP

### Input (~400-500 features, fixed-size tabular)

| Source | Features | Notes |
|--------|----------|-------|
| Character tensor (activated subset, ~12 axes) | ~156 | 4 floats + 2 one-hot categoricals per axis |
| Emotional state (8 Plutchik primaries) | ~48 | intensity float + awareness one-hot per primary |
| Self-edge | ~7 | trust (3), affection, debt, history_weight, projection_accuracy |
| Relational edges (padded to max 5) | ~120 | 5 substrate dimensions × 4 floats + topology one-hot per edge |
| Scene context | ~15 | scene type one-hot, temperature, phase, affordance flags |
| Classified player input | ~20 | event type one-hot, register one-hot, target flags, confidence |
| Recent intent history (3 turns) | ~60 | action type + register + awareness per prior turn |

### Output (~25-35 structured values)

| Output | Encoding |
|--------|----------|
| `ActionType` (6 classes) | Softmax |
| Action confidence | Sigmoid |
| Action target (cast member flags) | Sigmoid per cast member |
| Whether speech occurs | Sigmoid |
| `SpeechRegister` (4 classes) | Softmax |
| Speech confidence | Sigmoid |
| `AwarenessLevel` of thought (5 classes) | Softmax |
| Dominant emotion for thought (8 classes) | Softmax |
| Emotional deltas per primary (8 values) | Tanh |
| Awareness shifts per primary (8 × binary) | Sigmoid |

### Architecture

```
Input (400-500 features)
    │
    ▼
Shared Trunk: Linear(input, 384) → ReLU → Dropout(0.3)
    → Linear(384, 256) → ReLU → Dropout(0.3)
    → Linear(256, 256) → ReLU
    │
    ├─→ Action Head: Linear(256, 64) → ReLU → Linear(64, action_outputs)
    ├─→ Speech Head: Linear(256, 64) → ReLU → Linear(64, speech_outputs)
    ├─→ Thought Head: Linear(256, 64) → ReLU → Linear(64, thought_outputs)
    └─→ Emotion Head: Linear(256, 64) → ReLU → Linear(64, emotion_outputs)
```

~500K-1M parameters. Trains well on 2K-7.5K examples with dropout + early stopping. Inference <1ms on CPU via ort. The 50-150ms budget is three orders of magnitude above need.

---

## Workspace Structure

### New: `storyteller-ml/` (Rust crate)

Depends on `storyteller-core`. Owns training data generation and feature encoding — the parts that need Rust types.

```
storyteller-ml/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── feature_schema.rs    # Canonical feature encoding (shared contract)
│   ├── matrix/
│   │   ├── mod.rs
│   │   ├── archetypes.rs    # Character archetype templates → tensor profiles
│   │   ├── dynamics.rs      # Relational dynamic templates → edge configs
│   │   └── profiles.rs      # Scene profile templates → constraint/affordance sets
│   ├── generation.rs        # Combinatorial matrix → LLM-generated intents
│   ├── validation.rs        # Programmatic coherence checks
│   └── export.rs            # Serialize validated examples as JSONL
└── src/bin/
    ├── generate_training_data.rs  # CLI: matrix → Ollama → JSONL
    └── validate_dataset.rs        # CLI: coherence check existing JSONL
```

### New: `training/` (Python package)

Consumes JSONL from `storyteller-ml`, trains model, exports ONNX.

```
training/
├── pyproject.toml           # uv-managed, torch + numpy + onnx
├── src/training/
│   ├── model.py             # CharacterPredictor (multi-head MLP)
│   ├── dataset.py           # JSONL dataset loader
│   ├── train.py             # Training loop, validation, metrics
│   ├── export.py            # torch.onnx.export → character_predictor.onnx
│   └── feature_schema.py    # Generated from or synced with Rust feature_schema
└── tests/
    ├── test_model.py         # Forward pass, output shapes
    ├── test_dataset.py       # Loading, batching
    └── test_roundtrip.py     # Same input → same output in Python and Rust
```

### Modified: `storyteller-engine/src/inference/frame.rs`

Currently a stub. Will implement ort-based inference:
- Load `.onnx` session at startup
- `predict()`: encode `CharacterSheet` + context → feature vector → ort → decode outputs
- The `encode_features()` function uses `storyteller-ml::feature_schema` for canonical encoding

### Dependency Graph

```
storyteller-ml ──→ storyteller-core
(training data gen)    (types)

training/ (Python)
  reads: JSONL from storyteller-ml
  produces: .onnx model file

storyteller-engine ──→ storyteller-core
  ort loads .onnx     storyteller-ml::feature_schema
```

---

## Implementation Plan

### Step 1: Feature Schema (Rust — `storyteller-ml`)

Create the `storyteller-ml` crate. Implement `feature_schema.rs`:
- Define canonical axis ordering for character tensors
- Define one-hot encoding for all enums (TemporalLayer, Provenance, AwarenessLevel, ActionType, SpeechRegister, EventType, EmotionalRegister, SceneType, TopologicalRole)
- Define relational edge flattening (max edges, padding)
- Define the complete feature vector structure with named dimensions
- Export the schema as JSON (for Python to consume)
- Implement `encode_features()`: `CharacterSheet` + context → `Vec<f32>`
- Implement `decode_outputs()`: `Vec<f32>` → structured prediction values
- **Tests**: round-trip encode/decode with workshop Bramblehoof + Pyotir data

**Key files**: `storyteller-ml/src/feature_schema.rs`, `storyteller-ml/src/lib.rs`, `storyteller-ml/Cargo.toml`

### Step 2: Archetype Templates (Rust — `storyteller-ml`)

Implement the three axes of the combinatorial matrix:
- `archetypes.rs`: 10-15 character archetype templates, each generating a `CharacterTensor` with stochastic variation
- `dynamics.rs`: 8-10 relational dynamic templates, each generating `RelationalSubstrate` configurations
- `profiles.rs`: 8-10 scene profile templates, each generating scene context with constraints and affordances
- `generation.rs`: the matrix combinator — iterate archetypes × dynamics × profiles, apply stochastic variation

**Key files**: `storyteller-ml/src/matrix/`

### Step 3: Training Data Generation (Rust — `storyteller-ml`)

Build the `generate-training-data` binary:
- For each matrix cell: generate tensor, emotional state, relational edges, scene context
- Encode as feature vector (using Step 1 schema)
- Call LLM (Ollama via `ExternalServerProvider`) with the feature context to generate a structured `CharacterPrediction`
- Run programmatic coherence validation (emotional consistency, relational alignment, awareness discipline)
- Score and filter; write validated examples as JSONL
- Target: 50-100 examples for pilot validation (Phase 0), 2K-7.5K for first real model

**Key files**: `storyteller-ml/src/generation.rs`, `storyteller-ml/src/validation.rs`, `storyteller-ml/src/export.rs`, `storyteller-ml/src/bin/generate_training_data.rs`

### Step 4: Model Training (Python — `training/`)

Set up the Python training package:
- `model.py`: Multi-head MLP as described above
- `dataset.py`: JSONL loader that converts to tensors
- `train.py`: Training loop with Adam, dropout, early stopping, train/val split
- `export.py`: `torch.onnx.export` with named inputs/outputs
- Shared test fixture: known feature vector → known output, validated in both Python and Rust

**Key files**: `training/src/training/`, `training/pyproject.toml`

### Step 5: ort Inference in Engine

Implement `storyteller-engine/src/inference/frame.rs`:
- Load `.onnx` session at engine startup
- `FrameComputer::predict()`: uses `storyteller-ml::feature_schema::encode_features()` → ort session → `decode_outputs()`
- Integration with the existing context assembly pipeline
- Test: mock model (known weights → known outputs) verifies the encode/inference/decode path

**Key files**: `storyteller-engine/src/inference/frame.rs`

### Step 6: Prediction Type Refactoring

Split prediction types into raw ML output and assembled predictions:
- `RawCharacterPrediction`: only structured values (no strings) — what the ML model outputs
- `CharacterPrediction` (existing): the assembled version with descriptive annotations — populated by context assembly from raw predictions + graph context
- Update context assembly to generate the natural-language annotations

**Key files**: `storyteller-core/src/types/prediction.rs`, `storyteller-engine/src/context/`

### Step 7: Pilot Validation (the Phase 0 gate)

Generate 50-100 training examples. Train a trivial model. Evaluate:
- Do predictions align with tensor profiles? (high-empathy characters should prefer Speak/Examine over Resist)
- Are emotional deltas consistent with relational substrates?
- Does awareness discipline hold? (Defended emotions don't surface as Articulate thoughts)
- Does the context assembly system produce rich annotations from structured predictions?
- Compare: ML-predicted → assembled → narrated vs. the current prototype (no predictions, Narrator infers everything)

**Decision gate**: If predictions are coherent, proceed to Phase 3 (full-scale training). If not, iterate on the architecture or feature engineering before scaling.

---

## Verification

1. **Feature schema**: Round-trip test — encode Bramblehoof data → decode → values match
2. **Training data**: Generate 50 examples, validate coherence scores > threshold
3. **Model training**: Training loss converges, validation loss doesn't diverge
4. **ONNX export**: Python inference == ort inference for same inputs (within float tolerance)
5. **End-to-end**: Workshop scene runs with ML predictions in the context assembly → Narrator pipeline
6. **All existing 73 tests pass** at every step
7. `cargo clippy --workspace --all-targets --all-features` clean
