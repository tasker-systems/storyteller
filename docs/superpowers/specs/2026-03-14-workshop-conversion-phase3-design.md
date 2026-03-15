# Workshop Conversion (Phase 3): gRPC Client Migration + ts-rs Type Generation

## Problem Statement

The storyteller-workshop embeds the full engine runtime in-process — ~1800 lines of pipeline orchestration in `commands.rs`, direct dependencies on `storyteller-engine` (Bevy, ort, candle, rayon), `storyteller-ml`, and `storyteller-composer`. This was necessary before the gRPC server existed, but Phase 2 delivered a complete server with typed client library. The workshop should consume the server like any other client rather than maintaining a parallel orchestration path.

Additionally, the workshop maintains ~317 lines of hand-authored TypeScript interfaces in `types.ts` that manually mirror Rust structs. These drift silently — the compiler catches nothing when a Rust struct changes shape.

Phase 3 converts the workshop to a pure gRPC client via `storyteller-client` and adopts `ts-rs` for automated Rust → TypeScript type generation.

## Decisions Evolved from Parent Design Doc

The parent design doc (`docs/plans/2026-03-13-engine-server-and-playtest-harness-design.md`, lines 225-316) established the Phase 3 vision. This spec refines it:

1. **Clean break, no hybrid mode.** The workshop requires a running server. No fallback to in-process execution. Pre-alpha with no external users makes this the right moment for a breaking change.

2. **`ts-rs` for type generation.** Not in the parent doc. All types crossing the Tauri invoke boundary are generated from Rust source-of-truth structs. Generated files are checked into git (not gitignored) — diffs in generated files serve as a canary for unexpected type changes that cross the compile boundary into TypeScript.

3. **`GetGenreOptions` combined RPC.** The parent doc has the wizard calling individual descriptor RPCs. A single combined RPC reduces round trips for the wizard's per-genre step without the complexity of client-side caching and invalidation.

4. **Two-channel debug architecture.** `workshop:debug` for gameplay phase events (from `EngineEvent` stream), `workshop:logs` for server tracing (from `StreamLogs` RPC). Cleanly separable — the inspector can be gated for release builds without affecting gameplay.

5. **Request-response for primary actions.** Gameplay commands (`compose_scene`, `submit_input`) remain Tauri invoke request-response, not event streams. The narrator renders in one shot currently. Event stream architecture (`workshop:gameplay` channel) is documented as future work for when narrator token streaming arrives.

## Design

### 1. Dependency Graph

**Before:**
```
storyteller-workshop (Tauri)
  ├── storyteller-engine    (Bevy ECS, ort, candle, rayon, lapin)
  ├── storyteller-core      (types)
  ├── storyteller-ml        (prediction history)
  ├── storyteller-composer  (SceneComposer, descriptors)
  └── reqwest               (inline Ollama probe)
```

**After:**
```
storyteller-workshop (Tauri)
  ├── storyteller-client    (gRPC client, re-exports proto types)
  ├── storyteller-core      (shared domain types)
  └── ts-rs                 (TypeScript type generation)
```

Dropped dependencies: `storyteller-engine`, `storyteller-ml`, `storyteller-composer`, `reqwest`, and all their transitive deps (bevy_app, bevy_ecs, ort, candle-core, rayon, crossbeam, lapin).

### 2. Tauri Command Layer

Each Tauri command becomes a thin wrapper: receive args from Svelte → call `storyteller-client` → translate proto response to `ts-rs`-annotated Rust struct → return.

**Command mapping:**

| Tauri Command | gRPC RPC | Return Type |
|---|---|---|
| `check_health()` | `CheckHealth` | `HealthReport` |
| `load_catalog()` | `ListGenres` | `Vec<GenreSummary>` |
| `get_genre_options(genre_id)` | `GetGenreOptions` (new) | `GenreOptionsResult` |
| `compose_scene(selections)` | `ComposeScene` (streaming) | `SceneInfo` |
| `submit_input(session_id, input)` | `SubmitInput` (streaming) | `TurnResult` |
| `resume_session(session_id)` | `ResumeSession` (streaming) | `ResumeResult` |
| `list_sessions()` | `ListSessions` | `Vec<SessionSummary>` |
| `get_scene_state(session_id)` | `GetSceneState` | `SceneState` |
| `get_prediction_history(session_id)` | `GetPredictionHistory` | `PredictionHistoryResult` |

**Streaming command pattern** (`compose_scene`, `submit_input`, `resume_session`):
1. Open gRPC stream via `storyteller-client`
2. For each `EngineEvent`, translate to a `DebugEvent` struct and emit on Tauri event channel `workshop:debug`
3. Accumulate the final result (narrator prose, scene info, etc.)
4. Return the accumulated result when the stream completes

**Client lifecycle:** A `StorytellerClient` is constructed once at app startup and stored in Tauri managed state (`tauri::State`). Commands receive it via Tauri's dependency injection. Connection errors surface as command errors that the Svelte layer displays.

**Replaces `LlmStatus` with `HealthReport`:** The old `LlmStatus` struct (reachable, endpoint, model, available_models) is replaced by `HealthReport` which wraps the server's `CheckHealth` response — overall status plus per-subsystem breakdown. The server already probes Ollama reachability in its health check (implemented in Phase 2 review fixes).

### 3. `ts-rs` Type Generation

**Source of truth chain:**
```
Proto definitions
  → tonic-generated Rust types (build script, not annotated)
  → ts-rs annotated wrapper structs in types.rs (Tauri-facing)
  → generated .ts files in src/lib/generated/
  → Svelte imports
```

The workshop's `types.rs` defines structs that `#[derive(TS, Serialize)]` and are constructed from proto types in the command layer. Raw proto types are not annotated directly — they live in build script output and we don't control their shape.

**Output directory:** `crates/storyteller-workshop/src/lib/generated/`

**Checked into git.** Generated files are committed — diffs serve as a signal for unexpected cross-boundary type changes. No `.gitignore` entry.

**What gets generated:** Every struct returned by a Tauri command or emitted as a Tauri event — `SceneInfo`, `TurnResult`, `HealthReport`, `GenreSummary`, `DebugEvent` variants, `SessionSummary`, `GenreOptionsResult`, etc.

**What stays hand-authored in TypeScript:** UI-only state types — wizard step tracking, debug panel tab selection, `StoryBlock` rendering discriminated union, scroll position, loading states.

### 4. Debug Events and Log Streaming

Two event channels, cleanly separated:

**`workshop:debug`** — Gameplay phase events from the `EngineEvent` gRPC stream. Streaming Tauri commands translate each `EngineEvent` into a `DebugEvent` struct and emit it. The `DebugPanel` inspector consumes these via the existing `applyDebugEvent` reducer pattern.

Covers: phase transitions, decomposition results, prediction results, arbitration results, narrator output, timing, goals, errors.

**`workshop:logs`** — Server tracing data from the `StreamLogs` gRPC stream. Opened at app startup as a long-lived connection. Log entries forwarded to the Tauri event channel for the Logs inspector tab.

Replaces the current `TauriTracingLayer` which hooks into the in-process tracing subscriber. After conversion, the workshop has no in-process tracing to bridge — all tracing happens server-side.

**Inspector tab data sources:**

| Tab | Source | Change from Current |
|---|---|---|
| LLM | `DebugEvent` (NarratorComplete, timing) | Field mapping only |
| Context | `DebugEvent` (ContextAssembled) | Field mapping only |
| ML Predictions | `DebugEvent` (PredictionComplete) | Field mapping only |
| Characters | `GetSceneState` RPC | Was in-process, now RPC |
| Events | `DebugEvent` (DecompositionComplete) | Field mapping only |
| Arbitration | `DebugEvent` (ArbitrationComplete) | Field mapping only |
| Goals | `DebugEvent` (GoalsGenerated) | Field mapping only |
| Narrator | `DebugEvent` (NarratorComplete) | Field mapping only |
| Logs | `StreamLogs` RPC | New data source, same rendering |

The debug surface is cleanly separable from gameplay. The inspector can be gated behind a dev/debug flag for release builds without affecting the core experience.

### 5. Server-Side Additions

**New proto + RPC:**

```protobuf
// In composer.proto
message GenreOptionsRequest {
  string genre_id = 1;
}

message GenreOptions {
  repeated Archetype archetypes = 1;
  repeated Profile profiles = 2;
  repeated Dynamic dynamics = 3;
  repeated string names = 4;
  repeated Setting settings = 5;
}

rpc GetGenreOptions(GenreOptionsRequest) returns (GenreOptions);
```

Added to `ComposerService` alongside existing per-category RPCs. Server-side implementation delegates to the already-loaded `SceneComposer` descriptor set.

**`StreamLogs` RPC implementation:**
Already defined in the proto from Phase 1 but currently stubbed. Phase 3 implements it: the server subscribes to its own `tracing` layer and forwards log entries to connected gRPC stream clients.

**`EngineEvent` payload enrichment:**
During implementation, if any inspector tab requires detail not carried by current `EngineEvent` payloads, the proto is enriched. Adding fields to existing messages is non-breaking. This is handled per-tab during implementation rather than specified upfront.

### 6. Svelte Frontend Changes

**Minimal changes — mostly import paths and type names:**

1. **`types.ts`** — Hand-authored interfaces replaced by imports from `src/lib/generated/`. Slimmed down to UI-only types (wizard state, `StoryBlock`, tab selection).

2. **`api.ts`** — Invoke calls stay structurally the same. Function signatures use generated types. Key changes:
   - `checkLlm()` → `checkHealth()` (returns `HealthReport`)
   - New: `getGenreOptions(genreId)` replaces multiple descriptor calls
   - Other commands: same shape, updated type imports

3. **`SceneSetup.svelte`** — Wizard step handlers call `getGenreOptions()` instead of individual descriptor fetches. Wizard flow unchanged.

4. **`DebugPanel.svelte`** — `DebugEvent` type is now ts-rs generated. `applyDebugEvent` reducer and tab rendering need minimal adjustment.

5. **`+page.svelte`** — Health check uses `checkHealth()`. Session resume and turn submission structurally unchanged.

**What doesn't change:** Component structure and layout, wizard flow and step navigation, input bar behavior, debug inspector tab structure, story block rendering.

### 7. Deleted Files

**Rust (src-tauri/src/):**
- `commands.rs` — 1803 lines of in-process pipeline orchestration (replaced by thin gRPC wrapper)
- `engine_state.rs` — in-process session state holder (server owns session state)
- `session.rs` — local session persistence (server owns persistence)
- `session_log.rs` — local turn-by-turn JSONL logging (server owns event persistence)
- `events.rs` — in-process debug event emitter (replaced by `EngineEvent` stream forwarding)
- `tracing_layer.rs` — in-process tracing bridge (replaced by `StreamLogs` RPC)

**New Rust files:**
- `commands.rs` — thin gRPC command wrappers
- `types.rs` — `ts-rs` annotated structs for Tauri ↔ Svelte boundary

### 8. Session Management

The server owns all session persistence. The workshop queries sessions via `ListSessions` RPC which currently returns all sessions. The RPC is designed to accept a scope parameter in the future when auth/RBAC arrives (owned, group, managed, all — in ascending scope).

The workshop may maintain a lightweight local record of recently-accessed session IDs for quick resume (like browser history), but this is a UX convenience, not a persistence layer.

## Testing Strategy

- **Tauri command tests:** Unit tests for the translation layer — mock `StorytellerClient` responses, verify correct `ts-rs` struct construction.
- **`ts-rs` verification:** Build-time generation produces `.ts` files; CI checks that generated files are up to date (fail if `cargo test` regenerates files that differ from committed versions).
- **Integration:** Manual verification via the workshop UI. Start server, launch workshop, run through wizard → compose → play → resume → inspect.
- **Frontend:** Existing vitest suite for Svelte logic continues to pass with updated type imports.

## Future Work (Out of Scope)

### Event Stream Architecture (`workshop:gameplay`)

The current request-response pattern for gameplay commands works well when the narrator renders in one shot. When narrator token streaming arrives, the architecture should evolve to a `workshop:gameplay` event channel where all primary render data flows as progressive events — narrator tokens, scene transitions, turn completion signals. This unifies the data flow model across three channels:

- `workshop:gameplay` — player experience (prose, scene state)
- `workshop:debug` — inspector (phase events, predictions, arbitration)
- `workshop:logs` — tracing (server-side spans and diagnostics)

At that point, Tauri commands become fire-and-forget triggers rather than request-response, and Svelte manages all state reactively from event streams.

### FLUX.2 Async Image Generation

Side-stream narrator prose to a text-to-image model (FLUX.2 klein 4b/9b) to generate graphic-novel-style panels alongside the narrative. Key architectural properties:

- **Async and non-blocking** — image generation is decoupled from the gameplay turn cycle. Images arrive when ready, not gated on turn completion.
- **Rich context** — the image generation prompt receives setting, characters, scene, and genre aesthetic context alongside the narrator's grounding prose. Not the same data the narrator receives, but the same rich source material.
- **Aesthetic direction** — derived from genre and user preferences, leaning toward graphic novel panel styling.
- **Delivery** — images arrive on a `workshop:media` event channel as `file:///` or S3 resource URLs.
- **Depends on** the event stream architecture (`workshop:gameplay`) being in place.

This is a separate design effort that builds on the event stream architecture.
