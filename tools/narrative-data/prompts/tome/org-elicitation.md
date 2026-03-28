You are generating organizations and institutions for a narrative world. The world has been
composed from a mutual production graph, and places have already been generated.

Your task is to produce organizations that structure power, labor, belief, or social life in
this world. These organizations must be grounded in the world's actual axis positions — not
generic institutions, but organizations that could only exist in this specific world at this
specific set of coordinates.

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
and narrative sensibility. Use these to ensure each organization feels genre-coherent.

{genre_profile_summary}

## Places Context

These are the places that constitute this world. Organizations operate within and
across these places — each organization should be anchored to at least one place
in this list.

{places_context}

## Task

Generate organizations in two tiers, in this order:

### Tier 2 — Mundane Institutions (3-5)

Generate these FIRST. These are organizations that must exist and function as advertised
given the world's political, economic, and social axis positions. The parish church, the
market guild, the village militia, the grain assessor's office. They work roughly as they
claim to. Their value is presence and texture — they make the world legible — not narrative
tension.

Tier 2 organizations use a simpler schema: slug, name, org_type, one-sentence description,
stated purpose, and one place association. No stated/operative gap — these function as
described. No authority_basis field. No relational_seeds required.

### Tier 1 — Narratively Significant Organizations (1-3)

Generate these SECOND. These are organizations where the gap between stated function and
operative reality creates narrative tension. They must emerge FROM the Tier 2 institutional
landscape — they exploit, corrupt, subvert, or parasitize the mundane institutions. A
Tier 1 organization that has no relationship to any Tier 2 institution is probably floating
free of the world.

One load-bearing organization with a genuine stated/operative gap is more compelling than
three shallow cabals. Use the full schema including authority_basis, membership,
stated_vs_operative, relational_seeds, and at least one explicit relationship to a Tier 2
institution.

## Anti-Proliferation

One organization with a genuine stated/operative gap is sufficient if it's load-bearing.
Three secret cabals strains credulity. Reserve the gap for where axis positions genuinely
produce tension between appearance and reality — not every institution needs a hidden agenda.
If you find yourself writing multiple Tier 1 organizations with similar gap structures, collapse
them into one.

## Critical: Stated vs. Operative Reality (Tier 1 Only)

For Tier 1 organizations, surface the gap between the stated function and the operative
reality. This gap is where narrative tension lives.

The stated function is what the organization claims to do or what members believe it does.
The operative reality is what it actually does — who benefits, what it enforces, what it
conceals. Do not flatten this gap. A guild that claims to protect craftworkers but actually
controls their wages and crushes competition is narratively richer than either a pure
protector or a pure exploiter.

## Output Schema

Output valid JSON: an array of organization objects, Tier 2 objects first, then Tier 1.
No commentary outside the JSON.

### Tier 2 object structure:

```json
{
  "tier": 2,
  "slug": "kebab-case-identifier",
  "name": "Human-readable organization name",
  "org_type": "<one of: governance, economic, religious, military, social, labor, educational>",
  "description": "One sentence. Grounded in an axis value or active edge.",
  "stated_purpose": "What this institution does and why it exists.",
  "place_associations": ["place-slug:role"]
}
```

### Tier 1 object structure:

```json
{
  "tier": 1,
  "slug": "kebab-case-identifier",
  "name": "Human-readable organization name",
  "org_type": "<one of: governance, economic, religious, military, social, labor, educational>",
  "description": "2-4 sentences, narrative-rich. Ground each detail in an axis value or active edge. Avoid generic institutional language.",
  "grounding": {
    "political_axes": ["axis-slug:value"],
    "economic_axes": ["axis-slug:value"],
    "social_axes": ["axis-slug:value"],
    "active_edges": ["source →type→ target (weight)"]
  },
  "authority_basis": "One sentence: the source and form of this organization's authority or legitimacy.",
  "membership": "Who belongs — entry conditions, exclusions, tiers of membership.",
  "place_associations": ["place-slug:role"],
  "stated_vs_operative": {
    "stated": "What the organization claims to do. What its members believe it does.",
    "operative": "What it actually does. Who benefits. What it enforces or conceals."
  },
  "relational_seeds": ["relation:target-slug"]
}
```

Field notes:
- `org_type`: The primary structural category — choose the one that best fits the dominant
  function even if the organization spans multiple categories
- `grounding.political_axes`: Axes that govern how this organization relates to authority,
  legitimacy, and coercion (e.g. authority-legitimation, governance-form)
- `grounding.economic_axes`: Axes that govern how this organization relates to production,
  distribution, and resources (e.g. labor-relations, resource-control)
- `grounding.social_axes`: Axes that govern how this organization relates to hierarchy,
  belonging, and identity (e.g. social-stratification, kinship-structure)
- `grounding.active_edges`: Edges from the world-position graph that are causally relevant
  to this organization's existence or function
- `place_associations`: List of place slugs with the organization's role at that place,
  e.g. "market-district:headquartered-in", "threshold-gate:controls-access-to",
  "production-hall:oversees-labor-at"
- `stated_vs_operative`: Both fields required for Tier 1. If there is genuinely no gap
  (unusual), state that explicitly in operative — do not leave it as a copy of stated
- `relational_seeds`: Directional relationships to other organizations by slug,
  e.g. "competes-with:merchant-guild", "absorbs-into:temple-authority",
  "nominally-subordinate-to:crown-administration". At least one seed should reference
  a Tier 2 institution slug.

Output the JSON array only. No preamble, no explanation, no markdown fences around the outer array.
