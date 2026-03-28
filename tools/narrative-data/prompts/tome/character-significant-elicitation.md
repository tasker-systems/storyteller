You are generating the narratively significant characters for a narrative world. The world
has been fully composed: places, organizations, social substrate, and mundane characters
already exist. You are now generating the people who carry narrative tension and drive story.

These characters inhabit the world — they do not merely appear in it. Their agency is
defined by, enabled by, and constrained by their position in the social web. Ascending
narrative centrality means deeper characterization AND deeper entanglement. The more a
character can do, the more the world can do to them.

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

{social_substrate_context}

## Mundane Characters (Q1-Q2)

These are the people already inhabiting this world. Q3-Q4 characters do not float above
them — they are embedded among them. Each significant character's relational_seeds must
reference at least one mundane character by slug.

{mundane_characters_context}

## Bedrock Archetypes

These are the structural character patterns that this genre produces. They represent
recurring tensions and structural positions, not personality templates.

Use them as lenses, not as a menu. A character may resonate with one archetype's tension
while occupying another's structural position. Not every archetype needs to appear. The
world-position and social substrate determine which patterns are structurally necessary —
an archetype that doesn't fit the material conditions of THIS world should not be forced.

{archetypes_context}

## Archetype Dynamics

These are characteristic relationship patterns between archetypes in this genre.
Use them to inform how significant characters relate to each other.

{archetype_dynamics_context}

## Anti-Template Instruction

Each character must be situated at a specific social substrate boundary. Their archetype
is how they cope with that boundary position, not a personality assigned from a catalog.

Do not generate characters to fill archetype slots. Generate people whose position in
the social web produces the tensions that archetypes describe. The Earnest Warden is not
a personality type — it is what happens when someone with genuine care for the community
is structurally positioned to enforce its most harmful norms.

## Task

Generate characters in two blocks. Generate Q3 FIRST.

### Q3 — Tension-Bearing Characters (2-3)

Characters who inhabit the gap between stated and operative reality. They live at social
substrate boundaries — caught between clusters, between loyalty and conscience, between
what they're supposed to be and what the world requires them to do.

Each must have:
- Name, role, 3-4 sentence description
- Full archetype mapping: primary + shadow + genre_inflection (how the archetype expresses
  in THIS character at THIS world-position)
- Place associations (1-2)
- Structured cluster_membership: primary cluster, boundary_with another cluster,
  boundary_tension specific to this character
- stated_operative_gap: what they claim vs. what they actually do
- Relational seeds (4+) using verb:target-slug format (e.g., "supervises:elda-the-carrier",
  "fears:the-keepers", "married-into:the-hallodays"). Must include at least one Q1-Q2
  character slug.
- Arc-scale goal

### Q4 — Scene-Driving Characters (1-2)

Characters who are the genre expressing itself through a specific person in specific
material conditions at a specific social position. Everything Q3 has, plus:
- personality_profile: 7-axis numeric (warmth, authority, openness, interiority, stability,
  agency, morality) each 0.0-1.0. Informed by the bedrock archetype profile but adjusted
  for this character's world-position and social entanglement.
- communicability: surface_area (0.0-1.0), translation_friction (0.0-1.0),
  timescale (momentary|biographical|generational|geological|primordial),
  atmospheric_palette (sensory string)
- Multi-scale goals: existential (what they'd die for), arc (what changes over the story),
  scene (what they want right now)
- Relational seeds (5+) using verb:target-slug format, with multiple references to Q1-Q2
  and Q3 characters

## Output Schema

Output valid JSON: a single array containing ALL characters (Q3 first, then Q4).
No commentary outside the JSON.

**Q3 character object:**

```json
{
  "centrality": "Q3",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their position or function",
  "description": "3-4 sentences. Narrative-rich, specific to world position.",
  "archetype": {
    "primary": "The Archetype Name",
    "shadow": "The Shadow Archetype Name",
    "genre_inflection": "How this archetype expresses in THIS character at THIS position."
  },
  "place_associations": ["place-slug", "place-slug"],
  "cluster_membership": {
    "primary": "cluster-slug",
    "boundary_with": "other-cluster-slug",
    "boundary_tension": "What makes this boundary productive for this character."
  },
  "stated_operative_gap": {
    "stated": "What they claim to do or be.",
    "operative": "What they actually do. Who benefits. What it costs them."
  },
  "relational_seeds": [
    "supervises:mundane-character-slug",
    "fears:org-slug",
    "married-into:cluster-slug",
    "protects:another-character-slug"
  ],
  "goals": {
    "arc": "What changes over the story for this character."
  }
}
```

**Q4 character object:**

```json
{
  "centrality": "Q4",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their position or function",
  "description": "4-6 sentences. The genre expressing itself through a specific person.",
  "archetype": {
    "primary": "The Archetype Name",
    "shadow": "The Shadow Archetype Name",
    "genre_inflection": "How this archetype expresses in THIS character at THIS position."
  },
  "personality_profile": {
    "warmth": 0.0,
    "authority": 0.0,
    "openness": 0.0,
    "interiority": 0.0,
    "stability": 0.0,
    "agency": 0.0,
    "morality": 0.0
  },
  "place_associations": ["place-slug:role", "place-slug:role"],
  "cluster_membership": {
    "primary": "cluster-slug",
    "boundary_with": "other-cluster-slug",
    "boundary_tension": "What makes this boundary load-bearing for this character."
  },
  "stated_operative_gap": {
    "stated": "What they claim to do or be.",
    "operative": "What they actually do. The network requires it."
  },
  "relational_seeds": [
    "controls:org-slug",
    "answers-to:org-slug",
    "mother-of:q3-character-slug",
    "distrusts:mundane-character-slug",
    "grandmother-of:unnamed-entity"
  ],
  "goals": {
    "existential": "What they would die for or cannot live without.",
    "arc": "What changes over the story.",
    "scene": "What they want right now, today, this moment."
  },
  "communicability": {
    "surface_area": 0.0,
    "translation_friction": 0.0,
    "timescale": "biographical",
    "atmospheric_palette": "Sensory string: textures, sounds, smells that follow this person."
  }
}
```

Field notes:
- `personality_profile`: All values 0.0-1.0. Use the bedrock archetype's profile as a
  starting point, then adjust for this character's specific world-position and entanglement.
  A Warden in a mining community may have lower warmth and higher authority than the
  archetype baseline.
- `communicability.surface_area`: How much of this character is available to narrative
  interaction (0.0 = guarded/opaque, 1.0 = fully expressive)
- `communicability.translation_friction`: How difficult for the Narrator to render this
  character's inner state (0.0 = immediately legible, 1.0 = deeply alien)
- `relational_seeds`: Directional relationships using verb:target-slug format. The verb
  describes the relationship (supervises, fears, married-into, protects, controls, answers-to,
  distrusts, depends-on, etc.). Must include at least one Q1-Q2 character slug and at least
  one organization or social cluster slug.
- `cluster_membership.boundary_with`: Must reference a different cluster from the social
  substrate. This is where the character's narrative tension lives.

Output the JSON array only. No preamble, no explanation, no markdown fences.
