# storyteller

A world-building and storytelling engine — a multi-agent system where distinct AI agents collaborate to create interactive narrative experiences.

## What This Is

Storyteller is a system where a human player navigates a richly authored world through natural language, guided by AI agents that each hold partial knowledge of the story. Think theater company, not agent swarm: a Narrator who speaks to the player, a Storykeeper who guards the mystery, Character Agents who inhabit roles without knowing they're in a story, a Reconciler who sequences multi-character scenes, and a World Agent who translates the non-character world into narrative.

The system is built on several key ideas:
- **Narrative gravity** — stories are not branching trees but gravitational landscapes where pivotal scenes exert pull
- **Character as tensor** — characters are represented as multidimensional, context-sensitive models with geological temporal layers, not flat stat sheets
- **Imperfect information** — no agent has complete knowledge; richness emerges from the interplay of partial perspectives
- **Coherent worlds** — material conditions and social forms mutually produce each other; worlds feel real when their elements imply one another

## Status

**Pre-alpha.** The Rust workspace is scaffolded with four crates and compiles cleanly. No runtime logic has been implemented yet — implementation follows the extensive design documentation produced in Phase 1. A Python `doc-tools` package for extracting creative content from Scrivener and DOCX is functional.

**Related repositories**: [tasker-core](https://github.com/tasker-systems/tasker-core) (workflow orchestration), [tasker-contrib](https://github.com/tasker-systems/tasker-contrib) (framework integrations).

## Architecture

### Agents

| Agent | Role |
|---|---|
| **Storykeeper** | Holds the complete narrative graph, character tensors, relational web. Guards mystery and revelation. Filters what other agents may know. |
| **World Agent** | Holds world model (geography, physics, time, material state). Enforces hard constraints. Translates the non-character world's communicability for the narrative. |
| **Narrator** | Player-facing voice. Knows only the current scene and what Storykeeper reveals. Renders character intent in story voice. |
| **Character Agents** | Ephemeral — instantiated per-scene from tensor data and psychological frames. Express intent to Narrator. Don't know they're in a story. |
| **Reconciler** | Coordinates multi-character scenes. Sequences overlapping actions, resolves conflicts, surfaces dramatic potential. Adds no content. |
| **Player** | The only non-AI role. Has a tensor representation maintained by Storykeeper, but decisions are human. |

### Technology Stack

| Technology | Role |
|---|---|
| **Rust + Bevy ECS** | Core runtime — entities, components, systems. Agents run as in-process Bevy systems, not microservices. |
| **PostgreSQL + Apache AGE** | Unified persistence — event ledger, checkpoints, session state, and all graph data (relational web, narrative graph, setting topology). |
| **RabbitMQ** | Distributed messaging for tasker-core workflow integration. In-process events use Bevy's event system. |
| **gRPC (tonic)** | Machine-to-machine communication. REST only for public-facing APIs. |
| **ort (ONNX Runtime)** | Custom ML model inference — psychological frame computation, event classifiers. |
| **candle** | Optional local LLM inference for development and testing. |

See [`docs/technical/technical-stack.md`](docs/technical/technical-stack.md) for detailed rationale and [`docs/technical/infrastructure-architecture.md`](docs/technical/infrastructure-architecture.md) for how it all fits together.

## Workspace Structure

```
storyteller/
├── storyteller-core/           # Foundation: types, traits, errors, database, graph queries
├── storyteller-engine/         # Runtime: Bevy ECS, agents, turn cycle, ML inference
├── storyteller-api/            # HTTP layer: axum routes, session management (deployment-agnostic)
├── storyteller-cli/            # Binary: self-hosted server entry point
├── src/                        # Root crate: integration test coordinator
├── docs/
│   ├── foundation/             # Design philosophy and principles (9 documents)
│   ├── technical/              # Specifications and case studies (12 documents)
│   └── storybook/              # Symlink → storyteller-data repo (private, gitignored)
├── doc-tools/                  # Python package for Scrivener/DOCX extraction
├── cargo-make/                 # Build task templates
└── Cargo.toml                  # Workspace root
```

Dependencies flow in one direction: `cli → api → engine → core`. See [`docs/technical/crate-architecture.md`](docs/technical/crate-architecture.md) for the full breakdown.

## Development

### Rust

```bash
cargo make check                # All quality checks (clippy, fmt, test, doc)
cargo make test                 # Run all tests
cargo make build                # Build everything

cargo check --all-features      # Fast compilation check
cargo clippy --all-targets --all-features
cargo test --all-features
cargo fmt --check
```

### Python (doc-tools)

```bash
cd doc-tools
uv sync --dev                       # Install dependencies
uv run extract-scrivener <path>     # Extract Scrivener project
uv run convert-docx <path>          # Convert DOCX to structured markdown
uv run pytest                       # Run tests
uv run ruff check .                 # Lint
```

Requires Python >= 3.11. See `doc-tools/pyproject.toml` for configuration.

## Documentation

All design documentation lives in [`docs/`](docs/). See [`docs/README.md`](docs/README.md) for a full guide.

| Section | Contents |
|---|---|
| [`docs/foundation/`](docs/foundation/) | Design philosophy, system architecture, character modeling, narrative graph, world design, anthropological grounding, power, project organization |
| [`docs/technical/`](docs/technical/) | 12 documents — tensor case studies, schema specifications, entity model, scene model, event system, agent message catalog, relational web, crate architecture, technology stack, infrastructure |
| [`docs/storybook/`](docs/storybook/) | Source creative works — analytical references and workshop material |

## License

See [LICENSE](LICENSE).
