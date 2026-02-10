# ML Strategy

## Purpose

This directory documents the storyteller system's ML pipeline architecture — the models, training data, inference infrastructure, and design decisions that replaced multi-agent LLM calls with purpose-built prediction and classification models.

The documents here are descriptive, not exhaustive. They explain *what* each pipeline does, *why* it was designed that way, and *where* to find the implementation. For the motivating argument — why the system collapsed from multiple LLM agents to a single Narrator — see [`narrator-architecture.md`](../narrator-architecture.md).

---

## The Single-Narrator Argument

The first playable scene (Bramblehoof + Pyotir against local Ollama with Mistral 7B) proved the pipeline shape was correct: player input → classification → character deliberation → narrative rendering. It also proved that multi-agent LLM calls were computationally prohibitive — minutes between turns on an M4/64GB machine.

The architectural response: collapse the LLM-dependent agents to a **single Narrator**, the system's sole generative intelligence. Replace character behavior prediction with ML models. Replace world constraint enforcement with a deterministic rules engine. Replace the Storykeeper's multi-consumer filtering with single-consumer context assembly.

The computational graph now funnels: many fast ML inferences → deterministic resolution → one expensive but high-quality LLM call. Each layer does what it is actually good at.

---

## Two ML Pipelines

The system runs two distinct ML pipelines per turn, serving different roles in the turn cycle:

### Character Prediction

**Purpose**: Predict what each character would do, say, and think — structured intent with confidence values — given their personality tensor, emotional state, relational context, and scene conditions.

**Architecture**: A multi-head MLP (38KB ONNX) consuming 453-dimensional feature vectors. Four heads produce action, speech, thought, and emotional shift predictions. Sub-millisecond inference.

**Why this model**: The input is already feature-engineered — structured floats, one-hot categoricals, padded arrays. A transformer would add latency and parameters for no benefit over structured data. An MLP is fast, tiny, and sufficient.

See: [character-prediction.md](character-prediction.md)

### Event Classification

**Purpose**: Extract typed events and entity mentions from natural language — player input and narrator prose alike. Transform "I pick up the ancient stone from the riverbed" into `ActionOccurrence` (81% confidence) with entity spans: "I" (Character), "the ancient stone" (Object), "the riverbed" (Location).

**Architecture**: Dual DistilBERT models (~534MB total ONNX) — one for sequence-level event kind classification (8 labels, multi-label sigmoid), one for token-level NER (15 BIO labels, argmax span assembly). Shared tokenizer.

**Why this model**: Event classification requires contextual token representations and semantic understanding of natural language. Pre-trained transformer encoders provide this via self-attention and masked language model pre-training.

See: [event-classification.md](event-classification.md)

---

## Shared Architectural Patterns

Despite their different model architectures, both pipelines share the same inference infrastructure:

### Mutex-Wrapped ONNX Sessions

`ort::Session::run()` requires `&mut self`. Both `CharacterPredictor` and `EventClassifier` wrap their sessions in `std::sync::Mutex` so that the public inference methods can take `&self`. For the character predictor (sub-millisecond inference on a 38KB model), lock contention is negligible. For the event classifier (~5-15ms per forward pass), contention is manageable under sequential turn processing.

### Dedicated Rayon Thread Pools

Both predictors construct a dedicated 2-thread rayon pool (`thread_name: "ml-predict-{i}"` / `"event-classify-{i}"`). ML inference is CPU-bound and must not block the tokio async runtime. The pool provides compute isolation and enables parallel batch prediction for the character predictor.

### Feature Schemas as Rust-Python Contracts

Both pipelines define their encoding contracts as shared constants:

- **Character prediction**: `storyteller-ml/src/feature_schema.rs` defines 453 input dimensions and 42 output dimensions with named regions. The Python training code (`training/src/training/`) reads the same schema metadata (exported as JSON) to build matching DataLoaders.
- **Event classification**: `storyteller-ml/src/event_labels.rs` defines the 8 `EVENT_KIND_LABELS` and 15 `BIO_LABELS` in exact index order. The Python training code (`training/event_classifier/src/event_classifier/schema.py`) mirrors these constants. Label order is critical — it determines argmax decoding.

### Feature-Gated Integration Tests

Both pipelines use the `test-ml-model` feature flag to gate tests requiring ONNX models on disk. Models live in the private `storyteller-data` repository, accessed via `STORYTELLER_DATA_PATH`. Unit tests for encoding, decoding, and structural correctness run without models.

```bash
cargo test --workspace                           # Unit tests only
cargo test --workspace --features test-ml-model   # + ONNX model tests
```

---

## Interior Replaceability

ONNX is the abstraction layer between training and inference. The Python training pipeline can swap encoders (DistilBERT → DeBERTa → ModernBERT) without changing the Rust inference code — the `EventClassifier` loads whatever ONNX model is at the expected path and runs it through the same tokenize → pad → forward-pass → decode pipeline.

Similarly, the training data generation can iterate independently. New templates, expanded vocabularies, or real prose annotations improve model quality without touching inference.

This is deliberate: the system separates the **what** (feature schema, label contract, output types) from the **how** (model architecture, training procedure, hyperparameters). The contracts are stable; the models behind them improve.

---

## Documents

| Document | Description |
|---|---|
| [character-prediction.md](character-prediction.md) | Feature schema, MLP architecture, inference, enrichment and rendering |
| [event-classification.md](event-classification.md) | Dual-model architecture, BIO tagging, entity extraction, label contract |
| [training-data-generation.md](training-data-generation.md) | Combinatorial generation: descriptors, templates, reproducibility |
| [model-selection.md](model-selection.md) | Model choices, DeBERTa/DistilBERT tradeoffs, ONNX validation |

---

## Current Status

| Pipeline | Status | Deployed Models |
|----------|--------|-----------------|
| Character prediction | Deployed (Phase 0) | `character_predictor.onnx` (38KB) |
| Event classification | Deployed (Phase C.3) | `event_classifier.onnx` (~268MB) + `ner_classifier.onnx` (~266MB) |
| Training data (character) | Complete | ~7,500 examples via combinatorial matrix |
| Training data (event) | Complete | 8,000 examples via template expansion |

**In progress**: Phase C.4+ (Bevy system integration — wiring the event classifier into the turn cycle pipeline).

**Future**: Phase C.6 evaluation framework (real prose testing), potential model consolidation (single multi-task encoder), quantization for deployment size reduction, revisiting DeBERTa-v3-small when cloud GPU training is available.
