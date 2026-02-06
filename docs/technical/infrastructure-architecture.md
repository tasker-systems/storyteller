# Infrastructure Architecture

## Purpose

This document describes how the storyteller system's technology choices (`technical-stack.md`) integrate into a coherent runtime architecture. Where the stack document specifies *what* we use, this document specifies *how it fits together* — data flows, lifecycle management, durability guarantees, failure recovery, and deployment topology.

The organizing principle: **the player experience is the critical path.** Every architectural decision optimizes for responsiveness during play, durability of narrative state, and graceful recovery from failures. The system is designed so that the player never loses what they've said, never encounters an amnesiac character, and never waits longer than a few seconds for a response.

### Relationship to Other Documents

- **`technical-stack.md`** specifies the individual technologies. This document specifies their integration.
- **`event-system.md`** specifies the event lifecycle (classification, priority tiers, subscriber model). This document specifies the infrastructure that supports that lifecycle (durability, messaging, persistence).
- **`scene-model.md`** specifies the scene lifecycle (entry, play, exit). This document specifies the data flows at each stage.
- **`entity-model.md`** specifies the Entity component model. This document specifies how entity state moves between Bevy ECS and PostgreSQL.

---

## System Topology

### Service Boundaries

The storyteller system consists of four runtime components:

```
┌─────────────────────────────────────────────────────┐
│                   GAME ENGINE                        │
│  (Bevy ECS + agent orchestration + event system)     │
│                                                      │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │ Narrator  │ │Character │ │  Storykeeper      │   │
│  │          │ │ Agents   │ │                   │   │
│  └──────────┘ └──────────┘ └───────────────────┘   │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │Reconciler│ │World Agt │ │  Classifier Agts  │   │
│  └──────────┘ └──────────┘ └───────────────────┘   │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │  ML Inference (ort for frames, candle for    │   │
│  │  local LLMs — or API calls for cloud LLMs)   │   │
│  └──────────────────────────────────────────────┘   │
└──────────────┬────────────────┬───────────────┬─────┘
               │ sqlx           │ lapin         │ tonic/reqwest
               ▼                ▼               ▼
┌──────────────────┐  ┌──────────────┐  ┌──────────────┐
│   PostgreSQL     │  │  RabbitMQ    │  │ Cloud LLM    │
│   + Apache AGE   │  │              │  │ APIs         │
└──────────────────┘  └──────┬───────┘  └──────────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │   Tasker Core    │
                    │  (workflow       │
                    │   orchestration) │
                    └────────┬─────────┘
                             │ sqlx
                             ▼
                    ┌──────────────────┐
                    │   PostgreSQL     │
                    │   (shared or     │
                    │    separate)     │
                    └──────────────────┘
```

**Game Engine**: A Bevy application that runs the active play session. Hosts all agents in-process, manages the ECS entity state, runs the turn cycle, and coordinates ML inference. One instance per active play session (or multiple sessions per instance, depending on load — see Deployment Considerations).

**PostgreSQL + AGE**: Single database instance serving both the game engine and tasker-core (or separate instances if operational isolation is preferred). Stores all persistent state — event ledger, entity snapshots, session data, graph data.

**RabbitMQ**: Message broker for cross-service communication. The game engine publishes workflow dispatch messages; tasker-core consumes them. Tasker-core publishes completion notifications; the game engine consumes them.

**Tasker Core**: Existing workflow orchestration service. Consumes workflow dispatch from RabbitMQ, executes DAG-based task chains, publishes results back. Already deployed and operational.

### What Is NOT a Separate Service

Agents (Narrator, Storykeeper, Character Agents, World Agent, Reconciler) are not microservices. They are Bevy systems that run within the game engine process. Their "communication" is Bevy events and shared ECS state, not network calls. The agent-message-catalog defines logical message types, but at runtime these are in-memory data structures passed between systems.

ML inference for custom models (psychological frames, classifiers) runs in-process via `ort`. There is no separate ML inference service unless GPU resource isolation demands it (see Open Considerations).

LLM inference for agents is the one external call on the critical path — either a cloud API call or a local model running in candle. This is the primary source of turn latency (500ms-2s per LLM call).

---

## Data Lifecycle

### Scene Entry: Cold Store → Hot Cache

When a scene begins, the game engine loads everything the scene needs from PostgreSQL/AGE into Bevy ECS:

```
Scene Entry Pipeline:

  1. LOAD SCENE DEFINITION                              ~10ms
     FROM PostgreSQL: scene metadata, cast list, setting reference,
     goals, entity budget, authored mass
     → SceneDefinition component attached to scene entity

  2. LOAD ENTITY STATE                                  ~20-50ms
     FROM PostgreSQL: component snapshots for all cast members
     (tensor elements, persistence profiles, communicability)
     → Bevy entities spawned with full component sets

  3. LOAD GRAPH CONTEXT                                 ~20-50ms
     FROM AGE: relational web subgraph (edges between cast members),
     narrative graph neighborhood (current node + posteriors),
     setting state and adjacency
     → GraphContext resource available to all systems

  4. COMPUTE FRAMES                                     ~50-200ms
     IN BEVY: psychological frame computation for all cast members
     using ort (ONNX model) against loaded relational/tensor data
     → Frame components attached to character entities

  5. BUILD SENSITIVITY MAP                              ~20-50ms
     IN BEVY: pre-compute trigger targets from active subscriptions,
     echo patterns, scene goals, Storykeeper awareness
     → SensitivityMap resource available to classifier systems

  6. WARM NARRATOR CONTEXT                              ~10-20ms
     IN BEVY: assemble the Narrator's local context from scene
     definition, setting description, cast summaries, topographic data
     → NarratorContext resource

  TOTAL SCENE ENTRY: ~130-370ms
    (dominated by frame computation in step 4)
```

After scene entry, all data needed for real-time play is in Bevy ECS. The game engine does not query PostgreSQL or AGE during the turn cycle (except for the event ledger write — see below).

### Active Play: In-Memory Processing + Durable Writes

During a turn, processing happens in Bevy with two categories of external writes:

**Synchronous durable write (before processing begins)**:
```
Player sends input
  → Game engine receives input
  → WRITE to PostgreSQL event ledger: PlayerInput { session_id, turn, content, timestamp }
  → Acknowledge receipt to player client
  → Begin turn cycle processing in Bevy
```

This write is the **command sourcing guarantee**: the player's input is durably stored before the system processes it. If the server dies mid-turn, the input can be replayed.

**Asynchronous durable writes (during/after processing)**:
```
Turn cycle completes
  → WRITE to PostgreSQL event ledger: NarrativeEvents produced this turn
  → WRITE to PostgreSQL event ledger: InterpretiveJudgments confirmed this turn
  → UPDATE in PostgreSQL: session state (current turn, active scene)
  → Send narrative response to player client
```

These writes happen after the response is composed but can be batched for efficiency. They are not on the critical path for the player's perceived latency — the response is sent as soon as the Narrator renders it. Ledger writes can lag by a turn without affecting correctness, because the in-memory Bevy state is authoritative during play.

### Scene Exit: Hot Cache → Cold Store

When a scene ends, the game engine flushes accumulated state changes back to persistent storage:

**Synchronous (before scene transition)**:
```
Scene Exit — Immediate Flush:
  → WRITE to PostgreSQL: updated entity component snapshots (tensor topsoil changes,
    emotional states, persistence profiles, narrative weights)
  → WRITE to AGE: relational web edge updates (confirmed shifts from this scene)
  → WRITE to PostgreSQL: scene exit event, departure trajectory, new scene reference
  → WRITE to PostgreSQL: checkpoint (full serialized scene state for restore)
```

**Asynchronous (dispatched to tasker-core)**:
```
Scene Exit — Deferred Processing (via RabbitMQ → Tasker Core):
  Step 1: Deferred event resolution (tensor sediment/bedrock updates)
  Step 2: Off-screen character propagation (parallel with step 3)
  Step 3: Social graph ripple (parallel with step 2)
  Step 4: Narrative graph update (depends on steps 2, 3)
  Step 5: World state update (parallel with steps 2, 3)
  Step 6: Final checkpoint with all deferred effects resolved
```

The game engine does not wait for the deferred pipeline to complete. It can begin the next scene's entry pipeline immediately. The deferred pipeline results are incorporated when the game engine loads graph context for the new scene (Step 3 of scene entry) — if the deferred pipeline has completed by then, the data is fresh; if not, the game engine loads the pre-deferred state and applies deferred results when they arrive (via RabbitMQ completion notification).

### Between Sessions: Background Processing

When a player disconnects or a session times out:

```
Session Suspension:
  → Checkpoint current state (same as scene exit checkpoint)
  → Release Bevy ECS resources (despawn scene entities)
  → Session marked as suspended in PostgreSQL
  → Deferred processing continues via tasker-core if in progress

Session Resumption:
  → Load checkpoint from PostgreSQL
  → Check for completed deferred workflows (apply results)
  → Warm into Bevy ECS (same as scene entry)
  → Resume play from checkpoint position
```

Between sessions, tasker-core can run background processing:
- Entity decay across all tracked entities (not just those in the last scene)
- World state evolution (time passage, seasonal changes, economic shifts)
- Off-screen character goal progression
- Narrative graph rebalancing based on aggregate state
- Training data generation from session logs

---

## Durability Model

### The Three Guarantees

1. **Player input is never lost.** Every player statement is written to the event ledger before processing begins. If the server dies mid-turn, the input is recoverable.

2. **Confirmed state changes are never lost.** Every NarrativeEvent and confirmed InterpretiveJudgment is written to the event ledger. The event ledger is the source of truth. The Bevy ECS truth set is a materialized view that can be reconstructed from the ledger.

3. **Scene-boundary state is always consistent.** Every scene exit produces a checkpoint. The checkpoint contains enough state to reconstruct the full Bevy ECS for a scene entry at any future point. Deferred processing may be in-flight, but the checkpoint captures the pre-deferred state, and deferred results are applied on top when available.

### Event Ledger

The event ledger is an append-only table in PostgreSQL:

```sql
CREATE TABLE event_ledger (
    id          BIGSERIAL PRIMARY KEY,
    session_id  UUID NOT NULL REFERENCES sessions(id),
    scene_id    UUID,                              -- NULL for cross-scene events
    turn        INT,                               -- NULL for non-turn events
    event_type  TEXT NOT NULL,                      -- discriminator
    event_data  JSONB NOT NULL,                     -- serialized event payload
    confidence  REAL,                               -- NULL for factual events
    source      TEXT NOT NULL,                      -- which agent/system produced this
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- For temporal queries (After, Since, Sequence predicates)
    narrative_ts BIGINT NOT NULL                    -- monotonic narrative timestamp
);

CREATE INDEX idx_ledger_session_turn ON event_ledger(session_id, turn);
CREATE INDEX idx_ledger_scene ON event_ledger(scene_id);
CREATE INDEX idx_ledger_type ON event_ledger(event_type);
CREATE INDEX idx_ledger_narrative_ts ON event_ledger(narrative_ts);
```

The ledger serves multiple purposes:
- **Replay**: Reconstruct any turn's state by replaying events from the last checkpoint.
- **Temporal queries**: The `narrative_ts` column supports the temporal predicates from the trigger system (`After`, `Since`, `Sequence`).
- **Audit**: Complete record of everything that happened in a session, queryable for debugging, analytics, and training data extraction.
- **Cross-scene continuity**: Propositions that carry across scenes (the player still holds the stone, the relationship is still strained) are identified by querying the ledger for non-expired events.

### Checkpoints

Checkpoints are full state snapshots taken at scene boundaries:

```sql
CREATE TABLE checkpoints (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      UUID NOT NULL REFERENCES sessions(id),
    scene_id        UUID NOT NULL,
    turn            INT NOT NULL,
    checkpoint_type TEXT NOT NULL,                  -- 'scene_exit', 'manual_save', 'autosave'

    -- Serialized state
    entity_state    JSONB NOT NULL,                 -- all entity components
    truth_set       JSONB NOT NULL,                 -- current propositions
    narrative_position JSONB NOT NULL,              -- graph position, available posteriors
    world_state     JSONB NOT NULL,                 -- world agent state

    -- Metadata
    deferred_status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'complete'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

A checkpoint plus all ledger entries after it is sufficient to reconstruct any point in the session. The checkpoint is the "known good state"; the ledger entries are the delta.

### Truth Set Reconstruction

The in-memory truth set in Bevy is a performance optimization. It can be reconstructed from:

1. Load the most recent checkpoint's `truth_set` snapshot
2. Replay all ledger events since the checkpoint
3. Apply event → proposition translation rules
4. Result: the current truth set

This means the truth set is never a single point of failure. If Bevy loses it (process crash), it's rebuilt from durable storage.

---

## Session Resilience

### Failure Mode: Server Crash Mid-Turn

```
Timeline:
  T0: Player sends input
  T1: Input written to event ledger (synchronous, committed)
  T2: Turn cycle processing begins in Bevy
  T3: Server crashes

Recovery:
  T4: New game engine instance starts
  T5: Load session from PostgreSQL (last checkpoint)
  T6: Replay ledger events since checkpoint (including the T1 input)
  T7: Re-run turn cycle for the replayed input
  T8: Send response to player (if reconnected)
```

**What the player experiences**: A brief disconnection (seconds to tens of seconds, depending on recovery speed). When they reconnect, the game continues from where they were. Their input was preserved and produces a response. No narrative state is lost.

**Non-determinism caveat**: LLM calls are not deterministic — re-running the turn cycle with the same input may produce a different Narrator response or different Character Agent behavior. This is acceptable because the *factual* events are deterministic (same action, same constraint checks, same gate evaluations), and the *interpretive* results are probabilistic by design. The player gets a response that is consistent with what happened, even if the specific words differ from what the original (crashed) turn would have produced.

### Failure Mode: Player Disconnects Mid-Session

```
Timeline:
  T0: Player's connection drops
  T1: Game engine detects disconnect (heartbeat timeout)
  T2: Session marked as "disconnected" in PostgreSQL
  T3: Grace period begins (configurable, default 5 minutes)

  If player reconnects within grace period:
    T4: Session state still warm in Bevy ECS
    T5: Reconnect to existing session instantly
    T6: Any buffered client-side input is transmitted and processed

  If grace period expires:
    T4: Scene exit triggered (same as normal scene exit)
    T5: Checkpoint written, Bevy resources released
    T6: Session marked as "suspended"

  If player reconnects after suspension:
    T7: Load checkpoint + replay ledger delta
    T8: Warm into Bevy (full scene entry pipeline)
    T9: Resume play from checkpoint position
```

**What the player experiences**: If they reconnect quickly (within grace period), it's instant — they pick up where they left off. If they reconnect after the grace period, there's a brief loading pause (scene entry pipeline, ~130-370ms) and they resume from the last scene boundary.

**Client-side buffering**: The player's client application should buffer any text typed during disconnection. On reconnect, buffered input is transmitted to the server and processed as normal player input. The player never loses what they typed.

### Failure Mode: Server Crash Between Scenes

```
Timeline:
  T0: Scene exit completes, immediate flush done
  T1: Deferred processing dispatched to tasker-core via RabbitMQ
  T2: Server crashes before next scene entry

Recovery:
  T3: Tasker-core continues processing deferred work (independent of game engine)
  T4: New game engine instance starts
  T5: Load checkpoint from T0
  T6: Check deferred workflow status:
      - If complete: load results, proceed to next scene
      - If in-progress: proceed with pre-deferred state, apply results when they arrive
      - If failed: tasker-core retry handles it
```

**What the player experiences**: They reconnect and enter the next scene. If deferred processing completed, everything is up to date. If not, they start with the pre-deferred state — character reactions from the previous scene might not have fully propagated yet, but they will catch up within the next scene boundary cycle. This is narratively seamless because deferred effects (distant relationship changes, off-screen propagation) are not immediately visible to the player anyway.

---

## Event Flow Architecture

### Three Tiers of Durability

The event system (`event-system.md`) defines three priority tiers by urgency. The infrastructure maps these to three durability tiers:

| Priority Tier | Durability Tier | Transport | Persistence |
|---|---|---|---|
| Tier 1: Immediate | Ephemeral | Bevy events | None needed — derived from durable input |
| Tier 2: Scene-Local | Durable | Bevy events + PostgreSQL ledger write | Event ledger (append-only) |
| Tier 3: Deferred | Distributed | RabbitMQ → Tasker Core | Tasker-core workflow state + PostgreSQL |

**Ephemeral events** (Tier 1) are constraint checks, gate evaluations, cast changes. They happen in-memory during the turn cycle and don't need persistence because they are deterministic functions of the player's input (which IS persisted) and the current state (which is checkpointed). If the server crashes and replays, these events reproduce identically.

**Durable events** (Tier 2) are relational shifts, emotional changes, echo activations — events whose effects modify entity state. They are written to the event ledger as they are confirmed, ensuring that the state changes they represent survive a crash.

**Distributed events** (Tier 3) are dispatched as tasker-core workflows. They are durable by construction — RabbitMQ provides delivery guarantees, and tasker-core's workflow state machine tracks progress with PostgreSQL-backed state. If any component fails, the workflow retries from its last completed step.

### Player Input: The Special Case

Player input is the one event that crosses all three tiers:

1. **Durable** immediately — written to event ledger before processing
2. **Ephemeral** during the turn — the raw text is parsed, classified, and routed as Bevy events
3. **Distributed** if effects cascade — deferred consequences of the input dispatch via tasker-core

This ensures the player's contribution is never lost, regardless of where in the pipeline a failure occurs.

---

## Graph Data Modeling

### Schema Organization in Apache AGE

All three graph structures live in a single AGE graph, distinguished by vertex and edge labels. This enables cross-graph queries while maintaining logical separation:

```sql
-- Create the unified graph
SELECT create_graph('storyteller');

-- Vertex labels
SELECT create_vlabel('storyteller', 'Character');
SELECT create_vlabel('storyteller', 'Entity');
SELECT create_vlabel('storyteller', 'Scene');
SELECT create_vlabel('storyteller', 'Setting');
SELECT create_vlabel('storyteller', 'Zone');

-- Edge labels — Relational Web
SELECT create_elabel('storyteller', 'RELATES_TO');      -- character → character

-- Edge labels — Narrative Graph
SELECT create_elabel('storyteller', 'LEADS_TO');         -- scene → scene
SELECT create_elabel('storyteller', 'SCENE_AT');         -- scene → setting

-- Edge labels — Setting Topology
SELECT create_elabel('storyteller', 'ADJACENT_TO');      -- setting → setting
SELECT create_elabel('storyteller', 'CONTAINS');         -- zone → setting, setting → setting
```

### Relational Web Queries

Common query patterns for the relational web:

```sql
-- Load relational subgraph for a scene's cast
SELECT * FROM cypher('storyteller', $$
  MATCH (a:Character)-[r:RELATES_TO]->(b:Character)
  WHERE a.entity_id IN $cast_ids AND b.entity_id IN $cast_ids
  RETURN a, r, b
$$) AS (a agtype, r agtype, b agtype);

-- Social distance: shortest relational path between two characters
SELECT * FROM cypher('storyteller', $$
  MATCH path = shortestPath(
    (a:Character {entity_id: $from})-[:RELATES_TO*1..4]-(b:Character {entity_id: $to})
  )
  RETURN path
$$) AS (path agtype);

-- Find characters with high trust toward a target
SELECT * FROM cypher('storyteller', $$
  MATCH (a:Character)-[r:RELATES_TO]->(b:Character {entity_id: $target})
  WHERE r.trust_competence > 0.5 AND r.trust_benevolence > 0.3
  RETURN a, r.trust_competence, r.trust_benevolence
$$) AS (a agtype, tc agtype, tb agtype);
```

### Narrative Graph Queries

```sql
-- Reachable scenes from current position (up to 3 hops)
SELECT * FROM cypher('storyteller', $$
  MATCH (current:Scene {scene_id: $current})-[:LEADS_TO*1..3]->(future:Scene)
  RETURN future.scene_id, future.name, future.scene_type, future.authored_mass
$$) AS (id agtype, name agtype, scene_type agtype, mass agtype);

-- Cross-graph: scenes where relational tension exists among the cast
SELECT * FROM cypher('storyteller', $$
  MATCH (s:Scene)-[:LEADS_TO*1..2]->(future:Scene),
        (a:Character)-[r:RELATES_TO]->(b:Character)
  WHERE s.scene_id = $current
    AND a.entity_id IN future.cast
    AND b.entity_id IN future.cast
    AND (r.trust_competence < -0.3 OR r.debt > 0.5)
  RETURN DISTINCT future.scene_id, future.name,
         count(r) AS tension_edges
  ORDER BY tension_edges DESC
$$) AS (id agtype, name agtype, tension agtype);
```

### Setting Topology Queries

```sql
-- Adjacent settings with traversal cost
SELECT * FROM cypher('storyteller', $$
  MATCH (current:Setting {setting_id: $current})-[a:ADJACENT_TO]->(neighbor:Setting)
  RETURN neighbor.setting_id, neighbor.name, a.traversal_cost
  ORDER BY a.traversal_cost
$$) AS (id agtype, name agtype, cost agtype);

-- Cross-graph: what scenes exist at reachable settings?
SELECT * FROM cypher('storyteller', $$
  MATCH (current:Setting {setting_id: $current})-[:ADJACENT_TO*1..2]->(nearby:Setting),
        (scene:Scene)-[:SCENE_AT]->(nearby)
  RETURN nearby.name, scene.scene_id, scene.scene_type, scene.authored_mass
$$) AS (setting agtype, scene_id agtype, scene_type agtype, mass agtype);
```

### Graph ↔ SQL Integration

AGE queries can be combined with standard SQL joins, enabling queries that span graph and relational data:

```sql
-- Load character tensor data for characters with high relational tension
WITH tension_pairs AS (
    SELECT * FROM cypher('storyteller', $$
        MATCH (a:Character)-[r:RELATES_TO]->(b:Character)
        WHERE r.trust_competence < -0.3
        RETURN a.entity_id AS from_id, b.entity_id AS to_id,
               r.trust_competence AS trust
    $$) AS (from_id agtype, to_id agtype, trust agtype)
)
SELECT tp.from_id, tp.to_id, tp.trust,
       e.tensor_data
FROM tension_pairs tp
JOIN entity_snapshots e ON e.entity_id = tp.from_id::text::uuid
WHERE e.session_id = $1;
```

This is the specific capability that motivated co-locating all graph structures in AGE rather than using a separate graph database — graph traversals and relational queries compose in a single database round-trip.

---

## ML Pipeline

### Training → Deployment Flow

```
Training (Python / burn):
  1. Generate training data (combinatorial matrices → scenario skeletons → LLM expansion)
  2. Validate against schema (tensor-schema-spec.md, Decision 4)
  3. Train model (PyTorch or burn)
  4. Export (ONNX format if PyTorch; native burn format if burn)
  5. Validate inference quality
  6. Version and store artifact

Deployment (Rust):
  1. Game engine loads model at startup (ort for ONNX, burn for native)
  2. Model held in memory for the engine's lifetime
  3. Inference called synchronously during frame computation
     (wrapped in spawn_blocking for tokio compatibility)
  4. Model updates deployed by restarting the game engine with new artifact
```

### Inference Integration Points

| Model | Framework | When Called | Latency Target | Input | Output |
|---|---|---|---|---|---|
| Frame computation | ort (or burn) | Scene entry + incremental during play | 50-200ms (batch at entry), 10-30ms (incremental) | Substrate dimensions, topology, context | Compressed psychological frame (~200-400 tokens) |
| Stage 1 classifier | Rules → ort (future) | Every turn, before factual classification | 10-50ms | Raw player input | Action type, targets, keywords |
| Stage 3 sensitivity | Rules → ort (future) | Every turn, after factual classification | 20-100ms | Stage 1 output + sensitivity map | Provisional interpretive judgments |
| Character Agent LLM | candle / API | Every turn, agent coordination | 500ms-2s | Frame + scene context + prompt | Character intent (natural language) |
| Narrator LLM | candle / API | Every turn, narrative rendering | 500ms-2s | Coordinated agent outputs + scene context | Narrative text for player |
| Stage 4 deep interp. | candle / API | Async, parallel with agent coordination | 500ms-2s | Full turn context | Refined interpretive judgments |

### LLM Abstraction Layer

The game engine abstracts over LLM deployment mode. A trait defines the interface:

```rust
#[async_trait]
trait LlmProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_streaming(&self, request: CompletionRequest)
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;
}
```

Implementations:
- `CloudLlmProvider` — calls Claude/GPT via HTTP or gRPC
- `CandleLlmProvider` — runs a quantized model in-process via candle
- `ExternalServerProvider` — calls a local inference server (Ollama, vLLM) via HTTP/gRPC

The agent systems interact with `dyn LlmProvider` and don't know which implementation is active. Configuration determines the provider per agent role (the Narrator might use Claude while classifier agents use a local model).

---

## Deployment Considerations

### Required Services

A minimal deployment consists of:

| Service | Required | Notes |
|---|---|---|
| Game engine (Bevy) | Yes | One or more instances |
| PostgreSQL + AGE | Yes | Single instance sufficient at launch |
| RabbitMQ | Yes | Single instance sufficient at launch |
| Tasker Core | Yes | Existing deployment |
| Cloud LLM API access | For production | Claude or GPT API keys |
| Local LLM model files | For development | GGUF quantized models for candle |

### Development Configuration

For local development, all services run on a single machine:
- PostgreSQL with AGE extension installed
- RabbitMQ (or Docker container)
- Tasker Core (local instance)
- Game engine with candle-based local LLM (no cloud API needed)

### Scaling Characteristics

The game engine is the primary compute consumer. Its scaling characteristics:

- **CPU-bound**: Turn cycle processing, event classification, truth set evaluation
- **GPU-optional**: Frame computation (ort), local LLM inference (candle). Cloud LLM API calls offload this to the provider.
- **Memory**: Bevy ECS state for an active scene is modest (entity components for ~20-50 entities, truth set, sensitivity map). Estimated 10-50 MB per active session.
- **I/O**: PostgreSQL writes (event ledger) are the primary I/O on the critical path. One write per turn (~1 every few seconds). Not I/O-intensive.

Scaling is primarily horizontal — multiple game engine instances serving different sessions, all sharing PostgreSQL and RabbitMQ. Session affinity (routing a player to the same game engine instance) simplifies state management but is not required, because all state is reconstructable from PostgreSQL.

### Database Scaling

PostgreSQL with AGE handles the storyteller workload comfortably at launch scale:
- Graph queries over small graphs (hundreds of vertices, thousands of edges) are fast
- Event ledger writes are sequential (one per turn per session)
- Checkpoint writes are periodic (one per scene boundary)

If scale demands it, read replicas can serve graph queries while writes go to the primary. But this is unlikely to be needed in the near term.

---

## Open Considerations

1. **AGE extension deployment**: The AGE extension must be installed on the PostgreSQL instance. This is straightforward for self-managed PostgreSQL but may be constrained by managed database providers (AWS RDS, Google Cloud SQL, etc.). The deployment strategy should account for this — self-managed PostgreSQL, or a provider that supports custom extensions.

2. **Session-to-instance affinity**: Whether to use sticky sessions (route a player to the same game engine instance) or stateless routing (any instance can serve any session). Sticky sessions reduce checkpoint loading but complicate failover. Stateless routing is simpler but adds scene-entry latency on every request. The grace period model (keep state warm for N minutes) is a middle ground.

3. **Checkpoint size management**: Checkpoints include serialized entity state, truth sets, and world state. For long sessions with many tracked entities, checkpoints could grow. Compression and incremental checkpointing (delta from previous checkpoint) may be needed.

4. **Event ledger retention**: The append-only ledger grows indefinitely. A retention policy — archive old session events, summarize distant history, maintain queryability for temporal predicates — should be designed before the ledger becomes unwieldy.

5. **AGE query performance profiling**: The cross-graph queries shown above should be profiled against representative data. AGE's openCypher implementation may have performance characteristics that differ from dedicated graph databases, particularly for variable-length path queries. Early profiling prevents surprises.

6. **Multi-session game engine**: Whether a single game engine instance can serve multiple active sessions concurrently (different players in different stories) or whether each session gets a dedicated instance. The Bevy ECS supports this architecturally (separate entity hierarchies per session), but resource management (memory, LLM API rate limits, GPU) may favor isolation.

7. **LLM rate limiting and cost**: Cloud LLM API calls are the most expensive operational cost. The architecture should support rate limiting, cost tracking, and graceful degradation (fall back to local model if API is unavailable or budget is exceeded).

8. **Client-server protocol**: The player-to-game-engine protocol (WebSocket, gRPC-Web, HTTP long-polling) has not been specified. This depends on client architecture decisions and affects the game engine's connection handling layer.
