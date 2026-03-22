You are a structured data extractor. Read the text below and produce a single JSON object with the genre metadata fields.

## Rules

- genre_slug: convert the title to kebab-case — "Folk Horror" → "folk-horror", "High Epic Fantasy" → "high-epic-fantasy"
- genre_name: the full title exactly as written
- classification: default to "constraint_layer" unless the text explicitly says the genre is a standalone region or hybrid modifier
- modifies: list of genre slugs if the text describes genres this one layers onto or modifies — empty array if none mentioned
- flavor_text: any summary or introductory prose describing the genre's identity

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
