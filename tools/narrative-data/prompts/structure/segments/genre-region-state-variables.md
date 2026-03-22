You are a structured data extractor. Read the text below and produce a JSON array of state variable template objects for a genre region.

## Rules

- Each state variable has a canonical_id, genre_label, behavior, initial_value, threshold, threshold_effect, and activation_condition
- canonical_id: derive from the variable name in kebab-case — "Dread Level" → "dread-level"
- genre_label: the exact name the source uses for this variable
- behavior: one of depleting, accumulating, fluctuating, progression, countdown
- initial_value: estimate a 0.0-1.0 starting value from context clues:
  - "starts low/as outsider/at zero" → 0.1-0.2
  - "starts moderate/neutral" → 0.4-0.5
  - "starts high/full/at maximum" → 0.8-0.9
  - "tracked from 0 to 100" → initial 0.1 (near the low end at start)
  - For depleting resources (escape routes, sanity, rationality): initial 0.7-0.9 (starts high, depletes)
  - For accumulating variables (knowledge, trust, integration): initial 0.1-0.3 (starts low, builds)
  - For countdown variables: initial 1.0 (starts full, counts to 0)
- threshold: estimate the 0.0-1.0 point where something significant changes:
  - "high X means danger/death/transformation" → threshold 0.7-0.8
  - "when depleted/exhausted/gone" → threshold 0.1-0.2
  - "crossing point/tipping point" → threshold 0.5
  - For countdown: threshold 0.0 (when time runs out)
- threshold_effect: describe what happens when the threshold is crossed — extract from the source text
- activation_condition: what triggers or starts this variable tracking — extract from source or null

## General Rules

- All numeric values must be normalized floats between 0.0 and 1.0
- Always provide initial_value and threshold — infer from the variable's behavior type and description if not explicitly stated
- If a field genuinely cannot be determined even by inference, use null
- Do not invent information not present in the source, but DO infer reasonable defaults from behavior type

## Source Content

{raw_content}

## Target Schema

{schema}

Output only valid JSON, no markdown formatting.
