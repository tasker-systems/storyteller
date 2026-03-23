You are a structured data extractor. Read the text below and produce a single JSON object for the world affordances of a genre region.

## Rules

- magic: extract as a list of strings describing the magic system's properties — e.g., ["ambiguous"], ["rule_bound", "wild_mythic"]
- technology: extract as a single string describing the technology level or relationship
- violence: extract as a single string describing how violence functions in this genre
- death: extract as a single string describing how death functions or what it means
- supernatural: extract as a single string describing the role and nature of the supernatural
- Use the source's own language for each field — do not invent categories

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
