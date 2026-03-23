You are a structured data extractor. Read the text below and produce a JSON array of genre boundary drift objects for a genre region.

## Rules

- Each boundary describes a drift trigger: what causes the genre to drift toward another genre
- trigger: the condition or event that causes genre drift — use the source's language
- drift_target: the kebab-case slug of the genre the story drifts toward — "Folk Horror" → "folk-horror"
- description: explanation of how or why this transition happens
- If no boundary triggers or genre drift is described, return an empty array
- Only extract boundaries explicitly described in the source — do not infer unstated genre adjacencies

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
