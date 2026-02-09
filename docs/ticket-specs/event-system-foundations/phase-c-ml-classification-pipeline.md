# Phase C: ML Classification Pipeline

**Status**: Not started
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

### C.0: Infrastructure — `tokenizers` Crate Integration

**Goal**: Add the `tokenizers` crate to the workspace and verify it loads HuggingFace tokenizer files.

**Why first**: Every subsequent work item depends on tokenization. Getting this working (and understanding its API surface — offset mapping, word IDs, subword handling) before writing model code eliminates a class of integration surprises.

**Outputs**:
- `tokenizers` added to `[workspace.dependencies]` in root `Cargo.toml`
- `tokenizers = { workspace = true }` in `storyteller-engine/Cargo.toml`
- Small test in engine crate: load a tokenizer.json, tokenize sample text, verify token IDs and offset mappings

**Notes**:
- The `tokenizers` crate IS the reference implementation — Python's `tokenizers` is a binding to this Rust code. API documentation is excellent.
- Tokenizer files (`tokenizer.json`) are exported alongside ONNX models and should live in the same model directory. We make use of STORYTELLER_MODEL_PATH for this in .env - by default set to our workspace root sibling directory `storyteller-data/models`
- Key APIs: `Tokenizer::from_file()`, `encoding.get_ids()`, `encoding.get_offsets()`, `encoding.get_word_ids()` (maps subword tokens back to word indices — critical for NER span extraction).

**Verification**: `cargo check --all-features`, basic tokenization test passes.

---

### C.1: Training Data Generation — Event Classification

**Goal**: Build the combinatorial training data pipeline for event classification. Follows the pattern established in `storyteller-ml/src/matrix/` — parameterized templates, combinatorial expansion, JSONL output.

**Outputs**:
- New directory: `training/event_classifier/` — Python package (same uv/hatch pattern as `training/`)
- New module: `storyteller-ml/src/event_templates/` — Rust template definitions and combinatorial expansion
- JSONL training data: `$STORYTELLER_DATA_PATH/training/event_classification/`

**Training data format** (one JSON object per line):

```json
{
  "text": "I pick up the ancient stone from the riverbed",
  "register": "player",
  "event_kinds": ["ActionOccurrence"],
  "action_type": "Perform",
  "entities": [
    {"span": [0, 1], "text": "I", "category": "CHARACTER", "role": "Actor"},
    {"span": [5, 8], "text": "the ancient stone", "category": "OBJECT", "role": "Target"},
    {"span": [9, 11], "text": "the riverbed", "category": "LOCATION", "role": "Location"}
  ]
}
```

**Template structure** (Rust, in storyteller-ml):

Templates per EventKind, parameterized by entities, verbs, modifiers. Two registers per template: player-input (imperative, short) and narrator-prose (literary, past tense).

```
EventKind::ActionOccurrence:
  player: "I {verb} the {object}" / "I try to {verb} {object_with_article}"
  narrator: "{character} {verb}ed the {object} {adverb}" / "With {manner}, {character} {verb}ed..."

EventKind::SpeechAct:
  player: "I ask {character} about {topic}" / "I tell {character} that {statement}"
  narrator: "'{dialogue},' {character} said {adverb}" / "{character} whispered something to {other}"

EventKind::EmotionalExpression:
  player: (rare — players describe actions, not emotions directly)
  narrator: "Tears welled in {character}'s eyes" / "A {emotion_adj} look crossed {character}'s face"
```

Each template category needs ~20-50 verb/noun/modifier variants. Combinatorial expansion (verbs × characters × objects × modifiers × register) produces ~500-1,000 examples per EventKind from a modest template set.

**LLM augmentation** (optional, parallel track): Feed template skeletons to Ollama for naturalistic literary variations. Same pattern as character prediction training data. Produces higher-quality narrator-register examples.

**Entity annotation is programmatic**: Since we generate the text, we know where the entities are. Span offsets are computed during generation, not hand-annotated.

**Multi-label**: A single clause can express multiple EventKinds ("Sarah storms out" = SpatialChange + EmotionalExpression). Templates should include multi-label examples.

**Tests**: Generated JSONL validates against schema (EventKind values are valid, span offsets are within text bounds, entity categories are from the vocabulary).

---

### C.2: Model Fine-Tuning and ONNX Export

**Goal**: Fine-tune a pre-trained transformer encoder on the generated training data and export to ONNX.

**Outputs**:
- Python training script: `training/event_classifier/src/event_classifier/train.py`
- ONNX model(s): `$STORYTELLER_DATA_PATH/models/event_classifier/`
- Tokenizer file: `$STORYTELLER_DATA_PATH/models/event_classifier/tokenizer.json`

**Starting model**: DeBERTa-v3-small (44M params, ~180MB ONNX). Best accuracy/size tradeoff per the research document. If ONNX export issues arise (DeBERTa has historically had quirks), fall back to DistilBERT (66M params, well-tested ONNX path).

**Task 1 — Event classification (sequence classification)**:
- Input: tokenized text
- Output: 10-dim sigmoid logits (multi-label, one per EventKind)
- Loss: binary cross-entropy
- Metric: per-class F1, macro F1

**Task 2 — Entity extraction (token classification)**:
- Input: tokenized text
- Output: per-token BIO labels for 7 categories (CHARACTER, OBJECT, LOCATION, GESTURE, SENSORY, ABSTRACT, COLLECTIVE) = 15 labels (7×B + 7×I + O)
- Loss: cross-entropy with class weights (O tokens dominate)
- Metric: entity-level F1 (span-exact match)

**Task 3 — Participant role labeling (token classification)** — *deferred to C.2b*:
- Depends on having both events and entities extracted. Can be trained as a follow-up once C.2a stabilizes.
- In the interim, participant roles are inferred heuristically from entity position and EventKind (C.4).

**Training approach (Approach B — separate models)**:
1. Fine-tune one model for event classification (sequence-level [CLS] head)
2. Fine-tune one model for entity extraction (token-level BIO head)
3. Both use the same base encoder (DeBERTa-v3-small) but are trained independently
4. Export each to ONNX via HuggingFace Optimum: `optimum-cli export onnx --optimize O2`

**Why separate initially**: Easier to debug. Easier to evaluate task quality independently. Easier to regenerate training data for one task without retraining the other. Multi-task consolidation (Approach A) is a future optimization when both tasks are stable.

**Export validation**: Compare PyTorch vs. onnxruntime output for numerical consistency (same pattern as `training/src/training/export.py`).

**Python dependencies** (new in `training/event_classifier/pyproject.toml`):
- `transformers>=4.38.0` (HuggingFace model loading)
- `datasets>=2.18.0` (data loading utilities)
- `optimum[exporters]>=1.17.0` (ONNX export)
- `torch>=2.2.0`, `onnx`, `onnxruntime` (shared with character training)

**Tests**: Exported ONNX models load in onnxruntime, produce expected output shapes, inference matches PyTorch within tolerance.

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

1. **DeBERTa ONNX export stability**: DeBERTa-v3 has had intermittent ONNX export issues (HuggingFace transformers #35545). If export fails, DistilBERT is the immediate fallback. ModernBERT (Dec 2024) is the aspirational replacement when its ONNX path stabilizes.

2. **Training data volume calibration**: The research document estimates 500-1,000 examples per class. The combinatorial approach generates this easily, but the question is whether template-generated data is diverse enough. LLM augmentation increases diversity at the cost of annotation accuracy (LLM-generated text needs span re-annotation). Start with templates only; add LLM augmentation if quality plateaus.

3. **Entity extraction scope**: Should the NER model extract ALL entity mentions (comprehensive, higher recall) or only entities likely to participate in relational events (focused, higher precision)? Comprehensive extraction feeds the MentionIndex for retroactive promotion; focused extraction reduces noise. Start comprehensive — the promotion pipeline already filters on relational weight.

4. **Heuristic implication quality**: The EventKind × Role → ImplicationType mapping (C.4) is a hand-authored table. How much of the relational weight principle does it actually capture? Measure against manually annotated examples. If heuristic quality is <70% on realistic text, prioritize the ML relation classifier (C.4b).

5. **Model directory layout**: Where do ONNX models and tokenizer files live? Options:
   - `$STORYTELLER_DATA_PATH/models/event_classifier/` (with other data)
   - `$STORYTELLER_MODEL_PATH/event_classifier/` (separate model path)
   - Embedded in the binary (impractical for 180MB models)

   Follow the existing pattern from `CharacterPredictor` which uses `STORYTELLER_MODEL_PATH` or `STORYTELLER_DATA_PATH`.

6. **Shared vs. separate tokenizers**: If we start with separate models (Approach B), they each need a tokenizer. If both use DeBERTa-v3-small, the tokenizer is identical. Load it once and share. The `tokenizers::Tokenizer` type is `Send + Sync`.

7. **Zero-shot bootstrapping value**: **RESOLVED — skipped.** GLiNER/NLI integration adds overhead for a path we'd replace within sessions. We have actionable, better fit-for-purpose models via the fine-tuning pipeline. Go directly to fine-tuned models.

## Sequencing

For a single developer, the recommended order within Phase C:

1. **C.0** (infrastructure) — half a session. Get `tokenizers` working. Small victory, unblocks everything.
2. **C.1** (training data) — 1-2 sessions. Creative/generative work. The combinatorial template system is the durable artifact.
3. **C.2** (model training + ONNX export) — 1-2 sessions. Depends on C.1 output. Python-heavy. Export validation is the critical gate.
4. **C.3** (EventClassifier in Rust) — 1-2 sessions. Can scaffold against test fixtures before real models arrive from C.2. The tokenizer integration (C.0) and the `CharacterPredictor` pattern provide the template.
5. **C.4** (implication inference) — 1 session. Heuristic mapping is straightforward. The integration test (text → atoms → weight → promotion) is the validation.
6. **C.5** (pipeline integration) — 1 session. Wiring. The classify_and_extract() function and play_scene_context binary update.
7. **C.6** (evaluation) — ongoing. Not a single session — build the evaluation tooling early (after C.2) and use it throughout.

**C.1 and C.3 can partially overlap**: The Rust scaffolding for EventClassifier (struct, load, tokenize, post-processing) can be built with mock model outputs while C.1-C.2 produce real training data and models. The integration point is when real ONNX models slot into the scaffold.

**Estimated total**: 5-8 sessions, with the Python training pipeline (C.1-C.2) and Rust inference (C.3-C.5) as two parallel tracks joined at the ONNX model boundary.

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
