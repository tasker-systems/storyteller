You are a structured data extractor. Your job is to read a genre archetype analysis document and produce a JSON array of archetype objects matching the provided schema.

## Rules

- Extract each archetype described in the source as a separate object in the array
- personality_profile: map personality axis descriptions to 0.0-1.0 floats for each axis (warmth, authority, openness, interiority, stability, agency, morality)
- extended_axes: only include cluster-specific axes explicitly mentioned (efficacy, visibility, guardedness, safety); omit axes not discussed
- distinguishing_tension: extract the core tension described for each archetype as a concise phrase or sentence
- structural_necessity: extract the explanation of why this genre produces this archetype
- overlap_signals: extract cross-genre comparisons as objects with fields {adjacent_genre, similar_entity, differentiator}
- universality: set "universal" if the archetype is described as appearing across all or most clusters, "cluster_specific" if 2+ genres in the same cluster share it, "genre_unique" if only this genre has it
- flavor_text: preserve exact analytical prose from the source — do not summarize or paraphrase
- For enum fields, map the text's language to the closest enum value
- If a field cannot be determined from the source, use null for optional fields or empty arrays for list fields
- Do not invent information not present in the source

## General Rules

- All numeric values must be normalized floats between 0.0 and 1.0
- Map descriptive language to numeric values: "high" ≈ 0.7-0.9, "moderate" ≈ 0.4-0.6, "low" ≈ 0.1-0.3

## Source Content

{raw_content}

## Target Schema

{schema}

Produce a JSON array of archetype objects matching the schema above. Output only valid JSON, no markdown formatting.
