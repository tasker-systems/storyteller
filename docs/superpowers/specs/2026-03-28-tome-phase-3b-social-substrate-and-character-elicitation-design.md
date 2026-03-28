# Tome Phase 3b: Social Substrate and Character Elicitation — Design Specification

**Date:** 2026-03-28
**Ticket:** 2026-03-28-tome-phase-3b-social-substrate-and-character-elicitation
**Branch:** jcoletaylor/tome-data-design-minimal-viable-world
**Status:** Design approved, pending implementation plan

## Thesis

Characters inhabit a world — they do not merely appear in it. A character's agency is defined by, enabled by, and constrained by the socio-historical, political-economic, network-relational, and kinship-personhood axes and vectors that constitute their subjectivity. The Tome axes are not backdrop; they are the material out of which personhood is formed.

Phase 3b realizes this by adding two elicitation layers to the world composition pipeline: a **social substrate** (the named kinship groups, factions, and lineages that people are born into, marry across, and escape from) and **characters** stratified across a narrative centrality gradient that simultaneously deepens characterization and entangles characters in the social web.

## Design Principles

### 1. Entanglement Is the Price of Agency

Narrative centrality correlates with social entanglement. Ascending the centrality gradient simultaneously increases a character's capacity to act and the constraints on their action. Relational density is not decoration — it is the mechanism by which the social substrate exerts force on individual agency. Margaret Morrow can move the council because she is a Morrow, a matriarch, a chair, a grandmother — but every one of those bonds is a vector that others can pull on, and a loyalty she cannot simply discard. Q1 Elda the mail carrier is freer to walk away precisely because nobody depends on her staying.

### 2. Stated/Operative Gap as Structural Consequence

The stated/operative gap at Q3-Q4 is not a character flaw — it is a structural consequence of position. The more deeply woven into the social network, the wider the gap between what a character says they are doing and what the network requires them to do.

### 3. Mundane-First (Carried Forward)

Same ordering principle as Phase 3a: generate Q1-Q2 characters first to establish the demographic reality, then Q3-Q4 characters emerge from that substrate. The ordinary makes the extraordinary legible.

### 4. Boundaries, Not Centers

Narratively productive characters live at the edges of social clusters — the cross-cutting loyalties, the escaped identities, the marriages that bridge factions. The social substrate prompt generates groups, but the character prompts target the seams between them.

### 5. Genre Functions, Not Genre Templates (Carried Forward)

Archetypes are structural patterns a genre produces, not a menu to fill. Anti-template and anti-checklist instructions are load-bearing in every prompt.

## Pipeline Architecture

### Extended Composition Flow

```
Seeds → World Position → Places → Organizations → Social Substrate → Characters (Q1-Q2) → Characters (Q3-Q4)
```

### New Modules

Three new Python modules in `tools/narrative-data/src/narrative_data/tome/`, following the existing `elicit_places.py` / `elicit_orgs.py` pattern:

| Module | Input | Output | CLI Command |
|--------|-------|--------|-------------|
| `elicit_social_substrate.py` | world-position + places + orgs | `social-substrate.json` | `tome elicit-social-substrate --world-slug <slug>` |
| `elicit_characters_mundane.py` | world-position + places + orgs + social substrate | `characters-mundane.json` | `tome elicit-characters-mundane --world-slug <slug>` |
| `elicit_characters_significant.py` | all prior context + mundane characters + bedrock archetypes | `characters-significant.json` | `tome elicit-characters-significant --world-slug <slug>` |

### New Prompt Templates

Three new templates in `tools/narrative-data/prompts/tome/`:

- `social-substrate-elicitation.md`
- `character-mundane-elicitation.md`
- `character-significant-elicitation.md`

### Feed-Forward Data Flow

Each step reads its predecessors from the world directory (`storyteller-data/narrative-data/tome/worlds/<slug>/`):

```
elicit-social-substrate reads:
  └── world-position.json, places.json, organizations.json

elicit-characters-mundane reads:
  └── world-position.json, places.json, organizations.json, social-substrate.json

elicit-characters-significant reads:
  └── world-position.json, places.json, organizations.json, social-substrate.json,
      characters-mundane.json, bedrock archetypes (from discovery corpus)
```

No new infrastructure. Each module follows the existing pattern: load context files, render prompt template, call LLM, parse structured JSON response, write output file.

## Social Substrate Schema

### `social-substrate.json`

A flat list of clusters (not tiered — unlike places and orgs, every social cluster carries narrative weight; tension lives at the boundaries between them, not in a tier distinction within them). Organizations are formal power (what you join). Social clusters are identity and belonging (what you are).

```json
{
  "clusters": [
    {
      "slug": "the-morrows",
      "name": "The Morrows",
      "basis": "blood",
      "description": "2-3 sentences grounded in axes.",
      "grounding": {
        "social_axes": ["kinship-system:clan-tribal", "social-stratification:caste-hereditary"],
        "economic_axes": ["labor-organization:household-subsistence"],
        "active_edges": ["kinship-system →produces→ social-stratification (0.8)"]
      },
      "hierarchy_position": "established-dominant",
      "org_relationships": ["parish-council:founding-members", "keepers-of-the-rooted-name:excluded"],
      "history": "One sentence — how long established, what they survived, what they claim."
    }
  ],
  "relationships": [
    {
      "cluster_a": "the-morrows",
      "cluster_b": "the-hallodays",
      "type": "intermarriage-with-tension",
      "description": "One sentence describing the boundary.",
      "boundary_tension": "Land inheritance flows through Morrow blood but Halloday labor works it."
    }
  ]
}
```

### Field Constraints

- **`basis`** — One of: `blood`, `occupation`, `belief`, `geography`, `affiliation`. Driven by the `kinship-system` axis value: clan-tribal worlds produce blood-basis clusters, chosen-elective worlds produce affiliation-basis, etc.
- **`hierarchy_position`** — One of: `dominant`, `established`, `marginal`, `outsider`, `contested`. Driven by `social-stratification` (stated/operative) — the stated hierarchy may not match operative position.
- **`org_relationships`** — Directional references to existing orgs from `organizations.json`. Connects the social layer to the institutional layer without duplicating it.
- **`relationships` array** — Pairwise cluster dynamics. The `boundary_tension` field is the narratively productive seam where Q3-Q4 characters will be placed.

### Driving Axes

The social substrate prompt highlights these axes from the world position:

**Primary (social-forms domain):**
- `kinship-system` (categorical) — determines cluster basis
- `social-stratification` (stated/operative dual-mode) — determines hierarchy and its gaps
- `social-mobility` (stated/operative dual-mode) — determines permeability between clusters
- `community-cohesion` (bipolar: atomized ↔ bound) — determines social density
- `outsider-integration-pattern` (categorical) — determines boundary porosity
- `gender-system` (categorical) — shapes role access within clusters

**Secondary (cross-domain):**
- `labor-organization` (set) — economic basis for group formation
- `exchange-and-obligation` (categorical) — power relationships through economic form
- `historical-instrumentalization` (bipolar) — history as in-group/out-group boundary
- `expressive-autonomy` (bipolar) — spaces where autonomous identity groups form

## Character Schemas

### Narrative Centrality as Continuous Gradient

Characters are stratified across a continuous narrative centrality distribution using an IQR model. Similar counts per quartile of the rendered population. Centrality is positional, not categorical — characters can move along the spectrum through play. The elicitation generates an initial distribution.

### Q1 — Background Characters (4-6 per world)

Minimal schema. The world's demographic reality.

```json
{
  "centrality": "Q1",
  "slug": "elda-the-mail-carrier",
  "name": "Elda Farrow",
  "role": "mail carrier",
  "description": "One sentence. Grounded in a place and a function.",
  "place_association": "the-post-road",
  "cluster_membership": "the-farrows",
  "relational_seed": "delivers-to:parish-council"
}
```

No archetype. No tension. No communicability profile. A person doing a job in a place, belonging to a group.

### Q2 — Community Characters (3-4 per world)

Light expansion. One desire or tension, 2-3 relational seeds, archetype resonance (not mapping).

```json
{
  "centrality": "Q2",
  "slug": "gareth-the-water-tithe-collector",
  "name": "Gareth Morrow",
  "role": "water-tithe collector",
  "description": "2-3 sentences. Specific to this world's material conditions.",
  "archetype_resonance": "The Complicit Neighbor",
  "place_associations": ["the-tithe-house", "the-cistern-yard"],
  "cluster_membership": "the-morrows",
  "relational_seeds": [
    "collects-from:the-hallodays",
    "reports-to:water-keepers-guild",
    "neighbors-with:elda-the-mail-carrier"
  ],
  "tension": "Believes the tithe system is fair. Has never questioned why the Hallodays pay more."
}
```

The `archetype_resonance` field is a soft pointer — it says "this character echoes this pattern" without binding them to it. The prompt instructs: "name the archetype this character most resembles, but do not force the fit."

### Q3 — Tension-Bearing Characters (2-3 per world)

Full archetype mapping. Inhabits a stated/operative gap. Lives at a social substrate boundary.

```json
{
  "centrality": "Q3",
  "slug": "sienna-halloday",
  "name": "Sienna Halloday",
  "role": "council clerk",
  "description": "3-4 sentences. Narrative-rich, specific to world position.",
  "archetype": {
    "primary": "The Silenced Oracle",
    "shadow": "The Complicit Neighbor",
    "genre_inflection": "Knows the Keepers suppress knowledge but her Halloday position depends on not saying so."
  },
  "place_associations": ["the-scribes-atrium", "the-halloday-farmstead"],
  "cluster_membership": {
    "primary": "the-hallodays",
    "boundary_with": "the-morrows",
    "boundary_tension": "Married a Morrow cousin. Neither family fully claims her."
  },
  "stated_operative_gap": {
    "stated": "Faithful clerk who records council decisions accurately.",
    "operative": "Quietly omits entries that would expose the Keepers' influence on the council."
  },
  "relational_seeds": [
    "works-under:parish-council",
    "married-into:the-morrows",
    "fears:keepers-of-the-rooted-name",
    "protects:gareth-the-water-tithe-collector"
  ],
  "goals": {
    "arc": "Decide whether to expose what she knows or protect her family's position."
  }
}
```

### Q4 — Scene-Driving Characters (1-2 per world)

Full treatment. Everything Q3 has, plus personality profile, communicability, and multi-scale goals.

```json
{
  "centrality": "Q4",
  "slug": "margaret-morrow",
  "name": "Margaret Morrow",
  "role": "parish council chair",
  "description": "4-6 sentences. The genre expressing itself through a specific person.",
  "archetype": {
    "primary": "The Earnest Warden",
    "shadow": "The Unwilling Vessel",
    "genre_inflection": "Genuine care for the village, but the care itself is the mechanism of control. Her warmth prepares the sacrifice."
  },
  "personality_profile": {
    "warmth": 0.8,
    "authority": 0.9,
    "openness": 0.3,
    "interiority": 0.7,
    "stability": 0.8,
    "agency": 0.9,
    "morality": 0.6
  },
  "place_associations": [
    "the-parish-hall:presides",
    "the-morrow-house:matriarch",
    "the-earth-clearing:attends-reluctantly"
  ],
  "cluster_membership": {
    "primary": "the-morrows",
    "boundary_with": "the-hallodays",
    "boundary_tension": "Her daughter married a Halloday against her wishes. She loves her grandchildren."
  },
  "stated_operative_gap": {
    "stated": "Serves the village as elected chair. Decisions made by consensus.",
    "operative": "The Keepers tell her what consensus must look like. She tells herself this is wisdom."
  },
  "relational_seeds": [
    "controls:parish-council",
    "answers-to:keepers-of-the-rooted-name",
    "mother-of:sienna-halloday",
    "grandmother-of:unnamed-halloday-child",
    "distrusts:the-returned-exile"
  ],
  "goals": {
    "existential": "Keep the village whole. The alternative is unthinkable.",
    "arc": "Manage the growing tension between what the Keepers demand and what her conscience permits.",
    "scene": "Ensure tonight's council meeting ratifies the tithe increase without dissent."
  },
  "communicability": {
    "surface_area": 0.9,
    "translation_friction": 0.4,
    "timescale": "generational",
    "atmospheric_palette": "Warm kitchen light, firm handshake, the smell of bread, a locked drawer."
  }
}
```

### Schema Gradient Summary

| Field | Q1 | Q2 | Q3 | Q4 |
|-------|----|----|----|----|
| description | 1 sentence | 2-3 sentences | 3-4 sentences | 4-6 sentences |
| archetype | — | resonance (string) | primary + shadow + inflection | primary + shadow + inflection |
| personality_profile | — | — | — | 7-axis numeric |
| cluster_membership | string | string | structured (boundary) | structured (boundary) |
| stated/operative gap | — | — | yes | yes |
| goals | — | — | arc | existential + arc + scene |
| communicability | — | — | — | full profile |
| relational_seeds | 1 | 2-3 | 4+ | 5+ |

## Prompt Design

### Social Substrate Prompt (`social-substrate-elicitation.md`)

**Structure:**

1. **World Identity** — genre_slug, setting_slug
2. **World Position Preamble** — full axis positions with social-forms and economic-forms axes highlighted (kinship-system, social-stratification stated/operative, community-cohesion, outsider-integration-pattern, labor-organization, exchange-and-obligation)
3. **Places + Orgs Context** — loaded from existing files, summarized
4. **Genre Profile** — aesthetic/tonal signals from `region.json`
5. **Anti-Template Instruction** — "Do not produce genre-typical factions. Let the axes determine what social groups this world must produce. A clan-tribal kinship system with caste-hereditary stratification produces different clusters than a chosen-elective system with meritocratic stratification."
6. **Task** — Generate 3-5 social clusters as a flat list. Every person in this world belongs to one. Then generate pairwise relationships for each cluster pair, with explicit boundary tensions.

### Mundane Character Prompt (`character-mundane-elicitation.md`)

**Structure:**

1. **World Identity** — genre_slug, setting_slug
2. **World Position Preamble** — full axis positions (social-forms and economic-forms highlighted)
3. **Places Context** — loaded from `places.json`, summarized
4. **Organizations Context** — loaded from `organizations.json`, summarized
5. **Social Substrate Context** — loaded from `social-substrate.json`: cluster names, bases, hierarchy positions, pairwise relationships
6. **Genre Profile** — aesthetic/tonal signals
7. **Anti-Template Instruction** — "Do not generate characters to represent archetypes. Generate people who live in this world, do this work, and belong to these groups. A mail carrier in a clan-tribal village with caste-hereditary stratification is a different person than a mail carrier in a chosen-elective community with meritocratic credentialing."
8. **Task** — Generate in two blocks:
   - **Q1 (4-6):** Background characters. Each must have a place association, a cluster membership, and one relational seed. No archetype. One sentence.
   - **Q2 (3-4):** Community characters. Each gets archetype resonance (name the bedrock archetype they most echo, do not force the fit), 2-3 relational seeds, and one tension or desire. The tension should arise from their position in the world, not from genre convention.

**No bedrock archetype data injected.** The prompt names the archetype resonance field but does not provide the archetype catalog — Q1-Q2 characters should emerge from world conditions. The LLM's training-data knowledge of genre archetypes is sufficient for a soft resonance label.

### Significant Character Prompt (`character-significant-elicitation.md`)

**Structure:**

1. **World Identity** — genre_slug, setting_slug
2. **World Position Preamble** — full axis positions, stated/operative duals highlighted
3. **Places + Orgs Context** — summarized
4. **Social Substrate Context** — full clusters + relationships + boundary tensions
5. **Mundane Characters Context** — loaded from `characters-mundane.json`: Q1-Q2 characters with their cluster memberships and relational seeds. These are the people Q3-Q4 characters protect, exploit, depend on, or betray.
6. **Bedrock Archetypes** — all genre archetypes injected with personality profiles, distinguishing tensions, and structural necessities. Anti-checklist instruction: "These are structural patterns this genre produces. Use them as lenses, not as a menu. A character may resonate with one archetype's tension while occupying another's structural position."
7. **Archetype Dynamics** — pairwise pairing data (e.g., "The Warmth That Prepares the Sacrifice" for Vessel + Warden). Provides relational texture for how archetypes interact.
8. **Genre Profile** — aesthetic/tonal signals
9. **Anti-Template Instruction** — "Each character must be situated at a specific social substrate boundary. Their archetype is how they cope with that boundary position, not a personality assigned from a catalog."
10. **Task** — Generate in two blocks:
    - **Q3 (2-3):** Tension-bearing. Full archetype (primary + shadow + genre inflection). Must inhabit a stated/operative gap. Must sit at a named cluster boundary from the social substrate. Arc-scale goal.
    - **Q4 (1-2):** Scene-driving. Everything Q3 has, plus personality profile (7-axis), communicability profile, multi-scale goals (existential/arc/scene). The genre expressing itself through a specific person in specific material conditions at a specific social position.

**Key prompt instruction for Q3-Q4:** "Each character's relational seeds must reference at least one Q1-Q2 character by slug. The tension-bearing and scene-driving characters do not float above the world — they are embedded in it."

### Seam for Splitting Q3/Q4

The significant character prompt generates Q3 and Q4 in a single call. If testing reveals that the combined prompt produces compressed or checklist-y output, the template is designed to split cleanly: the Q3 block becomes its own prompt (with all the same context), and the Q4 prompt adds Q3 characters as additional feed-forward context. The module accepts a `--split-significant` flag that switches between single-call and two-call modes. This is a contingency — we build the single-call path first and only implement the split if testing reveals quality issues.

## Archetype Integration

### Archetype × Genre Lens Gradient

| Centrality | Archetype Treatment |
|------------|-------------------|
| Q1 | No archetype — characters are functions of the world |
| Q2 | Archetype resonance — echoes a pattern, not mapped to one |
| Q3 | Full archetype mapping (primary + shadow), inflected through world-position + social boundary |
| Q4 | Archetype × world-position × social position × group loyalties — the genre expressing itself through a specific person in specific material conditions |

### Bedrock Data Used

For each genre, the significant character prompt injects:

- **All genre archetypes** (folk-horror: 6, cyberpunk: 7) with personality profiles, distinguishing tensions, structural necessities
- **Archetype dynamics** (folk-horror: 7 pairings, cyberpunk: 6 pairings) with characteristic scenes and shadow pairings

The full set is injected rather than pre-selected to avoid the determinism problem solved in Phase 3a. The LLM sees all options and selects based on the full world context.

## Validation Plan

Run the full extended pipeline against all 4 existing test worlds:

- **McCallister's Barn** (folk-horror, temperate village)
- **The Windswept Crags** (folk-horror, mountain mining)
- **Neon Depths** (cyberpunk, compressed megacity)
- **Data Ghost** (cyberpunk, island archipelago)

### Validation Criteria

1. **Social substrate differentiation** — Do social substrates differ between intra-genre variants? Different kinship structures from different Tome positions (e.g., blood-basis in temperate village vs. occupation-basis in mining community).
2. **Boundary placement** — Do Q3-Q4 characters inhabit cluster boundaries? Not generic archetype instantiation but specific people at specific social seams.
3. **Archetype divergence** — Does the same archetype produce recognizably different characters in different worlds? The Earnest Warden in McCallister's Barn should be a materially different person than the Earnest Warden (or equivalent) in The Windswept Crags.
4. **Centrality gradient** — Does the distribution feel right? Q1 grounds reality, Q2 populates community, Q3 carries tension, Q4 drives story. The gradient should feel like zooming in, not switching categories.
5. **Relational coherence** — Do Q3-Q4 relational seeds reference Q1-Q2 characters? Is the web connected, not floating?
6. **Mundane-first effectiveness** — Do Q1-Q2 characters establish a demographic reality that makes Q3-Q4 characters legible by contrast?

## Output File Structure

After full pipeline execution, each world directory contains:

```
storyteller-data/narrative-data/tome/worlds/<slug>/
├── world-position.json          (existing — Phase 3a)
├── places.json                  (existing — Phase 3a)
├── organizations.json           (existing — Phase 3a)
├── social-substrate.json        (new — Phase 3b)
├── characters-mundane.json      (new — Phase 3b)
└── characters-significant.json  (new — Phase 3b)
```
