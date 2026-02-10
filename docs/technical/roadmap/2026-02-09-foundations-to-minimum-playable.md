# Roadmap: Foundations to Minimum Playable

**Date**: February 9, 2026
**Branch**: `jcoletaylor/event-system-foundations` (ready for PR)
**Starting point**: Phases A-C complete, D.1-D.2 complete, 336 Rust tests + 50 Python tests

## Where We Are

Three days of work produced a narrator-centric storytelling engine with:

- **Event grammar** (Phase A): 10 event kinds, typed payloads, entity references, relational implications, turn-level provenance — 948 lines of type definitions replacing the prior `serde_json::Value` payload
- **Entity promotion bridge** (Phase B): Weight computation, 4 resolution strategies, mention indexing, configurable promotion/demotion thresholds — 1,063 lines
- **ML classification pipeline** (Phase C): Event classifier + NER via fine-tuned DistilBERT ONNX models, character behavior predictor via custom ONNX model, heuristic relational implication inference, full pipeline integration — 2,297 lines of inference code, 8,000 training examples, Python training + evaluation framework
- **Turn pipeline** (Phase D.1-D.2): 8-stage Bevy ECS state machine (AwaitingInput through Rendering), in-memory turn history, real commit/resolve/assemble/render systems, async LLM bridge via oneshot polling — all stubs replaced with real implementations

**What works end-to-end**: Player types input, ML classifies events and extracts entities, ML predicts character behaviors, deterministic resolver wraps predictions, three-tier context assembles (preamble + journal + retrieval), Narrator LLM renders prose, journal compresses history. The `play_scene_context` binary runs this loop interactively against Ollama.

**What doesn't exist yet**: Persistent storage, real graph queries, player-facing protocol, TUI, gRPC server, multi-session support, real resolver mechanics, committed-turn classification, deduplication, safety mechanisms.

---

## Part I: Tenets and Principles

These emerged organically across three days of design and implementation. They are not aspirational — they describe choices already made and working.

### Architectural Tenets

1. **The Narrator is the only LLM agent.** Character behavior comes from ML prediction models. World constraints come from a rules engine. The Reconciler is a deterministic resolver. One LLM call per turn, not three-plus.

2. **Imperfect information by design.** No single agent has complete knowledge. The Storykeeper filters what downstream agents may know. The Narrator knows only what it's told. Character predictions don't know the full narrative graph. Richness emerges from partial perspectives.

3. **The turn is the atomic unit.** Nothing is committed until the player responds. ML predictions are hypotheses. Narrator prose is provisional. The player's next input triggers commitment of the previous turn. This enables replay, X-card safety, and clean idempotent extraction.

4. **Events create relationships, relationships create entities.** This is the implementation workflow driver. Classification feeds extraction, extraction feeds relation inference, relation inference feeds weight accumulation, weight accumulation drives entity promotion. The pipeline has a single purpose: make entities real through their relationships.

5. **Essential simplicity.** Simple atoms, rich behavior from composition. A `[central_tendency, variance, range_low, range_high]` tuple is not sophisticated — but composed across 30+ axes with temporal layering and contextual triggers, it produces a character.

6. **Use Bevy as Bevy.** Don't recreate tokio channels or actor mailboxes inside ECS. Services do actual work in framework-independent functions. The Bevy layer is orchestration and state management only.

### Design Commitments (Bets-on-Approach)

These are deliberate bets. We chose one approach over alternatives with the understanding that switching later has a real cost.

| Bet | What we chose | What we didn't choose | Switching cost |
|-----|---------------|----------------------|----------------|
| **Single-LLM Narrator** | One LLM call/turn; ML for everything else | Multi-agent LLM conversations | Low — agent trait abstractions exist |
| **Bevy ECS runtime** | In-process systems, Resource state, SystemSet ordering | Actor framework (tokio actors, separate processes) | High — pervasive, but well-isolated behind service functions |
| **PostgreSQL + Apache AGE** | Unified relational + graph in one database | Separate graph DB (Neo4j, NebulaGraph) | Medium — graph queries isolated in `graph/` module |
| **ort/ONNX for ML** | ONNX Runtime via Rust bindings | Python sidecar, TensorFlow Serving | Low — inference behind trait; `burn` is planned Rust-native successor |
| **DistilBERT over DeBERTa** | Stability on Apple MPS (no NaN) | DeBERTa-v3-small (44M params, better benchmarks) | Low — swap ONNX model files, adjust tokenizer config |
| **Turn-level composition only** | Near-turn event detection in memory | Full event DAG with cross-scene causal chains | Low — wider composition needs persistence anyway |
| **Command sourcing** | Player input persisted before processing | Process-then-persist | Medium — requires event ledger infrastructure |

### Extensibility Points (Swappable by Design)

| Component | Abstraction | Current | Future |
|-----------|------------|---------|--------|
| LLM provider | `LlmProvider` trait | `ExternalServerProvider` (Ollama) | `CloudLlmProvider`, `CandleLlmProvider` |
| Emotional grammar | `EmotionalGrammar` trait | PlutchikWestern (8 primaries) | Wu Xing, non-human, fey/mythic |
| Game mechanics | `GameDesignSystem` trait | Pass-through resolver | Genre-specific RPG mechanics |
| Phase observer | `PhaseObserver` trait | `NoopObserver`, `CollectingObserver` | Streaming events, metrics, TUI |
| ML inference | `CharacterPredictor`, `EventClassifier` structs | ONNX via `ort` | `burn` all-Rust, or cloud endpoints |
| Scene data | `SceneData`, `CharacterSheet` | Workshop hardcoded (`the_flute_kept.rs`) | Loaded from `storyteller-data`, authored tools |

---

## Part II: Document Landscape

### Inventory by Recency

53 documents across 4 directories. The design layer (foundation + technical) is stable. The implementation layer (ticket-specs) tracks active work.

**Active (Feb 8-9)**:
- `turn-cycle-architecture.md` — Bevy mapping, module organization, async bridge
- `phase-d-turn-pipeline-integration.md` — D.1-D.2 complete, D.3-D.6 deferred/specified
- `implementation-plan.md` — Master plan, Phases A-C complete, D-F specified
- `phase-c-ml-classification-pipeline.md` — Complete, 7 work items
- `event-dependency-graph.md` — DAG architecture for combinatorial triggers
- `scene-model.md` — Ludic contract, rendered space, player register
- `ml_strategy/` (5 files) — Character prediction, event classification, model selection, training data

**Stable (Feb 5-7)**:
- `narrator-architecture.md` — The architectural pivot document
- `tensor-schema-spec.md` — Formal type system, 7 resolved decisions
- `entity-model.md` — Unified Entity model, promotion lifecycle
- `emotional-model.md` — Plutchik-derived grammar, sedimentation mechanics
- Foundation documents (9 files) — Philosophy, world design, anthropological grounding, power

**Frozen (Feb 4-5)**:
- `narrative-graph-case-study-tfatd.md` — Gravitational landscape analysis
- `relational-web-tfatd.md` — Asymmetric relational web
- `agent-message-catalog.md` — Multi-agent message types (partially superseded by narrator-centric pivot)

### Superseded Concepts

These were valid in the multi-agent architecture but are now partially or fully replaced:

| Document | What's superseded | What remains valid |
|----------|-------------------|-------------------|
| `agent-message-catalog.md` | Character Agent ↔ Reconciler messages, multi-agent turn phases | Message format design, token budgets, Narrator ↔ Player protocol |
| `system_architecture.md` | Multi-agent collaboration model | Imperfect information principle, agent roles as conceptual boundaries |
| `event-system.md` | Agent subscription model, Character Agent as LLM | Event lifecycle, sensitivity map, priority tiers, two-track classification concept |

### Document Reorganization (Future Work)

Not blocking — the documents are internally consistent where they matter (type definitions match code, architecture docs match implementation). But a cleanup pass would:

1. Add "superseded by narrator-centric architecture" headers to affected sections in `agent-message-catalog.md`, `event-system.md`, and `system_architecture.md`
2. Consolidate the ML strategy docs (currently 5 separate files in `ml_strategy/`) into the main technical docs
3. Update `open_questions.md` to close resolved items and add new ones from Phase C-D work
4. Create a single "how to read these docs" guide that accounts for the architectural pivot

---

## Part III: What Remains from Event System Foundations

### D.3: Committed-Turn Classification — Ready to Implement

When a turn is committed (player sends next input), classify the combined narrator prose + player input for richer event extraction. The infrastructure exists — `classify_and_extract()` works, `CompletedTurn` stores classification. Implementation is straightforward: concatenate texts, classify, store on `CompletedTurn`.

**Deferred from this branch because**: The committed classification feeds the event ledger, which requires persistence (future branch). The per-player-input classification is sufficient for the current pipeline.

### D.4: Narrator Output Deduplication — Research Problem

The Narrator sometimes re-renders previously presented content. Two approaches:

1. **Prompt engineering**: Include recent output in context with "what has already been said" framing
2. **Post-rendering filter**: Sentence-level similarity scanning against recent journal entries

**Research needed**: Similarity thresholds. Narrative prose legitimately echoes previous phrases for effect (callbacks, refrains). Start with hash + Jaccard token overlap (deterministic), add embedding similarity if insufficient.

### D.5: Prediction Confirmation — Research Problem

After rendering, compare rendered text against character predictions to determine which were "confirmed" by the narrative. Requires semantic matching between structured predictions and free-form prose.

**Research needed**: Matching strategy — exact action-type match vs. fuzzy semantic similarity. The gap between "Character X speaks about Y" (prediction) and the narrator's rendering of that speech is a non-trivial alignment problem.

### D.6: X-Card / Rejection Flow — Needs Protocol Decision

Allow the player to reject a rendering before commitment. The `ProvisionalStatus` model already supports this — rendered-but-not-committed data can be discarded. But the mechanism needs a player-facing protocol (special input prefix? separate channel?).

**Blocked by**: Player-facing protocol decisions (see Part V).

---

## Part IV: The Persistence Layer

This is a plan-for-a-plan. The in-memory model validates data shapes; migration to persistent storage should be mechanical.

### What Needs to Persist

| Data | Current Location | Persistence Target | Priority |
|------|-----------------|-------------------|----------|
| Committed turns | `TurnHistory` (in-memory Vec) | Event ledger (PostgreSQL, append-only) | High |
| Player input (command source) | `TurnContext.player_input` | Event ledger (before processing) | High |
| Relational web | Not yet implemented | Apache AGE graph | High |
| Narrative graph | Not yet implemented | Apache AGE graph | High |
| Character tensors | `CharacterSheet` (hardcoded workshop) | PostgreSQL JSONB | Medium |
| Scene definitions | `SceneData` (hardcoded workshop) | PostgreSQL JSONB | Medium |
| Session state | Not yet implemented | PostgreSQL + checkpoint | Medium |
| Setting topology | Not yet implemented | Apache AGE graph | Low |

### Four Graphs

The system has four distinct graph structures, all planned for Apache AGE:

1. **Relational web**: Directed edges between entities with 5 substrate dimensions (trust[3], affection, debt, history, projection). Edges are asymmetric (A→B differs from B→A).
2. **Narrative graph**: Scene connectivity with gravitational mass. Scenes as nodes, transitions as edges. Attractor basins, not branching trees.
3. **Setting topology**: Re-enterable locations with spatial relationships. Geography constrains what's reachable.
4. **Event dependency DAG**: Combinatorial narrative triggers modeled as directed acyclic graph. Precondition sets → event enablement.

### Shared Async Services

Once persistence exists, several services need to be shared across the pipeline:

| Service | Pattern | Why |
|---------|---------|-----|
| Event ledger writer | Actor (mpsc channel) | Multiplexed writes from commit + classification + composition |
| GraphRAG retriever | Actor (mpsc channel) | Context retrieval for multiple stages (assembly, resolution) |
| Checkpoint manager | Periodic timer | Snapshot current state for crash recovery |
| RabbitMQ dispatcher | Actor (mpsc channel) | Outbound events to tasker-core |

These are where the actor/channel pattern becomes appropriate — persistent, multiplexed async services where the persistent task buys something real. The current oneshot pattern for the Narrator remains correct (single request-response per turn).

### State Machine Formalization

Several implicit state machines should be formalized when persistence arrives:

- **Scene lifecycle**: Entry → Play → Exit (with re-entry support)
- **Entity lifecycle**: Unmentioned → Referenced → Tracked → Persistent (with demotion)
- **Session lifecycle**: Created → Active → Suspended → Resumed → Ended
- **ProvisionalStatus** (already formalized): Hypothesized → Rendered → Committed

### Migration Strategy

The approach should be:

1. Add `sqlx` migrations for the event ledger schema
2. Implement `LedgerWriter` behind a trait, with in-memory and PostgreSQL implementations
3. Wire PostgreSQL implementation via a Bevy Resource (like `NarratorResource` pattern)
4. Add Apache AGE setup and graph schema
5. Implement graph query functions in `storyteller-core/src/graph/`
6. Wire graph retrieval into context assembly (Tier 3: retrieved context)

The checkpoint + ledger replay model for crash recovery should be implemented alongside, not after, the initial persistence layer.

---

## Part V: Player-Facing Interface

### TUI (ratatui)

The minimum playable experience needs a terminal interface richer than stdin/stdout. `ratatui` is the natural choice for Rust TUI applications.

#### Panel Layout

```
┌─────────────────────────────────────────────────────────────┐
│                    NARRATOR OUTPUT                           │
│  The fire crackles low in the hearth. Sarah watches the     │
│  embers drift upward, each one a tiny wandering star...     │
│                                                             │
├─────────────────────────────────────┬───────────────────────┤
│        EVENT LOG                    │   CHARACTER STATE     │
│  [ActionOccurrence] Sarah watches   │  Sarah                │
│  [EmotionalExpr] wonder (0.82)      │   joy: 0.3 ▓▓▓░░     │
│  [Entity] embers (Object, 0.95)     │   trust: 0.6 ▓▓▓▓░   │
│                                     │   fear: 0.1 ▓░░░░     │
│                                     │                       │
│                                     │  Predictions:         │
│                                     │   speak (0.7) →       │
│                                     │   "asks about stars"  │
├─────────────────────────────────────┴───────────────────────┤
│ > [player input]                                            │
└─────────────────────────────────────────────────────────────┘
```

Four regions:
1. **Narrator output** (top): Scrollable narrative prose. The player-facing story.
2. **Event log** (bottom-left): Real-time classification results, entity extractions, relational implications. Developer/designer observability.
3. **Character state** (bottom-right): Current tensor activations, predictions, emotional state. ML pipeline observability.
4. **Input** (bottom): Player text entry with command prefix support (`/undo`, `/x-card`, `/debug`, `/dump`).

#### Implementation Approach

- New crate: `storyteller-tui` (depends on `storyteller-engine`, `ratatui`, `crossterm`)
- Bevy integration: The TUI reads from Bevy Resources (same `TurnContext`, `TurnHistory`, `ActiveTurnStage`) and writes player input via `PendingInput`
- The TUI is an alternative frontend to `play_scene_context.rs`, not a replacement — the simple binary remains for testing and CI
- Event log and character state panels are behind a toggle (not visible to pure players, visible to designers and developers)

### Collaborative Observability

The TUI's event log and character state panels serve a specific design purpose: **Claude and the human collaborating during play sessions.** When Claude can see the same pipeline state the human sees — classification confidence, prediction outputs, emotional shifts, relational weight changes — it can provide meaningful feedback about system behavior in real time.

This means the observability panels should:
- Show confidence values, not just labels
- Show prediction→rendering alignment (which predictions the narrator honored)
- Show entity promotion/demotion events as they happen
- Show token budget allocation across the three context tiers
- Be serializable (see session dumps below)

### Session Dumps

Every play session should produce a serializable trace:

```
session-dump/
├── metadata.json           # Session ID, start/end time, scene, model config
├── turns/
│   ├── turn-001.json      # player_input, classification, predictions, rendering, timestamps
│   ├── turn-002.json
│   └── ...
├── entities.json           # Entity lifecycle: mentions, promotions, demotions, final state
├── events.json             # All extracted EventAtoms with turn provenance
├── journal.json            # SceneJournal state at session end
└── context-snapshots/      # NarratorContextInput for each turn (what the LLM actually saw)
    ├── turn-001.json
    └── ...
```

Every entity and event carries a UUID v7 ID. The dump should be traceable: given an entity ID, find every turn where it was mentioned, every event that affected it, every relational weight change.

This enables:
- Post-session analysis (did the system behave well?)
- Training data extraction (real prose for fine-tuning)
- Regression testing (replay a session, compare outputs)
- Design iteration (what did the narrator do with the predictions?)

---

## Part VI: Server and API Contract

### gRPC Server (tonic)

The API contract between the engine and any frontend (TUI, web client, mobile) should be gRPC with streaming:

```protobuf
service Storyteller {
  // Start or resume a session
  rpc CreateSession(CreateSessionRequest) returns (SessionInfo);

  // Submit player input and receive the full turn result
  rpc SubmitInput(PlayerInput) returns (TurnResult);

  // Stream pipeline events as they happen (classification, prediction, rendering)
  rpc ObservePipeline(ObserveRequest) returns (stream PipelineEvent);

  // Reject the current rendering (X-card)
  rpc RejectRendering(RejectRequest) returns (TurnResult);

  // Dump session state
  rpc DumpSession(DumpRequest) returns (SessionDump);
}
```

The streaming `ObservePipeline` RPC is key for collaborative observability. As each pipeline stage completes, the server emits a `PipelineEvent` with the stage result. The TUI (or any client) renders these incrementally.

### REST Fallback

For browser-based clients that can't use gRPC natively:

- `POST /api/v1/sessions` — Create session
- `POST /api/v1/input` — Submit player input
- `GET /api/v1/sessions/:id/events` (SSE) — Server-sent events for pipeline observation
- `POST /api/v1/sessions/:id/reject` — X-card
- `GET /api/v1/sessions/:id/dump` — Session dump

The existing `storyteller-api` crate scaffolds this with Axum. The gRPC server would be a separate service endpoint (both can run on the same instance).

### Proto File Strategy

- `storyteller.proto` in a shared `proto/` directory at workspace root
- Generated Rust types via `tonic-build` in a `storyteller-proto` crate
- Proto types are the wire format; internal types remain in `storyteller-core`
- Conversion traits (`From<CoreType> for ProtoType`) in the proto crate

---

## Part VII: Sequenced Plan

### Phase 1: PR and Merge (Now)

Merge `jcoletaylor/event-system-foundations` into `main`. The branch delivers:
- Event grammar types + entity promotion bridge + ML classification pipeline
- Full turn pipeline through Bevy ECS
- 336 Rust tests + 50 Python tests
- Comprehensive documentation

### Phase 2: Remaining D Items + Documentation (Next Branch)

**Goal**: Close out the event-system-foundations implementation plan.

| Item | Type | Depends on | Estimate |
|------|------|-----------|----------|
| D.3: Committed-turn classification | Implementation | Nothing (ready now) | Small |
| D.4: Narrator deduplication | Research + implementation | D.3 (needs real committed text) | Medium |
| D.5: Prediction confirmation | Research + implementation | D.3 | Medium |
| D.6: X-card / rejection flow | Implementation + protocol decision | Player-facing protocol | Small (code), Blocked (protocol) |
| Phase E: Turn-level composition | Implementation | D.3 | Medium |
| Phase F: Documentation update | Writing | All above | Small |

**Hypothesis-driven approach for D.4 and D.5**:

D.4 (deduplication): Start with prompt engineering (include recent output as "already said"). Measure repetition rate across 10 play sessions. If repetition rate >15%, add hash + Jaccard post-filter. If >5% after that, add embedding similarity.

D.5 (prediction confirmation): Start with exact action-type + character match. Measure confirmation rate across 10 play sessions. If <30% of rendered content maps to predictions, the matching is too strict — add fuzzy matching via shared-entity overlap.

### Phase 3: Persistence Layer

**Goal**: Durable state across sessions.

| Item | Depends on | Estimate |
|------|-----------|----------|
| PostgreSQL schema + sqlx migrations | Nothing | Medium |
| Event ledger writer (append-only) | Schema | Medium |
| Checkpoint snapshots | Ledger | Small |
| Command sourcing (input before processing) | Ledger | Small |
| Apache AGE setup + graph schema | PostgreSQL | Medium |
| Relational web queries | AGE schema | Medium |
| Narrative graph queries | AGE schema | Medium |
| Wire graph retrieval into Tier 3 context | Graph queries | Medium |
| Session lifecycle management | Ledger + checkpoint | Medium |

**Key decision point**: Before starting, verify Apache AGE's Cypher operator coverage for the queries we need (path traversal, centrality computation, neighborhood aggregation). If coverage is insufficient, evaluate alternatives (in-process graph library like `petgraph` for computation, AGE for storage only).

### Phase 4: Session Dumps and Traceability

**Goal**: Every play session produces a traceable artifact.

| Item | Depends on | Estimate |
|------|-----------|----------|
| Session dump schema (JSON) | Core types stable | Small |
| Dump serialization from TurnHistory + entities | Phase 2 (D.3 for events) | Medium |
| Entity lifecycle tracking across turns | Phase 3 (persistence) | Medium |
| Context snapshot capture per turn | Nothing (data available now) | Small |
| Replay from dump (deterministic re-execution) | Dump schema + persistence | Large |

### Phase 5: TUI

**Goal**: Minimum playable terminal experience.

| Item | Depends on | Estimate |
|------|-----------|----------|
| `storyteller-tui` crate scaffold | Nothing | Small |
| Narrator output panel (scrollable) | Nothing | Medium |
| Player input panel with command support | Nothing | Medium |
| Event log panel | PhaseObserver integration | Medium |
| Character state panel | TurnContext reading | Medium |
| Pipeline stage indicator | ActiveTurnStage reading | Small |
| Session dump command (`/dump`) | Phase 4 | Small |
| X-card command (`/x-card`, `/undo`) | D.6 | Small |

### Phase 6: gRPC Server

**Goal**: Machine-to-machine API for frontends beyond the TUI.

| Item | Depends on | Estimate |
|------|-----------|----------|
| `proto/storyteller.proto` | Core types stable | Medium |
| `storyteller-proto` crate (tonic-build) | Proto file | Small |
| Unary RPCs (CreateSession, SubmitInput) | Proto crate + persistence | Medium |
| Streaming RPC (ObservePipeline) | Proto crate + PhaseObserver | Medium |
| Session dump RPC | Phase 4 | Small |
| REST/SSE fallback in `storyteller-api` | Persistence | Medium |

### Phase 7: Scene Authoring and Content Pipeline

**Goal**: Move beyond hardcoded workshop data.

| Item | Depends on | Estimate |
|------|-----------|----------|
| Scene definition format (YAML/TOML) | Core types stable | Medium |
| Character sheet authoring format | Tensor schema | Medium |
| Content loader from `storyteller-data` | `STORYTELLER_DATA_PATH` (exists) | Medium |
| Authoring validation (schema check) | Definition formats | Small |
| Multiple scene support | Scene lifecycle systems | Large |

---

## Part VIII: Open Questions

### Resolved by This Branch

| Question | Resolution |
|----------|-----------|
| Classifier agent design | ML classification pipeline (DistilBERT ONNX) |
| Turn lifecycle model | `TurnCycleStage` (8 variants) + `ProvisionalStatus` (3 variants) |
| Entity promotion heuristics | Configurable `PromotionConfig` with threshold values |
| How Character Agents work in narrator-centric model | ML prediction models, not LLM agents |
| Reconciler role | Deterministic resolver (pass-through for now) |
| How to bridge async LLM in Bevy | Oneshot polling in `rendering_system`, not actor/channel |
| Multi-agent vs single-LLM | Single LLM (Narrator only), confirmed by latency testing |

### New Questions from This Branch

| Question | Context | Blocking? |
|----------|---------|-----------|
| Narrator deduplication thresholds | D.4 research | No (Phase 2) |
| 14B vs 7B model selection | Narrator rendering quality | No (testing needed) |
| Committed-turn classification on real prose | Templated training data F1=1.0, real prose unknown | No (Phase 2) |
| Optimal coreference window | 1-turn vs N-turn lookback | No (Phase 2) |
| AGE Cypher operator coverage | Path queries, centrality — untested | Yes (blocks Phase 3 approach) |
| Session-to-instance affinity | One engine per session vs shared | No (Phase 6) |
| Content resource store architecture | S3, local FS, pluggable backends | No (Phase 7) |

### Held Open (Philosophical/Design)

These don't block implementation but shape the system's character:

- **Configuration taxonomy**: Finite set of recognizable power configurations or continuous space?
- **The problem of evil**: Deliberate cruelty vs. structural harm — how the system models and presents malice
- **Power and play**: Player power, the good faith problem as a power problem
- **Connective space mass distribution**: Linear vs. diminishing returns for texture in connective scenes
- **Character complexity tiers**: Full/reduced/minimal tensor by role importance (performance vs. richness)

---

## Appendix: Codebase Health

### Test Distribution

| Crate | Tests | Coverage Focus |
|-------|-------|---------------|
| `storyteller-core` | 143 | Type construction, serde, weight computation, promotion logic, implication inference |
| `storyteller-engine` | 119 | Bevy system state transitions, ML inference, context assembly, rendering lifecycle |
| `storyteller-ml` | 74 | Feature encoding, training data generation, label constants |
| `storyteller-api` | 0 | (Stub — no behavior to test) |
| `storyteller-cli` | 0 | (Binary — tested via `play_scene_context` runs) |
| **Python** | 50 | Training pipeline, evaluation framework, ONNX export |

### Feature-Gated Test Tiers

```bash
cargo test --workspace                                    # 336 unit tests
cargo test --workspace --features test-ml-model           # + ONNX model integration
cargo test --workspace --features test-llm                # + Ollama LLM integration
cargo test --workspace --features test-ml-model,test-llm  # All tiers
```

### Lines of Code (Rust, excluding tests)

| Component | Lines | Status |
|-----------|-------|--------|
| Core types + traits | ~3,500 | Production-ready |
| Core promotion | ~1,063 | Production-ready |
| Core grammars | ~432 | Production-ready |
| Core database/graph | ~56 | Stubs |
| Engine inference | ~2,200 | Production-ready |
| Engine context | ~2,500 | Production-ready |
| Engine systems | ~1,100 | Production-ready |
| Engine agents | ~1,500 | Production-ready |
| Engine workshop | ~1,352 | Test data |
| API | ~148 | Stubs |
| CLI | ~517 | Working prototype |
| **Total** | ~14,600 | |

### Dependency Health

All workspace dependencies are aligned with tasker-core:
- `tokio 1.49`, `serde 1.0`, `sqlx 0.8`, `tracing 0.1`, `bevy_ecs 0.15`, `ort 2.0`
- Python: `torch`, `transformers`, `onnxruntime`, `datasets` (all current stable)
