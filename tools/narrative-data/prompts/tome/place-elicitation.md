You are generating named places for a narrative world composed from a mutual production graph.

Your task is to produce a set of primary locations where narrative can occur in this world.
These places must be grounded in the world's actual axis positions — not generic fantasy locations,
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

## Task

Generate 6-8 named places that constitute the primary locations where narrative can occur
in this world.

Each place must be:
- **Grounded in material conditions**: anchored in 2-3 Tome axes with specific axis values
- **Connected to genre spatial topology**: assigned a spatial role (center, threshold, or periphery)
- **Distinct from each other**: no two places should occupy the same dramatic function
- **Communicable**: the Narrator must be able to render this place in sensory terms

## Coherence Requirement

Each place should be traceable to the world's axis positions. A place that could exist in
any world is too generic. Justify each place's existence through at least one seed axis
and one inferred axis. Use the active edges (shown in the world preamble) to explain
how material conditions produce spatial forms.

## Output Schema

Output valid JSON: an array of place objects. No commentary outside the JSON.

Each place object must have exactly this structure:

```json
{
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

Output the JSON array only. No preamble, no explanation, no markdown fences around the outer array.
