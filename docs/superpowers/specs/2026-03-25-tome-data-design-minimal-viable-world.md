# Tome Data Design: Minimal Viable World

**Date:** 2026-03-25
**Status:** Design approved, pending implementation planning
**Epic ticket:** `2026-03-24-tome-data-design-minimal-viable-world`
**Branch:** `jcoletaylor/tome-data-design-minimal-viable-world`

---

## Context

The narrative grammar corpus (bedrock) is complete — 3,410 structured entities across 12 primitive types, 30 genres, loaded into PostgreSQL `bedrock.*` tables with the `BedrockQuery` trait providing 33 typed query methods. Bedrock describes *how narrative works* — the structural patterns, genre physics, archetype shapes, and dimensional profiles that constitute a grammar of storytelling.

Grammar is necessary but insufficient. A grammar without vocabulary produces stories that feel structurally correct but experientially hollow. The storyteller engine needs a **vocabulary layer** — specific worlds with specific material conditions, specific economies, specific histories, specific people — that gives grammar flesh.

This document specifies the design for the **Tome+Lore** data layer (sediment), which sits between bedrock (immutable grammar) and topsoil (live play state). Tome provides the world-shaping dimensional framework; Lore provides the populated instances. Together they constitute the vocabulary that the grammar activates.

### Key Inputs

- `docs/foundation/tome_and_lore.md` — grammar vs vocabulary distinction, the six lore domains
- `docs/foundation/world_design.md` — mutual production principle, authorial ingress points, coherence
- `docs/technical/storykeeper-context-assembly.md` — context packet, turn-lifecycle data flow
- `docs/technical/entity-model.md` — unified entity model, communicability gradient, promotion lifecycle
- `docs/technical/knowledge-graph-domain-model.md` — four graph structures, relational web, narrative graph
- Bedrock corpus: 12 types × 30 genres, `BedrockQuery` trait, `bedrock.*` schema

---

## The Three Geological Layers

The storyteller data architecture uses a geological metaphor for its three data layers:

**Bedrock** — Narrative grammar corpus. Immutable reference data. 12 primitive types (archetypes, dynamics, settings, goals, profiles, tropes, narrative shapes, ontological postures, spatial topology, place entities, archetype dynamics, genre dimensions) across 30 genre regions. 34 universal dimensions, 12 state variables. Queried via `BedrockQuery` trait. Schema: `bedrock.*`.

**Sediment (Tome+Lore)** — Authored or synthesized world vocabulary. Mutable between stories, stable during play. World-scoped — a single world can serve multiple stories and playthroughs. Contains both the dimensional framework for describing worlds (Tome) and the populated specifics of a particular world (Lore). Schema: `sediment.*` (future).

**Topsoil** — Live play state. Mutable per-session. Story-scoped. Runtime entities, event ledger, relational web, narrative graph, setting topology. Schema: `public.*` (existing migrations).

### Layer Relationships

**Bedrock → Sediment**: Genre grammar *constrains* world generation. Archetypes become *templates* for lore characters. Settings become *patterns* for lore places. Dynamics become *frameworks* for lore relationships. Lore entities carry references back to their bedrock primitives.

**Sediment → Topsoil**: Lore characters become runtime character sheets. Lore places become runtime settings with spatial data. World profiles inform story configuration. Lore relationships seed the relational web. Promotion from sediment to topsoil is *instantiation*, not creation — the lore already has the specifics.

**Sediment ↔ Topsoil (continuous pull)**: The narrative landscape is realized in topsoil during play, but sediment is pulled up on demand — scene-by-scene, sometimes turn-by-turn — as agents exert agency and gradient-matching requires detail. Once pulled up, lore becomes persistent story state. The world stays consistent because everything that could be promoted was already coherent in the sedimentary layer.

---

## Tome: The World-Shaping Dimensional Framework

### Why Tome Axes Differ from Bedrock Dimensions

Bedrock's 34 genre dimensions describe a *space* — they map regions, and a genre can sit anywhere without internal contradiction because genres are descriptive categories. Tome axes describe a *system* — they model a world that must be internally coherent, where values on one axis constrain what values are plausible on other axes.

A wealthy church in a feudal economy with universally poor adherents and no implication of corruption is *incoherent* — where does the money go? An arid-climate civilization with large-scale agriculture is *implausible* without an explanation (irrigation technology, magical water sources, imported food). Tome axes don't just describe a world's position in a dimensional space — they constrain each other through relationships of mutual production.

### The Mutual Production Graph

Tome axes are modeled as a **graph** where:

- **Vertices** are world-shaping axes (economic system, power topology, geography, religious framework, etc.)
- **Edges** are typed mutual production relationships between axes

This graph is the primary artifact of the Tome framework. It encodes what makes a world coherent — not through mechanical enforcement but as a *reasoning substrate for agents*. When the World Agent traverses the graph and finds that "arid climate *constrains* large-scale agriculture," it doesn't mean the system throws an error. It means the World Agent knows that a large farm in this world requires explanation — irrigation, magic, an aquifer — something the graph can absorb without incoherence.

The graph is declarative, not algorithmic. Edge types describe the *character* of relationships for agents to reason over, not gates that block or enable generation.

#### Vertex Schema: World Axis

```
WorldAxis {
    slug: String,                    // "land-tenure"
    name: String,                    // "Land Tenure System"
    domain: Domain,                  // material | economic | political | social | cultural | aesthetic
    axis_type: AxisType,             // numeric | categorical | ordinal | set | bipolar
    values_or_range: AxisValues,     // enum values, numeric range, or ordinal levels
    surfacing_argument: String,      // how this axis reaches gameplay
    provenance: Provenance,          // seed | elaborated | discovered
    source: String,                  // which document or elicitation pass produced it
}
```

#### Edge Schema: Mutual Production

```
MutualProduction {
    from_axis: String,               // "geography-climate"
    to_axis: String,                 // "land-tenure"
    edge_type: ProductionType,       // produces | constrains | enables | transforms
    description: String,             // natural language semantic description
    weight: f32,                     // strength of relationship (0.0-1.0)
    bidirectional: bool,             // whether the reverse also holds (often different type)
}
```

#### Edge Types

| Type | Meaning | Coherence Implication | Example |
|---|---|---|---|
| **produces** | A gives rise to B | If A is present, B is expected | feudal economy → hierarchical religion |
| **constrains** | A limits what B can be | If A is present, some B values are implausible | arid climate → no large-scale agriculture |
| **enables** | A makes B possible (but doesn't require it) | If A is absent, B needs alternative explanation | literacy → bureaucratic governance |
| **transforms** | A changes B's character over time | A's presence shifts B dynamically | war → land ownership restructured |

### Domain Clusters

The six domains from `tome_and_lore.md` serve as seed clusters in the graph:

1. **Material conditions** — geography, climate, natural resources, soil, water, disease ecology, infrastructure
2. **Economic forms** — how people make their living, production, trade, labor organization, debt, currency, land tenure
3. **Political structures** — formal power, authority legitimation, law, enforcement, institutions
4. **Social forms of production and reproduction** — kinship, gender roles, age hierarchies, class, marriage, inheritance, education
5. **History as active force** — how the past lives in the present: colonial/economic/social legacies visible in landscape and practice, patterns of displacement and return, the weight of what happened here. (Note: this domain describes axes for *how history operates* in a world — e.g., historical memory depth, trauma transmission mode, legacy visibility. Specific historical *events* are lore entities, not Tome axes.)
6. **Aesthetic and cultural forms** — music, food, architecture, clothing, speech, ritual, artistic traditions

Each domain starts with ~8-12 provisional axes. The elicitation methodology (Phase 1-2) discovers additional axes through `_commentary` and `_suggestions`, and the mutual production graph mapping (Phase 2) reveals which provisional axes are truly independent vs projections of the same underlying dimension.

### Surfacing Filter

Every axis must pass a surfacing test: **can this dimension, through the scene-entry or turn-lifecycle context assembly pipeline, inform the narrative experience?** The test traces a path from the axis through the system:

- Which agent consumes it? (World Agent, Storykeeper, Narrator, Dramaturge, Character Agent)
- At what pipeline stage? (Scene entry static context, per-turn character context, relational context, narrative position)
- In what form? (Material constraint, atmospheric detail, relational substrate, behavioral guidance)

Axes that can't reach gameplay within two hops of something an agent consumes are candidates for deferral — not killed, because mutual production may reveal their path, but flagged for review.

### Storage Strategy

During data elicitation and stabilization: structured JSON with AGE-ready labels and edge types. The data lives as files in `storyteller-data` following the same patterns as bedrock corpus data.

For persistence: Apache AGE in PostgreSQL, reusing the existing graph infrastructure. Axes as vertices, mutual production as typed edges. Cypher queries for coherence checking, subgraph clipping, world forking. This is future work — the schema design will be informed by the stabilized axis and edge data.

---

## Lore: Populated World Instances

### What Lore Is

Lore is the populated specifics of a particular world — named characters, places, organizations, historical events, cultural practices, artifacts. Lore entities are *positions in the Tome axis space elaborated into material detail*. Weyland-Yutani is a specific point in the economic/power/technology space; House Atreides occupies a different point. The richness of "synths" vs "replicants" vs "Cylons" emerges from where those worlds sit on the Tome axes.

### What Lore Is Not

Lore is not topsoil. Lore entities are the *vocabulary* that topsoil draws from, not the live state itself. Margaret's trust in the Outsider as a substrate value that changes turn-by-turn is topsoil. Margaret's history as a third-generation tenant farmer whose husband drowned in the well is lore — it doesn't change during play (though the *significance* of that history may be revealed progressively).

### Lore Entity Structure

Lore entities follow the unified entity model from `entity-model.md` — everything is an entity, differentiated by component configuration rather than type hierarchy. A lore entity is a named thing in the world with:

- **Bedrock references** (optional): archetype mapping, setting pattern, dynamic framework, trope connection. Not all lore entities have bedrock analogs — organizations, historical events, and artifacts emerge from the mutual production graph without a direct bedrock primitive.
- **Lore specifics**: name, history, material circumstances, cultural associations. The irreducible detail that makes "the granddaughter of a slave who used to work that field out back" *mean* something.
- **Mutual production connections** (required): which Tome axes this entity embodies or is situated within. This is what keeps lore world-coherent — nothing floats free of the axis graph.
- **Relational seeds**: initial relational substrate values, role assignments, dynamic references. These seed the relational web when the entity is promoted to topsoil.
- **Communicability hints**: pre-promotion estimates of surface area, translation friction, timescale. These inform what component set the entity receives upon promotion.

### Lore Entity Lifecycle

Lore entities move through three phases of concretization:

**Phase 1 — World Composition** (pre-play, sediment): The bulk creative work. Genre selections (bedrock) + world dimensional positions (tome) + mutual production graph → named characters with histories, places with material conditions, organizations, artifacts, historical events, relational web seeds, narrative landscape (scene graph). Runs once per world or world-region. Produces the stable lore body.

**Phase 2 — Story Initialization** (sediment → topsoil): When a story begins, the opening cast of lore entities are bulk-promoted to topsoil with the full entity-model component set. Characters gain runtime sheets, tensor elements, EntityIdentity, CommunicabilityProfile, PersistenceProfile. Places gain runtime settings with spatial data. Dynamics seed the relational web. The narrative landscape initializes the narrative graph. This is *instantiation*, not creation — the lore already has the specifics.

**Phase 3 — Mid-Play Pull-Up** (sediment → topsoil, on demand): During gameplay, when the narrative reaches a point requiring a lore entity not yet in topsoil — the player wanders to a new area, a dynamic activates an offscreen character, narrative gravity pulls a new scene — the Storykeeper pulls that entity from sediment and promotes it individually. The world stays consistent because the pulled entity was already coherent in the Phase 1 lore body.

### Lore Generation: Synthetic-First

Lore is primarily synthetically generated, not manually authored. The vision-level goal is to select composition sets of genre primitives (bedrock), position them on world dimensions (tome), and compose full narrative landscapes — complete with gravity scenes, attractor-basin plot directions, character arcs — populated with synthetic world data. This produces a playable story on its own, but it also becomes the grammar-and-vocabulary that authoring tools can elaborate, extend, or work alongside with novel generations.

Authoring tools become editing/extending tools for synthetically generated worlds, not blank-canvas creation. The grammar corpus is both reference data AND generative input.

---

## Elicitation Methodology

The methodology mirrors the proven Tier B approach used for bedrock data generation:

### The Discovery Loop

1. **Seed with provisional axes** from foundation docs (the six domains, specific examples already articulated)
2. **Elaborate within each domain** via LLM elicitation — "what other axes describe the space of possible economic forms?" — with `_commentary` and `_suggestions` preserved in each document
3. **Push toward density**: ~8-12 axes per domain, knowing some will collapse during mutual production mapping
4. **Genre-region coverage test**: a folk-horror village and a cyberpunk megacity should both be *describable* in the axis space
5. **Map mutual production edges**: LLM-assisted edge discovery — "what relationships between these axes want to exist?"
6. **Review commentary**: extract new axes, refine existing ones, re-run elicitation
7. **Iterate until stable**: new passes don't reveal significant new axes or edges
8. **Identify layer-2 intersections**: world-position × archetype, world-position × dynamics, etc. — warranting specific cross-prompts

### Key Difference from Tier B

Bedrock data generation was descriptive — mapping a space by its shapes and densities. Tome+Lore generation is *systemic* — modeling a world that must be self-consistent. The mutual production graph constrains generation: you can't generate a wealthy church in a feudal economy without the graph demanding an explanation for where the money flows.

This means elicitation prompts must include the mutual production context — not just "describe this axis" but "given these other axis positions, describe how this axis manifests and what it implies." The graph itself participates in generation, not just in validation.

---

## Epic Milestone Roadmap

### Phase 1: Seed Axes and Domain Clusters (First Ticket)

- Start from six domains in `tome_and_lore.md`
- Elaborate to ~8-12 axes per domain via LLM elicitation with `_commentary`/`_suggestions`
- Each axis gets: type, range/values, surfacing argument, domain membership, provenance
- Genre-region coverage test: can diverse worlds be described in this space?
- Surfacing filter applied throughout
- **Artifact**: Axis inventory document + provisional axis data (structured JSON, `_commentary` preserved)

### Phase 2: Mutual Production Graph

- Map edges between axes using four edge types (produces, constrains, enables, transforms)
- LLM-assisted edge discovery with `_commentary`
- Coherence testing: given partial axis positions, what does the graph demand?
- Iterate until graph stabilizes
- **Artifact**: Mutual production graph (structured JSON, AGE-ready labels). Revised axis inventory.

### Phase 3: Lore Elicitation Methodology + Entity Types

- Design prompts for generating lore entities from axis positions + bedrock references
- Define lore entity pre-promotion structure: bedrock references, lore specifics, axis connections, relational seeds, communicability hints
- Entity-model integration: lore entities carry enough structure for mechanical promotion to topsoil
- Layer-2 intersection prompts: world-position × archetype, world-position × dynamics
- **Artifact**: Elicitation methodology document. Lore entity structure definition. Prompt templates.

### Phase 4: First Test World

- One genre region, one world-dimensional position
- Full pipeline: axes → mutual production graph → lore generation → coherence validation
- Validate: do generated lore entities cohere? Could they instantiate cleanly to topsoil? Does the surfacing argument hold?
- **Artifact**: One test world with structured lore data. Framework gap assessment.

---

## Design Decisions

1. **Synthetic-first, not author-first.** Lore is primarily generated via LLM elicitation, with authoring as elaboration/extension of synthetic worlds. The grammar corpus is both reference data and generative input.

2. **Graph for mutual production, not relational tables.** The mutual production relationships between Tome axes are a genuine graph — traversal, subgraph clipping, coherence checking, world forking are natural graph operations. Apache AGE is the target persistence, reusing existing infrastructure.

3. **Declarative edges, not mechanical gates.** Edge types (produces, constrains, enables, transforms) describe the *character* of relationships for agents to reason over. The World Agent interprets the graph for coherence; the graph doesn't algorithmically block or enable.

4. **Sediment is world-scoped, not story-scoped.** A world serves multiple stories and playthroughs. Sediment is more mutable than bedrock (authoring tools evolve it) but stable during play.

5. **Lore entities are fluid, following the entity model.** No type hierarchy for lore entities. They are named things in the world with optional bedrock references and required mutual production connections. Component configuration determines behavior, not type.

6. **Surfacing filter throughout.** Every axis must demonstrate a path to gameplay through scene-entry or turn-lifecycle context assembly. Data that cannot reach, impact, inform, or add richness to the narrative experience is unnecessary.

7. **Axes-first, not world-first.** The first deliverable is the dimensional framework, not a playable world. Premature operationalization would narrow the axes to what's easy to populate, foreclosing dimensions that haven't been discovered yet.

8. **Domain-seeded graph expansion (Approach C).** Start with provisional axes from foundation docs, elaborate via LLM elicitation with `_commentary`/`_suggestions`, map mutual production edges, iterate until stable. Mirrors the proven Tier B methodology.

---

## Relationship to Other Documents

- **`docs/foundation/tome_and_lore.md`** — Establishes the grammar/vocabulary distinction. This document operationalizes it into a data architecture.
- **`docs/foundation/world_design.md`** — Establishes the mutual production principle and authorial ingress points. This document encodes mutual production as a graph structure.
- **`docs/technical/storykeeper-context-assembly.md`** — Defines the context assembly pipeline that Tome+Lore data must reach. The surfacing filter in this document validates against that pipeline.
- **`docs/technical/entity-model.md`** — Defines the unified entity model. Lore entities follow this model; promotion from sediment to topsoil is component addition, not type change.
- **`docs/technical/knowledge-graph-domain-model.md`** — Defines the four graph structures. The mutual production graph is a new fifth graph, distinct from but composable with the relational web, narrative graph, setting topology, and event dependency DAG.
- **`docs/superpowers/specs/2026-03-25-bedrock-query-trait-design.md`** — The bedrock implementation that this design builds upon. Tome+Lore will eventually have its own query trait (`SedimentQuery` or `TomeLoreQuery`) composing against `BedrockQuery`.
