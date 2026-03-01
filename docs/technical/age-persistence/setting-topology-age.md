# Setting Topology: AGE Persistence

## Purpose

This document maps the mathematical operations in [`setting-topology.md`](../graph-strategy/setting-topology.md) to persistence structures. The setting topology is the simplest and most stable of the four graphs — small, rarely written after authoring, and computationally trivial. Its primary value in AGE is enabling **cross-graph queries** that join settings with scenes.

**Prerequisites**: [README.md](README.md) for shared decisions, [`setting-topology.md`](../graph-strategy/setting-topology.md) for the mathematics.

**Storykeeper operations supported**: `get_adjacent_settings`, `get_shortest_path`, `get_reachable_settings`, `get_entity_proximity`, `get_scenes_at_reachable_settings` (from [`storykeeper-api-contract.md`](../storykeeper-api-contract.md)).

---

## Persistence Decision Matrix

| Data | Location | Rationale |
|------|----------|-----------|
| Setting identity (name, description) | PostgreSQL `settings` table | Already exists; single source of truth |
| Spatial data (affordances, sensory details) | PostgreSQL `settings.spatial_data` JSONB | Rich nested structure; not needed for traversal |
| Traversal cost triple (time, effort, risk) | AGE `:CONNECTS_TO` edge properties | Needed for Cypher adjacency queries |
| Directionality | AGE `:CONNECTS_TO` edge property | Needed for traversal direction filtering |
| Traversal conditions (TriggerPredicates) | PostgreSQL `setting_connections` table | Complex predicate structures; too large for AGE |
| Has-conditions flag | AGE `:CONNECTS_TO` edge property | Boolean; tells application whether to load conditions |
| Scene-at-setting link | AGE `:LOCATED_AT` edge | Cross-graph link enabling spatial + narrative queries |
| Entity locations (which entity is where) | PostgreSQL `entity_locations` table | Session-scoped; changes per-turn |

---

## AGE Schema

### Vertex: `:Setting`

```sql
SELECT * FROM cypher('storyteller', $$
    CREATE (:Setting {
        pg_id: '550e8400-e29b-41d4-a716-446655440010',
        story_id: '550e8400-e29b-41d4-a716-446655440000',
        name: 'The Rise',
        setting_type: 'exterior',
        accessibility: 'open'
    })
$$) AS (result agtype);
```

| Property | Type | Source | Notes |
|----------|------|--------|-------|
| `pg_id` | string (UUID) | `settings.id` | Identity anchor |
| `story_id` | string (UUID) | `settings.story_id` | Multi-story isolation |
| `name` | string | `settings.name` | Human-readable; Cypher debugging |
| `setting_type` | string | `settings.spatial_data` | `interior`, `exterior`, `liminal`, `abstract` |
| `accessibility` | string | Derived | `open`, `gated`, `conditional` — quick filter without loading full conditions |

**Index**:
```sql
CREATE INDEX ON storyteller."Setting" (properties ->> 'pg_id');
CREATE INDEX ON storyteller."Setting" (properties ->> 'story_id');
```

### Edge: `:CONNECTS_TO`

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:Setting {pg_id: $from_id}), (b:Setting {pg_id: $to_id})
    CREATE (a)-[:CONNECTS_TO {
        time_cost: 3.0,
        effort_cost: 0.6,
        risk_cost: 0.5,
        directionality: 'bidirectional',
        has_conditions: false
    }]->(b)
$$) AS (result agtype);
```

| Property | Type | Source | Notes |
|----------|------|--------|-------|
| `time_cost` | float | Authored | Narrative time units |
| `effort_cost` | float | Authored | Physical difficulty [0.0, 1.0] |
| `risk_cost` | float | Authored | Danger level [0.0, 1.0] |
| `directionality` | string | Authored | `bidirectional`, `one_way`, `conditional` |
| `has_conditions` | boolean | Derived | True if traversal requires truth-set evaluation |

For bidirectional connections, create **two** directed AGE edges (A→B and B→A) with the same cost properties. This keeps Cypher traversal patterns simple — all queries use directed edges.

### Cross-Graph Edge: `:LOCATED_AT`

This is the key cross-graph pattern — linking the narrative graph to the setting topology:

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (sc:Scene {pg_id: $scene_id}), (st:Setting {pg_id: $setting_id})
    CREATE (sc)-[:LOCATED_AT]->(st)
$$) AS (result agtype);
```

`:LOCATED_AT` has no properties — it is a pure structural link. It enables the cross-graph query "scenes at reachable settings" without application-level joins.

**Note**: This requires `:Scene` vertices to exist in AGE (they are created by the narrative graph schema, [narrative-graph-age.md](narrative-graph-age.md)). The migration for setting topology must run after the narrative graph migration.

---

## Cypher Queries for Key Operations

### 1. Adjacent Settings

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (s:Setting {pg_id: $setting_id})-[c:CONNECTS_TO]->(t:Setting)
    RETURN t.pg_id AS setting_id, t.name,
           c.time_cost, c.effort_cost, c.risk_cost,
           c.directionality, c.has_conditions
$$) AS (setting_id agtype, name agtype,
        time_cost agtype, effort_cost agtype, risk_cost agtype,
        directionality agtype, has_conditions agtype);
```

Expected latency: < 1ms.

### 2. Scenes at Reachable Settings (Cross-Graph Query)

The most valuable query — combines setting topology with narrative graph:

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (current:Setting {pg_id: $current_setting})-[:CONNECTS_TO*1..3]->(nearby:Setting)
    OPTIONAL MATCH (sc:Scene)-[:LOCATED_AT]->(nearby)
    RETURN nearby.pg_id AS setting_id, nearby.name AS setting_name,
           sc.pg_id AS scene_id, sc.scene_type, sc.authored_base
$$) AS (setting_id agtype, setting_name agtype,
        scene_id agtype, scene_type agtype, authored_base agtype);
```

This returns all settings within 3 hops and any scenes located at those settings. The result feeds the gravitational context assembly — "what narratively significant scenes are physically nearby?"

Expected latency: < 10ms (< 200 settings, sparse connectivity).

### 3. Setting Path Exists (Reachability Check)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH path = shortestPath(
        (a:Setting {pg_id: $from_id})-[:CONNECTS_TO*1..10]-(b:Setting {pg_id: $to_id})
    )
    RETURN length(path) AS hop_count
$$) AS (hop_count agtype);
```

Binary reachability check — does any path exist? The hop count is the unweighted distance. For weighted distance (entity-specific cost), use petgraph (see §5).

---

## New PostgreSQL Tables

### `setting_connections`

Full traversal conditions for gated edges:

```sql
CREATE TABLE setting_connections (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    from_setting_id UUID NOT NULL REFERENCES settings(id),
    to_setting_id   UUID NOT NULL REFERENCES settings(id),
    -- Conditions that must be satisfied for traversal
    forward_condition  JSONB,  -- TriggerPredicate for forward traversal
    reverse_condition  JSONB,  -- TriggerPredicate for reverse traversal (NULL if one-way)
    -- Narrative metadata
    description     TEXT,      -- "The path into the Shadowed Wood requires Kate's blessing"
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, from_setting_id, to_setting_id)
);

CREATE INDEX idx_setting_connections_story ON setting_connections(story_id);
CREATE INDEX idx_setting_connections_from ON setting_connections(from_setting_id);
```

**When loaded**: At session start (setting topology is stable). Conditions evaluated in-process against the truth set when a traversal is attempted.

### `entity_locations`

Session-scoped tracking of where entities are:

```sql
CREATE TABLE entity_locations (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id  UUID NOT NULL REFERENCES sessions(id),
    entity_id   UUID NOT NULL REFERENCES entities(id),
    setting_id  UUID NOT NULL REFERENCES settings(id),
    since_turn  INT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, entity_id)
);

CREATE INDEX idx_entity_locations_session ON entity_locations(session_id);
CREATE INDEX idx_entity_locations_setting ON entity_locations(session_id, setting_id);
```

**Purpose**: Enables entity proximity queries — "which entities are within budget-bounded distance of the current setting?" The proximity computation runs in application code (petgraph Dijkstra), but the entity location data comes from this table.

---

## Application-Level Computations (petgraph)

### Weighted Shortest Path (Dijkstra)

**Source math**: [`setting-topology.md`](../graph-strategy/setting-topology.md) §Weighted Shortest Path

AGE's `shortestPath()` is unweighted. The entity-specific cost function `effective_cost(base, entity)` with its (time, effort, risk) triple and per-entity capability modifiers requires in-process computation.

**Pattern**: Load the full setting topology into petgraph at session start (~50-200 nodes, < 0.1ms). Cache in-process for the session. Rerun Dijkstra per-query with entity-specific weights.

### Budget-Bounded Reachability

**Source math**: [`setting-topology.md`](../graph-strategy/setting-topology.md) §Budget-Bounded Reachability

Dijkstra with early termination at budget boundary. Uses the same in-process petgraph topology. Returns all settings reachable within cost budget for a specific entity.

### Entity Proximity

**Source math**: [`setting-topology.md`](../graph-strategy/setting-topology.md) §Entity Proximity

Cross-graph operation: reachability from petgraph + entity locations from PostgreSQL. Returns entities within budget-bounded distance.

```rust
fn entities_nearby(
    topology: &DiGraph<SettingVertex, SpatialEdge>,
    entity_locations: &HashMap<EntityId, SettingId>,
    center: SettingId,
    budget: f32,
    entity_capabilities: &EntityCapabilities,
) -> Vec<(EntityId, f32)> {
    let reachable = dijkstra_bounded(topology, center, budget, entity_capabilities);
    // Join reachable settings with entity locations
}
```

---

## Data Flow: Turn Cycle Integration

### Session Start

1. **Load full setting topology from AGE** (~2ms) — all `:Setting` vertices and `:CONNECTS_TO` edges
2. **Load setting conditions from PostgreSQL** (~1ms) — `setting_connections` for gated edges
3. **Build petgraph topology** (~0.1ms) — cached for session lifetime
4. **Load entity locations** (~1ms) — current positions of all tracked entities

Total: ~5ms (one-time per session)

### Scene Entry

1. **Compute reachable settings from current** (~0.1ms in petgraph) — budget-bounded Dijkstra
2. **Query scenes at reachable settings from AGE** (~5ms) — cross-graph Cypher query
3. **Compute entity proximity** (~0.1ms in petgraph) — which entities are nearby

Total: ~6ms

### Per-Turn

1. **Update entity locations if spatial change occurs** (~1ms) — PostgreSQL UPDATE on `entity_locations`
2. **Recompute proximity if location changed** (~0.1ms in petgraph)

Total: ~1ms (only when movement occurs; most turns don't change location)

### Scene Exit

No setting topology operations needed at scene exit.

---

## AGE Capability Assessment

| Requirement | AGE 1.7.0 Status | Notes |
|------------|------------------|-------|
| Basic traversal (MATCH, variable-length paths) | Supported | Adjacency and reachability |
| `shortestPath()` unweighted | Supported | Hop-count distance |
| Cross-label edge (`:LOCATED_AT` from `:Scene` to `:Setting`) | Needs Spike | Verify cross-label edge creation and traversal |
| `OPTIONAL MATCH` for left-join semantics | Supported | Settings without scenes at them |
| Property filters on edges | Supported | Directionality, has_conditions |
| Weighted shortest path | Not Supported | Use petgraph |

---

## Questions for TAS-244 Spike

1. **Cross-label edges**: Can we create a `:LOCATED_AT` edge from a `:Scene` vertex to a `:Setting` vertex? Does AGE allow edges between vertices of different labels in the same graph?
2. **Mixed-label path traversal**: Can a single Cypher query traverse `[:CONNECTS_TO*1..3]` and then follow `[:LOCATED_AT]` in one pattern? Or does this require two separate MATCH clauses?
3. **Bidirectional traversal**: Does `(a:Setting)-[:CONNECTS_TO*1..3]-(b:Setting)` (undirected pattern) work correctly with our approach of two directed edges per bidirectional connection?
4. **Performance at scale**: With 200 settings and ~500 connections, what is the latency for the cross-graph "scenes at reachable settings" query?
