You are a structured data extractor. Read the text below and produce a single JSON object for the locus of power of a genre region.

## Rules

- Extract the ranked list from "Primary/Secondary/Tertiary" entries in the source
- Use lowercase values only: place, person, system, relationship, cosmos
- Example: "Primary: Place (The Land). Secondary: System/Institution. Tertiary: Cosmos/Fate." → ["place", "system", "cosmos"]
- The result is an ordered array — first element is Primary, second is Secondary, third is Tertiary
- If fewer than three tiers are described, return only the tiers that are present

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
