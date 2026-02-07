# Documentation Hub

All design documentation for the storyteller project lives here, organized into three sections that correspond to different phases of the design-to-implementation pipeline.

## Sections

### [Foundation](foundation/) — Philosophy and Principles

Nine documents that establish the intellectual and architectural foundations of the system. These are essays, not specifications — they articulate *what* we are building and *why*, drawing on narratology, phenomenology, anthropology, network theory, game design, and the craft of fiction writing.

**Start here** if you want to understand the project's vision.

| Document | One-line summary |
|---|---|
| `design_philosophy.md` | Seven core principles: narrative gravity, imperfect information, character tensors, etc. |
| `system_architecture.md` | Agent roles (Narrator, Storykeeper, Character Agents, Reconciler, World Agent), information flow, constraints |
| `character_modeling.md` | Tensor representation with geological temporal layers (topsoil/sediment/bedrock) |
| `narrative_graph.md` | Gravitational model replacing branching trees — scenes have mass, stories have shape |
| `world_design.md` | Coherence principle, mutual production of material conditions and social forms, authorial ingress points |
| `anthropological_grounding.md` | Intellectual genealogy from Amazonian anthropological research into system design |
| `power.md` | Power as relational, emergent, and non-moral — the mechanism of change in the system |
| `project_organization.md` | Eleven work streams across three phases |
| `open_questions.md` | Eleven unresolved design challenges, honestly framed |

### [Technical](technical/) — Specifications and Case Studies

Concrete data models, protocols, and formulas derived from applying the foundation principles to real creative content. These bridge philosophy and code.

**Start here** if you want to understand the data structures and protocols. Twelve documents:

| Document | What it specifies |
|---|---|
| `tensor-case-study-sarah.md` | Full tensor for a human protagonist — establishes representation format |
| `tensor-case-study-wolf.md` | Full tensor for a non-human entity — stress-tests and extends the model |
| `tensor-schema-spec.md` | Formal type system for tensors, triggers, and frames — 7 resolved design decisions |
| `entity-model.md` | Unified Entity model — everything is an Entity with component-driven lifecycle |
| `scene-model.md` | The Scene as fundamental unit of play — anatomy, lifecycle, action space, ludic contract |
| `event-system.md` | Event lifecycle — classification pipeline, truth set, priority tiers, subscriber model |
| `narrative-graph-case-study-tfatd.md` | Story mapped as gravitational landscape — mass formulas, attractor basins |
| `agent-message-catalog.md` | All 22 message types between agents with schemas and token budgets |
| `relational-web-tfatd.md` | Asymmetric relational web for 6 characters across 6 dimensions |
| `technical-stack.md` | Technology choices — Bevy, PostgreSQL+AGE, RabbitMQ, gRPC, ML crates — with fit-for-purpose rationale |
| `infrastructure-architecture.md` | How it all fits together — data lifecycle, durability model, session resilience, deployment |
| `crate-architecture.md` | Rust workspace structure — five crates with strict layering, dependency graph, deployment strategy |

### [Storybook](storybook/) — Source Material

Creative works that serve as analytical references and workshop material. This content lives in the **private `storyteller-data` repository** and is symlinked into `docs/storybook/` for local development. It is not part of the engine repository's git history.

See the [storyteller-data README](https://github.com/tasker-systems/storyteller-data) for setup instructions and content details.

| Content | Role | Description |
|---|---|---|
| `the-fair-and-the-dead/` | Analytical reference | Dark fantasy — Sarah journeys into the Shadowed Wood to find her brother's lost spirit |
| `vretil/` | Analytical reference | Literary quest — 20 chapters of nested timelines, unreliable narration, mythopoetic ending |
| `bramblehoof/` | Workshop material | D&D satyr bard/warlock — the story we build, break, and learn with |

**Analytical references** are studied to validate modeling. **Workshop material** is where we experiment.

## How It Fits Together

```
Foundation (why)  →  Technical (how)  →  Implementation (code)
                          ↑                    ↑
                     Storybook (what)     Rust workspace
                     source material      storyteller-core,
                     that grounds         storyteller-engine,
                     specifications       storyteller-api,
                     in real creative     storyteller-cli
                     work
```

The foundation documents establish principles. The technical documents make those principles concrete through case studies against real stories. The implementation (Rust/Bevy workspace) builds from the technical specifications — see `technical/crate-architecture.md` for the workspace structure. The storybook content is the material that keeps everything grounded — without real stories, the abstractions are untested.

## Related Files (Repo Root)

| File | Purpose |
|---|---|
| `CLAUDE.md` | Developer guidance for AI coding assistants — project overview, commands, workspace architecture, Rust standards |
| `AGENTS.md` | Symlink to `CLAUDE.md` |
| `doc-tools/` | Python package for extracting content from Scrivener and DOCX |
