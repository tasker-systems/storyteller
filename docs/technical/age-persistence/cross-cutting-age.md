# Cross-Cutting Computations: Data Source Mapping

## Purpose

This document maps the cross-cutting mathematical operations from [`traversal-friction.md`](../graph-strategy/traversal-friction.md) and [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) to their concrete data sources and persistence requirements. Unlike the previous four documents, this is not a graph schema document — the cross-cutting computations are **application-level algorithms** that read from multiple data stores. This document identifies what each computation reads, where results are written, and what new tables are needed.

**Prerequisites**: All prior documents in this directory — [README.md](README.md), [relational-web-age.md](relational-web-age.md), [setting-topology-age.md](setting-topology-age.md), [narrative-graph-age.md](narrative-graph-age.md), [event-dag-age.md](event-dag-age.md).

**Storykeeper operations supported**: `compute_composite_score`, `resolve_cascade`, `compute_boundary_permeability`, `get_prophetic_cascades`, `allocate_token_budget` (from [`storykeeper-api-contract.md`](../storykeeper-api-contract.md)).

---

## Data Source Summary

Every cross-cutting computation assembles data from multiple stores. This table maps computations to their data sources:

| Computation | AGE (Cypher) | PostgreSQL (SQL) | In-Process (petgraph) | Application State |
|-------------|-------------|-----------------|----------------------|-------------------|
| Composite scoring | — | `scenes`, `event_condition_states` | — | Candidate list, current turn |
| Gravitational modifier | Reachable scenes (`:TRANSITIONS_TO`) | `scenes.narrative_mass`, `scene_activation_states` | — | Player distance vector |
| Sub-graph collective mass | — | `sub_graph_layers`, `sub_graph_visit_history` (new) | — | Visit emotional residue |
| Boundary permeability | — | `sub_graph_layers.permeability`, `boundary_perturbations` (new) | — | Current turn |
| Prophetic cascade | — | `prophetic_cascades` (new) | — | Active prophecy state |
| Information boundary | — | `event_condition_states` | Event DAG (petgraph) | Information ledger |
| Narrative temperature | — | `scenes.scene_data` | — | Scene state |
| Recency decay | — | — | — | Turn counter, candidate source turns |
| Social friction (permeability) | `:RELATES_TO` edge substrates | `relational_edge_details` | Cascade graph (petgraph) | Friction factor config |
| Temporal friction | — | `communication_affordances` (new) | — | Scene chronological data |
| Multi-path cascade | `:RELATES_TO` paths | — | Modified Dijkstra (petgraph) | Signal source, strength |
| Budget allocation | — | — | — | Scored candidates, token estimates |

**Pattern**: AGE provides graph structure (reachable sets, paths, neighborhoods). PostgreSQL provides authored configuration and session-scoped state. Petgraph runs the algorithms. Application state holds ephemeral per-turn data.

---

## Traversal Friction: Data Sources

### Social Friction (Relational Cascade)

**Source math**: [`traversal-friction.md`](../graph-strategy/traversal-friction.md) §Relational Web: Social Friction

The three-component permeability computation reads entirely from AGE edge properties:

| Component | Data Source | Location |
|-----------|-----------|----------|
| Trust factor: `(trust_competence + 2 × trust_benevolence) / 3` | `:RELATES_TO` edge `trust_competence`, `trust_benevolence` | AGE |
| History factor: `min(history_depth, 1.0)` | `:RELATES_TO` edge `history_depth` | AGE |
| Opacity modifier: `1.0 - opacity` | `:RELATES_TO` edge `opacity` | AGE |

The friction factor `F` (story-configurable) comes from story-level configuration:

```sql
-- Story-level friction factor configuration
-- Lives in stories.story_config JSONB (existing column)
-- Path: story_config -> 'relational_web' -> 'friction_factor'
-- Default: 0.5
```

No new table needed — the friction factor is a story configuration value.

### Temporal Friction (Communication Velocity)

**Source math**: [`traversal-friction.md`](../graph-strategy/traversal-friction.md) §Temporal Friction

Temporal friction introduces the concept of **communication velocity** as a setting-level property. This requires a new table.

**Data sources**:

| Data | Source | Location |
|------|--------|----------|
| Narrative time between scenes | Scene metadata | PostgreSQL `scenes.scene_data` or `scene_instances` |
| Communication velocity (baseline) | Setting configuration | `communication_affordances` (new) |
| Additional channels | Setting configuration | `communication_affordances` (new) |
| Signal category | Application state | In-process |

### Category-Shift Distortion

**Source math**: [`traversal-friction.md`](../graph-strategy/traversal-friction.md) §Category-Shift Distortion

Pure application-level computation. No persistence required — distortion is computed in-process during cascade propagation. The distance value comes from petgraph (hop count in the cast subgraph).

### Multi-Path Resolution (Modified Dijkstra)

**Source math**: [`traversal-friction.md`](../graph-strategy/traversal-friction.md) §Multi-Path Resolution

The strongest-signal modified Dijkstra runs in petgraph on the cast subgraph loaded from AGE at scene entry. No persistence for the algorithm itself — results feed into the cascade outcome, which is persisted as relational changes on AGE edges and in `relational_edge_history`.

---

## Cross-Graph Composition: Data Sources

### Composite Scoring

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §The Composite Score

The five-factor composite score is entirely application-level computation. Each factor reads from a different source:

| Factor | Reads From | Persistence |
|--------|-----------|-------------|
| Initial relevance | Candidate generation source | Application state (not persisted) |
| Gravitational modifier | AGE (reachable scenes) + PostgreSQL (mass, activation) | Computed per-turn, not persisted |
| Information boundary | PostgreSQL (`event_condition_states`) + petgraph (event DAG) | State already persisted by event DAG |
| Recency decay | Application state (turn counter) | Not persisted |
| Narrative temperature | PostgreSQL (`scenes.scene_data`) | Scene metadata, already exists |

**No new table needed for composite scoring** — it reads from data that already exists in other tables.

### Sub-Graph Collective Mass

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §Sub-Graph Collective Mass

The collective mass computation reads sub-graph definition, visit history, and completion state. Visit history is session-scoped and requires a new table.

**Data sources**:

| Component | Source | Location |
|-----------|--------|----------|
| Base mass | Story configuration | `sub_graph_layers.metadata` JSONB → `base_mass` |
| Visit emotional residue (per visit) | Session-scoped visits | `sub_graph_visit_history` (new) |
| Completion state | Session-scoped | `sub_graph_visit_history` or `sub_graph_session_states` (new) |
| Completion bonus | Story configuration | `sub_graph_layers.metadata` JSONB → `completion_bonus` |
| Thematic resonance | Story configuration | `sub_graph_layers.metadata` JSONB → `thematic_resonance` |

### Boundary Permeability Dynamics

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §Boundary Permeability Dynamics

The sigmoid permeability model requires:

| Data | Source | Location |
|------|--------|----------|
| Sigmoid parameters (Ψ_min, Ψ_max, k, t₀) | Story configuration | `sub_graph_layers` (extend existing table) |
| Event-driven perturbations | Session-scoped events | `boundary_perturbations` (new) |
| Current turn | Application state | In-process |

The existing `sub_graph_layers` table has a `permeability` column (REAL). This column currently holds a static value. To support the sigmoid model, we either extend the table with sigmoid parameters or add them to a JSONB config column.

### Prophetic Cascade Lifecycle

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §Prophetic Cascade Lifecycle

Prophetic cascades are session-scoped forward-pointing approach vectors from sub-graph events to parent-graph scenes. They require dedicated persistence.

**Data sources**:

| Data | Source | Location |
|------|--------|----------|
| Source sub-graph and event | Authored + session state | `prophetic_cascades` (new) |
| Target scene | Authored | `prophetic_cascades.target_scene_id` |
| Approach vector modifier | Authored | `prophetic_cascades` JSONB |
| Reinforcements | Session events | `prophetic_reinforcements` (new) |
| Fulfillment condition | Authored | `prophetic_cascades.fulfillment_condition_id` |
| Cascade magnitude (computed) | Application code | Not persisted (recomputed from base + reinforcements + decay) |

### Token Budget Allocation

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §Token Budget Allocation

Pure application-level computation (greedy knapsack). Reads scored candidates and token estimates. No persistence required — budget allocation is ephemeral per-turn.

---

## New PostgreSQL Tables

### `communication_affordances`

Setting-level communication velocity configuration for temporal friction:

```sql
CREATE TABLE communication_affordances (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    setting_id      UUID REFERENCES settings(id),  -- NULL = story-wide default
    -- Baseline communication
    baseline_velocity       REAL NOT NULL DEFAULT 0.5,  -- hops per narrative time unit
    baseline_categories     TEXT[] NOT NULL DEFAULT '{general}',
    -- Additional channels (ordered list of named channels)
    channels                JSONB DEFAULT '[]',
    -- e.g. [{"name": "messenger", "velocity": 1.0, "categories": ["factual"]},
    --       {"name": "sending_stone", "velocity": 100.0, "categories": ["factual", "emotional"]}]
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, setting_id)
);

CREATE INDEX idx_comm_affordances_story ON communication_affordances(story_id);
CREATE INDEX idx_comm_affordances_setting ON communication_affordances(setting_id);
```

**Design note**: `setting_id = NULL` represents the story-wide default. When computing temporal friction for a scene, look up the setting-specific affordance first; fall back to the story default. The `channels` JSONB array holds additional communication channels beyond baseline face-to-face — messengers, magic, technology — each with their own velocity and category filter.

### `sub_graph_visit_history`

Session-scoped sub-graph visit tracking for collective mass computation:

```sql
CREATE TABLE sub_graph_visit_history (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id      UUID NOT NULL REFERENCES sessions(id),
    layer_id        UUID NOT NULL REFERENCES sub_graph_layers(id),
    -- Visit data
    visit_number    INT NOT NULL,
    entry_turn      INT NOT NULL,
    exit_turn       INT,
    -- Emotional residue from this visit
    emotional_intensity REAL NOT NULL DEFAULT 0.0,
    emotional_valence   TEXT,  -- 'tension', 'wonder', 'grief', 'warmth', etc.
    -- Completion tracking
    completed       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, layer_id, visit_number)
);

CREATE INDEX idx_sg_visits_session ON sub_graph_visit_history(session_id);
CREATE INDEX idx_sg_visits_layer ON sub_graph_visit_history(session_id, layer_id);
```

**Purpose**: Each row records one visit to a sub-graph. The `emotional_intensity` and `emotional_valence` are written at sub-graph exit (when the Storykeeper assesses how impactful the visit was). The `completed` flag is set when the sub-graph's completion conditions are met during this visit.

### `boundary_perturbations`

Session-scoped perturbation events that shift boundary permeability beyond the sigmoid curve:

```sql
CREATE TABLE boundary_perturbations (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id      UUID NOT NULL REFERENCES sessions(id),
    layer_id        UUID NOT NULL REFERENCES sub_graph_layers(id),
    -- Perturbation data
    perturbation_type TEXT NOT NULL,  -- 'narrative_progression', 'entity_crossing', 'object_bridge', 'event_driven'
    magnitude       REAL NOT NULL,
    source_turn     INT NOT NULL,
    -- What triggered the perturbation
    source_event_id UUID,  -- Optional reference to event_ledger entry
    source_entity_id UUID, -- Optional: which entity crossed the boundary
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_boundary_perturb_session ON boundary_perturbations(session_id);
CREATE INDEX idx_boundary_perturb_layer ON boundary_perturbations(session_id, layer_id);
```

**Why separate from `sub_graph_layers`**: Perturbations are session-scoped and append-only. The `sub_graph_layers` table holds authored story-level configuration. Perturbations accumulate during a play session — each entity crossing, each bridge object, each revelation event adds a row. The permeability computation sums perturbation magnitudes with exponential decay based on age.

### Extending `sub_graph_layers`: Sigmoid Configuration

Rather than creating a new table, extend the existing `sub_graph_layers` with sigmoid parameters:

```sql
ALTER TABLE sub_graph_layers
    ADD COLUMN permeability_min REAL NOT NULL DEFAULT 0.1,
    ADD COLUMN permeability_max REAL NOT NULL DEFAULT 1.0,
    ADD COLUMN permeability_steepness REAL NOT NULL DEFAULT 0.5,
    ADD COLUMN permeability_midpoint REAL NOT NULL DEFAULT 50.0,  -- Turn number
    -- Collective mass configuration
    ADD COLUMN base_mass REAL NOT NULL DEFAULT 0.5,
    ADD COLUMN completion_bonus REAL NOT NULL DEFAULT 0.3,
    ADD COLUMN thematic_resonance REAL NOT NULL DEFAULT 0.0;
```

The existing `permeability` REAL column becomes the **initial** value. The sigmoid parameters control how permeability evolves over turns. If `permeability_min = permeability_max = permeability`, the boundary is static (backwards compatible).

### `prophetic_cascades`

Session-scoped forward-pointing approach vectors from sub-graph events to parent-graph scenes:

```sql
CREATE TABLE prophetic_cascades (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id      UUID NOT NULL REFERENCES sessions(id),
    -- Source
    source_layer_id UUID NOT NULL REFERENCES sub_graph_layers(id),
    source_event_description TEXT NOT NULL,  -- Human-readable: "fire-dream in fairy tale"
    -- Target
    target_scene_id UUID NOT NULL REFERENCES scenes(id),
    -- Prophecy configuration
    prophecy_type   TEXT NOT NULL,  -- 'foreshadowing', 'thematic_echo', 'causal_bridge', 'symbolic_link'
    approach_vector_modifier JSONB,  -- Modifier to target scene's approach vectors
    fulfillment_condition_id UUID REFERENCES event_conditions(id),
    -- Magnitude tracking
    base_magnitude  REAL NOT NULL DEFAULT 0.5,
    -- Lifecycle
    state           TEXT NOT NULL DEFAULT 'active',  -- 'active', 'fulfilled', 'decayed'
    created_at_turn INT NOT NULL,
    fulfilled_at_turn INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_prophecy_type CHECK (
        prophecy_type IN ('foreshadowing', 'thematic_echo', 'causal_bridge', 'symbolic_link')
    ),
    CONSTRAINT chk_cascade_state CHECK (
        state IN ('active', 'fulfilled', 'decayed')
    )
);

CREATE INDEX idx_prophetic_session ON prophetic_cascades(session_id);
CREATE INDEX idx_prophetic_target ON prophetic_cascades(session_id, target_scene_id);
CREATE INDEX idx_prophetic_state ON prophetic_cascades(session_id, state);
```

### `prophetic_reinforcements`

Reinforcement events from additional sub-graph visits that strengthen an existing prophetic cascade:

```sql
CREATE TABLE prophetic_reinforcements (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    cascade_id      UUID NOT NULL REFERENCES prophetic_cascades(id) ON DELETE CASCADE,
    -- Reinforcement data
    magnitude       REAL NOT NULL DEFAULT 0.25,  -- Each reinforcement at half strength of base (default)
    source_visit_id UUID REFERENCES sub_graph_visit_history(id),
    reinforced_at_turn INT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_prophetic_reinforce_cascade ON prophetic_reinforcements(cascade_id);
```

**Purpose**: When the player re-enters a sub-graph and encounters events related to an existing prophetic cascade, a reinforcement row is added. The magnitude computation sums `base_magnitude + Σ(reinforcement_magnitudes × 0.5)`, all multiplied by exponential decay based on age.

---

## Application-Level Computations

### Composite Scoring Pipeline

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §The Composite Score

The composite scoring pipeline assembles data from multiple stores per candidate. The data flow for a single scene-entry context assembly:

```rust
async fn assemble_scored_candidates(
    pool: &PgPool,
    session: &SessionState,
    candidates: Vec<ContextCandidate>,
) -> Vec<ScoredCandidate> {
    // Phase 1: Batch-load gravitational state from AGE + PostgreSQL
    let reachable_scenes = age_query_reachable_scenes(pool, session.current_scene_id).await;
    let scene_masses = pg_load_scene_masses(pool, &reachable_scenes).await;
    let activation_states = pg_load_activation_states(pool, session.id).await;

    // Phase 2: Load sub-graph state from PostgreSQL
    let sub_graph_states = pg_load_sub_graph_states(pool, session.id).await;
    let boundary_permeabilities = compute_boundary_permeabilities(
        pool, session.id, session.current_turn,
    ).await;

    // Phase 3: Load prophetic cascades from PostgreSQL
    let prophecies = pg_load_active_prophecies(pool, session.id).await;

    // Phase 4: Score each candidate (pure computation, no I/O)
    candidates.into_iter()
        .map(|c| score_candidate(c, &reachable_scenes, &scene_masses,
            &activation_states, &sub_graph_states,
            &boundary_permeabilities, &prophecies, session.current_turn))
        .collect()
}
```

**Key insight**: All I/O happens in phases 1-3 (batch loads). Phase 4 is pure computation — no per-candidate database queries. This keeps the scoring pipeline within the < 5ms budget.

### Boundary Permeability Computation

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §Boundary Permeability Dynamics

```rust
async fn compute_boundary_permeabilities(
    pool: &PgPool,
    session_id: Uuid,
    current_turn: i32,
) -> HashMap<SubGraphLayerId, f32> {
    // Load sigmoid parameters from sub_graph_layers
    let layers = sqlx::query_as::<_, SubGraphLayerRow>(
        "SELECT id, permeability_min, permeability_max,
                permeability_steepness, permeability_midpoint
         FROM sub_graph_layers WHERE story_id = $1"
    )
    .bind(story_id)
    .fetch_all(pool).await?;

    // Load perturbations for this session
    let perturbations = sqlx::query_as::<_, BoundaryPerturbationRow>(
        "SELECT layer_id, magnitude, source_turn
         FROM boundary_perturbations
         WHERE session_id = $1
         ORDER BY source_turn"
    )
    .bind(session_id)
    .fetch_all(pool).await?;

    // Compute permeability for each layer
    layers.iter().map(|layer| {
        let base = sigmoid(
            current_turn as f32,
            layer.permeability_min,
            layer.permeability_max,
            layer.permeability_steepness,
            layer.permeability_midpoint,
        );

        let perturbation_sum: f32 = perturbations.iter()
            .filter(|p| p.layer_id == layer.id)
            .map(|p| {
                let age = current_turn - p.source_turn;
                p.magnitude * (-PERTURBATION_DECAY * age as f32).exp()
            })
            .sum();

        (layer.id, (base + perturbation_sum).clamp(0.0, 1.0))
    }).collect()
}
```

### Scene Cascade Resolution

**Source math**: [`traversal-friction.md`](../graph-strategy/traversal-friction.md) §Temporal Friction

The four-step cascade resolution at scene exit reads from multiple sources:

```rust
async fn resolve_scene_cascade(
    pool: &PgPool,
    scene: &ClosedScene,
    next_scene: &SceneMetadata,
    session_id: Uuid,
) {
    // Step 1: Direct relational changes (in-scene characters)
    // Read: scene.relational_changes (application state)
    // Write: AGE `:RELATES_TO` edge substrates, relational_edge_history
    apply_direct_changes(pool, &scene.relational_changes).await;

    // Step 2: In-scene communication events
    // Read: scene.communication_events (application state)
    // No write — temporary edges exist only during cascade computation

    // Step 3: Temporally-bounded cascade
    // Read: communication_affordances (PostgreSQL), scene narrative times
    // Compute: temporal_propagation_ceiling
    let setting_comm = pg_load_communication_affordances(
        pool, scene.setting_id, scene.story_id,
    ).await;
    let duration = next_scene.narrative_time - scene.narrative_time;
    let ceiling = temporal_propagation_ceiling(duration, &setting_comm);

    // Read: AGE cast subgraph (already loaded in-process as petgraph)
    // Compute: modified Dijkstra with ceiling-bounded BFS
    // Write: AGE edge substrates for propagated changes, relational_edge_history
    propagate_with_ceiling(pool, &scene, ceiling).await;

    // Step 4: Boundary perturbations if sub-graph events occurred
    // Write: boundary_perturbations
    if let Some(boundary_event) = &scene.boundary_event {
        record_boundary_perturbation(pool, session_id, boundary_event).await;
    }
}
```

### Prophetic Cascade Management

**Source math**: [`cross-graph-composition.md`](../graph-strategy/cross-graph-composition.md) §Prophetic Cascade Lifecycle

```rust
async fn process_sub_graph_exit(
    pool: &PgPool,
    session_id: Uuid,
    layer_id: Uuid,
    exit_events: &[SubGraphEvent],
) {
    // Record visit
    // Write: sub_graph_visit_history
    let visit_id = record_sub_graph_visit(pool, session_id, layer_id, exit_events).await;

    // Create new prophetic cascades for events that target parent-graph scenes
    // Read: authored cascade definitions (story config or sub_graph_layers metadata)
    // Write: prophetic_cascades
    for event in exit_events.iter().filter(|e| e.has_prophetic_target()) {
        create_prophetic_cascade(pool, session_id, layer_id, event).await;
    }

    // Reinforce existing cascades if this visit strengthens them
    // Read: prophetic_cascades (active, matching layer)
    // Write: prophetic_reinforcements
    let active = pg_load_active_prophecies_for_layer(pool, session_id, layer_id).await;
    for prophecy in &active {
        if exit_events.iter().any(|e| e.reinforces(prophecy)) {
            create_reinforcement(pool, prophecy.id, visit_id).await;
        }
    }

    // Record boundary perturbation (entity crossing back)
    // Write: boundary_perturbations
    record_boundary_perturbation(pool, session_id, &BoundaryPerturbation {
        layer_id,
        perturbation_type: "entity_crossing".to_string(),
        magnitude: 0.05,
        source_turn: current_turn,
        ..Default::default()
    }).await;
}
```

---

## Data Flow: Turn Cycle Integration

### Session Start

1. **Load sub-graph layer configuration** (~1ms) — sigmoid parameters, base_mass, completion_bonus
2. **Load existing sub-graph visit history** (~1ms) — previous visits for collective mass computation
3. **Load communication affordances** (~1ms) — setting-level temporal friction config
4. **Load active prophetic cascades** (~1ms) — ongoing prophecies from prior sessions (if resuming)

Total: ~4ms (in parallel with graph-specific session start loads)

### Scene Entry

1. **Compute boundary permeabilities** (~2ms) — sigmoid + perturbation sums for all active sub-graph layers
2. **Compute sub-graph collective masses** (~1ms) — base + visit weights + completion bonuses
3. **Assemble gravitational state** (~5ms) — AGE reachable scenes + PostgreSQL mass data + activation states
4. **Score context candidates** (~2ms) — five-factor composite scoring, pure computation
5. **Allocate token budget** (~0.5ms) — greedy knapsack over scored candidates

Total: ~10ms (partially parallelizable with graph-specific scene entry loads)

### Per-Turn

1. **Recompute prophetic pressure if state changed** (~0.5ms) — magnitude with decay
2. **Rescore candidates if significant state change** (~2ms) — only when approach predicates or revelations change
3. **No database writes for cross-cutting state** — all cross-cutting state changes happen at scene boundaries

Total: ~0-2.5ms (most turns require no cross-cutting recomputation)

### Scene Exit

1. **Resolve scene cascade** (~5ms) — four-step cascade resolution with temporal friction
2. **Record boundary perturbations if applicable** (~1ms) — INSERT into `boundary_perturbations`
3. **Fulfill prophetic cascades if target scene was played** (~1ms) — UPDATE `prophetic_cascades` state
4. **Decay unfulfilled prophecies** (~0.5ms) — mark decayed prophecies whose magnitude has fallen below threshold

Total: ~7.5ms

### Sub-Graph Exit (Special Case)

1. **Record visit to `sub_graph_visit_history`** (~1ms)
2. **Create prophetic cascades** (~1ms per cascade) — INSERT into `prophetic_cascades`
3. **Reinforce existing cascades** (~1ms) — INSERT into `prophetic_reinforcements`
4. **Record entity crossing perturbation** (~1ms)
5. **Recompute collective mass** (~0.5ms) — for the exited sub-graph

Total: ~5ms (rare event — happens only on sub-graph transitions)

---

## New Tables Summary

This document proposes 5 new tables and 1 ALTER TABLE:

| Table | Scope | Purpose | Estimated Rows |
|-------|-------|---------|---------------|
| `communication_affordances` | Story | Setting-level temporal friction config | 10-50 per story |
| `sub_graph_visit_history` | Session | Sub-graph visit tracking for collective mass | 5-20 per session |
| `boundary_perturbations` | Session | Permeability perturbation events | 10-50 per session |
| `prophetic_cascades` | Session | Forward-pointing approach vector modifiers | 1-10 per session |
| `prophetic_reinforcements` | Session | Reinforcement events for existing cascades | 0-20 per session |
| `sub_graph_layers` (ALTER) | Story | Sigmoid parameters, mass config | Existing rows |

### Cumulative New Tables Across All Documents

| Document | New Tables |
|----------|-----------|
| [relational-web-age.md](relational-web-age.md) | `relational_edge_details`, `relational_edge_history` |
| [setting-topology-age.md](setting-topology-age.md) | `setting_connections`, `entity_locations` |
| [narrative-graph-age.md](narrative-graph-age.md) | `scene_transitions`, `scene_activation_states` |
| [event-dag-age.md](event-dag-age.md) | `event_conditions`, `event_dependencies`, `event_condition_states` |
| cross-cutting-age.md (this) | `communication_affordances`, `sub_graph_visit_history`, `boundary_perturbations`, `prophetic_cascades`, `prophetic_reinforcements`, ALTER `sub_graph_layers` |
| **Total** | **14 new tables + 1 ALTER** |

Combined with the existing 12 migrations (14 tables), the full schema will have ~28 tables. This is appropriate for the system's complexity — each table has a clear, non-overlapping purpose.

---

## Migration: `20260301000018_create_cross_cutting_tables.sql`

This migration depends on all prior migrations:
- `sub_graph_layers` (existing — for ALTER and foreign keys)
- `stories`, `settings`, `sessions`, `scenes` (existing — foreign keys)
- `event_conditions` (from `000017` — for prophetic cascade fulfillment reference)

Order within the migration:
1. ALTER `sub_graph_layers` (add sigmoid + mass columns)
2. CREATE `communication_affordances`
3. CREATE `sub_graph_visit_history`
4. CREATE `boundary_perturbations`
5. CREATE `prophetic_cascades`
6. CREATE `prophetic_reinforcements`

---

## AGE Capability Assessment

The cross-cutting computations do not use AGE directly — they are application-level algorithms. However, they depend on AGE data from the other graphs:

| Dependency | AGE Query | Source Document |
|-----------|-----------|-----------------|
| Reachable scenes for gravitational modifier | `MATCH (s:Scene)-[:TRANSITIONS_TO*1..4]->(t:Scene)` | [narrative-graph-age.md](narrative-graph-age.md) |
| Cast subgraph for cascade propagation | `MATCH (a:RelEntity)-[r:RELATES_TO]->(b:RelEntity)` | [relational-web-age.md](relational-web-age.md) |
| Scenes at reachable settings | `MATCH (s:Setting)-[:CONNECTS_TO*1..3]->(:Setting)<-[:LOCATED_AT]-(sc:Scene)` | [setting-topology-age.md](setting-topology-age.md) |

The cross-cutting layer is a **consumer** of AGE query results, not a contributor to AGE schema.

---

## Questions for TAS-244 Spike

1. **Batch AGE query latency**: The composite scoring pipeline makes 1-3 AGE queries per scene entry (reachable scenes, cast subgraph, scenes at settings). What is the total latency for executing all three queries in sequence? Can they be parallelized via `tokio::join!`?
2. **Perturbation table growth**: With 10-50 boundary perturbations per session and exponential decay, should we periodically prune perturbations whose decayed magnitude is below threshold? Or is the table small enough that carrying historical rows has negligible cost?
3. **Sub-graph visit emotional residue**: The `emotional_intensity` and `emotional_valence` are written at sub-graph exit. Validate that the Storykeeper's scene-exit processing window (~20ms budget) can accommodate both the standard cascade resolution AND the sub-graph visit recording.
4. **Configuration vs computed state boundary**: The `sub_graph_layers` ALTER adds authored columns (base_mass, completion_bonus) alongside dynamic sigmoid parameters. Should the sigmoid midpoint be story-authored or computed from story structure (e.g., midpoint at 60% of expected total turns)?
