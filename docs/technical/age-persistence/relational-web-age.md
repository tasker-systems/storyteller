# Relational Web: AGE Persistence

## Purpose

This document maps the mathematical operations in [`relational-web-math.md`](../graph-strategy/relational-web-math.md) to concrete persistence structures. The relational web is the most complex of the four graphs — richest edge structure, highest per-turn write frequency, and the most structural computation.

**Prerequisites**: [README.md](README.md) for shared decisions, [`relational-web-math.md`](../graph-strategy/relational-web-math.md) for the mathematics.

**Storykeeper operations supported**: `get_relationship`, `get_cast_subgraph`, `get_social_distance`, `detect_tension_pairs`, `get_topological_role`, `get_clusters`, `propagate_cascade` (from [`storykeeper-api-contract.md`](../storykeeper-api-contract.md)).

---

## Persistence Decision Matrix

| Data | Location | Rationale |
|------|----------|-----------|
| Entity identity (name, origin, tier, persistence_mode) | PostgreSQL `entities` table | Already exists; single source of truth |
| Substrate vector (5 floats) | AGE `:RELATES_TO` edge properties | Needed for Cypher `WHERE` filters (tension detection, trust thresholds) |
| History depth | AGE `:RELATES_TO` edge property | Needed for permeability computation in Cypher path queries |
| Opacity | AGE `:RELATES_TO` edge property | Needed for permeability filtering |
| Projection (A's model of B) | PostgreSQL `relational_edge_details` | Complex nested structure; too large for AGE edge properties |
| Information state (what A knows about B) | PostgreSQL `relational_edge_details` | Session-scoped, complex; not a graph traversal concern |
| History log (temporal substrate changes) | PostgreSQL `relational_edge_history` | Append-only; not needed for graph queries |
| Topological role (Gate/Bridge/Hub/Periphery) | AGE `:RelEntity` vertex property (cached) | Computed by petgraph, cached for Cypher access |
| Cluster ID | AGE `:RelEntity` vertex property (cached) | Computed by petgraph, cached for Cypher access |
| Relational weight (summary) | PostgreSQL `entities.relational_weight` | Already exists; materialized from event ledger |

**Principle**: AGE edges carry the substrate floats and the two scalars needed for friction computation (history_depth, opacity). Everything else stays in PostgreSQL. The AGE edge is a **traversal index**, not a full relationship record.

---

## AGE Schema

### Vertex: `:RelEntity`

```sql
-- Cypher for vertex creation
SELECT * FROM cypher('storyteller', $$
    CREATE (:RelEntity {
        pg_id: '550e8400-e29b-41d4-a716-446655440001',
        story_id: '550e8400-e29b-41d4-a716-446655440000',
        name: 'Sarah',
        promotion_tier: 'tracked',
        topological_role: 'periphery',
        cluster_id: 0,
        layer_id: null
    })
$$) AS (result agtype);
```

| Property | Type | Source | Notes |
|----------|------|--------|-------|
| `pg_id` | string (UUID) | `entities.id` | Identity anchor; JOIN key |
| `story_id` | string (UUID) | `entities.story_id` | Multi-story isolation filter |
| `name` | string | `entities.name` | For human-readable Cypher debugging |
| `promotion_tier` | string | `entities.promotion_tier` | Filter: only Tracked+ entities have vertices |
| `topological_role` | string | Computed (petgraph) | `gate`, `bridge`, `hub`, `periphery` — cached after structural recomputation |
| `cluster_id` | integer | Computed (petgraph) | Label propagation cluster assignment — cached |
| `layer_id` | string (UUID) or null | `entities.layer_id` | Sub-graph layer for boundary filtering |

**Index**: Create an AGE index on `pg_id` for efficient vertex lookup:
```sql
CREATE INDEX ON storyteller."RelEntity" (properties ->> 'pg_id');
CREATE INDEX ON storyteller."RelEntity" (properties ->> 'story_id');
```

### Edge: `:RELATES_TO`

```sql
-- Cypher for edge creation
SELECT * FROM cypher('storyteller', $$
    MATCH (a:RelEntity {pg_id: $from_id}), (b:RelEntity {pg_id: $to_id})
    CREATE (a)-[:RELATES_TO {
        trust_reliability: 0.6,
        trust_competence: 0.8,
        trust_benevolence: 0.9,
        affection: 0.95,
        debt: 0.3,
        history_depth: 0.9,
        opacity: 0.0,
        substrate_magnitude: 1.52,
        updated_at_turn: 0
    }]->(b)
$$) AS (result agtype);
```

| Property | Type | Source | Notes |
|----------|------|--------|-------|
| `trust_reliability` | float | Authored / cascade | ℝ⁵ substrate component |
| `trust_competence` | float | Authored / cascade | ℝ⁵ substrate component |
| `trust_benevolence` | float | Authored / cascade | ℝ⁵ substrate component |
| `affection` | float | Authored / cascade | ℝ⁵ substrate component |
| `debt` | float | Authored / cascade | ℝ⁵ substrate component; range [-1.0, 1.0] |
| `history_depth` | float | Authored / accumulated | For permeability computation; range [0.0, 1.0] |
| `opacity` | float | Authored / narrative control | For permeability; 0.0 = transparent, 1.0 = opaque |
| `substrate_magnitude` | float | Computed | `‖R‖` — cached for quick intensity checks |
| `updated_at_turn` | integer | System | Turn number when edge was last modified; monotonic |

**8 properties per edge** — well within AGE's per-edge property limits. At 500 edges, this is ~4,000 property values total.

**Index**: For tension detection queries:
```sql
CREATE INDEX ON storyteller."RELATES_TO" (properties ->> 'trust_benevolence');
CREATE INDEX ON storyteller."RELATES_TO" (properties ->> 'debt');
```

---

## Cypher Queries for Key Operations

### 1. Get Relationship (Direct Edge Lookup)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:RelEntity {pg_id: $from_id})-[r:RELATES_TO]->(b:RelEntity {pg_id: $to_id})
    RETURN r.trust_reliability, r.trust_competence, r.trust_benevolence,
           r.affection, r.debt, r.history_depth, r.opacity
$$) AS (trust_rel agtype, trust_comp agtype, trust_ben agtype,
        affection agtype, debt agtype, history agtype, opacity agtype);
```

Expected latency: < 1ms.

### 2. Get Cast Subgraph (Scene Entry Bulk Load)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:RelEntity)-[r:RELATES_TO]->(b:RelEntity)
    WHERE a.pg_id IN $cast_ids AND b.pg_id IN $cast_ids
    RETURN a.pg_id AS from_id, b.pg_id AS to_id,
           r.trust_reliability, r.trust_competence, r.trust_benevolence,
           r.affection, r.debt, r.history_depth, r.opacity,
           a.topological_role AS from_role, b.topological_role AS to_role
$$) AS (from_id agtype, to_id agtype,
        trust_rel agtype, trust_comp agtype, trust_ben agtype,
        affection agtype, debt agtype, history agtype, opacity agtype,
        from_role agtype, to_role agtype);
```

This returns all directed edges between cast members in a single query. For a cast of 6 (TFATD), this returns ~20 edges. For a cast of 8 (large scene), ~40-50 edges.

Expected latency: < 5ms.

### 3. Social Distance (Shortest Path)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH path = shortestPath(
        (a:RelEntity {pg_id: $from_id})-[:RELATES_TO*1..4]-(b:RelEntity {pg_id: $to_id})
    )
    RETURN length(path) AS distance
$$) AS (distance agtype);
```

**Limitation**: AGE's `shortestPath` computes unweighted shortest path (hop count). For the permeability-weighted "strongest signal" path from [`traversal-friction.md`](../graph-strategy/traversal-friction.md), use application code (see §5).

Expected latency: < 5ms.

### 4. Tension Detection (Substrate Filter)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:RelEntity {story_id: $story_id})-[r:RELATES_TO]->(b:RelEntity)
    WHERE r.trust_benevolence < 0.3 OR r.debt > 0.7
        OR (r.trust_competence < 0.3 AND r.affection > 0.7)
    RETURN a.pg_id AS from_id, b.pg_id AS to_id,
           r.trust_benevolence, r.debt, r.trust_competence, r.affection
$$) AS (from_id agtype, to_id agtype,
        trust_ben agtype, debt agtype, trust_comp agtype, affection agtype);
```

This finds edges with narratively significant tension patterns — low trust with high affection (cognitive dissonance), high debt (obligation pressure), or low benevolence (active distrust).

Expected latency: < 5ms for 500 edges.

### 5. Neighborhood (All Edges for an Entity)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:RelEntity {pg_id: $entity_id})-[r:RELATES_TO]-(b:RelEntity)
    RETURN a.pg_id AS from_id, b.pg_id AS to_id,
           r.trust_reliability, r.trust_competence, r.trust_benevolence,
           r.affection, r.debt,
           startNode(r) = a AS is_outgoing
$$) AS (from_id agtype, to_id agtype,
        trust_rel agtype, trust_comp agtype, trust_ben agtype,
        affection agtype, debt agtype, is_outgoing agtype);
```

Returns both outgoing and incoming edges, with direction flag. Used for degree centrality inputs and entity context display.

### 6. Edge Update (Per-Turn Substrate Modification)

```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (a:RelEntity {pg_id: $from_id})-[r:RELATES_TO]->(b:RelEntity {pg_id: $to_id})
    SET r.trust_benevolence = $new_trust_ben,
        r.substrate_magnitude = $new_magnitude,
        r.updated_at_turn = $turn_number
    RETURN r
$$) AS (result agtype);
```

This is the per-turn write path. A single edge update after cascade resolution. Multiple edges may be updated in one turn, but each as a separate Cypher statement within the same PostgreSQL transaction.

Expected latency: < 1ms per edge.

---

## New PostgreSQL Tables

### `relational_edge_details`

Rich edge data that exceeds what AGE edge properties should carry:

```sql
CREATE TABLE relational_edge_details (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    from_entity_id  UUID NOT NULL REFERENCES entities(id),
    to_entity_id    UUID NOT NULL REFERENCES entities(id),
    -- Projection: A's model of B's current state
    projection      JSONB,
    -- Information state: what A knows/believes about B
    information_state JSONB,
    -- Configuration: narrative designer overrides
    configuration   JSONB,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, from_entity_id, to_entity_id)
);

CREATE INDEX idx_rel_edge_details_story ON relational_edge_details(story_id);
CREATE INDEX idx_rel_edge_details_from ON relational_edge_details(from_entity_id);
CREATE INDEX idx_rel_edge_details_to ON relational_edge_details(to_entity_id);
```

**When loaded**: At scene entry, alongside the AGE cast subgraph. Only edges for cast members are loaded.

**When written**: At turn commitment, when cascade resolution affects projection or information state.

### `relational_edge_history`

Append-only log of substrate changes for temporal analysis:

```sql
CREATE TABLE relational_edge_history (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    session_id      UUID NOT NULL REFERENCES sessions(id),
    from_entity_id  UUID NOT NULL REFERENCES entities(id),
    to_entity_id    UUID NOT NULL REFERENCES entities(id),
    turn_id         UUID NOT NULL REFERENCES turns(id),
    -- What changed
    dimension       TEXT NOT NULL,  -- 'trust_benevolence', 'affection', etc.
    old_value       REAL NOT NULL,
    new_value       REAL NOT NULL,
    delta           REAL NOT NULL,
    -- Why it changed
    cause           TEXT NOT NULL,  -- 'direct_event', 'cascade', 'scene_resolution'
    source_event_id UUID REFERENCES event_ledger(id),
    committed_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rel_edge_history_story ON relational_edge_history(story_id);
CREATE INDEX idx_rel_edge_history_edge ON relational_edge_history(from_entity_id, to_entity_id, committed_at);
CREATE INDEX idx_rel_edge_history_turn ON relational_edge_history(turn_id);
```

**Purpose**: Enables "how did Sarah's trust in Adam change over the story?" queries. Also provides the data for the `history_depth` computation — depth is derived from the density and recency of history entries.

---

## Application-Level Computations (petgraph)

These algorithms run in-process, not in Cypher. The pattern: load data from AGE, build a petgraph, run the algorithm, cache results back.

### Load Pattern: AGE → petgraph

```rust
/// Load the relational web for a story into a petgraph DiGraph
async fn load_web_into_petgraph(
    pool: &PgPool,
    story_id: &Uuid,
) -> Result<(DiGraph<RelVertex, RelEdge>, HashMap<Uuid, NodeIndex>)> {
    // 1. Query all RelEntity vertices and RELATES_TO edges from AGE
    // 2. Build petgraph DiGraph with NodeIndex for each vertex
    // 3. Return graph + entity_id → NodeIndex mapping
}
```

### Brandes' Betweenness Centrality

**Source math**: [`relational-web-math.md`](../graph-strategy/relational-web-math.md) §Topological Role Classification

**Reads from**: AGE cast subgraph (or full story web at scene boundary)

**Writes to**: AGE `:RelEntity.topological_role` vertex property

```rust
fn compute_topological_roles(graph: &DiGraph<RelVertex, RelEdge>) -> HashMap<NodeIndex, TopologicalRole> {
    let betweenness = brandes_betweenness(graph);
    let articulation_points = tarjan_articulation(graph);

    for node in graph.node_indices() {
        let role = classify_role(
            betweenness[&node],
            articulation_points.contains(&node),
            graph.neighbors(node).count(),
        );
        // Cache back to AGE vertex property
    }
}
```

**When**: Scene boundary (entry or exit). Not per-turn — structural metrics are stable within a scene.

### Label Propagation Clustering

**Source math**: [`relational-web-math.md`](../graph-strategy/relational-web-math.md) §Cluster Detection

**Reads from**: AGE full story web

**Writes to**: AGE `:RelEntity.cluster_id` vertex property

**When**: Scene boundary. Cluster assignments change infrequently.

### Cascade Propagation (Strongest Signal)

**Source math**: [`traversal-friction.md`](../graph-strategy/traversal-friction.md) §Multi-Path Resolution

**Reads from**: AGE edge properties (substrate for permeability computation)

**Writes to**: AGE edge properties (updated substrate after cascade), PostgreSQL `relational_edge_history` (changelog)

This is the primary per-turn computation. The modified Dijkstra that maximizes signal strength along paths requires permeability computation at each edge — a three-component formula (trust × history × (1-opacity)) that AGE cannot compute natively.

```rust
fn propagate_cascade(
    web: &DiGraph<RelVertex, RelEdge>,
    source: NodeIndex,
    signal: RelationalSignal,
    friction_factor: f32,
    significance_threshold: f32,
) -> Vec<CascadeResult> {
    // Modified Dijkstra maximizing signal (from traversal-friction.md)
    // For each edge: signal *= permeability(edge) * friction_factor
    // Stop when signal < significance_threshold
}
```

**When**: After turn commitment, as part of scene resolution. Results are batched — all edge updates from the cascade are committed in a single PostgreSQL transaction containing multiple Cypher SET statements.

### Asymmetry Detection

**Source math**: [`relational-web-math.md`](../graph-strategy/relational-web-math.md) §Vector Operations

**Reads from**: AGE edge properties (both directions of a dyad)

**Writes to**: Nothing — returns query results consumed by context assembly

This could technically be a Cypher query (match both directions, compute distance in application code), but the vector math (`‖R(a,b) - R(b,a)‖`) is simpler in Rust.

---

## Data Flow: Turn Cycle Integration

### Scene Entry

1. **Load cast subgraph from AGE** (~5ms) — all `:RelEntity` vertices and `:RELATES_TO` edges for cast members
2. **Load edge details from PostgreSQL** (~3ms) — projection, information_state for cast edges
3. **Build petgraph for cast** (~0.1ms) — in-process graph for cast-scoped operations
4. **Optionally recompute structural metrics** (~10ms) — if scene boundary triggers full-web reload

Total: ~10-20ms

### Per-Turn

1. **Classify events** — determine which relational changes occur
2. **Update AGE edges** (~1-3ms) — Cypher SET for changed substrate dimensions
3. **Run cascade propagation** (~2-5ms in petgraph) — propagate changes through the web
4. **Write cascade results to AGE** (~1-3ms) — additional edge updates from cascade
5. **Append to edge history** (~1ms) — PostgreSQL INSERT for changelog

Total: ~5-12ms

### Scene Exit

1. **Recompute structural metrics if needed** (~15ms) — betweenness, articulation, clustering
2. **Write cached properties to AGE vertices** (~2ms) — topological_role, cluster_id
3. **Sync edge details to PostgreSQL** (~3ms) — updated projection, information_state

Total: ~20ms

### Session Start

1. **Verify all Tracked+ entities have `:RelEntity` vertices** — catch any entities promoted since last session
2. **No full web load** — web is loaded scene-by-scene, not all at once

---

## AGE Capability Assessment

| Requirement | AGE 1.7.0 Status | Notes |
|------------|------------------|-------|
| Directed edges with 8 float properties | Supported | Core relational web schema |
| `WHERE` filter on edge properties | Supported | Tension detection |
| `shortestPath()` (unweighted) | Supported | Social distance (hop count) |
| Cast subgraph by vertex set | Supported | `WHERE a.pg_id IN $list AND b.pg_id IN $list` |
| `SET` for edge property updates | Supported | Per-turn substrate modification |
| Property index on edges | Supported | Trust/debt thresholds for tension queries |
| Weighted shortest path | Not Supported | Use petgraph for permeability-weighted paths |
| Path aggregation (product of permeability) | Needs Spike | If supported, could move friction computation to Cypher |
| Triangle pattern matching | Needs Spike | Triadic closure detection for trust transitivity |
| `MERGE` for upsert | Supported | Idempotent edge creation at story authoring |

---

## Questions for TAS-244 Spike

1. **Edge property performance**: What is the latency for a cast subgraph query returning 8 properties per edge at 50 edges? At 200 edges?
2. **`IN` clause performance**: Does `WHERE a.pg_id IN [list of 8 UUIDs]` use indexes efficiently, or does it fall back to scan?
3. **Path aggregation**: Can we express `REDUCE(total = 1.0, r IN relationships(path) | total * r.trust_benevolence * r.history_depth * (1.0 - r.opacity))` in AGE 1.7.0 Cypher? If so, the friction model could partially move to Cypher.
4. **Triangle detection**: Does `MATCH (a)-[r1:RELATES_TO]->(b)-[r2:RELATES_TO]->(c)-[r3:RELATES_TO]->(a)` perform adequately for trust transitivity analysis?
5. **Batch edge update**: For cascade resolution updating 5-10 edges per turn, is it faster to issue 5-10 separate `MATCH...SET` statements or use a single `UNWIND` pattern?
6. **Index on agtype properties**: AGE indexes use `properties ->> 'key'` syntax. Verify this works for float comparison operators (`<`, `>`) and not just equality.
