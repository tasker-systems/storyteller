# Cross-Graph Composition Mathematics

## Purpose

This document formalizes the mathematics of **cross-graph composition** — the computations that span multiple graphs to produce the Storykeeper's most important outputs: composite retrieval scores, gravitational modifiers for context assembly, sub-graph collective mass, boundary permeability dynamics, prophetic cascade lifecycle, and token budget allocation.

This is the capstone document: it composes the math from all four individual graph documents and the traversal friction model into the integrated computations that drive the storyteller system's narrative intelligence.

**Prerequisites**: All prior documents in this directory — [relational-web-math.md](relational-web-math.md), [narrative-gravity.md](narrative-gravity.md), [setting-topology.md](setting-topology.md), [event-dag.md](event-dag.md), [traversal-friction.md](traversal-friction.md).

**Source material**: [`gravitational-context-assembly.md`](../gravitational-context-assembly.md), [`tales-within-tales.md`](../tales-within-tales.md), [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) §Cross-Graph Query Patterns.

---

## The Composite Score

### Definition

When the Storykeeper assembles the Narrator's retrieved context (Tier 3), each candidate context item receives a **composite score** that determines whether it makes the cut:

```
composite_score(candidate) = initial_relevance(candidate)
    × gravitational_modifier(candidate)
    × information_boundary_factor(candidate)
    × recency_decay(candidate)
    × narrative_temperature_modifier(candidate)
```

All five factors are multiplicative. A zero in any factor zeros the entire score (information boundaries are absolute). A high factor amplifies but cannot create relevance from nothing.

### Factor 1: Initial Relevance

Set by the candidate's generating source. Higher values mean the candidate is more directly responsive to the current turn:

| Source | Initial Relevance | Rationale |
|--------|------------------|-----------|
| Entity-triggered (player referenced this entity) | 1.0 | Direct response to player input |
| Information reveal (gate opened) | 0.9 | High-priority narrative event |
| Emotional state (character prediction shift) | 0.8 | Active emotional thread |
| Gravitational landscape (nearby attractor) | f(pull) | Proportional to gravitational pull |
| Sub-graph resonance (bridge objects, echoes) | f(mass × Ψ) | Mass modulated by boundary permeability |
| Thematic echo (rhyming moment detected) | 0.6 | Thematic enrichment |

### Factor 2: Gravitational Modifier

The gravitational modifier amplifies candidates that align with the narrative's current gravitational pull. This is the primary mechanism through which gravity shapes context.

```rust
fn gravitational_modifier(
    candidate: &ContextCandidate,
    gravitational_state: &GravitationalState,
) -> f32 {
    let scene_component = scene_attractor_component(candidate, gravitational_state);
    let sub_graph_component = sub_graph_component(candidate, gravitational_state);
    let prophetic_component = prophetic_component(candidate, gravitational_state);

    // Take the strongest component, not the sum
    let max_component = scene_component
        .max(sub_graph_component)
        .max(prophetic_component);

    1.0 + max_component
}
```

The modifier is `≥ 1.0` — it amplifies but never diminishes. A candidate with no gravitational alignment receives modifier 1.0 (no change).

#### Scene Attractor Component

```rust
fn scene_attractor_component(
    candidate: &ContextCandidate,
    state: &GravitationalState,
) -> f32 {
    state.nearby_attractors.iter()
        .filter(|attractor| candidate.relates_to(attractor.scene_id))
        .map(|attractor| {
            gravitational_pull(state.current_scene, attractor.scene_id, &state.player)
                * ATTRACTOR_WEIGHT // default: 0.3
        })
        .max_by(f32::total_cmp)
        .unwrap_or(0.0)
}
```

A candidate that "relates to" an attractor (involves its entities, settings, or themes) gets amplified in proportion to that attractor's gravitational pull.

#### Sub-Graph Component

```rust
fn sub_graph_component(
    candidate: &ContextCandidate,
    state: &GravitationalState,
) -> f32 {
    state.active_sub_graphs.iter()
        .filter(|sg| candidate.resonates_with(sg))
        .map(|sg| {
            sub_graph_collective_mass(sg)
                * boundary_permeability(sg)
                * SUB_GRAPH_WEIGHT // default: 0.25
        })
        .max_by(f32::total_cmp)
        .unwrap_or(0.0)
}
```

Sub-graph influence is modulated by boundary permeability — the fairy tale's collective mass affects retrieval only to the degree the boundary allows.

#### Prophetic Component

```rust
fn prophetic_component(
    candidate: &ContextCandidate,
    state: &GravitationalState,
) -> f32 {
    state.active_prophecies.iter()
        .filter(|prophecy| candidate.relates_to(prophecy.target_scene))
        .map(|prophecy| {
            prophecy.approach_vector_modifier.magnitude
                * PROPHETIC_WEIGHT // default: 0.2
        })
        .sum::<f32>() // Prophetic cascades compound (sum, not max)
}
```

Prophetic cascades are the only component that **sums** rather than taking the maximum. Multiple prophecies targeting the same scene compound — a scene targeted by three prophetic cascades from three sub-graph events is under enormous retrieval pressure.

### Factor 3: Information Boundary Factor

Binary for hard boundaries, graduated for soft ones:

```rust
fn information_boundary_factor(candidate: &ContextCandidate) -> f32 {
    if candidate.violates_hard_boundary() {
        0.0 // Absolutely excluded
    } else if candidate.contains_withheld_information() {
        0.0 // Storykeeper guards the mystery
    } else if candidate.contains_suspected_information() {
        0.5 // Can hint at but not state
    } else {
        1.0 // Fully available
    }
}
```

This is an absolute gate: a high-gravity, high-relevance candidate that violates an information boundary scores **zero**. The Storykeeper will not reveal information before its time.

### Factor 4: Recency Decay

```rust
fn recency_decay(candidate: &ContextCandidate, current_turn: TurnId) -> f32 {
    let age = current_turn - candidate.source_turn;
    let half_life = RECENCY_HALF_LIFE; // default: 10 turns

    0.5_f32.powf(age as f32 / half_life as f32)
}
```

Recent events get priority. Events from 10 turns ago have half their original recency weight. Events from 20 turns ago have one-quarter. This prevents stale context from crowding out fresh material.

### Factor 5: Narrative Temperature Modifier

```rust
fn narrative_temperature_modifier(
    candidate: &ContextCandidate,
    scene_temperature: SceneTemperature,
) -> f32 {
    match scene_temperature {
        SceneTemperature::High => {
            // Approaching climax — favor tension
            match candidate.emotional_valence {
                Valence::Tension | Valence::Stakes | Valence::Conflict => 1.2,
                Valence::Calm | Valence::Routine => 0.7,
                _ => 1.0,
            }
        }
        SceneTemperature::Low => {
            // Aftermath — favor reflection
            match candidate.emotional_valence {
                Valence::Reflection | Valence::Connection | Valence::Tenderness => 1.2,
                Valence::Tension | Valence::Urgency => 0.7,
                _ => 1.0,
            }
        }
        SceneTemperature::Neutral => 1.0,
    }
}
```

Temperature prevents tonal whiplash — gravity doesn't surface cave-sacrifice imagery during a quiet moment of rebuilding.

---

## Sub-Graph Collective Mass

### Definition

A sub-graph's gravitational influence on the parent narrative is measured by its **collective mass** — the accumulated narrative weight of the entire sub-graph as experienced by the player.

```rust
fn sub_graph_collective_mass(sg: &SubGraphState) -> f32 {
    let base = sg.definition.base_mass;

    let visit_weight = sg.visit_history.iter()
        .enumerate()
        .map(|(i, visit)| {
            let diminishing = 1.0 / (1.0 + VISIT_DECAY * i as f32); // default VISIT_DECAY = 0.3
            visit.emotional_residue.intensity * diminishing
        })
        .sum::<f32>();

    let completion_bonus = if sg.completed {
        sg.definition.completion_bonus
    } else {
        0.0
    };

    let thematic_resonance = sg.definition.thematic_resonance;

    base + visit_weight + completion_bonus + thematic_resonance
}
```

### Diminishing Visit Weight

Each sub-graph visit contributes emotional residue, but with diminishing returns:

```
visit_weight(i) = emotional_intensity(visit_i) / (1 + 0.3 × i)
```

| Visit | Multiplier | Effective Contribution (intensity 0.8) |
|-------|-----------|--------------------------------------|
| 1st | 1.0 | 0.80 |
| 2nd | 0.77 | 0.62 |
| 3rd | 0.63 | 0.50 |
| 5th | 0.45 | 0.36 |

First visits have the most impact. Subsequent visits add weight but less — the fairy tale's shock of newness fades, though its cumulative mass continues to grow.

---

## Boundary Permeability Dynamics

### The Sigmoid Model

Boundary permeability follows a sigmoid curve — slow initial change, rapid transition in the middle, asymptotic approach to the final state:

```
Ψ(t) = Ψ_min + (Ψ_max - Ψ_min) / (1 + e^(-k(t - t₀)))
```

Where:
- `Ψ_min` = initial permeability (story-configured, default 0.1)
- `Ψ_max` = maximum permeability (default 1.0 for convergence-capable boundaries)
- `k` = steepness (how rapidly permeability changes; default 0.5)
- `t₀` = midpoint (narrative time when permeability = 0.5)

### Event-Driven Perturbation

Events can push permeability beyond the natural sigmoid curve:

```rust
fn permeability_at(
    boundary: &BoundaryPermeability,
    current_turn: TurnId,
) -> f32 {
    let base = sigmoid(
        current_turn as f32,
        boundary.psi_min,
        boundary.psi_max,
        boundary.steepness,
        boundary.midpoint,
    );

    let event_perturbation: f32 = boundary.perturbations.iter()
        .map(|p| {
            let age = current_turn - p.turn;
            p.magnitude * (-PERTURBATION_DECAY * age as f32).exp() // Perturbations decay
        })
        .sum();

    (base + event_perturbation).clamp(0.0, 1.0)
}

fn sigmoid(t: f32, min: f32, max: f32, k: f32, t0: f32) -> f32 {
    min + (max - min) / (1.0 + (-k * (t - t0)).exp())
}
```

Perturbations decay exponentially — a revelation event spikes permeability temporarily, then the boundary settles back toward the sigmoid curve.

### What Crosses at Each Permeability Level

The permeability value determines which cascade categories can cross the boundary, as formalized in [`traversal-friction.md`](traversal-friction.md):

```rust
fn category_filter(signal: &CascadeSignal, psi: f32) -> f32 {
    match signal.category {
        CascadeCategory::Thematic => {
            if psi >= 0.0 { 1.0 } else { 0.0 } // Always crosses (thematic is lightest)
        }
        CascadeCategory::Emotional => {
            if psi >= 0.2 { (psi - 0.2) / 0.3 } else { 0.0 } // Gradual from 0.2 to 0.5
        }
        CascadeCategory::Informational => {
            if psi >= 0.5 { (psi - 0.5) / 0.2 } else { 0.0 } // Gradual from 0.5 to 0.7
        }
        CascadeCategory::Relational => {
            if psi >= 0.7 { (psi - 0.7) / 0.3 } else { 0.0 } // Gradual from 0.7 to 1.0
        }
    }
}
```

---

## Prophetic Cascade Lifecycle

### Definition

A **prophetic cascade** is a forward-pointing approach vector from a sub-graph event to an unreached parent-graph scene. It modifies the target scene's gravitational properties without the scene being aware of the source.

```rust
struct PropheticCascade {
    source_sub_graph: SubGraphId,
    source_event: EventId,
    target_scene: SceneId,
    prophecy_type: ProphecyType,
    approach_vector_modifier: ApproachVector,
    fulfillment_condition: Option<EventConditionId>,
    created_at: TurnId,
    base_magnitude: f32,
}
```

### Lifecycle Stages

```
Created (sub-graph exit) → Active (accumulating) → Fulfilled (target scene played) or Decayed (unfulfilled)
```

### Magnitude Over Time

Active prophetic cascades have a magnitude that grows with additional sub-graph visits and slowly decays if unfulfilled:

```rust
fn prophetic_magnitude(
    prophecy: &PropheticCascade,
    current_turn: TurnId,
    reinforcements: &[PropheticReinforcement],
) -> f32 {
    let base = prophecy.base_magnitude;

    // Reinforcements from additional sub-graph visits
    let reinforcement_bonus: f32 = reinforcements.iter()
        .map(|r| r.magnitude * 0.5) // Each reinforcement at half strength
        .sum();

    // Slow decay if unfulfilled
    let age = current_turn - prophecy.created_at;
    let decay = (-PROPHETIC_DECAY_RATE * age as f32).exp(); // default: 0.01 per turn

    (base + reinforcement_bonus) * decay
}
```

### Compounding: Multiple Prophecies

When multiple prophetic cascades target the same scene, their effects compound:

```rust
fn total_prophetic_pressure(
    target_scene: SceneId,
    active_prophecies: &[PropheticCascade],
    current_turn: TurnId,
) -> f32 {
    active_prophecies.iter()
        .filter(|p| p.target_scene == target_scene)
        .map(|p| prophetic_magnitude(p, current_turn, &p.reinforcements))
        .sum()
}
```

A scene with three prophetic cascades (from three separate sub-graph events) has three times the prophetic pressure of a scene with one. This compounds with the scene's inherent gravitational mass — the total pull is enormous, bending the narrative toward the prophesied scene.

---

## Token Budget Allocation

### The Greedy Knapsack

The retrieved context (Tier 3) has a hard token budget (400-800 tokens). After scoring all candidates, the Storykeeper must select which candidates to include. This is a variant of the **knapsack problem** — maximize total composite score within a token budget.

```rust
fn allocate_budget(
    candidates: &mut Vec<ScoredCandidate>,
    budget: usize, // tokens
) -> Vec<ScoredCandidate> {
    // Sort by composite_score / estimated_tokens (score density)
    candidates.sort_by(|a, b| {
        let density_a = a.composite_score / a.estimated_tokens as f32;
        let density_b = b.composite_score / b.estimated_tokens as f32;
        density_b.total_cmp(&density_a) // Descending
    });

    let mut selected = Vec::new();
    let mut remaining_budget = budget;

    // Mandatory items first (entity context for player references, active revelations)
    for candidate in candidates.iter() {
        if candidate.mandatory && candidate.estimated_tokens <= remaining_budget {
            remaining_budget -= candidate.estimated_tokens;
            selected.push(candidate.clone());
        }
    }

    // Greedy fill with remaining candidates by score density
    for candidate in candidates.iter() {
        if candidate.mandatory { continue; } // Already included
        if candidate.estimated_tokens > remaining_budget { continue; } // Doesn't fit
        if candidate.composite_score < MIN_CANDIDATE_SCORE { continue; } // Below threshold

        remaining_budget -= candidate.estimated_tokens;
        selected.push(candidate.clone());
    }

    selected
}
```

### Budget Structure

```
Total Budget: 400-800 tokens

Mandatory (always included):
  Active revelations (if any):           ~50-100 tokens
  Entity context for player references:  ~100-200 tokens

Gravity-Driven (fill by composite score):
  Gravitational landscape context:       ~100-200 tokens
  Sub-graph resonance:                   ~50-150 tokens

Enrichment (remaining budget):
  Emotional state annotations:           ~50-100 tokens
  Thematic echo context:                 ~50-100 tokens
```

When budget is tight (400 tokens), gravity-driven and enrichment candidates compete. When generous (800 tokens), more gravitational context survives, producing richer Narrator material.

### What Gravity Displaces

High-gravity candidates displace low-gravity candidates. In practice:

- When a high-mass scene is nearby: its thematic material displaces generic backstory
- When a sub-graph has high collective mass and permeable boundaries: its imagery displaces main-narrative-only context
- When multiple prophetic cascades target the same scene: that scene's thematic register dominates the retrieved tier

This displacement is the mechanism through which gravity shapes prose. The Narrator writes toward gravitational attractors because their themes occupy more of the context window.

---

## Vretil Chapter 16: Complete Worked Example

Chris arrives in New Mexico. Three cross-graph systems converge.

### Gravitational State

| Component | Value | Source |
|-----------|-------|--------|
| Cave scene effective mass | 2.1 | High authored base + gates + emotional hinge + approach satisfaction |
| Fairy tale collective mass | 0.8 | Three standalone chapters + completion bonus |
| Boundary permeability | 0.8 | Phase 4 (boundaries collapsing) |
| Active prophetic cascades | 1 | Fire-dream → cave scene |
| Bridge objects present | 1 | Datura flower (glass sculpture/bell-blossom/sacred plant) |

### Candidate Generation and Scoring

**Candidate 1: Sarah's relational context** (new character)
```
initial_relevance = 1.0 (entity-triggered, highest priority)
gravitational_modifier = 1.0 (no gravitational alignment)
boundary_factor = 1.0
recency = 1.0 (current turn)
temperature = 1.0

composite = 1.0 × 1.0 × 1.0 × 1.0 × 1.0 = 1.0
estimated_tokens = 150
```

**Candidate 2: Cave gravitational context** (stakes, thematic register)
```
initial_relevance = 0.7 (gravitational source)
gravitational_modifier:
  scene_component = G(current, cave) × 0.3
  where G = 2.1 / D² (cave is narratively very close)
  scene_component ≈ 2.1 / 0.1² × 0.3 = 63.0 (clamped to reasonable range)
  Actually: the raw pull is extremely high. Let's use normalized pull.
  Normalized: scene_component = 0.63 (after normalization to [0, 1])
  gravitational_modifier = 1.0 + 0.63 = 1.63
boundary_factor = 1.0
recency = 1.0
temperature = 1.0 (neutral)

composite = 0.7 × 1.63 × 1.0 × 1.0 × 1.0 = 1.14
estimated_tokens = 120
```

**Candidate 3: Datura bridge object** (cross-layer resonance)
```
initial_relevance = 0.7 (sub-graph resonance)
gravitational_modifier:
  sub_graph_component = 0.8 × 0.8 × 0.25 = 0.16
  gravitational_modifier = 1.0 + 0.16 = 1.16
boundary_factor = 1.0
recency = 0.95 (recent sub-graph visit)
temperature = 1.0

composite = 0.7 × 1.16 × 1.0 × 0.95 × 1.0 = 0.77
estimated_tokens = 80
```

**Candidate 4: Fire-dream prophetic imagery** (sand, cave, seed)
```
initial_relevance = 0.5 (prophetic cascade)
gravitational_modifier:
  prophetic_component = 0.5 × 0.2 = 0.10
  gravitational_modifier = 1.0 + 0.10 = 1.10
boundary_factor = 1.0
recency = 0.8 (prophecy from earlier chapter)
temperature = 1.0

composite = 0.5 × 1.10 × 1.0 × 0.8 × 1.0 = 0.44
estimated_tokens = 70
```

**Candidate 5: Chris's emotional trajectory**
```
initial_relevance = 0.8 (emotional state)
gravitational_modifier = 1.0 (no direct gravitational alignment)
boundary_factor = 1.0
recency = 1.0
temperature = 1.0

composite = 0.8
estimated_tokens = 80
```

**Candidate 6: Photographs reaching destination** (thematic echo)
```
initial_relevance = 0.6 (echo detection)
gravitational_modifier:
  scene_component = 0.3 (photographs relate to cave scene thematically)
  gravitational_modifier = 1.3
boundary_factor = 1.0
recency = 0.7
temperature = 1.0

composite = 0.6 × 1.3 × 1.0 × 0.7 × 1.0 = 0.55
estimated_tokens = 60
```

**Candidate 7: The Prophet's accumulated weight**
```
initial_relevance = 0.9 (entity-triggered)
gravitational_modifier = 1.0
boundary_factor = 1.0
recency = 0.6
temperature = 1.0

composite = 0.54
estimated_tokens = 40
```

### Budget Allocation (~600 tokens)

Sorted by score density (composite / tokens):

| Rank | Candidate | Score | Tokens | Density | Running Total |
|------|-----------|-------|--------|---------|---------------|
| 1 | Cave gravitational context | 1.14 | 120 | 0.0095 | 120 |
| 2 | Sarah's relational context | 1.00 | 150 | 0.0067 | 270 |
| 3 | Datura bridge object | 0.77 | 80 | 0.0096 | 350 |
| 4 | Chris's emotional trajectory | 0.80 | 80 | 0.0100 | 430 |
| 5 | Photographs echo | 0.55 | 60 | 0.0092 | 490 |
| 6 | Fire-dream imagery | 0.44 | 70 | 0.0063 | 560 |
| 7 | The Prophet | 0.54 | 40 | 0.0135 | 600 |

All seven candidates fit within the 600-token budget. The Narrator receives a context rich with: the cave's gravitational atmosphere, a new character (Sarah), the datura's cross-layer symbolic weight, Chris's emotional trajectory, the photographs reaching their destination, prophetic fire-dream imagery, and the Prophet's accumulated mystery.

**What the Narrator produces**: Prose where the desert corresponds to the fairy tale's landscape, the glass flower catches light in Chris's pocket, the photographs feel like a journey finding its endpoint, and the emotional register is one of terrifying clarity — approaching something that multiple narrative layers have been building toward.

The cave is never mentioned. Its gravitational field shapes every sentence.

---

## Computational Summary

| Operation | Algorithm | Complexity | Expected Latency |
|-----------|-----------|------------|-----------------|
| Composite scoring (all candidates) | Per-candidate formula | O(C) where C=candidates (~50-100) | < 2ms |
| Gravitational modifier | Max over attractors × candidate check | O(C × A) where A=attractors | < 1ms |
| Sub-graph collective mass | Sum over visit history | O(V) where V=visits | < 0.1ms |
| Boundary permeability | Sigmoid + perturbation sum | O(P) where P=perturbations | < 0.1ms |
| Prophetic pressure | Sum over active prophecies | O(prophecies) | < 0.1ms |
| Budget allocation | Sort + greedy fill | O(C log C) | < 0.5ms |
| **Total context assembly** | All of the above composed | | **< 5ms** |

This is within the context assembly budget (~25-70ms total, of which scoring is a small fraction).

---

## Theoretical Foundations

### What We Borrow

| Concept | Source | Our Adaptation |
|---------|--------|---------------|
| **Multiplicative scoring** | Information retrieval (TF-IDF, BM25) | Five multiplicative factors instead of term frequency |
| **Greedy knapsack** | Combinatorial optimization | Token budget as weight constraint, composite score as value |
| **Sigmoid dynamics** | Logistic growth (ecology, neural networks) | Boundary permeability evolution |
| **Exponential decay** | Recency weighting (search engines, recommendation systems) | Turn-based recency with configurable half-life |

### What We Invent

| Concept | Novelty |
|---------|---------|
| **Gravitational modifier as retrieval signal** | Using narrative gravity directly in retrieval ranking |
| **Prophetic cascade compounding** | Multiple forward-pointing sub-graph cascades summing |
| **Boundary permeability as cascade gate** | Dynamic permeability controlling which cascade categories cross |
| **Narrative temperature modifier** | Scene dramatic state adjusting which candidates surface |
| **Information boundary as absolute gate** | Binary exclusion regardless of gravitational weight |
| **Gravity as displacement mechanism** | High-gravity candidates occupying context space, displacing low-gravity material |

### What We Defer

- **Learning retrieval weights** — Tuning the five composite factors from player engagement data. Currently all weights are hand-configured.
- **Semantic similarity in gravitational alignment** — Currently "relates_to" and "resonates_with" are tag-based matches. Semantic vector similarity would enable fuzzy matching.
- **Multi-turn context coherence** — Currently each turn's context is assembled independently. Tracking what was in the previous turn's context to ensure coherence across turns is a natural extension.
- **Player-specific gravity** — Adjusting gravitational weights based on player preferences or play style. Currently all players experience the same gravitational landscape.
