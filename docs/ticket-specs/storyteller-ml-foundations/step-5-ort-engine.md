# Step 5: ort Inference Integration in Rust

## Status: COMPLETE

## What Was Built

Rust-side ONNX inference via `ort` in `storyteller-engine`, closing the full ML prediction loop: feature encoding → ONNX inference → decoded predictions, entirely in Rust at runtime.

## Architecture

```
PredictionInput (CharacterSheet + edges + scene + event + history)
        │
        ▼
storyteller_ml::feature_schema::encode_features()    →  Vec<f32> [453]
        │
        ▼
ort::Session::run()  (character_predictor.onnx)
        │
        ▼
4 output tensors: action[14] + speech[6] + thought[6] + emotion[16]
        │
        ▼
concatenate → Vec<f32> [42]
        │
        ▼
storyteller_ml::feature_schema::decode_outputs()     →  RawCharacterPrediction
```

## Files Modified/Created

### `storyteller-engine/Cargo.toml` (MODIFIED)
Added `storyteller-ml` as a path dependency so the engine can access `encode_features()` and `decode_outputs()`.

### `storyteller-core/src/errors.rs` (MODIFIED)
Added `Inference(String)` variant to `StorytellerError` for ort/ONNX Runtime errors.

### `storyteller-engine/src/inference/frame.rs` (REWRITTEN)
Replaced the 12-line stub with `CharacterPredictor`:

```rust
pub struct CharacterPredictor {
    session: Mutex<Session>,
    pool: rayon::ThreadPool,
}
```

- **`load(model_path)`** — Loads ONNX model, creates dedicated 2-thread rayon pool
- **`predict(input, character_id, axes, confidence)`** — Single-character inference
- **`predict_batch(inputs)`** — Parallel inference via rayon

### `docs/ticket-specs/storyteller-ml-foundations/step-5-ort-engine.md` (THIS FILE)

## Design Decisions

### 1. Session wrapped in `Mutex`, not bare `Arc`

`ort::Session::run()` requires `&mut self`. The session is wrapped in `Mutex<Session>` so that `predict()` can take `&self`. For a 38KB model, inference is sub-millisecond, so lock contention is negligible. This is simpler than maintaining per-thread sessions.

### 2. Compute isolation: rayon thread pool

ML inference is CPU-bound and must not block the tokio async runtime. `CharacterPredictor` owns a dedicated rayon `ThreadPool` (2 threads, named `ml-predict-{i}`). `predict_batch()` uses `pool.install()` to run parallel predictions on this pool.

### 3. Model path resolved by caller

`CharacterPredictor::load()` takes a `&Path` — the caller resolves `STORYTELLER_MODEL_PATH` or `STORYTELLER_DATA_PATH/models`. No config coupling.

### 4. Output reassembly: concatenate 4 named outputs

The ONNX model exports 4 named output tensors (`action`, `speech`, `thought`, `emotion`). These are concatenated in order into a flat `Vec<f32>` [42] matching the Python training label layout, then decoded by `decode_outputs()`.

### 5. Single-character inference (batch=1)

Each character is predicted independently with input shape `[1, 453]`. Parallelism comes from rayon, not ONNX batch dimension.

### 6. Character ID passed through, not encoded

The ML model doesn't know about entity IDs. The `character_id` is passed as a parameter and attached to the output `RawCharacterPrediction` without going through the model.

## Tests

| Test | Attribute | What it verifies |
|------|-----------|------------------|
| `load_nonexistent_model_errors` | (always runs) | Bad path → `StorytellerError::Inference` |
| `predict_with_real_model` | `test-ml-model` feature | Full pipeline: encode → infer → decode. Validates action type, speech register, awareness level are valid enum variants |
| `predict_batch_runs_parallel` | `test-ml-model` feature | Two characters predicted, distinct `character_id`s preserved |

Run model tests: `STORYTELLER_DATA_PATH=... cargo test --workspace --features test-ml-model`

## Verification Results

- `cargo check --all-features` — compiles
- `cargo clippy --all-targets --all-features` — no warnings
- `cargo fmt --check` — formatted
- `cargo test --all-features --workspace` — 113 tests pass, 3 ignored (2 new + 1 existing Ollama)
- Ignored tests pass with real model: predict Bramblehoof → valid ActionType, SpeechRegister, AwarenessLevel
