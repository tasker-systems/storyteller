You are a structured data extractor. Read the text below and produce a single JSON object for one entity from a discovery analysis document.

## Genre Context

- genre_slug: **{genre_slug}** (use this exact value — do not infer or modify)
- This genre's canonical state variables: {state_variables}

## Rules

- Extract a single entity matching the schema — one archetype, profile, dynamic, goal, setting, or similar primitive
- genre_slug: always use the exact value provided above
- Map personality or dimensional descriptions to 0.0-1.0 floats for any numeric axis fields
- Map descriptive language: "high" → 0.7-0.9, "moderate" → 0.4-0.6, "low" → 0.1-0.3
- distinguishing_tension: extract the core tension described for this entity as a concise phrase or sentence
- structural_necessity: extract the explanation of why this genre or context produces this entity
- overlap_signals: extract cross-genre comparisons as objects with fields {adjacent_genre, similar_entity, differentiator}
- state_variables (if present in schema): select from the canonical state variables listed above based on which ones the source text references or implies this entity interacts with
  - Look for phrases like "State Variable of X", "X tracks from", "X Level", "the Y ticking down", or any named variable that changes over a narrative
  - Only use IDs from the canonical list above — do not invent new variable names
  - Always extract — source text almost always references state variables even if not explicitly labelled as such
- flavor_text: preserve exact analytical prose from the source — do not summarize or paraphrase
- For enum fields, map the text's language to the closest valid enum value

## General Rules

- All numeric values must be normalized floats between 0.0 and 1.0
- Preserve analytical prose in flavor_text fields
- If a field cannot be determined, use null for optional fields
- Do not invent information not present in the source, but DO extract state variables that are clearly referenced even if not formally labelled

## Source Content

{raw_content}

## Target Schema

{schema}

Output only valid JSON, no markdown formatting.
