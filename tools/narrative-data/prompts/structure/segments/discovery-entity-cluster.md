You are a structured data extractor. Read the text below and produce a single JSON object for one canonical cluster entity from a discovery synthesis document.

## Rules

- canonical_name: the cluster-level name that unifies genre variants
- core_identity: the cross-genre description that applies to all variants in the cluster
- genre_variants: list of {genre_slug (kebab-case), variant_name, key_differences} from genre-specific sections
- uniqueness: one of universal (found across all clusters), cluster_specific (2+ genres in this cluster), genre_unique (only one genre)
- distinguishing_tension: the core tension that persists across all genre variants
- flavor_text: preserve synthesis prose that explains the canonical identity — do not summarize

## General Rules

- All numeric values must be normalized floats between 0.0 and 1.0
- Preserve analytical prose in flavor_text fields
- If a field cannot be determined, use null for optional fields
- Do not invent information not present in the source

## Source Content

{raw_content}

## Target Schema

{schema}

Output only valid JSON, no markdown formatting.
