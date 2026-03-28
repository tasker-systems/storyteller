You are generating named places for a narrative world composed from a mutual production graph.

Your task is to produce a set of primary locations where narrative can occur in this world.
These places must be grounded in the world's actual axis positions — not generic genre locations,
but places that could only exist in this specific world at this specific set of coordinates.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

The following axis positions define this world's narrative coordinates. Seeds are
author-provided; inferred positions were propagated from seeds via the Tome mutual
production graph, with justification and confidence shown.

{world_preamble}

## Genre Profile

The following signals describe the genre's aesthetic register, spatial affordances,
and narrative sensibility. Use these to ensure each place feels genre-coherent.

{genre_profile_summary}

## Anti-Template Instruction

Do NOT produce a standard set of genre-typical locations. The genre functions below
describe what the world needs; the Tome axes above describe what the world is. These
two things together determine spatial form — not genre convention alone.

If you find yourself generating a location because it sounds like it belongs in this
genre (a ritual clearing, a ruined tower, a neon-lit alley), stop and ask: do the
actual axis positions of THIS world produce that form? Or does the underlying need
take a different material shape here? Let the axes override the template.

## Task

Generate places in two tiers. Generate Tier 2 FIRST — the mundane substrate must
exist before Tier 1 places can emerge from it.

### Tier 2 — Mundane Infrastructure (generate 6-8 places)

Everyday places that must exist given the material conditions of this world. These
are not dramatic — they are the economic, domestic, and logistical fabric that the
world requires. A market, a dwelling district, a road, a source of water or fuel,
a place of work. Ground each in 1-2 Tome axes. Keep descriptions brief (one sentence).

Tier 2 places use a simpler schema (see Output Schema below).

### Tier 1 — Narratively Significant Places (generate 4-5 places)

Places where tensions concentrate, conflicts crystallize, or revelations occur.
These places EMERGE FROM the mundane substrate — they exist because of, adjacent to,
or in tension with the Tier 2 places. Each Tier 1 place must reference at least one
Tier 2 place in its relational_seeds.

Tier 1 places use the full schema with communicability, active edges, and relational seeds.

## Coherence Requirement

Each place should be traceable to the world's axis positions. A place that could exist in
any world is too generic. Justify each place's existence through at least one seed axis
and one inferred axis. Use the active edges (shown in the world preamble) to explain
how material conditions produce spatial forms.

## Output Schema

Output valid JSON: a single array containing ALL places (Tier 2 first, then Tier 1).
No commentary outside the JSON.

**Tier 2 place object** (simpler schema):

```json
{
  "tier": 2,
  "slug": "kebab-case-identifier",
  "name": "Human-readable place name",
  "place_type": "<one of: settlement, production-site, sacred-site, threshold, wilderness, infrastructure, gathering-place>",
  "description": "One sentence. Specific material detail anchored in an axis value.",
  "grounding": {
    "material_axes": ["axis-slug:value", "axis-slug:value"]
  }
}
```

**Tier 1 place object** (full schema):

```json
{
  "tier": 1,
  "slug": "kebab-case-identifier",
  "name": "Human-readable place name",
  "place_type": "<one of: settlement, production-site, sacred-site, threshold, wilderness, infrastructure, gathering-place>",
  "description": "2-4 sentences, narrative-rich, specific material details. Avoid generic atmospheric language — ground each detail in an axis value or active edge.",
  "grounding": {
    "material_axes": ["axis-slug:value", "axis-slug:value"],
    "active_edges": ["source →type→ target (weight)"]
  },
  "communicability": {
    "surface_area": 0.0,
    "translation_friction": 0.0,
    "timescale": "<one of: momentary, biographical, generational, geological, primordial>",
    "atmospheric_palette": "Sensory string: dominant textures, sounds, smells, temperatures, light quality"
  },
  "spatial_role": "<one of: center, threshold, periphery>",
  "relational_seeds": ["relation:target-slug"]
}
```

Field notes:
- `surface_area`: How much of this place is available to narrative interaction (0.0 = sealed/inaccessible, 1.0 = fully open)
- `translation_friction`: How difficult it is for the Narrator to render this place in player terms (0.0 = immediately legible, 1.0 = deeply alien)
- `timescale`: The dominant temporal register of this place (how old does it feel, how fast does it change)
- `relational_seeds`: Directional relationships to other places by slug, e.g. "controls:market-district", "adjacent-to:threshold-gate", "supplies:production-hall"
- For Tier 1, `relational_seeds` must include at least one reference to a Tier 2 slug

Output the JSON array only. No preamble, no explanation, no markdown fences around the outer array.
