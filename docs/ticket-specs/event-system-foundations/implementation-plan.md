# Implementation Plan — Event System Foundations

## Purpose

This document sequences the implementation work for the event grammar, entity reference model, and ML classification pipeline into phased work items. Each phase has clear inputs, outputs, and dependencies. Investigation areas are marked explicitly — these are places where the right approach is not yet known and experimentation is needed before committing to an implementation.

The plan builds incrementally on existing working code. No phase breaks the ML prediction pipeline, the context assembly system, or the existing turn cycle.

### The Relational Weight Principle as Workflow Driver

The implementation workflow is driven by a single organizing principle: **events create relationships, and relationships create entities that matter.** This is not just a theoretical insight — it determines what we build and in what order:

1. **Types first** (Phase A): Define the vocabulary — event atoms, entity references, relational implications. The type system encodes the principle: an `EventAtom` carries `RelationalImplication` instances that connect `Participant` entities. Without implications, an event is scene texture.

2. **Weight computation next** (Phase B): Before building classifiers, establish *what classification is for* — computing relational weight from events and making promotion decisions. This ensures that every subsequent phase has a clear success criterion: does the classified output produce meaningful relational weight?

3. **ML classification pipeline** (Phase C): Build the training data generation → model fine-tuning → ONNX inference pipeline for event classification, entity extraction, and relation inference. The classifier's purpose is not taxonomy for its own sake — it's discovering the relationships that give entities weight.

4. **Turn-unit extraction** (Phase D): Wire the full pipeline end-to-end — Narrator rendering, turn history, committed-turn classification, and the rejection flow.

5. **Composition** (Phase E): Detect compound events at the turn level where the combination carries more relational weight than the sum of its parts.

6. **Documentation** (Phase F): Update design documents to reflect what was actually built.

Every phase serves the relational weight principle. Classification feeds extraction, extraction feeds relation inference, relation inference feeds weight accumulation, weight accumulation drives entity promotion. The pipeline has a single purpose: **make entities real through their relationships.**

---

## Phase Overview

```
Phase A: Event Grammar Types            ✅ COMPLETE
    │
    ▼
Phase B: Event-Entity Bridge            ✅ COMPLETE
    │
    ├─── (B validates the weight principle; C builds the ML pipeline that feeds it)
    ▼
Phase C: ML Classification Pipeline     ✅ COMPLETE (C.0–C.6)
    │
    ▼
Phase D: Turn Pipeline Integration      ← wiring, LLM rendering, turn history, deduplication
    │
    ▼
Phase E: Turn-Level Composition         ← scoped to turn/near-turn atoms only
    │
    ▼
Phase F: Documentation Update           ← update docs for this branch
```

### Relationship to Other Documents

| Document | Role |
|---|---|
| `event-grammar.md` | Defines what an event *is* — types, composition, alignment with existing types |
| `entity-reference-model.md` | Defines how entities are referenced, resolved, promoted through events |
| `classifying-events-and-entities.md` | Explores ML/NLP approaches to classification — the technical foundation for Phase C |
| `turn-cycle-architecture.md` | Actor-command-service → Bevy mapping, stage enum design, turn lifecycle |
| This document | Sequences the work, identifies dependencies and investigation areas |

---

## Phase A: Event Grammar Types — ✅ COMPLETE

**Status**: Complete (Feb 8, 2026). 31 new tests (66 total in core). All checks pass.

**What was built**: All work items A.1–A.9 as specified. New file `storyteller-core/src/types/event_grammar.rs` with `EventAtom`, `EventKind` (10 variants), `Participant`, `ParticipantRole`, `RelationalImplication`, `ImplicationType`, `EventSource`, `EventConfidence`, `CompoundEvent`, `CompositionType`. Extended `entity.rs` with `EntityRef`, `ReferentialContext`, `PromotionTier`, `RelationalWeight`, `UnresolvedMention`, `EntityBudget`. Updated `event.rs` with typed `EventPayload` enum (replacing `serde_json::Value`), `TurnId` (UUID v7). Added `Turn`, `ProvisionalStatus` (originally `TurnState`, renamed during lifecycle refinement), `CommittedTurn`.

**Deviation from original plan**: `TurnState` was renamed to `ProvisionalStatus` during Phase C lifecycle work. This better captures its role — tracking data provenance (Hypothesized → Rendered → Committed), not the turn itself. The turn's lifecycle is driven by `TurnCycleStage`, not `ProvisionalStatus`.

---

## Phase B: Event-Entity Bridge — ✅ COMPLETE

**Status**: Complete (Feb 8, 2026). 48 new tests (114 total in core). All checks pass.

**What was built**: All work items B.1–B.4. New directory `storyteller-core/src/promotion/` with `mod.rs` (`PromotionConfig`), `weight.rs` (`compute_relational_weight`, `normalize_mention`, `entity_matches`), `tier.rs` (`determine_promotion_tier`, `evaluate_demotion`), `resolution.rs` (`resolve_entity_ref` with 4 strategies: possessive, spatial, anaphoric, descriptive; `TrackedEntity`, `SceneResolutionContext`), `mention_index.rs` (`MentionIndex` via BTreeMap, `ResolutionRecord`, `retroactively_promote`).

**No deviations**: Implemented as planned. Investigation question resolved — went with option 2 (separate mention index) for retroactive promotion.

---

## Phase C: ML Classification Pipeline — ✅ COMPLETE

**Status**: Complete (Feb 9, 2026). 7 work items (C.0–C.6). 316 Rust tests + 50 Python tests. All checks pass.

See `phase-c-ml-classification-pipeline.md` for detailed specifications and completion status.

### What was built

| Work Item | What |
|---|---|
| **C.0** | `EventClassifier` struct in `storyteller-engine/src/agents/classifier.rs` — tokenizers integration |
| **C.1** | 8,000 annotated training examples at `$STORYTELLER_DATA_PATH/training-data/event_classification.jsonl` |
| **C.2** | Python fine-tuning pipeline at `training/event_classifier/` (HuggingFace Trainer, 8 modules, 34 tests) |
| **C.3** | ONNX inference in `storyteller-engine/src/inference/event_classifier.rs` — `classify_text()` runs both models |
| **C.4** | Relational implication inference in `storyteller-core/src/types/implication.rs` — heuristic mapping from EventKind × participants |
| **C.5** | Pipeline integration in `context/prediction.rs` — `classify_and_extract()`, `classification_to_event_features()` |
| **C.6** | Python evaluation framework — `evaluation.py`, `evaluate_cli.py`, 16 tests, baseline F1=1.0 on templated data |

### Additionally built (beyond original plan scope)

| Component | What |
|---|---|
| **Turn cycle architecture** | `docs/technical/turn-cycle-architecture.md` — actor-command-service → Bevy mapping |
| **TurnCycleStage** | 8-variant state machine in `storyteller-core/src/types/turn_cycle.rs` with `CommittingPrevious` |
| **EntityCategory** | Core-level NER category abstraction (Character/Object/Location/Other) |
| **Bevy resources** | `ActiveTurnStage`, `TurnContext`, `NarratorTask` in `storyteller-engine/src/components/turn.rs` |
| **Bevy systems** | `classify_system` + `predict_system` (real) + 4 stubs in `systems/turn_cycle.rs` |
| **Plugin registration** | `StorytellerEnginePlugin` registers resources and systems with `run_if` stage gating |
| **Label contract** | `storyteller-ml/src/event_labels.rs` — shared constants between Rust and Python |
| **Lifecycle refinement** | `TurnState` → `ProvisionalStatus`, `TurnPhaseKind` eliminated (duplicative of `TurnCycleStage`) |

### Key deviations from original plan

1. **DistilBERT instead of DeBERTa-v3-small**: DeBERTa-v3-small has NaN instability on Apple MPS. DistilBERT used as production fallback.
2. **Separate models (Approach B)**: Event classification + NER as independent ONNX models. Multi-task consolidation deferred.
3. **EventFeatureInput is not legacy**: Recognized as a feature encoding region for the character prediction model. The EventClassifier should not produce it — conversion from `ClassificationOutput` → `EventFeatureInput` belongs in pipeline orchestration.
4. **C.5 scope change**: Original plan had C.5 as zero-shot bootstrapping. Repurposed as pipeline integration (wiring EventClassifier into prediction pipeline) since fine-tuned models were available immediately.
5. **Turn cycle architecture emerged organically**: Not in the original plan but needed to wire the ML systems into Bevy. Produced the `CommittingPrevious` insight — commitment of previous turn's provisional data triggered by player's next input.

---

## Phase D: Turn Pipeline Integration

**Goal**: Wire the full turn pipeline end-to-end — Narrator LLM rendering, in-memory turn history, committed-turn classification (narrator prose + player input together), prediction confirmation, narrator output deduplication, and the rejection flow. The ML and LLM infrastructure are already running; this is wiring and integration.

**Depends on**: Phase C (ML classifier, Bevy systems, `TurnCycleStage`).

**Key architectural insight from Phase C**: With `CommittingPrevious`, narrator prose does not need pre-classification before the player sees it. Instead, when the player responds, the previous turn's narrator prose and the new player input are classified *together* as a unit. This provides better coreference resolution — referents span both texts (the narrator's nouns and the player's pronouns, or vice versa).

**Outputs**:
- In-memory turn history (player input, narrator rendering, classification, predictions per turn)
- Real `commit_previous_system` (currently stub)
- Real `start_rendering_system` / `poll_rendering_system` (currently stubs)
- Narrator output deduplication
- Committed-turn classification (narrator prose + player input as unit)
- Prediction confirmation
- X-card / rejection flow

### Work Items

**D.1: In-Memory Turn History** — ✅ PARTIALLY COMPLETE

The `TurnContext` resource holds the current turn's accumulated data (`player_input`, `classification`, `predictions`, `rendering`). What's missing is *persistence across turns* — when `commit_previous_system` resets `TurnContext` for the new turn, the previous turn's data is lost.

Add a `TurnHistory` Bevy Resource that accumulates committed turn data in memory:

```rust
/// Bevy Resource: in-memory history of committed turns.
///
/// Until the persistence branch (PostgreSQL + event ledger), this is the
/// only record of previous turns. Used for:
/// - Narrator context assembly (scene journal)
/// - Coreference resolution (prior mentions)
/// - Deduplication (comparing new rendering against recent output)
#[derive(Debug, Default, Resource)]
pub struct TurnHistory {
    pub turns: Vec<CompletedTurn>,
}

/// A turn that has been committed — all provisional data confirmed.
#[derive(Debug, Clone)]
pub struct CompletedTurn {
    pub turn_number: u32,
    pub player_input: String,
    pub narrator_rendering: Option<String>,
    pub classification: Option<ClassificationOutput>,
    pub predictions: Vec<CharacterPrediction>,
    pub committed_at: std::time::Instant,
}
```

`commit_previous_system` archives the current `TurnContext` into `TurnHistory` before resetting.

Tests: history accumulates across turns, reset doesn't lose history, history is accessible to downstream systems.

**D.2: Narrator Rendering (Real System)**

Replace the `start_rendering_system` stub with a real async LLM call. The infrastructure exists — `ExternalServerProvider` (Ollama) is functional, `NarratorTask::InFlight` bridges async/sync. Wire:

1. `assemble_context_system` → builds `NarratorContextInput` (for now: preamble + turn history as journal + prediction markdown)
2. `start_rendering_system` → spawns LLM call via `ExternalServerProvider`, sets `NarratorTask::InFlight`
3. `poll_rendering_system` → checks oneshot receiver each frame, on completion: moves result to `TurnContext.rendering`, advances to `AwaitingInput`

Model consideration: Test with both 7B and 14B parameter models. 14B may be a sweet spot for expressiveness and instruction-following over 7B, without the latency penalty of 32B Qwen (which was demonstrably worse at following instructions despite being larger).

`[INVESTIGATION NEEDED]` Prompt engineering for narrator voice, instruction following, and output format. The narrator should render character intent in story voice without re-rendering previously presented content (see D.4).

Tests: Bevy system tests (mock LLM provider, verify `NarratorTask` state transitions). Feature-gated `test-llm` tests that run real Ollama and verify rendering output.

**D.3: Committed-Turn Classification**

With `CommittingPrevious`, the classification window includes *both* the previous turn's narrator prose *and* the current player input. This is architecturally significant — noun-pronoun-subject-object referents are contextually relevant across both texts.

In `commit_previous_system`:
1. Retrieve previous narrator rendering from `TurnContext.rendering`
2. Receive new player input
3. Concatenate: `"{narrator_prose}\n\n{player_input}"` as a single classification input
4. Run `classify_and_extract()` on the combined text
5. This gives entity extraction and event classification the full coreference context

The existing `classify_system` continues to classify *only* player input for the current turn's character prediction pipeline (fast, focused). The committed-turn classification is a separate, richer pass that feeds the event ledger and entity promotion.

```rust
/// Classify the committed turn unit — narrator prose + player input together.
///
/// This runs during CommittingPrevious and produces richer classification
/// than the per-turn classify_system because it has full coreference context
/// across both texts.
fn classify_committed_turn(
    narrator_prose: &str,
    player_input: &str,
    classifier: Option<&EventClassifier>,
    target_count: usize,
) -> (EventFeatureInput, Option<ClassificationOutput>)
```

Tests: combined text produces entities that span both narrator and player portions. Coreference (pronoun in player input referring to noun in narrator prose) resolves correctly.

**D.4: Narrator Output Deduplication**

The Narrator should render only *new* content — not re-render its full response each time. Because we can only prompt so far, we need similarity scanning as a safety net:

1. **Prompting**: Instruct the Narrator that it is continuing a conversation, not starting over. Include recent output in the context window as "what has already been said."
2. **Sentence-level scanning**: Compare each sentence in new output against recent turns in `TurnHistory`. Use:
   - Hash comparison (exact match — catches verbatim repetition)
   - Token-level overlap (catches minor rephrasing — Jaccard similarity on word tokens)
   - If available: embedding similarity from the tokenizer (cosine distance on mean-pooled token embeddings)
3. **Elision**: Sentences with similarity above threshold are removed from the player-facing output. The full rendering is preserved in `TurnHistory` for context assembly.

Start with hash + token overlap (deterministic, no additional model). Add embedding similarity if token overlap proves insufficient.

`[INVESTIGATION NEEDED]` Similarity thresholds. What level of sentence overlap constitutes "near-duplicate" vs. "intentional callback"? Narrative prose legitimately echoes previous phrases for effect. Start conservative (high threshold, only catch near-verbatim), tune from play data.

Tests: exact duplicate sentences are elided, minor rephrasing detected above threshold, intentionally different content preserved, narrative callbacks (partial echoes) are not falsely elided.

**D.5: Prediction Confirmation**

Cross-reference extracted atoms against ML prediction metadata. When a committed-turn-extracted atom matches a prediction (same character, same action type), the atom's confidence is boosted:

```rust
fn confirm_predictions(
    extracted_atoms: &[EventAtom],
    predictions: &[CharacterPrediction],
) -> Vec<EventAtom>
```

Events that were both predicted and rendered are higher-confidence than events that appeared only in prose (the Narrator added something unpredicted).

Tests: predicted-and-rendered atoms have higher confidence than unpredicted atoms, unmatched predictions don't produce phantom events.

**D.6: X-Card / Rejection Flow**

Implement the rejection path: when a player rejects a rendering, the turn transitions from `Rendered` back to `Hypothesized` (predictions are preserved, prose is discarded). The rejection is recorded in `TurnHistory`. The Storykeeper adds constraints and the Narrator re-renders.

For now: rejection resets `TurnContext.rendering` to `None` and returns the pipeline to `AssemblingContext` (with additional constraint context). `ProvisionalStatus::Rendered` → `ProvisionalStatus::Hypothesized`.

Tests: rejected turn produces no committed events, re-rendered turn can be committed normally, rejection is recorded.

### Verification

```bash
cargo check --all-features
cargo test --workspace                           # Unit + system tests
cargo test --workspace --features test-ml-model  # + ONNX model tests
cargo test --workspace --features test-llm       # + Ollama rendering tests
cargo clippy --all-targets --all-features
cargo fmt --check
```

---

## Phase E: Turn-Level Event Composition

**Goal**: Detect compound events from sequential atoms *within a single committed turn or adjacent turns*. Scoped to the near-turn window only — wider event-graph composition (cross-scene causal chains, thematic arcs) is deferred to the persistence branch where real narrative graphs and event ledger infrastructure exist.

**Depends on**: Phase D (committed turns producing atoms with turn-level classification).

**Why scoped**: The event dependency graph (`docs/technical/event-dependency-graph.md`) describes a full DAG of combinatorial narrative triggers. Building placeholder code to mimic this without real graph infrastructure and persistence produces throwaway work. Turn-level composition is feasible now because the data (sequential atoms from a single classification pass) is available in memory.

**Outputs**:
- New file: `storyteller-engine/src/context/event_composition.rs` (or similar location)
- `CompoundEvent` detection from turn-level atom sequences

### Work Items

**E.1: Temporal Composition Detection**

The simplest composition type — atoms within the same committed turn with participant overlap:

```rust
fn detect_temporal_compositions(
    atoms: &[EventAtom],
) -> Vec<CompoundEvent>
```

Groups atoms by participant overlap within the same turn. Atoms sharing an Actor or Target are candidates for temporal composition.

Tests: adjacent atoms with shared participants compose, atoms with no participant overlap do not.

**E.2: Causal Composition Detection**

Detect known causal patterns within a turn:

```rust
fn detect_causal_compositions(
    atoms: &[EventAtom],
) -> Vec<CompoundEvent>
```

Patterns (initial set):
- Examine → EmotionalExpression (saw something → reacted)
- SpeechAct → RelationalShift (said something → relationship changed)
- ActionOccurrence → SpatialChange (did something → moved)
- InformationTransfer → EmotionalExpression (learned something → felt something)
- SpeechAct → ActionOccurrence (said something → did something)

`[PLACEHOLDER]` The causal pattern library needs expansion from play data. Start with 5-10 patterns.

Tests: known causal patterns are detected, unrelated atoms are not falsely composed.

**E.3: Emergent Weight Calculation**

Compute the emergent relational implications of a compound event — the combination carries more weight than the sum of its parts:

```rust
fn compute_emergent_implications(
    compound: &CompoundEvent,
    atom_implications: &[Vec<RelationalImplication>],
) -> Vec<RelationalImplication>
```

`[PLACEHOLDER]` Composition multipliers (1.5x for causal, 1.0x for temporal) are initial guesses.

Tests: compound weight exceeds (or equals) sum of atom weights depending on composition type.

**E.4: Composition Integration**

Wire the composition detector into the committed-turn processing pipeline. After atoms are produced from the committed-turn classification (D.3), run composition detection and emit compound events alongside atoms.

Tests: end-to-end — committed turn → atoms → composition detection → compound events with correct types and weights.

### What is deferred

The following composition capabilities are deferred to the persistence branch:

- **Cross-scene composition**: Detecting causal chains that span multiple scenes (requires narrative graph traversal)
- **Thematic composition**: Detecting thematic resonance across distant events (requires event ledger queries)
- **Conditional composition**: "If X then Y" patterns spanning multiple turns or scenes
- **Event dependency DAG**: The full combinatorial trigger model from `event-dependency-graph.md`

### Verification

Existing tests pass. New tests cover each composition type and the end-to-end turn-level pipeline.

---

## Phase F: Documentation Update

**Goal**: Update design documents to reflect what was built in this branch. The original documents were written for the multi-agent architecture; Phases A–E introduced the narrator-centric architecture with ML classification, turn lifecycle, and Bevy system integration.

**Depends on**: All previous phases (to know what was actually built).

**Outputs**:
- Updated file: `docs/technical/event-system.md`
- Updated file: `docs/technical/narrator-architecture.md` (if needed)
- Updated file: `docs/foundation/open-questions.md` (close resolved questions, add new ones)
- Updated README files as needed

### Work Items

**F.1: event-system.md Revision**

Update to reflect the narrator-centric architecture:
- Replace multi-agent event subscription with turn-cycle stage pipeline
- Update event lifecycle with `ProvisionalStatus` and `CommittingPrevious`
- Add event classification via ML models (replacing hypothetical classifier agent)
- Integrate the relational weight principle and entity promotion pipeline
- Mark superseded sections (agent subscriptions for Character Agents as LLM agents)

**F.2: Turn Cycle Documentation**

Ensure `turn-cycle-architecture.md` accurately reflects the implemented state:
- Stage enum (8 variants including `CommittingPrevious`)
- Committed-turn classification (narrator prose + player input as unit)
- In-memory turn history model
- Narrator output deduplication approach

**F.3: Updated Open Questions**

Close questions that were resolved during implementation:
- Classifier agent design → ML classification pipeline (Phase C)
- Turn lifecycle → `TurnCycleStage` + `ProvisionalStatus`
- Entity promotion heuristics → configurable `PromotionConfig` with threshold values

Add new questions that emerged:
- Narrator deduplication thresholds (hash vs. embedding similarity)
- 14B vs 7B model tradeoffs for narrator rendering
- Committed-turn classification accuracy on real (non-templated) prose
- Optimal coreference window (single turn vs. N-turn lookback)

**Verification**: Document review for internal consistency. Type sketches in revised documents match the actual implemented types.

---

## Cross-Phase Concerns

### Testing Strategy

All phases follow the project's test tier conventions:
- **Unit tests** (default `cargo test`): Type constructibility, serde round-trip, weight computation, promotion logic, composition detection, Bevy system state transitions.
- **Feature-gated ML tests** (`test-ml-model`): End-to-end tests that run the ONNX model and verify event atom production from real predictions.
- **Feature-gated LLM tests** (`test-llm`): Turn extraction and rendering pipeline tests that require running Ollama to produce real Narrator prose.
- **Python tests** (`uv run pytest` in `training/event_classifier/`): Training pipeline and evaluation framework tests.

### In-Memory Until Persistence

This branch deliberately stores all turn-over-turn data in memory (`TurnHistory` resource). The persistence branch will introduce:
- PostgreSQL event ledger (append-only committed events)
- Checkpoint snapshots (periodic state dumps for crash recovery)
- Apache AGE graph storage (narrative graph, relational web)
- Command sourcing (player input persisted before processing begins)

The in-memory model in this branch validates the data shapes and pipeline flow. Migration to persistent storage should be mechanical — the `CompletedTurn` struct maps directly to ledger entries.

### Scene Gravity as Downstream Benefit

The Turn → Scene hierarchy enables post-hoc scene gravity computation. Once events are being extracted and committed against turns (Phase D), the system can compute turn density, scene gravity, and attractor refinement. This is not a deliverable of this branch — it emerges naturally once events flow through the system and persistence is available.

### Existing Code Preservation

Each phase is designed to be additive:
- Phase A added new types without modifying existing ones (except the `NarrativeEvent` payload).
- Phase B added new logic without changing existing functions.
- Phase C extended `classify_player_input()` alongside the new ML path.
- Phase D fills in existing stubs (no signature changes).
- Phase E adds a new pipeline stage.
- Phase F is documentation only.

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
| Deduplication similarity threshold | 0.9 | D | Sentence-level near-duplicate detection |
| Composition multiplier (causal) | 1.5x | E | Weight amplification for causal composition |
| Composition multiplier (temporal) | 1.0x | E | Weight for temporal composition |
| Max composition depth | 2 | E | Compounds contain atoms only, not other compounds |
| Causal pattern library | 5-10 patterns | E | Known causal event patterns |

### `[INVESTIGATION NEEDED]` Summary

| Area | Phase | Question | Starting Point |
|---|---|---|---|
| Narrator prompt engineering | D | How to instruct the Narrator to render only new content? | Include recent output in context, explicit instruction not to repeat |
| Deduplication approach | D | Hash vs. token overlap vs. embedding similarity? | Start with hash + Jaccard on word tokens |
| Deduplication threshold | D | Where is the line between "near-duplicate" and "intentional callback"? | Start conservative (0.9), tune from play data |
| 14B vs 7B model selection | D | Which parameter count gives best expressiveness/latency/instruction-following? | Test both with same prompts, compare output quality and latency |
| Prose clause segmentation | D | Does clause-level classification lose too much context for literary prose? | Whole-turn classification first (narrator prose + player input together), segment only if needed |
| Coreference window | D | How many turns back should coreference resolution look? | Start with 1-turn (narrator + player), extend if insufficient |
| Prediction confirmation matching | D | How to match extracted atoms to predictions (exact type match vs. fuzzy)? | Start with exact action-type match |
| Composition detection heuristics | E | What patterns indicate causal/conditional composition? | Start with 5-10 manual patterns |

---

## Sequencing Recommendation

For a single developer working on this, the recommended sequence for remaining work (D–F) is:

1. **D.1 + D.2** (in-memory turn history + real Narrator rendering) — the foundation. Everything else depends on having real LLM output flowing through the pipeline. Estimate: 2-3 sessions.

2. **D.4** (narrator output deduplication) — address this early because it directly affects the quality of D.3's classification input. If the narrator repeats itself, the repeated content contaminates classification. Estimate: 1-2 sessions.

3. **D.3** (committed-turn classification) — classify narrator prose + player input together. Can only be meaningfully tested once D.2 produces real rendering. Estimate: 1-2 sessions.

4. **D.5** (prediction confirmation) — cross-reference extracted atoms against predictions. Quick once D.3 works. Estimate: 1 session.

5. **D.6** (X-card / rejection flow) — the safety valve. Important but not blocking. Estimate: 1 session.

6. **Phase E** (turn-level composition) — operates on committed atoms from D.3. Estimate: 2-3 sessions.

7. **Phase F** (documentation) — after everything is settled. Estimate: 1 session.

Total remaining: approximately 8-12 sessions, dominated by D.2 (Narrator rendering integration) and E (composition detection).

---

## Appendix: Phase History

This plan was originally written to cover Phases A–F from scratch. Phases A, B, and C are now complete. The plan was revised on Feb 9, 2026 to:

1. Mark completed phases with deviation notes
2. Revise Phase D to reflect the `CommittingPrevious` timing insight — narrator prose and player input are classified *together* after commitment, not separately
3. Add narrator output deduplication (D.4) as a new concern
4. Add in-memory turn history (D.1) as explicit requirement (until persistence branch)
5. Scope Phase E to turn-level composition only — wider event-graph composition deferred to the persistence branch
6. Scope Phase F to documentation for this branch's changes

The original Phase C.5 (zero-shot bootstrapping) was repurposed as pipeline integration since fine-tuned models were available immediately. The original Phase D.2 (narrator prose pre-classification) was revised to post-commitment classification based on the `CommittingPrevious` architecture.
