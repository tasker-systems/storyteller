# Setting Topology Mathematics

## Purpose

This document formalizes the mathematical operations on the **setting topology** — the spatial graph of locations and connections that the Storykeeper uses for pathfinding, entity proximity computation, and spatial constraint enforcement. The setting topology is the simplest of the four graphs mathematically — it is primarily a weighted graph pathfinding problem.

**Prerequisites**: None. This document is self-contained.

**Source material**: [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) §Graph 3, [`narrative-graph-case-study-tfatd.md`](../narrative-graph-case-study-tfatd.md).

---

## The Setting Topology as a Weighted Graph

### Definition

The setting topology is a graph `Σ = (L, C)` where:

- `L` is the set of settings (locations)
- `C ⊆ L × L` is the set of spatial connections

Connections may be bidirectional, one-way, or conditional. Each connection carries a **traversal cost vector** — a multi-dimensional cost that varies by entity.

```rust
struct SettingTopology {
    settings: HashMap<SettingId, SettingVertex>,
    connections: HashMap<(SettingId, SettingId), SpatialConnection>,
}
```

### Scale

TFATD Part I implies ~8-10 distinct settings. A full novel might have 30-50 settings. A large-scale story (open world) might have 100-200 settings. The setting topology is typically the smallest of the four graphs.

---

## Multi-Dimensional Traversal Cost

### Definition

Each connection carries a traversal cost vector in ℝ³:

```
cost(σ_i, σ_j) = (time, effort, risk)
```

All components are non-negative. Zero cost means the connection is trivial (adjacent rooms). High cost means the connection is significant (crossing a mountain range).

```rust
struct TraversalCost {
    time: f32,    // narrative time units
    effort: f32,  // physical difficulty [0.0, 1.0]
    risk: f32,    // danger level [0.0, 1.0]
}
```

### Entity-Specific Cost Functions

Different entities experience the same connection differently. The Wolf traverses the Shadowed Wood effortlessly; Sarah finds it exhausting:

```rust
fn effective_cost(
    base_cost: &TraversalCost,
    entity: &EntityCapabilities,
) -> f32 {
    let time_cost = base_cost.time / entity.speed_factor;
    let effort_cost = base_cost.effort * entity.effort_multiplier;
    let risk_cost = base_cost.risk * entity.risk_sensitivity;

    // Weighted sum — configurable per context
    W_TIME * time_cost + W_EFFORT * effort_cost + W_RISK * risk_cost
}
```

Default weights: `W_TIME = 0.4`, `W_EFFORT = 0.3`, `W_RISK = 0.3`.

### TFATD Worked Example: Setting Connections

| Connection | Time | Effort | Risk | Notes |
|-----------|------|--------|------|-------|
| Home → Adam's Dwelling | 0.5 | 0.2 | 0.1 | Short walk, familiar |
| Adam's Dwelling → Wood Entry | 1.0 | 0.3 | 0.3 | Threshold crossing |
| Wood Entry → The Rise (S4) | 3.0 | 0.6 | 0.5 | Days of travel, mist, cold |
| The Rise → Abandoned Village | 0.5 | 0.3 | 0.4 | Short but uncertain |
| The Rise → Stream (S5) | 1.0 | 0.4 | 0.3 | Continued travel |
| Stream → Other Bank (S6) | 0.5 | 0.5 | 0.6 | Stream crossing, liminal |

For Sarah (speed_factor=0.8, effort_multiplier=1.2, risk_sensitivity=1.0):
```
Home → Adam's: 0.4×(0.5/0.8) + 0.3×(0.2×1.2) + 0.3×(0.1×1.0) = 0.25 + 0.072 + 0.03 = 0.352
Wood Entry → Rise: 0.4×(3.0/0.8) + 0.3×(0.6×1.2) + 0.3×(0.5×1.0) = 1.5 + 0.216 + 0.15 = 1.866
```

For the Wolf (speed_factor=2.0, effort_multiplier=0.3, risk_sensitivity=0.2):
```
Wood Entry → Rise: 0.4×(3.0/2.0) + 0.3×(0.6×0.3) + 0.3×(0.5×0.2) = 0.6 + 0.054 + 0.03 = 0.684
```

The Wolf traverses the same path at roughly one-third the effective cost.

---

## Weighted Shortest Path

### Dijkstra's Algorithm

The primary pathfinding operation: find the minimum-cost path between two settings.

```rust
fn shortest_path(
    topology: &SettingTopology,
    from: SettingId,
    to: SettingId,
    entity: &EntityCapabilities,
) -> Option<(Vec<SettingId>, f32)> {
    let mut distances: HashMap<SettingId, f32> = HashMap::new();
    let mut predecessors: HashMap<SettingId, SettingId> = HashMap::new();
    let mut queue = BinaryHeap::new();

    distances.insert(from, 0.0);
    queue.push(PathEntry { setting: from, cost: 0.0 });

    while let Some(current) = queue.pop() {
        if current.setting == to {
            return Some((reconstruct_path(&predecessors, from, to), current.cost));
        }

        if current.cost > *distances.get(&current.setting).unwrap_or(&f32::MAX) {
            continue; // Stale entry
        }

        for neighbor in topology.adjacent(current.setting) {
            let connection = topology.connection(current.setting, neighbor);

            // Check traversal conditions
            if !connection.traversable_by(entity) {
                continue;
            }

            let edge_cost = effective_cost(&connection.traversal_cost, entity);
            let new_cost = current.cost + edge_cost;

            if new_cost < *distances.get(&neighbor).unwrap_or(&f32::MAX) {
                distances.insert(neighbor, new_cost);
                predecessors.insert(neighbor, current.setting);
                queue.push(PathEntry { setting: neighbor, cost: new_cost });
            }
        }
    }

    None // No path exists
}
```

Complexity: `O((|L| + |C|) log |L|)` with a binary heap. At our scale (< 200 settings), this completes in microseconds.

---

## Conditional and Gated Edges

### Definition

Some connections are only traversable under specific conditions:

```rust
enum Directionality {
    Bidirectional,
    OneWay,
    Conditional {
        forward_condition: TriggerPredicate,
        reverse_condition: TriggerPredicate,
    },
}
```

### Integration with Pathfinding

Conditional edges are evaluated during pathfinding — a connection is only traversable if its conditions are currently satisfied:

```rust
fn traversable_by(connection: &SpatialConnection, entity: &EntityCapabilities) -> bool {
    match &connection.directionality {
        Directionality::Bidirectional => true,
        Directionality::OneWay => true, // only in the forward direction
        Directionality::Conditional { forward_condition, .. } => {
            forward_condition.evaluate(&entity.truth_set)
        }
    }
}
```

### Truth-Set Evaluation

Traversal conditions reference the truth set — the set of propositions that are currently true in the story:

```
gate_condition = All(
    CharacterHas(kate_blessing),
    EventOccurred(adam_summoned_wolf)
)
```

This means the path into the Shadowed Wood requires Kate's blessing and Adam's Wolf. Without these, the connection exists but is not traversable.

### TFATD Worked Example: Gated Connections

| Connection | Condition | Status (after S3) |
|-----------|-----------|------------------|
| Home → Shadowed Wood | `kate_blessing AND wolf_summoned` | Open (both gates satisfied) |
| Rise → Abandoned Village | `village_invitation` | Open (structural in S4) |
| Stream → Dark Bank | `sarah_sees_hidden_paths` | Open only after S6's revelation |

A player who skips S3 (refuses Kate's blessing) finds the Wood Entry → Shadowed Wood connection **closed**. The path exists structurally but is gated. This is how setting topology enforces narrative prerequisites without breaking spatial consistency.

---

## Budget-Bounded Reachability

### Definition

Given a starting setting and a traversal budget, find all settings reachable within that budget:

```rust
fn reachable_within_budget(
    topology: &SettingTopology,
    from: SettingId,
    budget: f32,
    entity: &EntityCapabilities,
) -> Vec<(SettingId, f32)> {
    let mut reachable = Vec::new();
    let mut distances: HashMap<SettingId, f32> = HashMap::new();
    let mut queue = BinaryHeap::new();

    distances.insert(from, 0.0);
    queue.push(PathEntry { setting: from, cost: 0.0 });

    while let Some(current) = queue.pop() {
        if current.cost > budget {
            continue; // Beyond budget
        }

        reachable.push((current.setting, current.cost));

        for neighbor in topology.adjacent(current.setting) {
            let connection = topology.connection(current.setting, neighbor);
            if !connection.traversable_by(entity) {
                continue;
            }

            let edge_cost = effective_cost(&connection.traversal_cost, entity);
            let new_cost = current.cost + edge_cost;

            if new_cost <= budget
                && new_cost < *distances.get(&neighbor).unwrap_or(&f32::MAX)
            {
                distances.insert(neighbor, new_cost);
                queue.push(PathEntry { setting: neighbor, cost: new_cost });
            }
        }
    }

    reachable
}
```

This is Dijkstra's algorithm with early termination at the budget boundary.

### Use Case: Entity Proximity

The primary consumer of budget-bounded reachability is the **entity proximity query** — a cross-graph operation that combines setting topology with entity locations:

```rust
fn entities_nearby(
    topology: &SettingTopology,
    entity_locations: &HashMap<EntityId, SettingId>,
    center: SettingId,
    budget: f32,
    entity: &EntityCapabilities,
) -> Vec<(EntityId, f32)> {
    let reachable = reachable_within_budget(topology, center, budget, entity);
    let reachable_settings: HashMap<SettingId, f32> = reachable.into_iter().collect();

    entity_locations.iter()
        .filter_map(|(&entity_id, &setting_id)| {
            reachable_settings.get(&setting_id)
                .map(|&cost| (entity_id, cost))
        })
        .collect()
}
```

### TFATD Worked Example: Reachability from the Rise

From The Rise (S4 setting), with budget 3.0 for Sarah:

| Setting | Cost | Reachable? |
|---------|------|-----------|
| The Rise | 0.0 | Yes (start) |
| Stream | 1.0 | Yes |
| Abandoned Village | 0.5 | Yes |
| Other Bank | 1.5 | Yes |
| Wood Entry | 1.87 | Yes |
| Home | 2.22 | Yes |
| Deeper Wood | 2.5 | Yes (est.) |

Sarah can reach every setting she has visited within budget 3.0 — but the costs differ. The Wolf, with lower effective costs, could reach even further within the same budget.

---

## Computational Summary

| Operation | Algorithm | Complexity | Expected Latency (200 settings) |
|-----------|-----------|------------|-------------------------------|
| Shortest path | Dijkstra | O((L + C) log L) | < 1ms |
| Budget-bounded reachability | Dijkstra with cutoff | O((L + C) log L) | < 1ms |
| Entity proximity | Reachability + location join | O((L + C) log L + E) | < 2ms |
| Adjacent settings | Direct lookup | O(1) | < 0.01ms |
| Condition evaluation | Truth-set predicate | O(P) per edge | < 0.1ms |

The setting topology is computationally trivial at our scale. All operations complete in under 2ms even at projected maximum scale.

---

## Theoretical Foundations

### What We Borrow

| Concept | Source | Our Adaptation |
|---------|--------|---------------|
| **Dijkstra's algorithm** | Dijkstra (1959), "A note on two problems in connexion with graphs" | Standard weighted shortest path, entity-specific cost functions |
| **Budget-bounded search** | Constrained shortest path (network optimization) | Dijkstra with budget cutoff for reachability |
| **Conditional edges** | Conditional graphs (AI planning, game AI) | Edges gated by truth-set predicates |

### What We Invent

| Concept | Novelty |
|---------|---------|
| **Multi-dimensional traversal cost** | The (time, effort, risk) triple as a domain-specific cost vector |
| **Entity-specific cost functions** | Different entities experiencing the same physical path with different effective costs |
| **Truth-set gated connections** | Spatial connections gated by narrative predicates (not just keys/items) |

### What We Defer

- **A\* with narrative heuristics** — Using gravitational pull as a heuristic for pathfinding (search toward high-mass scenes first). Would improve performance on larger topologies but unnecessary at our scale.
- **Dynamic topology** — Settings that change during play (bridges collapse, paths open). Currently handled by conditional edges; dynamic edge creation/deletion is deferred.
- **Multi-agent pathfinding** — Computing paths for multiple entities simultaneously with conflict resolution. Not needed until entity AI becomes more sophisticated.
