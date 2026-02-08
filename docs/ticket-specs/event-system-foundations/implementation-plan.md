# Implementation Plan — Event System Foundations

## Purpose

This document sequences the implementation work for the event grammar and entity reference model into phased work items. Each phase has clear inputs, outputs, and dependencies. Investigation areas are marked explicitly — these are places where the right approach is not yet known and experimentation is needed before committing to an implementation.

The plan builds incrementally on existing working code. No phase breaks the ML prediction pipeline, the context assembly system, or the existing turn cycle.

### The Relational Weight Principle as Workflow Driver

The implementation workflow is driven by a single organizing principle: **events create relationships, and relationships create entities that matter.** This is not just a theoretical insight — it determines what we build and in what order:

1. **Types first** (Phase A): Define the vocabulary — event atoms, entity references, relational implications. The type system encodes the principle: an `EventAtom` carries `RelationalImplication` instances that connect `Participant` entities. Without implications, an event is scene texture.

2. **Weight computation next** (Phase B): Before building classifiers, establish *what classification is for* — computing relational weight from events and making promotion decisions. This ensures that every subsequent phase has a clear success criterion: does the classified output produce meaningful relational weight?

3. **ML classification pipeline** (Phase C): Build the training data generation → model fine-tuning → ONNX inference pipeline for event classification, entity extraction, and relation inference. The classifier's purpose is not taxonomy for its own sake — it's discovering the relationships that give entities weight. See `classifying-events-and-entities.md` for the ML approach.

4. **Turn-unit extraction** (Phase D): Extend the same classifier to handle Narrator prose. The same model handles both registers (player input and literary prose).

5. **Composition** (Phase E): Detect compound events where the combination carries more relational weight than the sum of its parts.

Every phase serves the relational weight principle. Classification feeds extraction, extraction feeds relation inference, relation inference feeds weight accumulation, weight accumulation drives entity promotion. The pipeline has a single purpose: **make entities real through their relationships.**

---

## Phase Overview

```
Phase A: Event Grammar Types         ← pure types, no behavior
    │
    ▼
Phase B: Event-Entity Bridge         ← relational weight, promotion logic
    │
    ├─── (B validates the weight principle; C builds the ML pipeline that feeds it)
    ▼
Phase C: ML Classification Pipeline  ← training data + model + ONNX inference → EventAtom
    │
    ▼
Phase D: Turn-Unit Extraction        ← committed turn → events (same classifier, prose register)
    │
    ▼
Phase E: Event Composition           ← compound events from sequential atoms
    │
    ▼
Phase F: Revised event-system.md     ← documentation update
```

### Relationship to Other Documents

| Document | Role |
|---|---|
| `event-grammar.md` | Defines what an event *is* — types, composition, alignment with existing types |
| `entity-reference-model.md` | Defines how entities are referenced, resolved, promoted through events |
| `classifying-events-and-entities.md` | Explores ML/NLP approaches to classification — the technical foundation for Phases C and D |
| This document | Sequences the work, identifies dependencies and investigation areas |

---

## Phase A: Event Grammar Types

**Goal**: Define the complete type system for events, entity references, and their relationships. Pure type definitions with derives, constructibility tests, and serialization round-trip tests. No behavioral code.

**Depends on**: Nothing. Can start immediately.

**Inputs**: Type sketches from `event-grammar.md` and `entity-reference-model.md`, existing types in `storyteller-core/src/types/`.

**Outputs**:
- New file: `storyteller-core/src/types/event_grammar.rs`
- Extended file: `storyteller-core/src/types/entity.rs` (adds `EntityRef`, `ReferentialContext`, `PromotionTier`, `RelationalWeight`)
- Updated file: `storyteller-core/src/types/event.rs` (typed `NarrativeEvent` payload + `TurnId`)
- Updated file: `storyteller-core/src/types/message.rs` (migration annotations for `TurnId`)
- Module registration in `storyteller-core/src/types/mod.rs`

### Work Items

**A.1: EventAtom and EventKind**

New types in `event_grammar.rs`:
- `EventAtom` — the minimal event unit
- `EventKind` — semantic event taxonomy (10 variants)
- `SceneLifecycleType`, `EntityLifecycleType` — lifecycle subtypes
- Tests: constructibility for each `EventKind` variant, serde round-trip

Follows existing conventions:
- Derives: `Debug, Clone, serde::Serialize, serde::Deserialize`
- Ordered enums get `PartialOrd, Ord` where useful
- Copy types get `Copy` where the type is small enough
- `PartialEq, Eq` on all enums

**A.2: Participant and ParticipantRole**

New types in `event_grammar.rs`:
- `Participant` — entity reference + role
- `ParticipantRole` — 6-variant enum (Actor, Target, Instrument, Location, Witness, Subject)
- Tests: exhaustiveness, constructibility

**A.3: RelationalImplication and ImplicationType**

New types in `event_grammar.rs`:
- `RelationalImplication` — source, target, type, weight
- `ImplicationType` — 9-variant enum (Possession, Proximity, Attention, EmotionalConnection, TrustSignal, InformationSharing, Conflict, Care, Obligation)
- Tests: weight range validation, constructibility

**A.4: EventSource, EventConfidence, ClassifierRef**

New types in `event_grammar.rs`:
- `EventSource` — 5-variant enum (PlayerInput, TurnExtraction, ConfirmedPrediction, System, Composed)
- `ClassifierRef` — name + version
- `EventConfidence` — value + evidence
- `ConfidenceEvidence` — 5-variant enum
- Tests: confidence range validation, constructibility

**A.5: CompoundEvent and CompositionType**

New types in `event_grammar.rs`:
- `CompoundEvent` — ordered atoms with composition type
- `CompositionType` — 4-variant enum (Causal, Temporal, Conditional, Thematic)
- Tests: constructibility, serde round-trip

**A.6: EntityRef and ReferentialContext**

New types in `entity.rs` (extending the existing file):
- `EntityRef` — 3-variant enum (Resolved, Unresolved, Implicit)
- `ReferentialContext` — descriptors, spatial, possessor, prior mentions
- `EntityRef::entity_id()` and `EntityRef::is_resolved()` methods
- Tests: resolution state checks, constructibility

**A.7: PromotionTier and RelationalWeight**

New types in `entity.rs`:
- `PromotionTier` — 5-variant ordered enum (Unmentioned through Persistent)
- `RelationalWeight` — accumulator struct
- `UnresolvedMention` — lightweight ledger record
- `EntityBudget` — scene-level tracking budget
- Tests: tier ordering, weight accumulation

**A.8: NarrativeEvent Typed Payload**

Update `event.rs`:
- Add `EventPayload` enum (Atom, Compound, Untyped)
- Migrate `NarrativeEvent::payload` from `serde_json::Value` to `EventPayload`
- Keep `Untyped` variant for migration compatibility
- Tests: both typed and untyped payload construction, serde round-trip

**A.9: Turn Types (TurnId, Turn, TurnState)**

New types — `TurnId` in `event.rs` (alongside `EventId`, follows identical UUID v7 pattern) and `Turn`, `TurnState` in `event_grammar.rs`:

- `TurnId` — UUID v7 newtype with same derives as `EventId`/`SceneId` (Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize), `new()` and `Default`
- `Turn` — first-class turn representation: `id: TurnId`, `scene_id: SceneId`, `turn_number: u32` (ordinal), `state: TurnState`, `created_at`, `rendered_at`, `committed_at` timestamps
- `TurnState` — three-state lifecycle enum: `Hypothesized`, `Rendered`, `Committed`
- `CommittedTurn` — the extraction bundle: `turn_id: TurnId`, `scene_id: SceneId`, `turn_number: u32`, narrator prose, player response, prediction metadata, extraction flag

Migration annotations:
- `EventAtom` gains `turn_id: Option<TurnId>` — `Option` because system events (scene lifecycle) may not belong to a specific turn
- `EventSource::TurnExtraction` uses `turn_id: TurnId` instead of `turn_number: u32`
- `ReferentialContext.first_mentioned_turn` migrates from `u32` to `TurnId`
- `UnresolvedMention` gains `turn_id: TurnId`
- Existing `PlayerInput.turn_number` and `TurnPhase.turn_number` in `message.rs` will need migration — add `turn_id: TurnId` as the authoritative identifier, preserve `turn_number` as human-readable ordinal

Bevy integration note: The active `Turn` will become a Bevy Resource, replacing the current `turn_number: u32` tracking in `PlayerInput` and `TurnPhase`. This is a Phase D concern (turn lifecycle management) but the types are defined here so that Phase A establishes the vocabulary.

Tests: TurnId generation and ordering, Turn construction, TurnState transitions, CommittedTurn construction, serde round-trip

**Verification**: `cargo check --all-features`, `cargo test --all-features`, `cargo clippy --all-features`, `cargo fmt --check`. All existing tests continue to pass. New types have constructibility and serde round-trip tests.

---

## Phase B: Event-Entity Bridge

**Goal**: Implement the computational logic that connects events to entity promotion. Given a stream of events involving an entity reference, compute relational weight and determine promotion tier.

**Depends on**: Phase A (types must exist).

**Outputs**:
- New file: `storyteller-core/src/types/promotion.rs` or functions in `entity.rs`
- Possibly new file: `storyteller-engine/src/systems/entity_promotion.rs` (if Bevy system integration is appropriate)

### Work Items

**B.1: Relational Weight Computation**

Given a set of `EventAtom` instances involving an `EntityRef`, compute the accumulated `RelationalWeight`:

```rust
fn compute_relational_weight(
    entity: &EntityRef,
    events: &[EventAtom],
    player_entity_id: EntityId,
) -> RelationalWeight {
    // Sum relational implication weights
    // Count distinct events
    // Count distinct relationship partners
    // Separate player-interaction weight
}
```

Tests: weight computation for various event patterns (no implications = zero weight, player interaction = non-zero player weight, multiple implications accumulate).

**B.2: Promotion Tier Determination**

Given a `RelationalWeight`, determine the appropriate `PromotionTier`:

```rust
fn determine_promotion_tier(
    weight: &RelationalWeight,
    current_tier: PromotionTier,
    entity_origin: EntityOrigin,
    config: &PromotionConfig,
) -> PromotionTier {
    // Apply thresholds from config
    // Respect authored overrides
    // Apply player-interaction override
    // Never demote below authored tier
}
```

`[PLACEHOLDER]` Threshold values in `PromotionConfig` are initial guesses. They will be calibrated from play data.

Tests: promotion for each transition, player override, authored entity protection against demotion.

**B.3: EntityRef Resolution**

Given an `EntityRef::Unresolved`, attempt to resolve it against known tracked entities:

```rust
fn resolve_entity_ref(
    unresolved: &EntityRef,
    tracked_entities: &[TrackedEntity],
    scene_context: &SceneContext,
) -> Option<EntityId> {
    // Try possessive resolution
    // Try spatial resolution
    // Try anaphoric resolution
    // Try descriptive resolution
    // Return None if ambiguous
}
```

This is the rule-based resolution pipeline. It may be extended with ML resolution later (Phase C investigation).

Tests: each resolution strategy independently, ambiguous cases return None, chained resolution (possessive + spatial).

**B.4: Retroactive Promotion**

When an entity is promoted, walk back through the event ledger and resolve prior `Unresolved` mentions:

```rust
fn retroactively_promote(
    new_entity_id: EntityId,
    mention: &UnresolvedMention,
    ledger: &mut EventLedger,
) -> Vec<EventId> {
    // Find all prior mentions matching this entity
    // Resolve their EntityRef to the new EntityId
    // Return the list of updated event IDs
}
```

`[INVESTIGATION NEEDED]` Whether this mutates ledger entries in place or creates resolution overlay events. See `entity-reference-model.md` for the three options. Start with option 2 (separate mention index) for simplicity.

Tests: retroactive resolution finds prior mentions, resolution doesn't affect unrelated entities.

**Verification**: All Phase A tests still pass. New behavioral tests cover weight computation, promotion, resolution, and retroactive promotion.

---

## Phase C: ML Classification Pipeline

**Goal**: Build the ML-powered classification pipeline that produces `EventAtom` instances from natural language. This replaces the naive keyword classifier with trained models following the same pattern as the character prediction pipeline (combinatorial training data → PyTorch fine-tuning → ONNX export → `ort` inference in Rust). The existing `classify_player_input()` remains as a fallback during development.

See `classifying-events-and-entities.md` for the full ML approach, model selection rationale, and training data generation strategy.

**Depends on**: Phase A (types), Phase B (entity resolution logic to validate outputs against).

**Outputs**:
- New directory: `training/event_classifier/` — Python training pipeline (templates, augmentation, fine-tuning, ONNX export)
- New file: `storyteller-ml/src/event_schema.rs` — feature encoding/decoding for event classification (mirrors `feature_schema.rs` pattern)
- New file: `storyteller-engine/src/inference/event_classifier.rs` — `EventClassifier` struct (ort Session, tokenizer, multi-task inference)
- Modified file: `storyteller-engine/src/context/prediction.rs` — integrate `EventClassifier` output alongside existing `classify_player_input()`
- New dependency: `tokenizers` crate (workspace-level)

### Work Items

**C.1: Training Data Generation**

Build the combinatorial training data pipeline for event classification, entity extraction, and relation classification. Follows the pattern established in `storyteller-ml/src/matrix/`:

- Template definitions per `EventKind` (10 classes × ~20-50 templates each)
- Entity type templates per NER category (7 categories × referential patterns × narrative contexts)
- Relation templates per `ImplicationType` (9 types × entity pair configurations)
- Combinatorial expansion producing ~5,000-10,000 labeled examples per task
- LLM augmentation (Ollama) for naturalistic literary-register variations
- Output: JSONL files in TACRED-like format (sentence + spans + labels)

Both registers represented: player-input style (imperative, short) and Narrator-prose style (literary, past tense). The combinatorial matrix includes register as an axis.

Tests: generated data validates against the type schemas (EventKind, entity categories, ImplicationType are all parseable).

**C.2: Model Fine-Tuning and ONNX Export**

Fine-tune DeBERTa-v3-small (or DistilBERT) on the generated training data. Two approaches, evaluated in order:

**Approach B (separate models, initial)**: Fine-tune independent models for each task:
1. Event classification: sequence classification (10 EventKind labels, multi-label sigmoid)
2. Entity extraction: token classification (7 NER categories, BIO tagging)
3. Participant role labeling: token classification (6 ParticipantRole labels per entity token)

Export each via HuggingFace Optimum: `optimum-cli export onnx --optimize O2`

**Approach A (multi-task, target)**: Single shared encoder with 3 heads (event, NER, role). Export as single ONNX model with named outputs (`event_logits`, `ner_logits`, `role_logits`). Matches the `CharacterPredictor` pattern.

Start with Approach B (validates each task independently), consolidate to Approach A when task definitions stabilize.

Tests: exported ONNX models load in `ort`, produce expected output shapes, inference matches PyTorch within tolerance.

**C.3: EventClassifier in Rust**

New `EventClassifier` struct in `storyteller-engine/src/inference/event_classifier.rs`, following the `CharacterPredictor` pattern:

```rust
pub struct EventClassifier {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    pool: rayon::ThreadPool,
}

impl EventClassifier {
    pub fn classify_text(
        &self,
        input: &str,
        scene_cast: &[TrackedEntity],
    ) -> StorytellerResult<ClassificationResult> {
        // 1. Tokenize (tokenizers crate)
        // 2. Build tensors [1, seq_len]
        // 3. Run inference (ort)
        // 4. Decode: EventKind from [CLS] logits, entity spans from NER logits,
        //    participant roles from role logits
        // 5. Resolve extracted entities against scene_cast
        // 6. Return ClassificationResult (atoms + entity refs + confidence)
    }
}
```

The `classify_text()` method returns a `ClassificationResult` containing:
- `EventAtom` instances (with EventKind, participants, confidence)
- `EntityRef` instances (resolved and unresolved)
- The existing `EventFeatureInput` (backward compatible with the character prediction pipeline)

The existing `classify_player_input()` → `EventFeatureInput` path is preserved as a fallback during development and for the character prediction pipeline which depends on it.

Tests: `EventClassifier` loads model and tokenizer, produces valid atoms from test inputs, known entity names resolve correctly, participant roles are assigned.

**C.4: Relational Implication Inference**

Given an `EventAtom`'s kind and participants, infer relational implications. This is the **critical bridge** between classification and the relational weight principle — without this step, classified events produce no entity promotions.

Two complementary approaches:

1. **Heuristic mapping** (fast, deterministic): EventKind + ParticipantRoles → ImplicationType mapping table. `SpeechAct(Actor=A, Target=B)` → `[Attention(A→B), InformationSharing(A→B)]`. Covers ~70% of cases.

2. **ML relation classifier** (higher accuracy): Entity Marker approach — insert `[E1]`/`[E2]` markers around entity pairs, classify with fine-tuned model. Covers the remaining ~30% including figurative language and complex constructions.

Start with heuristic mapping (C.4a) and add ML relation classification (C.4b) when training data is available.

```rust
fn infer_implications(
    kind: &EventKind,
    participants: &[Participant],
) -> Vec<RelationalImplication> {
    // Heuristic: EventKind + roles → known implication patterns
    // ML: entity marker classification for ambiguous cases
}
```

Tests: each EventKind produces appropriate implications, heuristic mapping covers all EventKind × ParticipantRole combinations, weights are in valid range. **Integration test**: classified event → implications → weight computation (Phase B) → promotion decision.

**C.5: Zero-Shot Bootstrapping (Optional, Parallel)**

Before fine-tuned models are available, validate the pipeline with zero-shot models:
- GLiNER for entity extraction (pass narrative entity type descriptions at inference time)
- NLI model (DeBERTa-v3-base-mnli) for relation classification (ImplicationType as hypothesis)

This produces lower-quality output but validates the full pipeline end-to-end: text → entities → events → implications → weight → promotion. The pipeline architecture is the same — only the models change when fine-tuned versions are ready.

**Verification**: Existing `classify_player_input()` tests still pass. ML prediction pipeline is unaffected (EventFeatureInput still produced). New tests cover the full relationship-discovery pipeline: text → classification → extraction → implication → weight validation.

---

## Phase D: Turn-Unit Extraction Pipeline

**Goal**: Build the turn-unit extraction pipeline — the system that processes a committed turn (Narrator prose + player response + prediction metadata) into `EventAtom` instances, infers their relational implications, and feeds the results into the entity promotion pipeline. The same `EventClassifier` from Phase C handles both player input and prose — the difference is preprocessing (clause segmentation for prose) and source tagging (`EventSource::TurnExtraction`).

**Depends on**: Phase A (types), Phase C (ML classifier and entity extraction). Phase D extends Phase C's classifier to the full turn unit.

**This phase has investigation elements** (clause segmentation quality for prose) but the fundamental approach is settled: post-hoc classification of committed turns using the unified ML classifier pipeline, with implication inference feeding the relational weight accumulator.

### Work Items

**D.1: Turn Lifecycle Integration**

The Turn types (`TurnId`, `Turn`, `TurnState`, `CommittedTurn`, `PredictionMetadata`) are defined in Phase A.9. This work item implements the *behavioral* logic:

- **Turn state machine**: Transition logic for `Hypothesized → Rendered → Committed`, with the rejection path `Rendered → Hypothesized` (X-card / content concern). Guard against invalid transitions (e.g., `Hypothesized → Committed` without rendering).
- **Turn creation**: Scene entry creates the first `Turn` (state = `Hypothesized`). Each player response commits the current turn and creates the next.
- **Bevy Resource**: Register `Turn` as a Bevy Resource representing the active turn. `TurnPhase` and `TurnPhaseKind` (in `message.rs`) track pipeline progress within the active turn. Migrate `TurnPhase.turn_number: u32` → `TurnPhase.turn_id: TurnId`.
- **Idempotency guard**: `CommittedTurn.events_extracted` flag prevents re-processing. Extracting from an already-extracted turn is a no-op.

Tests: state transitions (valid and invalid), turn creation at scene entry, idempotency (extracting from an already-extracted turn is a no-op), Bevy Resource lifecycle.

**D.2: Narrator Prose Classification**

Use the Phase C `EventClassifier` on Narrator prose. The same `classify_text()` method handles both registers — the model was trained on both player-input and literary-prose examples (see C.1 training data generation). Prose classification adds a preprocessing step:

1. **Clause segmentation**: Split Narrator prose into clauses/sentences (rule-based: sentence boundaries, coordination conjunctions, semicolons). Each clause becomes an independent classification input.
2. **Per-clause classification**: `EventClassifier.classify_text()` on each clause → EventKind, entity refs, participant roles.
3. **Entity resolution across clauses**: Entities extracted from earlier clauses inform resolution of later clauses within the same turn. Coreference operates within the turn's text with the scene cast as constraint.
4. **Source tagging**: All resulting atoms tagged with `EventSource::TurnExtraction { turn_id }`.
5. **Implication inference**: Each atom's relational implications computed (C.4), feeding into the weight accumulator.

`[INVESTIGATION NEEDED]` Whether clause-level classification loses too much context for literary prose. Starting point: clause-level (simpler, reuses Phase C classifier directly). Measure: classification accuracy on manually annotated prose passages. Fallback: paragraph-level encoding with per-clause attention windows.

Tests: known prose patterns produce correct atoms (described flinch → `EmotionalExpression`, described movement → `SpatialChange`, described speech → `SpeechAct`). **Integration test**: prose → atoms → implications → weight accumulation → entity promotion decisions.

**D.3: Prediction Confirmation**

Cross-reference extracted atoms against ML prediction metadata. When a turn-extracted atom matches a prediction (same character, same action type), the atom's source is upgraded to `EventSource::ConfirmedPrediction` and its confidence is boosted:

```rust
fn confirm_predictions(
    extracted_atoms: &[EventAtom],
    predictions: &[PredictionMetadata],
) -> Vec<EventAtom> {
    // For each extracted atom, check if a prediction hypothesized it
    // If match: upgrade source to ConfirmedPrediction, boost confidence
    // If no match: keep as TurnExtraction (Narrator added something new)
}
```

This provides a confidence signal: events that were both predicted and rendered are higher-confidence than events that appeared only in prose.

Tests: predicted-and-rendered atoms have higher confidence than unpredicted atoms, unmatched predictions don't produce phantom events.

**D.4: Turn Extraction Integration**

Wire the full `extract_turn_events()` pipeline: player classification + prose classification + prediction confirmation + deduplication. Integrate with the event ledger (atoms are committed) and the promotion pipeline (Phase B).

Tests: end-to-end — committed turn → atoms → ledger. Workshop scene data provides test fixtures. Feature-gated tests (`test-llm`) that run real Narrator output through the pipeline.

**D.5: X-Card / Rejection Flow**

Implement the rejection path: when a player rejects a rendering, the turn transitions from `Rendered` back to `Hypothesized` (predictions are preserved, prose is discarded). The Storykeeper adds constraints and the Narrator re-renders.

Tests: rejected turn produces no events, re-rendered turn can be committed normally.

**Verification**: Existing tests pass. New tests cover turn lifecycle, prose classification, prediction confirmation, deduplication, and the rejection flow.

---

## Phase E: Event Composition

**Goal**: Detect compound events from sequential atoms. Build the composition pipeline.

**Depends on**: Phase C (atoms are being produced from player input), ideally Phase D (atoms from Narrator output, but composition can start with player+prediction atoms alone).

**Outputs**:
- New file: `storyteller-engine/src/context/event_composition.rs` (or similar location)
- CompoundEvent detection logic

### Work Items

**E.1: Temporal Composition Detection**

The simplest composition type — atoms within the same turn or adjacent turns with participant overlap:

```rust
fn detect_temporal_compositions(
    atoms: &[EventAtom],
    window_turns: u32,
) -> Vec<CompoundEvent> {
    // Group atoms by participant overlap within temporal window
    // Create Temporal CompoundEvent for groups of 2+ atoms
}
```

Tests: adjacent atoms with shared participants compose, distant atoms do not.

**E.2: Causal Composition Detection**

Detect known causal patterns:

```rust
fn detect_causal_compositions(
    atoms: &[EventAtom],
) -> Vec<CompoundEvent> {
    // Pattern: Examine → EmotionalExpression (saw something → reacted)
    // Pattern: SpeechAct → RelationalShift (said something → relationship changed)
    // Pattern: ActionOccurrence → SpatialChange (did something → moved)
}
```

`[PLACEHOLDER]` The causal pattern library needs expansion from play data. Initial implementation should include 5-10 common patterns and provide a mechanism for adding more.

Tests: known causal patterns are detected, unrelated atoms are not falsely composed.

**E.3: Emergent Weight Calculation**

Compute the emergent relational implications of a compound event:

```rust
fn compute_emergent_implications(
    compound: &CompoundEvent,
    atom_implications: &[Vec<RelationalImplication>],
) -> Vec<RelationalImplication> {
    // Apply composition multiplier to summed weights
    // Generate new implications not present in individual atoms
    // (e.g., the causal link itself is a new implication)
}
```

`[PLACEHOLDER]` Composition multipliers (1.5x for causal, 1.0x for temporal, etc.) are initial guesses.

Tests: compound weight exceeds (or equals) sum of atom weights depending on composition type.

**E.4: Composition Pipeline Integration**

Wire the composition detector into the event processing pipeline. After atoms are produced (from classifier, predictions, and Narrator extraction), run composition detection and emit compound events.

Tests: end-to-end — player input → atoms → composition detection → compound events with correct types and weights.

**Verification**: Existing tests pass. New tests cover each composition type and the end-to-end pipeline.

---

## Phase F: Revised event-system.md

**Goal**: Update the existing `docs/technical/event-system.md` to reflect the narrator-centric architecture and incorporate the event grammar, entity reference model, and revised classification pipeline.

**Depends on**: All previous phases (to know what was actually built vs. what was planned).

**Outputs**:
- Updated file: `docs/technical/event-system.md`

### Work Items

**F.1: Architecture Revision**

Update the document to reflect the narrator-centric architecture:
- Replace multi-agent references with the single-Narrator model
- Update the turn cycle diagram with ML prediction, rules engine, and context assembly stages
- Revise agent subscription model (Character Agents are no longer LLM agents)

**F.2: Event Grammar Integration**

Incorporate the event grammar into the spec:
- Replace the `NarrativeEvent` pseudo-code with `EventAtom` and `EventKind`
- Update the truth set section with typed propositions
- Integrate the relational weight principle

**F.3: Entity Reference Integration**

Add entity reference and promotion to the spec:
- New section on entity lifecycle driven by events
- Retroactive promotion mechanism
- Entity budget per scene

**F.4: Mark Superseded Sections**

The original event-system.md was designed for the multi-agent architecture. Some sections are fully superseded (agent subscriptions for Character Agents as LLM agents), others are modified (truth set structure), and some are unchanged (priority tiers, cascade handling). Mark each clearly.

**Verification**: Document review for internal consistency. Type sketches in the revised document match the actual implemented types.

---

## Cross-Phase Concerns

### Testing Strategy

All phases follow the project's test tier conventions:
- **Unit tests** (default `cargo test`): Type constructibility, serde round-trip, weight computation, promotion logic, composition detection.
- **Feature-gated integration tests** (`test-ml-model`): End-to-end tests that run the ONNX model and verify event atom production from real predictions.
- **Feature-gated LLM tests** (`test-llm`): Turn extraction pipeline tests (Phase D) that require running Ollama to produce real Narrator prose for extraction.

### Scene Gravity as Downstream Benefit

The Turn → Scene hierarchy enables post-hoc scene gravity computation. Once events are being extracted and committed against turns (Phase D), the system can compute:

- **Turn density**: Events extracted per turn, weighted by relational implications. High-density turns (many entity promotions, substrate shifts) indicate narrative intensity.
- **Scene gravity**: Aggregate turn density across a scene. A scene with many high-density turns has higher actual gravity than its authored mass alone.
- **Attractor refinement**: Feed computed gravity back into the narrative graph, refining the authored gravitational landscape with empirical play data.

This is not a deliverable of any specific phase — it emerges naturally from the Turn/Scene/Event hierarchy once events flow through the system. It belongs to the Storykeeper's context assembly responsibilities (when computing which scenes to retrieve for Tier 3 context) and to the narrative graph's gravitational model.

### Existing Code Preservation

Each phase is designed to be additive:
- Phase A adds new types without modifying existing ones (except the `NarrativeEvent` payload).
- Phase B adds new logic without changing existing functions.
- Phase C extends `classify_player_input()` without changing its signature — the new path runs alongside the existing one.
- Phase D builds on Phase C's classifier and extends it for prose — additive, not replacing.
- Phase E adds a new pipeline stage without changing existing stages.
- Phase F is documentation only.

The one breaking change is `NarrativeEvent::payload` (Phase A.8), which changes from `serde_json::Value` to `EventPayload`. The `EventPayload::Untyped` variant preserves backward compatibility for any code that constructs `NarrativeEvent` with raw JSON.

### `[PLACEHOLDER]` Summary

The following values need calibration from play data:

| Value | Initial Guess | Phase | Description |
|---|---|---|---|
| `TRACKING_THRESHOLD` | 0.5 | B | Relational weight for `Referenced` → `Tracked` |
| `PERSISTENCE_THRESHOLD` | 2.0 | B | Relational weight for `Tracked` → `Persistent` |
| `MIN_PERSISTENCE_EVENTS` | 3 | B | Minimum events for persistence |
| `DEMOTION_SCENE_COUNT` | 3 | B | Scenes without events before demotion |
| `DEMOTION_TURN_COUNT` | 10 | B | Turns without events before scene-local demotion |
| Entity budget soft limit | 12-20 | B | Max tracked entities per scene |
| Composition multiplier (causal) | 1.5x | E | Weight amplification for causal composition |
| Composition multiplier (thematic) | 0.5x entity / 2.0x narrative mass | E | Weight for thematic composition |
| Max composition depth | 2 | E | Compounds contain atoms only, not other compounds |
| Causal pattern library | 5-10 patterns | E | Known causal event patterns |

### `[INVESTIGATION NEEDED]` Summary

The following areas require experimentation before committing to an approach. See `classifying-events-and-entities.md` for the ML research informing these decisions.

| Area | Phase | Question | Starting Point |
|---|---|---|---|
| Multi-task vs. separate models | C | Does multi-task training degrade individual task accuracy? | Start separate (Approach B), consolidate to multi-task (Approach A) when tasks stabilize |
| Prose clause segmentation | D | Does clause-level classification lose too much context for literary prose? | Start clause-level, measure quality on annotated passages |
| Training data register balance | C | Do player-input and Narrator-prose registers need separate models or just balanced training data? | Balanced training data with register as combinatorial axis |
| Relation extraction approach | C | Heuristic mapping vs. entity-marker ML classifier? | Start heuristic (C.4a), add ML (C.4b) when training data is available |
| Prediction confirmation matching | D | How to match extracted atoms to predictions (exact type match vs. fuzzy)? | Start with exact action-type match |
| Entity resolution | B | Rule-based vs. embedding-based? | Start rule-based with cast-list constraint, add embedding similarity for cross-scene resolution |
| Coreference scope | C/D | Per-turn vs. per-scene coreference window? | Per-turn with cast-list constraint (~80% resolution), extend to per-scene over Tier 2 journal |
| Ledger storage for unresolved mentions | B | Inline mutation vs. separate index vs. resolution overlay? | Start with option 2 (separate mention index) |
| Composition detection heuristics | E | What patterns indicate causal/conditional/thematic composition? | Start with 5-10 manual patterns |
| Zero-shot bootstrapping value | C | Is GLiNER/NLI useful enough to justify the zero-shot prototyping path? | Evaluate GLiNER entity quality on workshop scene text |

---

## Sequencing Recommendation

For a single developer working on this, the recommended sequence is:

1. **Phase A** — 2-3 sessions. Pure types, satisfying to write, establishes the vocabulary. Can be reviewed and merged independently.

2. **Phase B** — 2-3 sessions. The core logic. Weight computation and promotion are the conceptual heart of the system. Tests here validate the relational weight principle *before* we build classifiers — establishing what classification is for.

3. **Phase C** — 4-6 sessions (the largest phase, with Python and Rust work). Has two parallel tracks:
   - **C.1-C.2** (Python): Training data generation and model fine-tuning. Can start as soon as Phase A types stabilize. This is creative/generative work — defining templates, running augmentation, evaluating model quality.
   - **C.3-C.4** (Rust): `EventClassifier` struct and implication inference. Depends on C.2 producing ONNX models, but the Rust scaffolding (tokenizer loading, session management, post-processing) can be built against test fixtures before real models are available.
   - **C.5** (optional): Zero-shot bootstrapping with GLiNER/NLI — validates the pipeline end-to-end before fine-tuned models are ready. Can run in parallel with C.1-C.2.

4. **Phase D** — 2-3 sessions. Extends Phase C's classifier to the full turn unit. The same model handles prose — the difference is clause segmentation preprocessing and turn lifecycle management. The turn state machine and X-card flow are the main new concepts.

5. **Phase E** — 2-3 sessions. Composition detection. Operates on committed atoms from Phases C and D.

6. **Phase F** — 1 session. Documentation update. Best done after all implementation is settled.

Phases A-B form a foundational unit (types + weight computation). Phase C is the ML pipeline (largest effort, with Python + Rust tracks). Phase D extends C to prose. Phase E builds on committed atoms. Phase F is cleanup.

The relational weight principle provides the integration test at every boundary: does the pipeline output produce meaningful entity promotions for test scenarios?
