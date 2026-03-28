You are generating the social substrate for a narrative world. The world has been composed
from a mutual production graph, and places and organizations have already been generated.

Your task is to produce the named social clusters — lineages, factions, kinship groups —
that people in this world are born into, marry across, and escape from. These are not
organizations (which are formal power — what you join). These are identity and belonging —
what you are.

Every person in this world belongs to one of these clusters. Narrative tension lives at
the boundaries between them, not at their centers.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

The following axis positions define this world's narrative coordinates. Seeds are
author-provided; inferred positions were propagated from seeds via the Tome mutual
production graph, with justification and confidence shown.

**Social-forms axes** (primary drivers of social clustering) are marked with ★.
**Economic-forms axes** (secondary — material basis for group formation) are marked with ◆.

{world_preamble}

## Genre Profile

{genre_profile_summary}

## Places Context

{places_context}

## Organizations Context

{orgs_context}

## Anti-Template Instruction

Do NOT produce genre-typical factions. Let the axes determine what social groups this
world must produce. A clan-tribal kinship system with caste-hereditary stratification
produces different clusters than a chosen-elective system with meritocratic stratification.

If you find yourself generating a group because it sounds like it belongs in this genre
(a coven, a street gang, a noble house), stop and ask: do the actual axis positions of
THIS world produce that social form? What does kinship look like when the kinship-system
is {kinship_system_value} and social-stratification is {stratification_value}?

## Task

### Part 1: Social Clusters (3-5)

Generate a flat list of social clusters. These are NOT tiered — every cluster carries
narrative weight. The basis of each cluster is driven by the kinship-system axis:
- clan-tribal → blood-basis clusters (lineages, family names)
- chosen-elective → affiliation-basis clusters (crews, lodges, cohorts)
- institutional-assigned → occupation-basis clusters (work units, professional castes)
- communal-collective → geography-basis clusters (neighborhoods, commons groups)
- nuclear-conjugal → smaller family units with weaker cluster identity

Each cluster needs:
- A name grounded in the world (not generic)
- A basis (blood, occupation, belief, geography, affiliation)
- A hierarchy position (dominant, established, marginal, outsider, contested)
- Relationships to existing organizations
- One sentence of history

### Part 2: Pairwise Relationships

For each pair of clusters, generate one relationship entry with:
- A relationship type (alliance, rivalry, intermarriage, avoidance, dependency, contested-boundary)
- One sentence describing the boundary
- A boundary tension — the specific point of friction or exchange between these groups

The boundary tensions are where characters will be placed. Make them specific and material,
not abstract.

## Output Schema

Output valid JSON: an object with `clusters` array and `relationships` array.
No commentary outside the JSON.

**Cluster object:**

```json
{
  "slug": "kebab-case-identifier",
  "name": "Human-readable cluster name",
  "basis": "<one of: blood, occupation, belief, geography, affiliation>",
  "description": "2-3 sentences. Grounded in axes. What does membership mean, feel like, require?",
  "grounding": {
    "social_axes": ["axis-slug:value"],
    "economic_axes": ["axis-slug:value"],
    "active_edges": ["source →type→ target (weight)"]
  },
  "hierarchy_position": "<one of: dominant, established, marginal, outsider, contested>",
  "org_relationships": ["org-slug:relationship"],
  "history": "One sentence — how long established, what they survived, what they claim."
}
```

**Relationship object:**

```json
{
  "cluster_a": "cluster-slug",
  "cluster_b": "cluster-slug",
  "type": "<one of: alliance, rivalry, intermarriage, avoidance, dependency, contested-boundary>",
  "description": "One sentence describing the boundary.",
  "boundary_tension": "The specific point of friction or exchange. Material, not abstract."
}
```

Field notes:
- `org_relationships`: Directional references to organizations from organizations.json,
  e.g. "parish-council:founding-members", "keepers:excluded", "labor-guild:primary-workforce"
- `hierarchy_position`: Driven by social-stratification axis. The stated hierarchy may not
  match operative position — if the world has a stated/operative gap on social-stratification,
  note which position is stated and which is operative in the description.
- Generate relationships for ALL cluster pairs (for 3 clusters: 3 pairs; for 4: 6 pairs; for 5: 10 pairs).

Output the JSON object only. No preamble, no explanation, no markdown fences.
