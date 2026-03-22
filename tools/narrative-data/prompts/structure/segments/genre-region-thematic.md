You are a structured data extractor. Read the text below and produce a single JSON object for the thematic dimensions of a genre region.

## Rules

- Identify 1-3 treatments per dimension (power, identity, knowledge, connection)
- Assign weights reflecting relative importance described in the text, normalized to 0.0-1.0
- Example: "Power: Stewardship and Systemic Pressure" → {"Stewardship": 0.7, "Systemic Pressure": 0.3}
- If a dimension is not mentioned in the source, omit it or use an empty object
- Use the exact treatment names from the source text, not invented labels

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
