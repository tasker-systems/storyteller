You are generating a single significant character skeleton for a narrative world. Significant
characters are people scenes will orbit — figures with enough social entanglement, enough
contradictions, enough position-specific pressure that stories naturally accumulate around
them.

The key word is skeleton. You are not writing a character. You are defining the structural
position that a character occupies — the crossing point of two social clusters, the person
who must navigate contradictions that most people avoid. The archetype emerges from the
boundary position. It is not assigned from a catalog.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## Centrality Level

{centrality}

Centrality levels:
- `Q3`: High centrality — positioned where major social tensions cross. Their choices
  affect others beyond their immediate cluster. Three to four sentences of characterization.
- `Q4`: Apex centrality — the person this world revolves around. Maximum two per world.
  Their position is load-bearing for the entire social structure. Four to five sentences.
  The tension in their archetype should be irreducible — not a simple hero/villain split.

## Cluster Position

Primary cluster: {primary_cluster}
{primary_cluster_desc}

Boundary cluster: {boundary_cluster}
{boundary_cluster_desc}

This character stands at the boundary between these two clusters. They are not simply a
member of one who visits the other. They are constitutively shaped by both — which means
they are fully trusted by neither. The boundary tension is what makes them significant.

## World Position (Axis Subset)

{axes_subset}

## Archetype Guidance

{archetypes_summary}

The archetype fields are not assignments from this list. They are observations about what
emerges when a person of this character's structural position navigates the tensions of
this world's material conditions. Generate:

- `primary`: The role this character performs most visibly — what others see them as
- `shadow`: The role they perform that contradicts the primary — what they fear, suppress,
  or are accused of
- `genre_inflection`: How the genre modulates these — what the genre does to this type of
  person, not what type of person the genre usually produces

## What You Are Generating

ONE significant character skeleton at {centrality} level. The description should read as
structural analysis, not biography. Explain the tensions their position creates. Explain
what they want that their position makes difficult. Explain what they cost others by
existing.

Ascending centrality means ascending entanglement. Q4 characters are not free agents — they
are the most constrained people in the world, bound by the weight of everyone who depends
on, fears, or needs them.

The `place_associations` array should contain place slugs that are structurally significant
to this character — places where their position is performed, tested, or revealed.

## Output

Output valid JSON only. No preamble, no explanation, no markdown fences.

{
  "centrality": "{centrality}",
  "slug": "<kebab-case identifier>",
  "name": "<character name>",
  "role": "<structural position — not job title, but what role they perform in the social fabric>",
  "description": "<3-5 sentences per level guidance above — structural, not biographical>",
  "archetype": {
    "primary": "<the visible role>",
    "shadow": "<the contradicting role>",
    "genre_inflection": "<what the genre does to this position>"
  },
  "cluster_membership": {
    "primary": "{primary_cluster}",
    "boundary_with": "{boundary_cluster}",
    "boundary_tension": "<one sentence — what the crossing of these clusters costs this person>"
  },
  "place_associations": ["<place-slug>"]
}

Output the JSON object only.
