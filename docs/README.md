# Documentation Guide

## How to Read These Docs

This project has ~60 documents across philosophy, technical specifications, implementation plans, and changelogs. They were written over an intensive design-and-build period (Feb 4-9, 2026) that included a significant **architectural pivot** on Feb 7 — from a multi-LLM-agent model to a narrator-centric single-LLM model. Understanding what's current vs. historical is essential.

### The Architectural Pivot (Feb 7, 2026)

The original design assumed multiple LLM agents — Narrator, Storykeeper, Character Agents, Reconciler, World Agent — each making LLM calls to produce their part of the narrative. The pivot simplified this to:

- **One LLM call per turn** (Narrator only)
- **ML prediction models** for character behavior (ONNX inference)
- **Deterministic rules engine** for conflict resolution
- **ML classification** for event extraction (fine-tuned DistilBERT)

The philosophical principles (imperfect information, character as tensor, narrative gravity, the constraint framework) are unchanged. The *implementation* of those principles changed substantially. Documents written before the pivot remain valuable for their conceptual content but their implementation details are historical.

---

## Recommended Reading Order

### If you want to understand the vision (start here)

1. **`foundation/design_philosophy.md`** — The seven core principles. Everything flows from here.
2. **`foundation/system_architecture.md`** — Agent roles and information boundaries. *Note: has architectural note about the pivot — the philosophical model is current, the multi-LLM implementation model is historical.*
3. **`foundation/character_modeling.md`** — Tensor representation with geological temporal layers. Fully current.
4. **`foundation/narrative_graph.md`** — Gravitational model for story structure. Fully current.

### If you want to understand the current architecture

1. **`technical/narrator-architecture.md`** — **Start here.** The post-pivot architecture document. Single Narrator LLM, ML character prediction, three-tier context assembly.
2. **`technical/turn-cycle-architecture.md`** — Bevy ECS mapping, 8-stage state machine, async LLM bridge.
3. **`technical/tensor-schema-spec.md`** — Formal type system for tensors, triggers, frames. 7 resolved design decisions. Fully current.
4. **`technical/entity-model.md`** — Unified Entity model, promotion lifecycle. Fully current.
5. **`technical/scene-model.md`** — Scene as unit of play. Conceptually current; implementation details may differ from turn pipeline reality.

### If you want to understand the ML pipeline

1. **`technical/ml_strategy/README.md`** — Overview of ML approach
2. **`technical/ml_strategy/character-prediction.md`** — ONNX character behavior prediction
3. **`technical/ml_strategy/event-classification.md`** — DistilBERT event classification
4. **`technical/ml_strategy/model-selection.md`** — Why DistilBERT over DeBERTa (MPS stability)
5. **`technical/ml_strategy/training-data-generation.md`** — Combinatorial template approach

### If you want to understand what was built

1. **`technical/roadmap/2026-02-09-foundations-to-minimum-playable.md`** — The comprehensive roadmap. Where we are, where we're going, what's been decided.
2. **`ticket-specs/event-system-foundations/implementation-plan.md`** — Master implementation plan for Phases A-F
3. Phase specs in `ticket-specs/event-system-foundations/` — Detailed per-phase documentation

### If you want the creative grounding

1. **`foundation/world_design.md`** — How worlds cohere
2. **`foundation/anthropological_grounding.md`** — Intellectual genealogy from Amazonian research
3. **`foundation/power.md`** — Power as relational, emergent, non-moral
4. **`foundation/emotional-model.md`** — Plutchik-derived emotional grammar, sedimentation mechanics
5. **`technical/tensor-case-study-sarah.md`** and **`tensor-case-study-wolf.md`** — Full tensor case studies against real characters
6. **`technical/narrative-graph-case-study-tfatd.md`** — Story mapped as gravitational landscape
7. **`technical/relational-web-tfatd.md`** — Asymmetric relational web for 6 characters

---

## Document Status Map

### Fully Current (post-pivot, reflects working code)

| Document | Description |
|---|---|
| `technical/narrator-architecture.md` | The architectural pivot document |
| `technical/turn-cycle-architecture.md` | Bevy ECS turn pipeline mapping |
| `technical/event-dependency-graph.md` | DAG architecture for combinatorial triggers |
| `technical/ml_strategy/*` (5 files) | ML approach documentation |
| `technical/roadmap/*` | Foundations to minimum playable roadmap |
| `ticket-specs/event-system-foundations/*` (8 files) | Implementation plans and phase specs |
| `changelog/*` (3 files) | Build milestones and observations |
| `workshop/*` (3 files) | Workshop scene and character specs |

### Stable Philosophy (pre-pivot, concepts fully valid)

| Document | Description |
|---|---|
| `foundation/design_philosophy.md` | Seven core principles |
| `foundation/character_modeling.md` | Tensor representation model |
| `foundation/narrative_graph.md` | Gravitational narrative model |
| `foundation/world_design.md` | World coherence principles |
| `foundation/anthropological_grounding.md` | Intellectual genealogy |
| `foundation/power.md` | Power as mechanism of change |
| `foundation/emotional-model.md` | Emotional grammar and sedimentation |
| `foundation/project_organization.md` | Work stream organization |
| `foundation/open_questions.md` | 11 open + 5 new + 6 resolved questions |

### Partially Superseded (concepts valid, implementation details historical)

These documents have architectural notes marking which sections are current vs. historical.

| Document | What's current | What's historical |
|---|---|---|
| `foundation/system_architecture.md` | Agent roles as conceptual boundaries, information flow philosophy, constraint framework | Character Agents as LLM agents, Reconciler as LLM, multi-agent message passing |
| `technical/agent-message-catalog.md` | Message format design, token budgets, Narrator ↔ Player protocol, information boundaries | CharacterAgent messages (sect. 4-5), Reconciler messages (sect. 6), multi-agent turn cycle |
| `technical/event-system.md` | Event taxonomy, priority tiers, truth set concept, sensitivity map, scene-boundary processing | Subscriber model, LLM-mediated classification pipeline, multi-agent turn integration |
| `technical/scene-model.md` | Scene anatomy, ludic contract, departure types | Some turn cycle details (see turn-cycle-architecture.md for current) |

### Infrastructure (current, independent of pivot)

| Document | Description |
|---|---|
| `technical/tensor-schema-spec.md` | Formal type system, 7 resolved decisions |
| `technical/entity-model.md` | Unified Entity model, promotion lifecycle |
| `technical/technical-stack.md` | Technology choices with rationale |
| `technical/infrastructure-architecture.md` | Data lifecycle, durability, deployment |
| `technical/crate-architecture.md` | Rust workspace structure |
| `technical/test-strategy.md` | Test approach and tiers |

### Case Studies (stable reference material)

| Document | Description |
|---|---|
| `technical/tensor-case-study-sarah.md` | Full tensor for human protagonist |
| `technical/tensor-case-study-wolf.md` | Full tensor for non-human entity |
| `technical/narrative-graph-case-study-tfatd.md` | Gravitational landscape analysis |
| `technical/relational-web-tfatd.md` | Asymmetric relational web, 6 characters |

---

## Directory Structure

```
docs/
├── README.md                  # This file
├── foundation/                # 10 docs — philosophy and principles
│   ├── design_philosophy.md
│   ├── system_architecture.md      # Has architectural note
│   ├── character_modeling.md
│   ├── narrative_graph.md
│   ├── world_design.md
│   ├── anthropological_grounding.md
│   ├── power.md
│   ├── emotional-model.md
│   ├── project_organization.md
│   └── open_questions.md           # 11 open + 5 new + 6 resolved
├── technical/                 # 19 docs — specifications and case studies
│   ├── narrator-architecture.md    # Post-pivot architecture (start here)
│   ├── turn-cycle-architecture.md  # Bevy ECS turn pipeline
│   ├── tensor-schema-spec.md
│   ├── entity-model.md
│   ├── scene-model.md
│   ├── event-system.md             # Has architectural note
│   ├── event-dependency-graph.md
│   ├── agent-message-catalog.md    # Has architectural note
│   ├── tensor-case-study-sarah.md
│   ├── tensor-case-study-wolf.md
│   ├── narrative-graph-case-study-tfatd.md
│   ├── relational-web-tfatd.md
│   ├── technical-stack.md
│   ├── infrastructure-architecture.md
│   ├── crate-architecture.md
│   ├── test-strategy.md
│   ├── ml_strategy/           # 5 docs — ML approach
│   └── roadmap/               # Foundations to minimum playable
├── ticket-specs/              # 3 subdirs — implementation plans
│   ├── event-system-foundations/   # 8 docs — Phases A-F
│   ├── narrator-centric-pivot/     # 3 docs — pivot implementation
│   └── storyteller-ml-foundations/ # 7 docs — Phase 0 ML pipeline
├── changelog/                 # 3 docs — build milestones
└── workshop/                  # 3 docs — workshop scene/character specs
```

## Source Material (Private Repository)

Creative works and training data live in the **private `storyteller-data` repository**, accessed via `STORYTELLER_DATA_PATH` (see `.env.example`).

| Content | Role |
|---|---|
| `the-fair-and-the-dead/` | Analytical reference (dark fantasy, 6 characters) |
| `vretil/` | Analytical reference (literary quest, 20 chapters) |
| `bramblehoof/` | Workshop material (D&D satyr, safe to experiment) |
| `training-data/` | ML pipeline training data (8,000 event classification examples) |

## Related Files

| File | Purpose |
|---|---|
| `CLAUDE.md` | Developer guidance for AI assistants |
| `AGENTS.md` | Symlink to `CLAUDE.md` |
| `doc-tools/` | Python package for Scrivener/DOCX extraction |
