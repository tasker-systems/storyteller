# Storykeeper Context Assembly: From Rich Data to Situated Character Guidance

**Date:** 2026-03-21
**Status:** Active design — supersedes the ML prediction pipeline in `character-tensor-evolution.md`
**Context:** The narrative data corpus (~4.8MB) provides genre-grounded reference material of exceptional qualitative richness. Rather than compressing this into numeric tensors for ML prediction, we let the richness stay rich and use deterministic database queries and graph traversals to select the relevant slices for each turn's context assembly.

---

## The Architectural Insight

The character tensor evolution document (its sibling) identified a genuine need: a character's behavioral guidance must be grounded in genre context, archetype positioning, multi-scale dynamics, narrative beat position, and goal-scale alignment. The question was whether that grounding should happen through an ML model (compress rich data → numeric tensor → predict orientation → re-expand to text) or through selection (retrieve the right rich data → present to a model that can synthesize directly).

The selection path wins for three reasons:

1. **No information compression.** The generated data is already in the form that LLMs can work with — rich, contextual, qualitative descriptions grounded in genre axes. Compressing it into numeric representations and then translating back to text loses signal.

2. **No training data bootstrapping problem.** An ML model needs ground-truth labels ("the correct intentional orientation for the Earnest Warden at integration level 60 during the Revelation beat"). That's a creative judgment, not a factual one. If an LLM generates the training labels, we're training an ML model to approximate what the LLM already does — but faster and with less context. The latency savings don't justify the signal degradation.

3. **The selection problem is database-hard, not AI-hard.** Choosing *which* archetype data, dynamics, goals, and narrative beat information to include in context assembly is a graph traversal and filtering problem — exactly what PostgreSQL + Apache AGE is designed for. The richness is in the data. The selection is in the graph structure and current state. The synthesis is where the LLM earns its keep.

### What Survives from the Tensor Evolution

The conceptual contributions of the tensor evolution document are operationalized here as *data structures and query patterns*, not as ML model inputs:

| Tensor Evolution Concept | Operationalized As |
|---|---|
| Multi-scale framing (orbital/arc/scene) | Edge properties on relational graph; goal-tracking table with scale column |
| Genre-as-dynamic-transform | Genre variant records with shifted axes, tensions, and constraint rules stored as structured data |
| One primary + one shadow archetype | Character record with `primary_archetype_id` and `shadow_archetype_id` FKs |
| Geological layers (bedrock/sediment/topsoil) | Three-tier goal tracking; multi-scale dynamic edges; character tensor with temporal layers |
| Dimensional heterogeneity | Genre-specific axes stored as flexible JSONB alongside core axes as typed columns |
| Cross-scale tension | Queryable: scene goal + arc goal + existential goal retrieved together; tension is visible to the assembler |

---

## The Turn-Loop Context Assembly Pipeline

At each turn, the Storykeeper assembles a context packet through deterministic queries. No ML inference. No model calls. The output is a structured document (~300-600 tokens) that the dramaturge and narrator receive as part of their context window.

### Step 1: Static Context (cached per scene, not per turn)

These don't change within a scene and can be cached:

```sql
-- Genre dimensions (one row, set at story init)
SELECT * FROM genre_dimensions WHERE story_id = $1;

-- Active narrative shape(s) for this scene
SELECT ns.* FROM narrative_shapes ns
  JOIN scene_state ss ON ss.active_shape_id = ns.id
  WHERE ss.scene_id = $2;

-- Scene profile (what kind of dramatic situation this is)
SELECT * FROM scene_profiles WHERE scene_id = $2;

-- Ontological posture (genre's stance toward beings)
SELECT * FROM ontological_postures WHERE genre_slug = $3;
```

### Step 2: Character Context (per character, per turn)

```sql
-- Character record with archetype composition
SELECT c.*,
       pa.canonical_name AS primary_archetype,
       pa.genre_variant_name AS primary_variant,
       pa.axis_positions AS primary_axes,
       pa.distinguishing_tension AS primary_tension,
       sa.canonical_name AS shadow_archetype,
       sa.genre_variant_name AS shadow_variant,
       sa.distinguishing_tension AS shadow_tension
FROM characters c
  JOIN archetype_in_genre pa ON c.primary_archetype_id = pa.id
  LEFT JOIN archetype_in_genre sa ON c.shadow_archetype_id = sa.id
WHERE c.id = $1;

-- Character's goals at all three scales
SELECT scale, goal_name, goal_description, pursuit_state, tension_notes
FROM character_goals
WHERE character_id = $1
ORDER BY scale;  -- existential, arc, scene

-- Character's current state variables
SELECT sv.canonical_id, sv.genre_label, sv.value, sv.behavior
FROM character_state_variables sv
WHERE sv.character_id = $1;
```

### Step 3: Relational Context (graph traversal)

```cypher
-- Active dynamics for this character (bounded depth)
MATCH (c:Character {id: $1})-[d:DYNAMIC]->(other)
WHERE d.active = true
RETURN d.scale, d.name, d.description, d.edge_type,
       d.current_state, other.name, other.entity_type
ORDER BY CASE d.scale
  WHEN 'orbital' THEN 1
  WHEN 'arc' THEN 2
  WHEN 'scene' THEN 3
END
LIMIT 8;

-- Dynamics involving nonhuman agents (setting, system, etc.)
MATCH (c:Character {id: $1})-[d:DYNAMIC]->(e:Entity)
WHERE e.entity_type <> 'character' AND d.active = true
RETURN d.scale, d.name, d.description, e.name, e.entity_type;
```

### Step 4: Narrative Position Context

```sql
-- Current beat in the active narrative shape
SELECT beat_name, position, tension_contribution, pacing_function,
       is_load_bearing, rest_rhythm_description
FROM narrative_beats nb
  JOIN scene_state ss ON ss.active_shape_id = nb.shape_id
WHERE ss.scene_id = $2
  AND nb.position_order = ss.current_beat_index;

-- Active tropes in this scene
SELECT trope_name, narrative_function, state_variable_interaction
FROM active_tropes
WHERE scene_id = $2;
```

### Step 5: Assembly

The Storykeeper formats the query results into a structured context packet. This is string templating, not AI:

```
CHARACTER: {variant_name} ({archetype_family})
  Shadow undertow: {shadow_variant}
  {primary_tension}
  Current state: {state_variables formatted}

DYNAMICS:
  [Orbital] {dynamic_name} — {description}
    {edge_type}, {current_state}
  [Arc] {dynamic_name} — {description}
  [Scene] {dynamic_name} — {description}

NARRATIVE POSITION:
  Shape: {shape_name}, Beat: {beat_name} ({position})
  Tension: {tension_contribution}. Pacing: {pacing_function}.
  Rest rhythm: "{rest_rhythm_description}"
  Active tropes: {trope_names with functions}

GOALS:
  [Existential] {goal} — {pursuit_state}
  [Arc] {goal} — {pursuit_state}
  [Scene] {goal} — {pursuit_state}
  Cross-scale tension: {tension_notes}

GENRE PHYSICS:
  Locus of power: {ranked list}
  Knowledge stance: {knowledge_treatment}
  Ontological posture: {summary — who counts, boundary stability}
```

This packet is ~300-600 tokens depending on the number of active dynamics and tropes. It is deterministic, inspectable, and reproducible for the same game state.

---

## Where the LLM Models Engage

The context packet feeds two consumers:

### The Dramaturge (async, per-turn)

The Dramaturge receives the context packet and produces a **dramatic directive** — a short natural language instruction that shapes how the narrator should handle this turn. It runs asynchronously on a small instruct model:

```
Given this character's situated context, what is the dramatic
quality of this moment? What should the narrator attend to?
What undertow should be felt but not named?
```

The directive might be: *"The Warden's warmth is genuine but the gathering has the quality of a last meal. Let the food be described with care — the precision is the horror. The Outsider's scene goal (stay calm) is visibly succeeding, which is what makes the arc goal (the ritual) feel inevitable."*

This replaces the ML prediction → intent synthesis chain with a single, context-rich model call that can engage with the full qualitative nuance.

### The Narrator (synchronous, per-turn)

The Narrator receives:
- The dramaturge directive (if available; graceful degradation if not)
- The character context packet
- The scene journal (recent turn history)
- Player input

And renders the turn. The narrator doesn't need to *compute* the character's behavioral orientation — it receives it as structured context. The narrator's job is to *voice* it in genre-appropriate prose.

---

## Latency Budget

| Step | Type | Latency | Blocking? |
|------|------|---------|-----------|
| Event extraction (structured LLM) | instruct model call | ~2-3s | Yes (committed turn phase) |
| Storykeeper context assembly | database queries | ~10-50ms | Yes |
| Dramaturge directive | instruct model call | ~2-4s | No (async, previous directive used if not ready) |
| ML character prediction | inference | ~~200-500ms~~ | ~~Yes~~ **Eliminated** |
| Intent synthesis | instruct model call | ~~2-3s~~ | ~~Yes~~ **Eliminated** |
| Narrator generation | LLM call | ~5-10s | Yes (streaming to player) |

**Net effect:** Removing the ML prediction and intent synthesis steps saves ~2-4 seconds per turn and eliminates two potential failure modes from the critical path. The dramaturge's async directive absorbs the work that intent synthesis was doing, but without blocking the turn loop.

---

## Data Persistence Model

The generated narrative data (~4.8MB of raw.md) needs to be persisted in a queryable form. The Stage 2 JSON structuring pipeline (qwen2.5:7b-instruct) extracts structured records from the rich markdown. These structured records populate the database tables that the Storykeeper queries.

### Core Tables

```
genre_dimensions          — 30 universal axes per genre (typed columns)
                            + genre-specific axes (JSONB)
archetype_in_genre        — canonical name, variant name, axis positions,
                            tensions, genre shift data
ontological_postures      — per-genre stance toward beings, boundary
                            stability, modes of being
scene_profiles            — dramatic situation types with tension signatures
narrative_shapes          — per-genre arc patterns with beat sequences
narrative_beats           — individual beats within shapes (position,
                            flexibility, tension, pacing, rest rhythm)
tropes                    — per-genre narrative devices with function tags
                            and state variable interactions
dynamics                  — multi-scale relational patterns with edge
                            properties and evolution patterns
goals                     — multi-scale character motivations with
                            cross-scale tension data
```

### Flavor Text Tables

```
axis_flavor_text          — qualitative descriptions per axis per genre
                            (FK to genre_dimensions)
archetype_flavor_text     — rich descriptions per archetype per genre
                            (FK to archetype_in_genre)
state_variable_flavor     — genre-specific descriptions of what state
                            variable means narratively (FK to genre)
```

### Runtime State Tables

```
characters                — primary_archetype_id, shadow_archetype_id,
                            personal variance, current tensor state
character_goals           — scale (existential/arc/scene), current state
character_state_variables — canonical_id, genre_label, current value
scene_state               — active_shape_id, current_beat_index
active_tropes             — which tropes are in play for this scene
event_ledger              — append-only log (existing)
```

### Graph (Apache AGE)

```
(:Character)-[:DYNAMIC {scale, name, type, state}]->(:Entity)
(:Scene)-[:CONNECTS {mass, approach_vector}]->(:Scene)
(:Place)-[:ADJACENT {friction, permeability, tonal_shift}]->(:Place)
```

---

## The Storykeeper's Contract

The Storykeeper is a deterministic information boundary manager. Its contract:

1. **Queries are bounded.** No unbounded recursion, no full-graph scans. Every query has a depth limit or a result limit. The event ledger CTE-DAG queries have explicit recursion depth. The relational graph traversals have explicit hop limits.

2. **Results are structured.** The Storykeeper returns typed records, not free text. The context assembly step formats them into the packet, but the formatting is string templating, not interpretation.

3. **Information boundaries are enforced.** The Narrator receives only what the Storykeeper releases for this character in this scene. Character Agents receive only their own tensor and the edges they can see. The Dramaturge receives the full context packet (it needs the meta-view). The World Agent receives environmental state and ontological posture.

4. **Missing data degrades gracefully.** If a character has no shadow archetype, that section is omitted. If the dramaturge directive hasn't arrived, the previous turn's directive is used. If a dynamic has no scene-level state (only orbital), the orbital description is used. The packet is always assemblable.

5. **The Storykeeper never interprets.** It selects and routes. The *meaning* of the data is for the Dramaturge and Narrator to synthesize. The Storykeeper's job is to ensure they receive the right data, not to tell them what it means.

---

## Relationship to Other Documents

- **Character Tensor Evolution** (`character-tensor-evolution.md`) — deprecated as implementation path but retained for conceptual insights. The multi-scale framing, genre-as-transform, archetype composition, and geological layers from that document are operationalized here as data structures and query patterns.
- **Axis Inventory and Type Design** (`storyteller-data/narrative-data/analysis/2026-03-19-axis-inventory-and-type-design.md`) — the 30 universal dimensions, 8 canonical state variables, and archetype personality axes from this analysis define the schema for the core tables.
- **Data-Driven Narrative Elicitation** (`docs/foundation/data_driven_narrative_elicitation.md`) — the methodology that produced the corpus this system queries.
- **Beings and Boundaries** (`docs/foundation/beings_and_boundaries.md`) — the ontological posture data that informs how nonhuman entities participate in the relational graph and context assembly.
- **Storykeeper API Contract** (`docs/technical/storykeeper-api-contract.md`) — the existing Storykeeper design. This document extends it with the context assembly pipeline.

---

*The richness is in the data. The selection is in the graph. The synthesis is where the LLM earns its keep.*
