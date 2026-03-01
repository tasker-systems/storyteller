# Narrative Graph: AGE Persistence

## Purpose

This document maps the mathematical operations in [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md) to persistence structures. The narrative graph presents an interesting **duplication question** — the PostgreSQL `scenes` table already holds `narrative_mass` JSONB. This document resolves that boundary carefully: AGE stores only what is needed for graph traversal, PostgreSQL remains the authority for scene content.

**Prerequisites**: [README.md](README.md) for shared decisions, [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md) for the mathematics, [setting-topology-age.md](setting-topology-age.md) for the `:LOCATED_AT` cross-graph pattern.

**Storykeeper operations supported**: `get_reachable_scenes`, `compute_gravitational_pull`, `get_attractor_basins`, `get_approach_vector_satisfaction`, `get_effective_mass`, `get_scenes_by_gravitational_pull` (from [`storykeeper-api-contract.md`](../storykeeper-api-contract.md)).

---

## Persistence Decision Matrix

| Data | Location | Rationale |
|------|----------|-----------|
| Scene definition (title, cast, stakes, constraints) | PostgreSQL `scenes.scene_data` JSONB | Already exists; rich nested structure |
| Narrative mass (authored_base, structural_modifier, dynamic_adjustment) | PostgreSQL `scenes.narrative_mass` JSONB | Already exists; single source of truth |
| `authored_base` scalar (only) | AGE `:Scene` vertex property (denormalized) | One float needed for pull computation in graph traversal context |
| Scene type | AGE `:Scene` vertex property | Needed for Cypher type-filtering (gravitational, connective, gate, threshold) |
| Approach vectors | PostgreSQL `scenes.scene_data` | Complex predicate arrays; too structured for AGE |
| Departure trajectories | PostgreSQL `scene_transitions` table (new) | Runtime data about how the player departed; per-outcome transition metadata |
| Scene activation state | PostgreSQL `scene_activation_states` table (new) | Session-scoped; cannot live on AGE vertex |
| Scene transitions | AGE `:TRANSITIONS_TO` edges | Core graph structure for reachability |
| Scene-at-setting link | AGE `:LOCATED_AT` edge | Cross-graph (defined in [setting-topology-age.md](setting-topology-age.md)) |

**Critical decision**: The full `narrative_mass` JSONB is NOT duplicated on the AGE vertex. Only `authored_base` (a single float) is denormalized for pull approximation during Cypher traversal. The complete three-component mass computation (authored_base + structural_modifier + dynamic_adjustment) happens in application code, reading from PostgreSQL.

---

## AGE Schema

### Vertex: `:Scene`

```sql
SELECT * FROM cypher('storyteller', $$
    CREATE (:Scene {
        pg_id: '550e8400-e29b-41d4-a716-446655440020',
        story_id: '550e8400-e29b-41d4-a716-446655440000',
        title: 'Mothers Prayer',
        scene_type: 'gate',
        authored_base: 0.85,
        provenance: 'authored',
        layer_id: null
    })
$$) AS (result agtype);
```

| Property | Type | Source | Notes |
|----------|------|--------|-------|
| `pg_id` | string (UUID) | `scenes.id` | Identity anchor |
| `story_id` | string (UUID) | `scenes.story_id` | Multi-story isolation |
| `title` | string | `scenes.title` | Human-readable Cypher debugging |
| `scene_type` | string | `scenes.scene_type` | `gravitational`, `connective`, `gate`, `threshold` — for type-filtered queries |
| `authored_base` | float | `scenes.narrative_mass->>'authored_base'` | Single scalar for approximate pull ranking in Cypher |
| `provenance` | string | `scenes.provenance` | `authored`, `collaborative`, `generated` — origin tracking |
| `layer_id` | string (UUID) or null | `scenes.layer_id` | Sub-graph layer for boundary queries |

**Index**:
```sql
CREATE INDEX ON storyteller."Scene" (properties ->> 'pg_id');
CREATE INDEX ON storyteller."Scene" (properties ->> 'story_id');
CREATE INDEX ON storyteller."Scene" (properties ->> 'scene_type');
```

### Edge: `:TRANSITIONS_TO`

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:Scene {pg_id: $from_id}), (b:Scene {pg_id: $to_id})
    CREATE (a)-[:TRANSITIONS_TO {
        transition_weight: 0.8
    }]->(b)
$$) AS (result agtype);
```

| Property | Type | Source | Notes |
|----------|------|--------|-------|
| `transition_weight` | float | Authored | How strongly this transition pulls narrative forward (used for approximate ranking) |

The edge is intentionally minimal. The structural relationship — *scene A can lead to scene B* — is encoded by the edge's existence and direction. The `transition_weight` is the only property needed for Cypher-level ranking; all other transition data lives in PostgreSQL:

- **Departure type** (`DepartureType` in `storyteller-core`) is a runtime property — *how the player left* — not a structural property of the edge. It is recorded on the session's scene instance, not the graph edge.
- **Outcome labels**, departure conditions, approach effects, and momentum transfer live in the PostgreSQL `scene_transitions` table, which holds the rich authored metadata for each possible transition.

A scene with multiple possible next scenes has multiple outgoing `:TRANSITIONS_TO` edges. All edges exist structurally for reachability computation; the `scene_transitions` table in PostgreSQL describes the conditions under which each transition activates.

**Scene provenance**: The narrative graph includes both authored and runtime-generated scenes (connective tissue, pacing beats). All scenes are story-scoped regardless of provenance — generated scenes enrich the graph for all players. A `provenance` property on the `:Scene` vertex distinguishes `authored`, `collaborative`, and `generated` scenes for metadata and quality-tier purposes. Session-specific state (how a scene plays for a given player) lives in PostgreSQL's session-scoped tables, not on the AGE vertex. See [`scene-provenance.md`](../../game-design/scene-provenance.md) for the full design rationale.

---

## Cypher Queries for Key Operations

### 1. Reachable Scenes (N-Hop BFS)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (current:Scene {pg_id: $current_scene})-[:TRANSITIONS_TO*1..4]->(future:Scene)
    RETURN DISTINCT future.pg_id AS scene_id, future.title,
           future.scene_type, future.authored_base
$$) AS (scene_id agtype, title agtype, scene_type agtype, authored_base agtype);
```

Returns all scenes reachable within 4 transitions. The `authored_base` on the vertex enables approximate gravitational ranking without a PostgreSQL JOIN — the application sorts by `authored_base / distance²` for a quick approximation, then loads full mass data from PostgreSQL for the top candidates.

Expected latency: < 5ms for 200 scenes.

### 2. Gate Scenes Ahead

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH path = (current:Scene {pg_id: $current_scene})-[:TRANSITIONS_TO*1..6]->(gate:Scene)
    WHERE gate.scene_type = 'gate'
    RETURN DISTINCT gate.pg_id AS scene_id, gate.title,
           gate.authored_base, length(path) AS distance
    ORDER BY length(path)
$$) AS (scene_id agtype, title agtype, authored_base agtype, distance agtype);
```

Finds upcoming gate scenes — structural commitments that reshape the possibility space. Used by the gravitational context assembly to determine how close the player is to a narrative commitment point.

### 3. Transition Path Between Scenes

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH path = shortestPath(
        (a:Scene {pg_id: $from_scene})-[:TRANSITIONS_TO*1..10]->(b:Scene {pg_id: $to_scene})
    )
    RETURN [n IN nodes(path) | n.pg_id] AS scene_ids,
           length(path) AS hop_count
$$) AS (scene_ids agtype, hop_count agtype);
```

Finds the shortest transition path between two scenes. Used for narrative distance computation (the structural component of multi-dimensional distance).

### 4. Scenes in Sub-Graph Layer

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (s:Scene {story_id: $story_id, layer_id: $layer_id})
    RETURN s.pg_id AS scene_id, s.title, s.scene_type, s.authored_base
$$) AS (scene_id agtype, title agtype, scene_type agtype, authored_base agtype);
```

Returns all scenes in a specific narrative sub-graph layer. Used for sub-graph collective mass computation (from [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md)).

---

## New PostgreSQL Tables

### `scene_transitions`

Rich transition metadata that complements the structural AGE edges. Each row corresponds to one possible transition between scenes, with the authored conditions and effects that determine when and how it activates:

```sql
CREATE TABLE scene_transitions (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    from_scene_id   UUID NOT NULL REFERENCES scenes(id),
    to_scene_id     UUID NOT NULL REFERENCES scenes(id),
    outcome_label   TEXT NOT NULL,
    -- Departure conditions and effects
    departure_conditions JSONB,  -- What must be true for this transition to fire
    approach_effects     JSONB,  -- How this transition affects approach vector satisfaction
    momentum_transfer    JSONB,  -- What narrative momentum carries forward
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, from_scene_id, to_scene_id, outcome_label)
);

CREATE INDEX idx_scene_transitions_story ON scene_transitions(story_id);
CREATE INDEX idx_scene_transitions_from ON scene_transitions(from_scene_id);
CREATE INDEX idx_scene_transitions_to ON scene_transitions(to_scene_id);
```

### `scene_activation_states`

Session-scoped scene activation tracking:

```sql
CREATE TABLE scene_activation_states (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id  UUID NOT NULL REFERENCES sessions(id),
    scene_id    UUID NOT NULL REFERENCES scenes(id),
    -- Activation state
    state       TEXT NOT NULL DEFAULT 'dormant',  -- dormant, approaching, active, completed, bypassed
    -- Dynamic mass adjustment (session-specific)
    dynamic_adjustment REAL NOT NULL DEFAULT 0.0,
    -- Visit tracking for connective space diminishing returns
    visit_count INT NOT NULL DEFAULT 0,
    last_visited_turn INT,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, scene_id)
);

CREATE INDEX idx_scene_activation_session ON scene_activation_states(session_id);
CREATE INDEX idx_scene_activation_state ON scene_activation_states(session_id, state);
```

**Why not on AGE vertex**: AGE vertices are story-scoped, not session-scoped. Two players playing the same story would see each other's activation state. Session-specific data must live in PostgreSQL, keyed by `(session_id, scene_id)`.

**`dynamic_adjustment`**: The third component of effective mass (`approach_satisfaction × 0.2` from [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md)). This is session-specific because different players satisfy different approach predicates.

---

## Application-Level Computations

### Effective Mass Computation

**Source math**: [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md) §Effective Mass

All three components require data from different sources:

```rust
fn compute_effective_mass(
    scene_id: &SceneId,
    scenes_table: &ScenesRow,           // PostgreSQL: scenes.narrative_mass
    activation: &SceneActivationState,   // PostgreSQL: scene_activation_states
    event_dag: &EventDag,                // In-process: petgraph DAG
) -> f32 {
    let authored = scenes_table.narrative_mass.authored_base;
    let structural = compute_structural_modifier(scene_id, event_dag);
    let dynamic = activation.dynamic_adjustment;
    authored + structural + dynamic
}
```

Not possible in Cypher — requires cross-store data aggregation.

### Gravitational Pull

**Source math**: [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md) §Gravitational Pull

```
pull(from, to) = M_effective(to) / D(from, to)²
```

The four-dimensional narrative distance `D` requires:
- Information distance (from event DAG state — what the player knows)
- Emotional distance (from ML pipeline — psychological proximity)
- Relational distance (from relational web — social distance to cast)
- Physical distance (from setting topology — spatial proximity)

This is the quintessential cross-graph computation. AGE provides the reachable scene set; the application computes pull by assembling data from all four graphs.

### Attractor Basin Assignment

**Source math**: [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md) §Attractor Basins

For each reachable scene, find the attractor with maximum pull. This is a comparison operation on computed pull values — pure application code.

### Approach Vector Satisfaction

**Source math**: [`narrative-gravity.md`](../graph-strategy/narrative-gravity.md) §Approach Vector Satisfaction

Evaluate scene approach vectors (complex predicates) against the current truth set. Predicates are JSONB in the `scenes.scene_data` field; truth set is in-process state. Pure application code.

---

## Data Flow: Turn Cycle Integration

### Scene Entry

1. **Query reachable scenes from AGE** (~5ms) — N-hop BFS with `authored_base` for quick ranking
2. **Load full scene data from PostgreSQL** (~3ms) — `scenes` table for top-ranked reachable scenes (full `narrative_mass`, `scene_data`)
3. **Load activation states from PostgreSQL** (~2ms) — `scene_activation_states` for session
4. **Compute effective mass for reachable scenes** (~1ms) — application code combining three components
5. **Compute gravitational pull** (~2ms) — cross-graph distance + inverse-square pull
6. **Rank scenes by pull** — feeds context assembly

Total: ~15ms

### Per-Turn

1. **Recompute approach satisfaction if state changed** (~1ms) — update `dynamic_adjustment` in `scene_activation_states`
2. **No AGE queries per-turn** — narrative graph structure doesn't change during a scene

Total: ~1ms (only when approach predicates change)

### Scene Exit

1. **Update activation state** (~1ms) — mark current scene as `completed`, update visit_count
2. **Recompute structural modifier** (~1ms) — if events were resolved that affect gate status
3. **Generate runtime scenes if needed** — if the gravitational landscape reveals topological gaps (sparse connective tissue, pacing needs), the Storykeeper creates new `:Scene` vertices and `:TRANSITIONS_TO` edges in AGE. These are story-scoped and enrich the graph for all players. See [`scene-provenance.md`](../../game-design/scene-provenance.md).

Total: ~2ms (no generation); ~TBD when runtime scene generation occurs

---

## AGE Capability Assessment

| Requirement | AGE 1.7.0 Status | Notes |
|------------|------------------|-------|
| Variable-length directed paths `*1..N` | Supported | N-hop reachability |
| `shortestPath()` | Supported | Structural distance between scenes |
| Property filter on vertex (`scene_type`) | Supported | Gate scene queries |
| `DISTINCT` aggregation | Supported | De-duplicate multi-path reachable scenes |
| Path node extraction `[n IN nodes(path)]` | Needs Spike | Used for transition path queries |
| `ORDER BY` on computed expressions | Needs Spike | Sorting by distance |
| Cross-label traversal via `:LOCATED_AT` | Needs Spike | Depends on setting-topology-age.md validation |

---

## Provisional Decision: Story-Scoped Generation

Runtime-generated scenes are **story-scoped**, not session-scoped. When the system identifies a topological gap — insufficient connective tissue between gravitational attractors — the generated scene enriches the narrative graph for all players, not just the current session.

This is the correct scoping because the gap is structural: if there's sparse connectivity between scene A and the next attractor, every player traversing that region encounters the same problem. The generated vertex fills a graph-level need. What's session-specific — which characters are present, what emotional register plays, what relational threads are active — is already handled by the existing session-scoped machinery (`scene_activation_states`, scene instances, the turn-level event pipeline).

A `provenance` property on the `:Scene` vertex (`authored`, `collaborative`, `generated`) tracks origin for metadata, author visibility, and quality-tier signaling to the Narrator. No `session_id` on the vertex — no nullable property, no filter overhead on reachability queries. The queries stay exactly as written throughout this document.

Over time, the narrative graph grows richer as play generates connective tissue. This is a desirable property — the story's topology improves with use. A second-order analysis layer (outside the gameplay loop) can examine the graph for remaining sparse regions and generate further enrichment nodes.

See [`scene-provenance.md`](../../game-design/scene-provenance.md) for the full design rationale and open questions.

---

## Questions for TAS-244 Spike

1. **Variable-length path with DISTINCT**: Does `MATCH (a)-[:TRANSITIONS_TO*1..4]->(b) RETURN DISTINCT b` correctly de-duplicate when multiple paths lead to the same scene?
2. **Path node extraction**: Does `[n IN nodes(path) | n.pg_id]` work in AGE 1.7.0, or is list comprehension on paths unsupported?
3. **Performance at depth**: What is the latency for 6-hop reachability (worst case for gate scene search) with 200 scenes?
4. **authored_base denormalization**: When `scenes.narrative_mass` JSONB is updated in PostgreSQL, the AGE vertex property must also be updated. Validate the latency of a paired PostgreSQL UPDATE + Cypher SET in one transaction.
5. **Layer-scoped queries**: Does filtering by `layer_id` property on vertices efficiently reduce the search space, or does AGE scan all vertices regardless?
6. **Runtime vertex creation latency**: What is the cost of creating a new `:Scene` vertex + 2-3 `:TRANSITIONS_TO` edges within the scene-exit pipeline? This must fit within the scene boundary budget.
