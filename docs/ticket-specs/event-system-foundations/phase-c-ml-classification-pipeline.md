# Phase C: ML Classification Pipeline

**Status**: C.0–C.2 complete, C.3–C.6 not started
**Branch**: `jcoletaylor/event-system-foundations`
**Depends on**: Phase A (types), Phase B (promotion logic)

## Context

Phases A and B established the event grammar vocabulary and the relational weight principle:

- **Phase A**: EventAtom, EventKind (10 variants), Participant, ParticipantRole (6 roles), RelationalImplication, ImplicationType (9 types), EntityRef, EventSource, EventConfidence, Turn types
- **Phase B**: compute_relational_weight(), determine_promotion_tier(), resolve_entity_ref() (4 strategies), MentionIndex with retroactive promotion

The current event classification is deliberately naive — `classify_player_input()` in `context/prediction.rs` uses hardcoded keyword patterns, always outputs confidence 0.8, and produces `EventFeatureInput` (a 4-field struct: EventType, EmotionalRegister, confidence, target_count). This was always a placeholder.

Phase C replaces this with an ML pipeline that produces the richer types from Phase A: `EventAtom` instances with typed `EventKind`, extracted `EntityRef` participants, and inferred `RelationalImplication` instances that feed into Phase B's weight computation and entity promotion.

### Design Philosophy: Interior Replaceability

This phase will undergo significant iteration. Model choices, training data strategies, and architectural decisions made here are **working hypotheses**, not commitments. The implementation should optimize for:

1. **Swappable models**: The inference interface (`EventClassifier`) abstracts over which ONNX model backs it. Model architecture changes (DeBERTa → ModernBERT, separate models → multi-task, fine-tuned → zero-shot) require only new model files and minor decode logic, not pipeline restructuring.

2. **Swappable training data**: The combinatorial template system should make it cheap to regenerate training data with different templates, different LLM augmentation, or different label schemas. Training data is disposable — the templates and generation logic are the durable artifact.

3. **Measurable quality**: Every stage has explicit quality metrics (accuracy per EventKind, entity extraction F1, implication precision). When we swap a component, we can measure whether it improved.

4. **Graceful degradation throughout**: The existing keyword classifier continues to serve the character prediction pipeline via `EventFeatureInput`. The new ML classifier runs alongside it, producing richer `EventAtom` output. Nothing breaks if the ML classifier is absent or underperforming — the system falls back to keyword classification for character predictions and skips event extraction.

### Relationship to Other Documents

| Document | Role |
|---|---|
| `classifying-events-and-entities.md` | ML research — model selection, academic frameworks, training data strategy, latency analysis |
| `implementation-plan.md` | Original Phase C outline — this document supersedes and concretizes it |
| `event-grammar.md` | Target type system — EventAtom, EventKind, Participant, RelationalImplication |
| `entity-reference-model.md` | Entity extraction targets — EntityRef, referential context, promotion lifecycle |
| `event-dependency-graph.md` | Downstream consumer — the classifier's vocabulary must be rich enough for DAG node conditions |

---

## Architecture Overview

### Two Output Paths, One Pipeline

The ML classifier produces two outputs from the same text:

```
Player input text
       │
       ├──→ EventFeatureInput        (backward-compatible, feeds character prediction)
       │    4 fields: EventType, EmotionalRegister, confidence, target_count
       │
       └──→ Vec<EventAtom>           (new, feeds event ledger + promotion pipeline)
            Full event grammar: EventKind, Participant[], RelationalImplication[], confidence
```

Both are derived from the same classification pass. `EventFeatureInput` is a lossy projection of the richer `EventAtom` output — we compute it from the EventAtom for backward compatibility rather than running a separate classifier.

### Pipeline Stages

```
Text → Tokenize → Encode → Classify → Extract → Resolve → Imply → Assemble
  │        │         │         │          │         │         │         │
  │     tokenizers  ort     EventKind  EntityRef  Phase B  Implication  EventAtom
  │      crate    Session   per clause  per span  resolve  inference    output
  │                                                _ref()
```

Each stage is a pure function (or small set of functions) operating on the output of the previous stage. Stages can be tested independently. The full pipeline is composed in `EventClassifier::classify_text()`.

### Model Strategy: Start Separate, Consolidate When Stable

The research document (`classifying-events-and-entities.md`) recommends starting with separate models per task (Approach B) and consolidating to a multi-task model (Approach A) when task definitions stabilize. This is the right call — multi-task training introduces coupling between tasks that makes iteration harder during the phase where we're still discovering what works.

**Initial deployment**: 1-2 separate ONNX models (event classification, entity extraction). Relation inference starts as heuristic mapping.

**Target deployment**: Single multi-task ONNX model with 3 heads (event, NER, role). Relation inference optionally ML-backed.

The `EventClassifier` struct abstracts this — callers don't know whether one or three models are running behind the interface.

---

## Work Items

### C.0: Infrastructure — `tokenizers` Crate Integration ✅

**Status**: Complete (Feb 9 2026, commit `f561e76`)

**Goal**: Add the `tokenizers` crate to the workspace and verify it loads HuggingFace tokenizer files.

**Why first**: Every subsequent work item depends on tokenization. Getting this working (and understanding its API surface — offset mapping, word IDs, subword handling) before writing model code eliminates a class of integration surprises.

**What was built**:
- `tokenizers` added to `[workspace.dependencies]` in root `Cargo.toml`
- `tokenizers = { workspace = true }` in `storyteller-engine/Cargo.toml`
- `EventClassifier` struct scaffolded in `storyteller-engine/src/agents/classifier.rs` — module declaration with doc comments establishing the classification pipeline architecture (Stage 1 factual + Stage 3 interpretive)

**Notes**:
- The `tokenizers` crate IS the reference implementation — Python's `tokenizers` is a binding to this Rust code. API documentation is excellent.
- Tokenizer files (`tokenizer.json`) are exported alongside ONNX models and should live in the same model directory. We make use of STORYTELLER_MODEL_PATH for this in .env - by default set to our workspace root sibling directory `storyteller-data/models`
- Key APIs: `Tokenizer::from_file()`, `encoding.get_ids()`, `encoding.get_offsets()`, `encoding.get_word_ids()` (maps subword tokens back to word indices — critical for NER span extraction).

**Verification**: `cargo check --all-features` passes.

---

### C.1: Training Data Generation — Event Classification ✅

**Status**: Complete (Feb 9 2026, commit `eafa3d7`)

**Goal**: Build the combinatorial training data pipeline for event classification. Follows the pattern established in `storyteller-ml/src/matrix/` — parameterized templates, combinatorial expansion, JSONL output.

**What was built**:
- Rust template engine in `storyteller-ml/src/event_templates/` — combinatorial expansion across all 8 classifiable EventKinds
- Binary: `generate-event-training-data` CLI for JSONL generation
- 8,000 annotated training examples generated at `$STORYTELLER_DATA_PATH/training-data/event_classification.jsonl`
- Two registers per template: player-input (imperative, short) and narrator-prose (literary, past tense)
- Programmatic entity annotation with character-offset spans computed during generation
- Multi-label examples included (e.g., SpatialChange + EmotionalExpression)

**Training data format** (one JSON object per line):

```json
{
  "id": "action-occurrence-player-042",
  "text": "I pick up the ancient stone from the riverbed",
  "register": "player",
  "event_kinds": ["ActionOccurrence"],
  "entities": [
    {"start": 0, "end": 1, "text": "I", "category": "CHARACTER", "role": "Actor"},
    {"start": 15, "end": 31, "text": "the ancient stone", "category": "OBJECT", "role": "Target"},
    {"start": 37, "end": 49, "text": "the riverbed", "category": "LOCATION", "role": "Location"}
  ]
}
```

**Note**: Entity spans use `start`/`end` character offsets (not word-index arrays). This aligns with how the `tokenizers` crate's `offset_mapping` works for BIO label alignment.

**Data distribution** (8,000 examples across 8 EventKinds):
- All event kinds well-represented (1,000 per kind)
- 14,956 total entity annotations across 7 NER categories
- Both player and narrator registers

**Tests**: Generated JSONL validates against schema (EventKind values are valid, span offsets are within text bounds, entity categories are from the vocabulary).

---

### C.2: Model Fine-Tuning and ONNX Export ✅

**Status**: Complete (Feb 9 2026, commit `81ad27f`)

**Goal**: Fine-tune a pre-trained transformer encoder on the generated training data and export to ONNX.

**What was built**:

A complete Python fine-tuning pipeline at `training/event_classifier/` — 8 modules, 34 tests, HuggingFace Trainer-based training with ONNX export and validation.

**Package structure**:

```
training/event_classifier/
  pyproject.toml                    # torch, transformers, datasets, optimum, seqeval, accelerate
  uv.lock                          # Reproducible dependency resolution
  src/event_classifier/
    __init__.py
    schema.py                       # Label vocabs: 8 EventKinds, 15 BIO labels, model constants
    dataset.py                      # JSONL loading, tokenization, BIO alignment, HF Dataset creation
    models.py                       # load_event_classifier(), load_ner_model() thin wrappers
    metrics.py                      # compute_event_metrics() (macro F1), compute_ner_metrics() (seqeval)
    train.py                        # TrainConfig dataclass, HF Trainer, WeightedNerTrainer subclass
    export.py                       # ONNX export via torch.onnx.export + PyTorch-vs-ORT validation
    cli.py                          # CLI: --task {event,ner}, --validate-only, --export-only, --cpu
  tests/
    conftest.py                     # Fixtures: synthetic JSONL, pytest configuration
    test_schema.py                  # Label vocab consistency, ID mappings
    test_dataset.py                 # BIO alignment (critical surface), HF Dataset creation
    test_models.py                  # Output shapes: [batch, 8] event, [batch, seq_len, 15] NER
    test_metrics.py                 # Metric computation from synthetic predictions
    test_export.py                  # ONNX export and numerical validation
```

**Deployed models** at `$STORYTELLER_DATA_PATH/models/event_classifier/`:

| File | Size | Description |
|---|---|---|
| `event_classifier.onnx` | ~268MB | Multi-label sequence classification (8 EventKinds) |
| `ner_classifier.onnx` | ~266MB | Token-level BIO NER (15 labels, 7 entity categories) |
| `tokenizer.json` | ~712KB | Shared DistilBERT tokenizer |

**Model: DistilBERT (fallback)**. DeBERTa-v3-small was the primary choice but produces NaN gradients on Apple Silicon MPS due to numerical instability in its disentangled attention mechanism (content-to-content, content-to-position, position-to-content triple attention). CPU training was numerically correct but prohibitively slow (~66s/step, ~78 hours for 10 epochs on M4). DistilBERT (`distilbert-base-uncased`, 66M params) works correctly on MPS at ~6.3 it/s.

**Training results** (DistilBERT, 10 epochs, 8,000 examples):

| Task | Metric | Score | Eval Loss | ONNX Validation |
|---|---|---|---|---|
| Event classification | macro F1 | 1.0 | 0.00047 (monotonically decreasing) | max diff 4.53e-06 |
| NER entity extraction | entity F1 | 1.0 | < 0.001 | max diff 4.39e-05 |

**Note on perfect F1**: Expected for combinatorial/templated training data where patterns are highly regular. Real prose will score lower. The models will need re-evaluation on actual manuscript text (Bramblehoof workshop scenes) — see C.6.

**Task 1 — Event classification (sequence classification)**:
- Input: tokenized text (max 128 tokens)
- Output: 8-dim sigmoid logits (multi-label, one per classifiable EventKind)
- Loss: binary cross-entropy
- Metric: per-class F1, macro F1
- Training: HuggingFace `Trainer`, `load_best_model_at_end=True`

**Task 2 — Entity extraction (token classification)**:
- Input: tokenized text (max 128 tokens)
- Output: per-token BIO labels for 7 categories (CHARACTER, OBJECT, LOCATION, GESTURE, SENSORY, ABSTRACT, COLLECTIVE) = 15 labels (7×B + 7×I + O)
- Loss: cross-entropy with inverse-frequency class weights (O tokens dominate)
- Metric: entity-level F1 via `seqeval` (span-exact match)
- Training: `WeightedNerTrainer` subclass with class-weighted loss

**Task 3 — Participant role labeling (token classification)** — *deferred to C.2b*:
- Depends on having both events and entities extracted. Can be trained as a follow-up once C.2a stabilizes.
- In the interim, participant roles are inferred heuristically from entity position and EventKind (C.4).

**ONNX export**: Manual `torch.onnx.export(dynamo=False)` with dynamic axes for batch and sequence dimensions. HuggingFace Optimum's `main_export()` available as alternative path. Validation compares PyTorch and ORT outputs on the same input (atol=1e-4).

**CLI usage**:

```bash
cd training/event_classifier && uv sync --dev

# Validate data
uv run train-event-classifier --validate-only $STORYTELLER_DATA_PATH/training-data/event_classification.jsonl

# Train + export (event classification)
uv run train-event-classifier --task event $DATA --epochs 10 -o /tmp/event_out

# Train + export (NER)
uv run train-event-classifier --task ner $DATA --epochs 10 -o /tmp/ner_out

# Export from existing checkpoint
uv run train-event-classifier --task event --export-only /tmp/event_out/model -o /tmp/export

# Force CPU (workaround for DeBERTa on Apple Silicon)
uv run train-event-classifier --task event --cpu $DATA
```

**Lesson learned — DeBERTa-v3 on Apple MPS**: DeBERTa-v3's disentangled attention has known numerical instability on Apple's MPS backend. Training starts normally but NaN gradients appear at ~epoch 0.59, the model never recovers, and all subsequent F1 scores are 0. The `--cpu` flag works around this numerically but is impractically slow. DistilBERT is the correct choice for local development on Apple Silicon. If DeBERTa quality is needed in the future, train on CUDA/cloud GPU.

---

### C.3: EventClassifier in Rust

**Goal**: Build the `EventClassifier` struct that loads ONNX model(s) and tokenizer, runs inference, and decodes outputs into Phase A types.

**Outputs**:
- New file: `storyteller-engine/src/inference/event_classifier.rs`
- Modified: `storyteller-engine/src/inference/mod.rs` (add module)

**Core struct**:

```rust
pub struct EventClassifier {
    /// Event classification model (sequence-level).
    event_session: Mutex<Session>,
    /// Entity extraction model (token-level).
    ner_session: Mutex<Session>,
    /// Shared tokenizer (thread-safe, no mutex needed).
    tokenizer: Tokenizer,
    /// Dedicated thread pool for CPU-bound inference.
    pool: rayon::ThreadPool,
}
```

**Two Sessions initially** (Approach B), collapsible to one when multi-task model is ready. The public API doesn't change — `classify_text()` returns the same `ClassificationOutput` regardless of how many models back it.

**Public API**:

```rust
impl EventClassifier {
    /// Load models and tokenizer from a directory.
    pub fn load(model_dir: &Path) -> Result<Self, StorytellerError>;

    /// Classify a single text input (player input or prose clause).
    /// Returns event kinds, entity mentions, and confidence.
    pub fn classify_text(
        &self,
        input: &str,
        scene_cast: &[TrackedEntity],
    ) -> Result<ClassificationOutput, StorytellerError>;
}

/// Output of a single classification pass.
pub struct ClassificationOutput {
    /// Detected event kinds with confidence scores.
    pub event_kinds: Vec<(EventKind, f32)>,
    /// Extracted entity mentions with spans and categories.
    pub entity_mentions: Vec<ExtractedEntity>,
    /// Backward-compatible event feature input for character prediction.
    pub legacy_event_input: EventFeatureInput,
}

/// An entity mention extracted from text.
pub struct ExtractedEntity {
    /// The text span.
    pub text: String,
    /// Character offsets in the original input.
    pub start: usize,
    pub end: usize,
    /// NER category (CHARACTER, OBJECT, etc.).
    pub category: NerCategory,
    /// Resolved entity reference (if matched against scene cast).
    pub entity_ref: EntityRef,
    /// Extraction confidence.
    pub confidence: f32,
}
```

**Inference flow**:

1. **Tokenize**: `tokenizer.encode(input, true)` → token IDs, attention mask, offsets
2. **Event classification**: Build `[1, seq_len]` tensors → `event_session.run()` → `[1, 10]` sigmoid logits → threshold → `Vec<(EventKind, f32)>`
3. **Entity extraction**: Same tensors → `ner_session.run()` → `[1, seq_len, 15]` BIO logits → argmax → span assembly using `get_offsets()` → `Vec<ExtractedEntity>`
4. **Entity resolution**: For each extracted entity, attempt resolution against `scene_cast` using Phase B's `resolve_entity_ref()` and simple string matching
5. **Legacy bridge**: Map the highest-confidence EventKind to `EventType`, map entity count to `target_count`, produce `EventFeatureInput` for backward compatibility

**Key implementation detail — span assembly**: BIO tags operate on subword tokens. The `tokenizers` crate's `get_word_ids()` maps subword tokens back to word indices, and `get_offsets()` maps tokens back to character positions. Span assembly groups consecutive B-I tokens with the same category, then maps back to character offsets in the original text. This is a well-known NER post-processing step.

**Rayon pool**: Same pattern as `CharacterPredictor` — dedicated 2-thread pool isolates CPU-bound inference. Could share the existing pool if configured to allow it.

**Error handling**: Model loading failures produce `StorytellerError::Inference`. Classification failures on individual inputs are logged and return empty results (graceful degradation, not panics).

**Tests**:
- Unit: `ClassificationOutput` constructibility, legacy bridge mapping (EventKind → EventType)
- Feature-gated (`test-ml-model`): Load real ONNX models, classify test inputs, verify output shapes and value ranges
- Entity resolution: Extracted entities resolve against mock scene cast

---

### C.4: Relational Implication Inference

**Goal**: Given classified events and extracted entities, infer relational implications. This is the **critical bridge** — without it, classified events produce no entity promotions.

**Outputs**:
- New file: `storyteller-engine/src/context/implication.rs` (or extend `prediction.rs`)
- Heuristic mapping tables: EventKind × ParticipantRole → ImplicationType

**Two-tier approach**:

**Tier 1 — Heuristic mapping (deterministic, ~1ms)**:

```rust
pub fn infer_implications_heuristic(
    kind: &EventKind,
    participants: &[Participant],
) -> Vec<RelationalImplication>
```

Uses a lookup table:

| EventKind | Actor→Target | Actor→Instrument | Actor→Location | Witness→Actor |
|---|---|---|---|---|
| `SpeechAct` | Attention + InformationSharing | — | — | Attention |
| `ActionOccurrence(Perform)` | Attention | Possession | Proximity | Attention |
| `ActionOccurrence(Examine)` | Attention | — | — | — |
| `SpatialChange` | — | — | Proximity | — |
| `EmotionalExpression` | EmotionalConnection | — | — | Attention |
| `InformationTransfer` | InformationSharing + TrustSignal | — | — | — |
| `RelationalShift` | (direct: uses delta) | — | — | — |
| `StateAssertion` | — | — | — | — |
| `EnvironmentalChange` | — | — | — | — |

Weight assignment: base weight per ImplicationType (Attention=0.3, Possession=0.5, EmotionalConnection=0.6, TrustSignal=0.7, InformationSharing=0.5, Conflict=0.8, Care=0.7, Obligation=0.8, Proximity=0.2), scaled by event confidence.

**Tier 2 — ML relation classifier (future, C.4b)**:

Entity Marker approach for ambiguous cases. Deferred until we have training data and can measure where the heuristic falls short. The interface is ready — `infer_implications()` dispatches to heuristic first, ML second for low-confidence cases.

**Participant role assignment** (until C.2b provides ML role labeling):

Heuristic based on entity position and EventKind:
- First CHARACTER entity in an ActionOccurrence/SpeechAct → Actor
- Second CHARACTER entity → Target
- OBJECT entity in ActionOccurrence → Target or Instrument (based on verb semantics — simplified: if only one OBJECT, it's Target)
- LOCATION entity → Location
- All other CHARACTER entities in scene → Witness (if present in scene cast but not Actor/Target)

This is imprecise but covers ~80% of cases for typical player input. The 20% that are wrong produce incorrect implications at reduced weight (confidence-scaled), which the relational substrate smooths over across many events.

**Tests**:
- Every EventKind × ParticipantRole combination produces expected implications
- Weight values are in [0.0, 1.0]
- Confidence scaling works correctly
- **Integration test**: classified text (C.3) → participant assignment → implication inference → weight computation (Phase B) → promotion tier — the full pipeline from text to promotion decision

---

### C.5: Pipeline Integration

**Goal**: Wire the EventClassifier into the existing turn cycle alongside the character prediction pipeline. Both paths run from the same player input.

**Outputs**:
- Modified: `storyteller-engine/src/context/prediction.rs` — add `classify_and_extract()` function that runs both classification paths
- Modified: `storyteller-cli/src/bin/play_scene_context.rs` — optionally load EventClassifier and display extracted events

**Integration point**:

```rust
/// Classify player input and produce both character prediction features
/// and full event extraction output.
pub fn classify_and_extract(
    input: &str,
    scene_cast: &[TrackedEntity],
    event_classifier: Option<&EventClassifier>,
    character_count: usize,
) -> (EventFeatureInput, Option<Vec<EventAtom>>) {
    match event_classifier {
        Some(classifier) => {
            let output = classifier.classify_text(input, scene_cast)?;
            // Build EventAtom instances from classification output
            let atoms = assemble_atoms(&output, scene_cast);
            (output.legacy_event_input, Some(atoms))
        }
        None => {
            // Fall back to keyword classifier
            let event = classify_player_input(input, character_count);
            (event, None)
        }
    }
}
```

The `assemble_atoms()` function constructs `EventAtom` instances:
- `EventKind` from classification
- `Participant` list from entity extraction + role assignment
- `RelationalImplication` list from C.4 heuristic inference
- `EventSource::PlayerInput` with `ClassifierRef` identifying the model
- `EventConfidence` from classification scores

**Atom assembly** is where classification, extraction, and implication inference come together into the Phase A types. Each classified EventKind becomes one EventAtom. Multi-label classifications (one clause → two EventKinds) produce two atoms from the same text.

**Character prediction pipeline unchanged**: `predict_character_behaviors()` still receives `EventFeatureInput` and runs identically. The new `EventAtom` output is an additional, parallel data path.

**Tests**:
- Integration: `classify_and_extract()` with ML classifier produces both outputs
- Integration: `classify_and_extract()` without ML classifier falls back to keywords
- Integration: assembled EventAtoms have valid structure (all required fields populated, confidence in range, entity refs present)
- Feature-gated (`test-ml-model`): full pipeline with real ONNX model

---

### C.6: Evaluation Framework

**Goal**: Build tooling to measure classification quality against labeled test data. This is essential for the "interior replaceability" principle — when we swap models or training data, we need to know if quality improved.

**Outputs**:
- New binary: `storyteller-ml/src/bin/evaluate-classifier.rs` (or Python script in `training/event_classifier/`)
- Evaluation metrics per task

**Metrics**:

| Task | Primary Metric | Target | Notes |
|---|---|---|---|
| Event classification | Per-class F1 + macro F1 | >85% macro F1 | Multi-label — evaluate per EventKind |
| Entity extraction | Entity-level F1 (span-exact) | >80% F1 | Strict span matching |
| Implication inference | Precision per ImplicationType | >75% precision | Recall less critical — missing implications are low-cost |
| End-to-end | Promotion correctness | qualitative | Does the pipeline promote the right entities for test scenarios? |

**Evaluation data**: Hold out 10-20% of generated training data. Additionally, manually annotate ~50-100 examples from workshop scene text (Bramblehoof) for realistic-register evaluation.

**This is lightweight tooling, not a test harness**: The evaluation script loads a model, runs it against labeled data, prints metrics. It runs manually (not in CI). The goal is to have a repeatable measurement when we change something.

---

## What Phase C Does NOT Include

- **Narrator prose classification** — Phase D. Phase C classifies player input only. Prose requires clause segmentation and cross-clause coreference, which are Phase D concerns.
- **Turn lifecycle management** — Phase D. Phase C produces EventAtoms from individual text inputs; Phase D manages the committed-turn extraction flow.
- **Prediction confirmation** — Phase D. Cross-referencing extracted atoms against ML prediction metadata.
- **Event composition** — Phase E. Detecting compound events from sequential atoms.
- **Bevy system integration** — Phase D. Phase C provides the `EventClassifier` struct; Phase D wires it into the Bevy turn cycle as a system.
- **Full coreference resolution** — Phase D. Phase C does entity resolution against the scene cast (string matching + Phase B's `resolve_entity_ref()`). Cross-turn and pronominal coreference is Phase D.
- **Event dependency DAG** — Future work. The classifier produces EventAtoms; the DAG evaluates dependency relationships between event conditions across the narrative arc.

## Open Questions

1. **DeBERTa ONNX export stability**: **RESOLVED — not applicable.** DeBERTa-v3-small has NaN instability on Apple MPS during training (not just export). DistilBERT used as fallback with excellent results. ModernBERT remains an aspirational replacement when its ecosystem matures. If DeBERTa quality is needed, train on CUDA/cloud GPU.

2. **Training data volume calibration**: **PARTIALLY RESOLVED.** 8,000 examples (1,000 per EventKind) with combinatorial templates produced F1=1.0 on both tasks. This confirms template data is sufficient for model learning, but the real test is performance on naturalistic prose. LLM augmentation remains a lever for when real-text evaluation reveals gaps.

3. **Entity extraction scope**: Should the NER model extract ALL entity mentions (comprehensive, higher recall) or only entities likely to participate in relational events (focused, higher precision)? Comprehensive extraction feeds the MentionIndex for retroactive promotion; focused extraction reduces noise. Start comprehensive — the promotion pipeline already filters on relational weight.

4. **Heuristic implication quality**: The EventKind × Role → ImplicationType mapping (C.4) is a hand-authored table. How much of the relational weight principle does it actually capture? Measure against manually annotated examples. If heuristic quality is <70% on realistic text, prioritize the ML relation classifier (C.4b).

5. **Model directory layout**: **RESOLVED.** Models deployed to `$STORYTELLER_DATA_PATH/models/event_classifier/` — consistent with `CharacterPredictor` which uses the adjacent `$STORYTELLER_DATA_PATH/models/` directory. Both `STORYTELLER_DATA_PATH` and `STORYTELLER_MODEL_PATH` are defined in `.env`.

6. **Shared vs. separate tokenizers**: **RESOLVED.** Both models use the same DistilBERT tokenizer. A single `tokenizer.json` is deployed alongside the ONNX models. In Rust, `Tokenizer` is `Send + Sync` — load once, share across both inference sessions.

7. **Zero-shot bootstrapping value**: **RESOLVED — skipped.** GLiNER/NLI integration adds overhead for a path we'd replace within sessions. We have actionable, better fit-for-purpose models via the fine-tuning pipeline. Go directly to fine-tuned models.

8. **CI/MLOps for large ONNX models** *(new, from C.2)*: The character prediction ONNX model was <1MB and could be committed to the repo for test fixtures. The event classifier models are ~268MB and ~266MB — far too large for git (even with LFS, this is unwieldy for CI). This creates a gap between local dev (where models are available) and CI (where they aren't).

   **Implications**:
   - **Local dev/test**: Works today — models at `$STORYTELLER_DATA_PATH/models/event_classifier/`, feature-gated tests (`test-ml-model`) load from disk.
   - **CI gap**: No model artifacts available in CI. Feature-gated tests that require ONNX models will not run in CI until we solve model distribution.

   **Options to evaluate**:
   - **(a) Noop/mock inference in CI** (recommended near-term): A pluggable `EventClassifier` trait with a `MockClassifier` that returns fixed outputs. Tests verify pipeline wiring without real model inference.
   - **(b) Model registry**: Store models in S3/GCS/HuggingFace Hub, pull on demand in CI. Adds infrastructure complexity but enables real inference in CI.
   - **(c) Quantized/distilled test models**: Train a tiny model (~5MB) specifically for CI — not for quality, just for shape/pipeline validation.
   - **(d) Git LFS**: Technically possible but poorly suited for models this size in a multi-developer workflow.

   **Decision**: Start with (a) — the `EventClassifier` struct in C.3 should be designed with a trait boundary that enables mock implementations. Revisit (b)/(c) when CI model validation becomes critical.

## Sequencing

For a single developer, the recommended order within Phase C:

1. **C.0** (infrastructure) — ✅ Complete. `tokenizers` crate integrated, `EventClassifier` module scaffolded.
2. **C.1** (training data) — ✅ Complete. 8,000 examples across all 8 EventKinds with entity annotations.
3. **C.2** (model training + ONNX export) — ✅ Complete. DistilBERT models trained (F1=1.0), ONNX exported and validated, deployed to `$STORYTELLER_DATA_PATH`.
4. **C.3** (EventClassifier in Rust) — Next. Real ONNX models now available. The tokenizer integration (C.0) and the `CharacterPredictor` pattern provide the template. Design with trait boundary for CI mock (see Open Question 8).
5. **C.4** (implication inference) — 1 session. Heuristic mapping is straightforward. The integration test (text → atoms → weight → promotion) is the validation.
6. **C.5** (pipeline integration) — 1 session. Wiring. The classify_and_extract() function and play_scene_context binary update.
7. **C.6** (evaluation) — ongoing. Not a single session — build the evaluation tooling early (after C.2) and use it throughout.

**Estimated remaining**: 3-5 sessions for C.3–C.6, with C.3 as the critical next step now that trained ONNX models are available.

## Verification

At the end of Phase C:

```
cargo check --all-features         # Compiles with tokenizers dependency
cargo test --workspace              # All existing tests pass + new unit tests
cargo clippy --all-targets --all-features  # No warnings
cargo fmt --check                   # Formatted

# Feature-gated
cargo test --workspace --features test-ml-model  # EventClassifier loads model,
                                                  # classifies test inputs,
                                                  # produces valid EventAtoms
```

The integration test that validates the relational weight principle end-to-end:

```
"I pick up the ancient stone" →
  EventClassifier.classify_text() →
    EventKind::ActionOccurrence, entities: [player=Actor, stone=Target] →
      infer_implications_heuristic() → [Possession(player→stone, 0.5), Attention(player→stone, 0.3)] →
        compute_relational_weight(stone, ...) → RelationalWeight { total: 0.8, ... } →
          determine_promotion_tier(...) → PromotionTier::Referenced (or higher with player multiplier)
```

Text in, promotion decision out. The pipeline has a single purpose: **make entities real through their relationships.**
