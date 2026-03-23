You are a structured data extractor. Read the genre analysis below and produce a single JSON object.

## Field Extraction Guide

**genre_slug**: Use kebab-case from the title. "Folk Horror" → "folk-horror", "High Epic Fantasy" → "high-epic-fantasy".

**classification**: Almost all genres are "constraint_layer". Use "standalone_region" only if the document explicitly says the genre is an independent region. Use "hybrid_modifier" only if it explicitly modifies other genres without standing alone.

**Continuous axes** (aesthetic, tonal, temporal, agency, epistemological): Each axis has a value (0.0-1.0), low_label, high_label, and flavor_text.
- Extract the pole labels from "X ←→ Y" patterns. Example: "Grounded/mundane ←→ Heightened/mythic" → low_label: "Grounded/mundane", high_label: "Heightened/mythic"
- Map position words: "high" → 0.7-0.9, "moderate" → 0.4-0.6, "low" → 0.1-0.3
- flavor_text: Copy the *Note* text that explains the positioning. Preserve the original analytical prose.
- can_be_state_variable: true only if the document says this dimension shifts during play

**thematic dimensions** (weighted tags): Extract 1-3 treatments with weights 0.0-1.0.
- Example: "Power: Stewardship and Systemic Pressure" → {"Stewardship": 0.7, "Systemic Pressure": 0.3}

**locus_of_power**: Extract the ranked list from "Primary/Secondary/Tertiary" entries. Use lowercase values from: place, person, system, relationship, cosmos.
- Example: "Primary: Place (The Land). Secondary: System/Institution. Tertiary: Cosmos/Fate." → ["place", "system", "cosmos"]

**narrative_structure**: Extract from the "Narrative Structure" section. Use lowercase values from: quest, mystery, tragedy, comedy, romance, horror.

**world_affordances**: Extract magic (list of strings like "ambiguous", "rule_bound"), technology, violence, death, supernatural as strings.

**narrative_contracts**: Extract from "Genre Contracts" or similar section. Each is {invariant: string, enforced: boolean}.

**active_state_variables**: Extract named state variables with behavior type (depleting, accumulating, fluctuating, progression, countdown). Set initial_value and threshold as 0.0-1.0 when described.

**boundaries**: Extract genre drift triggers. Each has trigger, drift_target (genre slug), description.

**modifies**: List genre slugs this genre can layer onto, if described.

## Source Content

{raw_content}

## Target Schema

{schema}

Output a single valid JSON object. No markdown, no explanation.
