# Engine Server and Playtest Harness Design

## Problem Statement

The storyteller engine's turn pipeline is tightly coupled to the Tauri workshop frontend. All orchestration logic — LLM provider construction, scene composition, turn pipeline execution, session persistence — lives in `commands.rs` (~1700 lines). This creates three problems:

1. **No programmatic access.** The only way to run a scene is through the Tauri UI. Validating pipeline changes requires manual playthroughs.
2. **Deployment blocker.** End users downloading Storyteller can't be expected to run Ollama, ONNX models, and PostgreSQL locally. The engine must run server-side.
3. **No shareable artifacts.** Scene compositions are ephemeral UI state, not portable files that others can play through.

## Design

### Architecture: Two-Process Model

```
┌─────────────────────────┐     ┌──────────────────────────┐
│  storyteller-api         │     │  storyteller-workshop     │
│  (axum + tonic)          │     │  (Tauri + Svelte)         │
│                          │     │                           │
│  gRPC service            │◄────│  gRPC client              │
│  Engine orchestration    │     │  UI state (ephemeral)     │
│  Session persistence     │     │  Debug inspector          │
│  LLM/ML providers        │     │                           │
│  Event streaming         │     └──────────────────────────┘
│                          │
│                          │     ┌──────────────────────────┐
│                          │     │  storyteller-cli           │
│                          │◄────│  gRPC client               │
│                          │     │  playtest subcommand       │
│                          │     │  composition generator     │
└─────────────────────────┘     └──────────────────────────┘
```

### Crate Changes

**`storyteller-api`** — Currently stub Axum routes. Becomes the gRPC engine server. Tonic service implementations, protobuf definitions, EngineState management, session persistence, event streaming. Depends on `storyteller-engine`, `storyteller-core`, `storyteller-composer`.

**`storyteller-composer`** — New crate. Owns the creative assembly layer: descriptor catalogs, genre/archetype/setting/dynamics selection, character generation, scene composition, goal intersection. Today hydrated from `storyteller-data/training-data/descriptors/`, eventually database-backed. Extracted from `storyteller-engine/src/scene_composer/`. Distinct from `storyteller-storykeeper` which owns information boundaries, event ledger, graph queries, and narrative state propagation.

**`storyteller-client`** — New crate. Typed gRPC client library wrapping tonic-generated stubs. Handles connection management. Later handles auth, retry. Any downstream consumer (CLI, workshop) depends on this rather than raw tonic.

**`storyteller-cli`** — Depends on `storyteller-client`. Server entry point (`storyteller-cli serve`). Also drives playtests (`storyteller-cli playtest`) and generates compositions (`storyteller-cli generate composition`).

**`storyteller-workshop`** — Depends on `storyteller-client`. Tauri thins down to UI state + gRPC calls. `commands.rs` becomes thin wrappers that forward gRPC event streams to Svelte via Tauri event channels. Debug inspector stays unchanged on the frontend.

**Dependency graph:**
```
storyteller-cli ──→ storyteller-client ──→ storyteller-core (shared types)

storyteller-workshop ──→ storyteller-client

storyteller-api ──→ storyteller-engine ──→ storyteller-core
               ──→ storyteller-composer ──→ storyteller-core
```

### gRPC Service Definition

**Service: `StorytellerEngine`**

Gameplay RPCs (server-streaming):
- `ComposeScene(SceneSelections) → stream EngineEvent` — compose scene, generate intentions, render opening. Streams phase events as they complete.
- `SubmitInput(PlayerInput) → stream EngineEvent` — full turn pipeline. Streams decomposition, prediction, arbitration, intent synthesis, narrator tokens.
- `ResumeSession(SessionId) → stream EngineEvent` — reload session, hydrate state, stream scene info + turn history.

Composer RPCs (unary, read-only):
- `ListGenres() → GenreList`
- `GetProfilesForGenre(genre_id) → ProfileList`
- `GetArchetypesForGenre(genre_id) → ArchetypeList`
- `GetDynamicsForGenre(genre_id, selected_archetypes) → DynamicsList`
- `GetNamesForGenre(genre_id) → NameList`

Query RPCs (unary):
- `ListSessions() → SessionList`
- `GetSceneState(SessionId) → SceneState` — characters, goals, intentions, current turn
- `CheckLlmStatus() → LlmStatus`
- `GetPredictionHistory(PredictionHistoryRequest) → PredictionHistory`

Event replay:
- `GetSessionEvents(SessionEventsRequest) → stream StoredEvent` — read back persisted events with optional type filter

Continuous push:
- `StreamLogs(LogFilter) → stream LogEntry` — tracing subscriber piped to gRPC

### EngineEvent Envelope

```protobuf
message EngineEvent {
  string event_id = 1;        // UUIDv7
  string session_id = 2;
  uint32 turn = 3;
  string timestamp = 4;       // RFC3339
  oneof payload {
    PhaseStarted phase_started = 10;
    DecompositionComplete decomposition = 11;
    PredictionComplete prediction = 12;
    ArbitrationComplete arbitration = 13;
    IntentSynthesisComplete intent_synthesis = 14;
    ContextAssembled context = 15;
    NarratorToken narrator_token = 16;
    NarratorComplete narrator_complete = 17;
    SceneComposed scene_composed = 18;
    GoalsGenerated goals = 19;
    TurnComplete turn_complete = 20;
    ErrorOccurred error = 30;
  }
}
```

Each event gets a UUIDv7 ID and is persisted to `events.jsonl`. The gRPC stream and the persisted file are the same shape — one is live, the other is at rest.

### Event Persistence Model

Three files per session:

**`composition.json`** — Write-once. All scene setup data: selections, scene, characters, goals, intentions. The complete "what we started with" artifact. Also the shareable format — `composition.json` files can be handed to other users to play through the same scene setup.

**`events.jsonl`** — Append-only. Every event the server emits, each with a UUIDv7 ID, event type, and payload. Source of truth. Events that don't belong to a turn (background work, system health) get `turn: null`.

**`turns.jsonl`** — Append-only. Turn index referencing events:
```json
{"turn": 0, "timestamp": "...", "player_input": null, "event_ids": ["019ce4c3-a1", "019ce4c3-a2"]}
{"turn": 1, "timestamp": "...", "player_input": "I examine the fence", "event_ids": ["019ce4c3-b1", ...]}
```

Session directory:
```
.story/sessions/{uuidv7}/
├── composition.json     # selections + scene + characters + goals + intentions
├── events.jsonl         # all events (source of truth)
└── turns.jsonl          # turn index into events
```

The `.json` file is write-once-read-many. The `.jsonl` files are append-only streams. Clean separation between structural data and temporal event data.

### Server-Side Engine State

**`EngineStateManager`** — Manages all active sessions with lock-free reads using `ArcSwap`.

```rust
pub struct EngineStateManager {
    sessions: DashMap<String, SessionState>,
}

struct SessionState {
    composition: Arc<Composition>,              // immutable after creation
    runtime: ArcSwap<RuntimeSnapshot>,          // lock-free reads
    write_handle: tokio::sync::Mutex<RuntimeMut>, // single writer
}

struct Composition {
    scene: SceneData,
    characters: Vec<CharacterSheet>,
    goals: Option<ComposedGoals>,
    intentions: Option<GeneratedIntentions>,
}

struct RuntimeSnapshot {
    journal: SceneJournal,
    turn_count: u32,
    player_entity_id: Option<EntityId>,
    prediction_history: PredictionHistory,
}
```

**Concurrency model**: The turn pipeline is inherently sequential per session. The write side takes `Mutex<RuntimeMut>`, progresses through pipeline phases, and publishes `Arc<RuntimeSnapshot>` via `ArcSwap` at natural phase boundaries (after prediction, after context assembly, after narrator). Readers call `runtime.load()` for an instant consistent snapshot — no lock, no contention, no decision needed. Each publish point is both "emit an event to the gRPC stream" and "update the readable snapshot."

**Shared providers** (not per-session):

```rust
pub struct EngineProviders {
    pub narrator_llm: Arc<dyn LlmProvider>,
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    pub predictor: Option<CharacterPredictor>,
    pub grammar: PlutchikWestern,
}
```

LLM providers and the ONNX model are stateless — shared across sessions. Multiple sessions can run concurrently against the same Ollama instance.

### Tauri Client Conversion

**`commands.rs` transforms from ~1700 lines of orchestration to thin gRPC wrappers:**

Each Tauri command:
1. Calls the gRPC client method via `storyteller-client`
2. Consumes the server-streaming response
3. Forwards `EngineEvent`s to Svelte via the existing Tauri event channel (`workshop:debug`)

The debug event system stays almost unchanged on the frontend. `EngineEvent` maps to the existing `DebugEvent` TypeScript union. The `applyDebugEvent` reducer in `logic.ts` and `DebugPanel.svelte` rendering stay as-is — the Tauri layer translates protobuf to the existing TypeScript types.

**What Tauri still owns:**
- Wizard UI state (step navigation, cast selection, dynamics pairing)
- Debug panel visibility, tab selection, log filters
- Input bar state, scroll position
- Offline input queueing (buffer locally if server unreachable, retry when reconnected)

**What moves to the server:**
- All LLM/ML provider construction and management
- Scene composition orchestration
- Turn pipeline execution
- Session persistence
- Goal/intention generation

Composer data for the wizard (genre list, archetype list, name pools, profiles, dynamics) hydrates via gRPC queries to the `ComposerService` RPCs rather than direct `SceneComposer` calls.

### Playtest Harness (CLI Subcommand)

Two CLI subcommands with separated responsibilities:

**Generate a composition:**
```bash
storyteller-cli generate composition \
  --genre dark_fantasy \
  --profile tavern_encounter \
  --cast hero:protagonist,trickster:antagonist \
  --seed 42 \
  > composition.json
```

Calls the composer service to build a complete `composition.json`. Pure function — descriptors in, composition out.

**Run a playtest:**
```bash
# From file
storyteller-cli playtest --turns 10 --player-model qwen2.5:7b-instruct -f composition.json

# From stdin
cat composition.json | storyteller-cli playtest --turns 10
```

The playtest loop:
1. Connects to the gRPC server via `storyteller-client`
2. Sends the composition via `ComposeScene`, consumes the event stream until opening prose arrives
3. For each turn up to `--turns`:
   - Takes the latest narrator output
   - Sends it to the player simulation LLM with a system prompt derived from the protagonist's character sheet (tensor axes, backstory, performance notes)
   - Sends the generated player input via `SubmitInput`, consumes the event stream
4. Prints a summary: turn count, total time, token usage, session ID
5. The server has already persisted `composition.json`, `events.jsonl`, `turns.jsonl`

`composition.json` is also the first shareable artifact — "here's a scene I set up, try playing through it."

### Model Configuration

Per-role model configuration, server-side, driven by environment variables:

| Role | Env var | Default |
|------|---------|---------|
| Narrator | `STORYTELLER_NARRATOR_MODEL` | `qwen2.5:14b` |
| Event decomposition | `STORYTELLER_DECOMPOSITION_MODEL` | `qwen2.5:3b-instruct` |
| Intent synthesis | `STORYTELLER_INTENT_MODEL` | `qwen2.5:3b-instruct` |
| Intention generation | `STORYTELLER_INTENTION_MODEL` | `qwen2.5:14b` |
| Ollama endpoint | `OLLAMA_URL` | `http://localhost:11434` |

Player simulation model is client-side (the playtest CLI owns it):

| Role | Env var / flag | Default |
|------|----------------|---------|
| Player simulation | `STORYTELLER_PLAYER_MODEL` / `--player-model` | `qwen2.5:7b-instruct` |

### Implementation Phasing

This is designed as one coherent architecture but implemented in phases. Each phase has a clear stopping point where the system is usable (though not necessarily via all clients).

**Phase 1: Engine Server** — Build the gRPC service in `storyteller-api`. Extract `storyteller-composer` from engine. Implement `EngineStateManager`, event persistence, core RPCs (`ComposeScene`, `SubmitInput`, `ResumeSession`). Server runs standalone, testable via `grpcurl` or a simple test client.

**Phase 2: Client Library + CLI** — Build `storyteller-client` crate. Add `serve` and `playtest` subcommands to `storyteller-cli`. `generate composition` for creating composition files. Automated playtests run end-to-end.

**Phase 3: Workshop Conversion** — Slim Tauri's `commands.rs` to gRPC client wrappers. Wire wizard data hydration through composer RPCs. Debug inspector receives events from gRPC stream instead of in-process emission. Frontend TypeScript/Svelte stays largely unchanged.

The system is in a non-viable state between Phase 1 start and Phase 3 completion (Tauri stops working when `commands.rs` orchestration is removed but not yet replaced with gRPC calls). This is acceptable at pre-alpha with no external users.

### Testing Strategy

- **gRPC service tests**: Integration tests in `storyteller-api` that start the server, call RPCs via `storyteller-client`, and verify event streams and persistence.
- **Composer extraction**: Unit tests move with the code from `storyteller-engine` to `storyteller-composer`. Existing tests should pass with import path changes only.
- **Event persistence**: Unit tests for `EventWriter`, `TurnWriter`, and round-trip read-back.
- **`EngineStateManager`**: Unit tests for SWMR semantics — concurrent reads during writes, snapshot consistency.
- **Playtest harness**: Integration test that runs a 2-turn playtest against a running server and verifies `composition.json`, `events.jsonl`, `turns.jsonl` are produced with expected structure.
- **Workshop conversion**: Manual verification via the workshop UI once Phase 3 completes. Existing vitest suite for frontend logic continues to pass (event types unchanged).

### Followup Work (Out of Scope)

- **Narrator streaming**: `LlmProvider` trait gains a streaming variant. `NarratorToken` events flow in real-time. Currently the narrator renders in one shot.
- **Database-backed descriptors**: `storyteller-composer` reads from PostgreSQL instead of JSON files. Authoring tools for creating/editing descriptors.
- **Authentication**: `storyteller-client` handles auth tokens. Server validates. Multi-user session isolation.
- **Warm cache intentions**: Pre-generated `GeneratedIntentions` stored in the composer's descriptor database, selected at composition time instead of generated via LLM.
- **Playtest analysis tooling**: Python scripts or notebooks that read `events.jsonl` and compute metrics (token drift, emotion trajectories, intent coherence).
