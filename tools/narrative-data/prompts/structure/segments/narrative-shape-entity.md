You are a structured data extractor. Read the text below and produce a single JSON object for one narrative shape entity.

## Rules

- tension_profile: extract the family enum and a description — family values: rise_fall, oscillating, flat_then_sudden, inverted, staircase, wave, spiral, double_peak
- beats: parse each named beat from text or markdown tables — each beat has position (0.0-1.0), flexibility (load_bearing if required by the shape, ornamental if optional), tension_effect (what happens to tension), pacing_effect (what happens to pacing)
- rest_beats: extract rest beat type and tension_behavior — type values: relief, suspension, recharge; tension_behavior values: releases, holds, rebuilds
- composability: can_layer_with (list of shape names or types that combine well) and layer_type (what the combination produces)
- flavor_text: preserve the analytical prose explaining the shape's narrative logic — do not summarize

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
