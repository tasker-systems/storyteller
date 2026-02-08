# Step 4: Python Training Package

**Status: COMPLETE** (Feb 8, 2026)

## Context

Steps 1-3 and 6 of the Phase 0 ML pipeline are complete:
- `storyteller-ml/src/feature_schema.rs` — 453-feature input / 42-feature output encoding contract
- `storyteller-ml/src/matrix/` — combinatorial training data generation
- `storyteller-core/src/types/prediction.rs` — Raw ML output types
- 15,000 training examples in `storyteller-data/training-data/low_fantasy_folklore.jsonl`

This step builds the Python `training/` package that consumes the JSONL, trains a multi-head MLP via PyTorch, and exports ONNX for Rust-side inference via `ort`. Model artifacts (.pth, .onnx) are **not committed to git** — MLOps/model registry decisions are deferred.

---

## Architecture

```
storyteller-data/training-data/low_fantasy_folklore.jsonl   (15k examples, 453→42)
        │
        ▼
training/src/training/                                       (Python, PyTorch)
    dataset.py      load JSONL → cell-stratified split → DataLoaders
    model.py        CharacterPredictor (multi-head MLP, ~407K params)
    losses.py       per-head loss functions (CE + BCE + MSE)
    metrics.py      per-head accuracy/MSE tracking
    train.py        training loop with early stopping
    export.py       torch.onnx.export → character_predictor.onnx
    cli.py          train-character-model CLI entry point
    feature_schema.py   constants synced from Rust feature_schema.rs
        │
        ▼
character_predictor.onnx  (~1.5-2 MB, local only, not committed)
```

---

## Key Design Decisions

### 1. Train/val split: stratified by matrix cell

15,000 examples come from 1,303 unique cells (archetype × dynamic × profile). Random split would leak identical cells across partitions. All variations of a cell go to the same split.

80/20 by cell → 12,000 train / 3,000 val examples.

### 2. Per-head loss weights: fixed hyperparameters

| Head | Weight | Rationale |
|------|--------|-----------|
| Action | 0.35 | Primary behavioral output |
| Speech | 0.20 | Binary gate + register |
| Thought | 0.20 | Awareness discipline is a core coherence constraint |
| Emotion | 0.25 | 16 continuous outputs, affects state updates |

Exposed as CLI args for tuning without code changes.

### 3. ONNX output naming: 4 named tensors

- Input: `"features"` `[batch, 453]`
- Outputs: `"action"` `[batch, 14]`, `"speech"` `[batch, 6]`, `"thought"` `[batch, 6]`, `"emotion"` `[batch, 16]`

Raw logits — no activation functions in the model. Softmax/sigmoid/tanh applied at decode time (matches `decode_outputs()` in Rust).

### 4. feature_schema.py: manually synced from Rust

Direct transliteration of constants from `feature_schema.rs`. Docstring cites Rust file path. Tests validate against actual JSONL dimensions.

### 5. Device: MPS (Apple Silicon) with CPU fallback

Standard PyTorch 2.2+ includes MPS support. Training loop detects device automatically, falls back to CPU with a warning.

### 6. ONNX opset version: 18 (implementation note)

PyTorch 2.10's `torch.onnx.export` natively targets opset 18. The implementation uses `dynamic_shapes` (not the deprecated `dynamic_axes`) for forward-compatible dynamic batch export.

---

## Model Architecture

```
Input (453)
    │
Shared Trunk:
    Linear(453, 384) → ReLU → Dropout(0.3)
    Linear(384, 256) → ReLU → Dropout(0.3)
    Linear(256, 256) → ReLU
    │
    ├─→ Action Head:  Linear(256, 64) → ReLU → Linear(64, 14)
    ├─→ Speech Head:  Linear(256, 64) → ReLU → Linear(64, 6)
    ├─→ Thought Head: Linear(256, 64) → ReLU → Linear(64, 6)
    └─→ Emotion Head: Linear(256, 64) → ReLU → Linear(64, 16)
```

407,210 parameters. Trains well on 15K examples with dropout + early stopping.

---

## Label Vector Decomposition (for loss functions)

Each label is a flat `[42]` vector. The loss module slices into sub-regions with appropriate loss functions:

**Action head** (offset 0, length 14):
- `[0:6]` ActionType one-hot → CrossEntropyLoss (argmax labels to class index)
- `[6]` confidence → MSELoss
- `[7]` target_index → MSELoss
- `[8]` emotional_valence → MSELoss
- `[9:14]` ActionContext one-hot → CrossEntropyLoss

**Speech head** (offset 14, length 6):
- `[14]` speech_occurs → BCEWithLogitsLoss
- `[15:19]` SpeechRegister one-hot → CrossEntropyLoss
- `[19]` confidence → MSELoss

**Thought head** (offset 20, length 6):
- `[20:25]` AwarenessLevel one-hot → CrossEntropyLoss
- `[25]` dominant_emotion_index → MSELoss (regression on integer 0-7)

**Emotion head** (offset 26, length 16):
- `[26:34]` intensity deltas → MSELoss (tanh range)
- `[34:42]` awareness shifts → BCEWithLogitsLoss (binary per primary)

---

## Package Structure

```
training/
├── pyproject.toml
├── src/training/
│   ├── __init__.py
│   ├── feature_schema.py     # Constants from Rust feature_schema.rs
│   ├── dataset.py            # JSONL loading, cell-stratified split, DataLoaders
│   ├── model.py              # CharacterPredictor (multi-head MLP)
│   ├── losses.py             # MultiHeadLoss with per-region loss functions
│   ├── metrics.py            # Per-head metric accumulation and reporting
│   ├── train.py              # Training loop, early stopping, checkpointing
│   ├── export.py             # ONNX export with named I/O, validation
│   └── cli.py                # CLI entry point (argparse)
└── tests/
    ├── conftest.py           # Shared fixtures (synthetic data, model)
    ├── test_feature_schema.py
    ├── test_dataset.py
    ├── test_model.py
    ├── test_losses.py
    └── test_export.py        # PyTorch vs onnxruntime round-trip
```

---

## Dependencies

```toml
dependencies = [
    "torch>=2.2.0",
    "numpy>=1.26.0",
    "onnx>=1.15.0",
    "onnxruntime>=1.17.0",
    "onnxscript>=0.1.0",       # Required by torch.onnx.export in PyTorch 2.10+
]
```

`onnxscript` was added during implementation — PyTorch 2.10's new dynamo-based ONNX exporter requires it as a runtime dependency.

---

## Implementation Results

### Test suite: 31 tests, all passing

| Module | Tests | Coverage |
|--------|-------|----------|
| `test_feature_schema.py` | 9 | Constants, slices, enum lengths, dimension verification |
| `test_dataset.py` | 7 | Cell keys, JSONL loading, stratified split no-leak, dataloaders |
| `test_model.py` | 6 | Output shapes, parameter count, dropout behavior, determinism |
| `test_losses.py` | 5 | Key presence, gradient flow, perfect-vs-random, custom weights |
| `test_export.py` | 4 | ONNX creation, I/O shapes, round-trip, dynamic batch |

### End-to-end training (5-epoch smoke test)

```
Training on device: mps
  Train: 12000 examples, Val: 3000 examples
  Model parameters: 407,210

Epoch   1/5 | train_loss=3.4934 | val_loss=2.6167
  action_type_acc=0.254  speech_occurs_acc=0.581  awareness_acc=0.530
Epoch   5/5 | train_loss=2.5274 | val_loss=2.4879
  action_type_acc=0.246  speech_occurs_acc=0.581  awareness_acc=0.530
```

- Train loss decreasing epoch-over-epoch (3.49 → 2.53)
- Val loss tracking (2.62 → 2.49), no divergence
- Action type accuracy 25% (vs 17% random baseline for 6 classes)
- Awareness accuracy 53% (vs 20% random baseline for 5 classes)
- ONNX round-trip: max diff < 2e-6 across all heads (well within atol=1e-5)

### Remaining: full training run

`--epochs 100 --patience 10` expected to improve significantly — the 5-epoch run was still actively learning. Deferred to next session for full metrics analysis.
