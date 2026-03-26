# Tome Phase 1: Seed Axes and Domain Clusters — Design Spec

## Context

First implementation phase of the Tier Betwixt milestone. The Tome framework defines
world-shaping axes — dimensional parameters that describe the space of possible worlds.
These axes sit in the sediment layer between bedrock (immutable genre grammar) and
topsoil (live play state). Unlike bedrock dimensions which describe *narrative space*
(how stories work in a genre), Tome axes describe *world space* (how a specific world
is structured materially, economically, politically, socially, historically, and culturally).

**Parent spec**: `docs/superpowers/specs/2026-03-25-tome-data-design-minimal-viable-world.md`
**Milestone**: `tier-betwixt-grammar-to-vocabulary`
**Ticket**: `2026-03-25-tome-phase-1-seed-axes-and-domain-clusters`

## Goal

Produce a complete axis inventory across six foundational domains, with structured
surfacing arguments and relevance conditions for each axis. The inventory must be
dense enough that diverse worlds feel describable and every axis demonstrates a
plausible path to gameplay.

## Decisions

### Axis Schema

Each axis carries:

| Field | Type | Description |
|-------|------|-------------|
| `slug` | string | Canonical identifier (e.g., `"land-tenure-system"`) |
| `name` | string | Human-readable name (e.g., `"Land Tenure System"`) |
| `domain` | string | Parent domain slug |
| `description` | string | What this axis describes and why it matters for world-building |
| `axis_type` | enum | `numeric`, `categorical`, `ordinal`, `bipolar`, `set`, `profile` |
| `values` | polymorphic | See Values section below |
| `surfacing` | object | Structured surfacing argument (see Surfacing section) |
| `provenance` | enum | `seed`, `elaborated`, `discovered` |
| `source` | string | Which document or elicitation pass produced this axis |
| `_commentary` | string | Design notes, open questions, observations |
| `_suggestions` | array[string] | Ideas for future iteration |

**Values** are polymorphic by `axis_type`:

- **numeric**: `{"low": 0.0, "high": 1.0, "low_label": "Sparse", "high_label": "Abundant"}`
- **bipolar**: `{"low": 0.0, "high": 1.0, "low_label": "Centralized", "high_label": "Distributed"}`
- **categorical**: `["communal", "feudal", "freehold", "state-owned", "corporate", "mixed-tenure"]`
- **ordinal**: `["none", "minimal", "moderate", "substantial", "dominant"]`
- **set**: `["agriculture", "mining", "fishing", "trade", "manufacturing"]` (multiple can be selected)
- **profile**: `{"sub_dimensions": ["mineral-deposits", "potable-water", ...], "levels": ["absent", "scarce", "limited", "moderate", "abundant", "dominant"]}` (map of sub-dimension to ordinal level — used when a single axis encompasses multiple related sub-dimensions that each need independent positioning)

The first five mirror bedrock dimensional extraction types (`bedrock.dimension_values`).
The `profile` type is new to Tome — it emerged because some world properties (resource
abundance, power distribution) are too coarse as a single value but too tightly coupled
to split into independent axes. A profile axis defines a fixed vocabulary of
sub-dimensions and ordinal levels; a world position assigns one level per sub-dimension.

**Cross-domain axes**: Some axes naturally span multiple domains (e.g., infrastructure
touches material conditions, economic forms, and political structures). Each axis
belongs to exactly one domain — its *primary* domain, where the axis is most
fundamentally rooted. Cross-domain relationships are captured in `_commentary`
during Phase 1 and become formal mutual production edges in Phase 2.

### Surfacing Argument

Every axis carries a structured surfacing argument:

```json
{
  "agents": ["world_agent", "storykeeper"],
  "pipeline_stages": ["scene_entry_static", "per_turn_character"],
  "pathways": ["material_constraint", "relational_substrate"],
  "relevance": "When scene involves land use, property disputes, inheritance, agricultural labor, or displacement. High relevance for rural and village settings.",
  "prose": "Determines who can own, use, and contest land. World Agent enforces property disputes and trespass. Storykeeper provides tenure context when characters discuss inheritance, debt, or displacement."
}
```

**Agents** (which agent consumes this data):
- `world_agent` — world model enforcement and interpretation
- `storykeeper` — context assembly and information filtering
- `narrator` — sensory rendering and atmospheric detail
- `dramaturge` — dramatic directive generation
- `character_agent` — behavioral guidance and goal framing

**Pipeline stages** (when in the turn cycle):
- `scene_entry_static` — cached per scene, loaded at scene initialization
- `per_turn_character` — refreshed each turn for character context
- `relational_context` — graph traversal for character relationships
- `narrative_position` — beat and shape context

**Pathways** (how the data reaches gameplay):
- `material_constraint` — World Agent says what's possible/impossible
- `atmospheric_detail` — Narrator's sensory rendering
- `relational_substrate` — how characters interact
- `behavioral_guidance` — what characters think is thinkable

**Relevance** (when this axis matters enough to surface):
Prose describing the conditions under which this axis earns its tokens in a context
window. Guides future Storykeeper filtering decisions but serves as an elicitation
quality gate now — if you can't articulate when an axis matters, it may not pass
the surfacing filter.

**Architectural note — agent routing**: The question of whether Storykeeper always
provides Tome data to World Agent (simple, higher baseline context cost) or
filters by scene profile tags (conditional, lower cost, more complex) is deferred
to SedimentQuery trait design. The `relevance` field captures the metadata needed
for either approach. For now, it primarily guides elicitation quality — axes with
vague or universal relevance conditions warrant scrutiny.

### Mutability

Tome axes are sediment-layer data — stable during play, mutable between stories.
If a world property changes during play (political stability eroding, economic
collapse), that change happens in topsoil as a state variable that was *initialized
from* a Tome axis position. The axis definition itself does not change mid-play.
No `can_shift_during_play` flag in Phase 1; this may be revisited if Phase 2
mutual production mapping reveals axes that naturally want to describe their
volatility.

### World Affordance Axes

A distinct cluster within the material conditions domain captures what the world
*permits* at a physical and metaphysical level. These axes describe world mechanics
rather than narrative stance:

- **Bedrock** says: "this genre treats magic as [rare/common/absent] and functions
  narratively as [wonder/corruption/tool]" (genre dimension `magic`)
- **Tome** says: "in this world, magic mechanically does [specific capabilities and
  constraints]" (world affordance axis)

The two must be *compatible* but aren't *identical*. A dark fantasy world where
magic is mechanically powerful but narratively treated as corrupting is coherent.
A cozy mystery world where magic is mechanically devastating but narratively treated
as whimsical probably isn't.

World affordance axes explicitly reference their bedrock genre dimension counterparts
in their `_commentary`. This cross-layer relationship becomes a formal edge in
Phase 2 (mutual production graph mapping).

Examples of world affordance axes:
- Supernatural permeability (how porous is the mundane/supernatural boundary)
- Technological ceiling (what level of sophistication exists and functions)
- Divine responsiveness (do the gods/forces answer, and how)
- Biological plasticity (can bodies be enhanced, transformed, transcended)
- Physical law flexibility (how much do real-world physics apply)

These axes carry particularly strong surfacing arguments because they directly
power World Agent adjudication — grounded, in-world pushback ("the world doesn't
work that way here") rather than meta-narrative refusal ("that's not genre-appropriate").

### Deferred Axes

Axes that fail the surfacing filter are not deleted. They move to a `_deferred`
array in the domain file with a note explaining why they were deferred:

```json
{
  "_deferred": [
    {
      "slug": "linguistic-diversity",
      "name": "Linguistic Diversity",
      "reason": "No clear surfacing path — would require language-modeling capabilities that don't exist in the agent pipeline. May become relevant if translation/miscommunication mechanics are added.",
      "provenance": "elaborated"
    }
  ]
}
```

The deferred corpus serves multiple purposes:
- Phase 2 mutual production mapping may reveal indirect surfacing paths
- In aggregate, deferred axes document "expressible things in world-building" that
  the system intentionally does not act on — making the opinionated boundary visible
- Future authoring tools can surface deferred axes as creative aids — nothing wrong
  with an author developing theories and flavor text that the system won't mechanically
  act on, as long as the boundary is clear

## Data Organization

### File Structure

```
storyteller-data/narrative-data/tome/
└── domains/
    ├── material-conditions.json
    ├── economic-forms.json
    ├── political-structures.json
    ├── social-forms.json
    ├── history-as-force.json
    └── aesthetic-cultural-forms.json
```

Domains are the organizational unit, paralleling genres as the organizational unit
in bedrock. Each file is self-contained with domain metadata, axis array, and
domain-level commentary.

### Domain File Structure

```json
{
  "domain": {
    "slug": "material-conditions",
    "name": "Material Conditions",
    "description": "Geography, climate, natural resources, infrastructure, disease ecology, and world affordances. The ground layer from which everything grows.",
    "source": "tome_and_lore.md"
  },
  "axes": [
    { "slug": "...", "name": "...", "..." : "..." }
  ],
  "_deferred": [],
  "_meta": {
    "pass": 2,
    "last_updated": "2026-03-26",
    "axis_count": 10,
    "_commentary": "Pass 2 added infrastructure-quality and disease-ecology.",
    "_suggestions": ["Explore carrying-capacity as synthetic axis"]
  }
}
```

### Six Domains

| Slug | Name | Scope |
|------|------|-------|
| `material-conditions` | Material Conditions | Geography, climate, resources, infrastructure, disease ecology, world affordances |
| `economic-forms` | Economic Forms | Production, trade, labor, debt, currency, land tenure |
| `political-structures` | Political Structures | Formal power, authority legitimation, law, enforcement, institutions |
| `social-forms` | Social Forms of Production and Reproduction | Kinship, gender, class, marriage, inheritance, education, religion |
| `history-as-force` | History as Active Force | How the past lives in the present — memory depth, trauma transmission, legacy visibility |
| `aesthetic-cultural-forms` | Aesthetic and Cultural Forms | Music, food, architecture, clothing, speech, ritual, artistic traditions |

## Elicitation Workflow

### Approach: Breadth-First Passes

Three passes across all six domains, each with a validation gate:

**Pass 1 — Skeleton** (4-6 axes per domain)
- Seed from foundation docs (`tome_and_lore.md`, `world_design.md`)
- Each axis: slug, name, domain, description, axis_type, provisional values
- Surfacing argument: rough — agents and pathways, prose can be thin
- Provenance: `seed` for all
- Gate: every domain has at least 4 axes, no obvious gaps from foundation docs

**Pass 2 — Elaboration** (push toward 8-12 per domain)
- For each domain: "What other axes describe this space? What's missing?"
- Fill in surfacing arguments fully (agents, stages, pathways, relevance, prose)
- Refine axis types and values
- `_commentary` and `_suggestions` captured per axis
- Provenance: `elaborated` for new axes, `seed` updated with refined values
- Gate: every axis has a complete surfacing argument with relevance conditions

**Pass 3 — Coherence Review**
- Genre-region coverage test (see Validation)
- Cross-domain overlap check
- Provenance audit
- Commentary extraction: what patterns emerged across domains?
- Gate: three test worlds feel describable

Between passes, current state is written to domain files so artifacts are always
recoverable. If early passes reveal that the framework needs significant restructuring,
we restart from the skeleton rather than carrying forward stale axes.

### Elicitation Method

Axes are discovered collaboratively in this conversation (Claude as analytical
partner, user as domain expert and design authority). This is analytical/sociological
work requiring depth in anthropology, political economy, and material culture —
not pattern-matching against genre conventions.

Ollama-based models may be used later for stress-testing coverage: "Given these
axes, describe a [specific world]. What can't you express?" This is validation,
not discovery.

If the breadth-first passes stabilize the axis space and approach, individual
domains may be explored via anchored expansion (following mutual production chains
from material conditions outward) to test the boundaries of the modeling approach.

## Validation

### 1. Surfacing Filter (per-axis, during elicitation)

Every axis must demonstrate a gameplay path: which agent, at what stage, through
what pathway, under what conditions. Axes that can't demonstrate a path within
two hops are moved to `_deferred` with an explanation.

### 2. Genre-Region Coverage Test (Pass 3)

Three test worlds stress different parts of the axis space:

- **Folk-horror village** — dense material conditions, strong history-as-force,
  tight social forms, thin economic complexity
- **Cyberpunk megacity** — heavy economic/political/infrastructure, weak history
  visibility, fragmented social forms
- **Epic-fantasy kingdom** — full spread across all domains, high aesthetic-cultural
  weight

For each: sketch the world's position on every axis. Note irrelevant axes (possible
over-specification) and gaps (framework too narrow).

### 3. Axis Count and Distribution (Pass 3)

- Target: 8-12 axes per domain, ~50-72 total
- Domain with fewer than 6 after elaboration: under-specified or merge candidate
- Domain exceeding 15: needs decomposition or axes too granular
- Cross-domain distribution roughly even — significant imbalance suggests bias

## Artifacts

| Artifact | Location | Contents |
|----------|----------|----------|
| Domain files (6) | `storyteller-data/narrative-data/tome/domains/*.json` | Axis definitions with surfacing, commentary, deferred |
| This spec | `docs/superpowers/specs/2026-03-26-tome-phase-1-seed-axes-design.md` | Schema, workflow, validation |

## Key Inputs

- `docs/foundation/tome_and_lore.md` — six domains, mutual production principle
- `docs/foundation/world_design.md` — coherence model, five ingress points, communicability
- `docs/technical/storykeeper-context-assembly.md` — how data surfaces to gameplay
- `docs/superpowers/specs/2026-03-25-tome-data-design-minimal-viable-world.md` — parent spec
- Bedrock genre dimensions (especially `magic`, `technology`, `supernatural`, `violence`, `death`) — world affordance axis counterparts
