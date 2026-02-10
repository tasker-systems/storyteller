# Phase D: Turn Pipeline Integration

**Status**: Complete (D.1–D.2), Deferred (D.3–D.6)
**Branch**: `jcoletaylor/event-system-foundations`
**Depends on**: Phase C (ML classification pipeline, turn cycle systems)

## Context

Phases A-C established the event grammar types, entity promotion bridge, and ML classification pipeline (316 Rust tests + 50 Python tests). The turn cycle Bevy pipeline has two real systems (`classify_system`, `predict_system`) and four stubs (`commit_previous_system`, `resolve_system`, `assemble_context_system`, `start_rendering_system`). All the downstream infrastructure exists — `NarratorAgent`, `ExternalServerProvider`, three-tier context assembly, `SceneJournal` — but it's only used in the manual `play_scene_context.rs` binary, not wired through Bevy systems.

Phase D replaces the stubs with real implementations and adds in-memory turn history. The goal: a complete turn pipeline running through the Bevy ECS, from player input to LLM-rendered narrator output.

### Design Philosophy: PendingInput and Commitment Timing

The key architectural decision in D.1 is `PendingInput`. When a new player input arrives and advances the stage to `CommittingPrevious`, the `TurnContext` still holds the *previous* turn's data (rendering, predictions, classification). If we overwrite `player_input`, the previous turn's input is lost before archiving. `PendingInput` holds the new input separately until `commit_previous_system` archives old data and moves it.

This aligns with the broader commitment model: nothing is committed until the player responds. The player's *next* input triggers commitment of the *previous* turn's provisional data.

### Relationship to Other Documents

| Document | Role |
|---|---|
| `implementation-plan.md` | Master plan — Phase D section references this spec |
| `phase-c-ml-classification-pipeline.md` | Prior phase — systems this phase builds on |
| `docs/technical/narrator-architecture.md` | Three-tier context and narrator-centric pipeline |
| `docs/technical/turn-cycle-architecture.md` | Bevy mapping, module organization, async bridge pattern |
| `docs/technical/event-system.md` | Event lifecycle and commitment model |

---

## Architecture Overview

### Data Flow Through the Pipeline

```
External code sets PendingInput + advances to CommittingPrevious
       │
       ▼
CommittingPrevious ── archive TurnContext → TurnHistory
       │                add rendering → SceneJournal
       │                reset TurnContext
       │                move PendingInput → TurnContext.player_input
       ▼
Classifying ────────── classify_and_extract() → classification
       ▼
Predicting ─────────── predict_character_behaviors() → predictions
       ▼
Resolving ──────────── wrap predictions → ResolverOutput (pass-through)
       ▼
AssemblingContext ───── assemble_narrator_context() → NarratorContextInput
       ▼
Rendering ──────────── spawn async NarratorAgent.render()
       │                poll oneshot receiver each frame
       │                on completion → TurnContext.rendering
       ▼
AwaitingInput ──────── display rendering to player
```

### New Bevy Resources

| Resource | Purpose | Auto-initialized |
|---|---|---|
| `TurnHistory` | In-memory archive of completed turns | Yes (empty) |
| `PendingInput` | Holds new player input until commit completes | Yes (None) |
| `NarratorResource` | `Arc<tokio::sync::Mutex<NarratorAgent>>` for async rendering | No (needs LLM) |
| `JournalResource` | Mutable `SceneJournal` for progressive compression | No (needs SceneId) |
| `TokioRuntime` | `tokio::runtime::Handle` for spawning async tasks | No (needs runtime) |

### Rendering System State Machine

The `rendering_system` replaces `start_rendering_system` with a single system that dispatches on `NarratorTask` state:

- **Idle**: Extract context, spawn async task, set `InFlight`. Stay in `Rendering`.
- **InFlight**: Poll `oneshot::Receiver`. If ready → extract, reset to Idle, advance. If not → stay.
- **Complete**: Direct completion path for tests/sync fallback. Extract, advance.

The pipeline does NOT complete in one frame when rendering is real. The system stays in `Rendering` across multiple Bevy updates until the oneshot resolves.

### Module Organization

The turn pipeline is split by concern — synchronous orchestration vs async lifecycle:

```
systems/
├── turn_cycle.rs   # 5 sync systems + SystemSets + run conditions
├── rendering.rs    # Async narrator bridge: rendering_system + NarratorTask lifecycle
```

`turn_cycle.rs` owns the pipeline orchestration: committing previous turns, ML classification, ML prediction, resolver pass-through, and three-tier context assembly. All five systems are thin wrappers around framework-independent service functions.

`rendering.rs` owns the one genuinely async operation — spawning a tokio task for the LLM call and polling for completion. It participates in the pipeline via the same `run_if` stage gating, but manages the `NarratorTask` state machine internally.

**Why not an actor/channel pattern**: The Narrator is a single request-response operation (one call per turn). An actor pattern (mpsc channels, persistent background task) adds indirection without benefit — the persistent task buys nothing since calls are sequential. The oneshot pattern is the correct tool. Actor/channel abstractions become appropriate when persistent, multiplexed async services arrive (GraphRAG workers, event ledger, RabbitMQ integration).

---

## Work Items

### D.1: In-Memory Turn History — Complete

**Goal**: Track completed turns in memory so the pipeline can reference previous turns for journal updates and future committed-turn classification.

**What was built**:

- `CompletedTurn` struct: captures turn_number, player_input, narrator_rendering, classification, predictions, committed_at timestamp
- `TurnHistory` resource: `Vec<CompletedTurn>` with helper methods
- `PendingInput` resource: `Option<String>` holding new input until commitment
- Real `commit_previous_system`: archives TurnContext → TurnHistory, adds rendering to journal, moves PendingInput → player_input

**Tests**: TurnHistory accumulation, PendingInput consumption, first-turn no-op, journal integration

### D.2: Real Narrator Rendering — Complete

**Goal**: Wire the existing `NarratorAgent`, three-tier context assembly, and `SceneJournal` through the Bevy pipeline.

**What was built**:

- `NarratorResource`: `Arc<tokio::sync::Mutex<NarratorAgent>>` for async access
- `JournalResource`: mutable `SceneJournal` for progressive compression
- `TokioRuntime`: `tokio::runtime::Handle` for spawning from Bevy systems
- Real `resolve_system`: pass-through wrapping predictions into `ResolverOutput`
- Real `assemble_context_system`: calls `assemble_narrator_context()` with data from resources
- Real `rendering_system`: state-machine dispatching on NarratorTask (Idle/InFlight/Complete)
- Module extraction: `rendering_system` moved to `systems/rendering.rs`, separating async LLM lifecycle from sync pipeline orchestration in `systems/turn_cycle.rs`

**Tests**: Rendering system states (Idle skip, Complete extract, InFlight polling), context assembly with workshop data, resolve wrapping, full pipeline cycle (20 engine tests, 119 total engine, 336 total workspace)

### D.3: Committed-Turn Classification — Deferred

**Goal**: When a turn is committed, run the event classifier on both the narrator's prose and the player's input from that turn (combined). This produces richer `EventAtom` instances from the full turn text rather than just the player's input.

**Specification**:

The `commit_previous_system` would, after archiving:
1. Concatenate `narrator_rendering.text` + `player_input` into a combined text
2. Run `classify_and_extract()` on the combined text
3. Store the committed classification on the `CompletedTurn`

This is deferred because: (a) the current per-player-input classification is sufficient for the pipeline, (b) committed classification feeds the event ledger which requires persistence (future branch).

### D.4: Narrator Output Deduplication — Deferred

**Goal**: Detect when the narrator repeats information from the journal or previous turn, and either suppress or flag it.

**Specification**:

Post-rendering filter that compares rendered text against recent journal entries using simple n-gram overlap or sentence embedding similarity. If overlap exceeds a threshold, either re-render with a "don't repeat" instruction or flag for the session observer.

Deferred because: requires experimentation with overlap detection thresholds and re-render strategies. The journal's progressive compression already reduces the risk of repetition.

### D.5: Prediction Confirmation — Deferred

**Goal**: After the narrator renders, compare rendered text against character predictions to determine which predictions were "confirmed" by the narrative.

**Specification**:

Post-rendering analysis that matches rendered narrative content against the original character predictions. Confirmed predictions would have their `ProvisionalStatus` updated to `Rendered`. On the next turn's commitment, they'd move to `Committed`.

Deferred because: requires the prediction → rendered confirmation matching logic (semantic similarity or structured extraction from narrator output), which is a research problem.

### D.6: X-Card / Rejection Flow — Deferred

**Goal**: Allow the player to reject or undo a narrator rendering before it becomes committed.

**Specification**:

A special player input (e.g., `/undo` or `/x-card`) would:
1. Discard the current `TurnContext.rendering`
2. Optionally re-render with modified constraints
3. NOT commit the rejected turn to history

The `ProvisionalStatus` model already supports this — rendered-but-not-committed data can be discarded. The pipeline returns to `AwaitingInput` without the rejected turn entering `TurnHistory`.

Deferred because: requires player-facing protocol decisions (how rejection is communicated) and UI integration.

---

## What Phase D Does NOT Include

- **Persistent storage**: TurnHistory is in-memory only. Event ledger persistence is a future branch concern.
- **PreambleCache**: The preamble is rebuilt by `assemble_narrator_context` each turn, which is fine for prototype. Caching is a future optimization.
- **Changes to play_scene_context.rs**: The manual binary continues working independently of the Bevy pipeline.
- **Multi-session support**: One scene, one pipeline instance.
- **Real resolver logic**: `resolve_system` is a pass-through. Real RPG-like mechanics are future work.

---

## Verification

```bash
# All unit tests
cargo test --workspace

# With ONNX model tests
cargo test --workspace --features test-ml-model

# Quality checks
cargo clippy --all-targets --all-features
cargo fmt --check

# Feature-gated LLM tests (requires Ollama running)
cargo test --workspace --features test-llm
```
