# AGE Persistence: Graph Strategy to Data Structures

## Purpose

This directory maps the mathematical foundations in [`graph-strategy/`](../graph-strategy/) to concrete persistence structures in PostgreSQL + Apache AGE. Where graph-strategy says "how to compute it," these documents say "where it lives, how to query it, and what runs in application code."

These documents guide **TAS-244** (AGE spike — validate Cypher coverage and performance) and **TAS-245** (AGE schema implementation). They are design documents, not implementation code.

**Prerequisites**: Familiarity with the four graph structures from [`graph-strategy/README.md`](../graph-strategy/README.md).

---

## Document Map

| Document | Source Math | Scope |
|----------|-----------|-------|
| [relational-web-age.md](relational-web-age.md) | [relational-web-math.md](../graph-strategy/relational-web-math.md) | Substrate edges in AGE, structural metrics via petgraph, cascade data flow |
| [setting-topology-age.md](setting-topology-age.md) | [setting-topology.md](../graph-strategy/setting-topology.md) | Spatial graph in AGE, cross-graph `:LOCATED_AT` pattern, in-process Dijkstra |
| [narrative-graph-age.md](narrative-graph-age.md) | [narrative-gravity.md](../graph-strategy/narrative-gravity.md) | Scene transitions in AGE, duplication avoidance with `scenes` table, session-scoped state |
| [event-dag-age.md](event-dag-age.md) | [event-dag.md](../graph-strategy/event-dag.md) | PostgreSQL relational tables (not AGE), recursive CTEs from tasker-core prior art |
| [cross-cutting-age.md](cross-cutting-age.md) | [traversal-friction.md](../graph-strategy/traversal-friction.md), [cross-graph-composition.md](../graph-strategy/cross-graph-composition.md) | Data source mapping for friction and composition computations, new table proposals |

**Reading order**: README → relational-web-age → setting-topology-age → narrative-graph-age → event-dag-age → cross-cutting-age. Each document is self-contained; the sequence builds understanding of the AGE-vs-PostgreSQL boundary.

---

## Shared Architectural Decisions

### Single AGE Graph Instance

All graph data lives in one AGE graph, distinguished by vertex and edge label namespaces:

```sql
-- Migration: 20260301000013_create_age_graph.sql
LOAD 'age';
SET search_path = ag_catalog, public;

SELECT create_graph('storyteller');
```

**Rationale**: Cross-graph queries (e.g., "scenes at reachable settings with relational tension in the cast") are first-class operations. Co-location enables single-transaction Cypher queries spanning multiple label namespaces. The Storykeeper trait abstraction insulates the engine — if co-location becomes a problem, switching to separate graphs is an implementation change behind the same trait surface.

### Label Namespace Convention

| Graph | Vertex Label | Edge Labels |
|-------|-------------|-------------|
| Relational Web | `:RelEntity` | `:RELATES_TO` |
| Setting Topology | `:Setting` | `:CONNECTS_TO` |
| Narrative Graph | `:Scene` | `:TRANSITIONS_TO` |
| Cross-Graph | — | `:LOCATED_AT` (Scene → Setting) |

The Event DAG does **not** use AGE — it lives in PostgreSQL relational tables (see [event-dag-age.md](event-dag-age.md) for rationale).

### Cross-Graph Linking: Shared UUIDs

AGE vertices reference PostgreSQL tables via a `pg_id` property holding the UUID primary key from the relational table. Cross-graph queries use a two-phase pattern:

**Phase 1: Cypher query** — traverse the graph, return `pg_id` values:
```sql
SELECT * FROM cypher('storyteller', $$
    MATCH (s:Setting {pg_id: $current_setting})-[:CONNECTS_TO*1..3]-(t:Setting)
    RETURN t.pg_id AS setting_id
$$) AS (setting_id agtype);
```

**Phase 2: PostgreSQL JOIN** — enrich with relational data:
```sql
WITH reachable AS (
    -- Cypher query from Phase 1
)
SELECT r.setting_id, s.name, s.spatial_data
FROM reachable r
JOIN settings s ON s.id = r.setting_id::text::uuid;
```

**Why not cross-graph edges?** AGE does not support edges between vertices with different labels in a way that composes cleanly with variable-length path patterns. Shared UUIDs with SQL JOINs are more flexible and leverage PostgreSQL's relational strengths.

### The Petgraph Fallback Rule

**Use Cypher** when the operation is a local graph traversal:
- Neighborhood lookup (direct edges from/to a vertex)
- Pattern matching (find edges matching property filters)
- Bounded path queries (N-hop reachability, `shortestPath()`)
- Cast subgraph extraction (all edges between a vertex set)

**Use petgraph** when the operation requires global graph structure:
- Centrality computation (Brandes' betweenness, degree centrality)
- Articulation point detection (Tarjan's algorithm)
- Clustering (label propagation, community detection)
- Custom pathfinding (entity-specific cost functions, permeability-weighted Dijkstra)
- Topological sort (Kahn's algorithm for DAG ordering)
- Cascade propagation (modified Dijkstra maximizing signal)

**The pattern**: Load the relevant subgraph from AGE into petgraph at a defined boundary (scene entry, session start, scene exit). Run the algorithm in-process. Cache results back to AGE vertex properties or PostgreSQL tables.

### AGE 1.7.0 OpenCypher Coverage

Based on the Apache AGE 1.7.0 documentation. Items marked "Needs Spike" require empirical validation in TAS-244.

| Capability | Status | Notes |
|-----------|--------|-------|
| `CREATE`, `MATCH`, `MERGE`, `SET`, `DELETE` | Supported | Core CRUD operations |
| Directed edges with labels | Supported | Essential for asymmetric relational web |
| Properties on vertices and edges (agtype) | Supported | All scalar types, nested objects |
| `WHERE` with property filters | Supported | Substrate dimension filtering |
| Variable-length paths `*1..N` | Supported | N-hop reachability |
| `shortestPath()` | Supported | Unweighted shortest path only |
| `RETURN` with aggregation (`count`, `sum`, `avg`) | Supported | Basic aggregations |
| `WITH` for query chaining | Supported | Multi-step Cypher queries |
| `OPTIONAL MATCH` | Supported | Left-outer-join semantics |
| `UNWIND` for list processing | Supported | Batch vertex/edge creation |
| Path aggregation (product along path) | Needs Spike | Required for friction model — may need application code |
| Weighted shortest path (Dijkstra) | Not Supported | Use petgraph |
| `allShortestPaths()` | Needs Spike | Useful but not critical |
| `FOREACH` | Not Supported | Use `UNWIND` + `MERGE` instead |
| Stored procedures (`CALL`) | Not Supported | All logic in application code or SQL functions |
| Graph projections / subgraph views | Not Supported | Extract via MATCH, build in-process |
| `DETACH DELETE` | Supported | Vertex + edge cleanup |
| Index on vertex/edge properties | Supported | Via `CREATE INDEX` on AGE label properties |

### Data Loading Budget

From the [graph-strategy computational summaries](../graph-strategy/README.md), the performance envelope is:

| Operation | Budget | When |
|-----------|--------|------|
| Scene entry (full graph load) | < 50ms total | Scene boundary |
| Per-turn graph updates | < 5ms | Every turn |
| Structural recomputation (centrality, clustering) | < 20ms | Scene boundary |
| Cross-graph query | < 20ms | Scene entry, context assembly |
| Adjacent/neighborhood lookup | < 2ms | Per-turn |

Scene entry is the bulk-load window. Per-turn operations must be fast — they are on the player's critical path. Structural recomputation (petgraph algorithms) runs at scene boundaries, not per-turn.

### Existing PostgreSQL Tables

The 12 existing migrations define tables that serve as **identity anchors** for AGE vertices. AGE vertices reference these via `pg_id` — they do not duplicate them:

| Table | AGE Vertex | Relationship |
|-------|-----------|-------------|
| `entities` | `:RelEntity` | `entities.id` = `:RelEntity.pg_id` |
| `settings` | `:Setting` | `settings.id` = `:Setting.pg_id` |
| `scenes` | `:Scene` | `scenes.id` = `:Scene.pg_id` |
| `sub_graph_layers` | — | Referenced by `layer_id` properties on vertices |
| `stories` | — | Root anchor; `story_id` on all vertices for multi-story isolation |

**Rule**: If data already exists in a PostgreSQL table, do not duplicate it on the AGE vertex unless it is needed for Cypher `WHERE` filters or path computation. When in doubt, query PostgreSQL via JOIN rather than duplicating on the vertex.

### Migration Numbering

AGE-related migrations continue from the existing sequence:

| Migration | Content |
|-----------|---------|
| `20260301000013_create_age_graph.sql` | AGE graph creation, label indexes |
| `20260301000014_create_age_relational_web.sql` | `:RelEntity` vertices, `:RELATES_TO` edges (initial schema setup functions) |
| `20260301000015_create_age_setting_topology.sql` | `:Setting` vertices, `:CONNECTS_TO` edges, `:LOCATED_AT` cross-graph edges |
| `20260301000016_create_age_narrative_graph.sql` | `:Scene` vertices, `:TRANSITIONS_TO` edges |
| `20260301000017_create_event_dag_tables.sql` | PostgreSQL tables for event conditions, dependencies, session state |
| `20260301000018_create_cross_cutting_tables.sql` | Prophetic cascades, boundary dynamics, communication affordances |

The actual dates will be set at implementation time. These numbers establish the dependency order.

---

## Relationship to Other Documents

| Document | Role |
|----------|------|
| [`graph-strategy/`](../graph-strategy/) | Mathematical foundations — what to compute |
| [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) | Domain model — what the graphs represent |
| [`storykeeper-api-contract.md`](../storykeeper-api-contract.md) | Trait interface — what operations the persistence must support |
| [`postgresql-schema-design.md`](../postgresql-schema-design.md) | Existing table design rationale |
| [`infrastructure-architecture.md`](../infrastructure-architecture.md) | Runtime data lifecycle — when graph data moves between stores |
| `storyteller-storykeeper/migrations/` | Existing 12 migration files — the tables these docs reference |

---

## Current Status

| Document | Status |
|----------|--------|
| README.md | Complete |
| relational-web-age.md | Complete |
| setting-topology-age.md | Complete |
| narrative-graph-age.md | Complete |
| event-dag-age.md | Complete |
| cross-cutting-age.md | Complete |
