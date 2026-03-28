You are generating the people who inhabit a narrative world. The world has been composed
from a mutual production graph, and places, organizations, and social substrate have
already been generated.

Your task is to produce background and community characters — the people who make this
world feel inhabited. These are not heroes or villains. They are the mail carrier, the
smallholder, the guard, the scribe. They belong to social clusters, they work in places,
they have neighbors. Their ordinariness is what makes the extraordinary legible.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{world_preamble}

## Genre Profile

{genre_profile_summary}

## Places Context

{places_context}

## Organizations Context

{orgs_context}

## Social Substrate

These are the social clusters — lineages, factions, kinship groups — that people in
this world belong to. Every character you generate must belong to one of these clusters.

{social_substrate_context}

## Anti-Template Instruction

Do not generate characters to represent archetypes. Generate people who live in this world,
do this work, and belong to these groups. A mail carrier in a clan-tribal village with
caste-hereditary stratification is a different person than a mail carrier in a chosen-elective
community with meritocratic credentialing.

If you find yourself generating a character because this genre typically has one (the wise
elder, the suspicious stranger, the troubled youth), stop and ask: does THIS world at THIS
set of coordinates, with THESE social clusters and THESE organizations, produce this person?
What work do they do? Who do they answer to? What cluster were they born into?

## Task

Generate characters in two blocks. Generate Q1 FIRST.

### Q1 — Background Characters (4-6)

People who exist because the world requires their labor and presence. Each must have:
- A name (grounded in their cluster — use naming patterns consistent with the world)
- A role (their actual job or function)
- One sentence describing who they are in this world
- One place association (where they work or live)
- One cluster membership (which social group they belong to)
- One relational seed (one directional relationship to another entity)

No archetype. No tension. No communicability profile. These are people doing jobs.

### Q2 — Community Characters (3-4)

People who are slightly more visible in the community — they occupy positions with some
social weight, have a few relationships, and carry one tension or desire that arises
from their position in the world.

Each must have:
- A name and role
- 2-3 sentences of description, specific to this world's material conditions
- Archetype resonance — name the genre archetype this character most resembles, but do
  not force the fit. This is a soft echo, not a mapping.
- 1-2 place associations
- One cluster membership
- 2-3 relational seeds (directional relationships to other entities — places, orgs, Q1 characters)
- One tension or desire that arises from their position in the world, NOT from genre convention

## Output Schema

Output valid JSON: a single array containing ALL characters (Q1 first, then Q2).
No commentary outside the JSON.

**Q1 character object:**

```json
{
  "centrality": "Q1",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their job or function",
  "description": "One sentence. Grounded in a place and a function.",
  "place_association": "place-slug",
  "cluster_membership": "cluster-slug",
  "relational_seed": "relation:target-slug"
}
```

**Q2 character object:**

```json
{
  "centrality": "Q2",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their job or function",
  "description": "2-3 sentences. Specific to this world's material conditions.",
  "archetype_resonance": "The Archetype Name",
  "place_associations": ["place-slug", "place-slug"],
  "cluster_membership": "cluster-slug",
  "relational_seeds": [
    "relation:target-slug",
    "relation:target-slug"
  ],
  "tension": "One sentence. Arises from world-position, not genre convention."
}
```

Field notes:
- `relational_seed` / `relational_seeds`: Directional relationships using entity slugs,
  e.g. "delivers-to:parish-council", "works-at:the-market", "neighbors-with:elda-the-carrier"
- `archetype_resonance`: Name the bedrock archetype this character most resembles. If no
  archetype fits naturally, write "none" — do not force it.
- `cluster_membership`: Must reference a slug from the social substrate.
- Q2 characters should reference at least one Q1 character in their relational_seeds
  when it makes sense (they share a workplace, are neighbors, have a transactional relationship).

Output the JSON array only. No preamble, no explanation, no markdown fences.
