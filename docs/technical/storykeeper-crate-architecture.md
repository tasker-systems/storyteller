# Storykeeper Crate Architecture

## Purpose

This document specifies the workspace crate structure required to support the Storykeeper API contract (`storykeeper-api-contract.md`). The primary decision: the Storykeeper's domain operations live in a dedicated `storyteller-storykeeper` crate, separate from both the type/trait layer (`storyteller-core`) and the Bevy ECS runtime (`storyteller-engine`).

This is not a refactor for the sake of tidiness. It is driven by a concrete requirement: **the Storykeeper's domain operations must be accessible outside the context of a running Bevy ECS instance.** Querying event history, inspecting relational webs, evaluating gate conditions, extracting training data — none of these require the turn cycle to be running. If the Storykeeper lives inside the engine, you must spin up Bevy to ask "what does character X know?" That is architecturally wrong.

### Relationship to Other Documents

- **`storykeeper-api-contract.md`** — Defines the domain operations (traits, inputs, outputs, semantics). This document specifies where those operations are implemented and how crates compose.
- **`CLAUDE.md`** (workspace root) — Current crate dependency graph. This document proposes a revision.
- **`infrastructure-architecture.md`** — System topology. The crate structure maps to deployment concerns discussed there.

---

## Revised Workspace Architecture

### Current Structure

```
storyteller-cli → storyteller-api → storyteller-engine → storyteller-core
                                                              ↑
                                            storyteller-ml ───┘
```

Five crates, strict downstream dependency flow. `storyteller-core` holds types, traits, errors, and stub modules for `database/` and `graph/` that have never been implemented. `storyteller-engine` holds everything else: Bevy ECS runtime, ML inference, context assembly, agents, and all orchestration logic.

### Proposed Structure

```
storyteller-core                types, traits, errors, domain contracts
       ↑
storyteller-storykeeper         domain operations, persistence, graph queries
       ↑          ↑
storyteller-engine │            Bevy ECS runtime, ML inference, turn cycle
       ↑          │
storyteller-api ──┘             HTTP/gRPC routes (depends on BOTH engine and storykeeper)
       ↑
storyteller-cli                 entry point, binary assembly
```

With `storyteller-ml` continuing as a sibling to `storyteller-engine`:

```
storyteller-core ←── storyteller-ml
       ↑
storyteller-storykeeper
       ↑
storyteller-engine (depends on storykeeper + ml)
```

The critical structural change is that `storyteller-api` has a **dual dependency**: it depends on the engine for active play operations, and it depends on the storykeeper directly for domain data operations that don't require a running game.

---

## The Deciding Requirement: Two API Responsibilities

The `storyteller-api` crate serves two fundamentally distinct purposes with different consumers, different auth models, and different runtime requirements.

### Responsibility 1: Game Play API

**Purpose**: Expose endpoints for playing the game from a client (TUI, web, mobile).

**Consumer**: A player interacting with an active session.

**Auth model**: Player identity → session binding → permissions scoped to their game. The player can submit input, receive narrative output, observe pipeline events, and control their session (pause, resume, dump, x-card). They cannot see other players' sessions or access raw narrative data.

**Runtime requirement**: Requires a running Bevy ECS engine instance with an active session. The game play API routes into the engine, which manages the turn cycle and delegates to the Storykeeper internally.

**Endpoints** (illustrative):
- `POST /api/v1/sessions` — Create/resume a session (starts the engine)
- `POST /api/v1/input` — Submit player input (enters the turn cycle)
- `GET /api/v1/sessions/:id/events` (SSE/WebSocket) — Stream pipeline events
- `POST /api/v1/sessions/:id/reject` — X-card / rejection flow
- `GET /api/v1/sessions/:id/dump` — Session dump

**Dependency path**: API → Engine → Storykeeper → Core

### Responsibility 2: Domain Data API

**Purpose**: Interact with the Storykeeper's domain data contract — the persistent narrative state that exists independent of any running game.

**Consumer**: Story designers, developers, analysts, observability tools, content authoring systems, training data pipelines.

**Auth model**: Role-based access scoped to story/world data, not to player sessions. A story designer might read/write scene definitions and character tensors. An analyst might read event history and relational web state. A developer might inspect information boundaries and gate conditions. None of these require a player session to exist.

**Runtime requirement**: Requires the Storykeeper (with a database connection) but NOT a running Bevy ECS instance. No turn cycle, no ML inference, no Narrator. Just domain queries and writes against persistent state.

**Endpoints** (illustrative):
- `GET /api/v1/entities/:id` — Entity state, promotion history, relational weight
- `GET /api/v1/entities/:id/relationships` — Relational web edges for an entity
- `GET /api/v1/entities/:id/information` — Information boundary state (what does this entity know?)
- `GET /api/v1/scenes/:id/events` — Event history for a scene
- `GET /api/v1/sessions/:id/turns` — Turn history for a session
- `GET /api/v1/gates` — Active gate conditions and proximity
- `GET /api/v1/graph/relational` — Relational web query interface
- `GET /api/v1/graph/narrative` — Narrative graph query interface
- `POST /api/v1/characters` — Create/update character definitions (authoring)
- `POST /api/v1/scenes` — Create/update scene definitions (authoring)

**Dependency path**: API → Storykeeper → Core (no engine)

### Structural Options

These two responsibilities could be organized as:

**Option A: One crate, two route groups, different middleware**
A single `storyteller-api` crate with game-play routes and domain-data routes mounted on the same Axum router, with different auth middleware per group. Simple to deploy, single binary. The dual dependency (engine + storykeeper) lives in one crate.

**Option B: One crate, two binaries**
A single `storyteller-api` crate with two `main.rs` binaries: one that starts the engine + game-play routes, another that starts only the storykeeper + domain-data routes. Shared route implementations, separate startup configurations. Allows deploying them independently.

**Option C: Two crates**
`storyteller-api-game` (depends on engine) and `storyteller-api-data` (depends on storykeeper). Maximum decoupling. The game API crate doesn't compile anything related to domain-data endpoints. Separate deployment units.

**Recommendation**: Start with **Option A** — it is the simplest and defers the deployment-topology decision. The two route groups are logically separated by path prefix and middleware, which makes splitting into Option B or C mechanical when the need arises. The key architectural enabler — the dual dependency on engine and storykeeper — is the same regardless of which option we choose.

The important thing is not which option we pick now, but that the **crate structure makes all three options possible**. Because the Storykeeper is its own crate, the domain-data routes never import anything from `storyteller-engine`. If we later split into two crates, the domain-data crate's dependency tree is clean: just `storyteller-storykeeper` and `storyteller-core`.

---

## Crate Contents

### storyteller-core (revised)

**Role**: Types, traits, errors, and domain contracts. The headless layer that every other crate depends on. No Bevy, no sqlx, no persistence runtime.

**What changes**:
- `traits/` gains `storykeeper.rs` — the three trait definitions (`StorykeeperQuery`, `StorykeeperCommit`, `StorykeeperLifecycle`) and their associated request/response types
- `types/` gains new domain types required by the Storykeeper traits: `TruthSet`, `Proposition`, `TriggerPredicate`, `EntityInformationState`, `InformationGate`, `RevelationEvent`, `FrictionModel`, `CascadePolicy`, `BoundedRelationalEdge`, `GateProximityItem`, etc.
- `database/` module **removed** — stubs migrate to `storyteller-storykeeper`
- `graph/` module **removed** — stubs migrate to `storyteller-storykeeper`

**What stays unchanged**:
- `types/` — all existing domain types (EntityRef, EventAtom, CompletedTurn, CharacterSheet, SceneData, etc.)
- `traits/` — existing traits (`LlmProvider`, `EmotionalGrammar`, `GameDesignSystem`, `PhaseObserver`)
- `errors.rs`, `config.rs`, `grammars/`, `promotion/`

**Dependencies**: serde, uuid, chrono, thiserror, async-trait, tracing (unchanged)

### storyteller-storykeeper (new)

**Role**: Domain-level data access. Implements the Storykeeper traits from core. Owns all persistence logic (event ledger, graph queries, truth set management, information boundaries, cascade/friction). Does NOT know about Bevy.

```
storyteller-storykeeper/src/
├── lib.rs                      # Public API: re-exports trait implementations
│
├── in_memory.rs                # InMemoryStorykeeper — test/prototype impl
│                               #   wraps current behavior behind trait surface
│
├── postgres.rs                 # PostgresStorykeeper — production impl (future)
│                               #   delegates to internal modules below
│
├── ledger/
│   ├── mod.rs
│   ├── writer.rs               # Append-only event ledger writes
│   ├── reader.rs               # Event queries: by turn, by entity, temporal predicates
│   └── command_source.rs       # Command sourcing: durable write before processing
│
├── graph/
│   ├── mod.rs
│   ├── relational_web.rs       # Relational web: edge queries, neighborhood, centrality
│   ├── narrative.rs            # Narrative graph: reachability, gravitational pull
│   ├── settings.rs             # Setting topology: adjacency, traversal
│   └── event_dag.rs            # Event dependency DAG: preconditions, enablement
│
├── truth_set/
│   ├── mod.rs
│   ├── store.rs                # Proposition storage, confidence tracking
│   ├── evaluation.rs           # Trigger predicate evaluation
│   └── temporal.rs             # Temporal index: After/Since/Sequence queries
│
├── information/
│   ├── mod.rs
│   ├── boundaries.rs           # Per-entity information state, boundary checks
│   ├── gates.rs                # Gate condition evaluation, revelation tracking
│   └── revelations.rs          # Revelation event recording and queries
│
├── friction/
│   ├── mod.rs
│   ├── cascade.rs              # Cascade propagation with attenuation/distortion
│   └── permeability.rs         # Edge permeability computation
│
├── checkpoint/
│   ├── mod.rs
│   ├── writer.rs               # State snapshot serialization
│   └── reader.rs               # Checkpoint loading, ledger delta replay
│
└── session/
    ├── mod.rs
    └── lifecycle.rs            # Session create/suspend/resume/end
```

**Dependencies**: `storyteller-core`, `sqlx` (PostgreSQL, when ready), `async-trait`, `tracing`, `serde`, `serde_json`

**Key implementation notes**:
- `InMemoryStorykeeper` maintains its own state (Vec<CompletedTurn>, event log, entity weights). This is NOT shared with Bevy Resources — it is the Storykeeper's authoritative copy.
- `PostgresStorykeeper` delegates to the internal modules (ledger/, graph/, etc.) for actual SQL/Cypher operations.
- The crate exports both the implementations AND convenience constructors. Consumers pick their implementation at configuration time.

### storyteller-engine (revised)

**Role**: Bevy ECS runtime, ML inference, turn cycle orchestration. The "hot" layer that manages in-flight game state. Delegates all persistence and domain queries to the Storykeeper.

**What changes**:
- NEW: `resources/storykeeper.rs` — Bevy Resource wrapping the Storykeeper trait objects
- `context/retrieval.rs` — refactored to call `StorykeeperQuery::query_entity_relevance`
- `context/mod.rs` — Tier 3 assembly delegates to Storykeeper
- `systems/turn_cycle.rs` — `commit_previous_system` calls `storykeeper.commit_turn()` instead of managing writes directly
- `plugin.rs` — Storykeeper resource injected during plugin registration

**What stays unchanged**:
- `components/` — all Bevy components and resources for turn state
- `agents/` — Narrator agent (still in the engine, still uses LlmProvider)
- `inference/` — ML inference (CharacterPredictor, EventClassifier)
- `context/preamble.rs`, `context/journal.rs`, `context/tokens.rs` — Tiers 1 and 2 (these are engine concerns, managed as Bevy Resources during play)
- `systems/rendering.rs` — async Narrator rendering
- `workshop/` — test data

**Dependencies**: adds `storyteller-storykeeper` (replaces direct use of `storyteller-core::database` and `storyteller-core::graph`)

### storyteller-api (revised)

**Role**: HTTP/gRPC routes. Now serves two distinct responsibilities.

**What changes**:
- NEW: `routes/domain/` — domain data routes (entity queries, graph queries, information boundaries, event history)
- NEW: `middleware/domain_auth.rs` — auth middleware for domain data access (role-based, not session-based)
- `state.rs` — `AppState` gains a direct Storykeeper handle alongside the engine handle
- Existing `routes/player.rs`, `routes/session.rs` — unchanged (game play routes)

**Dependencies**: adds `storyteller-storykeeper` (direct dependency for domain data routes)

---

## The Hot-State / Persistent-State Boundary

During active play, two representations of narrative state coexist:

1. **Hot state** in Bevy ECS Resources — the working draft for the current turn. Fast, mutable, scoped to the running engine instance.
2. **Persistent state** in the Storykeeper — the authoritative record. Durable (or simulated-durable for InMemoryStorykeeper), queryable, independent of Bevy.

These are not the same data. They serve different purposes and are authoritative at different times:

| State | Authoritative When | Managed By | Stored As |
|---|---|---|---|
| `TurnContext` (current turn in progress) | During the active turn cycle | Engine (Bevy Resource) | In-memory only — ephemeral, reset each turn |
| `PendingInput` (buffered player input) | Between input receipt and turn start | Engine (Bevy Resource) | Ephemeral until `source_command()` persists it |
| `JournalResource` (rolling scene journal) | During scene play | Engine (Bevy Resource) | In-memory; included in checkpoint at scene exit |
| `ActiveTurnStage` (FSM state) | During the active turn cycle | Engine (Bevy Resource) | In-memory only — engine orchestration concern |
| Event ledger (committed events) | Always (source of truth) | Storykeeper | In-memory (InMemory impl) or PostgreSQL |
| Entity weights / promotion state | After commitment | Storykeeper | In-memory or PostgreSQL |
| Relational web edges | After commitment | Storykeeper | In-memory or AGE graph |
| Truth set | After commitment | Storykeeper | In-memory; checkpointed at scene exit |
| Information boundaries | After commitment | Storykeeper | In-memory or PostgreSQL |

### The Mediation Flow

The engine mediates between hot and persistent state at defined transition points:

**Scene Entry** (persistent → hot):
```
engine calls storykeeper.enter_scene(scene_id, session_context)
    → storykeeper loads from persistent storage
    → returns SceneLoadResult (domain types from core)
engine populates Bevy Resources from SceneLoadResult
    → TurnHistory, JournalResource, scene components, cast entities
turn cycle begins — Bevy Systems work with Resources
```

**Turn Commitment** (hot → persistent):
```
engine builds CompletedTurn from Bevy Resources
    → TurnContext fields → CompletedTurn struct
engine calls storykeeper.commit_turn(completed_turn, scene_context, truth_set)
    → storykeeper appends events, updates entities, cascades changes
    → returns CommitResult
engine applies CommitResult to Bevy Resources if needed
    → e.g., entity promotions might spawn new Bevy entities
```

**Scene Exit** (hot → persistent, final flush):
```
engine calls storykeeper.exit_scene(scene_context, truth_set, deferred_queue)
    → storykeeper flushes all accumulated state
    → writes checkpoint
    → dispatches deferred work
engine despawns Bevy scene entities
```

**Command Sourcing** (hot → persistent, synchronous):
```
engine receives player input
engine calls storykeeper.source_command(input, session_context)
    → storykeeper writes to durable ledger
    → returns confirmation
engine proceeds with turn cycle only after confirmation
```

The Storykeeper never reads from Bevy Resources. The engine never reads from the Storykeeper's internal storage. They communicate exclusively through domain types defined in core.

---

## What This Means for Current Stubs

The `database/` and `graph/` stub modules currently in `storyteller-core` were placeholders for future persistence work. With the Storykeeper crate:

- `storyteller-core/src/database/ledger.rs` → `storyteller-storykeeper/src/ledger/`
- `storyteller-core/src/database/checkpoint.rs` → `storyteller-storykeeper/src/checkpoint/`
- `storyteller-core/src/database/session.rs` → `storyteller-storykeeper/src/session/`
- `storyteller-core/src/graph/relational_web.rs` → `storyteller-storykeeper/src/graph/relational_web.rs`
- `storyteller-core/src/graph/narrative.rs` → `storyteller-storykeeper/src/graph/narrative.rs`
- `storyteller-core/src/graph/settings.rs` → `storyteller-storykeeper/src/graph/settings.rs`

The stub documentation in these files remains valuable — it captures the design intent. The migration is: move the files, replace the module-level doc comments with actual implementations (behind the Storykeeper traits), and remove the empty modules from core.

---

## Implications for Implementation Sequence

The crate structure should be established early — before individual persistence features are built — because it determines where code lives and how it's tested.

**Proposed sequence**:

1. **Create the `storyteller-storykeeper` crate** — scaffold, Cargo.toml, empty modules
2. **Define Storykeeper traits in `storyteller-core`** — `StorykeeperQuery`, `StorykeeperCommit`, `StorykeeperLifecycle` with associated types
3. **Implement `InMemoryStorykeeper`** — wrap current in-memory behavior behind trait surface
4. **Refactor engine to use Storykeeper traits** — inject Storykeeper as Bevy Resource, refactor `commit_previous_system` and `assemble_context_system`
5. **Move stubs from core to storykeeper** — `database/` and `graph/` modules migrate
6. **Add truth set, information boundary, friction types to core** — new domain types
7. **Implement truth set management in storykeeper** — TruthSet operations
8. **Implement information boundary tracking in storykeeper** — boundary enforcement, gates, revelations
9. *(TAS-267 knowledge graph design happens in parallel with the above)*
10. **Design PostgreSQL schema** (TAS-242, revised) — derived from Storykeeper operations
11. **AGE spike** (TAS-244) — informed by graph query patterns from TAS-267
12. **Implement PostgresStorykeeper** — backed by real persistence

Steps 1-5 are mechanical refactoring that can happen immediately. Steps 6-8 add new domain concepts. Steps 10-12 are the persistence implementation that the current Linear tickets describe.

---

## Testing Strategy

The crate structure enables clean testing at each level:

**storyteller-storykeeper unit tests**: Test Storykeeper operations against `InMemoryStorykeeper`. No Bevy, no database, fast. This is where the bulk of domain logic tests live.

**storyteller-engine integration tests**: Test that Bevy Systems correctly call Storykeeper operations. Use `InMemoryStorykeeper` injected as a Bevy Resource. Verifies the mediation layer.

**storyteller-storykeeper integration tests** (feature-gated): Test `PostgresStorykeeper` against a real database. Behind a `test-database` feature flag, similar to existing `test-ml-model` and `test-llm` patterns.

**storyteller-api integration tests**: Test both route groups. Game-play routes use engine + InMemoryStorykeeper. Domain-data routes use InMemoryStorykeeper directly (no engine).

The trait-based approach means every test that doesn't specifically need PostgreSQL or Bevy can run with `InMemoryStorykeeper` — fast, deterministic, no external dependencies.

---

## Appendix: Dependency Table

| Crate | Depends On | Key Dependencies | Knows About Bevy? | Knows About sqlx? |
|---|---|---|---|---|
| `storyteller-core` | — | serde, uuid, chrono, async-trait | No | No |
| `storyteller-ml` | core | ort, ndarray | No | No |
| `storyteller-storykeeper` | core | sqlx (future), async-trait, serde_json | **No** | Yes (impl detail) |
| `storyteller-engine` | core, storykeeper, ml | bevy_app, bevy_ecs, tokio | **Yes** | No |
| `storyteller-api` | core, storykeeper, engine | axum, tokio, serde | No | No |
| `storyteller-cli` | core, engine, api | clap, tracing-subscriber | No | No |

The key column: the Storykeeper crate does not know about Bevy, and the engine crate does not know about sqlx. Each crate's external dependencies are appropriate to its role.
