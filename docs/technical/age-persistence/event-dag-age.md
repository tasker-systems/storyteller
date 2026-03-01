# Event Dependency DAG: PostgreSQL Persistence

## Purpose

This document maps the mathematical operations in [`event-dag.md`](../graph-strategy/event-dag.md) to persistence structures. After careful analysis, the recommendation is: **the Event DAG lives in PostgreSQL relational tables with recursive CTEs, not in Apache AGE.**

This is the architecturally decisive document in the series — it argues against using AGE for one of the four graphs, and provides the PostgreSQL-native alternative drawing on proven patterns from the tasker-core sibling project.

**Prerequisites**: [README.md](README.md) for shared decisions, [`event-dag.md`](../graph-strategy/event-dag.md) for the mathematics.

**Storykeeper operations supported**: `get_evaluable_frontier`, `resolve_event`, `propagate_resolution`, `get_event_reachability`, `get_critical_path`, `check_exclusion_cascade` (from [`storykeeper-api-contract.md`](../storykeeper-api-contract.md)).

---

## Why Not AGE?

The Event DAG is structurally a graph — nodes with typed directed edges. It seems natural for AGE. But the access patterns and computational requirements make PostgreSQL relational tables a better fit:

### Arguments Against AGE

**1. The DAG is loaded entirely at story start and doesn't change structure at runtime.**

Event conditions and their dependency edges are authored. At runtime, only the **resolution state** changes — a condition moves from `Unresolved` to `Resolved` or `Excluded`. The graph structure itself (which conditions exist, what depends on what) is static. This means:
- No graph mutations during play (only state updates)
- The full graph can be loaded once and cached in-process
- AGE's value (transactional graph mutations, path queries on evolving structure) is not leveraged

**2. The key algorithms are not expressible in Cypher.**

From [`event-dag.md`](../graph-strategy/event-dag.md):
- **Kahn's topological sort** — iterative algorithm removing zero-in-degree nodes. No Cypher equivalent.
- **Exclusion cascade** — BFS from excluded nodes, orphaning unreachable descendants. Requires stateful traversal that Cypher can't express.
- **Evaluable frontier maintenance** — incremental update on resolution. Requires reading resolution state of all parents per node.
- **Amplification compounding** — multiplicative aggregation along paths with clamping. No Cypher aggregate for this.

**3. Resolution state is per-session.**

Two players playing the same story resolve different events in different orders. Resolution state is `(session_id, condition_id, state)` — a relational table with session scoping. AGE vertices are story-scoped, not session-scoped.

**4. Cross-graph reachability is answered by querying the narrative graph.**

The one cross-graph operation ("is this condition's resolving scene still reachable?") is a Cypher query against the narrative graph's `:Scene` vertices — it doesn't require the Event DAG itself to be in AGE.

### Arguments For AGE (Weighed and Rejected)

- **Graph traversal for reachability** — true, but the DAG is small enough (< 500 nodes) that petgraph traversal is sub-millisecond.
- **Unified query language** — convenience, not necessity. The DAG's queries are specialized enough that custom SQL functions serve better.
- **Co-location with other graphs** — cross-graph queries work via shared UUIDs regardless of whether the DAG is in AGE.

---

## Persistence Decision Matrix

| Data | Location | Rationale |
|------|----------|-----------|
| Event conditions (nodes) | PostgreSQL `event_conditions` table | Static authored content; loaded at session start |
| Event dependencies (edges) | PostgreSQL `event_dependencies` table | Static authored structure; loaded at session start |
| Resolution state | PostgreSQL `event_condition_states` table | Session-scoped; changes per-turn |
| DAG computation (topological sort, frontier, cascades) | In-process petgraph | Sub-millisecond at our scale; full algorithmic control |
| Cross-graph reachability | Cypher query against AGE `:Scene` vertices | Only cross-graph operation; uses narrative graph |

---

## PostgreSQL Tables

### `event_conditions`

The DAG nodes — narrative conditions that can be resolved during play:

```sql
CREATE TABLE event_conditions (
    id                UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id          UUID NOT NULL REFERENCES stories(id),
    -- Identity
    name              TEXT NOT NULL,
    description       TEXT,
    -- Type classification (from event-dag.md)
    condition_type    TEXT NOT NULL,  -- 'prerequisite', 'discovery', 'gate', 'emergent', 'exclusion'
    -- Resolution
    resolution_predicate JSONB NOT NULL,  -- What must be true for this condition to resolve
    resolving_scene_ids  UUID[] DEFAULT '{}',  -- Scenes where resolution can occur
    -- Narrative weight
    narrative_weight  REAL NOT NULL DEFAULT 1.0,
    -- Amplification (base value before compounding)
    base_amplification REAL NOT NULL DEFAULT 1.0,
    -- Metadata
    layer_id          UUID REFERENCES sub_graph_layers(id),
    metadata          JSONB,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, name)
);

CREATE INDEX idx_event_conditions_story ON event_conditions(story_id);
CREATE INDEX idx_event_conditions_type ON event_conditions(story_id, condition_type);
CREATE INDEX idx_event_conditions_layer ON event_conditions(layer_id);
CREATE INDEX idx_event_conditions_resolving_scenes
    ON event_conditions USING GIN (resolving_scene_ids);
```

### `event_dependencies`

The DAG edges — typed relationships between conditions:

```sql
CREATE TABLE event_dependencies (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    from_condition_id UUID NOT NULL REFERENCES event_conditions(id),
    to_condition_id   UUID NOT NULL REFERENCES event_conditions(id),
    -- Type from event-dag.md: requires, excludes, enables, amplifies
    dependency_type TEXT NOT NULL,
    -- Amplification weight (only meaningful for 'amplifies' type)
    amplification_weight REAL DEFAULT 1.0,
    -- Metadata
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, from_condition_id, to_condition_id, dependency_type),
    CONSTRAINT chk_dependency_type CHECK (
        dependency_type IN ('requires', 'excludes', 'enables', 'amplifies')
    )
);

CREATE INDEX idx_event_deps_story ON event_dependencies(story_id);
CREATE INDEX idx_event_deps_from ON event_dependencies(from_condition_id);
CREATE INDEX idx_event_deps_to ON event_dependencies(to_condition_id);
CREATE INDEX idx_event_deps_type ON event_dependencies(dependency_type);
```

### `event_condition_states`

Session-scoped resolution tracking:

```sql
CREATE TABLE event_condition_states (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id      UUID NOT NULL REFERENCES sessions(id),
    condition_id    UUID NOT NULL REFERENCES event_conditions(id),
    -- Resolution state from event-dag.md
    resolution_state TEXT NOT NULL DEFAULT 'unresolved',
    -- Computed consequence magnitude (after amplification compounding)
    consequence_magnitude REAL NOT NULL DEFAULT 1.0,
    -- When resolved
    resolved_at_turn INT,
    resolved_at     TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, condition_id),
    CONSTRAINT chk_resolution_state CHECK (
        resolution_state IN ('unresolved', 'resolved', 'excluded', 'unreachable')
    )
);

CREATE INDEX idx_event_states_session ON event_condition_states(session_id);
CREATE INDEX idx_event_states_state ON event_condition_states(session_id, resolution_state);
```

---

## SQL Functions: Recursive CTE Patterns

### Prior Art: tasker-core

The tasker-core sibling project has a battle-tested DAG framework in PostgreSQL for workflow step dependencies. Three patterns translate directly to the Event DAG:

| tasker-core Pattern | Storyteller Adaptation |
|--------------------|-----------------------|
| `workflow_step_edges` table | `event_dependencies` table (same from/to/type structure) |
| `step_dag_relationships` view (recursive CTE for depth) | `event_dag_levels` view (dependency levels from root conditions) |
| `calculate_dependency_levels()` function | `calculate_event_dependency_levels()` function |
| `get_step_transitive_dependencies()` (transitive closure with distance) | `get_transitive_dependencies()` (walk dependency chain) |
| `get_step_readiness_status()` (all parents complete?) | `get_evaluable_frontier()` (all Requires-parents resolved?) |

### `calculate_event_dependency_levels()`

Adapted from `tasker.calculate_dependency_levels()`:

```sql
CREATE FUNCTION calculate_event_dependency_levels(input_story_id uuid)
RETURNS TABLE(condition_id uuid, dependency_level integer)
LANGUAGE plpgsql STABLE
AS $$
BEGIN
    RETURN QUERY
    WITH RECURSIVE dependency_levels AS (
        -- Base case: root conditions (no 'requires' dependencies)
        SELECT
            ec.id AS condition_id,
            0 AS level
        FROM event_conditions ec
        WHERE ec.story_id = input_story_id
            AND NOT EXISTS (
                SELECT 1 FROM event_dependencies ed
                WHERE ed.to_condition_id = ec.id
                    AND ed.dependency_type = 'requires'
            )

        UNION ALL

        -- Recursive case: conditions whose 'requires' parents are at current level
        SELECT
            ed.to_condition_id AS condition_id,
            dl.level + 1 AS level
        FROM dependency_levels dl
        JOIN event_dependencies ed ON ed.from_condition_id = dl.condition_id
        JOIN event_conditions ec ON ec.id = ed.to_condition_id
        WHERE ec.story_id = input_story_id
            AND ed.dependency_type = 'requires'
            AND dl.level < 50  -- Prevent infinite recursion
    )
    SELECT
        dl.condition_id,
        MAX(dl.level) AS dependency_level  -- MAX handles multiple paths (diamond dependencies)
    FROM dependency_levels dl
    GROUP BY dl.condition_id
    ORDER BY dependency_level, condition_id;
END;
$$;
```

This is structurally identical to tasker-core's `calculate_dependency_levels()` — the recursive CTE walks from root nodes (no incoming `requires` edges) outward, and `MAX(level)` handles diamond dependencies where a node is reachable via multiple paths of different lengths.

### `get_evaluable_frontier()`

The DAG equivalent of tasker-core's "ready steps" — conditions whose `Requires` parents are all resolved:

```sql
CREATE FUNCTION get_evaluable_frontier(
    input_session_id uuid,
    input_story_id uuid
)
RETURNS TABLE(
    condition_id uuid,
    condition_name text,
    condition_type text,
    narrative_weight real,
    dependency_level integer,
    total_requires integer,
    resolved_requires integer
)
LANGUAGE plpgsql STABLE
AS $$
BEGIN
    RETURN QUERY
    WITH levels AS (
        SELECT * FROM calculate_event_dependency_levels(input_story_id)
    ),
    -- Current resolution state for this session
    states AS (
        SELECT ecs.condition_id, ecs.resolution_state
        FROM event_condition_states ecs
        WHERE ecs.session_id = input_session_id
    ),
    -- Count requires dependencies and how many are resolved
    requires_status AS (
        SELECT
            ec.id AS condition_id,
            COUNT(ed.from_condition_id) AS total_requires,
            COUNT(ed.from_condition_id) FILTER (
                WHERE COALESCE(s.resolution_state, 'unresolved') = 'resolved'
            ) AS resolved_requires
        FROM event_conditions ec
        LEFT JOIN event_dependencies ed
            ON ed.to_condition_id = ec.id AND ed.dependency_type = 'requires'
        LEFT JOIN states s ON s.condition_id = ed.from_condition_id
        WHERE ec.story_id = input_story_id
        GROUP BY ec.id
    )
    SELECT
        ec.id AS condition_id,
        ec.name AS condition_name,
        ec.condition_type,
        ec.narrative_weight,
        COALESCE(l.dependency_level, 0),
        COALESCE(rs.total_requires, 0)::integer,
        COALESCE(rs.resolved_requires, 0)::integer
    FROM event_conditions ec
    JOIN requires_status rs ON rs.condition_id = ec.id
    LEFT JOIN levels l ON l.condition_id = ec.id
    LEFT JOIN states s ON s.condition_id = ec.id
    WHERE ec.story_id = input_story_id
        -- Not yet resolved or excluded
        AND COALESCE(s.resolution_state, 'unresolved') = 'unresolved'
        -- All requires-parents are resolved
        AND rs.total_requires = rs.resolved_requires
        -- Not excluded by any resolved exclusion
        AND NOT EXISTS (
            SELECT 1
            FROM event_dependencies excl
            JOIN states excl_state ON excl_state.condition_id = excl.from_condition_id
            WHERE excl.to_condition_id = ec.id
                AND excl.dependency_type = 'excludes'
                AND excl_state.resolution_state = 'resolved'
        )
    ORDER BY l.dependency_level, ec.narrative_weight DESC;
END;
$$;
```

### `get_transitive_dependencies()`

Adapted from tasker-core's `get_step_transitive_dependencies()`:

```sql
CREATE FUNCTION get_transitive_dependencies(target_condition_id uuid)
RETURNS TABLE(
    condition_id uuid,
    condition_name text,
    dependency_type text,
    distance integer
)
LANGUAGE plpgsql STABLE
AS $$
BEGIN
    RETURN QUERY
    WITH RECURSIVE transitive_deps AS (
        -- Base case: direct parents
        SELECT
            ec.id AS condition_id,
            ec.name AS condition_name,
            ed.dependency_type,
            1 AS distance
        FROM event_dependencies ed
        JOIN event_conditions ec ON ec.id = ed.from_condition_id
        WHERE ed.to_condition_id = target_condition_id

        UNION ALL

        -- Recursive case: parents of parents
        SELECT
            ec.id AS condition_id,
            ec.name AS condition_name,
            ed.dependency_type,
            td.distance + 1
        FROM transitive_deps td
        JOIN event_dependencies ed ON ed.to_condition_id = td.condition_id
        JOIN event_conditions ec ON ec.id = ed.from_condition_id
        WHERE td.distance < 50
    )
    SELECT
        td.condition_id,
        td.condition_name,
        td.dependency_type,
        td.distance
    FROM transitive_deps td
    ORDER BY td.distance ASC, td.condition_id;
END;
$$;
```

### `event_dag_overview` View

Adapted from tasker-core's `step_dag_relationships` view:

```sql
CREATE VIEW event_dag_overview AS
SELECT
    ec.id AS condition_id,
    ec.story_id,
    ec.name,
    ec.condition_type,
    ec.narrative_weight,
    COALESCE(parent_data.parent_count, 0) AS parent_count,
    COALESCE(child_data.child_count, 0) AS child_count,
    COALESCE(parent_data.parent_count, 0) = 0 AS is_root,
    COALESCE(child_data.child_count, 0) = 0 AS is_leaf,
    depth_info.dependency_level
FROM event_conditions ec
LEFT JOIN (
    SELECT to_condition_id, COUNT(*) AS parent_count
    FROM event_dependencies
    WHERE dependency_type = 'requires'
    GROUP BY to_condition_id
) parent_data ON parent_data.to_condition_id = ec.id
LEFT JOIN (
    SELECT from_condition_id, COUNT(*) AS child_count
    FROM event_dependencies
    WHERE dependency_type = 'requires'
    GROUP BY from_condition_id
) child_data ON child_data.from_condition_id = ec.id
LEFT JOIN (
    WITH RECURSIVE step_depths AS (
        SELECT ec_inner.id AS condition_id, 0 AS depth_from_root, ec_inner.story_id
        FROM event_conditions ec_inner
        WHERE NOT EXISTS (
            SELECT 1 FROM event_dependencies e
            WHERE e.to_condition_id = ec_inner.id AND e.dependency_type = 'requires'
        )
        UNION ALL
        SELECT e.to_condition_id, sd.depth_from_root + 1, sd.story_id
        FROM step_depths sd
        JOIN event_dependencies e ON e.from_condition_id = sd.condition_id
        WHERE e.dependency_type = 'requires' AND sd.depth_from_root < 50
    )
    SELECT condition_id, MIN(depth_from_root) AS dependency_level
    FROM step_depths
    GROUP BY condition_id
) depth_info ON depth_info.condition_id = ec.id;
```

---

## Application-Level Computations (petgraph)

Despite the SQL functions above, the DAG is **also** loaded into petgraph at session start for in-process algorithms that are more natural in Rust than SQL:

### Kahn's Topological Sort

**Source math**: [`event-dag.md`](../graph-strategy/event-dag.md) §Topological Sort

Iterative algorithm removing zero-in-degree nodes. Validates DAG acyclicity (if nodes remain after the algorithm, there's a cycle). Petgraph provides `toposort()` natively.

### Exclusion Cascade

**Source math**: [`event-dag.md`](../graph-strategy/event-dag.md) §Exclusion Cascade

BFS from excluded nodes, marking all descendants as `Unreachable` unless they have alternative paths from non-excluded parents. This is stateful traversal with backtracking — natural in Rust, awkward in SQL.

### Amplification Compounding

**Source math**: [`event-dag.md`](../graph-strategy/event-dag.md) §Amplification Compounding

Walk `Amplifies` edges, multiply weights, clamp to `MAX_AMPLIFICATION`. Simple graph traversal with accumulation — one line in petgraph, complex in SQL.

### Cross-Graph Reachability

The one operation that touches AGE:

```rust
async fn check_scene_reachable(
    pool: &PgPool,
    current_scene_id: &Uuid,
    target_scene_id: &Uuid,
) -> Result<bool> {
    // Cypher query against narrative graph
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM cypher('storyteller', $$
            MATCH path = (a:Scene {pg_id: $1})-[:TRANSITIONS_TO*1..10]->(b:Scene {pg_id: $2})
            RETURN 1 LIMIT 1
        $$) AS (result agtype)"
    )
    .bind(current_scene_id.to_string())
    .bind(target_scene_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(result.is_some())
}
```

This queries the AGE narrative graph to determine if a condition's resolving scene is still reachable from the player's current position. The Event DAG itself doesn't need to be in AGE for this — it just needs to know the `resolving_scene_ids` from the `event_conditions` table.

---

## Data Flow: Turn Cycle Integration

### Session Start

1. **Load all event conditions from PostgreSQL** (~2ms) — `event_conditions` for the story
2. **Load all event dependencies from PostgreSQL** (~1ms) — `event_dependencies` for the story
3. **Load session resolution state** (~1ms) — `event_condition_states` for this session (creates rows for any new conditions)
4. **Build petgraph DAG** (~0.1ms) — in-process, cached for session
5. **Validate DAG acyclicity** (~0.1ms) — petgraph `toposort()`, should never fail on authored content
6. **Compute initial frontier** (~0.1ms) — petgraph traversal

Total: ~5ms

### Per-Turn

1. **Check if events resolve conditions** (~0.5ms) — match committed events against resolution predicates
2. **Update resolution state in PostgreSQL** (~1ms) — INSERT/UPDATE on `event_condition_states`
3. **Run resolution propagation in petgraph** (~0.5ms) — update frontier, check exclusion cascades
4. **Check cross-graph reachability if needed** (~5ms) — Cypher query against narrative graph (only when scene transitions occur)
5. **Compute amplification if applicable** (~0.1ms) — multiplicative compounding in petgraph

Total: ~2-7ms (depending on whether cross-graph reachability is needed)

### Scene Exit

1. **Sync frontier state to PostgreSQL** (~1ms) — ensure `event_condition_states` reflects current petgraph state
2. **No structural changes** — DAG structure is immutable at runtime

---

## AGE Capability Assessment

The Event DAG does not use AGE. This section documents why each AGE capability is insufficient for the DAG's needs:

| DAG Requirement | AGE Alternative | Why AGE Falls Short |
|----------------|-----------------|-------------------|
| Topological sort | No equivalent | Kahn's algorithm is iterative; Cypher is declarative |
| Evaluable frontier | Complex CTE possible but fragile | SQL function is cleaner and tested |
| Exclusion cascade | Multi-step BFS with state | Requires per-node state tracking during traversal |
| Amplification compounding | `REDUCE` along path | AGE `REDUCE` support is untested; clamping adds complexity |
| Per-session state | No native session scoping | AGE vertices are story-scoped, not session-scoped |
| Acyclicity validation | No equivalent | DFS coloring algorithm; petgraph `toposort()` |

---

## Questions for TAS-244 Spike

Even though the Event DAG doesn't use AGE, the spike should validate:

1. **Cross-graph reachability performance**: What is the latency for `MATCH path = (a:Scene)-[:TRANSITIONS_TO*1..10]->(b:Scene) RETURN 1 LIMIT 1` at 200 scenes? This query is called from the Event DAG's resolution propagation.
2. **PostgreSQL recursive CTE performance**: With 500 event conditions and 1000 dependencies, what is the latency for `get_evaluable_frontier()`? Validate that the SQL function scales.
3. **petgraph load time**: Loading 500 nodes and 1000 edges from PostgreSQL into petgraph — confirm this stays under 5ms including the query.
4. **Validate the tasker-core patterns**: Run the `event_dag_overview` view and `calculate_event_dependency_levels()` function against a representative TFATD dataset. Confirm the recursive CTEs produce correct results for diamond dependencies and multi-path DAGs.
