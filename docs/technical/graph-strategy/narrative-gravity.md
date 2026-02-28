# Narrative Gravity Mathematics

## Purpose

This document formalizes the **gravitational model** of the narrative graph — the math that computes how strongly scenes attract the story, how narrative distance is measured, and how attractor basins partition the story landscape. Where the domain model says "scenes have mass and stories bend toward pivotal moments," this document specifies exactly how to compute that bending.

**Prerequisites**: None. This document is self-contained.

**Source material**: [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) §Graph 2, [`narrative-graph-case-study-tfatd.md`](../narrative-graph-case-study-tfatd.md), [`gravitational-context-assembly.md`](../gravitational-context-assembly.md).

---

## The Narrative Graph as a Directed Weighted Graph

### Definition

The narrative graph is a directed graph `N = (S, T)` where:

- `S` is the set of scenes
- `T ⊆ S × S` is the set of scene transitions

Each scene vertex carries a **narrative mass** that determines its gravitational pull. Each transition edge carries approach vectors (state predicates for entry) and departure conditions.

```rust
struct NarrativeGraph {
    scenes: HashMap<SceneId, SceneVertex>,
    transitions: HashMap<(SceneId, SceneId), SceneTransition>,
}
```

Unlike the relational web (which is densely connected), the narrative graph is typically sparse — each scene connects to 1-4 successor scenes.

---

## Effective Mass

### Three Components

A scene's effective mass — how strongly it attracts the narrative — is computed from three components:

```
M(s, player) = M_base(s) + M_struct(s) + M_dynamic(s, player)
```

### Component 1: Authored Base Mass

Set by the story designer. Represents the scene's inherent narrative importance independent of player state.

```
M_base(s) ∈ [0.0, 1.0]
```

Scene types have typical ranges:

| Scene Type | Typical M_base | Examples (TFATD) |
|-----------|---------------|-----------------|
| Gravitational | 0.7 — 1.0 | S1 (0.8), S6 (0.9) |
| Gate/Threshold | 0.5 — 0.8 | S2 (0.7) |
| Connective | 0.2 — 0.5 | S4 (0.5), S5 (0.3) |

### Component 2: Structural Modifier

Computed from the scene's position and properties in the narrative graph:

```rust
fn structural_modifier(scene: &SceneVertex) -> f32 {
    let mut modifier = 0.0;

    // Information gates increase mass (+0.05 per gate)
    modifier += scene.information_gates.len() as f32 * 0.05;

    // Convergence points (multiple paths lead here)
    if is_convergence_point(scene) {
        modifier += 0.1;
    }

    // Threshold scenes (boundary between narrative zones)
    if scene.scene_type == SceneType::Threshold {
        modifier += 0.1;
    }

    // Required-for-story (removing this breaks the narrative)
    if scene.required_for_story {
        modifier += 0.1;
    }

    // Emotional hinge (scene where relationships transform)
    if scene.emotional_hinge {
        modifier += 0.1;
    }

    // Branch point (genuine player choice)
    if scene.contains_branch_point {
        modifier += 0.05;
    }

    modifier
}
```

### Component 3: Dynamic Adjustment

Computed from the player's current state — how well-prepared the player is for this scene:

```
M_dynamic(s, player) = approach_satisfaction(s, player) × 0.2
```

The dynamic adjustment ranges `[0.0, 0.2]`. A scene the player is perfectly prepared for has higher effective mass — it pulls harder because it would land better. A scene whose approach vectors are unsatisfied has zero dynamic adjustment.

### TFATD Worked Example: Effective Mass

For S3 (A Mother's Prayer), with a player who has completed S2 (determined, knows about the Wood):

```
M_base(S3) = 0.85

M_struct(S3) = 0.05 × 3 (three gates: kate_has_power, water_as_borderland, john_cannot_enter)
             + 0.1 (emotional hinge)
             = 0.25

M_dynamic(S3, player) = approach_satisfaction(S3, player) × 0.2
```

Approach vector: `determined_and_ready` requires `{emotional: resolved, information: wood+wolf known}`. Player has completed S2, knows about the Wolf and the Wood, has resolved to enter → high satisfaction.

```
approach_satisfaction = 0.9 (emotional resolved ✓, info satisfied ✓, minor gaps)
M_dynamic = 0.9 × 0.2 = 0.18

M(S3) = 0.85 + 0.25 + 0.18 = 1.28
```

This exceeds the authored base by nearly 50% — the structural importance and player readiness amplify the scene's pull significantly.

---

## Multi-Dimensional Narrative Distance

### Definition

Narrative distance between two scenes is not spatial distance — it is a weighted combination of four dimensions that measure how "far" the player is from a scene in narrative terms:

```
D(current, target) = w_info × D_info + w_emo × D_emo + w_rel × D_rel + w_phys × D_phys
```

Where each component distance is in `[0.0, 1.0]` and weights are scene-specific.

### Component 1: Information Distance

How much information the player still needs to unlock this scene:

```rust
fn information_distance(player_info: &InfoState, scene_required: &InfoRequirements) -> f32 {
    let total_gates = scene_required.gates.len();
    if total_gates == 0 {
        return 0.0; // No information requirements
    }

    let satisfied = scene_required.gates.iter()
        .filter(|gate| player_info.satisfies(gate))
        .count();

    1.0 - (satisfied as f32 / total_gates as f32)
}
```

A scene whose gates are all satisfied has zero information distance. A scene with 4 gates of which 1 is satisfied has distance 0.75.

### Component 2: Emotional Distance

How far the player's emotional state is from the scene's optimal emotional approach:

```rust
fn emotional_distance(player_emotion: &EmotionalState, scene_optimal: &EmotionalProfile) -> f32 {
    // Each approach vector defines an optimal emotional state
    // Take the minimum distance to any approach vector
    scene_optimal.approach_vectors.iter()
        .map(|av| emotional_vector_distance(player_emotion, &av.emotional_state))
        .min_by(f32::total_cmp)
        .unwrap_or(1.0)
}

fn emotional_vector_distance(current: &EmotionalState, target: &EmotionalState) -> f32 {
    // Simplified: compare dominant emotional dimensions
    let valence_diff = (current.valence - target.valence).abs();
    let arousal_diff = (current.arousal - target.arousal).abs();
    let determination_diff = (current.determination - target.determination).abs();

    (valence_diff + arousal_diff + determination_diff) / 3.0
}
```

### Component 3: Relational Distance

How well the player's current relationships match the scene's cast requirements:

```rust
fn relational_distance(
    player_rels: &RelationalContext,
    scene_cast: &CastRequirements,
) -> f32 {
    if scene_cast.required_presences.is_empty() {
        return 0.0;
    }

    let total_requirements = scene_cast.relational_requirements.len();
    if total_requirements == 0 {
        return 0.0;
    }

    let satisfied = scene_cast.relational_requirements.iter()
        .filter(|req| player_rels.satisfies(req))
        .count();

    1.0 - (satisfied as f32 / total_requirements as f32)
}
```

### Component 4: Physical Distance

Spatial distance through the setting topology (cross-graph query):

```rust
fn physical_distance(player_setting: SettingId, scene_setting: SettingId) -> f32 {
    // Normalized traversal cost from setting-topology
    let cost = setting_topology.shortest_path_cost(player_setting, scene_setting);
    (cost / MAX_TRAVERSAL_COST).min(1.0)
}
```

### Scene-Specific Weights

Different scene types weight distance dimensions differently:

| Scene Type | w_info | w_emo | w_rel | w_phys |
|-----------|--------|-------|-------|--------|
| Gravitational | 0.2 | 0.4 | 0.3 | 0.1 |
| Gate | 0.5 | 0.1 | 0.2 | 0.2 |
| Threshold | 0.3 | 0.3 | 0.1 | 0.3 |
| Connective | 0.1 | 0.2 | 0.1 | 0.6 |

Gate scenes weight information heavily (you need to know things to reach them). Gravitational scenes weight emotion and relationships (you need to feel things for them to land). Connective scenes weight physical distance (you need to be nearby).

### TFATD Worked Example: Narrative Distance

Player is at S4 (A Haunt on the Rise), heading toward S6 (The Other Bank).

```
D_info(S4 → S6):
  S6 requires: water_as_borderland gate from S3
  Player has completed S3: gate satisfied
  D_info = 0.0

D_emo(S4 → S6):
  S6 optimal: exhausted_but_persisting {tired + determined}
  Player at S4: tense + watchful (close but not yet exhausted)
  D_emo ≈ 0.3

D_rel(S4 → S6):
  S6 requires: Wolf trust at moderate level
  Player has walked with Wolf through S4: trust building
  D_rel ≈ 0.2

D_phys(S4 → S6):
  S4 and S6 are separated by S5 (the stream) or direct connective space
  D_phys ≈ 0.3

Weights for Gravitational scene:
  w_info=0.2, w_emo=0.4, w_rel=0.3, w_phys=0.1

D(S4, S6) = 0.2×0.0 + 0.4×0.3 + 0.3×0.2 + 0.1×0.3
           = 0.0 + 0.12 + 0.06 + 0.03
           = 0.21
```

This is a short narrative distance — S6 is pulling strongly from S4's position.

---

## Gravitational Pull

### The Inverse-Square Law

Gravitational pull follows an inverse-square law adapted from physics:

```
G(current, target) = M(target) / D(current, target)²
```

This means pull increases rapidly as distance decreases — a scene that is "close" in narrative terms pulls with quadratic strength. The inverse-square law naturally creates the experience of gravity: weak pull at a distance, strong pull nearby, overwhelming pull at arrival.

### Singularity Clamping

When narrative distance approaches zero, the gravitational formula would produce infinite pull. We clamp with a minimum distance:

```rust
fn gravitational_pull(
    current: SceneId,
    target: SceneId,
    player: &PlayerState,
    graph: &NarrativeGraph,
) -> f32 {
    let mass = effective_mass(&graph.scenes[&target], player);
    let distance = narrative_distance(current, target, player, graph);

    let clamped_distance = distance.max(MIN_DISTANCE); // MIN_DISTANCE = 0.05
    mass / (clamped_distance * clamped_distance)
}
```

At minimum distance (0.05), a scene with mass 1.0 produces pull of `1.0 / 0.0025 = 400.0`. This is the maximum gravitational influence — the scene is about to activate.

### TFATD Worked Example: Gravitational Pull from S4

Player is at S4, computing pull from all reachable scenes:

| Target | M(target) | D(S4, target) | G(S4, target) |
|--------|-----------|---------------|---------------|
| S5 | 0.3 | 0.15 | 0.3 / 0.0225 = **13.3** |
| S6 | ~1.2 | 0.21 | 1.2 / 0.0441 = **27.2** |
| S7 | ~0.5 | 0.25 | 0.5 / 0.0625 = **8.0** |
| S8 | ~1.1 | 0.55 | 1.1 / 0.3025 = **3.6** |

**S6 has the strongest pull** — more than double any other scene. This matches the narrative: The Other Bank is Part I's climactic scene, and the player is approaching it. S8 (Meeting the Witch) has high mass but is still narratively distant.

**Strongest attractor**: S6 (The Other Bank) with pull 27.2.

---

## Approach Vector Satisfaction

### Definition

Each scene defines one or more **approach vectors** — state predicates that describe different ways to enter the scene. The satisfaction score measures how well the player's current state matches:

```rust
fn approach_satisfaction(scene: &SceneVertex, player: &PlayerState) -> f32 {
    scene.approach_vectors.iter()
        .map(|av| match_score(av, player))
        .max_by(f32::total_cmp)
        .unwrap_or(0.0)
}

fn match_score(vector: &ApproachVector, player: &PlayerState) -> f32 {
    let dimensions = &vector.state_predicate;
    let mut total_weight = 0.0;
    let mut total_match = 0.0;

    for (dimension, weight) in dimensions {
        total_weight += weight;
        total_match += weight * dimension_match(dimension, player);
    }

    if total_weight == 0.0 { 0.0 } else { total_match / total_weight }
}

fn dimension_match(dimension: &StatePredicate, player: &PlayerState) -> f32 {
    match dimension.evaluate(player) {
        PredicateResult::FullySatisfied => 1.0,
        PredicateResult::PartiallySatisfied(degree) => degree,
        PredicateResult::Unsatisfied => 0.0,
    }
}
```

### TFATD Worked Example: S6 Approach Vectors

S6 (The Other Bank) has three approach vectors:

**Vector A: `exhausted_but_persisting`**
```
Predicates:
  emotional: tired + determined (weight 0.4)
  relational: wolf_trust moderate (weight 0.3)
  information: water_as_borderland known (weight 0.3)
```

**Vector B: `growing_trust_with_wolf`**
```
Predicates:
  relational: wolf_trust high (weight 0.5)
  emotional: companionable (weight 0.3)
  information: water_as_borderland known (weight 0.2)
```

**Vector C: `frightened_and_alone`**
```
Predicates:
  emotional: fear dominant (weight 0.6)
  information: minimal (weight 0.2)
  relational: wolf_trust low (weight 0.2)
```

A player who has walked with the Wolf through S4 and S5, building trust and growing exhausted:

```
Vector A: 0.4×0.8 + 0.3×0.6 + 0.3×1.0 = 0.32 + 0.18 + 0.30 = 0.80
Vector B: 0.5×0.6 + 0.3×0.4 + 0.2×1.0 = 0.30 + 0.12 + 0.20 = 0.62
Vector C: 0.6×0.1 + 0.2×0.0 + 0.2×0.8 = 0.06 + 0.00 + 0.16 = 0.22

approach_satisfaction = max(0.80, 0.62, 0.22) = 0.80
```

The player best matches "exhausted but persisting" — the default emotional trajectory for a player who engaged with the connective space.

---

## Attractor Basins

### Definition

An **attractor basin** is the region of narrative space where a particular scene's gravitational pull dominates. Every position in the narrative landscape belongs to the basin of whichever scene pulls strongest from that position.

This is analogous to a **Voronoi partition** in gravitational space — but using gravitational pull rather than Euclidean distance as the metric.

### Computing Basin Membership

```rust
fn attractor_basin(
    current: SceneId,
    player: &PlayerState,
    graph: &NarrativeGraph,
) -> SceneId {
    let reachable = graph.reachable_scenes(current);

    reachable.iter()
        .filter(|&&s| s != current) // Don't include the scene we're in
        .max_by(|&&a, &&b| {
            let pull_a = gravitational_pull(current, a, player, graph);
            let pull_b = gravitational_pull(current, b, player, graph);
            pull_a.total_cmp(&pull_b)
        })
        .copied()
        .unwrap_or(current)
}
```

### Basin Boundary

The boundary between two basins is the set of positions where their gravitational pulls are equal:

```
G(position, scene_A) = G(position, scene_B)
M(A) / D(position, A)² = M(B) / D(position, B)²
D(position, A)² / D(position, B)² = M(A) / M(B)
D(position, A) / D(position, B) = √(M(A) / M(B))
```

For scenes of equal mass, the basin boundary is equidistant. For scenes of unequal mass, the boundary shifts toward the lighter scene — the heavier scene's basin extends further.

### TFATD Worked Example: Basin Map at S4

From S4 (A Haunt on the Rise), the gravitational landscape:

```
Basin of S6 (pull 27.2): dominates from S4 through S5
Basin of S7 (pull 8.0):  accessible via the village door
Basin of S8 (pull 3.6):  distant but growing

S6's basin is ~3.4× stronger than S7's pull at S4.
The player is firmly in S6's basin — this is the natural trajectory.
Choosing S7 (the village) requires actively resisting S6's pull.
```

---

## Connective Space: Distributed Mass

### The Problem

Between major scenes lies **connective space** — travel, atmosphere, small interactions. This space has narrative mass, but it is distributed rather than concentrated.

### Distributed Mass Model

```
M_connective(time_spent) = M_base_per_unit × time_spent × diminishing(time_spent)

diminishing(t) = 1.0 / (1.0 + α × t)
```

Where `α` controls how quickly returns diminish (default `α = 0.5`).

```rust
fn connective_mass(base_rate: f32, time_spent: f32, alpha: f32) -> f32 {
    base_rate * time_spent / (1.0 + alpha * time_spent)
}
```

**Properties**:
- At `t = 0`: mass = 0 (no time spent, no mass accumulated)
- At `t = 1`: mass ≈ `base_rate × 0.67` (first unit gives the most)
- At `t = 10`: mass ≈ `base_rate × 1.67` (diminishing returns)
- As `t → ∞`: mass → `base_rate / α` (hard ceiling)

This rewards engagement without punishing speed or rewarding loitering.

### TFATD Worked Example

Connective space between S3 (Mother's Prayer) and S6 (The Other Bank), with `base_rate = 0.1`:

| Time Spent | Accumulated Mass | What's Happening |
|-----------|-----------------|------------------|
| 0 | 0.0 | Just entered the Wood |
| 1 | 0.067 | Walking with the Wolf |
| 3 | 0.12 | Building texture: mist, cold, doubt |
| 5 | 0.143 | Rich engagement: Wolf conversations, observations |
| 10 | 0.167 | Extended exploration (diminishing returns) |

A player who rushes through accumulates mass 0.067. A player who engages deeply accumulates 0.143 — double the atmospheric weight, producing richer material for the Narrator's context.

---

## Computational Summary

| Operation | Algorithm | Complexity | Expected Latency |
|-----------|-----------|------------|-----------------|
| Effective mass | Component sum | O(1) per scene | < 0.01ms |
| Narrative distance | 4-dimension weighted sum | O(1) per pair | < 0.1ms (excluding cross-graph queries) |
| Gravitational pull | Inverse-square | O(1) per pair | < 0.01ms |
| Approach satisfaction | Max over vectors, weighted predicate matching | O(V × P) where V=vectors, P=predicates | < 0.5ms |
| Strongest attractor | Pull computation for all reachable scenes | O(R) where R=reachable scenes | < 5ms |
| Basin membership | Max pull among reachable scenes | O(R) | < 5ms |
| Mass recalculation | Recompute structural + dynamic for affected scenes | O(affected scenes) | < 2ms |

Gravitational queries run at **scene boundaries** (not per-turn), except for `strongest_attractor` which may be consulted during context assembly. All operations are well within the scene-entry budget.

---

## Theoretical Foundations

### What We Borrow

| Concept | Source | Our Adaptation |
|---------|--------|---------------|
| **Inverse-square law** | Newtonian gravity | Mass is narrative importance, distance is multi-dimensional narrative distance |
| **Voronoi partition** | Computational geometry | Basin boundaries computed by gravitational dominance rather than Euclidean distance |
| **Weighted distance** | Multi-criteria decision analysis | Four distance dimensions with scene-specific weights |
| **Diminishing returns** | Diminishing marginal utility (economics) | Applied to connective space mass accumulation |

### What We Invent

| Concept | Novelty |
|---------|---------|
| **Narrative distance** | The four-dimensional distance metric (information, emotional, relational, physical) is domain-specific |
| **Approach vector satisfaction** | Measuring player-state readiness as a contribution to effective mass |
| **Scene-type weight profiles** | Different scene types weighting distance dimensions differently |
| **Dynamic mass adjustment** | Mass changing based on player state — scenes pull harder when the player is ready for them |
| **Connective space distributed mass** | Treating travel/texture as mass accumulated over engagement time |
| **Singularity clamping** | MIN_DISTANCE to prevent infinite pull as narrative distance approaches zero |

### What We Defer

- **N-body interaction** — Scenes attracting each other (not just attracting the player). Would enable modeling how scenes cluster gravitationally. Complex and speculative; deferred to future work.
- **Relativistic corrections** — Adjusting pull based on the player's "velocity" through the narrative (how quickly they are progressing). Interesting but adds complexity without clear benefit.
- **Mass radiation** — Completed scenes losing mass over time (attention decay). Currently, completed scenes' mass is handled by activation state changes rather than mass decay.
