You are a structured data extractor. Your job is to read a cluster-level relational dynamics synthesis document and produce a JSON array of canonical dynamic objects matching the provided schema.

## Rules

- Extract canonical meta-dynamics from the cluster synthesis, not individual genre instances
- canonical_name: the cluster-level name for the meta-dynamic
- core_relational_pattern: the cross-genre description of the dynamic's essential structure
- scale: classify as "orbital", "arc", or "scene" — use the cluster-level scale if one is stated
- genre_variants: list of objects {genre_slug, variant_name, key_differences} from the genre-specific sections
- canonical_directionality: the directionality that holds across most variants
- evolution_pattern: the characteristic evolution that applies at the cluster level
- state_variable_interactions: cross-genre state variable interactions mentioned in the synthesis
- flavor_text: preserve synthesis prose that explains why this meta-dynamic unifies the cluster variants — do not summarize
- If a field cannot be determined from the source, use null for optional fields or empty arrays for list fields
- Do not invent information not present in the source

## General Rules

- All numeric values must be normalized floats between 0.0 and 1.0
- Map descriptive language to numeric values: "high" ≈ 0.7-0.9, "moderate" ≈ 0.4-0.6, "low" ≈ 0.1-0.3

## Source Content

{raw_content}

## Target Schema

{schema}

Produce a JSON array of canonical dynamic objects matching the schema above. Output only valid JSON, no markdown formatting.
