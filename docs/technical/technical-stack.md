# Technical Stack

## Purpose

This document specifies the technology choices for the storyteller system — what we use, why we use it, and what role each technology plays. Every choice is evaluated against two principles:

1. **Essential simplicity**: Use the right tool for each job. Avoid redundant infrastructure. Prefer composition of well-understood components.
2. **Fit for purpose, not cargo culting**: Each technology earns its place by solving a specific problem better than the alternatives. We do not use a tool everywhere just because we use it somewhere.

### Relationship to Other Documents

- **`infrastructure-architecture.md`** describes how these technologies integrate — data flows, lifecycle, deployment topology.
- **`event-system.md`** describes the event lifecycle that spans these technologies (in-process Bevy events → durable PostgreSQL ledger → distributed RabbitMQ messages).
- **`entity-model.md`** describes the Bevy ECS entity model that these technologies serve.
- **`scene-model.md`** describes the scene lifecycle that drives data flow between stores.

---

## Core Runtime: Bevy ECS

### Role

Bevy is the game engine and runtime for the storyteller system. It provides the Entity-Component-System architecture that implements the Entity model from `entity-model.md`, the event system for in-process turn-cycle events, and the scheduler that coordinates agent processing during active play.

### Why Bevy

The Entity model's core design decision — everything is an Entity with a dynamic set of components — maps directly onto Bevy's ECS. This is not incidental; the Entity model was designed with ECS as the target implementation. The specific advantages:

- **Component-based composition**: Adding a `CommunicabilityProfile` to a stone is `commands.entity(stone_id).insert(CommunicabilityProfile { ... })`. No type hierarchy, no migration, no downcast. This is how promotion works.
- **Memory-contiguous component storage**: Bevy stores components of the same type contiguously in memory (archetype-based storage). When the frame computation system iterates over all entities with `PersonalityAxes` and `CommunicabilityProfile`, the data is cache-friendly. For a system that may process dozens of entity frames at scene entry, this matters.
- **System scheduling**: Bevy's system scheduler handles parallelism, ordering dependencies, and resource access control. The turn cycle's stages (structural parsing → factual classification → sensitivity matching → agent coordination → rendering) can be expressed as systems with explicit ordering, and Bevy ensures they execute correctly.
- **Event system**: Bevy's built-in `EventWriter<T>` / `EventReader<T>` handles the high-frequency, ephemeral events during the turn cycle. These events are typed, cheap, and don't need to survive a process restart because they're derived from durably stored inputs.

### What Bevy Does Not Do

Bevy is the runtime, not the persistence layer. All Bevy state (entities, components, events) is in-memory and volatile. The persistent state lives in PostgreSQL. Bevy is a hot cache that is loaded at scene entry and flushed at scene exit. If the process dies, nothing in Bevy is lost because the durable stores contain everything needed to reconstruct it.

Bevy also does not handle cross-service communication. Agent coordination within a single game session is in-process (Bevy systems and events). Communication with external services (tasker-core, ML inference, cloud LLM APIs) goes through gRPC or HTTP.

### Key Crate

`bevy` — we will use Bevy's ECS, event, and scheduling subsystems. We do not need Bevy's rendering, audio, or UI subsystems for the narrative engine (though they may be relevant if a graphical client is developed later). Bevy supports feature-gated compilation, so we include only what we need.

---

## Persistence: PostgreSQL + Apache AGE

### Role

PostgreSQL is the single persistent store for the storyteller system. With the Apache AGE extension, it also serves as the graph database for relational webs, narrative graphs, and setting topology. Everything that must survive a process restart lives here.

### What Lives in PostgreSQL

| Data | Table/Schema | Why |
|---|---|---|
| Event ledger | Append-only events table | Source of truth for everything that happened; supports replay and temporal queries |
| Session state | Sessions table with JSONB state | Player session tracking, resumption after disconnect |
| Checkpoints | Checkpoint table with full state snapshots | Save/restore, unwind points, scene-boundary consistency |
| Entity component snapshots | Entity + component tables | Persistent entity state between scenes; loaded into Bevy at scene entry |
| Tensor data | Serialized tensor elements | Character tensors, authored content, designer-placed data |
| Game metadata | Configuration tables | Story definitions, vocabulary registries, authored scene data |
| Player accounts | Users table | Authentication, session association |

### What Lives in Apache AGE

| Graph | Vertex Labels | Edge Labels | Why Graph |
|---|---|---|---|
| Relational web | Characters, Entities | Directed edges with substrate dimensions (trust[3], affection, debt, history, projection), information_state, configuration annotation | Multi-hop traversals for social distance, information propagation, power computation |
| Narrative graph | Scene nodes, Gate nodes | Directed edges with approach vectors, traversal conditions | Path-finding for reachable scenes, gravitational mass computation, posterior node evaluation |
| Setting topology | Settings, Zones | Geographic adjacency, containment, traversal cost | Spatial reasoning for movement validation, re-enterable setting lookup |

### Why PostgreSQL

PostgreSQL is already in the stack through tasker-core. It is the infrastructure dependency we are most confident in — mature, well-understood, operationally proven. Using it as the primary store means:

- **One database to operate**: No separate graph database server to deploy, monitor, back up, and upgrade.
- **Transactional consistency**: Graph updates and event ledger writes can happen in the same transaction. When a relational shift is confirmed, the edge update in AGE and the `RelationshipShifted` event in the ledger are atomic.
- **Shared tooling**: `sqlx` is the client for both relational and graph queries. The same connection pool, the same migration system, the same observability.
- **Proven at our scale**: The storyteller system's data is not large. A story has tens to hundreds of characters, hundreds of scenes, dozens of settings. PostgreSQL handles this trivially. The choice of database should be driven by the system's actual scale, not hypothetical future scale.

### Why Apache AGE (and Not a Separate Graph Database)

The storyteller system models three distinct graph structures: the relational web, the narrative graph, and the setting topology. These graphs are not independent — narrative mass is enriched by the concentration of power dynamics (relational web) among a scene's cast (narrative graph), and scene transitions reference setting adjacency (setting topology). Querying across these structures is a common operation, not an edge case.

**The co-location argument**: If the graphs lived in a separate database, cross-graph queries would require application-level joins — fetch from the graph database, fetch from PostgreSQL, combine in Rust. This is expensive, error-prone, and loses transactional guarantees. AGE eliminates this by making graph queries part of the PostgreSQL query engine.

**The alternatives considered**:

- **NebulaGraph**: Pete's prior experience. Excellent graph database. However, the Rust client ecosystem is not production-ready — the most mature community client was last updated in January 2023, all clients are pre-1.0 with heavy fbthrift dependencies, and there is no official Rust support. Using NebulaGraph would require maintaining a custom Rust client or running a Go/Python sidecar, both of which add complexity that the graph database was meant to reduce.
- **Neo4j**: Explicitly avoided. Licensing concerns and a vendor ecosystem that encourages lock-in.
- **ArangoDB**: Multi-model (document + graph + key-value). The "too multipurpose" problem — breadth of offerings makes the usability and stability of any single capability uncertain.
- **SurrealDB**: Rust-native, multi-model. Similar concern to ArangoDB — trying to be everything, which means being excellent at nothing in particular. Also young and rapidly changing.
- **Raw PostgreSQL (recursive CTEs)**: Technically sufficient for our graph sizes, but ergonomically painful. Writing a 3-hop traversal with property filters in recursive CTEs is verbose and error-prone. openCypher (AGE's query language) expresses the same query naturally.

**AGE's specific advantages**:

- **openCypher query language**: Well-documented, widely known. `MATCH (a)-[r:TRUST]->(b) WHERE r.competence > 0.5 RETURN a, b` is readable and expressive for graph patterns.
- **SQL + Cypher interop**: AGE queries can be embedded in SQL, and SQL queries can reference AGE graph data. This means the frame computation pipeline can join entity component data (SQL) with relational topology (Cypher) in a single query.
- **No additional client**: AGE queries go through the standard PostgreSQL wire protocol. `sqlx` sends them as parameterized queries. No special driver, no additional dependency.

### Graph Data Model

The three graph structures share a single AGE graph instance with distinct vertex and edge labels:

```
-- Relational Web
(:Character {entity_id, name})
(:Character)-[:RELATES_TO {
  trust_competence: f32, trust_reliability: f32, trust_benevolence: f32,
  affection: f32, debt: f32, history: text, projection: text,
  information_state: jsonb, configuration_annotation: text,
  provenance: text, last_updated: timestamp
}]->(:Character)

-- Narrative Graph
(:Scene {scene_id, name, scene_type, authored_mass, setting_ref})
(:Scene)-[:LEADS_TO {
  approach_vector: jsonb, traversal_conditions: jsonb,
  departure_type: text
}]->(:Scene)

-- Setting Topology
(:Setting {setting_id, name, zone, features: jsonb})
(:Setting)-[:ADJACENT_TO {traversal_cost: f32, direction: text}]->(:Setting)
(:Setting)-[:CONTAINS]->(:Setting)
```

Cross-graph queries are natural:

```cypher
-- "What characters have relational tension in scenes reachable from here?"
MATCH (current:Scene {scene_id: $current})-[:LEADS_TO*1..3]->(future:Scene),
      (a:Character)-[r:RELATES_TO]->(b:Character)
WHERE a.entity_id IN future.cast AND b.entity_id IN future.cast
  AND r.trust_competence < -0.3
RETURN future, a, b, r
```

---

## Messaging: RabbitMQ

### Role

RabbitMQ handles durable, cross-service message delivery. It is the communication layer between the storyteller engine and tasker-core, and the dispatch mechanism for distributed workflows.

### What Goes Through RabbitMQ

- **Tasker-core workflow dispatch**: Scene-boundary batch processing, deep interpretation overflow, cross-session background jobs.
- **Durable fan-out**: Events that multiple services need to know about (e.g., a session checkpoint that triggers both tasker-core workflows and analytics).
- **Event-driven notifications**: Tasker-core workflow completion notifications that route results back to the game engine.

### What Does NOT Go Through RabbitMQ

- **Turn-cycle events**: These are in-process Bevy events. The latency of an external message broker (even single-digit milliseconds) is unnecessary overhead for events that are produced and consumed within the same process in the same turn.
- **Agent coordination**: Agents within a scene communicate through Bevy's event system and shared ECS state, not through a message broker.
- **Player input/output**: Player communication goes through the game server's connection handler (WebSocket or gRPC stream), not through RabbitMQ.

### Why RabbitMQ

RabbitMQ is already the preferred messaging backend for tasker-core. Using it for storyteller integration means:

- **Shared infrastructure**: One message broker for the tasker-systems ecosystem.
- **Proven patterns**: Tasker-core's producer/consumer patterns, dead-letter handling, and retry logic are already implemented and tested.
- **Lower latency than PGMQ**: For multi-step workflows where messages cascade through dependent tasks, RabbitMQ's dedicated broker architecture saves tens of milliseconds per hop compared to PostgreSQL-backed queuing. This compounds across the six-step scene-boundary pipeline.

### Key Crate

`lapin` (v3.7) — AMQP 0.9.1 client for Rust. Async (tokio-compatible), connection pooling, publisher confirms. Already used in tasker-core.

---

## Workflow Orchestration: Tasker Core

### Role

Tasker Core handles all non-real-time processing that the storyteller engine delegates. Rather than reinvent workflow orchestration, retry logic, DAG-based task dependencies, and distributed state tracking, the storyteller system leverages existing infrastructure from the tasker-systems ecosystem.

### What Tasker Core Handles

The integration points are documented in `event-system.md`, "Workflow Orchestration: Tasker Core Integration." In summary:

- **Scene-boundary batch processing**: The six-step DAG (deferred event resolution → off-screen propagation → social graph ripple → narrative graph update → world state update → checkpoint) with parallel execution of independent steps and automatic retry on failure.
- **Deep interpretation overflow**: When Stage 4 classification cannot complete within the turn cycle's ~2s budget, it dispatches as a tasker task with results routed back via event notification.
- **Cross-session processing**: Long-running background work — narrative graph rebalancing, training data generation, entity decay across inactive sessions, world state evolution over elapsed real time.

### What Stays In the Game Engine

Everything on the turn cycle's critical path. If the player is waiting for a response, the work stays in-process. Tasker Core handles everything the player is not waiting for.

### Integration Pattern

The game engine dispatches workflows to tasker-core via RabbitMQ. Tasker-core processes them (potentially spawning multiple dependent steps), and sends completion notifications back via RabbitMQ. The game engine's event system picks up the notifications and integrates the results into the current scene state (if a scene is active) or queues them for the next scene entry.

---

## Inter-Service Communication: gRPC

### Role

gRPC is the protocol for machine-to-machine communication within the storyteller system. All structured communication between services uses Protocol Buffer definitions and tonic-generated clients and servers.

### Why gRPC Over REST

The storyteller system's inter-service communication is exclusively machine-to-machine — the game engine talking to ML inference services, to tasker-core's API, to any future microservices. In this context:

- **Type safety**: Protobuf service definitions are compiled into Rust types. A malformed request is a compile error, not a runtime surprise.
- **Efficiency**: Binary serialization (protobuf) is more compact and faster to parse than JSON. For high-frequency communication like ML inference requests, this matters.
- **Streaming**: gRPC supports bidirectional streaming. An LLM inference call that streams tokens back to the game engine as they're generated is a natural fit for gRPC server streaming.
- **Code generation**: `tonic` and `prost` generate both client and server code from `.proto` files. The service contract is defined once and enforced everywhere.

REST (via `axum`) remains available for any public-facing API that needs OpenAPI 3.x compatibility — player-facing endpoints, admin interfaces, health checks. But the core system communication is gRPC.

### Key Crates

- `tonic` (v0.14) — gRPC framework for Rust. Async, HTTP/2, TLS, interceptors.
- `prost` (v0.14) — Protocol Buffer implementation for Rust. Code generation from `.proto` files.
- `tonic-health` — gRPC health checking protocol implementation.
- `tonic-reflection` — gRPC server reflection for debugging and tooling.

---

## ML Inference and Training

### Role

The storyteller system uses machine learning at three distinct points, each with different requirements:

1. **Custom inference models** — the psychological frame computation layer, classifier agents, sensitivity matchers. Small, fast, domain-specific models that must run with low latency during the turn cycle.
2. **Large language model inference** — Character Agents, Narrator, Storykeeper, Stage 4 deep interpretation. Large models that generate natural language, called via API or run locally.
3. **Model training** — producing the custom models in (1) from training data.

### Custom Models: ort (ONNX Runtime)

The primary path for custom model deployment: train in Python (PyTorch), export to ONNX format, run in Rust via `ort`.

**Why this path**:
- PyTorch is the dominant training framework. The ecosystem of tutorials, pretrained models, and tooling is unmatched.
- ONNX is the standard model interchange format. Any PyTorch model can be exported to ONNX via `torch.onnx.export`.
- `ort` wraps Microsoft's ONNX Runtime, which applies hardware-specific optimizations automatically (CUDA, CoreML, TensorRT). The Rust code loads a `.onnx` file and runs inference without knowing or caring what hardware is available.
- `ort` is mature (~5.7M downloads), actively maintained, and supports both inference and training.

**What runs through ort**:
- **Psychological frame computation**: The ML inference layer between relational data and Character Agent performance (`power.md`). Takes substrate dimensions + topology + scene context as input, produces compressed psychological frames (~200-400 tokens) as output. Must complete in milliseconds at scene entry and incrementally during play.
- **Classifier agents** (Stages 1 and 3): If these graduate from rule-based heuristics to learned models, they export to ONNX and run through `ort`. Latency target: 10-100ms.
- **Any future custom model**: The pattern is general — train anywhere, deploy through ONNX.

**Compute isolation**: `ort` inference is synchronous and purely compute-bound (matrix operations, no I/O). It should not run on the tokio async runtime, which is optimized for I/O-bound cooperative tasks. The isolation strategy depends on the workload pattern:

- **Batch inference at scene entry** (computing frames for all cast members): `rayon` parallel iterators with work-stealing. The cast members are independent — `cast.par_iter().map(compute_frame)` distributes across CPU cores naturally. `rayon` is built on `crossbeam` and manages its own thread pool, fully isolated from tokio.
- **Incremental inference during play** (single-entity frame updates, classifier calls): `crossbeam` scoped threads allow the inference call to borrow tensor data directly from Bevy ECS components without cloning to satisfy `'static` bounds. A dedicated compute thread (or small pool) communicating via `crossbeam` channels keeps inference isolated from the async runtime while avoiding unnecessary copies.
- **Fallback**: `tokio::task::spawn_blocking()` works but is the least precise option — the blocking thread pool is shared with actual I/O-blocking work and requires `'static` data (forcing clones). Acceptable for prototyping, not ideal for production.

The right strategy will be confirmed by profiling actual inference workloads. The architectural commitment is: `ort` inference runs on dedicated compute threads, not the async runtime.

### Key Crates

- `ort` (v2.0.0-rc) — ONNX Runtime bindings for Rust. Supports 10+ execution providers (CUDA, TensorRT, CoreML, OpenVINO, etc.).
- `rayon` — Work-stealing parallelism for batch compute. Built on `crossbeam`.
- `crossbeam` — Scoped threads, lock-free channels, concurrent primitives. Used directly for incremental inference and as the foundation under `rayon`.

### Large Language Models: candle + API

LLM inference serves two deployment modes:

**Cloud LLMs (Claude, GPT)**: Called via HTTP/gRPC API. The game engine sends a prompt with context and receives a response. This is the primary mode for production deployment — cloud models are the most capable and require no local GPU infrastructure. Integration is straightforward HTTP (`reqwest`) or gRPC (`tonic`) calls.

**Local LLMs (development, testing, offline)**: Run quantized models directly in Rust via `candle`. This enables:
- Development without cloud API costs or latency
- Testing with reproducible model behavior (fixed weights, fixed seeds)
- Offline operation for demos or environments without internet

`candle` (Hugging Face's Rust ML framework) supports GGUF quantized models, the same format used by Ollama and llama.cpp. It runs transformer models natively in Rust with CUDA, Metal, and CPU backends. This means the game engine can load a local model in-process without a separate inference server.

The architecture abstracts over the deployment mode: Character Agents, Narrator, and Storykeeper produce prompts and consume responses. Whether the response comes from a Claude API call or a local Mistral model running in candle is a configuration choice, not an architectural one.

### Key Crates

- `candle-core`, `candle-nn`, `candle-transformers` (v0.9) — Hugging Face Rust ML framework. Transformer inference, GGUF quantization, multi-backend (CUDA, Metal, CPU).
- `reqwest` — HTTP client for cloud LLM API calls.

### Future Direction: burn

`burn` (Tracel AI) is a comprehensive Rust ML framework that supports both training and inference with swappable backends (CUDA, ROCm, Metal, Vulkan, WebGPU, CPU). If burn reaches sufficient maturity for our model architectures, it could replace the Python→ONNX→Rust pipeline entirely:

- **Train in Rust** (burn) instead of Python (PyTorch)
- **Deploy in Rust** (burn) instead of ONNX Runtime (ort)
- **Single language** for the entire ML pipeline — no Python dependency for custom models

This is a future option, not a current commitment. burn is at v0.20, actively developed, with a rapid release cadence. The decision to adopt it would be made when we begin training the psychological frame model — if burn's operator coverage, training utilities, and performance meet our needs at that point, we switch. If not, the Python→ONNX→Rust path is proven and reliable.

### Key Crate (future)

`burn` (v0.20) — Rust-native ML framework. Training + inference, swappable backends, `no_std` support.

### Training Infrastructure

Model training happens in Python (or potentially burn in the future). Training is not part of the game engine runtime — it produces artifacts (`.onnx` files or burn model files) that the engine loads.

Training jobs can be orchestrated via tasker-core:
- Combinatorial training data generation (tensor-schema-spec.md, Decision 4): genre/tone/tension matrices → scenario skeletons → LLM-generated training examples → schema validation
- Model training runs dispatched as tasker workflows with GPU worker support
- Trained model artifacts stored and versioned for deployment

---

## Shared Crate Ecosystem

The storyteller project shares infrastructure conventions with the tasker-systems ecosystem. Where tasker-core has proven a crate in production, storyteller adopts the same version and patterns.

### Core Dependencies

| Crate | Version | Role | Shared With |
|---|---|---|---|
| `tokio` | 1.49 | Async runtime (full features) | tasker-core |
| `serde` | 1.0 | Serialization (derive + std) | tasker-core |
| `serde_json` | 1.0 | JSON serialization | tasker-core |
| `sqlx` | 0.8 | PostgreSQL client (with AGE queries) | tasker-core |
| `lapin` | 3.7 | RabbitMQ AMQP client | tasker-core |
| `tonic` | 0.14 | gRPC framework | tasker-core |
| `prost` | 0.14 | Protocol Buffers | tasker-core |
| `tracing` | 0.1 | Structured logging | tasker-core |
| `tracing-subscriber` | 0.3 | Log formatting and filtering | tasker-core |
| `opentelemetry` | 0.31 | Distributed tracing | tasker-core |
| `uuid` | 1.11 | Entity IDs (v4, v7) | tasker-core |
| `chrono` | 0.4 | Timestamps | tasker-core |
| `thiserror` | 2.0 | Error types | tasker-core |
| `anyhow` | 1.0 | Error context | tasker-core |
| `axum` | 0.8 | HTTP (health checks, admin API) | tasker-core |
| `clap` | 4.5 | CLI argument parsing | tasker-core |
| `config` | 0.15 | Configuration management | tasker-core |

### Storyteller-Specific Dependencies

| Crate | Version | Role |
|---|---|---|
| `bevy` | latest | ECS runtime, event system, scheduling |
| `ort` | 2.0-rc | ONNX Runtime for custom ML models |
| `rayon` | latest | Work-stealing parallelism for batch inference |
| `crossbeam` | latest | Scoped threads, channels for compute isolation |
| `candle-core` | 0.9 | Local LLM inference |
| `candle-transformers` | 0.9 | Transformer model implementations |

### Conventions (tasker-systems)

- `#[expect(lint_name, reason = "...")]` instead of `#[allow]`
- All public types implement `Debug`
- All MPSC channels are bounded (no `unbounded_channel()`)
- `thiserror` for library error types, `anyhow` for application error handling
- `tracing` for all logging (not `log` or `println!`)
- `sqlx` with compile-time checked queries where practical

---

## Open Considerations

1. **AGE maturity verification**: Apache AGE should be evaluated with a spike — install the extension, model a subset of the relational web, run representative queries (multi-hop traversals, centrality approximation, cross-graph joins with SQL data), and measure performance. This should happen early in Phase 2.

2. **Bevy version pinning**: Bevy releases can include breaking API changes. The project should pin a specific Bevy version and upgrade deliberately, not track latest.

3. **candle vs. external inference server**: For local LLM development, running models in-process (candle) is simpler but couples GPU resource management to the game engine process. An alternative is running a local inference server (Ollama, vLLM) and calling it via gRPC/HTTP. The in-process approach is preferred for simplicity but may need revisiting if GPU memory management becomes complex.

4. **burn evaluation timing**: The decision to use burn instead of the Python→ONNX path should be made when we begin designing the psychological frame model's architecture. That's the first custom model and the natural evaluation point.

5. **AGE Cypher coverage**: Apache AGE implements a subset of openCypher. The specific graph algorithms we need (shortest path, variable-length path matching, property-filtered traversals) should be tested against AGE's implementation. If gaps exist, they can be supplemented with PostgreSQL recursive CTEs for specific queries.

6. **gRPC service definitions**: The `.proto` files for storyteller services should be designed alongside the Rust crate structure. Service boundaries in protobuf should mirror crate boundaries in the workspace.

7. **Player-facing protocol**: The technology for player-to-server communication (WebSocket, gRPC-Web, or something else) has not been decided. This depends on client architecture decisions that are outside the current scope but should be resolved before the game engine serves player connections.
