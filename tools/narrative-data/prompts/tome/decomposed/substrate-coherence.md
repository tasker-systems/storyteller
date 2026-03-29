You are reviewing and enriching a set of independently-generated social clusters for a narrative
world, then generating the pairwise relationships between them. Social clusters are not
organizations — they are what people are born into, marry across, and carry in their bones.
Narrative tension lives at the boundaries between clusters, not at their centers.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Upstream Context

{upstream_context}

## Draft Entities

The following social clusters were generated independently. They may lack inter-cluster
relationships, have inconsistent hierarchy positions, or use grounding that is too generic:

{draft_entities}

## Your Tasks

### Part 1: Cluster Review

For each cluster, verify and enrich the following:

**Consistency check**: Does the cluster's `basis` (blood, occupation, belief, geography,
affiliation) match the world's kinship-system axis? A world with clan-tribal kinship should
not produce clusters based purely on occupational affiliation unless the axis positions
explicitly support occupational heredity.

**Hierarchy coherence**: The `hierarchy_position` values across the cluster set should
reflect actual social stratification — not all clusters can be dominant or established.
Ensure at least one cluster is marginal or contested. The stated hierarchy must be
connected to the social-stratification axis from the World Position.

**Material grounding**: Each cluster's `grounding` should reference specific axis values,
not genre-typical group descriptions. "A warrior clan with blood ties" is insufficient.
"A blood-basis cluster whose hierarchy_position is dominant because governance-form is
hereditary-rule and they hold ancestral claim to the commons" is load-bearing.

**Org relationships**: Verify that `org_relationships` reference actual organizations from
the upstream context. If a cluster has no org relationships, assign at least one —
every social cluster interacts with formal institutions in some way, even if that interaction
is exclusion or avoidance.

### Part 2: Pairwise Relationships

Generate one relationship entry for EVERY pair of clusters. For N clusters, you must produce
N*(N-1)/2 relationships:
- 3 clusters → 3 relationships
- 4 clusters → 6 relationships
- 5 clusters → 10 relationships

Each relationship must have:
- `from` and `to` cluster slugs (directed — the relationship is from one cluster's perspective)
- `relationship_type`: one of `tension`, `cooperation`, `dependency`, `rivalry`, `deference`
- `description`: one sentence describing the inter-cluster boundary
- A materially specific account of the friction or exchange

Relationships must be specific and material. "These groups distrust each other" is not
a boundary tension. "The Hallodays control access to the grain store that Farrow labor
fills — dependency that neither cluster calls by its name" is a boundary tension.

Make boundaries productive for character placement. Characters will be situated AT these
boundaries. A character caught between two clusters becomes more interesting the more
specific and material the boundary is.

## Output

Output valid JSON: an object with two keys — `"clusters"` (enriched array) and
`"relationships"` (pairwise array).

**Cluster object** — same schema as input, enriched in place:
```json
{
  "slug": "kebab-case-identifier",
  "name": "...",
  "basis": "...",
  "description": "...",
  "grounding": { "social_axes": [...], "economic_axes": [...], "active_edges": [...] },
  "hierarchy_position": "...",
  "org_relationships": ["org-slug:relationship"],
  "history": "..."
}
```

**Relationship object:**
```json
{
  "from": "cluster-slug",
  "to": "cluster-slug",
  "relationship_type": "tension|cooperation|dependency|rivalry|deference",
  "description": "One sentence describing the inter-cluster boundary.",
  "boundary_tension": "The specific material point of friction or exchange."
}
```

No preamble, no explanation, no markdown fences. Output the JSON object only.
