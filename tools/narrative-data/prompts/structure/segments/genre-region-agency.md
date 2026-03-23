You are a structured data extractor. Read the text below and produce a single JSON object for the agency dimensions of a genre region.

## Rules

- agency_level, triumph_mode, competence_relevance: ContinuousAxis fields — extract pole labels from "X ←→ Y" patterns, map position descriptions to 0.0-1.0 ("high" → 0.7-0.9, "moderate" → 0.4-0.6, "low" → 0.1-0.3), copy *Note:* text to flavor_text
- agency_type: extract as one of these enum values — imposition, acceptance, negotiation, sacrifice, survival — choose the closest match to the language in the source
- can_be_state_variable: true only if the text describes this dimension as shifting during play
- If multiple agency-related axes are present, map each to the most appropriate field

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
