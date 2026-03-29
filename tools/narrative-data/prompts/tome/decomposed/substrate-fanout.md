You are generating a single social cluster for a narrative world. Social clusters are the
identity-and-belonging layer — the groups people are born into, sorted into, or claimed by.
They are distinct from organizations (which people join or are assigned to).

This cluster must be grounded in the world's material conditions. Its basis should follow
logically from the axis values — do not assign a basis because it sounds interesting. Assign
it because the world's conditions actually produce this form of social cohesion.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position (Axis Subset)

{axes_subset}

## Cluster Basis

{cluster_basis}

Basis options and their social logic:
- `blood`: Kinship lineages — family names, descent groups, clan identity
- `occupation`: Work-defined groups — professional castes, labor cohorts, trade identities
- `belief`: Cosmological or moral commitments — sects, denominations, ideological factions
- `geography`: Place-based belonging — neighborhoods, commons groups, territorial identities
- `affiliation`: Chosen cohorts — crews, lodges, voluntary associations with initiation

## Existing Places and Organizations

{upstream_summary}

## What You Are Generating

ONE social cluster with the given basis. It should connect to the existing social
infrastructure — clusters gather somewhere, align or conflict with organizations, occupy
particular places.

The `place_associations` array should contain slugs from the upstream summary.
The `org_associations` array should contain slugs from the upstream summary.
Do not invent new places or organizations.

The `grounding.axes` array should name the specific axis values that explain why this
cluster exists in this form. Entanglement with places and organizations is not optional —
social clusters do not float free of material context.

## Output

Output valid JSON only. No preamble, no explanation, no markdown fences.

{
  "slug": "<kebab-case identifier>",
  "name": "<cluster name>",
  "basis": "{cluster_basis}",
  "description": "<2-3 sentences — what holds this group together, what distinguishes it, what it costs to belong>",
  "place_associations": ["<place-slug>"],
  "org_associations": ["<org-slug>"],
  "grounding": {
    "axes": ["<axis:value>", "<axis:value>"]
  }
}

Output the JSON object only.
