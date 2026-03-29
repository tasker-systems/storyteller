You are reviewing and enriching a set of independently-generated organizations for a narrative
world. These organizations were produced by separate fan-out calls and may lack place bindings,
have overlapping power structures, or use axis grounding that is too generic. Your task is to
bind them into a coherent institutional fabric.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Upstream Context

{upstream_context}

## Draft Entities

The following organizations were generated independently and require relational binding:

{draft_entities}

## Your Tasks

### 1. Place Binding

Every organization must be associated with at least one place from the upstream context.
If a Tier 2 organization has no `place_associations`, assign one based on what its stated
function requires. If a Tier 1 organization has only one place association, verify it is
the right one and consider whether a second is warranted (e.g., an organization that claims
jurisdiction over a territory but is never present at the entry point).

Use the format `place-slug:role`, e.g., `market-hall:headquartered-in`,
`boundary-gate:controls-access-to`, `village-commons:convenes-at`.

### 2. Power Structure

Verify that the power relationships between organizations are coherent and grounded. If two
Tier 1 organizations have overlapping authority in the same domain without a stated relationship,
add a `relational_seed` to clarify the relationship: competition, subordination, mutual avoidance,
nominal deference, etc.

Tier 2 organizations should generally be subordinate to, licensed by, or ignored by Tier 1
organizations. If a Tier 2 organization exists in a power vacuum, that's a structural problem —
assign it a relationship to a Tier 1 institution.

Do not flatten tensions. If two organizations are in genuine conflict, their relationship seeds
should name that conflict specifically, not neutralize it.

### 3. Axis Grounding

Check each Tier 1 organization's `grounding` fields. Generic references should be replaced
with specific axis values from the World Position. An organization grounded in
"authority-legitimation:religious-mandate" and "resource-control:extraction-and-export" is
load-bearing. An organization grounded in "power and tradition" is not.

For Tier 2 organizations, confirm that the description references at least one material condition
of the world rather than generic institutional language.

### 4. Differentiation

If two organizations perform similar functions (e.g., two economic institutions that both
"regulate trade"), differentiate them by scope, method, or constituency. Use their axis
groundings to determine which material dimension each controls. If they cannot be
meaningfully differentiated, merge them.

## Output

Output valid JSON: a complete array of enriched organization objects. Include ALL organizations —
both Tier 1 and Tier 2. Use the same schema as the input. Do not add commentary, explanation,
or fields not present in the input schema.

No preamble, no explanation, no markdown fences around the outer array. Output the JSON array only.
