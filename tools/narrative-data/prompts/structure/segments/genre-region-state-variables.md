You are a structured data extractor. Read the text below and produce a JSON array of state variable template objects for a genre region.

## Rules

- Each state variable has a canonical_id (kebab-case identifier), genre_label (the genre's own name for it), behavior, initial_value, and threshold
- canonical_id: derive from the variable name in kebab-case — "Dread Level" → "dread-level"
- genre_label: the exact name the source uses for this variable
- behavior: one of depleting, accumulating, fluctuating, progression, countdown — choose the closest match
- initial_value and threshold: 0.0-1.0 floats when described in the source — use null if not specified
- If no state variables are described, return an empty array

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
