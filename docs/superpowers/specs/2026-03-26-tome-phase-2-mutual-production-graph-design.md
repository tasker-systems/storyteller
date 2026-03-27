# Tome Phase 2: Mutual Production Graph — Design Spec

## Context

Second phase of the Tier Betwixt milestone. Phase 1 (complete) produced 51 world-shaping
axes across 6 domains with structured surfacing arguments. The axes are the vertices;
this phase maps the edges — the mutual production relationships that make worlds coherent
rather than arbitrary collections of properties.

**Parent spec**: `docs/superpowers/specs/2026-03-25-tome-data-design-minimal-viable-world.md`
**Phase 1 spec**: `docs/superpowers/specs/2026-03-26-tome-phase-1-seed-axes-design.md`
**Milestone**: `tier-betwixt-grammar-to-vocabulary` (Phase 2 section)
**Ticket**: `2026-03-26-tome-phase-2-mutual-production-graph`

## Goal

Produce a complete mutual production graph mapping relationships between the 51 Tome
axes — what produces, constrains, enables, or transforms what. The graph is a declarative
reasoning substrate for agents, not an enforcement mechanism. It tells the World Agent,
Dramaturge, and Narrator *how axes relate* so they can reason about coherence, not so
the system can reject invalid worlds.

## Decisions

### Edges as Reasoning Context, Not Enforcement

When a world has `geography-climate: arid-desert` and the graph has a `constrains` edge
to `resource-profile` (no fisheries without coast), the system does not enforce or reject.
Instead:

- **At runtime**: The World Agent uses the edge as interpretive context — "fisheries are
  unusual here, so if someone tries to go fishing, I should know this is unexpected."
- **During world composition** (Phase 3/4): The authoring pipeline uses edges as generation
  guidance — synthetic worlds follow the constraints by default. When an author overrides,
  the system surfaces the divergence as a suggestion, not a gate.

This means edge precision matters less than edge coverage. A rough edge that points agents
in the right direction is more valuable than a precise edge that only covers one narrow case.
The `weight` field captures reasoning salience ("how strongly should an agent consider
this"), not enforcement strength.

### Edge Schema

Each pairwise edge:

```json
{
  "from_axis": "geography-climate",
  "from_domain": "material-conditions",
  "to_axis": "resource-profile",
  "to_domain": "material-conditions",
  "edge_type": "constrains",
  "description": "Geographic regime determines which resources can exist — no fisheries without coast, no timber without forests",
  "weight": 0.8,
  "bidirectional": false,
  "provenance": "seed",
  "_commentary": "Strong constraint but not deterministic — irrigation can create arable land in arid regions"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `from_axis` | string | Source axis slug |
| `from_domain` | string | Source domain slug (redundant but enables fast filtering) |
| `to_axis` | string | Target axis slug |
| `to_domain` | string | Target domain slug |
| `edge_type` | enum | `produces`, `constrains`, `enables`, `transforms` |
| `description` | string | Natural language description of the relationship |
| `weight` | float | 0.0-1.0, reasoning salience for agents |
| `bidirectional` | bool | Whether the reverse edge holds with same type and weight |
| `provenance` | enum | `seed`, `systematic`, `discovered` |
| `_commentary` | string | Design notes, exceptions, open questions |

**Edge types** (from milestone spec):

| Type | Meaning | Agent Interpretation |
|------|---------|---------------------|
| `produces` | A gives rise to B | If A is present, B is expected |
| `constrains` | A limits what B can be | Some B values are implausible given A |
| `enables` | A makes B possible (not required) | If A is absent, B needs alternative explanation |
| `transforms` | A changes B's character over time | A's presence shifts B dynamically |

### Compound Edges

Some relationships are not pairwise but compound — (A ∧ B) → C, where neither A→C
nor B→C alone captures the combined effect. A coastal geography with abundant fisheries
produces maritime trade in a way that neither axis predicts independently.

```json
{
  "type": "compound",
  "from_axes": ["geography-climate", "resource-profile"],
  "from_domains": ["material-conditions", "material-conditions"],
  "to_axis": "trade-network-reach",
  "to_domain": "economic-forms",
  "edge_type": "produces",
  "description": "Coastal geography + marine resource abundance produces maritime trade networks; arid geography + mineral abundance produces overland caravan routes. Neither axis alone determines the trade pattern.",
  "weight": 0.7,
  "provenance": "systematic",
  "_commentary": ""
}
```

Compound edges are first-class objects in the graph. They capture local topology
insights that are not inferrable from the individual pairwise edges. They emerge
during curation when we notice that a pair of axes produces effects on a third that
neither captures alone — they are not exhaustively generated.

### Cluster Annotations

Higher-level prose descriptions of how axes within a region of the graph interact as
a system. These capture aggregate insights about local topology that inform Phase 3
prompt design and world-composition guidance but aren't reducible to individual edges
or compound edges.

### File Structure

A single canonical file with three sections:

```json
{
  "edges": [ ...pairwise edges... ],
  "compound_edges": [ ...multi-source edges... ],
  "_cluster_annotations": {
    "material-to-economic": "Geography and resource profile jointly determine the economic possibility space...",
    ...
  }
}
```

Location: `storyteller-data/narrative-data/tome/edges.json`

A single file because:
- Total edge count will be ~80-150 pairwise + ~10-30 compound — manageable in one file
- Cross-domain analysis requires seeing all edges at once
- Phase 3 tooling can `jq`/Python subselect by domain pair as needed for prompt generation
- Rejected pairs preserved separately as `edges-rejected.json` for provenance

## Exhaustive Generation Pipeline

Follows the proven bedrock elicitation pattern: generate structure, LLM-annotate
in chunks, human-curate the annotations, export canonical artifacts.

### Step 1: Generate Combinatorial Structure

A Python script reads all 6 domain files, extracts the 51 axes, and generates:

- **Per-domain-pair markdown files** at `storyteller-data/narrative-data/tome/edges/{source-domain}/{target-domain}.md`
  (21 files: 6 within-domain + 15 cross-domain)
- **`manifest.json`** at `storyteller-data/narrative-data/tome/edges/manifest.json`
  tracking processing state for each chunk

Each markdown file includes:
- Subselected axis descriptions from both relevant domain JSON files (name,
  description, axis_type, values — enough context for the model to reason about
  relationships)
- Edge type definitions with examples
- Compound edge definition with example
- A table of all directed pairs between the two domains, with seed edges
  pre-filled from Phase 1 commentary

The manifest tracks: domain pair, pair count, status (`pending`, `annotated`,
`curated`, `exported`), model used, timestamp, notes.

### Step 2: LLM Annotation (qwen3.5:35b)

For each pending chunk, a prompt template asks qwen3.5:35b to:
- Assess each pair: `produces`, `constrains`, `enables`, `transforms`, or `none`
- Provide a description for each meaningful edge
- Suggest a weight (0.0-1.0)
- Flag any compound edges noticed (pairs of source axes that jointly produce
  effects on the target)
- Include `_commentary` for edges where the relationship is nuanced or conditional

Each chunk is small enough for qwen3.5:35b (~10 source axes × ~10 target axes
= ~80-100 pairs, with both domains' axis descriptions as context = ~3-4k tokens
of input). The model annotates the markdown file in place.

manifest.json updated to `annotated` per chunk.

### Step 3: Human Curation

Review each annotated chunk:
- Correct edge types and descriptions where the model's assessment is wrong
- Adjust weights
- Add compound edges the model missed
- Write cluster annotations per domain-pair section (aggregate insights about
  how axes in this region of the graph interact as a system)
- Flag any discovered axes or axis refinements

manifest.json updated to `curated` per chunk.

### Step 4: Export Curated Edges

An export script reads all 21 curated markdown files and produces:
- `edges.json` — the canonical graph (pairwise + compound + cluster annotations)
- `edges-rejected.json` — pairs assessed as `none` with rationale

manifest.json updated to `exported`.

## Schema Formalization (Side Deliverable)

Phase 1 identified two patterns to formalize during this phase:

### Stated/Operative Pair

4+ axes carry dual values from the same categorical list — the public narrative
and the operative reality. Axes identified: authority-legitimation, social-stratification,
relationship-to-past, social-mobility.

Formalization: add an optional `dual_mode` field to the axis schema:

```json
{
  "dual_mode": {
    "type": "stated_operative",
    "stated": "democratic-consent",
    "operative": "corporate-economic"
  }
}
```

Applied only to categorical axes where the pattern was identified during Phase 1.
Not a universal feature — the gap between stated and operative must be narratively
productive for the axis to carry it.

### Set Type Updates

6+ axes need multiple simultaneous values. Change `axis_type` from `categorical`
to `set` for: production-mode, labor-organization, trauma-transmission-mode,
aesthetic-register, knowledge-system-structure, historical-memory-depth.

Values lists stay the same; a world position selects 1-3 values from the list.
The `set` type already exists in the schema.

## Coherence Testing

Three scenarios validate that the graph produces useful reasoning chains:

### Test 1: McCallister's Barn

Trace why the McCallister field went fallow:
geography-climate(temperate) + resource-profile(soil:moderate) →
production-mode(subsistence-agriculture) → economic-volatility(high) →
exchange-and-obligation(honor-debt, unpaid) → land-tenure-system(mixed-contested) →
legacy-visibility(high, the empty field).

Each step must follow a graph edge. Missing steps reveal missing edges.

### Test 2: Cyberpunk Augmentation Debt

Trace how technology + economics produces the augmentation debt trap:
technological-ceiling(post-digital) + biological-plasticity(augmentable) →
exchange-and-obligation(financialized-credit) → labor-organization(gig-precarious) →
wealth-concentration(extreme) → social-mobility(stated:fluid, operative:frozen).

The graph should explain how technology and economic structure jointly produce the trap.

### Test 3: Implausible World Detection

Position a world at contradictory values: geography-climate(arid-desert) +
resource-profile(fisheries:abundant, timber:dominant) + production-mode(subsistence-agriculture)
+ trade-network-reach(autarkic).

The graph should surface which `constrains` edges are violated. Not to reject the
world but to flag which relationships are unusual and would need explanation. An agent
should arrive at "this combination is unusual because..." — interpretive context,
not enforcement.

### Programmatic Chain Generation (Stress Test)

After the three hand-crafted tests pass, a Python script generates a directed but
extensive subset of technically-valid edge chains (including compound edge interactions)
and renders them as world sketches. This is coherence testing by exhaustive inference:
what worlds does the graph *imply* when you traverse it combinatorially?

The script:
- Starts from varying seed positions on material-conditions axes
- Follows `produces`, `constrains`, and `enables` edges forward through domains
- Generates world-position sketches at varying levels of chain depth (3-hop, 5-hop, full)
- Flags chains that produce contradictions (constrains edges violated by earlier
  positions) or surprising combinations worth investigating

This serves dual purpose:
- **Coherence validation**: implausible or degenerate worlds reveal graph problems
- **Phase 3 input validation**: if edge chains produce interesting, differentiated
  world sketches, the graph will be useful for lore elicitation prompts. If chains
  produce uniform or nonsensical worlds, the graph isn't discriminating enough.

## Artifacts

| Artifact | Location | Contents |
|----------|----------|----------|
| Generation + export scripts | `tools/narrative-data/src/narrative_data/tome/` | Combinatorial generation, prompt template, export |
| Per-domain-pair edge files (21) | `storyteller-data/narrative-data/tome/edges/{domain}/{domain}.md` | Annotated + curated edge tables per domain pair |
| Pipeline manifest | `storyteller-data/narrative-data/tome/edges/manifest.json` | Processing state for each chunk |
| Canonical edge graph | `storyteller-data/narrative-data/tome/edges.json` | Curated pairwise + compound + cluster annotations |
| Rejected pairs | `storyteller-data/narrative-data/tome/edges-rejected.json` | Pairs assessed as `none` with rationale |
| Updated domain files | `storyteller-data/narrative-data/tome/domains/*.json` | Set type + stated/operative updates, any discovered axes |
| This spec | `docs/superpowers/specs/2026-03-26-tome-phase-2-mutual-production-graph-design.md` | Schema, pipeline, validation |

## Key Inputs

- Phase 1 domain files: `storyteller-data/narrative-data/tome/domains/*.json` (51 axes, 42 edge references)
- Milestone Phase 2 section: `milestones/storyteller/tier-betwixt-grammar-to-vocabulary.md`
- Parent design spec: `docs/superpowers/specs/2026-03-25-tome-data-design-minimal-viable-world.md`
- Phase 1 design spec: `docs/superpowers/specs/2026-03-26-tome-phase-1-seed-axes-design.md`
- Phase 1 design insights: history as interpretive meta-domain, sub-community axis divergence, world affordance cluster, environmental justice broadening, Graeber-informed exchange-and-obligation framing
