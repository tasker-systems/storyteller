# Relational Web Mathematics

## Purpose

This document formalizes the mathematical operations on the **relational web** — the directed graph of entity relationships that the Storykeeper uses for prediction features, context assembly, and frame computation. It covers: substrate as a vector space, topological role classification, cluster detection, triadic patterns, and cascade propagation with the friction model.

**Prerequisites**: None. This document is self-contained.

**Source material**: [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) §Graph 1, [`relational-web-tfatd.md`](../relational-web-tfatd.md).

---

## The Relational Web as a Directed Graph

### Definition

The relational web is a directed graph `W = (V, E)` where:

- `V` is the set of entities (characters, presences, conditions) at `PromotionTier::Tracked` or higher
- `E ⊆ V × V` is the set of directed relational edges

Each edge `(e_i, e_j) ∈ E` carries a **substrate vector** and metadata. The graph is typically not complete — not every pair of entities has a relationship. Edges are asymmetric: `(e_i, e_j)` and `(e_j, e_i)` are distinct edges with potentially different substrate values.

```rust
struct RelationalWeb {
    vertices: HashMap<EntityId, RelationalVertex>,
    edges: HashMap<(EntityId, EntityId), RelationalEdge>,
}
```

### Scale

For a story with `n` entities at Tracked tier or above, the relational web has at most `n(n-1)` directed edges. In practice, the graph is sparse — TFATD has 6 characters and ~20 directed edges (not all pairs have relationships). A production story might have 50-100 tracked entities with 200-500 edges.

---

## The Substrate Vector Space

### Definition

Each directed edge carries a **substrate vector** in ℝ⁵:

```
R(e_i, e_j) = (trust_reliability, trust_competence, trust_benevolence, affection, debt)
```

All components are in `[0.0, 1.0]` except `debt` which can be negative (the target owes the source) or positive (the source owes the target). For consistency with the ℝ⁵ representation, debt is normalized to `[-1.0, 1.0]`.

```rust
struct RelationalSubstrate {
    trust_reliability: f32,  // does target follow through?
    trust_competence: f32,   // can target do what's needed?
    trust_benevolence: f32,  // does target have good intentions toward source?
    affection: f32,          // warmth, attachment, love
    debt: f32,               // obligation (positive = source owes target)
}
```

### Vector Operations

Because the substrate is a vector in ℝ⁵, standard vector operations apply:

**Distance** between two substrate vectors (Euclidean):

```
‖R₁ - R₂‖ = √(Σᵢ (R₁ᵢ - R₂ᵢ)²)
```

This measures how different two relationships are. Sarah→Tom and Sarah→Kate have similar substrates (high affection, high trust); Sarah→Adam has a very different substrate (low trust_benevolence, acknowledged debt).

**Magnitude** of a substrate vector:

```
‖R‖ = √(Σᵢ Rᵢ²)
```

High magnitude means an intense relationship (whether positive or negative). Low magnitude means indifference or absence.

**Asymmetry score** for a dyad:

```
asymmetry(e_i, e_j) = ‖R(e_i, e_j) - R(e_j, e_i)‖
```

High asymmetry indicates power imbalance, information gaps, or mismatched feelings. Adam→Sarah vs Sarah→Adam has high asymmetry — Adam's instrumental condescension vs Sarah's wary dependence.

### TFATD Worked Example: Substrate Vectors

Selected edges from `relational-web-tfatd.md`, normalized to ℝ⁵:

| Edge | trust_rel | trust_comp | trust_ben | affection | debt |
|------|-----------|------------|-----------|-----------|------|
| Sarah → Tom | 0.6 | 0.8 | 0.9 | 0.95 | 0.3 |
| Tom → Sarah | 0.9 | 0.5 | 0.9 | 0.85 | 0.7 |
| Sarah → Adam | 0.4 | 0.7 | 0.4 | 0.2 | 0.5 |
| Adam → Sarah | 0.7 | 0.3 | N/A→0.0 | 0.3 | -0.3 |
| Tom → Beth | 0.9 | 0.6 | 0.9 | 0.9 | 0.9 |
| Kate → Sarah | 0.9 | 0.8 | 0.9 | 0.95 | 0.0 |

**Asymmetry calculation** for the Adam-Sarah dyad:

```
R(Sarah → Adam) = (0.4, 0.7, 0.4, 0.2, 0.5)
R(Adam → Sarah) = (0.7, 0.3, 0.0, 0.3, -0.3)

asymmetry = ‖(0.4-0.7, 0.7-0.3, 0.4-0.0, 0.2-0.3, 0.5-(-0.3))‖
          = ‖(-0.3, 0.4, 0.4, -0.1, 0.8)‖
          = √(0.09 + 0.16 + 0.16 + 0.01 + 0.64)
          = √1.06
          ≈ 1.03
```

This is the highest asymmetry in the web — a clear signal that the Adam-Sarah relationship is the most structurally imbalanced. Compare with the Kate-Sarah dyad:

```
R(Sarah → Kate) = (0.7, 0.7, 0.9, 0.85, 0.6)
R(Kate → Sarah) = (0.9, 0.8, 0.9, 0.95, 0.0)

asymmetry = ‖(-0.2, -0.1, 0.0, -0.1, 0.6)‖
          = √(0.04 + 0.01 + 0.0 + 0.01 + 0.36)
          = √0.42
          ≈ 0.65
```

Lower asymmetry — a more balanced relationship, though the debt dimension still diverges (the existential debt a child carries toward a parent).

---

## Topological Role Classification

### The Four Roles

Every entity in the relational web occupies a **topological role** that describes its structural position in the network. These roles are computed, not authored — they emerge from the graph's shape.

| Role | Definition | Structural Signature |
|------|-----------|---------------------|
| **Gate** | Controls the only path between disconnected components | High betweenness centrality, removal disconnects the graph |
| **Bridge** | Connects components that have alternative paths | High betweenness but graph remains connected on removal |
| **Hub** | Densely connected within a cluster | High degree centrality, low betweenness relative to degree |
| **Periphery** | Minimally connected, at the network's edge | Low degree, low betweenness |

### Degree Centrality

The simplest structural measure — how many connections an entity has.

**Out-degree** (edges originating from the entity):

```
deg⁺(e) = |{(e, e_j) ∈ E}|
```

**In-degree** (edges pointing to the entity):

```
deg⁻(e) = |{(e_i, e) ∈ E}|
```

**Total degree** (treating the graph as undirected for structural analysis):

```
deg(e) = |{e_j : (e, e_j) ∈ E ∨ (e_j, e) ∈ E}|
```

For topological role computation, we use undirected degree — we care about the number of distinct entities an entity is connected to, regardless of edge direction.

### Betweenness Centrality

Betweenness measures how often an entity lies on shortest paths between other entities. This identifies structural bottlenecks — entities whose removal would disrupt information flow.

```
betweenness(e) = Σ_{s≠e≠t} (σ_st(e) / σ_st)
```

Where:
- `σ_st` = total number of shortest paths from `s` to `t`
- `σ_st(e)` = number of those shortest paths that pass through `e`

Normalized to `[0.0, 1.0]` by dividing by `(n-1)(n-2)/2` (max possible in an undirected graph of `n` vertices).

**Algorithm**: Brandes' algorithm computes betweenness for all vertices in `O(VE)` time — efficient for our scale (< 100 vertices, < 500 edges).

```rust
fn compute_betweenness(web: &RelationalWeb) -> HashMap<EntityId, f32> {
    let n = web.vertices.len();
    let mut centrality: HashMap<EntityId, f32> = HashMap::new();

    for s in web.vertices.keys() {
        // BFS from s
        let (predecessors, distances, num_paths) = bfs_from(web, s);

        // Back-propagate dependencies
        let dependencies = back_propagate(&predecessors, &distances, &num_paths, s);

        for (v, dep) in dependencies {
            *centrality.entry(v).or_default() += dep;
        }
    }

    // Normalize (undirected)
    let norm = ((n - 1) * (n - 2)) as f32 / 2.0;
    for val in centrality.values_mut() {
        *val /= norm;
    }

    centrality
}
```

### Role Classification Algorithm

Given degree and betweenness centrality, classify each entity:

```rust
fn classify_topological_role(
    entity: EntityId,
    degree: usize,
    betweenness: f32,
    web: &RelationalWeb,
) -> TopologicalRole {
    // Gate: removing this entity disconnects the graph
    if is_articulation_point(entity, web) {
        return TopologicalRole::Gate;
    }

    // Bridge: high betweenness relative to the network
    let avg_betweenness = average_betweenness(web);
    if betweenness > avg_betweenness * 2.0 {
        return TopologicalRole::Bridge;
    }

    // Hub: high degree, moderate-to-low betweenness
    let avg_degree = average_degree(web);
    if degree as f32 > avg_degree * 1.5 {
        return TopologicalRole::Hub;
    }

    // Periphery: everything else
    TopologicalRole::Periphery
}
```

**Articulation point detection** (for Gate identification) uses Tarjan's algorithm in `O(V + E)` time. An articulation point is a vertex whose removal increases the number of connected components.

### TFATD Worked Example: Topological Roles

Computing roles for the 6-character web:

| Entity | Degree | Betweenness | Articulation Point? | Role |
|--------|--------|-------------|---------------------|------|
| Sarah | 4 (Tom, Kate, John, Adam) | Moderate | No | Hub (would be, but Adam's gate is stronger) |
| Tom | 3 (Sarah, Beth, Adam) | Moderate | No | Displaced Hub |
| Kate | 3 (Sarah, John, +hidden links) | Low | No | Bridge (hidden) |
| John | 2 (Sarah, Kate) | Very low | No | Periphery |
| Adam | 3 (Sarah, Tom, off-graph) | **Very high** | **Yes** | **Gate** |
| Beth | 1 (Tom only) | Zero | No | Periphery |

Adam is the only articulation point — removing him disconnects the off-graph entities (Ghostlight Queen, Wolf) from the human characters. His Gate role is topological, not authored.

Kate's Bridge role is "hidden" because her connections to the otherworldly dimension are not visible in the explicit web. If those latent edges were materialized, her betweenness would spike. This is an example of latent structural power — topologically invisible until revealed.

---

## Cluster Detection

### Definition

A **cluster** is a densely connected subgraph — a group of entities with more edges within the group than between the group and the rest of the web. Clusters correspond to social groups (the household, the sacrifice pair, the gate).

### Algorithm: Label Propagation

For our scale (< 100 entities), label propagation is simple and effective:

1. Initialize each vertex with a unique label
2. In each iteration, each vertex adopts the most common label among its neighbors (ties broken randomly)
3. Repeat until labels stabilize (usually 3-5 iterations at our scale)

```rust
fn detect_clusters(web: &RelationalWeb) -> HashMap<EntityId, ClusterId> {
    let mut labels: HashMap<EntityId, ClusterId> = web.vertices.keys()
        .enumerate()
        .map(|(i, &id)| (id, ClusterId(i)))
        .collect();

    for _ in 0..MAX_ITERATIONS {
        let mut changed = false;
        for entity in web.vertices.keys() {
            let neighbor_labels = neighbors(web, entity)
                .map(|n| labels[&n])
                .collect::<Vec<_>>();

            let most_common = mode(&neighbor_labels);
            if labels[entity] != most_common {
                labels.insert(*entity, most_common);
                changed = true;
            }
        }
        if !changed { break; }
    }

    labels
}
```

### Weighted Variant

Substrate magnitude can weight the label propagation — stronger relationships (higher `‖R‖`) have more influence on cluster assignment:

```rust
let weighted_neighbor_labels = neighbors(web, entity)
    .map(|n| {
        let weight = substrate_magnitude(web.edge(entity, n));
        (labels[&n], weight)
    })
    .collect::<Vec<_>>();

let most_common = weighted_mode(&weighted_neighbor_labels);
```

### TFATD Worked Example: Clusters

Running label propagation on the 6-character web produces:

| Cluster | Members | Binding Force |
|---------|---------|---------------|
| **Household** | Sarah, Kate, John | High mutual affection, deep history, bedrock relationships |
| **Sacrifice** | Tom, Beth | Overwhelming affection + debt; intense but isolated |
| **Gate** | Adam | Connects clusters; no cluster allegiance |

Sarah straddles Household and the quest — her connections to Adam and (through Adam) to the Gate cluster make her the traveler who moves between social worlds. Tom straddles Sacrifice and Household — his displacement is both physical and structural.

---

## Triadic Patterns

### Trust Transitivity

In social networks, trust is often transitive: if A trusts B and B trusts C, A may extend some trust to C. But trust transitivity is **not automatic** — it depends on the trust dimension and the relationship context.

**Transitive trust estimate** (A→C via B):

```
trust_transitive(A, C, via B) = trust_benevolence(A, B) * trust_competence(B, C) * decay
```

Where `decay` reflects that indirect trust is weaker than direct. This is not the full friction model (that's in `traversal-friction.md`) — it's a simplified computation for estimating what trust A might extend to C before direct interaction.

The key insight: we multiply A's trust in B's *benevolence* (does B have good intentions — will B's recommendation be honest?) by B's trust in C's *competence* (can C actually do what B might vouch for?). Different trust dimensions compose differently across hops.

### Structural Balance

**Structural balance theory** (Heider, 1946): in a triad of three entities, certain configurations are stable (balanced) and others are unstable (unbalanced):

| A→B | B→C | A→C | Balance |
|-----|-----|-----|---------|
| + | + | + | Balanced (friends of friends are friends) |
| + | + | - | **Unbalanced** (tension — friend likes someone I dislike) |
| + | - | - | Balanced (enemy of friend is enemy) |
| - | - | + | **Unbalanced** (tension — I like someone my friend dislikes) |

Where `+` means high affection and `-` means low affection or conflict.

**Computing balance for a triad**:

```rust
fn triad_balance(
    a_to_b: &RelationalSubstrate,
    b_to_c: &RelationalSubstrate,
    a_to_c: &RelationalSubstrate,
) -> TriadBalance {
    let ab_sign = if a_to_b.affection > 0.5 { 1.0 } else { -1.0 };
    let bc_sign = if b_to_c.affection > 0.5 { 1.0 } else { -1.0 };
    let ac_sign = if a_to_c.affection > 0.5 { 1.0 } else { -1.0 };

    let product = ab_sign * bc_sign * ac_sign;
    if product > 0.0 {
        TriadBalance::Balanced
    } else {
        TriadBalance::Unbalanced
    }
}
```

Unbalanced triads are **narratively interesting** — they create tension, drive character choices, and are natural sites for dramatic events. The Storykeeper flags unbalanced triads for narrative priming.

### TFATD Worked Example: Triadic Analysis

**Triad: Sarah — Kate — John**

| Edge | Affection | Sign |
|------|-----------|------|
| Sarah → Kate | 0.85 | + |
| Kate → John | 0.8 | + |
| Sarah → John | 0.7 | + |

Product: (+)(+)(+) = **Balanced**. The household is a stable triad — all members have positive mutual affection. The tension lies not in structural balance but in information asymmetry (Kate knows things John doesn't).

**Triad: Sarah — Adam — Tom**

| Edge | Affection | Sign |
|------|-----------|------|
| Sarah → Adam | 0.2 | - |
| Adam → Tom | 0.1 | - |
| Sarah → Tom | 0.95 | + |

Product: (-)(-)( +) = **Balanced** (enemy's enemy is friend). But this superficial balance hides the real tension: Sarah doesn't know Adam's instrumental relationship with Tom. The structural balance masks the information catastrophe.

**Triad: Kate — Adam — Sarah (hidden)**

Kate has latent connections to Adam's domain that are not explicitly in the web. If materialized:

| Edge | Affection | Sign |
|------|-----------|------|
| Kate → Adam | unknown (low?) | - |
| Adam → Sarah | 0.3 | - |
| Kate → Sarah | 0.95 | + |

Product: (-)(-)( +) = **Balanced** (enemy of friend is enemy). Kate's hidden knowledge of Adam's domain aligns with structural balance — she knows he is not to be trusted, but she sent Sarah to him anyway. This is a balanced triad under enormous internal pressure.

---

## Cascade Propagation

### The Problem

When a relational event occurs (trust shift, information reveal, debt change), its effects propagate through the web — other entities may perceive or be affected by the change. The cascade model governs what propagates, how far, and with what fidelity.

The full friction model is formalized in [`traversal-friction.md`](traversal-friction.md). Here we cover the relational-web-specific mechanics.

### Signal Propagation on a Single Path

Given a relational signal originating at edge `(e_source, e_target)`, propagating along a path `π = (e_target, e₁, e₂, ..., e_dest)`:

```
signal_at(e_dest) = signal_at(e_source)
    × Π_{edge ∈ π} permeability(edge)
    × F^|π|
```

Where:
- `F` is the friction factor (default 0.5)
- `permeability(edge)` is computed per-edge (see below)
- `|π|` is the path length

### Per-Edge Permeability

Permeability determines how freely information flows across an edge:

```rust
fn permeability(edge: &RelationalEdge) -> f32 {
    let trust_factor = (edge.substrate.trust_competence
        + edge.substrate.trust_benevolence) / 2.0;
    let history_factor = edge.history.depth.min(1.0);
    let opacity = edge.opacity_modifier; // 0.0 = transparent, 1.0 = opaque

    trust_factor * history_factor * (1.0 - opacity)
}
```

- **High-trust edges** propagate more signal — information flows between entities that trust each other
- **Deep history** carries information better than new relationships
- **Opacity** explicitly blocks propagation — secret-keepers have low permeability on outgoing edges

### Multi-Path Resolution

When multiple paths exist between source and destination, each path delivers a signal. The **strongest signal wins** — we take the path with maximum delivered signal:

```rust
fn cascade_signal(
    web: &RelationalWeb,
    source: EntityId,
    signal: f32,
    friction_factor: f32,
    significance_threshold: f32,
) -> HashMap<EntityId, f32> {
    // Modified Dijkstra: maximize signal instead of minimizing cost
    let mut best_signal: HashMap<EntityId, f32> = HashMap::new();
    let mut queue = BinaryHeap::new();

    best_signal.insert(source, signal);
    queue.push(SignalEntry { entity: source, signal });

    while let Some(current) = queue.pop() {
        if current.signal < significance_threshold {
            continue; // Below threshold — stop propagating
        }

        for neighbor in neighbors(web, current.entity) {
            let edge = web.edge(current.entity, neighbor);
            let propagated = current.signal
                * permeability(edge)
                * friction_factor;

            if propagated > *best_signal.get(&neighbor).unwrap_or(&0.0) {
                best_signal.insert(neighbor, propagated);
                queue.push(SignalEntry { entity: neighbor, signal: propagated });
            }
        }
    }

    best_signal
}
```

This is a modified Dijkstra's algorithm where we maximize signal strength rather than minimize distance. The priority queue ensures we always process the strongest remaining signal first.

### TFATD Worked Example: Cascade from Trust Shift

**Scenario**: Sarah's trust in Adam shifts from 0.4 to 0.2 during scene S2 (Speaking with the Gate). What propagates?

Signal source: `signal = |0.4 - 0.2| = 0.2` (magnitude of the shift)

Friction factor: `F = 0.5`

**Path 1: Sarah → Kate** (distance 1)
```
permeability(Sarah → Kate) = ((0.7 + 0.9)/2) * 0.9 * 1.0 = 0.72
signal_at_Kate = 0.2 × 0.72 × 0.5¹ = 0.072
```

Kate perceives something — Sarah's wariness registers faintly. Above significance threshold (0.1)? No, 0.072 < 0.1. Dropped.

**Path 2: Sarah → Tom** (distance 1)
```
permeability(Sarah → Tom) = ((0.6 + 0.9)/2) * 0.9 * 1.0 = 0.675
signal_at_Tom = 0.2 × 0.675 × 0.5¹ = 0.0675
```

Below threshold. Tom (who is unconscious/absent) perceives nothing.

**Path 3: Sarah → Kate → John** (distance 2)
```
signal_at_John = 0.072 × permeability(Kate → John) × 0.5
               = 0.072 × ((0.7 + 0.9)/2 × 0.8 × (1.0 - high_opacity))
```

Kate keeps secrets (high opacity on outgoing edges to John). Even if this path were above threshold at Kate, John receives almost nothing.

**Result**: At default friction (0.5), Sarah's trust shift toward Adam is a purely local event — it doesn't propagate beyond the direct participants. In a tightly-knit community (friction 0.7), the signal would reach Kate above threshold. This matches narrative intuition: in the Shadowed Wood, far from home, Sarah's wariness is hers alone.

---

## Tension Detection

### Definition

A **tension edge** is one where substrate dimensions are in conflict — the relationship carries contradictory forces. Tension is the source of narrative energy in relational dynamics.

### Tension Score

```rust
fn tension_score(edge: &RelationalEdge) -> f32 {
    let s = &edge.substrate;

    // Trust vs. debt: trusting someone you owe creates obligation tension
    let trust_debt = ((s.trust_benevolence + s.trust_reliability) / 2.0 - s.debt.abs()).abs();

    // Affection vs. trust: loving someone you don't trust creates vulnerability
    let affection_trust = (s.affection - s.trust_benevolence).max(0.0);

    // High asymmetry in the dyad is itself a tension source
    let asymmetry = if let Some(reverse) = edge.reverse_edge() {
        substrate_distance(s, &reverse.substrate)
    } else {
        0.0
    };

    (trust_debt + affection_trust + asymmetry) / 3.0
}
```

### TFATD Worked Example: Highest Tension Edges

| Edge | Trust-Debt Tension | Affection-Trust Tension | Asymmetry | Total |
|------|-------------------|------------------------|-----------|-------|
| Tom → Beth | |(0.9+0.9)/2 - 0.9| = 0.0 | (0.9 - 0.9) = 0.0 | ~0.3 | **0.10** |
| Sarah → Adam | |(0.4+0.4)/2 - 0.5| = 0.1 | (0.2 - 0.4) = 0.0 | **1.03** | **0.38** |
| Adam → Sarah | |(0.7+0.0)/2 - 0.3| = 0.05 | (0.3 - 0.0) = 0.3 | **1.03** | **0.46** |
| Kate → John | |(0.7+0.9)/2 - 0.2| = 0.6 | (0.8 - 0.9) = 0.0 | ~0.65 | **0.42** |

The Adam-Sarah dyad and Kate-John edge produce the highest tension — exactly where the narrative's dramatic energy lives. Adam→Sarah is the most dangerous configuration in the web; Kate→John is the loving relationship built on hidden knowledge.

---

## Computational Summary

| Operation | Algorithm | Complexity | Expected Latency (100 entities) |
|-----------|-----------|------------|-------------------------------|
| Substrate distance | Vector arithmetic | O(1) per pair | < 0.01ms |
| Degree centrality | Edge counting | O(V + E) | < 1ms |
| Betweenness centrality | Brandes | O(VE) | < 5ms |
| Articulation points | Tarjan's DFS | O(V + E) | < 1ms |
| Role classification | Degree + betweenness + articulation | O(VE) | < 5ms |
| Cluster detection | Label propagation (5 iterations) | O(k × E) | < 2ms |
| Cascade propagation | Modified Dijkstra | O((V + E) log V) | < 2ms |
| Tension detection | Per-edge computation | O(E) | < 1ms |
| Triad enumeration | Triangle listing | O(E^{3/2}) | < 3ms |

All operations are well within the scene-entry budget (~50ms for full graph loading).

---

## Theoretical Foundations

### What We Borrow

| Concept | Source | Our Adaptation |
|---------|--------|---------------|
| **Betweenness centrality** | Freeman (1977), "A set of measures of centrality based on betweenness" | Standard definition, applied to relational substrate graph |
| **Articulation points** | Tarjan (1972), "Depth-first search and linear graph algorithms" | Standard algorithm for Gate detection |
| **Label propagation clustering** | Raghavan et al. (2007), "Near linear time algorithm to detect community structures" | Weighted by substrate magnitude |
| **Structural balance** | Heider (1946), Cartwright & Harary (1956) | Applied to affection dimension; extended with information-state analysis |
| **Trust transitivity** | Social network trust models (Ziegler & Lausen, 2005) | Decomposed into dimension-specific transitivity |

### What We Invent

| Concept | Novelty |
|---------|---------|
| **Substrate as ℝ⁵ vector** | Specific to our domain — five dimensions chosen for narrative relationship modeling, not standard social network dimensions |
| **Asymmetry as structural signal** | Using vector distance between edge pairs as a narrative tension indicator |
| **Opacity-modulated permeability** | The `opacity_modifier` on edges for secret-keeping is domain-specific |
| **Tension score** | The specific formula combining trust-debt, affection-trust, and asymmetry is ours |
| **Topological role taxonomy** | Gate/Bridge/Hub/Periphery mapped to narrative archetypes (structural power analysis) |

### What We Defer

- **Spectral clustering** — More sophisticated than label propagation but unnecessary at our scale. If stories exceed ~500 entities, revisit.
- **Temporal centrality** — How structural position changes over time. Important for the temporal slider but not needed for per-scene computation.
- **Weighted betweenness** — Using substrate magnitude as edge weight in betweenness computation. A natural extension but adds complexity without clear benefit at our scale.
