# Crate Architecture

The storyteller Rust workspace is organized into five crates with strict layering. Each crate has a single responsibility, and dependencies flow in one direction — downstream crates never depend on upstream crates.

## Dependency Graph

```
storyteller-cli ──→ storyteller-api ──→ storyteller-engine ──→ storyteller-core
(self-hosted)       (routes, HTTP)      (Bevy ECS runtime)    (types, DB, graph)

storyteller-shuttle ──→ storyteller-api ──→ ...
(future: Shuttle.dev)
```

The root `storyteller` crate is a workspace coordinator for integration tests. It has dev-dependencies on all crates but no library code of its own.

## Crates

### storyteller-core

**Foundation layer.** Types, traits, errors, database operations, and graph queries. Has **no Bevy dependency** — this is the headless layer that can be tested, inspected, and reused independently of the ECS runtime.

```
storyteller-core/src/
├── lib.rs              # Public API: re-exports StorytellerError, StorytellerResult
├── errors.rs           # StorytellerError enum with domain variants
├── config.rs           # StorytellerConfig, environment loading
├── types/
│   ├── entity.rs       # EntityId, EntityOrigin, PersistenceMode
│   ├── tensor.rs       # AxisValue, TemporalLayer, Provenance
│   ├── event.rs        # EventId, EventPriority, NarrativeEvent
│   ├── scene.rs        # SceneId, SceneType, DepartureType
│   ├── relational.rs   # DirectedEdge, RelationalSubstrate, TopologicalRole
│   └── narrative.rs    # NarrativeMass, ApproachVector
├── traits/
│   └── llm.rs          # LlmProvider trait, CompletionRequest/Response
├── database/
│   ├── ledger.rs       # Event ledger (append-only log)
│   ├── checkpoint.rs   # Periodic state snapshots
│   └── session.rs      # Player session lifecycle
└── graph/
    ├── relational_web.rs  # Character relationship queries (AGE/Cypher)
    ├── narrative.rs       # Scene connectivity and gravitational landscape
    └── settings.rs        # Setting topology (re-enterable locations)
```

**Key dependencies:** serde, sqlx (PostgreSQL), uuid, chrono, thiserror, async-trait, tracing

**Design rationale:** Separating types and database access from the Bevy runtime means `storyteller-core` can be used by tooling, tests, migration scripts, and future crates without pulling in the ECS. It follows the same pattern as `tasker-shared` in the tasker-core workspace.

### storyteller-engine

**Bevy ECS runtime.** Components, systems, agent implementations, ML inference, and messaging. This is where the storytelling actually happens — the turn cycle runs here, agents deliberate here, and narrative events flow here.

```
storyteller-engine/src/
├── lib.rs              # Public API: re-exports StorytellerEnginePlugin
├── plugin.rs           # Bevy Plugin registration
├── components/
│   ├── identity.rs     # EntityIdentity (Bevy Component)
│   ├── communicability.rs  # CommunicabilityProfile (4 dimensions)
│   ├── persistence.rs  # PersistenceProfile
│   ├── tensor.rs       # FullTensorMarker and tensor data components
│   └── scene.rs        # ActiveScene (Bevy Resource)
├── systems/
│   ├── turn_cycle.rs   # Player input → classification → deliberation → rendering
│   ├── scene_lifecycle.rs  # Scene entry (frame computation, context warming), play, exit
│   ├── event_pipeline.rs   # Two-track classification (factual + interpretive)
│   ├── entity_lifecycle.rs # Promotion, demotion, decay
│   └── observability.rs    # Three-layer observability (system, session, player)
├── agents/
│   ├── narrator.rs     # Player-facing voice — renders character intent in story voice
│   ├── storykeeper.rs  # Guardian of complete narrative state — filters information downstream
│   ├── character.rs    # Ephemeral per-scene agents from tensor + psychological frame
│   ├── world.rs        # Translator for non-character entities (geography, weather, economies)
│   ├── reconciler.rs   # Multi-character scene coordinator — structures, adds no content
│   └── classifier.rs   # NL input → typed events (Stage 1 factual, Stage 3 interpretive)
├── inference/
│   ├── frame.rs        # Psychological frame computation (ort/ONNX, rayon thread pool)
│   ├── cloud.rs        # CloudLlmProvider (reqwest, feature-gated: cloud-llm)
│   ├── local.rs        # CandleLlmProvider (candle, feature-gated: local-llm)
│   └── external.rs     # ExternalServerProvider (Ollama, etc.)
└── messaging/
    └── tasker.rs       # Deferred event dispatch to tasker-core via RabbitMQ
```

**Key dependencies:** storyteller-core, bevy_app, bevy_ecs, ort, rayon, crossbeam, lapin, reqwest (optional), candle (optional)

**Feature flags:**
- `cloud-llm` (default) — enables `CloudLlmProvider` via reqwest
- `local-llm` (optional) — enables `CandleLlmProvider` via candle

**Design rationale:** Agents are Bevy systems — lightweight functions that query ECS state, call LLM providers, and emit events. They run in-process, not as microservices. The engine exposes a single `StorytellerEnginePlugin` that deployment crates add to their Bevy `App`. If the crate grows unwieldy, agents can be extracted into a separate crate later — but we start simple.

### storyteller-api

**Deployment-agnostic HTTP layer.** Axum routes for player input, session management, health checks, and middleware (auth, rate limiting). This crate produces a `Router` that any deployment target can mount.

```
storyteller-api/src/
├── lib.rs              # Public API: router(state) → axum::Router
├── state.rs            # AppState — shared state for handlers
├── routes/
│   ├── player.rs       # Player input/output (POST /api/v1/input)
│   ├── session.rs      # Session management (POST /api/v1/sessions)
│   └── health.rs       # Liveness probe (GET /health)
└── middleware/
    └── auth.rs         # Auth extraction and validation
```

**Key dependencies:** storyteller-core, storyteller-engine, axum, tokio, serde, tracing

**Design rationale:** The API layer is separated from the deployment layer so that the same route definitions serve both `storyteller-cli` (self-hosted, you run the binary) and a future `storyteller-shuttle` (Shuttle.dev, platform-managed). The deployment crate's job is to wire up infrastructure (database pools, secrets, TLS) and mount the router — not to define routes.

### storyteller-cli

**Self-hosted entry point.** A thin binary that assembles the Bevy App with the engine plugin, starts the HTTP server, and provides CLI subcommands for administration and development.

```
storyteller-cli/src/
├── main.rs             # CLI entry point (clap subcommands)
└── bin/
    └── server.rs       # Game engine server (Bevy App + tracing + engine plugin)
```

**Key dependencies:** storyteller-core, storyteller-engine, storyteller-api, bevy_app, clap, config, tracing-subscriber

**Design rationale:** This is the "run it yourself" deployment target. It initializes tracing, loads configuration, builds the Bevy App, and starts the server. Analogous to `tasker-cli` in the tasker-core workspace.

### storyteller-shuttle (future)

**Shuttle.dev deployment target.** Not yet created. Will use `#[shuttle_runtime::main]` to provision databases and secrets via Shuttle's infrastructure-from-code macros, then mount the same `storyteller-api::router()`.

**Design rationale:** Shuttle simplifies deployment for a user/customer-facing service — managed PostgreSQL, automatic TLS, zero-config scaling. The shuttle crate will be a thin adapter, just like `storyteller-cli`, but delegating infrastructure to the platform instead of managing it locally.

## Workspace Configuration

### Shared Dependencies

All dependency versions are declared in the workspace root `Cargo.toml` under `[workspace.dependencies]`. Crates reference them with `{ workspace = true }`. Versions are aligned with tasker-core where they overlap (tokio, serde, sqlx, lapin, tonic, tracing, etc.).

### Bevy Sub-Crates

The workspace uses `bevy_app` and `bevy_ecs` sub-crates directly rather than the `bevy` umbrella crate. Bevy 0.15's umbrella doesn't expose these as features, and we don't need rendering, audio, or UI — just the ECS scheduler, components, systems, and resources.

### Lint Configuration

Workspace-level lint configuration in `[workspace.lints]`:

**Clippy (Phase 1):**
- `correctness`, `suspicious`, `cargo` — category-level warnings
- `dbg_macro`, `undocumented_unsafe_blocks` — restriction lints
- `module_inception`, `multiple_crate_versions`, `uninlined_format_args` — permanent exceptions

**Rust:**
- `missing_debug_implementations` — all public types must implement Debug
- `redundant_imports` — catch accidental duplicate imports
- `unsafe_op_in_unsafe_fn` — require unsafe blocks inside unsafe functions

### Build Profiles

Four profiles matching tasker-core conventions:
- `dev` — fast iteration (no optimization, debug symbols)
- `release` — production (LTO, strip, abort on panic)
- `coverage` — instrumented for code coverage collection
- `profiling` — release optimization with debug symbols for flamegraphs

### Tooling

- `.cargo/config.toml` — aliases (`b`, `r`, `t`, `c`, `lint`)
- `.clippy.toml` — threshold configuration
- `cargo-make/` — base task templates extended by per-crate `Makefile.toml` files
- Root `Makefile.toml` — composite tasks: `cargo make check`, `test`, `fix`, `build`

## What Each Crate Knows

The information boundary design from the foundation documents maps onto crate boundaries:

| Crate | Knows about | Does NOT know about |
|---|---|---|
| `storyteller-core` | Types, schemas, database operations | Bevy, agents, game loop, HTTP |
| `storyteller-engine` | ECS, agents, turn cycle, ML inference | HTTP routes, deployment details |
| `storyteller-api` | Route definitions, request/response shapes | How it's hosted, infrastructure provisioning |
| `storyteller-cli` | How to start the server locally | Shuttle, cloud platforms |
| `storyteller-shuttle` | How to deploy on Shuttle.dev | Local server management |

This mirrors the imperfect information principle from the agent architecture — each layer has a specific perspective and information boundary.
