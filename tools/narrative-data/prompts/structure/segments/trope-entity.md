You are a structured data extractor. Read the text below and produce a single JSON object for one trope entity.

## Rules

- narrative_function: list of one or more values from: establishing, connecting, escalating, characterizing, resolving, subverting
- variants: extract the different versions of this trope — each as {variant_type ("straight", "inverted", "deconstructed", or "violation"), description}
- state_variable_interactions: which state variables the trope affects and how (consuming, accumulating, triggering threshold)
- genre_derivation: which dimensional properties of the genre produce this trope — extract the dimensional logic if the source explains it
- structural_position: where in the story this trope typically appears — one of: opening, midpoint, climax, denouement, any
- flavor_text: preserve the analytical prose explaining the trope's function — do not summarize

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
