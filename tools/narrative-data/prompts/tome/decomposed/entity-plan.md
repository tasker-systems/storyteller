You are planning the entity budget for a narrative world. The world has been composed from
a mutual production graph and its axis positions have been compressed into a domain-grouped
summary below.

Your task is to determine how many entities of each type to generate, based on the world's
material conditions — not genre conventions. Entity counts should reflect the density and
complexity implied by the axis positions.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Genre Profile

{genre_profile_summary}

## Entity Budget Guidelines

These are ranges, not targets. Choose counts that reflect what THIS world's material
conditions actually require.

**Places** (8-15 total):
Distribute by type. Choose counts based on settlement density, economic complexity,
and social geography implied by the axes.

Types and typical functions:
- `infrastructure`: Roads, bridges, utilities, systems-of-movement
- `institution`: Formal buildings — councils, courts, schools, temples
- `dwelling`: Where people live — individual or communal
- `commercial`: Markets, trades, exchange sites
- `liminal`: Threshold spaces — edges, boundaries, marginal territory
- `sacred`: Ritual, belief, commemorative sites
- `natural`: Landscape features with narrative agency
- `workshop`: Production, craft, labor spaces

**Organizations** (3-8 total):
Formal power structures — what people join or are assigned to. Count should reflect
political complexity and institutional density of the world.

**Social Clusters** (3-6 total):
Identity and belonging — what people are born into. Basis driven by kinship-system axis:
- `clan-tribal` → blood basis (lineages, family names)
- `chosen-elective` → affiliation basis (crews, lodges, cohorts)
- `institutional-assigned` → occupation basis (work units, professional castes)
- `communal-collective` → geography basis (neighborhoods, commons groups)
- `nuclear-conjugal` → smaller family units with weaker cluster identity

Include a `basis_hint` for each cluster that names the kinship-system axis value
and its implication.

**Characters — Mundane** (Q1: 4-8, Q2: 3-5):
Q1 = peripheral centrality — present, necessary, background. Every world has more Q1 than Q2.
Q2 = mid-level centrality — regular roles, some agency within their domain.

**Characters — Significant** (Q3: 2-4, Q4: 1-2):
Q3 = high centrality — people scenes will orbit. Shaped by the social substrate.
Q4 = apex centrality — the person or persons this world revolves around. Never more than 2.

## Output

Output valid JSON only. No preamble, no explanation, no markdown fences.

```json
{
  "places": {
    "count": <integer 8-15>,
    "distribution": {
      "infrastructure": <integer>,
      "institution": <integer>,
      "dwelling": <integer>,
      "commercial": <integer>,
      "liminal": <integer>,
      "sacred": <integer>,
      "natural": <integer>,
      "workshop": <integer>
    }
  },
  "organizations": {
    "count": <integer 3-8>
  },
  "clusters": {
    "count": <integer 3-6>,
    "basis_hint": "<kinship-system axis value> → <basis implication>"
  },
  "characters_mundane": {
    "q1_count": <integer 4-8>,
    "q2_count": <integer 3-5>
  },
  "characters_significant": {
    "q3_count": <integer 2-4>,
    "q4_count": <integer 1-2>
  }
}
```

The `distribution` values in `places` must sum to `places.count`.
Output the JSON object only.
