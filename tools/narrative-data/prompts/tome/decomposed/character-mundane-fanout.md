You are generating a single mundane character for a narrative world. Mundane characters are
the texture of a world — the people who make it feel inhabited rather than staged. They are
not protagonists. They are not designed to be interesting. They are present, necessary, and
real within the social fabric.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## Centrality Level

{centrality}

Centrality levels:
- `Q1`: Peripheral — present and functional, background figures. A name, a role, a place.
  Do not over-characterize. Q1 characters are defined by what they do, not who they are.
- `Q2`: Mid-level — regular roles with some agency within their domain. Light personality
  sketch with one specific quirk or habit that makes them recognizable. No backstory arc.

## Cluster Membership

This character belongs to: {cluster_name} (slug: {cluster_slug})

Their role, habits, and associations should reflect what membership in this cluster actually
means — not generic social roles pasted onto the cluster.

## World Position (Axis Subset)

{axes_subset}

## What You Are Generating

ONE character at the {centrality} level. Ground them in the material conditions of the world
and the social logic of their cluster. Do not generate a character who could live in any
world — generate a character who could only exist in this one.

The `place_associations` array should contain place slugs where this character regularly
appears or has significance. Q1: one place is sufficient. Q2: one or two.

For Q1: description is 1 sentence — role and presence only.
For Q2: description is 2 sentences — role plus one specific detail (a habit, a peculiarity,
a recognizable behavior) that makes them a person rather than a function.

## Output

Output valid JSON only. No preamble, no explanation, no markdown fences.

{
  "centrality": "{centrality}",
  "slug": "<kebab-case identifier>",
  "name": "<character name>",
  "role": "<what they do — occupation, function, or social position>",
  "cluster_membership": "{cluster_slug}",
  "description": "<1-2 sentences per level guidance above>",
  "place_associations": ["<place-slug>"]
}

Output the JSON object only.
