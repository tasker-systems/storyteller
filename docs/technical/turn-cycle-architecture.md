# Turn Cycle Architecture

> How the narrator-centric pipeline becomes a Bevy-native state machine.

## Actor-Command-Service → Bevy Mapping

The storyteller engine uses Bevy ECS as its runtime. This section maps
tasker-core's actor patterns to their Bevy equivalents, documenting what
translates directly and what requires different idioms.

| tasker-core | Bevy Equivalent | Rationale |
|---|---|---|
| `OrchestrationActor` (stateful, lifecycle hooks) | `Resource` (singleton, `Plugin::build` for init) | Both hold service state; Bevy lifecycle is Plugin-managed |
| `Handler<M>` (typed async dispatch) | System with `run_if` condition | Both dispatch on message/state; Bevy is declarative-ordered |
| `Message` (typed payload + Response) | Bevy `Event<T>` or direct return | Bevy events are fire-and-forget within a frame |
| `ActorRegistry` (centralized lifecycle) | `Plugin::build(app)` | Both register services and wire dependencies at startup |
| Actor wraps Service | Resource wraps service struct | EventClassifier, CharacterPredictor, NarratorAgent are services held as Resources |
| tokio mpsc channels | SystemSet ordering + Resource state | Bevy's schedule replaces message routing with declared execution order |

### What doesn't translate

- **Reactive dispatch.** Bevy systems run on a schedule, not on message
  arrival. There is no "wake on message" — the stage enum IS the routing
  table. Systems check `Res<ActiveTurnStage>` with `run_if` conditions.

- **Async handlers.** Bevy systems are synchronous within a single frame.
  Long-running async work (the Narrator LLM call) requires explicit
  bridging: start task → poll completion → advance stage.

- **Per-message routing.** tasker-core routes messages to handlers by type.
  Bevy routes execution by SystemSet ordering. The `TurnCycleStage` enum
  replaces the message type as the dispatch mechanism.

### What translates well

- **Service encapsulation.** Business logic (classify, predict, render)
  lives in framework-independent functions. Bevy systems are thin wrappers
  that read Resources, call service functions, and write results back.

- **Lifecycle management.** `Plugin::build` registers Resources, configures
  SystemSets, and adds systems — analogous to `ActorRegistry` startup.

- **Typed communication.** Bevy Events carry typed data between systems,
  though they are fire-and-forget rather than request/response.

### Design principle

Use Bevy as Bevy. Don't recreate tokio channels or actor mailboxes inside
ECS. Services that do actual work are framework-independent and testable
without Bevy. The Bevy layer is orchestration and state management only.

## Two Lifecycle Concepts

The system has two distinct lifecycle types. They are related by sequence
but have separate responsibilities:

### 1. `TurnCycleStage` — pipeline orchestration and observability

Drives which systems run and what observers report. Held as a Bevy
Resource (`ActiveTurnStage`). Systems gate on the current stage with
`run_if` conditions. Each active system advances the stage when its
work completes. `PhaseEvent` carries the current stage directly.

```
AwaitingInput → CommittingPrevious → Classifying → Predicting →
Resolving → AssemblingContext → Rendering → AwaitingInput
```

Eight variants. `AwaitingInput` is the rest state — no pipeline systems
run. The seven active stages model the full turn lifecycle including
commitment of the previous turn's provisional data.

**Why commitment comes first**: The player's response is what transforms
provisional ML/LLM outputs into committed data. When a player sends
non-X-card input, the previous turn's predictions (Hypothesized) and
rendering (Rendered) become Committed — written to the event ledger,
truth set updated. Only then does classification of the new input begin.
On the first turn of a scene, `CommittingPrevious` is a no-op.

### 2. `ProvisionalStatus` — data provenance

Tracks how final the data produced by inference stages is:
`Hypothesized → Rendered → Committed`. Defined in `event_grammar.rs`.

This models the *provenance* of ML/LLM outputs, not the turn itself:
- ML character predictions start as **Hypothesized** (working state)
- Narrator prose moves to **Rendered** (provisional, can be rejected)
- Player response triggers **Committed** (written to event ledger)

These two concepts are not redundant — they model different things:
- **Stage** answers "what is the pipeline doing right now?"
- **Status** answers "how final is this inference output?"

## TurnContext — Accumulated State

Each pipeline stage reads what prior stages produced and writes its output.
`TurnContext` is a Bevy Resource that accumulates context across stages
within a single turn. Reset at the start of each new turn.

```
                 TurnContext
┌───────────────────────────────────────────┐
│ player_input:    Option<String>           │ ← set by Input stage
│ classification:  Option<Classification>   │ ← set by Classifying stage
│ event_features:  Option<EventFeatureInput>│ ← derived in Classifying
│ predictions:     Option<Vec<Prediction>>  │ ← set by Predicting stage
│ resolver_output: Option<ResolverOutput>   │ ← set by Resolving stage
│ narrator_context:Option<NarratorContext>  │ ← set by AssemblingContext
│ rendering:       Option<NarratorRendering>│ ← set by Rendering stage
└───────────────────────────────────────────┘
```

`TurnContext` lives in `storyteller-engine` (not core) because it
references engine types like `ClassificationOutput`. The pure domain
enum `TurnCycleStage` lives in core because it has no engine dependencies.

## Async Bridge: Narrator Rendering

The Narrator LLM call is the one async operation in the pipeline. All
other stages (classification, prediction, resolution, context assembly)
are synchronous CPU-bound work.

The async bridge uses a three-state enum as a Bevy Resource:

```
NarratorTask::Idle
    → start_rendering_system spawns tokio task, transitions to InFlight
NarratorTask::InFlight(oneshot::Receiver)
    → poll_rendering_system checks receiver each frame
NarratorTask::Complete(Result)
    → result moved to TurnContext.rendering, stage advances to AwaitingInput
```

This avoids blocking the Bevy schedule while the LLM generates tokens.
The poll system runs every frame until the task completes.

## System Registration

Systems are organized into ordered `SystemSet`s that mirror the pipeline:

```
Input → CommittingPrevious → Classification → Prediction →
Resolution → ContextAssembly → Rendering
```

Each system is gated by `run_if(in_stage(TurnCycleStage::X))`. Only the
system matching the current stage runs. Stage advancement is explicit —
each system writes the next stage to `ActiveTurnStage` when done.

This is a single-threaded pipeline, not parallel execution. The stages
are sequential by design: each stage's output is the next stage's input.
Parallelism exists within stages (e.g., character predictions run in
parallel on a rayon pool), not between stages.

## Future: Bevy Events for Observability

Currently, `PhaseObserver` is a trait-based callback. The natural Bevy
evolution is to emit `Event<PhaseEvent>` from each system, with
observability systems consuming these events for logging, metrics, and
player-facing progress. This is a future enhancement — the current
architecture supports it by keeping the `PhaseEvent` type in core and
using `TurnCycleStage` directly for stage identification.
