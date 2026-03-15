# Workshop Conversion (Phase 3) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the Tauri workshop from embedding the engine in-process to being a pure gRPC client via `storyteller-client`, and adopt `ts-rs` for automated Rust → TypeScript type generation.

**Architecture:** Replace ~1800 lines of pipeline orchestration in `commands.rs` with thin gRPC wrappers. Drop `storyteller-engine`, `storyteller-ml`, `storyteller-composer` dependencies. Add `storyteller-client` and `ts-rs`. Enrich server-side `EngineEvent` proto payloads so the debug inspector retains full functionality. Server required for workshop operation.

**Tech Stack:** Rust, Tauri 2, tonic (gRPC client via `storyteller-client`), ts-rs, Svelte 5 (Runes), TypeScript. Existing crates: `storyteller-core`, `storyteller-client`, `storyteller-server`.

**Spec:** `docs/superpowers/specs/2026-03-14-workshop-conversion-phase3-design.md`

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `proto/storyteller/v1/composer.proto` (modified) | Add `GetGenreOptions` RPC and `GenreOptions` message |
| `proto/storyteller/v1/engine.proto` (modified) | Enrich `EngineEvent` payloads, add `fields` to `LogEntry` |
| `crates/storyteller-workshop/src-tauri/src/commands.rs` (replaced) | Thin gRPC command wrappers |
| `crates/storyteller-workshop/src-tauri/src/types.rs` | `ts-rs` annotated structs for Tauri ↔ Svelte boundary |
| `crates/storyteller-workshop/src/lib/generated/*.ts` | Auto-generated TypeScript interfaces (checked in) |
| `crates/storyteller-client/src/client.rs` (modified) | Add `get_genre_options()`, `get_prediction_history()` methods |

### Modified Files

| File | Changes |
|------|---------|
| `crates/storyteller-workshop/src-tauri/Cargo.toml` | Drop engine/ml/composer/reqwest deps, add client + ts-rs |
| `crates/storyteller-workshop/src-tauri/src/lib.rs` | Replace managed state setup, register new commands |
| `crates/storyteller-server/src/grpc/composer_service.rs` | Implement `GetGenreOptions` handler |
| `crates/storyteller-server/src/grpc/engine_service.rs` | Enrich event emissions, implement `StreamLogs` + `GetPredictionHistory` |
| `crates/storyteller-server/src/server.rs` | Wire tracing layer for `StreamLogs` |
| `crates/storyteller-server/Cargo.toml` | Add tracing-subscriber dep if needed |
| `crates/storyteller-workshop/src/lib/api.ts` | Update invoke calls to use generated types |
| `crates/storyteller-workshop/src/lib/types.ts` | Slim to UI-only types, import generated types |
| `crates/storyteller-workshop/src/routes/+page.svelte` | Update health check, type imports |
| `crates/storyteller-workshop/src/lib/SceneSetup.svelte` | Use `getGenreOptions()` via updated API |
| `crates/storyteller-workshop/src/lib/DebugPanel.svelte` | Use generated debug event types |

### Deleted Files

| File | Reason |
|------|--------|
| `crates/storyteller-workshop/src-tauri/src/engine_state.rs` | Server owns session state |
| `crates/storyteller-workshop/src-tauri/src/session.rs` | Server owns persistence |
| `crates/storyteller-workshop/src-tauri/src/session_log.rs` | Server owns event persistence |
| `crates/storyteller-workshop/src-tauri/src/events.rs` | Replaced by EngineEvent stream forwarding |
| `crates/storyteller-workshop/src-tauri/src/tracing_layer.rs` | Replaced by StreamLogs RPC |

---

## Chunk 1: Server-Side Proto Enrichment and New RPCs

This chunk enriches the server before touching the workshop. After this chunk, the server emits richer events and supports the new RPCs the workshop will need.

### Task 1: Enrich `EngineEvent` proto payloads

**Files:**
- Modify: `proto/storyteller/v1/engine.proto`

The current proto payloads are too sparse for the debug inspector. Enrich them to carry the data the workshop's `DebugEvent` variants currently provide.

- [ ] **Step 1: Enrich NarratorComplete message**

In `proto/storyteller/v1/engine.proto`, replace line 112:
```protobuf
message NarratorComplete { string prose = 1; uint64 generation_ms = 2; }
```
with:
```protobuf
message NarratorComplete {
  string prose = 1;
  uint64 generation_ms = 2;
  string system_prompt = 3;
  string user_message = 4;
  string raw_response = 5;
  string model = 6;
  float temperature = 7;
  uint32 max_tokens = 8;
  uint32 tokens_used = 9;
}
```

- [ ] **Step 2: Enrich ContextAssembled message**

Replace line 110:
```protobuf
message ContextAssembled { uint32 preamble_tokens = 1; uint32 journal_tokens = 2; uint32 retrieved_tokens = 3; uint32 total_tokens = 4; }
```
with:
```protobuf
message ContextAssembled {
  uint32 preamble_tokens = 1;
  uint32 journal_tokens = 2;
  uint32 retrieved_tokens = 3;
  uint32 total_tokens = 4;
  string preamble_text = 5;
  string journal_text = 6;
  string retrieved_text = 7;
  uint64 timing_ms = 8;
}
```

- [ ] **Step 3: Enrich remaining event messages**

Replace lines 106-109:
```protobuf
message DecompositionComplete { string raw_json = 1; }
message PredictionComplete { string raw_json = 1; }
message ArbitrationComplete { string verdict = 1; string details = 2; }
message IntentSynthesisComplete { string intent_statements = 1; }
```
with:
```protobuf
message DecompositionComplete {
  string raw_json = 1;
  uint64 timing_ms = 2;
  string model = 3;
  optional string error = 4;
}
message PredictionComplete {
  string raw_json = 1;
  uint64 timing_ms = 2;
  bool model_loaded = 3;
}
message ArbitrationComplete {
  string verdict = 1;
  string details = 2;
  string player_input = 3;
  uint64 timing_ms = 4;
}
message IntentSynthesisComplete {
  string intent_statements = 1;
  uint64 timing_ms = 2;
}
```

- [ ] **Step 4: Fix GoalsGenerated character_drives to repeated string**

Replace line 114. Note: changing `optional string character_drives = 4` to `repeated string` requires a new field number to avoid proto wire-format incompatibility with any persisted events:
```protobuf
message GoalsGenerated {
  repeated string scene_goals = 1;
  repeated string character_goals = 2;
  optional string scene_direction = 3;
  reserved 4; // was: optional string character_drives (deprecated)
  optional string player_context = 5;
  uint64 timing_ms = 6;
  repeated string character_drives = 7;
}
```

- [ ] **Step 5: Add fields to LogEntry**

Replace line 126:
```protobuf
message LogEntry { string level = 1; string target = 2; string message = 3; string timestamp = 4; }
```
with:
```protobuf
message LogEntry {
  string level = 1;
  string target = 2;
  string message = 3;
  string timestamp = 4;
  map<string, string> fields = 5;
}
```

- [ ] **Step 6: Verify proto compiles**

Run: `cargo check -p storyteller-server -p storyteller-client`
Expected: Clean compilation. The enriched messages are backward-compatible (added fields only).

- [ ] **Step 7: Commit**

```bash
git add proto/
git commit -m "proto: enrich EngineEvent payloads and LogEntry for debug inspector"
```

### Task 2: Update server event emissions with enriched fields

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs`

The server's `SubmitInput` and `ComposeScene` handlers emit events with the current sparse fields. Update them to populate the new enriched fields.

- [ ] **Step 1: Study current event emission points**

Read `crates/storyteller-server/src/grpc/engine_service.rs`. Search for all `engine_event::Payload::` constructions. Map each to the enriched proto fields that need populating. The key emission points are in the `submit_input` handler (the turn pipeline) and the `compose_scene` handler (opening narration).

- [ ] **Step 2: Enrich NarratorComplete emissions**

Find all `NarratorComplete` event constructions. Update them to include the new fields. The narrator agent's render result and the assembled context provide the data:

```rust
engine_event::Payload::NarratorComplete(NarratorComplete {
    prose: rendering.text.clone(),
    generation_ms: narrator_ms,
    system_prompt: context.system_prompt.clone(),
    user_message: context.user_message.clone(),
    raw_response: rendering.text.clone(),
    model: "narrator".to_string(), // or extract from provider config
    temperature: 0.8,
    max_tokens: 0, // narrator doesn't set a max
    tokens_used: rendering.tokens_used as u32,
})
```

The exact field names will depend on what's available from `NarratorAgent::render()` and the assembled context. The implementor should trace the data flow from `assemble_narrator_context()` through `NarratorAgent::new()` to `render()` to identify what's accessible.

- [ ] **Step 3: Enrich remaining event emissions**

Update `DecompositionComplete`, `PredictionComplete`, `ArbitrationComplete`, `IntentSynthesisComplete`, and `ContextAssembled` emissions with the new fields. Each phase in `submit_input` already calculates timing — add `timing_ms` from the `Instant::now()` / `elapsed()` pattern already used. Add `model` and `error` fields where applicable. For `ContextAssembled`, add the text content from the assembled context sections.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p storyteller-server`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-server/
git commit -m "feat(server): populate enriched EngineEvent fields for debug inspector"
```

### Task 3: Add `GetGenreOptions` RPC to ComposerService

**Files:**
- Modify: `proto/storyteller/v1/composer.proto`
- Modify: `crates/storyteller-server/src/grpc/composer_service.rs`
- Modify: `crates/storyteller-client/src/client.rs`

- [ ] **Step 1: Add proto definition**

In `proto/storyteller/v1/composer.proto`, add to the `ComposerService` service block:

```protobuf
rpc GetGenreOptions(GenreOptionsRequest) returns (GenreOptions);
```

And add the new messages:

```protobuf
message GenreOptionsRequest {
  string genre_id = 1;
  repeated string selected_archetype_ids = 2;
}

message GenreOptions {
  repeated ArchetypeInfo archetypes = 1;
  repeated ProfileInfo profiles = 2;
  repeated DynamicInfo dynamics = 3;
  repeated string names = 4;
  repeated SettingInfo settings = 5;
}
```

- [ ] **Step 2: Implement server handler**

In `crates/storyteller-server/src/grpc/composer_service.rs`, add the `get_genre_options` method to the `ComposerService` trait implementation. It should call the existing per-category methods on `SceneComposer` and combine the results:

```rust
async fn get_genre_options(
    &self,
    request: Request<GenreOptionsRequest>,
) -> Result<Response<GenreOptions>, Status> {
    let req = request.into_inner();
    let composer = &self.composer;

    let archetypes = composer.archetypes_for_genre(&req.genre_id);
    let profiles = composer.profiles_for_genre(&req.genre_id);
    let dynamics = composer.dynamics_for_genre(&req.genre_id, &req.selected_archetype_ids);
    let names = composer.names_for_genre(&req.genre_id);
    let settings = composer.settings_for_genre(&req.genre_id);

    // Map to proto types using the same patterns as the existing per-category handlers
    // ...

    Ok(Response::new(GenreOptions {
        archetypes: /* mapped */,
        profiles: /* mapped */,
        dynamics: /* mapped */,
        names: /* mapped */,
        settings: /* mapped */,
    }))
}
```

Reference the existing `get_archetypes_for_genre`, `get_profiles_for_genre`, etc. handlers in the same file for the exact mapping patterns.

- [ ] **Step 3: Add client method**

In `crates/storyteller-client/src/client.rs`, add:

```rust
pub async fn get_genre_options(
    &mut self,
    genre_id: &str,
    selected_archetype_ids: Vec<String>,
) -> Result<crate::proto::GenreOptions, ClientError> {
    let response = self
        .composer
        .get_genre_options(crate::proto::GenreOptionsRequest {
            genre_id: genre_id.to_string(),
            selected_archetype_ids,
        })
        .await?;
    Ok(response.into_inner())
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p storyteller-server -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add proto/ crates/storyteller-server/ crates/storyteller-client/
git commit -m "feat: add GetGenreOptions combined RPC to ComposerService"
```

### Task 4: Implement `GetPredictionHistory` RPC

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs`
- Modify: `crates/storyteller-client/src/client.rs`

- [ ] **Step 1: Implement server handler**

In `engine_service.rs`, find the `get_prediction_history` method (currently returns `Status::unimplemented`). Replace with an implementation that reads from `EngineStateManager`:

```rust
async fn get_prediction_history(
    &self,
    request: Request<PredictionHistoryRequest>,
) -> Result<Response<PredictionHistoryResponse>, Status> {
    let req = request.into_inner();
    let snapshot = self
        .state_manager
        .get_runtime_snapshot(&req.session_id)
        .ok_or_else(|| Status::not_found("session not found"))?;

    let history = &snapshot.prediction_history;
    let raw_json = serde_json::to_string(history).unwrap_or_default();

    Ok(Response::new(PredictionHistoryResponse { raw_json }))
}
```

- [ ] **Step 2: Add client method**

In `crates/storyteller-client/src/client.rs`, add:

```rust
pub async fn get_prediction_history(
    &mut self,
    session_id: &str,
    from_turn: Option<u32>,
    to_turn: Option<u32>,
) -> Result<crate::proto::PredictionHistoryResponse, ClientError> {
    let response = self
        .engine
        .get_prediction_history(crate::proto::PredictionHistoryRequest {
            session_id: session_id.to_string(),
            from_turn,
            to_turn,
        })
        .await?;
    Ok(response.into_inner())
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-server -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-server/ crates/storyteller-client/
git commit -m "feat: implement GetPredictionHistory RPC (previously stubbed)"
```

### Task 5: Implement `StreamLogs` RPC

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs`
- Modify: `crates/storyteller-server/src/server.rs`
- Modify: `crates/storyteller-server/Cargo.toml` (if needed)

This is more complex than the other RPCs because it requires a server-side tracing subscriber that forwards log entries to gRPC stream clients.

- [ ] **Step 1: Add a broadcast channel for log entries**

In `crates/storyteller-server/src/server.rs` or a new `crates/storyteller-server/src/logging.rs`, create a `tokio::sync::broadcast` channel for log entries. The server's tracing subscriber will write to this channel, and `StreamLogs` clients will read from it.

```rust
use tokio::sync::broadcast;

pub type LogBroadcast = broadcast::Sender<crate::proto::LogEntry>;

pub fn create_log_broadcast() -> LogBroadcast {
    let (tx, _) = broadcast::channel(256);
    tx
}
```

- [ ] **Step 2: Create a tracing layer that writes to the broadcast channel**

Create a `tracing_subscriber::Layer` implementation that converts tracing events to `LogEntry` proto messages and sends them on the broadcast channel. Reference `crates/storyteller-workshop/src-tauri/src/tracing_layer.rs` for the pattern — the server version writes to a broadcast channel instead of a Tauri event channel.

- [ ] **Step 3: Wire the tracing layer in server startup**

In `crates/storyteller-server/src/server.rs` or `src/bin/main.rs`, add the broadcast tracing layer alongside the existing fmt layer. Store the `LogBroadcast` sender so it can be passed to `EngineServiceImpl`.

- [ ] **Step 4: Implement the `stream_logs` handler**

In `engine_service.rs`, replace the stubbed `stream_logs` method:

```rust
async fn stream_logs(
    &self,
    request: Request<LogFilter>,
) -> Result<Response<Self::StreamLogsStream>, Status> {
    let filter = request.into_inner();
    let mut rx = self.log_broadcast.subscribe();
    let (tx, rx_out) = mpsc::channel(32);

    tokio::spawn(async move {
        while let Ok(entry) = rx.recv().await {
            // Apply filter if specified
            if let Some(ref level) = filter.level {
                if entry.level != *level {
                    continue;
                }
            }
            if let Some(ref target) = filter.target {
                if !entry.target.starts_with(target.as_str()) {
                    continue;
                }
            }
            if tx.send(Ok(entry)).await.is_err() {
                break;
            }
        }
    });

    Ok(Response::new(ReceiverStream::new(rx_out)))
}
```

- [ ] **Step 5: Update `EngineServiceImpl` to hold the broadcast sender**

Add `log_broadcast: LogBroadcast` to `EngineServiceImpl`. Update `new()` and all construction sites.

- [ ] **Step 6: Add `stream_logs` client method**

In `crates/storyteller-client/src/client.rs`, add:

```rust
pub async fn stream_logs(
    &mut self,
    level: Option<String>,
    target: Option<String>,
) -> Result<tonic::Streaming<crate::proto::LogEntry>, ClientError> {
    let response = self
        .engine
        .stream_logs(crate::proto::LogFilter { level, target })
        .await?;
    Ok(response.into_inner())
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check -p storyteller-server -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 8: Commit**

```bash
git add crates/storyteller-server/ crates/storyteller-client/
git commit -m "feat: implement StreamLogs RPC with broadcast tracing layer and client method"
```

### Task 6: Server-side integration test for enriched events

**Files:**
- Modify: `crates/storyteller-server/tests/grpc_integration.rs`

- [ ] **Step 1: Add test for enriched NarratorComplete fields**

Add a test (behind `#[cfg(feature = "test-llm")]`) that calls `ComposeScene`, consumes the event stream, and verifies that `NarratorComplete` now includes `system_prompt`, `model`, and `tokens_used` fields (non-empty).

- [ ] **Step 2: Add test for GetGenreOptions**

Add a test that calls `GetGenreOptions` with a known genre and verifies that archetypes, profiles, dynamics, and names are all non-empty in the response.

- [ ] **Step 3: Verify tests pass**

Run: `cargo test -p storyteller-server` (unit tests, no server required)
Run: `cargo test -p storyteller-server --features test-llm` (with running server + Ollama)

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-server/
git commit -m "test(server): add integration tests for enriched events and GetGenreOptions"
```

---

## Chunk 2: Workshop Rust Backend — ts-rs Types and Thin Commands

This chunk replaces the workshop's Rust backend. After this chunk, the workshop compiles against `storyteller-client` with `ts-rs` generated types.

### Task 7: Add `ts-rs` to workspace and workshop dependencies

**Files:**
- Modify: `Cargo.toml` (root workspace)
- Modify: `crates/storyteller-workshop/src-tauri/Cargo.toml`

- [ ] **Step 1: Add ts-rs to workspace dependencies**

In the root `Cargo.toml` under `[workspace.dependencies]`, add:
```toml
ts-rs = { version = "10", features = ["serde-compat"] }
```

Check the latest `ts-rs` version on crates.io — version 10 is the latest major at time of writing. The `serde-compat` feature ensures `#[serde(rename_all)]` attributes are respected in generated TypeScript.

- [ ] **Step 2: Update workshop Cargo.toml**

In `crates/storyteller-workshop/src-tauri/Cargo.toml`:

Replace the dependency section. Remove:
- `storyteller-engine`
- `storyteller-ml`
- `reqwest`
- `rand`

Add:
- `storyteller-client = { path = "../../storyteller-client", version = "=0.1.0" }`
- `ts-rs = { workspace = true }`

Keep:
- `storyteller-core` (shared types)
- `tauri`, `tauri-plugin-opener`
- `serde`, `serde_json`, `tokio`, `chrono`, `uuid`
- `tracing`, `tracing-subscriber`
- `dotenvy`

Note: Do NOT remove `storyteller-composer` yet if any types are imported. Check imports first. The workshop may import `SceneSelections` or `ComposedGoals` from composer — if so, these types should come from proto/client instead.

- [ ] **Step 3: Verify workspace resolves**

Run: `cargo check -p storyteller-workshop` (or whatever the workshop's package name is in Cargo.toml)
Expected: Will likely fail with import errors — that's expected. We just need the dependency resolution to succeed.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/storyteller-workshop/src-tauri/Cargo.toml Cargo.lock
git commit -m "build: add ts-rs to workspace, update workshop dependencies"
```

### Task 8: Create `types.rs` with ts-rs annotated structs

**Files:**
- Create: `crates/storyteller-workshop/src-tauri/src/types.rs`

Define all structs that cross the Tauri invoke boundary. Each derives `TS`, `Serialize`, `Deserialize`, and `Debug`. The `#[ts(export, export_to = "...")]` attribute generates TypeScript files.

- [ ] **Step 1: Create types.rs with gameplay types**

```rust
//! Types for the Tauri ↔ Svelte boundary.
//!
//! Every struct here derives `ts_rs::TS` and generates a TypeScript interface
//! in `src/lib/generated/`. These are the source of truth — the generated `.ts`
//! files replace the hand-authored `types.ts` interfaces.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Scene information returned after composition.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct SceneInfo {
    pub session_id: String,
    pub title: String,
    pub setting_description: String,
    pub cast: Vec<String>,
    pub opening_prose: String,
}

/// Result of a player turn.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct TurnResult {
    pub turn: u32,
    pub narrator_prose: String,
    pub timing: TurnTiming,
    pub context_tokens: ContextTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct TurnTiming {
    pub prediction_ms: u64,
    pub assembly_ms: u64,
    pub narrator_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct ContextTokens {
    pub preamble: u32,
    pub journal: u32,
    pub retrieved: u32,
    pub total: u32,
}

/// Server health report.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct HealthReport {
    pub status: String,
    pub subsystems: Vec<SubsystemStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct SubsystemStatus {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

/// Genre summary for the wizard catalog.
/// Note: `id` maps from `slug` (not `entity_id`) because downstream calls
/// (get_genre_options, compose_scene) use slugs for matching.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct GenreSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub archetype_count: u32,
    pub profile_count: u32,
    pub dynamic_count: u32,
}

/// Combined genre options for a wizard step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct GenreOptionsResult {
    pub archetypes: Vec<ArchetypeSummary>,
    pub profiles: Vec<ProfileSummary>,
    pub dynamics: Vec<DynamicSummary>,
    pub names: Vec<String>,
    pub settings: Vec<SettingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct ArchetypeSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub scene_type: String,
    pub tension_min: f64,
    pub tension_max: f64,
    pub cast_size_min: u32,
    pub cast_size_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct DynamicSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub role_a: String,
    pub role_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct SettingSummary {
    pub id: String,
    pub name: String,
}

/// Session summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct SessionInfo {
    pub session_id: String,
    pub genre: String,
    pub profile: String,
    pub title: String,
    pub cast_names: Vec<String>,
    pub turn_count: u32,
    pub created_at: String,
}

/// Turn summary for session resume.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct TurnSummary {
    pub turn: u32,
    pub player_input: Option<String>,
    pub narrator_output: String,
}

/// Resume result with scene info and turn history.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct ResumeResult {
    pub scene_info: SceneInfo,
    pub turns: Vec<TurnSummary>,
}

/// Scene selections for composition (received from Svelte wizard).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct SceneSelections {
    pub genre_id: String,
    pub profile_id: String,
    pub cast: Vec<CastSelection>,
    pub dynamics: Vec<DynamicSelection>,
    pub setting_override: Option<String>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct CastSelection {
    pub archetype_id: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct DynamicSelection {
    pub dynamic_id: String,
    pub cast_index_a: u32,
    pub cast_index_b: u32,
}
```

- [ ] **Step 2: Add debug event types to types.rs**

Append to `types.rs` — these replace the `events.rs` `DebugEvent` enum. They're emitted on the `workshop:debug` Tauri event channel:

```rust
/// Debug events emitted on the "workshop:debug" Tauri event channel.
///
/// Each variant is translated from an `EngineEvent` proto payload in the
/// streaming command handlers.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "../src/lib/generated/")]
pub enum DebugEvent {
    #[serde(rename = "phase_started")]
    PhaseStarted { turn: u32, phase: String },

    #[serde(rename = "prediction_complete")]
    PredictionComplete {
        turn: u32,
        raw_json: String,
        timing_ms: u64,
        model_loaded: bool,
    },

    #[serde(rename = "context_assembled")]
    ContextAssembled {
        turn: u32,
        preamble_text: String,
        journal_text: String,
        retrieved_text: String,
        token_counts: ContextTokens,
        timing_ms: u64,
    },

    #[serde(rename = "narrator_complete")]
    NarratorComplete {
        turn: u32,
        system_prompt: String,
        user_message: String,
        raw_response: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        tokens_used: u32,
        timing_ms: u64,
    },

    #[serde(rename = "event_decomposed")]
    EventDecomposed {
        turn: u32,
        raw_json: Option<String>,
        timing_ms: u64,
        model: String,
        error: Option<String>,
    },

    #[serde(rename = "intent_synthesized")]
    IntentSynthesized {
        turn: u32,
        intent_statements: String,
        timing_ms: u64,
    },

    #[serde(rename = "action_arbitrated")]
    ActionArbitrated {
        turn: u32,
        verdict: String,
        details: String,
        player_input: String,
        timing_ms: u64,
    },

    #[serde(rename = "goals_generated")]
    GoalsGenerated {
        turn: u32,
        scene_goals: Vec<String>,
        character_goals: Vec<String>,
        scene_direction: Option<String>,
        character_drives: Vec<String>,
        player_context: Option<String>,
        timing_ms: u64,
    },

    #[serde(rename = "error")]
    Error {
        turn: u32,
        phase: String,
        message: String,
    },
}

/// Log entry from the server's tracing stream.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/lib/generated/")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: std::collections::HashMap<String, String>,
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/types.rs
git commit -m "feat(workshop): add ts-rs annotated types for Tauri-Svelte boundary"
```

### Task 9: Write new thin `commands.rs`

**Files:**
- Replace: `crates/storyteller-workshop/src-tauri/src/commands.rs`

This is the core of the conversion. Delete the old 1803-line file and write the new thin gRPC wrapper layer.

- [ ] **Step 1: Delete old files and create new commands.rs**

Delete:
- `crates/storyteller-workshop/src-tauri/src/commands.rs`
- `crates/storyteller-workshop/src-tauri/src/engine_state.rs`
- `crates/storyteller-workshop/src-tauri/src/session.rs`
- `crates/storyteller-workshop/src-tauri/src/session_log.rs`
- `crates/storyteller-workshop/src-tauri/src/events.rs`
- `crates/storyteller-workshop/src-tauri/src/tracing_layer.rs`

Create new `crates/storyteller-workshop/src-tauri/src/commands.rs`:

```rust
//! Thin gRPC command wrappers for the Tauri frontend.
//!
//! Each command calls `storyteller-client`, translates the proto response
//! to a `ts-rs` annotated struct, and returns it. Streaming commands
//! forward `EngineEvent`s to the `workshop:debug` Tauri event channel.

use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::{AppHandle, Emitter, State};
use storyteller_client::{
    engine_event, ClientConfig, ComposeSceneRequest, StorytellerClient, SubmitInputRequest,
    ResumeSessionRequest,
};

use crate::types::*;

/// Debug event channel name.
const DEBUG_EVENT_CHANNEL: &str = "workshop:debug";
/// Log event channel name.
const LOG_EVENT_CHANNEL: &str = "workshop:logs";

pub type ClientState = Arc<Mutex<StorytellerClient>>;

#[tauri::command]
pub async fn check_health(
    client: State<'_, ClientState>,
) -> Result<HealthReport, String> {
    let mut client = client.lock().await;
    let health = client.check_health().await.map_err(|e| e.to_string())?;

    Ok(HealthReport {
        status: health.status.to_string(),
        subsystems: health
            .subsystems
            .iter()
            .map(|s| SubsystemStatus {
                name: s.name.clone(),
                status: s.status.to_string(),
                message: s.message.clone(),
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn load_catalog(
    client: State<'_, ClientState>,
) -> Result<Vec<GenreSummary>, String> {
    let mut client = client.lock().await;
    let genres = client.list_genres().await.map_err(|e| e.to_string())?;

    Ok(genres
        .genres
        .iter()
        .map(|g| GenreSummary {
            id: g.slug.clone(), // Use slug, not entity_id — downstream calls match on slug
            display_name: g.display_name.clone(),
            description: g.description.clone(),
            archetype_count: g.archetype_count,
            profile_count: g.profile_count,
            dynamic_count: g.dynamic_count,
        })
        .collect())
}

#[tauri::command]
pub async fn get_genre_options(
    genre_id: String,
    selected_archetypes: Vec<String>,
    client: State<'_, ClientState>,
) -> Result<GenreOptionsResult, String> {
    let mut client = client.lock().await;
    let options = client
        .get_genre_options(&genre_id, selected_archetypes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GenreOptionsResult {
        archetypes: options.archetypes.iter().map(|a| ArchetypeSummary {
            id: a.slug.clone(), // Use slug for downstream matching
            display_name: a.display_name.clone(),
            description: a.description.clone(),
        }).collect(),
        profiles: options.profiles.iter().map(|p| ProfileSummary {
            id: p.slug.clone(),
            display_name: p.display_name.clone(),
            description: p.description.clone(),
            scene_type: p.scene_type.clone(),
            tension_min: p.tension_min,
            tension_max: p.tension_max,
            cast_size_min: p.cast_size_min,
            cast_size_max: p.cast_size_max,
        }).collect(),
        dynamics: options.dynamics.iter().map(|d| DynamicSummary {
            id: d.slug.clone(),
            display_name: d.display_name.clone(),
            description: d.description.clone(),
            role_a: d.role_a.clone(),
            role_b: d.role_b.clone(),
        }).collect(),
        names: options.names.clone(),
        settings: options.settings.iter().map(|s| SettingSummary {
            id: s.profile_id.clone(),
            name: s.name.clone(),
        }).collect(),
    })
}

#[tauri::command]
pub async fn compose_scene(
    selections: SceneSelections,
    app: AppHandle,
    client: State<'_, ClientState>,
) -> Result<SceneInfo, String> {
    let request = ComposeSceneRequest {
        genre_id: selections.genre_id,
        profile_id: selections.profile_id,
        cast: selections.cast.iter().map(|c| storyteller_client::CastMember {
            archetype_id: c.archetype_id.clone(),
            name: c.name.clone(),
            role: c.role.clone(),
        }).collect(),
        dynamics: selections.dynamics.iter().map(|d| storyteller_client::DynamicPairing {
            dynamic_id: d.dynamic_id.clone(),
            cast_index_a: d.cast_index_a,
            cast_index_b: d.cast_index_b,
        }).collect(),
        seed: selections.seed,
        title_override: None,
        setting_override: selections.setting_override,
    };

    let mut stream = {
        let mut client = client.lock().await;
        client.compose_scene(request).await.map_err(|e| e.to_string())?
    };

    let mut scene_info = SceneInfo {
        session_id: String::new(),
        title: String::new(),
        setting_description: String::new(),
        cast: vec![],
        opening_prose: String::new(),
    };

    while let Some(event) = stream.message().await.map_err(|e| e.to_string())? {
        let turn = event.turn.unwrap_or(0);
        scene_info.session_id = event.session_id.clone();

        if let Some(payload) = &event.payload {
            // Forward debug events
            let debug_event = translate_engine_event(turn, payload);
            if let Some(de) = debug_event {
                let _ = app.emit(DEBUG_EVENT_CHANNEL, &de);
            }

            // Accumulate result
            match payload {
                engine_event::Payload::SceneComposed(scene) => {
                    scene_info.title = scene.title.clone();
                    scene_info.setting_description = scene.setting_description.clone();
                    scene_info.cast = scene.cast_names.clone();
                }
                engine_event::Payload::NarratorComplete(narrator) => {
                    scene_info.opening_prose = narrator.prose.clone();
                }
                engine_event::Payload::Error(err) => {
                    return Err(format!("{}: {}", err.phase, err.message));
                }
                _ => {}
            }
        }
    }

    Ok(scene_info)
}

#[tauri::command]
pub async fn submit_input(
    session_id: String,
    input: String,
    app: AppHandle,
    client: State<'_, ClientState>,
) -> Result<TurnResult, String> {
    let request = SubmitInputRequest {
        session_id,
        input,
    };

    let mut stream = {
        let mut client = client.lock().await;
        client.submit_input(request).await.map_err(|e| e.to_string())?
    };

    let mut result = TurnResult {
        turn: 0,
        narrator_prose: String::new(),
        timing: TurnTiming {
            prediction_ms: 0,
            assembly_ms: 0,
            narrator_ms: 0,
        },
        context_tokens: ContextTokens {
            preamble: 0,
            journal: 0,
            retrieved: 0,
            total: 0,
        },
    };

    while let Some(event) = stream.message().await.map_err(|e| e.to_string())? {
        let turn = event.turn.unwrap_or(0);
        result.turn = turn;

        if let Some(payload) = &event.payload {
            let debug_event = translate_engine_event(turn, payload);
            if let Some(de) = debug_event {
                let _ = app.emit(DEBUG_EVENT_CHANNEL, &de);
            }

            match payload {
                engine_event::Payload::NarratorComplete(narrator) => {
                    result.narrator_prose = narrator.prose.clone();
                    result.timing.narrator_ms = narrator.generation_ms;
                }
                engine_event::Payload::Context(ctx) => {
                    result.context_tokens = ContextTokens {
                        preamble: ctx.preamble_tokens,
                        journal: ctx.journal_tokens,
                        retrieved: ctx.retrieved_tokens,
                        total: ctx.total_tokens,
                    };
                    result.timing.assembly_ms = ctx.timing_ms;
                }
                engine_event::Payload::Prediction(pred) => {
                    result.timing.prediction_ms = pred.timing_ms;
                }
                engine_event::Payload::Error(err) => {
                    return Err(format!("{}: {}", err.phase, err.message));
                }
                _ => {}
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn list_sessions(
    client: State<'_, ClientState>,
) -> Result<Vec<SessionInfo>, String> {
    let mut client = client.lock().await;
    let sessions = client.list_sessions().await.map_err(|e| e.to_string())?;

    Ok(sessions
        .sessions
        .iter()
        .map(|s| SessionInfo {
            session_id: s.session_id.clone(),
            genre: s.genre.clone(),
            profile: s.profile.clone(),
            title: s.title.clone(),
            cast_names: s.cast_names.clone(),
            turn_count: s.turn_count,
            created_at: s.created_at.clone(),
        })
        .collect())
}

#[tauri::command]
pub async fn resume_session(
    session_id: String,
    app: AppHandle,
    client: State<'_, ClientState>,
) -> Result<ResumeResult, String> {
    let request = ResumeSessionRequest {
        session_id,
    };

    let mut stream = {
        let mut client = client.lock().await;
        client.resume_session(request).await.map_err(|e| e.to_string())?
    };

    let mut scene_info = SceneInfo {
        session_id: String::new(),
        title: String::new(),
        setting_description: String::new(),
        cast: vec![],
        opening_prose: String::new(),
    };
    let mut turns = Vec::new();

    while let Some(event) = stream.message().await.map_err(|e| e.to_string())? {
        let turn = event.turn.unwrap_or(0);
        scene_info.session_id = event.session_id.clone();

        if let Some(payload) = &event.payload {
            let debug_event = translate_engine_event(turn, payload);
            if let Some(de) = debug_event {
                let _ = app.emit(DEBUG_EVENT_CHANNEL, &de);
            }

            match payload {
                engine_event::Payload::SceneComposed(scene) => {
                    scene_info.title = scene.title.clone();
                    scene_info.setting_description = scene.setting_description.clone();
                    scene_info.cast = scene.cast_names.clone();
                }
                engine_event::Payload::NarratorComplete(narrator) => {
                    if turn == 0 {
                        scene_info.opening_prose = narrator.prose.clone();
                    }
                    // TODO: ResumeSession may stream turn summaries via a different mechanism
                }
                _ => {}
            }
        }
    }

    Ok(ResumeResult {
        scene_info,
        turns,
    })
}

#[tauri::command]
pub async fn get_scene_state(
    session_id: String,
    client: State<'_, ClientState>,
) -> Result<serde_json::Value, String> {
    let mut client = client.lock().await;
    let request = storyteller_client::GetSceneStateRequest {
        session_id,
    };
    let state = client
        .get_scene_state(request)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::to_value(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prediction_history(
    session_id: String,
    client: State<'_, ClientState>,
) -> Result<serde_json::Value, String> {
    let mut client = client.lock().await;
    let history = client
        .get_prediction_history(&session_id, None, None)
        .await
        .map_err(|e| e.to_string())?;
    // raw_json is already a JSON string
    serde_json::from_str(&history.raw_json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// EngineEvent → DebugEvent translation
// ---------------------------------------------------------------------------

fn translate_engine_event(turn: u32, payload: &engine_event::Payload) -> Option<DebugEvent> {
    match payload {
        engine_event::Payload::PhaseStarted(p) => Some(DebugEvent::PhaseStarted {
            turn,
            phase: p.phase.clone(),
        }),
        engine_event::Payload::Prediction(p) => Some(DebugEvent::PredictionComplete {
            turn,
            raw_json: p.raw_json.clone(),
            timing_ms: p.timing_ms,
            model_loaded: p.model_loaded,
        }),
        engine_event::Payload::Context(c) => Some(DebugEvent::ContextAssembled {
            turn,
            preamble_text: c.preamble_text.clone(),
            journal_text: c.journal_text.clone(),
            retrieved_text: c.retrieved_text.clone(),
            token_counts: ContextTokens {
                preamble: c.preamble_tokens,
                journal: c.journal_tokens,
                retrieved: c.retrieved_tokens,
                total: c.total_tokens,
            },
            timing_ms: c.timing_ms,
        }),
        engine_event::Payload::NarratorComplete(n) => Some(DebugEvent::NarratorComplete {
            turn,
            system_prompt: n.system_prompt.clone(),
            user_message: n.user_message.clone(),
            raw_response: n.raw_response.clone(),
            model: n.model.clone(),
            temperature: n.temperature,
            max_tokens: n.max_tokens,
            tokens_used: n.tokens_used,
            timing_ms: n.generation_ms,
        }),
        engine_event::Payload::Decomposition(d) => Some(DebugEvent::EventDecomposed {
            turn,
            raw_json: if d.raw_json.is_empty() { None } else { Some(d.raw_json.clone()) },
            timing_ms: d.timing_ms,
            model: d.model.clone(),
            error: d.error.clone(),
        }),
        engine_event::Payload::IntentSynthesis(i) => Some(DebugEvent::IntentSynthesized {
            turn,
            intent_statements: i.intent_statements.clone(),
            timing_ms: i.timing_ms,
        }),
        engine_event::Payload::Arbitration(a) => Some(DebugEvent::ActionArbitrated {
            turn,
            verdict: a.verdict.clone(),
            details: a.details.clone(),
            player_input: a.player_input.clone(),
            timing_ms: a.timing_ms,
        }),
        engine_event::Payload::Goals(g) => Some(DebugEvent::GoalsGenerated {
            turn,
            scene_goals: g.scene_goals.clone(),
            character_goals: g.character_goals.clone(),
            scene_direction: g.scene_direction.clone(),
            character_drives: g.character_drives.clone(),
            player_context: g.player_context.clone(),
            timing_ms: g.timing_ms,
        }),
        engine_event::Payload::Error(e) => Some(DebugEvent::Error {
            turn,
            phase: e.phase.clone(),
            message: e.message.clone(),
        }),
        // SceneComposed, TurnComplete, NarratorToken — handled by accumulation, not debug
        _ => None,
    }
}
```

- [ ] **Step 2: Verify the command signatures match what Svelte expects**

Cross-reference each `#[tauri::command]` function name with the `invoke()` calls in `src/lib/api.ts`. The Tauri command name is derived from the function name (snake_case).

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/
git commit -m "feat(workshop): replace in-process orchestration with thin gRPC command wrappers"
```

### Task 10: Update `lib.rs` to wire new commands and client state

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs`

- [ ] **Step 1: Replace lib.rs**

Replace the full contents of `lib.rs` with:

```rust
mod commands;
mod types;

use std::sync::Arc;
use tokio::sync::Mutex;
use storyteller_client::{ClientConfig, StorytellerClient};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Tracing for the workshop process itself (not the server)
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info".into()),
                )
                .compact()
                .init();

            dotenvy::dotenv().ok();

            // Connect to the storyteller server
            let config = ClientConfig::from_env();
            let rt = tokio::runtime::Handle::current();
            let client = rt.block_on(async {
                StorytellerClient::connect(config).await
            });

            match client {
                Ok(c) => {
                    app.manage(Arc::new(Mutex::new(c)) as commands::ClientState);
                    tracing::info!("Connected to storyteller server");
                }
                Err(e) => {
                    tracing::error!("Failed to connect to server: {e}");
                    // We require a running server at pre-alpha.
                    panic!("Storyteller server is required. Start it with: cargo run -p storyteller-server");
                }
            }

            // Start log streaming from server → workshop:logs event channel
            let app_handle = app.handle().clone();
            let log_config = ClientConfig::from_env();
            tokio::spawn(async move {
                // Open a separate client connection for the long-lived log stream
                let mut log_client = match StorytellerClient::connect(log_config).await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("Log streaming unavailable: {e}");
                        return;
                    }
                };
                match log_client.stream_logs(None, None).await {
                    Ok(mut stream) => {
                        while let Ok(Some(entry)) = stream.message().await {
                            let log_entry = crate::types::LogEntry {
                                timestamp: entry.timestamp,
                                level: entry.level,
                                target: entry.target,
                                message: entry.message,
                                fields: entry.fields,
                            };
                            let _ = app_handle.emit("workshop:logs", &log_entry);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("StreamLogs RPC failed: {e}");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_health,
            commands::load_catalog,
            commands::get_genre_options,
            commands::compose_scene,
            commands::submit_input,
            commands::list_sessions,
            commands::resume_session,
            commands::get_scene_state,
            commands::get_prediction_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running storyteller workshop");
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p storyteller-workshop`
Expected: Clean compilation. If there are import errors, fix them — the exact package name for the workshop may differ.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/lib.rs
git commit -m "feat(workshop): wire new gRPC client commands and remove engine managed state"
```

### Task 11: Generate TypeScript types and run ts-rs export

**Files:**
- Create: `crates/storyteller-workshop/src/lib/generated/` (directory)

- [ ] **Step 1: Create the generated directory**

```bash
mkdir -p crates/storyteller-workshop/src/lib/generated
```

- [ ] **Step 2: Run ts-rs export**

`ts-rs` generates files via `#[test]` functions. Run:

```bash
cargo test -p storyteller-workshop
```

This should produce `.ts` files in `crates/storyteller-workshop/src/lib/generated/`.

- [ ] **Step 3: Verify generated files**

```bash
ls crates/storyteller-workshop/src/lib/generated/
```

Expected: `.ts` files for each `#[ts(export)]` struct — `SceneInfo.ts`, `TurnResult.ts`, `DebugEvent.ts`, etc.

- [ ] **Step 4: Create an index.ts barrel export**

Create `crates/storyteller-workshop/src/lib/generated/index.ts` that re-exports all generated types for convenient imports:

```typescript
// Auto-generated type re-exports. Regenerate with: cargo test -p storyteller-workshop
export type { SceneInfo } from './SceneInfo';
export type { TurnResult } from './TurnResult';
export type { TurnTiming } from './TurnTiming';
export type { ContextTokens } from './ContextTokens';
export type { HealthReport } from './HealthReport';
export type { SubsystemStatus } from './SubsystemStatus';
export type { GenreSummary } from './GenreSummary';
export type { GenreOptionsResult } from './GenreOptionsResult';
export type { ArchetypeSummary } from './ArchetypeSummary';
export type { ProfileSummary } from './ProfileSummary';
export type { DynamicSummary } from './DynamicSummary';
export type { SettingSummary } from './SettingSummary';
export type { SessionInfo } from './SessionInfo';
export type { TurnSummary } from './TurnSummary';
export type { ResumeResult } from './ResumeResult';
export type { SceneSelections } from './SceneSelections';
export type { CastSelection } from './CastSelection';
export type { DynamicSelection } from './DynamicSelection';
export type { DebugEvent } from './DebugEvent';
export type { LogEntry } from './LogEntry';
```

This barrel file is hand-maintained — update it when new types are added.

- [ ] **Step 5: Commit generated types**

```bash
git add crates/storyteller-workshop/src/lib/generated/
git commit -m "feat(workshop): generate TypeScript types from Rust via ts-rs"
```

---

## Chunk 3: Svelte Frontend Updates

This chunk updates the Svelte frontend to use generated types and the new API surface.

### Task 12: Update `api.ts` to use generated types

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/api.ts`

- [ ] **Step 1: Replace api.ts**

Update imports to use generated types and adjust function signatures:

```typescript
import { invoke } from '@tauri-apps/api/core';
import type {
  HealthReport,
  SceneInfo,
  TurnResult,
  GenreSummary,
  GenreOptionsResult,
  SessionInfo,
  ResumeResult,
  SceneSelections,
} from './generated';

export async function checkHealth(): Promise<HealthReport> {
  return invoke<HealthReport>('check_health');
}

export async function loadCatalog(): Promise<GenreSummary[]> {
  return invoke<GenreSummary[]>('load_catalog');
}

export async function getGenreOptions(
  genreId: string,
  selectedArchetypes: string[]
): Promise<GenreOptionsResult> {
  return invoke<GenreOptionsResult>('get_genre_options', {
    genreId,
    selectedArchetypes,
  });
}

export async function composeScene(selections: SceneSelections): Promise<SceneInfo> {
  return invoke<SceneInfo>('compose_scene', { selections });
}

export async function submitInput(sessionId: string, input: string): Promise<TurnResult> {
  return invoke<TurnResult>('submit_input', { sessionId, input });
}

export async function listSessions(): Promise<SessionInfo[]> {
  return invoke<SessionInfo[]>('list_sessions');
}

export async function resumeSession(sessionId: string): Promise<ResumeResult> {
  return invoke<ResumeResult>('resume_session', { sessionId });
}

export async function getSceneState(sessionId: string): Promise<unknown> {
  return invoke<unknown>('get_scene_state', { sessionId });
}

export async function getPredictionHistory(sessionId: string): Promise<unknown> {
  return invoke<unknown>('get_prediction_history', { sessionId });
}
```

Note: `submitInput` now takes `sessionId` as a parameter since the workshop no longer holds session state locally — the session ID comes from `SceneInfo.session_id` after composition.

- [ ] **Step 2: Commit**

```bash
git add crates/storyteller-workshop/src/lib/api.ts
git commit -m "feat(workshop): update API layer to use generated types and new command signatures"
```

### Task 13: Update `types.ts` to UI-only types

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/types.ts`

- [ ] **Step 1: Slim types.ts**

Remove all interfaces that now come from generated types. Keep only UI-state types. The exact remaining types depend on what the Svelte components use for local state, but the pattern is:

- Remove: `SceneInfo`, `TurnResult`, `TurnTiming`, `ContextTokens`, `LlmStatus`, `GenreSummary`, `GenreOptions`, `ProfileSummary`, `ArchetypeSummary`, `DynamicSummary`, `CastSelection`, `DynamicSelection`, `SceneSelections`, `SessionSummary`, `TurnSummary`, `ResumeResult`, all debug event types
- Keep: `StoryBlock` (discriminated union for rendering), `PhaseStatus`, `DebugState`, and any other UI-only types
- Add: `import type { ... } from './generated'` where needed for type references

- [ ] **Step 2: Update imports across components**

Search all `.svelte` and `.ts` files for imports from `$lib/types` that now live in `$lib/generated`. Update import paths.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/
git commit -m "refactor(workshop): slim types.ts to UI-only types, import generated types"
```

### Task 14: Update `+page.svelte`

**Files:**
- Modify: `crates/storyteller-workshop/src/routes/+page.svelte`

- [ ] **Step 1: Update health check**

Replace `checkLlm()` / `checkLlmReachable()` calls with `checkHealth()`:

```typescript
import { checkHealth, submitInput, resumeSession } from '$lib/api';
```

Update the health check logic to use `HealthReport` instead of `LlmStatus`.

- [ ] **Step 2: Update submitInput call**

The `submitInput` function now requires `sessionId`. Add session ID tracking:

```typescript
let sessionId: string = '';
```

Update the compose callback to capture `sessionId` from `SceneInfo`:
```typescript
function handleLaunch(info: SceneInfo) {
  sessionId = info.session_id;
  // ... rest of launch logic
}
```

Update submit to pass it:
```typescript
const result = await submitInput(sessionId, text);
```

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/routes/+page.svelte
git commit -m "feat(workshop): update page component for new API and session tracking"
```

### Task 15: Update `SceneSetup.svelte`

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/SceneSetup.svelte`

- [ ] **Step 1: Update genre options call**

Replace calls to `getGenreOptions(genreId, selectedArchetypes)` to use the new `GenreOptionsResult` type. The wizard currently calls this when a genre is selected. The function signature is the same, but the return type shape may differ — verify field names match.

- [ ] **Step 2: Update type imports**

Replace imports from `$lib/types` with imports from `$lib/generated` for types like `GenreSummary`, `GenreOptionsResult`, `SceneSelections`, etc.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/lib/SceneSetup.svelte
git commit -m "feat(workshop): update wizard to use generated types and combined genre options"
```

### Task 16: Update `DebugPanel.svelte` and debug event handling

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/DebugPanel.svelte`
- Modify: `crates/storyteller-workshop/src/lib/logic.ts` (if it exists — handles `applyDebugEvent`)

- [ ] **Step 1: Update debug event type imports**

Replace hand-authored debug event types with generated `DebugEvent` type from `$lib/generated`. Update the event listener and `applyDebugEvent` reducer.

- [ ] **Step 2: Update LLM tab to use HealthReport**

Replace `LlmStatus` references with `HealthReport`. The `checkLlm()` button should call `checkHealth()`.

- [ ] **Step 3: Update Logs tab for new LogEntry shape**

The log entry now has a `fields: Record<string, string>` instead of a general `fields: Record<string, unknown>` from the tracing layer. Update rendering if needed.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src/lib/
git commit -m "feat(workshop): update debug panel for generated types and new health check"
```

### Task 17: Update `SessionPanel.svelte`

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/SessionPanel.svelte` (or wherever session listing lives)

- [ ] **Step 1: Update type imports**

Replace `SessionSummary` with `SessionInfo` from generated types. Update any field name references (`session_id` etc.).

- [ ] **Step 2: Commit**

```bash
git add crates/storyteller-workshop/src/lib/SessionPanel.svelte
git commit -m "feat(workshop): update session panel for generated types"
```

---

## Chunk 4: Verification and Cleanup

### Task 18: Full workspace compilation check

- [ ] **Step 1: Check full workspace**

Run: `cargo check --workspace`
Expected: Clean compilation. Fix any remaining import errors.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Run formatting**

Run: `cargo fmt --check`
If needed: `cargo fmt`

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix: resolve compilation and lint issues from workshop conversion"
```

### Task 19: Run all tests

- [ ] **Step 1: Run unit tests**

Run: `cargo test --workspace`
Expected: All unit tests pass.

- [ ] **Step 2: Run server tests with LLM**

Start the server, then:
Run: `cargo test --all-features -p storyteller-server -p storyteller-client`
Expected: All tests pass including integration tests.

- [ ] **Step 3: Verify ts-rs generated files are up to date**

Run: `cargo test -p storyteller-workshop`
Then: `git diff crates/storyteller-workshop/src/lib/generated/`
Expected: No diff — generated files match committed versions.

- [ ] **Step 4: Commit if needed**

### Task 20: Frontend build verification

- [ ] **Step 1: Install frontend dependencies**

```bash
cd crates/storyteller-workshop && bun install
```

- [ ] **Step 2: Run TypeScript type check**

```bash
bun run check
```
Expected: No type errors.

- [ ] **Step 3: Run vitest if available**

```bash
bun run test
```
Expected: All tests pass.

- [ ] **Step 4: Commit any fixes**

### Task 21: Manual smoke test

- [ ] **Step 1: Start the server**

```bash
cargo run -p storyteller-server
```

- [ ] **Step 2: Launch the workshop**

```bash
cd crates/storyteller-workshop && cargo tauri dev
```

- [ ] **Step 3: Verify wizard flow**

Walk through: open app → health check shows green → select genre → select profile → configure cast → add dynamics → compose → see opening prose → submit player input → see narrator response → check debug inspector tabs.

- [ ] **Step 4: Verify session resume**

Close and reopen the app. List sessions. Resume the session from step 3. Verify turn history loads correctly.

### Task 22: Update .env.example and documentation

**Files:**
- Modify: `.env.example`

- [ ] **Step 1: Add STORYTELLER_SERVER_URL to .env.example**

The workshop now needs the server URL (defaults to `http://localhost:50051` via `ClientConfig::from_env()`). Add a comment in `.env.example`:

```bash
# Workshop requires a running storyteller-server
# STORYTELLER_SERVER_URL=http://localhost:50051
```

- [ ] **Step 2: Commit**

```bash
git add .env.example
git commit -m "docs: update .env.example for workshop server dependency"
```
