# Event Dependency DAG Mathematics

## Purpose

This document formalizes the mathematical operations on the **event dependency DAG** — the directed acyclic graph that models how narrative events depend on each other, what can happen given what has happened, and what can no longer happen. It covers: DAG structure, acyclicity validation, topological ordering, evaluable frontier maintenance, exclusion cascades, and amplification compounding.

**Prerequisites**: None. This document is self-contained.

**Source material**: [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) §Graph 4, [`event-dependency-graph.md`](../event-dependency-graph.md).

---

## The Event Dependency DAG

### Definition

The event dependency DAG is a directed acyclic graph `Δ = (N, D)` where:

- `N` is the set of event condition nodes
- `D ⊆ N × N` is the set of dependency edges

Each edge `(n_i, n_j) ∈ D` is typed:

```rust
enum DependencyType {
    Requires,      // n_j cannot resolve until n_i has resolved
    Excludes,      // n_i's resolution makes n_j permanently unreachable
    Enables,       // n_i's resolution makes n_j possible (but not guaranteed)
    Amplifies(f32), // n_i's resolution multiplies n_j's consequence magnitude
}
```

Each node has a **resolution state**:

```rust
enum ResolutionState {
    Unresolved,          // not yet met, may or may not be reachable
    Resolved(TurnId),    // condition met at this turn
    Unreachable,         // can no longer be met
    Orphaned,            // unreachable inherited from ancestor
}
```

### Acyclicity Invariant

The DAG must be acyclic — an event cannot depend on something that depends on it. This is guaranteed by temporal ordering: events happen in time, and dependencies flow forward.

---

## Acyclicity Validation

### DFS Back-Edge Detection

When event conditions are authored, the system validates acyclicity using DFS-based cycle detection:

```rust
fn validate_acyclic(dag: &EventDag) -> Result<(), CycleError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Color { White, Gray, Black }

    let mut colors: HashMap<EventConditionId, Color> = dag.nodes.keys()
        .map(|&id| (id, Color::White))
        .collect();

    for &node in dag.nodes.keys() {
        if colors[&node] == Color::White {
            if let Some(cycle) = dfs_visit(dag, node, &mut colors) {
                return Err(CycleError { cycle });
            }
        }
    }

    Ok(())
}

fn dfs_visit(
    dag: &EventDag,
    node: EventConditionId,
    colors: &mut HashMap<EventConditionId, Color>,
) -> Option<Vec<EventConditionId>> {
    colors.insert(node, Color::Gray); // Currently visiting

    for neighbor in dag.successors(node) {
        match colors[&neighbor] {
            Color::Gray => {
                // Back edge — cycle detected
                return Some(vec![node, neighbor]);
            }
            Color::White => {
                if let Some(mut cycle) = dfs_visit(dag, neighbor, colors) {
                    cycle.insert(0, node);
                    return Some(cycle);
                }
            }
            Color::Black => {} // Already processed, no cycle
        }
    }

    colors.insert(node, Color::Black); // Done visiting
    None
}
```

Complexity: `O(|N| + |D|)` — linear in the size of the DAG.

### When to Validate

- **On content load**: When a story's authored event conditions are loaded into the system
- **On dynamic node addition**: If the system supports emergent (non-authored) nodes, validate after each addition
- **Never at runtime evaluation**: The DAG is validated once; runtime operations assume acyclicity

---

## Topological Sort

### Definition

A **topological ordering** of the DAG is a linear ordering of nodes such that for every edge `(n_i, n_j)`, `n_i` appears before `n_j`. This ordering represents the valid evaluation sequence — we can only evaluate a node after all its dependencies.

### Kahn's Algorithm

```rust
fn topological_sort(dag: &EventDag) -> Vec<EventConditionId> {
    let mut in_degree: HashMap<EventConditionId, usize> = dag.nodes.keys()
        .map(|&id| (id, 0))
        .collect();

    // Count incoming edges (Requires and Enables only — Excludes and Amplifies
    // don't affect evaluation order)
    for &(from, to) in dag.edges.keys() {
        let edge = &dag.edges[&(from, to)];
        if matches!(edge.dependency_type, DependencyType::Requires | DependencyType::Enables) {
            *in_degree.entry(to).or_default() += 1;
        }
    }

    // Seed with zero-in-degree nodes (leaf conditions: authored preconditions)
    let mut queue: VecDeque<EventConditionId> = in_degree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut sorted = Vec::with_capacity(dag.nodes.len());

    while let Some(node) = queue.pop_front() {
        sorted.push(node);

        for successor in dag.successors(node) {
            let edge = &dag.edges[&(node, successor)];
            if matches!(edge.dependency_type, DependencyType::Requires | DependencyType::Enables) {
                let deg = in_degree.get_mut(&successor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(successor);
                }
            }
        }
    }

    sorted
}
```

Complexity: `O(|N| + |D|)`.

### Use Case

The topological sort determines **evaluation order** — when processing a batch of newly resolved events, we evaluate nodes in topological order to ensure that a node's dependencies are always processed before the node itself. This prevents order-dependent bugs where evaluating node B before node A could miss the fact that A's resolution enables B.

---

## The Evaluable Frontier

### Definition

The **evaluable frontier** is the set of unresolved nodes whose all `Requires` dependencies are resolved. These are the nodes that *could* resolve on the next turn if their conditions are met.

```rust
fn evaluable_frontier(dag: &EventDag) -> Vec<EventConditionId> {
    dag.nodes.iter()
        .filter(|(_, node)| node.resolution_state == ResolutionState::Unresolved)
        .filter(|(&id, _)| {
            dag.predecessors_of_type(id, DependencyType::Requires)
                .all(|pred| {
                    matches!(dag.nodes[&pred].resolution_state,
                             ResolutionState::Resolved(_))
                })
        })
        .map(|(&id, _)| id)
        .collect()
}
```

### Incremental Maintenance

Rather than recomputing the frontier from scratch each turn, we maintain it incrementally:

```rust
fn update_frontier(
    frontier: &mut HashSet<EventConditionId>,
    newly_resolved: &[EventConditionId],
    dag: &EventDag,
) {
    for &resolved_node in newly_resolved {
        // Remove the resolved node from the frontier
        frontier.remove(&resolved_node);

        // Check if any successors become evaluable
        for successor in dag.successors(resolved_node) {
            let edge = &dag.edges[&(resolved_node, successor)];
            if edge.dependency_type == DependencyType::Requires
                && dag.nodes[&successor].resolution_state == ResolutionState::Unresolved
            {
                // Check if ALL Requires dependencies are now resolved
                let all_requires_met = dag
                    .predecessors_of_type(successor, DependencyType::Requires)
                    .all(|pred| matches!(
                        dag.nodes[&pred].resolution_state,
                        ResolutionState::Resolved(_)
                    ));

                if all_requires_met {
                    frontier.insert(successor);
                }
            }
        }
    }
}
```

This is `O(k × d)` where `k` is the number of newly resolved nodes and `d` is the average out-degree — constant work per turn.

---

## Incremental Resolution Propagation

### The Per-Turn Algorithm

When events are committed at the end of a turn, the DAG evaluates incrementally:

```rust
fn propagate_resolution(
    dag: &mut EventDag,
    committed_events: &[CommittedEvent],
    truth_set: &TruthSet,
    frontier: &mut HashSet<EventConditionId>,
) -> PropagationResult {
    let mut newly_resolved = Vec::new();
    let mut newly_unreachable = Vec::new();
    let mut consequences = Vec::new();

    // Step 1: Match committed events against frontier nodes
    for &node_id in frontier.iter() {
        let node = &dag.nodes[&node_id];
        if node.resolution_condition.evaluate(truth_set, committed_events) {
            newly_resolved.push(node_id);
        }
    }

    // Step 2: Mark resolved
    for &node_id in &newly_resolved {
        dag.nodes.get_mut(&node_id).unwrap().resolution_state =
            ResolutionState::Resolved(current_turn());
    }

    // Step 3: Propagate exclusions
    for &node_id in &newly_resolved {
        let excluded = propagate_exclusion(dag, node_id);
        newly_unreachable.extend(excluded);
    }

    // Step 4: Fire consequences
    for &node_id in &newly_resolved {
        if let Some(consequence) = &dag.nodes[&node_id].consequence {
            let magnitude = compute_consequence_magnitude(dag, node_id);
            consequences.push((consequence.clone(), magnitude));
        }
    }

    // Step 5: Update frontier
    update_frontier(frontier, &newly_resolved, dag);

    // Also remove newly unreachable nodes from frontier
    for &unreachable in &newly_unreachable {
        frontier.remove(&unreachable);
    }

    PropagationResult { newly_resolved, newly_unreachable, consequences }
}
```

---

## Exclusion Cascade

### Definition

When a node resolves and has `Excludes` edges, the excluded nodes become permanently `Unreachable`. Unreachability then propagates transitively to downstream nodes — nodes that can *only* be reached through unreachable nodes become `Orphaned`.

### Algorithm: BFS Exclusion Propagation

```rust
fn propagate_exclusion(
    dag: &mut EventDag,
    resolved_node: EventConditionId,
) -> Vec<EventConditionId> {
    let mut newly_unreachable = Vec::new();
    let mut queue = VecDeque::new();

    // Find directly excluded nodes
    for successor in dag.successors(resolved_node) {
        let edge = &dag.edges[&(resolved_node, successor)];
        if edge.dependency_type == DependencyType::Excludes {
            if dag.nodes[&successor].resolution_state == ResolutionState::Unresolved {
                dag.nodes.get_mut(&successor).unwrap().resolution_state =
                    ResolutionState::Unreachable;
                newly_unreachable.push(successor);
                queue.push_back(successor);
            }
        }
    }

    // BFS: propagate orphaning to exclusive descendants
    while let Some(unreachable_node) = queue.pop_front() {
        for successor in dag.successors(unreachable_node) {
            let edge = &dag.edges[&(unreachable_node, successor)];
            if edge.dependency_type != DependencyType::Requires {
                continue; // Only Requires edges create orphaning
            }

            let successor_node = &dag.nodes[&successor];
            if successor_node.resolution_state != ResolutionState::Unresolved {
                continue; // Already resolved or already unreachable
            }

            // Check if ALL Requires predecessors are either Resolved or Unreachable/Orphaned
            // If any Requires predecessor is still Unresolved, this node is NOT orphaned
            // (it might be reachable through the other predecessor)
            let all_requires_dead = dag
                .predecessors_of_type(successor, DependencyType::Requires)
                .all(|pred| !matches!(
                    dag.nodes[&pred].resolution_state,
                    ResolutionState::Unresolved
                ));

            let any_requires_live = dag
                .predecessors_of_type(successor, DependencyType::Requires)
                .any(|pred| matches!(
                    dag.nodes[&pred].resolution_state,
                    ResolutionState::Resolved(_)
                ));

            if all_requires_dead && !any_requires_live {
                // All predecessors are dead (Unreachable/Orphaned), none Resolved
                dag.nodes.get_mut(&successor).unwrap().resolution_state =
                    ResolutionState::Orphaned;
                newly_unreachable.push(successor);
                queue.push_back(successor);
            }
        }
    }

    newly_unreachable
}
```

### Orphaning Logic

The key distinction: a node becomes `Orphaned` only if **all** of its `Requires` predecessors are either `Unreachable` or `Orphaned`, and **none** are `Resolved`. If even one `Requires` predecessor is still `Unresolved`, the node is not orphaned — it still has a chance.

This handles the case where a node has multiple paths to resolution. If one path is excluded but another remains open, the node stays `Unresolved`.

### TFATD Worked Example: Exclusion Cascade

**Scenario**: The player refuses to enter the Wood at S2 (decides not to accept the quest).

```
Node: quest_accepted (S2 departure → entering Wood)
Resolution: Unreachable (player refused)

Exclusion cascade:
  quest_accepted --Requires--> enter_shadowed_wood → Orphaned
  enter_shadowed_wood --Requires--> haunt_encounter (S4) → Orphaned
  enter_shadowed_wood --Requires--> stream_crossing (S5) → Orphaned
  enter_shadowed_wood --Requires--> hidden_path_revelation (S6) → Orphaned
  hidden_path_revelation --Requires--> sarah_power_revealed → Orphaned
```

The entire Wood subgraph becomes orphaned. The Storykeeper notes these as "roads not taken" — the system enters narrative contraction mode. The remaining story is about sitting with a dying brother.

---

## Amplification Compounding

### Definition

`Amplifies` edges don't block or enable resolution — they modify the **consequence magnitude** when the target node resolves. Amplification is multiplicative: multiple amplifying edges compound.

```rust
fn compute_consequence_magnitude(
    dag: &EventDag,
    node: EventConditionId,
) -> f32 {
    let base_magnitude = dag.nodes[&node].consequence
        .as_ref()
        .map(|c| c.base_magnitude)
        .unwrap_or(1.0);

    let amplification = dag.predecessors(node)
        .filter(|&(_, edge)| matches!(edge.dependency_type, DependencyType::Amplifies(_)))
        .filter(|&(pred_id, _)| matches!(
            dag.nodes[&pred_id].resolution_state,
            ResolutionState::Resolved(_)
        ))
        .map(|(_, edge)| match edge.dependency_type {
            DependencyType::Amplifies(weight) => weight,
            _ => 1.0,
        })
        .product::<f32>();

    // Clamp to prevent runaway amplification
    let clamped_amplification = amplification.min(MAX_AMPLIFICATION); // default: 5.0

    base_magnitude * clamped_amplification
}
```

### Compounding Behavior

With three `Amplifies` edges carrying weights 1.2, 1.3, and 1.5:

```
total_amplification = 1.2 × 1.3 × 1.5 = 2.34
```

A consequence with base magnitude 1.0 becomes 2.34 — more than double. The narrative effect: a discovery amplified by accumulated suspicion, prior revelations, and relational tension lands dramatically harder than one in isolation.

### Configurable Bounds

`MAX_AMPLIFICATION` prevents unbounded compounding in stories with many amplifying paths:

| Stories with... | Recommended MAX_AMPLIFICATION |
|----------------|-------------------------------|
| Few amplifiers (< 5) | 5.0 (generous) |
| Many amplifiers (10+) | 3.0 (constrained) |
| Testing/debugging | 10.0 (for visibility) |

### TFATD Worked Example: Amplified Discovery

**Node**: `sarah_discovers_hidden_perception` (S6: Sarah can see paths the Wolf cannot)

**Amplifying predecessors**:

| Predecessor | Weight | Status | Reasoning |
|-------------|--------|--------|-----------|
| `kate_water_blessing` (S3) | 1.3 | Resolved | The blessing primed Sarah's perception |
| `wolf_trust_building` (S4-S5) | 1.2 | Resolved | Trust enables vulnerability, which enables perception |
| `exhaustion_accumulation` (connective) | 1.1 | Resolved | Exhaustion thins the boundary between worlds |

```
amplification = 1.3 × 1.2 × 1.1 = 1.716

consequence_magnitude = 1.0 × 1.716 = 1.716
```

Sarah's revelation lands 72% harder because of the accumulated preparation. A player who rushed through (skipping Wolf trust-building) would see:

```
amplification = 1.3 × 1.0 × 1.0 = 1.3
consequence_magnitude = 1.3
```

Still amplified by Kate's blessing, but less impactful. The system doesn't prevent the revelation — it adjusts its narrative weight.

---

## Cross-Graph Reachability

### The Problem

A node's reachability depends not just on the DAG structure but on the **narrative graph** — if the scene where a discovery condition can resolve is no longer reachable, the node is effectively unreachable even if its DAG predecessors are fine.

### Algorithm

```rust
fn check_cross_graph_reachability(
    dag: &EventDag,
    narrative_graph: &NarrativeGraph,
    current_scene: SceneId,
    node: EventConditionId,
) -> bool {
    let node_data = &dag.nodes[&node];

    // What scenes can resolve this condition?
    let resolving_scenes = node_data.resolution_condition.required_scenes();

    if resolving_scenes.is_empty() {
        return true; // No scene requirement — resolvable anywhere
    }

    // Can we reach any of those scenes from the current position?
    let reachable_scenes = narrative_graph.reachable_scenes(current_scene);

    resolving_scenes.iter()
        .any(|scene| reachable_scenes.contains(scene))
}
```

This is evaluated at **scene boundaries** — when the player exits a scene, the Storykeeper checks which DAG nodes have lost their resolving scenes and marks them `Unreachable`.

---

## Critical Path Analysis

### Definition

The **critical path** to a narrative goal is the longest dependency chain that must be resolved. This determines the minimum number of turns/events required to reach the goal.

```rust
fn critical_path(
    dag: &EventDag,
    target: EventConditionId,
) -> Vec<EventConditionId> {
    // Longest path in a DAG (the reverse of shortest — we want the bottleneck)
    let mut longest_distance: HashMap<EventConditionId, usize> = HashMap::new();
    let mut predecessor: HashMap<EventConditionId, EventConditionId> = HashMap::new();

    // Process in topological order
    let topo_order = topological_sort(dag);

    for &node in &topo_order {
        longest_distance.insert(node, 0);
    }

    for &node in &topo_order {
        for successor in dag.successors(node) {
            let edge = &dag.edges[&(node, successor)];
            if edge.dependency_type == DependencyType::Requires {
                let new_dist = longest_distance[&node] + 1;
                if new_dist > *longest_distance.get(&successor).unwrap_or(&0) {
                    longest_distance.insert(successor, new_dist);
                    predecessor.insert(successor, node);
                }
            }
        }
    }

    // Reconstruct path
    let mut path = vec![target];
    let mut current = target;
    while let Some(&pred) = predecessor.get(&current) {
        path.push(pred);
        current = pred;
    }
    path.reverse();
    path
}
```

Complexity: `O(|N| + |D|)` — linear in the DAG size, since we process in topological order.

### Use Case

Critical path length determines **pacing** — how many narrative beats must occur before a goal is achievable. A story designer can use this to ensure that climactic moments require sufficient buildup:

```
Critical path to sarah_discovers_hidden_perception:
  tommy_is_ill → quest_accepted → enter_shadowed_wood → kate_water_blessing
    → wolf_trust_building → sarah_discovers_hidden_perception

Length: 5 steps minimum
```

---

## Computational Summary

| Operation | Algorithm | Complexity | Expected Latency (500 nodes) |
|-----------|-----------|------------|------------------------------|
| Acyclicity validation | DFS cycle detection | O(N + D) | < 1ms |
| Topological sort | Kahn's algorithm | O(N + D) | < 1ms |
| Evaluable frontier | In-degree filtering | O(N) initial, O(k×d) incremental | < 1ms |
| Resolution propagation | Incremental frontier update | O(k × d) per turn | < 0.5ms |
| Exclusion cascade | BFS from excluded nodes | O(N + D) worst case, typically O(k) | < 1ms |
| Amplification computation | Product of amplifying edges | O(d) per node | < 0.01ms |
| Cross-graph reachability | Scene reachability + node mapping | O(R + N) | < 5ms |
| Critical path | Longest path in DAG | O(N + D) | < 1ms |

All operations are well within the per-turn budget. The DAG evaluation adds < 2ms to turn commitment.

---

## Theoretical Foundations

### What We Borrow

| Concept | Source | Our Adaptation |
|---------|--------|---------------|
| **DAG topological sort** | Kahn (1962), "Topological sorting of large networks" | Standard algorithm for evaluation ordering |
| **Cycle detection** | DFS back-edge detection (Cormen et al., *CLRS*) | Standard DFS coloring for authored content validation |
| **Critical path method** | CPM (Kelley & Walker, 1959) | Longest path in DAG for pacing analysis |
| **Incremental graph maintenance** | Dynamic graph algorithms | Frontier maintained incrementally per turn |

### What We Invent

| Concept | Novelty |
|---------|---------|
| **Typed dependency edges** | The four types (Requires/Excludes/Enables/Amplifies) specific to narrative dependencies |
| **Orphaning propagation** | Transitively propagating unreachability through exclusively-dependent nodes |
| **Amplification compounding** | Multiplicative consequence amplification from narrative preparation |
| **Cross-graph reachability** | Node reachability depending on scene reachability in a separate graph |
| **Resolution states** | Four-state model (Unresolved/Resolved/Unreachable/Orphaned) for narrative tracking |

### What We Defer

- **Probabilistic resolution** — Nodes with probability-of-resolution estimates for narrative planning. Useful for AI story management but adds complexity.
- **Partial resolution** — Nodes that are "partially" resolved (e.g., 60% of threshold conditions met). Currently handled by EmergentState nodes with threshold conditions.
- **DAG versioning** — Adding nodes/edges to a live DAG during gameplay. Currently the DAG is loaded at story start and only resolution state changes at runtime.
