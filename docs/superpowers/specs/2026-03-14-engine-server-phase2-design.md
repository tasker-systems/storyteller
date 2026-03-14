# Engine Server Phase 2: Client Library, CLI, and Pipeline Port

## Problem Statement

Phase 1 delivered a gRPC server with scaffolded RPCs, event persistence, and state management. But there's no typed client library, no way to run playtests programmatically, and the turn pipeline in `SubmitInput` uses placeholder data instead of the real LLM/ML pipeline that already works in the workshop's `commands.rs`.

Phase 2 stitches the client side together and ports the existing orchestration into the server — less invention, more assembly.

## Approach: Vertical Slice

Get a single end-to-end path working first (compose → playtest with real pipeline), then widen with composition cache and CLI ergonomics.

Order:
1. Rename + health types (mechanical)
2. `storyteller-client` — minimal, enough for compose + submit + health
3. `playtest` CLI subcommand — validates client against server with real pipeline
4. Port full turn pipeline from `commands.rs` into `SubmitInput`
5. Composition cache + `composer sync` + `compose` subcommand
6. Integration tests

## Design

### 1. Crate Rename: `storyteller-api` → `storyteller-server`

The `storyteller-api` crate is the full server infrastructure, not just an API wrapper. Renaming to `storyteller-server` reflects this.

**Scope:**
- Rename directory `crates/storyteller-api/` → `crates/storyteller-server/`
- Update package name in `Cargo.toml`
- Update workspace members in root `Cargo.toml`
- Update all `use storyteller_api::` imports (only `storyteller-cli` depends on it currently)
- Update CI workflow paths if any reference `storyteller-api`
- Update `build.rs` proto paths if any are relative to the crate directory

**Server binary:** `storyteller-server` gets its own binary target. The server starts itself — `cargo run -p storyteller-server` or the compiled binary directly. The CLI does not start the server.

**Dependency separation after rename:**

```
storyteller-server  ──→ storyteller-engine ──→ storyteller-core
                    ──→ storyteller-composer
                    ──→ storyteller-ml

storyteller-cli     ──→ storyteller-client ──→ storyteller-core (shared types only)

storyteller-workshop ──→ storyteller-client (future, Phase 3)
```

Server and CLI share nothing except core types and the proto contract. The CLI has no dependency on `storyteller-server`.

### 2. Health Types in `storyteller-core`

Generic health types that both server and client compile against, with a proto mirror on the wire.

**Rust types (`storyteller-core/src/types/health.rs`):**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHealth {
    pub status: HealthStatus,
    pub subsystems: Vec<SubsystemHealth>,
}
```

Core doesn't know what "narrator" or "postgres" means — the server populates subsystem entries, the client consumes them. New subsystems (e.g., database) don't require core changes.

**Proto update (`engine.proto`):**

Rename `CheckLlmStatus` RPC to `CheckHealth`. Response:

```protobuf
message HealthResponse {
  string status = 1;
  repeated SubsystemHealth subsystems = 2;
}

message SubsystemHealth {
  string name = 1;
  string status = 2;
  optional string message = 3;
}
```

Server populates subsystems: `narrator_llm`, `structured_llm`, `intent_llm`, `predictor`. Later: `database`.

### 3. `storyteller-client` Crate

**Location:** `crates/storyteller-client/`

**Dependencies:** `storyteller-core` (shared types), `tonic` (gRPC client stubs), `tokio`, `tracing`

**Build:** Own `build.rs` running `tonic-build` against `proto/storyteller/v1/`, generating client stubs.

**Public API (initial):**

```rust
pub struct StorytellerClient {
    engine: StorytellerEngineClient<Channel>,
    composer: ComposerServiceClient<Channel>,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub endpoint: String,   // default: "http://localhost:50051"
}

impl StorytellerClient {
    pub async fn connect(config: ClientConfig) -> Result<Self>;
    pub async fn check_health(&mut self) -> Result<ServerHealth>;

    // Engine RPCs
    pub async fn compose_scene(&mut self, request: ComposeSceneRequest)
        -> Result<tonic::Streaming<EngineEvent>>;
    pub async fn submit_input(&mut self, request: SubmitInputRequest)
        -> Result<tonic::Streaming<EngineEvent>>;

    // Composer RPCs (for cache sync)
    pub async fn list_genres(&mut self) -> Result<GenreList>;
    pub async fn get_archetypes_for_genre(&mut self, genre_id: &str) -> Result<ArchetypeList>;
    pub async fn get_profiles_for_genre(&mut self, genre_id: &str) -> Result<ProfileList>;
    pub async fn get_dynamics_for_genre(&mut self, genre_id: &str) -> Result<DynamicsList>;
    pub async fn get_names_for_genre(&mut self, genre_id: &str) -> Result<NameList>;
    pub async fn get_settings_for_genre(&mut self, genre_id: &str) -> Result<SettingList>;
}
```

**Error type:**

```rust
pub enum ClientError {
    ConnectionFailed(String),
    RpcError(tonic::Status),
    SubsystemUnavailable { subsystem: String, message: String },
}
```

**`check_health()`** provides two-layer health:
- **Transport level:** Can I reach the gRPC server? (`ConnectionFailed` if not)
- **Server level:** `ServerHealth` with per-subsystem status (maps proto → core types)

**Configuration from environment:**
- `STORYTELLER_SERVER_URL` → default `http://localhost:50051`

**What it doesn't do yet:** retry, reconnect, auth, event stream helpers. Follow-up concerns.

### 4. CLI Subcommands

**`storyteller-cli` depends only on `storyteller-client`.** No dependency on `storyteller-server`.

**Subcommand structure:**

```
storyteller-cli playtest [options]
storyteller-cli compose [options]
storyteller-cli composer sync
storyteller-cli composer list genres
storyteller-cli composer list archetypes --genre <slug>
storyteller-cli composer list profiles --genre <slug>
storyteller-cli composer list dynamics --genre <slug>
storyteller-cli composer list names --genre <slug>
storyteller-cli composer list settings --genre <slug>
```

#### Playtest

```bash
storyteller-cli playtest \
  --turns 10 \
  --player-model qwen2.5:7b-instruct \
  -f composition.json
```

Flow:
1. Connect to server via `StorytellerClient`
2. `check_health()` — fail early if narrator unavailable
3. Read composition file, send `ComposeScene`, consume stream until `NarratorComplete` (opening narration)
4. Extract protagonist from `SceneComposed` event for player simulation system prompt
5. For each turn up to `--turns`:
   - Send narrator output to player simulation LLM (same Ollama endpoint, configurable model via `--player-model`) with system prompt derived from protagonist character sheet (tensor axes, backstory, performance notes)
   - Send generated player input via `SubmitInput`, consume stream until `NarratorComplete`
6. Print summary: session ID, turn count, elapsed time

**Player simulation lives in the CLI crate** — it's a scripting/testing concern, not a server concern.

**Future extension (noted, not this phase):** Player simulation LLM may get its own endpoint configuration (`--player-ollama-url`) for load spreading or cloud provider abstraction.

#### Compose

```bash
storyteller-cli compose \
  --genre dark_fantasy \
  --profile tavern_encounter \
  --cast hero:protagonist,trickster:antagonist \
  --dynamics "hero,trickster:rivalry" \
  --output composition.json
```

Flow:
1. Connect to server via `StorytellerClient`
2. Resolve slugs → UUIDs from local composition cache
3. Build `ComposeSceneRequest` with resolved UUIDs
4. Call `ComposeScene`, consume stream, collect `SceneComposed` event
5. Write `composition.json` to `--output` (or stdout if not specified)

#### Composer Cache

**Location:** `.story/composition-cache/`

**Structure:**
```
.story/composition-cache/
├── genres.json               # [{ slug, entity_id, display_name }]
├── dark_fantasy/
│   ├── archetypes.json       # [{ slug, entity_id, display_name }]
│   ├── profiles.json
│   ├── dynamics.json
│   ├── names.json
│   └── settings.json
└── low_fantasy_folklore/
    └── ...
```

Slim index files — slug, entity_id, display_name only. Not full descriptor representations.

**`composer sync`:** Calls all ComposerService RPCs via `storyteller-client`, writes index files. Subsequent `compose` commands resolve slugs locally.

**`composer list`:** Reads from local cache, prints to stdout. Fails with "run `composer sync` first" if cache is missing.

### 5. Port Turn Pipeline into Server

The full turn pipeline already works in the workshop's `commands.rs`. Phase 2 ports this orchestration into the server's `SubmitInput` gRPC handler.

**Server startup constructs all providers** (same as the workshop does today):
- `narrator_llm`: `ExternalServerProvider` (Ollama, narrator model)
- `structured_llm`: `OllamaStructuredProvider` (decomposition model)
- `intent_llm`: `ExternalServerProvider` (Ollama, intent model)
- `predictor`: `CharacterPredictor` (ONNX, when model available)

**`SubmitInput` runs the same pipeline phases as `commands.rs`:**
1. Event decomposition via structured LLM
2. Character prediction via ONNX model
3. Action arbitration via world model constraints
4. Intent synthesis via intent LLM
5. Context assembly (preamble + journal + player input)
6. Narrator LLM call with assembled context

Each phase emits a streaming `EngineEvent` as it completes — the Phase 1 scaffolding already defines these event types and the streaming infrastructure.

**`ComposeScene`** also gets real narrator output — opening narration generated via `narrator_llm` instead of the current hardcoded string.

**What's new vs. what's ported:**
- Pipeline logic: ported from `commands.rs`
- Event emission at phase boundaries: new (uses Phase 1 scaffolding)
- Provider construction in server startup: new (mirrors workshop provider construction)
- Types and traits: already exist in `storyteller-engine` and `storyteller-core`

### 6. Testing Strategy

**Integration tests (behind `test-llm` feature flag):**

Client → server round-trip:
- Start server in test, connect via `StorytellerClient`
- `check_health()` returns healthy with expected subsystems
- `ComposeScene` streams events ending in real narrator output
- `SubmitInput` runs full pipeline and streams all phase events
- Session persists correctly (composition.json, events.jsonl, turns.jsonl)

Composer cache:
- `composer sync` populates `.story/composition-cache/` with expected structure
- Slug resolution works from cache
- `compose` with slug flags resolves and calls server successfully

Playtest end-to-end:
- Run a 2-turn playtest against a running server
- Verify session artifacts are produced with expected structure
- Verify player simulation generates contextually relevant input

**Unit tests (no feature gate):**
- `StorytellerClient` connection error handling (server not running)
- Composer cache read/write and slug resolution
- Player simulation prompt construction from character sheet data
- `ServerHealth` type serialization round-trip
- `ClientError` variant coverage

## Dependency Graph (Final)

```
storyteller-server  ──→ storyteller-engine ──→ storyteller-core
                    ──→ storyteller-composer ──→ storyteller-core
                    ──→ storyteller-ml ──→ storyteller-core

storyteller-client  ──→ storyteller-core (shared types: ServerHealth)

storyteller-cli     ──→ storyteller-client

storyteller-workshop ──→ storyteller-client (Phase 3, future)
```

## Out of Scope (Follow-up)

- Client retry/reconnect/auth
- Workshop conversion to gRPC client (Phase 3)
- Player simulation with separate Ollama endpoint
- `GetPredictionHistory`, `GetSessionEvents`, `StreamLogs` RPCs
- Narrator token streaming (currently one-shot)
- Database-backed descriptors
- Playtest analysis tooling
