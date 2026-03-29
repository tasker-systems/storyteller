You are doing the deep enrichment pass on significant characters (Q3-Q4) for a narrative world.
These character skeletons were generated independently and require generative enrichment — not
just editing. Your task is to bind them relationally, ground their gaps in social position,
and give Q4 characters the full characterization depth their centrality demands.

## Design Principle

Ascending centrality simultaneously increases capacity to act and constraints on action.
The more a character can do, the more the world can do to them. Entanglement is the price
of agency.

This is not a metaphor. Q4 characters have more relational seeds, more specific place
associations with role context, more scale of goals — because they are more deeply woven
into the world's fabric. Their power and their vulnerability are the same thing.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Genre Profile

{genre_profile_summary}

## Upstream Context

{upstream_context}

## Social Substrate

{substrate_context}

## Mundane Characters (Q1-Q2)

{mundane_context}

## Bedrock Archetypes

{archetypes_context}

## Archetype Dynamics

{archetype_dynamics_context}

## Draft Entities

The following Q3 and Q4 character skeletons were generated independently:

{draft_entities}

## Your Tasks

### For All Significant Characters (Q3 + Q4)

**Relational seed binding**: Every character's `relational_seeds` must use verb:slug format.
Q3 characters need 4+ seeds. Q4 characters need 5+ seeds. Each character's seeds must
include at least one Q1-Q2 character slug AND at least one organization or social cluster slug.

If seeds are missing, generic (e.g., "knows:unnamed"), or reference entities not in the
upstream context, replace them with specific, directional seeds that reference valid slugs.
The verb matters: `supervises`, `fears`, `married-into`, `owes-debt-to`, `competes-with`,
`enforces-for`, `protects-against`, `depends-on` each carry different narrative weight.
Use the verb that names the actual relationship.

**Stated/operative gap grounding**: The gap between stated and operative reality must be
rooted in structural social position — not personal moral failure. "She claims to be
impartial but is actually corrupt" is a personal flaw. "He claims to adjudicate fairly but
the guild's fee structure means only established members can afford proceedings" is a
structural gap grounded in the world's economic axes. Rewrite gaps that are personal
moralizing as structural consequences of social position.

**Arc goal**: Every Q3 and Q4 character must have an `arc` goal that describes what
changes over the story — not what they want, but what their arc does to them or through them.

### For Q4 Characters Specifically

Q4 characters are the genre expressing itself through a specific person at a specific
social position. They require complete characterization:

**Place associations with role context**: Q4 characters must have `place_associations` in
the format `place-slug:role` (e.g., `the-hall:authority`, `the-mere:exile`, `market:debtor`).
The role context specifies how the character is positioned at that place — not just present.
If existing place associations lack role context, add it.

**personality_profile**: All 7 axes required (warmth, authority, openness, interiority,
stability, agency, morality), each 0.0-1.0. Use the bedrock archetype's profile as a
starting point, then adjust for this character's specific world-position and social
entanglement. A character with high authority in a world where governance-form is
contested should have lower stability than the archetype baseline suggests. If no
`personality_profile` exists, generate one. If one exists but is not grounded in this
character's specific conditions, revise it.

**communicability**: All 4 fields required (surface_area, translation_friction, timescale,
atmospheric_palette). The atmospheric_palette is a sensory string — textures, sounds,
smells, temperatures that follow this person. It should be specific to who they are and
where they stand in the world, not generic genre atmosphere.

**Multi-scale goals**: Q4 characters must have all three goal scales:
- `existential`: What they would die for or cannot live without. Must be constrained by
  their social position — not a universal human value, but what THIS person at THIS
  position in THIS world cannot relinquish.
- `arc`: What changes over the story — what their arc costs or produces.
- `scene`: What they want right now, today, in the immediate moment. Specific and concrete.

### Cross-Character Tension Verification

After enriching all characters individually, verify that Q3 and Q4 characters are
in productive relationship with each other. If all Q3-Q4 characters are in the same
cluster with no inter-cluster tension, that is a structural problem. At least one
relational seed between Q3-Q4 characters should name a tension, not just a connection.

If Q4 and Q3 characters have no relational seeds referencing each other, add the most
load-bearing connection. The significant characters should be entangled.

## Output

Output valid JSON: a complete array of all significant characters (Q3 first, then Q4).
Use the same schema as the Phase 3b character-significant-elicitation output. Do not add
fields not in that schema. Do not add commentary or explanation.

No preamble, no explanation, no markdown fences. Output the JSON array only.
