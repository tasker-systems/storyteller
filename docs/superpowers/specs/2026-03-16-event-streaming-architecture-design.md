# Event Streaming Architecture — Design Specification

**Date:** 2026-03-16
**Branch:** `jcoletaylor/event-streaming-architecture`
**Status:** Design
**Meta-plan reference:** Work Unit A.3 (Gameplay Streaming Channel + Player Intent Review + Turn Lifecycle)
**Roadmap reference:** `docs/technical/roadmap/2026-03-16-scenes-chapters-and-stories.md`, Part I §2, §5

## Purpose

This spec designs the gameplay streaming channel, narrator prose streaming, turn lifecycle formalization, and async integration points for the storyteller workshop. It addresses three interconnected concerns from the roadmap as a single coherent system:

1. **Gameplay streaming channel** (`workshop:gameplay`) — a player-facing event channel for primary story rendering
2. **Narrator prose streaming** — sentence-batched LLM output streaming to reduce time-to-first-paint
3. **Turn lifecycle formalization** — collapsing the Bevy pipeline, defining two distinct flows, and designing integration points for future async agents

Player intent review, originally scoped as a post-composition interaction step, has been redesigned as a pre-composition wizard input — simpler, more natural, and eliminates upstream client-to-server event flow.

## Design Decisions

### D1: Single spec for three concerns

The gameplay channel, prose streaming, and turn lifecycle are deeply coupled. The turn lifecycle *is* the state machine that the gameplay channel transits through. Designing them separately risks misaligned event sequences. Player intent review is the last concern addressed because it's an artifact of the turn lifecycle formalization, not a driver of it.

### D2: Bevy pipeline collapse

The current 7-variant `TurnCycleStage` enum spreads responsibilities across stages that don't need top-level representation. Classifying, Predicting, and Resolving are all "understand what just happened" work that should be encapsulated behind a single stage boundary. Collapsing to 5 variants with an internal enrichment sub-pipeline makes the top-level state machine cleaner and the enrichment pipeline independently extensible.

### D3: Narrator prose batching over token-level streaming

With ~200-250 words per narrator response (~300-400 LLM tokens), token-level streaming would produce 300+ individual events per turn. Sentence/paragraph batching produces ~4-8 `NarratorProse` events instead, achieving the same time-to-first-paint benefit without the event frequency overhead. The batching logic is simple: flush buffer on sentence-ending punctuation (`. ` `? ` `! `) or paragraph breaks (`\n\n`), flush remainder when the stream ends.

### D4: Player intent as wizard input, not post-composition review

Moving intent capture before `ComposeScene` eliminates the `IntentProposal` / `IntentConfirmed` interaction loop, keeps the gameplay channel purely downstream, and lets the composer generate NPC goals *in response to* the player's stated intent. The player character already receives a full tensor representation — the wizard step provides seed attributes and intent that feed into tensor assignment.

### D5: Bottom-up implementation approach

Infrastructure first (structural changes), then plumbing (channel wiring), then features (visible behavior). Optimized for autonomous agent execution with clear layer boundaries and minimal rework.

### D6: `directives.jsonl` as async integration surface

Following the established pattern of `turns.jsonl` and `events.jsonl`, a new append-only structured store for async agent outputs. Designed now with read path wired into context assembly, populated later by Tier C agents (dramaturge, world agent). The append-only, seek-sortable pattern matches the existing session persistence model and serves as a pre-database modeling surface.

### D7: Same `EngineEvent` gRPC stream for prose events

Phase events and prose events are naturally sequential within a turn — they never interleave. A separate gRPC stream would force the client to manage two concurrent streams and coordinate their lifetimes for no benefit. The Tauri command layer discriminates on `engine_event::Payload` variants and routes to the appropriate channel (`workshop:debug` for phase events, `workshop:gameplay` for prose/gameplay events).

---

## Section 1: Bevy Pipeline Refactor

### Current state (7 variants)

```
AwaitingInput → CommittingPrevious → Classifying → Predicting →
Resolving → AssemblingContext → Rendering → [back to AwaitingInput]
```

Each stage has a corresponding `TurnCycleSets` entry, a `run_if(in_stage(...))` condition, and a dedicated system function. `Classifying` is currently a no-op pass-through (DistilBERT removed). `Predicting` calls `predict_character_behaviors()`. `Resolving` wraps predictions into `ResolverOutput` and runs basic arbitration.

### Revised state (5 variants)

```
AwaitingInput → CommittingPrevious → Enriching → AssemblingContext →
Rendering → [back to AwaitingInput]
```

`Classifying`, `Predicting`, and `Resolving` collapse into `Enriching`. The `TurnCycleStage` enum loses three variants and gains one (7 → 5).

### Enrichment sub-pipeline

A new `EnrichmentPhase` enum internal to the `Enriching` stage:

```
EventClassification → BehaviorPrediction → GameSystemArbitration → IntentSynthesis
```

The enrichment system manages this sub-pipeline within a single Bevy stage. Each phase executes sequentially. New enrichment phases slot in by extending the `EnrichmentPhase` enum without changing the top-level Bevy schedule.

The `EnrichmentPhase` enum and its state are tracked via a new `EnrichmentState` Bevy Resource, reset to the first phase when the `Enriching` stage begins. The enrichment system advances through phases within the stage, then advances the top-level stage when all phases complete.

### CommittingPrevious semantics

Clarified: this stage performs an enum-backed state change (Provisional → Committed) on already-persisted records. It is not a persistence operation — data is persisted during the Rendering/TurnComplete flow of the previous turn. Commitment means the player has responded, so provisional ML/LLM outputs are now confirmed as the basis for subsequent processing.

### SystemSet changes

`TurnCycleSets` consolidates from 7 to 5 variants: `Input`, `CommittingPrevious`, `Enrichment`, `ContextAssembly`, `Rendering`. The plugin's `configure_sets` and `add_systems` calls update accordingly. (Note: the source docstring on `TurnCycleStage` incorrectly claims "Eight variants" — fix this during implementation.)

### Files affected

- `crates/storyteller-core/src/types/turn_cycle.rs` — `TurnCycleStage` enum, `next()` method, serde tests
- `crates/storyteller-engine/src/systems/turn_cycle.rs` — `TurnCycleSets`, `in_stage()`, system functions (collapse three into one enrichment system)
- `crates/storyteller-engine/src/plugin.rs` — SystemSet configuration, system registration
- `crates/storyteller-engine/src/components/turn.rs` — new `EnrichmentState` Resource

---

## Section 2: LLM Provider Streaming

### Trait addition

`LlmProvider` gains a `stream_complete()` method:

```rust
async fn stream_complete(
    &self,
    request: CompletionRequest,
) -> StorytellerResult<NarratorTokenStream>;
```

Default implementation spawns a task that calls `complete()` and sends the full response as a single chunk through the channel, returning the receiver immediately. Providers that support real streaming override this.

### Newtype wrapper

`NarratorTokenStream` is a newtype around `tokio::mpsc::Receiver<String>`, following the codebase convention of named channel types for compile-time differentiation:

```rust
pub struct NarratorTokenStream(pub tokio::mpsc::Receiver<String>);
```

The corresponding sender side uses a paired newtype `NarratorTokenSender(pub tokio::mpsc::Sender<String>)`.

Channel capacity: **64** (bounded, per project convention — no `unbounded_channel()`). This is sufficient for token-level streaming throughput while providing backpressure if the consumer falls behind.

### ExternalServerProvider streaming

`ExternalServerProvider` (Ollama) implements real streaming via Ollama's `/api/generate` endpoint with `stream: true`. The response is a newline-delimited JSON stream where each line contains a `response` field with the next token chunk. The provider spawns a task that reads the HTTP response body incrementally and sends each token through the `NarratorTokenSender`.

### Prose batching

The engine service (`engine_service.rs`) consumes the `NarratorTokenStream` and implements sentence/paragraph batching:

1. Read tokens from the stream, append to a buffer
2. On sentence-ending punctuation (`. ` `? ` `! `) or paragraph break (`\n\n`), flush the buffer as a `NarratorProse` event on the gRPC stream. (Note: simple punctuation matching may false-positive on abbreviations like "Dr. Smith" — acceptable for ~4-8 events per turn where cosmetic splitting is not harmful.)
3. On stream completion, flush any remaining buffer
4. Emit `NarratorComplete` with the full accumulated prose

This produces ~4-8 `NarratorProse` events per turn for typical 200-250 word narrator responses.

### Bevy rendering system

The `NarratorTask` async bridge in the rendering system stays structurally the same. The oneshot receiver still signals completion with the full `NarratorRendering`. Streaming is a server-layer concern (`engine_service.rs`), not a Bevy-layer concern. The Bevy system does not need to know whether the underlying LLM call was streaming or batch.

### Files affected

- `crates/storyteller-core/src/traits/llm.rs` — `stream_complete()` method, `NarratorTokenStream` / `NarratorTokenSender` newtypes
- `crates/storyteller-engine/src/inference/external.rs` — `ExternalServerProvider` streaming implementation
- `crates/storyteller-server/src/grpc/engine_service.rs` — prose batching logic in narrator rendering phase

---

## Section 3: Gameplay Channel & Event Vocabulary

### Channel architecture

Four Tauri event channels, each with a distinct audience and purpose:

| Channel | Audience | Purpose |
|---------|----------|---------|
| `workshop:gameplay` | Player | Primary story rendering — what the player sees and interacts with |
| `workshop:debug` | Developer | Phase-level pipeline detail — ML predictions, context tiers, prompt params |
| `workshop:logs` | Developer | Raw tracing stream — everything tracing would emit |
| `workshop:media` | Player | Visual content generation — reserved, deferred to Tier B |

### GameplayEvent discriminated union

Six variants, all downstream (server → client):

**SceneReady** — Emitted when scene composition completes. Signals UI transition from wizard to story view.
- `scene_id: String`
- `title: String`
- `setting_summary: String`
- `cast_names: Vec<String>`
- `player_character: String`
- `player_intent: Option<String>` — the player's stated intent (free text) or system-generated intent if defaulted

**InputReceived** — Emitted when server receives player input. Frontend disables input bar and shows processing state.
- `turn: u32`

**ProcessingUpdate** — Lightweight progress indicator between input receipt and first prose chunk. Thematically-styled phrases consistent with storytelling aesthetic.
- `phase: String` — `"enriching"` | `"assembling"` | `"rendering"` (or thematic variants)

**NarratorProse** — Sentence or paragraph chunk of narrator output. ~4-8 events per turn.
- `chunk: String`
- `turn: u32`

**NarratorComplete** — Full prose buffer for reconciliation against streamed chunks, persistence, and future media pipeline trigger.
- `prose: String`
- `turn: u32`

**TurnComplete** — Turn cycle complete. Re-enables input bar.
- `turn: u32`
- `ready_for_input: bool`

### Proto additions

`engine.proto` gains new messages: `NarratorProse`, `SceneReady`, `InputReceived`, `ProcessingUpdate`. The `EngineEvent.payload` oneof gains corresponding variants. The existing `NarratorComplete` message is reused as-is. The existing `TurnComplete` message gains a `ready_for_input` bool field — the `GameplayEvent::TurnComplete` variant on the Tauri side maps directly from this proto message rather than wrapping it separately.

### Frontend wiring

- `types.ts` — `GameplayEvent` discriminated union type definition, `GAMEPLAY_CHANNEL` constant
- `+page.svelte` — listen on `workshop:gameplay`, dispatch to `StoryPane` and `InputBar`
- `StoryPane.svelte` — append `NarratorProse` chunks to reactive blocks array; reconcile on `NarratorComplete`
- `InputBar.svelte` — disable on `InputReceived`, re-enable on `TurnComplete`

### Files affected

- `proto/storyteller/v1/engine.proto` — new message types, oneof variants
- `crates/storyteller-workshop/src-tauri/src/commands.rs` — gameplay event emission
- `crates/storyteller-workshop/src-tauri/src/types.rs` — `GameplayEvent` Rust types (ts-rs)
- `crates/storyteller-workshop/src/lib/types.ts` — TypeScript discriminated union
- `crates/storyteller-workshop/src/routes/+page.svelte` — gameplay channel listener
- `crates/storyteller-workshop/src/lib/StoryPane.svelte` — incremental prose rendering
- `crates/storyteller-workshop/src/lib/InputBar.svelte` — state driven by gameplay events

---

## Section 4: Two Distinct Flows

### Scene-start flow (once per scene)

Triggered when the wizard completes and `ComposeScene` is called. Purely downstream — no player interaction after the wizard.

```
Wizard completes
→ ComposeScene RPC (with player character data + intent)
→ Server composes scene, assigns tensors, generates goals
→ SceneReady event on workshop:gameplay
→ ProcessingUpdate ("composing" | "assembling" | "rendering")
→ Context assembly (three-tier, with player intent in preamble)
→ Narrator streaming → NarratorProse chunks
→ NarratorComplete (full prose)
→ Persist turn 0 (player_input: null, events.jsonl + turns.jsonl)
→ TurnComplete (ready_for_input: true)
→ Enter per-turn flow
```

### Per-turn flow (turns 1..N)

Repeats for each player input. Maps onto the revised Bevy state machine.

```
Player submits input
→ InputReceived on workshop:gameplay
→ CommitPrevious (Provisional → Committed on prior turn's records)
→ Enriching (sub-pipeline: classify → predict → arbitrate → synthesize)
→ ProcessingUpdate ("enriching" | "assembling" | "rendering")
→ AssembleContext (three-tier + directives.jsonl read)
→ Narrator streaming → NarratorProse chunks
→ NarratorComplete (full prose)
→ Persist turn N (events.jsonl + turns.jsonl)
→ TurnComplete (ready_for_input: true)
→ Await next input
```

### Shared mechanics

- Context assembly pipeline (three-tier with budget trimming)
- Narrator streaming with sentence/paragraph batching
- Persistence to `events.jsonl` and `turns.jsonl`
- `ProcessingUpdate` → `NarratorProse*` → `NarratorComplete` → `TurnComplete` event sequence

### Distinct mechanics

- Scene-start has no `CommitPrevious` or `Enriching` (no prior turn to process)
- Scene-start takes player character seed + intent as compositional input
- Scene-start emits `SceneReady` (per-turn does not)
- Per-turn emits `InputReceived` (scene-start does not)

---

## Section 5: Player Character Wizard Step

### New wizard step

The current wizard has 6 steps: Genre → Profile → Cast → Dynamics → Setting → Launch. The Player Character step is inserted after Dynamics:

```
Genre → Profile → Cast → Dynamics → Player Character → Setting → Launch
```

### Step contents

- **Name** — free text input
- **Age** — selection from descriptors in `storyteller-data/training-data/descriptors/axis-vocabulary.json` (age categories)
- **Gender presentation** — selection from `axis-vocabulary.json` gender descriptors (masc, fem, nb)
- **Intent** — free text input with a pre-checked checkbox: "Let the system decide my character's intention"
  - When checked: composer generates intent using the same mechanism as NPC goal generation
  - When unchecked: player's free text feeds into the player character's goal structure in `composition.json`

### Data flow

Player character data flows into the `ComposeScene` RPC as additional input fields. The composer:
1. Assigns the player character a full tensor representation (same as NPCs)
2. Parses stated intent into the goal structure (or generates goals if defaulted)
3. Writes the player character's goals into `composition.json`
4. Generates NPC goals that can respond to the player's stated intent

The `SceneReady` gameplay event includes `player_intent: Option<String>` so the frontend can render it in an unobtrusive UI element. The narrator also receives the intent in context, so the opening narration can naturally reinforce it.

### ML prediction continuity

The ML predictor runs on the player character just like NPCs. Divergence between player actions and predicted behavior is a narrative signal — actions out of keeping with the character's tensor produce tension or distress in the narrative, which is by design.

### Files affected

- `crates/storyteller-workshop/src/lib/SceneSetup.svelte` — new wizard step
- `crates/storyteller-workshop/src/lib/api.ts` — player character data in compose call
- `proto/storyteller/v1/composer.proto` — player character fields in `ComposeSceneRequest`
- `crates/storyteller-server/src/grpc/engine_service.rs` — handle player character input in composition

---

## Section 6: Async Integration Points — `directives.jsonl`

### New persistence artifact

File: `.story/sessions/{uuid}/directives.jsonl`

Append-only structured store for async agent outputs. Follows the same pattern as `turns.jsonl` and `events.jsonl`.

### Entry schema

```json
{
  "id": "ulid",
  "agency": "dramaturge | world_agent",
  "type": "arc_directive | spatial_enrichment | staleness_flag | ...",
  "applicable_turns": [3, 4, 5],
  "based_on_turns": [1, 2, 3],
  "payload": {},
  "timestamp": "2026-03-16T20:00:00Z"
}
```

- **`id`** — unique identifier (ULID for time-sortability)
- **`agency`** — which async agent produced this entry
- **`type`** — discriminator for the payload structure
- **`applicable_turns`** — which turns this directive is relevant for
- **`based_on_turns`** — provenance: which turns informed this directive
- **`payload`** — type-discriminated content (arc position, spatial data, staleness signals, etc.)
- **`timestamp`** — when the directive was written

### DirectiveStore

New persistence component with:
- **`append(entry)`** — write a directive entry to the file
- **`latest_by_agency(agency)`** — read the most recent entry for a given agent
- **`applicable_for_turn(turn_number)`** — read all directives applicable to a given turn
- **`last_n(n)`** — read the N most recent entries across all agencies

### Context assembly integration

Context assembly reads from `DirectiveStore` at assembly time. If directives exist, they fold into the narrator's context:
- Turn-level guidance → preamble (compact natural-language note)
- Active narrative forces → retrieved context (Tier 3)
- Arc position → signal for dramatic pressure level

If no directives exist (early turns, or before Tier C agents are implemented), context assembly proceeds without them. This is correct — early turns are establishment and don't need dramatic direction.

### A.3 scope

A.3 creates the `DirectiveStore`, wires the read path into context assembly, and adds `directives.jsonl` to session lifecycle management (creation on session start, inclusion in session resume). No writers are implemented — those come in Tier C (dramaturge C.1, world agent C.2).

### Files affected

- `crates/storyteller-server/src/persistence/directives.rs` — **new file**: `DirectiveStore` implementation
- `crates/storyteller-server/src/persistence/mod.rs` — modify: export `DirectiveStore`
- `crates/storyteller-server/src/persistence/session_store.rs` — modify: add `directives` field to `SessionStore`
- `crates/storyteller-engine/src/context/mod.rs` — modify: directive read path in `assemble_narrator_context()` function (the context module's assembly functions, called by the turn_cycle system)

---

## Section 7: Implementation Approach

Bottom-up infrastructure first, optimized for clarity and autonomous agent execution.

### Layer 1 — Structural Changes

No behavior change. Internal refactoring and type definitions.

1. Bevy pipeline collapse (`TurnCycleStage` 7→5, `EnrichmentPhase` sub-enum, `TurnCycleSets` consolidation)
2. `stream_complete()` on `LlmProvider` trait with default fallback, `NarratorTokenStream` / `NarratorTokenSender` newtypes
3. `GameplayEvent` discriminated union in proto and TypeScript types
4. `DirectiveStore` with append/query methods, `directives.jsonl` lifecycle
5. `ProcessingUpdate` thematic phrase definitions

### Layer 2 — Plumbing

Wiring infrastructure together. No new visible features yet.

6. `workshop:gameplay` Tauri channel emission in `commands.rs`
7. Frontend `workshop:gameplay` listener in types.ts and page component
8. `ExternalServerProvider` (Ollama) real streaming implementation
9. Sentence/paragraph batching logic in engine service
10. Context assembly directive read path (graceful on empty)
11. Wire player character data into `ComposeScene` RPC

### Layer 3 — Features

Visible behavior changes. Each delivers independently.

12. Wizard player character step (name, age, gender, intent, checkbox)
13. `SceneReady` emission with player intent on scene composition
14. `NarratorProse` streaming through gameplay channel
15. `ProcessingUpdate` emission at phase transitions
16. `InputReceived` / `TurnComplete` on gameplay channel
17. `StoryPane` incremental prose rendering from gameplay events
18. `InputBar` enable/disable driven by gameplay events

### Verification

- `cargo check --all-features` after each layer
- `cargo test --all-features` after Layer 1 (Bevy pipeline tests must pass with new stage structure)
- `bun run check` after Layer 3 (frontend type checking)
- End-to-end playtest: compose scene with player character → verify `SceneReady` → verify prose streaming → verify turn cycle

---

## What This Spec Does Not Contain

- **Dramaturge or World Agent implementation** — Tier C scope. This spec designs the integration surface (`directives.jsonl`) but does not implement writers.
- **`workshop:media` implementation** — deferred to Tier B. The gameplay channel design does not preclude later addition.
- **Database persistence layer** — `directives.jsonl` is a pre-database modeling surface, like `turns.jsonl` and `events.jsonl`.
- **Structured intent taxonomy** — player intent is free text for now. Structured goal templates are Tier B/C territory.
- **Multi-scene or chapter-level lifecycle** — this spec addresses single-scene turn lifecycle only. Scene resolution and scene-to-scene handoff are Tier C (C.3).

## Relationship to Prior Work

| Prior artifact | Relationship |
|---|---|
| `docs/superpowers/specs/2026-03-16-scenes-chapters-stories-meta-plan.md` | Parent meta-plan. This spec implements Work Unit A.3. |
| `docs/superpowers/specs/2026-03-16-sequential-rpcs-and-turn-zero-persistence-design.md` | A.1 and A.2 spec. A.3 builds on the per-step RPC pattern (A.1) and turn 0 persistence (A.2). |
| `docs/technical/roadmap/2026-03-16-scenes-chapters-and-stories.md` | Roadmap. Part I §2 (streaming), §5 (player intent review), plus turn lifecycle formalization. |
| `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md` | Established scene goals and per-character intentions. Player intent wizard step surfaces these to the player pre-composition. |
| `docs/plans/2026-03-13-engine-server-and-playtest-harness-design.md` | Established server/client architecture. A.3 builds streaming channels on this infrastructure. |
