You are a structured data extractor. Your job is to read a genre dimensional analysis document and produce a single JSON object matching the provided schema.

## Rules

- All continuous axis values must be normalized floats between 0.0 and 1.0
- Map descriptive language to numeric values: "high" ≈ 0.7-0.9, "moderate" ≈ 0.4-0.6, "low" ≈ 0.1-0.3
- For weighted tags (thematic dimensions), identify 1-3 primary treatments mentioned in the text and assign weights that reflect their relative importance (values 0.0-1.0)
- For state variables, identify the behavior type (depleting, accumulating, fluctuating, progression, countdown) from how the text describes the variable's dynamics
- Set initial_value and threshold for state variables when the text provides numeric or qualitative guidance
- Preserve the original analytical prose in flavor_text fields — do not summarize or paraphrase
- For enum fields, map the text's language to the closest enum value
- For locus_of_power and narrative_structure, extract the ranked ordering (up to 3 items)
- classification should be "constraint_layer" for most genres (26 of 30), "standalone_region" for genres described as independent regions, or "hybrid_modifier" for modifier genres
- For narrative_contracts, extract hard invariants the genre guarantees (e.g., "Must resolve relationship", "Knowledge rewarded")
- For boundaries, extract genre transition triggers (when state variables cross thresholds and the genre shifts)
- If a field cannot be determined from the source text, use null for optional fields or empty arrays for list fields
- Do not invent information not present in the source

## Source Content

{raw_content}

## Target Schema

{schema}

Produce a single JSON object matching the schema above. Output only valid JSON, no markdown formatting.
