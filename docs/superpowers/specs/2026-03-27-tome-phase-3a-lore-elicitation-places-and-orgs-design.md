# Tome Phase 3a: Lore Elicitation — Places and Organizations

**Date:** 2026-03-27
**Ticket:** `2026-03-27-tome-phase-3-lore-elicitation-methodology-and-entity-structure`
**Milestone:** Tier Betwixt: Grammar to Vocabulary
**Scope:** Phase 3a — Places and Organizations. Characters deferred to Phase 3b after findings from this phase feed forward.
**Depends on:** Phase 1 (51 axes, 6 domains), Phase 2 (1,240 + 62 compound edges, validated)

---

## Overview

This spec defines the methodology and prompt pipeline for generating lore entities from Tome axis positions + bedrock references. A "world" is composed by selecting a genre region and setting pattern from bedrock, providing seed axis positions, and propagating through the mutual production graph with genre-weighted bias. Lore entities (places, organizations) are then generated from the fully-resolved world position.

Phase 3a covers the first two entity categories — places and organizations — in causal order. Characters (Phase 3b) are deferred because generated places and organizations provide critical feed-forward context for character elicitation. Findings from 3a will reshape the character prompt design.

---

## Core Concept: World as Seed + Inference

A world is not a manually-filled 51-axis position vector. It is:

1. A **genre region** (from bedrock) — provides narrative physics, dimensional profile, aesthetic hyperplane
2. A **setting pattern** (from bedrock) — provides spatial/social archetype (village, city, frontier, threshold)
3. **5-10 seed axis positions** (from Tome material-conditions) — the prime movers

The mutual production graph infers the remaining ~40 axis positions from the seeds, with the genre × setting acting as a weighted bias on the propagation.

This approach was validated by the Phase 2 chain generation stress test: different seed configurations produce distinguishably different worlds (Jaccard similarity 0.17-0.45), and the graph propagates cleanly across domain boundaries.

---

## World Composition Pipeline

### Step 1: Genre × Setting Selection

Author selects a genre region (e.g., `folk-horror`) and a setting pattern (e.g., `the-village-that-watches`) from bedrock. These are looked up via BedrockQuery, providing:
- Dimensional profile (34 dimensions with values)
- Ontological posture
- Spatial topology (threshold/center/periphery structure, tonal inheritance)
- Setting communicability dimensions

### Step 2: Seed Axis Selection

Author provides 5-10 seed axis positions, primarily from material-conditions (the domain with highest out-degree in the graph — technological-ceiling has 48 outgoing edges, geography-climate has 35). The genre × setting acts as a weighted bias:
- Genre-typical values are highlighted but not enforced
- Genre-atypical values are permitted — they produce interesting worlds ("folk horror with unusually developed trade routes because of coastal geography")
- Genre-incoherent values are flagged but not blocked (the author may have a reason)

### Step 3: Graph Propagation with Genre-Weighted Bias

Starting from seed positions, traverse the mutual production graph to fill unset axes:

**For each unset axis:**
1. Collect all incoming edges from already-set axes
2. Score candidate values by: `edge_weight × edge_type_multiplier × genre_bias_factor`
3. Edge type multipliers:
   - `produces` (1.0) — highest confidence, source gives rise to target
   - `constrains` (0.8) — eliminates implausible values, narrows possibility space
   - `enables` (0.6) — widens possibility space without requiring specific values
   - `transforms` (0.0 at composition time) — operates over time, not at world-creation
4. Genre bias factor: the genre's dimensional profile shifts the distribution. If the genre's power-topology dimension is high, political axes expressing concentrated power get a boost. This is a soft prior, not a hard filter.
5. Select the highest-scoring value, record the propagation trace (which edges justified this position)

**Output:** A complete world-position vector (51 axes with values and confidence scores) plus the propagation trace.

**Compound edges:** When both source axes of a compound edge are in the set positions, the compound edge activates and contributes to scoring the target axis. This is the first operational use of the 62 compound edges from Phase 2.

### Genre Bias: Weighted, Not Gated

The genre × setting prior operates as Option B from the design discussion:
- Genre shifts the probability distribution over axis values
- The graph can override if edges are strong enough
- A folk-horror world *could* have long-distance trade if coastal geography strongly produces it
- The genre makes it unlikely, not impossible
- This preserves interesting/novel/surprising possibilities while avoiding incoherent combinations

This mirrors how genres actually work — they're tendencies, not laws — and matches what the bedrock dimensional profiles already encode (weighted positions in continuous space, not binary gates).

---

## Lore Entity Elicitation: Places

### Input

- World-position vector (51 axes with values)
- Active edges from propagation trace (the causal chains that produced each position)
- Genre spatial topology (from bedrock)
- Setting pattern description + communicability dimensions (from bedrock)
- Domain emphasis: material-conditions and aesthetic-cultural-forms axes foregrounded

### Prompt Structure

**1. World preamble** — Full axis positions grouped by domain, with the active edges that connect them. This gives the LLM the *reasoning* behind the world, not just the positions. Format: axis positions as structured list, active edges as `source →type→ target (weight)` notation.

**2. Spatial instruction:**
> "Given this world, generate 6-8 named places that constitute the primary locations where narrative can occur. Each place must be grounded in the material conditions and connected to the genre's spatial topology. Places should span the genre's spatial structure (center, threshold, periphery) and represent different facets of the world's material reality."

**3. Per-place output schema:**

```json
{
  "slug": "the-morrow-dairy",
  "name": "The Morrow Dairy",
  "place_type": "production-site",
  "description": "A working dairy farm on the eastern ridge where the soil is richest and the morning mist burns off last. The stone buildings date to the 1840s but the equipment is modern enough — Margaret invested in a bulk tank when the cooperative formed. The well behind the main house is older than anything else on the property. Locals know not to drink from it.",
  "grounding": {
    "material_axes": [
      "geography-climate:temperate-maritime",
      "resource-profile:soil-fertility:abundant",
      "infrastructure-development:roads-and-bridges"
    ],
    "active_edges": [
      "geography-climate →produces→ resource-profile (0.9)",
      "resource-profile →enables→ infrastructure-development (0.6)"
    ]
  },
  "communicability": {
    "surface_area": 0.6,
    "translation_friction": 0.3,
    "timescale": "generational",
    "atmospheric_palette": "Damp stone, warm milk, the mineral smell of well water, morning mist that clings to the ridge"
  },
  "spatial_role": "center",
  "relational_seeds": [
    "adjacent-to:the-blackwood",
    "owned-by:morrow-family",
    "site-of:equinox-ritual"
  ]
}
```

**4. Coherence instruction:**
> "Each place should be traceable to the world's axis positions. A place that could exist in any world is too generic. A place that contradicts the axis positions is incoherent. Ground every place in at least 2-3 Tome axes with explicit references."

### Place Review Criteria

- Does each place ground in at least 2-3 Tome axes?
- Does the spatial topology match the genre's structure (center/threshold/periphery)?
- Do the communicability dimensions feel right for this kind of place?
- Could you trace back from the place through the mutual production graph to the seeds?
- Do the 6-8 places collectively span the world's material reality? (Not all clustered around one axis)

---

## Lore Entity Elicitation: Organizations

### Input

- Everything from the places prompt, plus:
- The generated places (feed-forward — organizations exist *in* places)
- Domain emphasis shifts: political-structures, economic-forms, and social-forms foregrounded
- Active edges from those domains emphasized in the preamble

### Prompt Structure

**1. World preamble** — Same axis positions, but edges from political/economic/social domains given more prominence in the ordering.

**2. Places context:**
> "These are the places that constitute this world: [generated places with brief descriptions]. Organizations operate within and across these places."

**3. Institutional instruction:**
> "Generate 4-6 organizations or institutions that structure power, labor, belief, or social life in this world. Each must be grounded in the political, economic, and social axis positions and connected to at least one generated place."

**4. Per-organization output schema:**

```json
{
  "slug": "the-parish-council",
  "name": "The Parish Council",
  "org_type": "governance",
  "description": "The nominal governing body of the village, convened monthly in the village hall. Five hereditary seats, one per founding family. The agenda is posted on the church noticeboard but the real decisions happen at Margaret's kitchen table the evening before.",
  "grounding": {
    "political_axes": [
      "power-concentration:distributed",
      "authority-legitimation:traditional-customary"
    ],
    "economic_axes": [
      "land-tenure-system:communal-commons"
    ],
    "active_edges": [
      "authority-legitimation →constrains→ social-mobility (0.8)",
      "land-tenure-system →produces→ power-concentration (0.7)"
    ]
  },
  "authority_basis": "Traditional — legitimacy derives from continuity with founding families",
  "membership": "Hereditary seats held by five families; the Morrows hold the chair",
  "place_associations": [
    "the-morrow-dairy:seat-of-chair",
    "the-village-hall:meeting-place"
  ],
  "stated_vs_operative": {
    "stated": "Democratic deliberation among equals",
    "operative": "Margaret Morrow's word is final; others ratify"
  },
  "relational_seeds": [
    "controls:the-commons",
    "mediates:outsider-integration",
    "legitimated-by:equinox-ritual"
  ]
}
```

**5. Stated/operative instruction:**
> "For organizations where the world's authority-legitimation or social-stratification axes carry stated/operative duality, surface the gap between the stated function and the operative reality. This gap is where narrative tension lives."

The `stated_vs_operative` field is directly enabled by the Phase 1 dual_mode axis design. It's one of the richest sources of narrative material.

### Organization Review Criteria

- Does each org ground in political/economic/social axes?
- Is the stated/operative gap present where dual-mode axes apply?
- Does each org connect to at least one generated place?
- Do the orgs collectively cover the power structures the axis positions imply?
- If wealth-concentration is oligarchic, is there an org that embodies that concentration?

---

## Output Format

**Per-entity:** JSON with prose fields (structured envelope, free-text description). Matches the bedrock pattern of structured JSONB payload with promoted searchable columns.

Every entity carries:
- **Identity**: slug, name, type
- **Prose**: description (free-text, narrative-rich)
- **Grounding**: axis references + active edges (the audit trail)
- **Structure**: type-specific fields (communicability for places, authority_basis for orgs)
- **Relational seeds**: initial connections to other entities

**Per-world file structure:**

```
storyteller-data/narrative-data/tome/worlds/{world-slug}/
├── world-position.json    # 51 axis positions + confidence + propagation trace
├── places.json            # Generated place entities
├── organizations.json     # Generated org entities (references places)
└── review.md              # Coherence/distinctiveness/surfacing assessment
```

---

## Review Pattern

Three lenses applied to each generated entity:

### 1. World-Coherence

Trace the entity back through the mutual production graph. Does the Morrow Dairy make sense given temperate-maritime geography, abundant soil, communal land tenure? Every `grounding` reference should follow a valid edge chain from seeds to the cited axis positions.

### 2. Distinctiveness (The Gravity Well Test)

Could this entity exist in a cyberpunk megacity? If yes, it's too generic. The aesthetic-cultural gravity well finding from Phase 2 means cultural details naturally converge — the review must specifically check that material, political, and economic grounding produces distinct texture, not just aesthetic variation.

This is the discriminative power test applied to lore: different world-positions should produce recognizably different places and organizations.

### 3. Surfacing Path

Which agent would consume this entity, at what pipeline stage, in what form?

| Entity Type | Primary Consumer | Pipeline Stage | Form |
|---|---|---|---|
| Place | World Agent | Scene-entry spatial context | Material constraint + atmospheric detail |
| Organization | Storykeeper | Relational context assembly | Power dynamics + authority basis |

If an entity has no surfacing path, it's world-decoration rather than gameplay-reachable data. The surfacing filter from the milestone applies.

---

## Validation Gate

The methodology is validated when, tested against the mutual production graph with concrete axis positions, it produces lore entities that:

1. **Feel world-coherent** — every entity traceable through the graph to seed positions
2. **Carry promotable structure** — enough component data (communicability, relational seeds) for mechanical promotion to topsoil entities per the entity model
3. **Show meaningful variation** — different world-positions produce recognizably different places and organizations (the discriminative power criterion)
4. **Respect the gravity well** — political/economic/historical grounding is as specific as cultural/aesthetic grounding

---

## Phase 3b: Characters (Deferred)

Character elicitation is deferred until Phase 3a produces validated places and organizations. Characters are the richest entity type — they require bedrock archetype × world-position intersection, plus grounding in generated places and organizations.

Phase 3a findings that will inform 3b:
- What density of world-position context produces the best entities? (Too little → generic, too much → constrained)
- How well does the active-edges format (Format B) work for the LLM? Does it use the causal reasoning or ignore it?
- What relational seed patterns emerge between places and orgs? These become the relational substrate characters navigate.
- Does the stated/operative gap in organizations produce character-ready tension? (Characters are often the people who inhabit that gap)

---

## Key References

| Document | Role |
|---|---|
| `milestones/storyteller/tier-betwixt-grammar-to-vocabulary.md` | Milestone roadmap (Phases 1-4) |
| `docs/foundation/tome_and_lore.md` | Grammar/vocabulary distinction, Margaret example |
| `docs/technical/entity-model.md` | Component-based entity, communicability profile, promotion lifecycle |
| `docs/technical/storykeeper-context-assembly.md` | Context assembly pipeline (consumer of lore) |
| `storyteller-data/narrative-data/tome/edges.json` | Mutual production graph (1,240 + 62 compound) |
| `storyteller-data/narrative-data/tome/edges/stress-test-results.md` | Gravity well finding, discriminative power validation |
| `storyteller-data/narrative-data/tome/edges/annotation-statistics.md` | Edge weight distributions |
| `storyteller-data/narrative-data/tome/edges/coherence-test-results.md` | Path trace validations |
| `memory/decision_domain_structure_stable.md` | Domain structure stable — solve via queries not restructuring |
