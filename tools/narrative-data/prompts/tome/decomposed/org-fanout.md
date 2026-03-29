You are generating a single organization entity for a narrative world. This organization must
emerge from the world's material conditions — its structure, purpose, and reach should be
legible from the axis positions, not borrowed from genre convention.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position (Axis Subset)

{axes_subset}

## Existing Places

{places_summary}

## What You Are Generating

ONE organization. It must reference at least one existing place from the places summary —
organizations exist somewhere, meet somewhere, conduct business somewhere.

Org type options:
- `governance`: Formal authority structures — councils, courts, enforcement bodies
- `economic`: Commerce, production, exchange, resource control
- `religious`: Belief, ritual, moral authority, cosmological interpretation
- `military`: Organized force, security, conflict, defense
- `cultural`: Artistic, intellectual, or identity-maintaining collectives
- `professional`: Trade guilds, skilled-labor associations, credentialing bodies

Tier meanings:
- Tier 1: Major power — visible across the setting, affects most inhabitants
- Tier 2: Significant but bounded — relevant to specific domains or populations
- Tier 3: Minor — local, specialized, or marginal in the broader power structure

The `grounding.axes` array should name the specific axis values that explain why this
organization exists in this form in this world. Do not use genre tropes as explanation.

The `place_associations` array should contain slugs of places from the provided summary.
Include at minimum one association. Do not invent new places.

## Output

Output valid JSON only. No preamble, no explanation, no markdown fences.

{
  "slug": "<kebab-case identifier>",
  "name": "<organization name>",
  "org_type": "<governance|economic|religious|military|cultural|professional>",
  "tier": <1|2|3>,
  "description": "<2-3 sentences grounded in what this world's conditions actually produce>",
  "place_associations": ["<place-slug>"],
  "grounding": {
    "axes": ["<axis:value>", "<axis:value>"]
  }
}

Output the JSON object only.
