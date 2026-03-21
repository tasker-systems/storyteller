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

## Event Ledger, Graph Evolution, and Narrative Gravity

The context assembly pipeline described above answers: *how does the system assemble what it knows at a single turn?* But there is a companion question: *how does the graph evolve turn by turn so that what the system knows reflects the accumulated weight of what has happened?*

This section addresses the event ledger's role, the rejection of trigger-based narrative events, and the gravity-based alternative.

### The Event Ledger as Ground-Truth Substrate

Per-turn event extraction produces entity-action-vector-entity records: "the Warden handed the Outsider a cup of tea during the gathering." This is the World Agent's domain — the closest thing the system has to an objective record. Not because objectivity is achievable (the extraction itself involves judgment), but because *some* agent must maintain "what actually happened" as distinct from how any character experienced, interpreted, or remembers it.

The event ledger is append-only and immutable. It is necessary but not sufficient — it captures the atomic events but not their narrative significance.

### Why Not Trigger-Based Narrative Events

The naive approach to narrative events is authored conditions: "IF trust < 0.3 AND secrets_known > 5 THEN trigger betrayal_event." This is the choose-your-own-adventure model — an author builds a decision tree, the engine traverses it. This system has evolved beyond that model for several reasons:

1. **Brittleness.** Authored conditions can't anticipate the emergent relational configurations that the system's multi-scale dynamics produce. A betrayal that arises from the accumulated erosion of an orbital dynamic through 40 turns of subtle scene-level friction isn't capturable as a boolean condition.

2. **Reductiveness.** Narrative events are not binary state transitions. A relationship doesn't flip from "loyal" to "betrayed." It moves through a region of narrative space where the genre's tropes and dynamics say "betrayal lives here" — and the *texture* of how the narrative enters that region is the story. A trigger collapses the texture into a switch.

3. **Wrong model of authorship.** The purpose of this engine is not to enable authors to build decision trees. It is to enable authors to create characters, genre-regions, archetypes, scene profiles of different gravitational mass, and relational configurations — and then let the engine generate the story dynamically. Authoring is about shaping the gravitational landscape, not scripting the paths through it. The oft-cited visualization: the author places metal balls of varying mass on a rubber sheet, creating basins of attraction. The story is the marble's path — influenced by gravity, not predetermined by track.

### Narrative Events as Emergent Threshold Crossings

The alternative is gravity-based. Narrative events are not triggered — they are *recognized* as the accumulated weight of turn-by-turn events shifts the relational graph into regions where the genre's narrative shapes, dynamics, and tropes say "this is where that kind of moment lives."

The mechanism:

**Each turn's events produce small perturbations.** The event ledger records the atomic event. The graph update step (post-commit, pre-next-turn) applies small deltas to the relevant edges and state variables:
- "The Outsider accepted food from the Warden" → integration_level += 3, trust_in_warden += 0.05, moral_position -= 0.02 (accepting hospitality from the people who will sacrifice you erodes your moral separateness)
- No single event is significant. The deltas are small. The significance is cumulative.

**The graph IS the accumulated state.** There is no separate "narrative event aggregation layer." The relational graph, with its multi-scale edge properties and state variables, *is* the aggregation. The current values of integration_level, trust, moral_position — these are the sum of all the small deltas from all the turns. The graph doesn't need a rules engine to tell it what state it's in. It *is* in that state.

**The Dramaturge recognizes patterns, not triggers.** The Dramaturge has the meta-view: the context packet (current graph state, narrative shape position, genre physics). It pattern-matches the current state against the narrative shape's beat descriptions. The narrative shapes data already describes what the graph should *feel like* at each beat:

- "The Participation" beat in the Descent into Belonging: "The protagonist helps to hide a victim. Integration is high. Moral standing is low. Escape routes are depleted."
- The Dramaturge doesn't check `integration_level > 70 AND moral_position < 0.3`. It receives the current state as part of its context packet and recognizes the *resemblance* to the beat's description.

This is pattern matching, not rules execution. And it's the kind of situated pattern matching that instruct models excel at — "does the current relational configuration resemble what this genre's narrative shape describes as the Participation beat?"

**Beat advancement is recognition, not triggering.** The narrative shape's beat sequence doesn't advance because a condition was met. It advances because the Dramaturge — looking at the graph state, the genre's narrative shapes, and the accumulated event history — recognizes that the story has arrived at a new beat. The Dramaturge's per-turn directive reflects this: *"The accumulated weight of the Outsider's compromises has reached the point where the Descent into Belonging shape says this is the Participation beat. The scene that's forming is not authored — it's the scene the gravity demands."*

### The Per-Turn Update Cycle

```
Turn N committed
  → Event extraction (entity-action-entity from player input + narrator output)
  → Event ledger append (immutable ground truth)
  → Graph update (shift edge weights, nudge state variables by small deltas)
  → [No explicit threshold check. No trigger evaluation.]

Turn N+1 begins
  → Storykeeper queries the graph as it now stands
  → Context packet assembled from current state
  → Dramaturge receives packet, recognizes narrative position
  → Dramaturge directive: "this is where we are in the shape;
      here is what the genre's gravity is pulling toward"
  → Narrator renders the turn
```

### What the Deltas Look Like

The graph update step applies deltas derived from the extracted events. These deltas are not authored per-event — they are computed from the *type* of action and the *genre's state variable interactions*:

- An action classified as "acceptance of hospitality from a threatening source" → delta to integration_level (positive), delta to moral_position (negative), delta to trust (positive but complicated)
- An action classified as "withholding information from an ally" → delta to trust on that edge (negative), delta to information_state (the secret grows heavier)
- An action classified as "physical labor in service of the community" → delta to social_standing (positive), delta to resource_pressure (negative — effort costs energy)

The classification of actions into types that interact with state variables is where the event extraction system and the genre's dynamics data connect. The dynamics data describes what kinds of actions produce what kinds of relational shifts. The event extraction identifies what kind of action this was. The delta computation maps one to the other.

**Critical nuance: events don't carry their own significance.** "The Warden handed the Outsider a cup of tea" is the same entity-action-entity regardless of whether it's genuine hospitality or preparation for sacrifice. The event-to-delta mapping cannot be purely mechanical — it requires contextual interpretation. A shove can be playful or malicious, and the relational deltas differ completely.

Two paths:

1. **Enhanced event extraction (preferred):** The instruct model doing entity-action-entity extraction already has scene context in its prompt. Extend its structured output to include a significance annotation: *"this action functions as [hospitality/obligation/deception/threat] in the current relational context"* and *"suggested state variable effects: integration_level +3, trust +0.05, moral_position -0.02."* This keeps interpretation in a single pass rather than adding another model call. The generated dynamics data provides the vocabulary of action-significance types and their typical state variable interactions per genre.

2. **Dramaturge post-processing (fallback):** If the event extraction model's contextual judgment proves insufficient, the Dramaturge — which already has the full context packet — could annotate events as a secondary function alongside its dramatic directive. This adds latency but brings the deepest contextual understanding to the interpretation. The Dramaturge would not re-extract events but would attach significance and delta recommendations to the extracted events.

The preferred path grounds the "why this event shifts the relational weighting by X amount" in the same model call that identifies the event, keeping the pipeline lean. The Dramaturge's dramatic directive can then reference the annotated events — "the tea was offered as preparation; the Outsider's acceptance deepens the Descent into Belonging" — rather than re-interpreting raw events from scratch.

### Authoring in the Gravity Model

If narrative events are not authored as triggers, what does the author do?

The author shapes the gravitational landscape:

1. **Characters** — archetype composition, personal variance, initial relational configuration. These are the masses on the rubber sheet.
2. **Genre region** — the dimensional position that determines the physics. Which state variables exist, what the narrative shapes look like, what the ontological posture is. This is the curvature of the sheet itself.
3. **Scene profiles** — dramatic situations with gravitational mass. High-gravity scenes (the confrontation, the ritual, the revelation) are the deep basins. The author decides where they sit in narrative space and how massive they are.
4. **Initial relational web** — the starting configuration of edges. Which dynamics are orbital (pre-existing), which are available for arc-level development, what the initial state variable values are.

The author does *not* specify: "when X happens, do Y." The author specifies the landscape. The story is the path the marble takes through it — shaped by gravity, inflected by the player's choices, rendered by the narrator.

Multiple players simply means multiple marbles on the same sheet, each experiencing the gravitational field from their own position, each contributing perturbations that shift the field for everyone else. The context assembly pipeline handles this naturally — each player-character gets their own context packet assembled from their own relational graph position.

### What Changes from Current Architecture

1. **Event extraction stays** — entity-action-entity per turn remains the grounding data
2. **Event-to-delta mapping is new** — a lookup from action classification + genre dynamics data to state variable deltas
3. **Graph update cycle is new** — post-commit delta application to relational graph edges and state variables
4. **Narrative event triggers are removed** — no authored conditions, no state-machine transitions
5. **Dramaturge beat recognition is new** — pattern matching current state against narrative shape beat descriptions
6. **Authoring model shifts** — from scripting events to shaping gravitational landscapes

---

## Relationship to Other Documents

- **Character Tensor Evolution** (`character-tensor-evolution.md`) — deprecated as implementation path but retained for conceptual insights. The multi-scale framing, genre-as-transform, archetype composition, and geological layers from that document are operationalized here as data structures and query patterns.
- **Axis Inventory and Type Design** (`storyteller-data/narrative-data/analysis/2026-03-19-axis-inventory-and-type-design.md`) — the 30 universal dimensions, 8 canonical state variables, and archetype personality axes from this analysis define the schema for the core tables.
- **Data-Driven Narrative Elicitation** (`docs/foundation/data_driven_narrative_elicitation.md`) — the methodology that produced the corpus this system queries.
- **Beings and Boundaries** (`docs/foundation/beings_and_boundaries.md`) — the ontological posture data that informs how nonhuman entities participate in the relational graph and context assembly.
- **Storykeeper API Contract** (`docs/technical/storykeeper-api-contract.md`) — the existing Storykeeper design. This document extends it with the context assembly pipeline.

---

*The richness is in the data. The selection is in the graph. The synthesis is where the LLM earns its keep.*
