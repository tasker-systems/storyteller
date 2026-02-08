# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) and other AI coding assistants when working with code in this repository.

## Project Overview

**storyteller** is a world-building and storytelling engine — a multi-agent system where distinct AI agents (Narrator, Storykeeper, Character Agents, Reconciler, World Agent) collaborate to create interactive narrative experiences. Think theater company, not agent swarm.

**Status**: Pre-alpha, first playable scene achieved. Character agents, narrator, storykeeper, and reconciler are functional. Interactive scene runs against local Ollama via `cargo run --bin play-scene`.

**Related repositories**: `tasker-core` (workflow orchestration, Rust), `tasker-contrib` (framework integrations).

## Development Commands

### Rust

```bash
# cargo-make (preferred)
cargo make check                # All quality checks (clippy, fmt, test, doc)
cargo make test                 # Run all tests
cargo make build                # Build everything
cargo make fix                  # Auto-fix all fixable issues

# Direct cargo commands
cargo check --all-features      # Fast compilation check
cargo build --all-features
cargo test --all-features
cargo test <test_name>          # Single test
cargo clippy --all-targets --all-features
cargo fmt
cargo fmt --check               # CI check
cargo doc --all-features --open # Generate and open docs

# Test tiers (feature-gated integration tests)
cargo test --workspace                                       # Unit tests only
cargo test --workspace --features test-ml-model              # + ONNX model tests
cargo test --workspace --features test-llm                   # + Ollama tests
cargo test --workspace --features test-ml-model,test-llm     # All tiers

# Cargo aliases (.cargo/config.toml)
cargo b                         # build
cargo r                         # run
cargo t                         # test
cargo c                         # check
cargo lint                      # clippy --all-targets --all-features -- -D warnings
```

### Python (doc-tools)

```bash
cd doc-tools
uv sync --dev                       # Install dependencies
uv run pytest                       # Run tests
uv run ruff check .                 # Lint
uv run ruff format .                # Format
uv run extract-scrivener <path>     # Extract Scrivener project to markdown
uv run convert-docx <path>          # Convert DOCX to structured markdown
```

Python config: `doc-tools/pyproject.toml` — requires Python >=3.11, ruff line-length 100, target py311.

## Workspace Architecture

### Crate Dependency Graph

```
storyteller-cli ──→ storyteller-api ──→ storyteller-engine ──→ storyteller-core
(self-hosted)       (routes, HTTP)      (Bevy ECS runtime)    (types, DB, graph)
```

The root `storyteller` crate is a workspace coordinator for integration tests only. Dependencies flow strictly downstream — no cycles, no reverse dependencies.

### storyteller-core

Foundation layer. Types, traits, errors, database operations, and graph queries. **No Bevy dependency** — the headless layer that can be tested and reused independently of the ECS runtime.

```
storyteller-core/src/
├── lib.rs              # Public API: re-exports StorytellerError, StorytellerResult
├── errors.rs           # StorytellerError enum with domain variants
├── config.rs           # StorytellerConfig, environment loading
├── types/
│   ├── entity.rs       # EntityId, EntityOrigin, PersistenceMode
│   ├── tensor.rs       # AxisValue, TemporalLayer, Provenance
│   ├── character.rs    # CharacterTensor, CharacterSheet, SceneData, ContextualTrigger
│   ├── message.rs      # PlayerInput, StorykeeperDirective, CharacterIntent, TurnPhase
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

**Dependencies:** serde, sqlx (PostgreSQL), uuid, chrono, thiserror, async-trait, tracing

### storyteller-engine

Bevy ECS runtime. Components, systems, agent implementations, ML inference, and messaging. This is where the storytelling happens.

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
│   ├── turn_cycle.rs       # Player input → classification → deliberation → rendering
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
│   └── classifier.rs   # NL input → typed events (factual + interpretive)
├── inference/
│   ├── frame.rs        # Psychological frame computation (ort/ONNX, rayon thread pool)
│   ├── cloud.rs        # CloudLlmProvider (reqwest, deferred)
│   ├── local.rs        # CandleLlmProvider (candle, feature-gated: local-llm)
│   └── external.rs     # ExternalServerProvider (Ollama — primary prototype LLM path)
├── workshop/
│   └── the_flute_kept.rs  # Hardcoded scene data for prototype testing
└── messaging/
    └── tasker.rs       # Deferred event dispatch to tasker-core via RabbitMQ
```

**Dependencies:** storyteller-core, bevy_app, bevy_ecs, ort, rayon, crossbeam, lapin, reqwest, candle (optional)

**Feature flags:**
- `local-llm` (optional) — enables `CandleLlmProvider` via candle
- `test-ml-model` (test only) — enables tests requiring ONNX model on disk
- `test-llm` (test only) — enables tests requiring running Ollama

### storyteller-api

Deployment-agnostic HTTP layer. Axum routes for player input, session management, and health checks. Produces a `Router` that any deployment target can mount.

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

**Dependencies:** storyteller-core, storyteller-engine, axum, tokio, serde, tracing

### storyteller-cli

Self-hosted entry point. Assembles the Bevy App with the engine plugin, starts the HTTP server, and provides CLI subcommands.

```
storyteller-cli/src/
├── main.rs             # CLI entry point (clap subcommands)
└── bin/
    └── server.rs       # Game engine server (Bevy App + tracing + engine plugin)
```

**Dependencies:** storyteller-core, storyteller-engine, storyteller-api, bevy_app, clap, config, tracing-subscriber

## Architecture

### Core Design: Imperfect Information

The system's defining architectural choice is **imperfect information by design**. No single agent has complete knowledge. Each agent has a specific role, perspective, and information boundary. Richness emerges from the interplay of partial perspectives. This principle extends to crate boundaries — each crate has a specific scope and does not know about layers above it.

**Agent roles and information boundaries:**

- **Storykeeper** holds the complete narrative graph, information ledger, character tensors, relationship web. Filters what downstream agents may know. Guards mystery and revelation.
- **World Agent** holds world model (geography, physics, time, material state). Enforces hard constraints (genre physics). Translates the non-character world's communicability for the narrative.
- **Narrator** is the player-facing voice. Knows only current scene and what Storykeeper reveals. Has voice/personality defined by story designer.
- **Character Agents** are ephemeral — instantiated per-scene from tensor data + psychological frames. Express intent to Narrator (who renders it in story voice). Don't know they're in a story.
- **Reconciler** coordinates multi-character scenes: sequences overlapping actions, resolves conflicts, surfaces dramatic potential. Adds no content.
- **Player** is the only non-AI role. Has a tensor representation maintained by Storykeeper but decisions are human.

### Key Concepts

- **Narrative Gravity**: Scenes have mass; stories bend toward pivotal moments. Not a branching tree — a gravitational landscape with attractor basins.
- **Character as Tensor**: Multidimensional personality/motivation/relationship representation with geological temporal layers (topsoil/sediment/bedrock). Context-dependent activation.
- **Scene as Unit of Play**: Scenes are bounded creative constraints providing cast, setting, stakes, entity budget, graph position, and warmed data.
- **Three-tier Constraints**: Hard (world physics, genre contract), Soft (character capacity in context), Perceptual (what can be sensed/inferred). Communicated narratively, never as bare refusals.
- **Psychological Frames**: ML inference layer between relational data and Character Agent performance. Computes contextual configurations, produces compressed frames (~200-400 tokens) for LLM agents.
- **Command Sourcing**: Player input persisted to event ledger before processing begins. Server crashes recoverable via checkpoint + ledger replay.

### Technology Stack

- **Bevy ECS** (`bevy_app` + `bevy_ecs` sub-crates, not the umbrella) for core runtime. Agents are in-process Bevy systems, not microservices.
- **PostgreSQL + Apache AGE** for unified persistence — event ledger, checkpoints, session state, AND all graph data (relational web, narrative graph, setting topology).
- **RabbitMQ** for distributed messaging to tasker-core. In-process events use Bevy's event system.
- **gRPC (tonic)** for machine-to-machine communication. REST only for public-facing APIs.
- **ort** (ONNX Runtime) for custom ML inference (frame computation, classifiers). Runs on dedicated rayon thread pool.
- **candle** (optional) for local LLM inference in development/testing.
- **LLM abstraction**: `LlmProvider` trait with `CloudLlmProvider`, `CandleLlmProvider`, `ExternalServerProvider`. Agents don't know which is active.

### Python doc-tools

```
doc-tools/src/doc_tools/
├── docx_reader.py        # DOCX extraction with ParagraphType enum
├── docx_converter.py     # CLI: DOCX → markdown with chapter splitting
├── markdown_writer.py    # Markdown output, slug generation
└── scrivener/
    ├── binder.py         # Scrivener XML parsing (BinderItem/ScrivenerProject dataclasses)
    └── extractor.py      # CLI: .scriv → structured markdown
```

## Rust Standards (tasker-systems conventions)

- Use `#[expect(lint_name, reason = "...")]` instead of `#[allow]`
- All public types must implement `Debug`
- All MPSC channels must be bounded (no `unbounded_channel()`)
- Follow Microsoft Universal Guidelines + Rust API Guidelines
- Workspace dependency versions in root `Cargo.toml` under `[workspace.dependencies]`; crates use `{ workspace = true }`
- Versions aligned with tasker-core where they overlap (tokio, serde, sqlx, lapin, tonic, tracing)

## Documentation

Design documentation in `docs/`:

| Section | Contents |
|---|---|
| [`docs/foundation/`](docs/foundation/) | 9 documents — design philosophy, system architecture, character modeling, narrative graph, world design, anthropological grounding, power, project organization, open questions |
| [`docs/technical/`](docs/technical/) | 12 documents — tensor case studies, schema specifications, entity model, scene model, event system, agent message catalog, relational web, crate architecture, technology stack, infrastructure |
| [`docs/ticket-specs/`](docs/ticket-specs/) | Implementation plans and ticket specifications |

**Private content**: Creative works and training data live in the separate `storyteller-data` repository, accessed via `STORYTELLER_DATA_PATH` (see `.env.example`).

See [`docs/README.md`](docs/README.md) for a full guide with reading order.
