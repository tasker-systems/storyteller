You are a structured data extractor. Read the text below and produce a JSON array of narrative contract objects for a genre region.

## Rules

- Each contract has an invariant (string description) and enforced (boolean)
- Extract from "hard guarantees", "genre contracts", "invariants", or similar language in the source
- enforced: true if the text describes the contract as a hard requirement or invariant, false if it is a convention or soft expectation
- invariant: use the source's own language — preserve the exact phrasing where possible
- If no contracts are described, return an empty array

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
