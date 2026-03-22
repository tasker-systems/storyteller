You are a structured data extractor. Your job is to read a cluster-level archetype synthesis document and produce a JSON array of canonical archetype objects matching the provided schema.

## Rules

- Extract canonical archetypes from the cluster synthesis, not individual genre instances
- canonical_name: the cluster-level name that unifies the variants
- core_identity: the cross-genre description that applies to all variants in the cluster
- genre_variants: list of objects {genre_slug, variant_name, key_differences} from the genre-specific sections
- uniqueness: "universal" if found across all clusters, "cluster_specific" if 2+ genres in this cluster, "genre_unique" if only one genre
- distinguishing_tension: extract the core tension that persists across all genre variants
- structural_role: the canonical narrative function this archetype serves
- personality_profile: cross-cluster numeric values (warmth, authority, openness, interiority, stability, agency, morality) — only include axes where the synthesis establishes a cluster-level value
- flavor_text: preserve synthesis prose that explains the canonical identity — do not summarize
- If a field cannot be determined from the source, use null for optional fields or empty arrays for list fields
- Do not invent information not present in the source

## General Rules

- All numeric values must be normalized floats between 0.0 and 1.0
- Map descriptive language to numeric values: "high" ≈ 0.7-0.9, "moderate" ≈ 0.4-0.6, "low" ≈ 0.1-0.3

## Source Content

{raw_content}

## Target Schema

{schema}

Produce a JSON array of canonical archetype objects matching the schema above. Output only valid JSON, no markdown formatting.
