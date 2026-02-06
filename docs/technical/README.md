# Technical Specifications

Concrete data models, protocols, and case studies that bridge the foundation documents (philosophy) and the implementation (code). Where the foundation documents describe *what* and *why*, these documents describe *how* — specific representations, formats, formulas, and schemas.

These specifications are derived from case studies against real creative content. Each document records the design decisions that emerged from the process of making abstract concepts concrete.

## Documents

### Character Tensor Case Studies

Built from "The Fair and the Dead" characters to validate and refine the tensor model from `character_modeling.md`.

#### [tensor-case-study-sarah.md](tensor-case-study-sarah.md)

Sarah's full tensor representation — the first case study, establishing the representation format.

**Key decisions made here**:
- Tensor values as `[central_tendency, variance, range_low, range_high]` tuples
- Scale: `[-1.0, 1.0]` for bipolar axes, `[0.0, 1.0]` for intensity
- Contextual triggers as `(trigger_context, axis_shift_direction, magnitude)` triples
- Temporal layer tagging (`topsoil`, `sediment`, `bedrock`) on each element
- Full tensor ~2,500-3,000 tokens; activated subset ~800-1,200 tokens
- Two context-dependent activation examples (sickbed scene, other-bank scene)

#### [tensor-case-study-wolf.md](tensor-case-study-wolf.md)

The Wolf's tensor — a stress test for non-human entities.

**Key decisions made here**:
- Same axis structure works with reinterpreted labels (warmth→connection, optimism→presence)
- `primordial` temporal layer needed below `bedrock` for ancient entities
- `capacity_domain` field (natural, supernatural, conceptual) needed for the constraint framework
- Non-human tensor is *simpler* (~1,500-2,000 tokens), not more complex
- `expression_mode` guidance needed for non-verbal characters
- Contradictory orders (protect Sarah / ensure she fails) are a natural fit for motivational layering

### Narrative Graph Case Study

#### [narrative-graph-case-study-tfatd.md](narrative-graph-case-study-tfatd.md)

"The Fair and the Dead" Part I mapped as a gravitational landscape.

**Key decisions made here**:
- Narrative mass as composite: `authored_base + structural_modifiers + dynamic_adjustment(player_state)`
- Scene types: `gravitational`, `connective`, `gate`, `threshold`
- Approach vectors as state predicates over `(emotional, information, relational, physical)` state
- Gravitational pull formula: `effective_mass / narrative_distance^2` (inverse-square law)
- Connective space needs distributed mass (`mass_per_unit_time`), not point mass
- Player agency map identifying high-agency and low-agency points

### Agent Communication

#### [agent-message-catalog.md](agent-message-catalog.md)

Every message type exchanged between agents, with schemas, token budgets, and information boundaries.

**Key decisions made here**:
- Hybrid format: structured metadata (JSON) + natural language content
- Every message carries a context budget (max token count)
- 22 message types across 8 flow directions
- Full turn cycle: 3 serial LLM calls (single character), 4 (multi-character with parallel CharacterAgents)
- Information boundary is the most critical design element per message type

### Schema Specifications

#### [tensor-schema-spec.md](tensor-schema-spec.md)

Formal type system for character tensors, relational edges, and scene contexts — the structured representation that bridges case studies and computation.

**Key content**:
- Six element types: PersonalityAxis, CharacterCapacity, PersonalValue, Motivation, EmotionalState, EchoPattern
- Temporal layer semantics, inter-element relationships, the Intertwining (ports between inner and outer)
- Event-driven architecture with factual vs. interpretive conditions
- Frame computation pipeline (5 steps)
- Seven resolved design decisions: closed vocabularies, sensible defaults, entity model, training data, set-theoretic triggers, formal schema, performance architecture

#### [entity-model.md](entity-model.md)

Unified Entity model — everything in the system is an Entity with a component-driven lifecycle.

**Key decisions made here**:
- Single Entity type with component configuration (not a type hierarchy)
- Promotion/demotion lifecycle: ephemeral → persistent → full → decay → dissolution
- Communicability profile as four dimensions (surface area, translation friction, timescale, reciprocity)
- Decay mechanics with narrative weight as the key measure
- Non-character entities participate in power dynamics through communicability and topology
- Direct mapping to Bevy ECS (entities, components, systems)

#### [scene-model.md](scene-model.md)

The Scene as the fundamental unit of play — anatomy, lifecycle, action space, and the ludic contract.

**Key decisions made here**:
- Scene as bounded creative constraint (prevents aimlessness and context overwhelm)
- Scene anatomy: cast, setting, stakes/goals, entity budget, graph position, warmed data
- Rendered space: bounded meaningful action space defined by affordances, characters, entities, information, constraints
- Scene lifecycle: entry (frame computation + context warming), active play (turn cycle), exit (trajectory selection)
- Four scene types: authored gravitational, authored gate, connective (procedurally generated), threshold
- Ludic contract: Narrator as active guide, topographic signaling (not quest logs), narrative position awareness
- Graduated unsuccessful engagement response: invitation → character initiative → world pressure → gravitational escalation → narrative contraction
- Gravitational expression through character behavior, environmental signals, narrative coincidence, pacing

#### [event-system.md](event-system.md)

The event lifecycle — how narrative events are classified, routed, and processed across agents.

**Key decisions made here**:
- Two-track classification: factual events (fast, deterministic) and interpretive events (slower, LLM-mediated, confidence-weighted)
- Four-stage classification pipeline: structural parsing → factual classification → sensitivity matching → deep interpretation
- Sensitivity map: pre-computed trigger targets built at scene entry for fast interpretive matching
- Three priority tiers: immediate (this turn), scene-local (within the scene), deferred (scene boundary batch)
- Agent subscription model with pattern-based routing and priority levels
- Bounded cascade system with depth limits, refractory periods, and cycle detection
- Provisional judgments refined asynchronously — one-turn lag is narratively acceptable

### Relational Web Case Study

#### [relational-web-tfatd.md](relational-web-tfatd.md)

Asymmetric relational web for all 6 TFATD characters.

**Key decisions made here**:
- Each directed edge: ~150-250 tokens across 6 dimensions (trust[3], affection, debt, power, history, projection)
- Asymmetry is the default; symmetric relationships are a special case
- The `projection` field (what A assumes about B) is the primary source of dramatic irony
- The `debt` dimension is the plot engine in TFATD
- Unknown/sparse values handled gracefully — better sparse than hallucinated

### Infrastructure

#### [technical-stack.md](technical-stack.md)

Technology choices for the storyteller system — each technology's role, fit-for-purpose rationale, and alternatives considered.

**Key decisions made here**:
- Bevy ECS as core runtime (entity model, in-memory state, event system, scheduling)
- PostgreSQL + Apache AGE as unified persistence (event ledger, checkpoints, session state, AND graph data)
- AGE over NebulaGraph: Rust client ecosystem not production-ready; AGE eliminates cross-database joins and additional infrastructure
- RabbitMQ for distributed messaging (tasker-core integration, workflow dispatch)
- gRPC (tonic) for machine-to-machine communication (not REST — efficiency and type safety)
- `ort` for custom ML model inference (train in Python, deploy via ONNX in Rust)
- `candle` for local LLM inference (GGUF quantized models, development/testing)
- `burn` as future consideration (all-Rust training + inference pipeline)
- Shared crate ecosystem with tasker-core (sqlx, lapin, tokio, serde, tracing, tonic)

#### [infrastructure-architecture.md](infrastructure-architecture.md)

How the technology choices integrate into a coherent runtime — data flows, lifecycle, durability, resilience, deployment.

**Key decisions made here**:
- Agents run in-process (Bevy systems), not as microservices
- Command sourcing: player input persisted before processing (never lost)
- Three durability tiers: ephemeral (Bevy events), durable (PostgreSQL ledger), distributed (RabbitMQ → Tasker Core)
- Scene entry loads from PostgreSQL/AGE → warms Bevy ECS (~130-370ms)
- Scene exit flushes synchronously (entity state, checkpoints) then dispatches deferred work to tasker-core
- Truth set is a materialized view reconstructable from the event ledger
- Session resilience: server crash recovery via checkpoint + ledger replay, player disconnect with grace period, client-side input buffering
- All three graph structures (relational web, narrative graph, setting topology) in a single AGE graph with cross-graph Cypher queries
- LLM abstraction trait: `CloudLlmProvider`, `CandleLlmProvider`, `ExternalServerProvider` — agents don't know which is active

## Relationship to Other Documentation

```
docs/foundation/     →  Philosophy and principles (what and why)
docs/technical/      →  Specifications and case studies (how)  ← you are here
docs/storybook/      →  Source material (the stories themselves)
```

The technical documents reference foundation documents for principles and storybook content for examples. They are the bridge between thinking and building.

## Status

These are Phase 1 outputs — specifications derived from case study analysis. They will be refined as implementation begins (Phase 2) and discoveries from code reshape the models.
