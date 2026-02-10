# Event Classification Pipeline

## Purpose

The event classifier transforms natural language text into typed events and entity mentions. It processes both player input ("I pick up the ancient stone from the riverbed") and narrator prose ("Sarah trembles with fear as the wolf approaches"), producing:

1. **Event kinds** — multi-label classification over 8 categories with confidence scores
2. **Entity mentions** — text spans with NER category labels and character offsets

This is the entry point of the turn cycle. Every turn begins with the classifier extracting structured meaning from unstructured text. Downstream systems — character prediction, action resolution, context assembly — consume the classifier's output rather than operating on raw prose.

The classifier was always planned as an ML model (see [`narrator-architecture.md`](../narrator-architecture.md) § Event Classification). It replaced a prototype keyword-based classifier that used string matching for event type detection.

---

## Dual-Model Architecture

The event classifier uses two separate DistilBERT models sharing a common tokenizer:

### Event Classification Model

**Task**: Sequence-level, multi-label classification.
**Input**: Tokenized text (padded/truncated to 128 tokens).
**Output**: 8 logits (one per EventKind), activated via sigmoid.
**Decision**: Labels above threshold (default 0.5) are emitted.

A single input may produce multiple event kinds — "I tell Sarah about the hidden path" is both `SpeechAct` and `InformationTransfer`. The sigmoid activation and thresholding allow this naturally, unlike softmax which forces a probability distribution.

### NER Classification Model

**Task**: Token-level, BIO sequence tagging.
**Input**: Tokenized text (same padded/truncated encoding).
**Output**: 15 logits per token (one per BIO label), decoded via argmax.
**Decision**: Per-token argmax selects the most likely BIO label. A state machine assembles contiguous spans.

The two models share a tokenizer (`tokenizer.json`, HuggingFace format) but are trained independently and can be iterated separately. The public API (`classify_text()`) runs both models in a single call and returns a unified `ClassificationOutput`.

The implementation lives at `storyteller-engine/src/inference/event_classifier.rs`.

---

## Event Kinds

The 8 classifiable EventKinds correspond to types of narrative action that can occur in player input or narrator prose:

| Index | Label | Description |
|-------|-------|-------------|
| 0 | `StateAssertion` | Declares a fact or condition: "The door is locked" |
| 1 | `ActionOccurrence` | A physical action: "I pick up the stone" |
| 2 | `SpatialChange` | Movement or location change: "I walk to the clearing" |
| 3 | `EmotionalExpression` | Emotion expressed or displayed: "Sarah trembles with fear" |
| 4 | `InformationTransfer` | Knowledge shared or revealed: "I tell her about the path" |
| 5 | `SpeechAct` | Spoken dialogue: "I say hello to the stranger" |
| 6 | `RelationalShift` | Change in a relationship: "I betray the old man's trust" |
| 7 | `EnvironmentalChange` | World state change: "The storm breaks overhead" |

Two additional EventKinds — `SceneLifecycle` and `EntityLifecycle` — exist in the event grammar but are system-generated, not extracted from text by the classifier. The classifier never produces them.

The label constants are defined in `storyteller-ml/src/event_labels.rs`. Index order is critical: it determines which logit maps to which label during sigmoid decoding.

---

## Entity Extraction

### NER Categories

The NER model extracts 7 categories of narrative entities:

| Category | BIO Tags | Description | Examples |
|----------|----------|-------------|----------|
| Character | B-CHARACTER, I-CHARACTER | A person or being | "Sarah", "the old man", "I" |
| Object | B-OBJECT, I-OBJECT | A physical thing | "the stone", "a sword" |
| Location | B-LOCATION, I-LOCATION | A place or spatial reference | "the riverbed", "the clearing" |
| Gesture | B-GESTURE, I-GESTURE | Body language or physical expression | "clenched fists", "turned away" |
| Sensory | B-SENSORY, I-SENSORY | A sensory detail | "a distant howl", "the smell of smoke" |
| Abstract | B-ABSTRACT, I-ABSTRACT | An abstract concept | "the truth", "the betrayal" |
| Collective | B-COLLECTIVE, I-COLLECTIVE | A group | "the villagers", "the council" |

Plus the `O` (outside) tag for tokens that are not part of any entity mention. Total: 15 BIO labels (1 + 7 × 2).

The NER categories map directly to the `NerCategory` enum in `storyteller-ml/src/event_templates/mod.rs`, which is also used by the training data generation pipeline.

### BIO Tagging Scheme

**B-X** (begin) marks the first token of an entity mention of category X.
**I-X** (inside) marks continuation tokens of the same entity mention.
**O** (outside) marks tokens that are not part of any entity.

Example: "I pick up the ancient stone from the riverbed"

```
I        → B-CHARACTER
pick     → O
up       → O
the      → B-OBJECT
ancient  → I-OBJECT
stone    → I-OBJECT
from     → O
the      → B-LOCATION
riverbed → I-LOCATION
```

---

## BIO Span Assembly

The span assembly algorithm is a state machine that walks the per-token predictions left-to-right, building entity mentions from contiguous BIO-tagged spans.

### Algorithm

1. **Argmax per token**: For each token position, select the BIO label with the highest logit. Compute softmax confidence for the winning label.

2. **Skip special tokens**: Tokens with `word_id = None` ([CLS], [SEP], padding) are skipped. Any active span is emitted before skipping.

3. **State machine transitions**:
   - **B-X encountered**: Start a new span. If a previous span was active, emit it first.
   - **I-X matching current category**: Extend the current span (grow end offset, accumulate confidence).
   - **I-X not matching**: Emit the current span, reset. The orphaned I-X is dropped (strict BIO: I-X requires a preceding B-X of the same category).
   - **O encountered**: Emit any active span, reset.

4. **Character offset mapping**: Token positions are mapped back to character offsets in the original text via the tokenizer's offset mapping. This handles subword tokenization — "ancient" might tokenize to `["an", "##cient"]`, both mapping to the same original word.

5. **Entity emission**: Each emitted span produces an `ExtractedEntity` with the original text slice, character offsets, NER category, and average softmax confidence across the span's tokens.

### Subword Token Handling

DistilBERT uses WordPiece tokenization, which splits uncommon words into subword pieces. The `word_ids` from the tokenizer group subword tokens back to their original word index. The span assembly uses character offsets (not word indices) for precision — the `offsets` array from the tokenizer provides exact `(start, end)` character positions for each token.

---

## Inference in Rust

The `EventClassifier` struct in `storyteller-engine/src/inference/event_classifier.rs` owns two ONNX sessions, a shared tokenizer, and a dedicated rayon thread pool.

### Loading

```
EventClassifier::load(model_dir)
  Expects:
    model_dir/event_classifier.onnx    — event classification model
    model_dir/ner_classifier.onnx      — NER entity extraction model
    model_dir/tokenizer.json           — HuggingFace tokenizer configuration
```

### Classification Pipeline

```
classify_text(&self, input: &str) → ClassificationOutput

1. Tokenize
   tokenizer.encode(input, add_special_tokens=true)
   → token_ids, attention_mask, offsets, word_ids

2. Pad/truncate to MAX_SEQ_LENGTH (128)
   Short inputs: zero-padded
   Long inputs: truncated (first 128 tokens including [CLS])

3. Event classification
   Build i64 tensors [1, 128] for input_ids and attention_mask
   session.run(inputs!["input_ids", "attention_mask"])
   Extract "logits" output → sigmoid → threshold → event_kinds

4. NER classification
   Rebuild tensors (consumed by event model)
   session.run(inputs!["input_ids", "attention_mask"])
   Extract "logits" output → assemble_entity_spans()
   → entity_mentions

5. Return ClassificationOutput { event_kinds, entity_mentions }
```

The entire pipeline runs on the dedicated rayon thread pool via `self.pool.install(|| ...)`.

### Thread Safety

The HuggingFace `Tokenizer` is `Send + Sync` and shared without locking. The ONNX sessions require `Mutex` wrapping. The two sessions are locked independently — the event model lock is released before the NER model lock is acquired, so there is no deadlock risk.

---

## Label Contract

The label constants serve as the shared contract between Python training and Rust inference. Any change to label order, addition, or removal must be synchronized across both sides.

### Rust Side

`storyteller-ml/src/event_labels.rs` defines:
- `EVENT_KIND_LABELS: [&str; 8]` — event kind labels in index order
- `BIO_LABELS: [&str; 15]` — BIO labels: O first, then B-X/I-X pairs per category
- `NUM_EVENT_KINDS`, `NUM_BIO_LABELS`, `MAX_SEQ_LENGTH` — dimension constants
- `bio_label_to_category()` — parse a BIO label string to `NerCategory`
- `is_begin_tag()` — check if a label is a B- tag

### Python Side

`training/event_classifier/src/event_classifier/schema.py` mirrors these constants:
- `EVENT_KINDS: list[str]` — same 8 labels in same order
- `NER_LABELS: list[str]` — same 15 BIO labels in same order
- Label-to-index and index-to-label mappings

### Tests

The Rust test suite verifies:
- Label counts match expected values (8 event kinds, 15 BIO labels)
- BIO labels alternate B-X/I-X after the initial O
- Every B-/I- label round-trips through `bio_label_to_category()`
- Event kind labels match the Python schema's exact order (positional assertions)

---

## Boundary with Pipeline Orchestration

The `EventClassifier` produces `ClassificationOutput` — event kind labels with confidences, and entity mentions with spans and categories. This is the classifier's complete responsibility.

Importantly, the classifier does **not** produce `EventFeatureInput` (the character prediction model's feature encoding for the "player event" input region). `EventFeatureInput` is a feature encoding type — it belongs to the character prediction pipeline's feature schema, not to the event classification pipeline.

The conversion from `ClassificationOutput` → `EventFeatureInput` belongs in the pipeline orchestration layer (the turn cycle Bevy system, Phase C.4/C.5). The two pipelines are related by sequence — event classification runs first, character prediction runs second — but not by responsibility. Each pipeline owns its own input and output types.

This separation means the event classifier can be improved independently (new event kinds, better NER, different thresholds) without touching the character prediction pipeline, and vice versa.
