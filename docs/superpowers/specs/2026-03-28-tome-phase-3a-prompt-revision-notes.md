# Tome Phase 3a: Prompt Revision Notes

**Date:** 2026-03-28
**Context:** Post-validation review of 4 test worlds (2 folk-horror, 2 cyberpunk) revealed prompt overdetermination from genre settings archetypes. Tome axis positions drive real material differences; genre context overdetermines spatial/organizational *shape*.

---

## Diagnosis

### What's working
- Tome axis positions produce coherent, materially-distinct worlds
- Genre profile (world_affordances, aesthetic, agency) provides correct tonal bias
- Stated/operative gaps are narratively productive across all worlds
- Feed-forward from places → orgs works well

### What's overdetermining
1. **Genre settings archetypes** injected as concrete examples become a checklist. The LLM produces one place per archetype rather than composing from axis positions.
2. **Org prompt asks for 4-6 narratively significant orgs** → every world gets the same functional slots (governance, knowledge-control, labor, religious, military). Three secret cabals is a scheduling conflict.
3. **All entities are "foreground"** — Proper Name Narratively Important Things. No mundane infrastructure to ground the felt reality.

### What's not the problem
- The Tome graph and propagation engine
- The genre bias mechanism (weighted, not gated)
- The output schema (JSON with prose, grounding traces, communicability)
- The active-edges format (Format B) — the LLM uses it well

---

## Revision 1: Soften Settings Archetype Injection

**Current:** Full settings archetypes listed with canonical names, narrative functions, communicability dimensions.

**Revised:** Abstract the settings into spatial *functions* without naming specific archetypes.

Replace `_build_settings_context()` output from:

```
**Genre Setting Archetypes** (from bedrock — use as spatial inspiration):

- **The Ancestral Hearth (The Local Family Home)**
  Function: Externalizes History; Imposes Constraints; Enables Scenes; Generates Events
  Communicability: atmospheric: Inviting, sensory: Community Trust, spatial: Intimate, temporal: Linear but Repetitive
  Atmosphere: Cooking smells, smoke, animals

- **The Earth's Altar (The Ritual Clearing)**
  ...
```

To:

```
**Genre Spatial Functions** (the genre typically requires spaces that serve these narrative roles):

- A space where power is exercised and legitimated (may be formal or informal)
- A space where history/knowledge is stored, curated, or contested
- A space where the boundary between the known and unknown is crossed
- A space where labor and material production happen
- A space where community gathers (willingly or under pressure)
- A space that resists human presence or intention

These functions should emerge from the world's axis positions — not as genre templates but
as structural necessities that the material conditions, political structures, and social
forms of THIS world demand. The specific form each function takes should surprise: the
"space where power is exercised" in a bronze-iron mountain community looks nothing like
the same function in a temperate village. Let the Tome axes determine the shape.

The genre's aesthetic register is: {aesthetic_summary}
The genre's spatial sensibility tends toward: {spatial_summary}
```

**Key change:** The LLM gets functional requirements (the genre needs these *kinds* of spaces) without specific shapes (not "a ritual clearing" but "a space where the boundary between known and unknown is crossed"). The Tome axes fill in the specific form.

---

## Revision 2: Two-Tier Place Elicitation

**Current:** "Generate 6-8 named places" — all foreground, all narratively significant.

**Revised:** Two-tier request in a single prompt, **mundane first**.

```
Generate places for this world in two tiers. Generate Tier 2 first.

**Tier 2 — Mundane Infrastructure (6-8)**
Places that must exist given this world's material conditions. The blacksmith,
the market, the road junction, the communal well, the smallholdings. These are
the practical, everyday substrate of the world — what must be here for people
to live. For each, provide only:
- slug, name, place_type
- one sentence description grounded in axis positions
- 1-2 grounding axes

Generate these first. They establish the material reality of the world.

**Tier 1 — Narratively Significant Places (4-5)**
Places where the story's tensions are concentrated. These must emerge FROM the
mundane substrate — they exist because of, in contrast to, or in tension with
the everyday places above. A sacred grove is meaningful because the surrounding
farms avoid it. An archive is powerful because the market below it operates on
information the archive controls. Each carries full grounding traces,
communicability profiles, and relational seeds, including at least one
relationship to a Tier 2 mundane place.
```

**Key insight (from review):** Inverting the order so mundane comes first means the
narratively significant places must be *situated within and emergent from* the practical
reality, rather than dropped in from genre convention. The LLM establishes the material
substrate, then discovers where narrative tension lives within it.

**Output schema change:** The JSON array contains objects with a `tier` field (1 or 2). Tier 2 objects have a simpler schema — no communicability profile, no active edges, just grounding and one relational seed.

---

## Revision 3: Narrative Weight for Organizations

**Current:** "Generate 4-6 organizations or institutions" — all foreground, all with stated/operative gaps.

**Revised:** Differentiate by narrative weight, **mundane first** (same inversion as places).

```
Generate organizations for this world in two categories. Generate Tier 2 first.

**Tier 2 — Mundane Institutions (3-5)**
Organizations that must exist given the world's political, economic, and social
axis positions. The parish church, the market guild, the village militia. These
function as advertised — they are how the world works. For each, provide:
- slug, name, org_type
- one sentence description
- stated purpose (these institutions do what they say)
- one place association (where they operate)

Generate these first. They establish the institutional reality of the world.

**Tier 1 — Narratively Significant Organizations (1-3)**
Organizations where the stated/operative gap creates playable tension. These
must emerge FROM the institutional landscape above — they exist because the
mundane institutions create the conditions for hidden power. One load-bearing
org with a genuine stated/operative gap is more compelling than three. Reserve
the gap for where the axis positions genuinely produce tension between
appearance and reality.

Each carries the full schema: authority basis, membership, stated_vs_operative,
relational seeds, and at least one relationship to a Tier 2 institution.
```

**Key insight (same as places):** One rotten institution in a functioning society is far
more unsettling than a world where every institution is secretly sinister. The mundane
makes the significant significant.

---

## Revision 4: Anti-Template Instruction

Add an explicit instruction to both prompts:

```
**Anti-template notice:** You have been given genre spatial functions and axis positions.
Do NOT produce a standard set of genre-typical locations. The genre functions describe
what the world *needs*; the Tome axes describe what the world *is*. A folk-horror world
in the mountains has different spatial forms than one in a temperate valley — the
function may be similar (a threshold space) but the specific place should be novel.

If you find yourself generating "a ritual clearing," ask whether THIS world's axis
positions (supernatural-permeability, divine-responsiveness, geography-climate,
population-density) actually produce a clearing, or whether the threshold takes a
different material form here — a mine shaft, a cistern, a ridge.
```

---

## Implementation

These are prompt template revisions + a small change to `_build_settings_context()` in `elicit_places.py`. The output schema needs a `tier` field added. The org prompt needs `narrative_weight` or similar.

Estimated work: 1-2 hours of prompt iteration + one re-run of the 4 test worlds to validate.

---

## Value-Selection Enrichment (replacing edge-bounds proposal)

The repeated-shapes problem has two layers:
1. **Prompt overdetermination** (revisions above) — the genre context is too specific
2. **Propagation determinism** — the graph traversal always follows the same paths, and value selection is flat random within valid values

The original edge-bounds proposal (`[min, max, weight, friction]` per edge) would require re-annotating 1,240 edges. A more elegant solution: enrich the **value selection step** with a lightweight LLM call.

### Current value selection
```
axis reached → random.choice(valid_values)
```
Flat distribution. Two folk-horror worlds with different geography-climate seeds land on the same axis and pick values with equal probability. The graph correctly determines *which axis* to fill next but has no influence on *what value* it gets.

### Proposed value selection
```
axis reached → small LLM call → weighted distribution over valid values
```

For each axis being filled, call qwen2.5:7b-instruct (the fast structuring model, 120s timeout) with:

```
Given this world context:
- Genre: {genre_slug}
- Setting: {setting_slug}
- Already-set positions: {set_positions with values}
- Active incoming edges for {target_axis}: {incoming_edges with descriptions}

The axis "{target_axis}" ({axis_description}) has these valid values:
{valid_values}

Rank these values from most to least plausible for this specific world.
Return a JSON array of {"value": "...", "weight": 0.0-1.0} objects.
```

Then sample from the weighted distribution rather than flat random.

### Why this works
- The genre bias enters naturally — the LLM knows what "folk-horror + mountainous" implies for social-mobility even though the graph edge is a generic weight
- The seed values propagate their specificity — "mountainous + mineral-rich" produces different resource-profile rankings than "temperate + soil-rich"
- 51 calls × ~2-5 seconds each = ~2-4 minutes total (using 7b-instruct, not 35b)
- No need to re-annotate 1,240 edges with bounds/friction

### Implementation
- Add `_select_value_enriched()` to `propagation.py` alongside current `_select_value()`
- New CLI flag: `--enriched` on `compose-world` to enable LLM-assisted value selection
- Default remains random for fast iteration; enriched for quality runs
- Uses `OllamaClient.generate_structured()` with a simple JSON schema

### Relationship to prompt revisions
Both fixes are needed and complement each other:
- Prompt revisions → shape diversity (different material forms for genre functions)
- Value-selection enrichment → position diversity (different axis values from same seeds)
- Together → genuinely orthogonal variation between intra-genre worlds
