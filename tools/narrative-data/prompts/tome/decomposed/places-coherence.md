You are reviewing and enriching a set of independently-generated places for a narrative world.
These places were produced by separate fan-out calls and may lack spatial relationships to each
other, have naming inconsistencies, or contain redundant entries. Your task is to bind them into
a coherent spatial fabric without losing the material specificity of individual entries.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Genre Profile

{genre_profile_summary}

## Upstream Context

{upstream_context}

## Draft Entities

The following places were generated independently and require relational binding:

{draft_entities}

## Your Tasks

### 1. Spatial Relationships

Assign or verify `relational_seeds` for every Tier 1 place. Seeds use verb:slug format
(e.g., "adjacent-to:the-crossroads", "controls-access-to:market-hall", "overlooks:the-mere").
Every Tier 1 place must have at least 2 relational seeds referencing other places in this set.
Tier 2 places should have at least 1 relational seed if one is warranted by proximity or function.

Do not invent seeds that contradict the spatial logic implied by the descriptions. If a seed
cannot be grounded in the world's material conditions, omit it.

### 2. Grounding Review

Check each place's `grounding.material_axes` array. If a place references generic genre axes
rather than specific axis values from this world, replace them with specific references from
the World Position above. A place grounded in "violence and darkness" is insufficiently
specific. A place grounded in "governance-form:hereditary-rule" and "resource-control:enclosure"
is load-bearing.

### 3. Naming Consistency

Names should feel like they emerged from the same naming culture. If places have inconsistent
naming patterns (some formal, some colloquial, some descriptive, some proper), adjust for
consistency without stripping specificity. The names should feel like locals use them.

### 4. Narrative Function Assignment

For Tier 1 places that lack a `spatial_role`, assign one: `center`, `threshold`, or `periphery`.
Ensure the set as a whole has at least one of each. If the set is top-heavy with centers,
demote the least load-bearing to threshold or periphery.

### 5. Redundancy Removal

If two places perform identical narrative functions, are spatially co-located with no meaningful
distinction, and have similar axis groundings, merge them. Keep the richer entry. Do not merge
places that are functionally adjacent but serve different narrative purposes.

## Output

Output valid JSON: a complete array of enriched place objects. Include ALL places — both Tier 1
and Tier 2. Use the same schema as the input (Tier 2 simpler, Tier 1 full). Do not add commentary,
explanation, or fields not present in the input schema.

No preamble, no explanation, no markdown fences around the outer array. Output the JSON array only.
