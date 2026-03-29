You are doing an editorial review of mundane characters (Q1-Q2) for a narrative world. These
characters were generated independently and may have uneven cluster distribution, inconsistent
naming, duplicate roles, or missing place associations. Your task is a light editorial pass —
adjust and bind, do not reinvent.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Upstream Context

{upstream_context}

## Draft Entities

The following Q1 and Q2 characters were generated independently:

{draft_entities}

## Your Tasks

### 1. Cluster Distribution

Check that Q1 and Q2 characters are distributed across the available social clusters.
If the majority of characters belong to one cluster, reassign some to underrepresented
clusters — make the reassignment plausible given their role and place association.

Every character must have a `cluster_membership` that references a valid cluster slug
from the upstream context. Characters with missing or implausible cluster memberships
should be reassigned.

### 2. Naming Consistency

Names should feel like they come from the same cultural world. If characters have wildly
inconsistent naming conventions (some formal, some colloquial, some transliterated from
another tradition), adjust for coherence. Naming should echo the cluster culture the
character belongs to.

Do not change names that are clearly intentional and specific. Adjust only names that
feel generic or jarring relative to the world's naming patterns.

### 3. Duplicate Role Check

If two characters have effectively the same role (same function, same place, same cluster),
differentiate them or flag one for removal. Every character should occupy a distinct
functional position in the world. Two grain carriers at the same market is redundant unless
their narrative positions differ.

To differentiate: adjust role specificity (not "laborer" but "weights-keeper at the common
store"), assign different place associations, or give them distinct cluster memberships that
put them in productive tension.

### 4. Place Associations

Every Q1 character must have a `place_association` that references a valid place slug from
the upstream context. Every Q2 character must have at least one valid `place_associations`
entry.

If a character has a place association that doesn't exist in the upstream context, replace it
with the closest valid place slug. Do not invent new places.

### 5. Q1/Q2 Distinction

Q1 characters are background — they do jobs, they exist, they are not narrative engines.
Q2 characters are community figures with some social weight, one tension or desire, and
3+ relational seeds.

If a Q1 character has been given a tension, archetype resonance, or communicability profile,
strip those fields — they belong at Q2 or above.

If a Q2 character has only a one-sentence description and one relational seed, enrich them
to Q2 standards (2-3 sentences, archetype_resonance, 2-3 relational seeds, one tension
grounded in world-position).

## Output

Output valid JSON: a complete array of all characters (Q1 first, then Q2). Use the same
schema as the input. Do not add fields not present in the Q1/Q2 schema. Do not add
commentary or explanation.

No preamble, no explanation, no markdown fences. Output the JSON array only.
