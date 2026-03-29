You are generating a single place entity for a narrative world. This place must feel like a
product of the world's material conditions — not a generic genre location dropped in from
outside.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## Place Type to Generate

{place_type}

## Spatial Function

{spatial_function}

## World Position (Axis Subset)

{axes_subset}

## What You Are Generating

ONE place of type `{place_type}`. It should perform the spatial function described above and
be grounded in the material conditions implied by the axis values. Do not invent details that
contradict the axis positions. Do not use genre-typical decoration as a substitute for
material reasoning.

Tier meanings:
- Tier 1: Central, frequently visited, high social or functional significance
- Tier 2: Secondary, known to regular inhabitants, moderate significance
- Tier 3: Peripheral, specialized, known to few, low general significance

The `grounding.material_axes` array should name the specific axis values from the world
position that most directly shaped this place — not generic genre references.

## Output

Output valid JSON only. No preamble, no explanation, no markdown fences.

{
  "slug": "<kebab-case identifier>",
  "name": "<place name>",
  "place_type": "{place_type}",
  "tier": <1|2|3>,
  "description": "<2-3 sentences grounded in material conditions, not genre atmosphere>",
  "spatial_role": "<what function this place performs in the social/economic/political fabric>",
  "grounding": {
    "material_axes": ["<axis:value>", "<axis:value>"]
  }
}

Output the JSON object only.
