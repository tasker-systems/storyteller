# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**storyteller** is a world-building and storytelling engine — a multi-agent system where distinct AI agents (Narrator, Storykeeper, Character Agents, Reconciler, World Agent) collaborate to create interactive narrative experiences. Think theater company, not agent swarm.

**Status**: Pre-alpha, greenfield. Extensive design documentation exists; Rust/Bevy implementation has not yet begun. A Python `doc-tools` package for document extraction is functional.

**Related repositories**: `tasker-core` (workflow orchestration, Rust), `tasker-contrib` (framework integrations).

## Development Commands

### Rust (planned — once Cargo.toml exists)

```bash
cargo build --all-features
cargo test --all-features
cargo test <test_name>              # Single test
cargo clippy --all-targets --all-features
cargo fmt
cargo fmt --check                   # CI check
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

## Architecture

### Core Design: Agents as Collaborators with Partial Knowledge

The system's defining architectural choice is **imperfect information by design**. No single agent has complete knowledge. Each agent has a specific role, perspective, and information boundary. Richness emerges from the interplay of partial perspectives.

**Agent hierarchy and information flow:**

- **Story Designer** (human) authors narrative graphs, character tensors, scene definitions, world rules
- **Storykeeper** holds the complete narrative graph, information ledger, character tensors, relationship web. Filters what downstream agents may know. Guards mystery and revelation.
- **World Agent** holds world model (geography, physics, time, material state). Enforces hard constraints (genre physics). Translates the non-character world's communicability for the narrative. Collaborates with Storykeeper but maintains distinct domain — story ≠ world.
- **Narrator** is the player-facing voice. Knows only current scene and what Storykeeper reveals. Has voice/personality defined by story designer. Never lies outright.
- **Character Agents** are ephemeral — instantiated per-scene from Storykeeper's tensor data. Express intent to Narrator (who renders it in story voice). Don't know they're in a story.
- **Reconciler** coordinates multi-character scenes: sequences overlapping actions, resolves conflicts, surfaces dramatic potential. Adds no content — only structures what Character Agents express.
- **Player** is the only non-AI role. Has a tensor representation maintained by Storykeeper but decisions are human.

### Key Concepts

- **Narrative Gravity**: Scenes have mass; stories bend toward pivotal moments. Not a branching tree — a gravitational landscape with attractor basins.
- **Character as Tensor**: Multidimensional personality/motivation/relationship representation, not flat stat sheets. Context-dependent activation. Geological temporal model (topsoil/sediment/bedrock).
- **Three-tier Constraints**: Hard (world physics, genre contract), Soft (character capacity in context), Perceptual (what can be sensed/inferred). Constraints are communicated narratively, never as bare refusals.
- **N-dimensional Narrative Space**: Position, emotional valence, information state, relational state, thematic resonance — the same scene reached different ways is a different experience.

### Technical Direction

- **Rust + Bevy ECS** for core engine (entities = characters/locations/objects, components = traits/states/relationships, systems = agents/narrative engine)
- **Custom orchestration layer** — no third-party agent frameworks. Information flow is too specific to the design.
- **LLM integration** supporting both API models (Claude, GPT) and local models (Ollama)
- Eleven work streams across three phases; see `docs/foundation/project_organization.md`

### Python doc-tools Structure

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

## Foundation Documents

Design documentation in `docs/foundation/`:

| Document | Purpose |
|---|---|
| `design_philosophy.md` | Seven core principles (narrative gravity, character tensors, imperfect information, etc.) |
| `system_architecture.md` | Agent roles, information flow diagrams, constraint framework |
| `project_organization.md` | Eleven work streams, three phases, dependencies |
| `narrative_graph.md` | Gravitational model, attractor basins, scene design |
| `character_modeling.md` | Tensor representation, relational webs, temporal layers |
| `world_design.md` | Coherence principle, mutual production model, authorial ingress points, communicability gradient for entities |
| `anthropological_grounding.md` | Intellectual genealogy: Amazonian shamanism, exchange networks, embodiment → system design |
| `power.md` | Power as relational, emergent, non-moral — mechanism of change, psychological frames, network topology |
| `open_questions.md` | Unresolved design decisions |
