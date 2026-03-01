# Traversal Friction Mathematics

## Purpose

This document formalizes the **traversal friction model** — the cross-cutting math that governs how signals propagate through graphs, attenuate with distance, and distort as they cross boundaries. The friction model applies to all four graphs but manifests differently in each. This document unifies those manifestations into a general framework.

**Prerequisites**: Familiarity with all four graph structures ([relational-web-math.md](relational-web-math.md), [narrative-gravity.md](narrative-gravity.md), [setting-topology.md](setting-topology.md), [event-dag.md](event-dag.md)).

**Source material**: [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) §Traversal Friction Model, [`tales-within-tales.md`](../tales-within-tales.md) §Information Flow.

---

## The General Friction Model

### Intuition

When something happens in the story — a trust shift, a revelation, a consequence — its effect doesn't propagate infinitely. Effects attenuate with distance, distort as they pass through intermediaries, and are filtered by the permeability of the edges they traverse. The friction model formalizes this: **signals weaken, change character, and get filtered as they propagate through graph structure**.

### Three Components

Every signal propagation in the system has three components:

```
signal_at(destination) = signal_at(source)
    × attenuation(distance)
    × Π(permeability(edge) for edge in path)
```

**1. Attenuation** — signal strength decreases with distance:

```
attenuation(d) = F^d
```

Where `F ∈ (0.0, 1.0)` is the **friction factor**. This is exponential decay — signal halves at each hop (for F=0.5).

**2. Permeability** — each edge filters the signal:

```
permeability(edge) ∈ [0.0, 1.0]
```

Permeability is edge-specific — some edges freely transmit signals, others block them.

**3. Distortion** — signal changes character at each hop (covered separately below).

### The Significance Threshold

Signals below a configurable threshold are dropped — they are too weak to matter narratively. The threshold determines the maximum propagation distance:

```
significance_threshold = 0.1 (default)

max_distance = ⌊log(significance_threshold) / log(F)⌋
```

For F = 0.5:
```
max_distance = ⌊log(0.1) / log(0.5)⌋ = ⌊-1.0 / -0.301⌋ = ⌊3.32⌋ = 3
```

Signals propagate at most 3 hops before falling below threshold. For F = 0.7 (tightly-knit community):
```
max_distance = ⌊log(0.1) / log(0.7)⌋ = ⌊-1.0 / -0.155⌋ = ⌊6.46⌋ = 6
```

Tighter communities (higher F) propagate further — gossip travels more hops in a village than in a city.

---

## Graph-Specific Friction

### Relational Web: Social Friction

The relational web is the primary friction domain. Signals are relational changes (trust shifts, information reveals, debt changes) propagating through the social network.

**Friction factor**: Story-configurable. Default `F = 0.5`.

| Community Type | Friction Factor | Max Hops | Example |
|---------------|----------------|----------|---------|
| Tight-knit village | 0.7 | 6 | Everyone knows everyone's business |
| Extended family | 0.5 | 3 | News travels through close relations |
| Urban strangers | 0.3 | 1-2 | Effects barely propagate beyond direct contacts |
| Spies / secret-keepers | 0.2 | 1 | Deliberate information containment |

**Permeability computation**:

```rust
fn relational_permeability(edge: &RelationalEdge) -> f32 {
    let trust = trust_factor(edge);
    let history = history_factor(edge);
    let opacity = opacity_modifier(edge);

    trust * history * (1.0 - opacity)
}

fn trust_factor(edge: &RelationalEdge) -> f32 {
    // Average of trust dimensions, biased toward benevolence
    // (benevolence = "will they share honestly?" is more relevant to signal transmission)
    let s = &edge.substrate;
    (s.trust_competence + s.trust_benevolence * 2.0) / 3.0
}

fn history_factor(edge: &RelationalEdge) -> f32 {
    // Deeper relationships carry signals more reliably
    edge.history.depth.min(1.0)
}

fn opacity_modifier(edge: &RelationalEdge) -> f32 {
    // Secret-keepers have high opacity on outgoing edges
    // 0.0 = transparent, 1.0 = fully opaque
    edge.opacity
}
```

### Narrative Graph: Gravitational Friction

The narrative graph does not have friction in the signal-propagation sense — gravitational pull is computed directly via inverse-square distance, not through edge-by-edge propagation. However, **approach vector satisfaction** acts as a proximity modifier analogous to friction:

```
effective_pull(from, to) = M(to) / D(from, to)²
```

There is no per-edge permeability — scenes don't filter gravitational pull. The "friction" is in the distance metric itself: a scene that is informationally, emotionally, relationally, and physically distant has high narrative distance, and pull falls off with the square of that distance.

### Setting Topology: Physical Friction

The setting topology uses **traversal cost** rather than signal attenuation. Friction here is physical — it costs time, effort, and risk to move between locations.

```
effective_cost(path) = Σ_{edge ∈ path} effective_cost(edge, entity)
```

There is no permeability (if a path exists, it's traversable — unless gated) and no signal attenuation (spatial information doesn't degrade with distance). The friction is purely in the cost of traversal.

### Event DAG: Inverse Friction (Amplification)

The event DAG has **no friction** in the propagation sense — resolution is binary (it either happened or it didn't). But the `Amplifies` edge type functions as **inverse friction**: it increases consequence magnitude rather than attenuating it.

```
consequence_magnitude(node) = base_magnitude × Π(amplification_weights)
```

Where the friction model attenuates signals toward zero, amplification compounds them toward larger values (bounded by MAX_AMPLIFICATION).

### Comparison Table

| Graph | Signal Type | Attenuation | Permeability | Distortion | Special |
|-------|------------|-------------|-------------|------------|---------|
| Relational Web | Relational changes | Exponential (F^d) | Trust × history × (1-opacity) | Category shift at each hop | Multi-path: strongest signal wins |
| Narrative Graph | Gravitational pull | Inverse-square (1/D²) | None (scenes don't filter) | None | Distance is multi-dimensional |
| Setting Topology | Physical traversal | Cost accumulation | Gated (binary) | None | Entity-specific costs |
| Event DAG | Consequence magnitude | None | None | None | Amplification (inverse friction) |

---

## Category-Shift Distortion

### The Problem

When relational signals propagate through intermediaries, they don't just get weaker — they change character. Sarah's trust shift toward Adam at distance 0 is a specific trust-dimension change. By the time it reaches Kate (distance 1), it has become a less specific "something is wrong." By distance 2, it's a vague "there's tension in the family."

### The Distortion Model

At each hop, the signal undergoes a **category shift** — it moves from a specific substrate dimension to a more generalized category:

| Distance | Information Character | Category |
|----------|---------------------|----------|
| 0 | Direct observation/experience | Specific substrate dimension (e.g., trust_benevolence shift) |
| 1 | Secondhand knowledge/perception | Same dimension, reduced specificity |
| 2 | Social inference | Generalized to `history.quality_trajectory` |
| 3+ | Vague awareness | Below significance threshold — dropped |

```rust
fn distort_signal(signal: &RelationalSignal, distance: usize) -> DistortedSignal {
    match distance {
        0 => DistortedSignal::Specific {
            dimension: signal.dimension,
            delta: signal.delta,
        },
        1 => DistortedSignal::Reduced {
            dimension: signal.dimension,
            delta: signal.delta * 0.7, // Less precise
            note: "perceived shift",
        },
        2 => DistortedSignal::Generalized {
            category: SignalCategory::HistoryQuality,
            description: generalize_signal(signal),
            magnitude: signal.delta.abs() * 0.3,
        },
        _ => DistortedSignal::BelowThreshold,
    }
}

fn generalize_signal(signal: &RelationalSignal) -> String {
    match signal.dimension {
        Dimension::TrustBenevolence | Dimension::TrustCompetence => "tension".to_string(),
        Dimension::Affection => "warmth_shift".to_string(),
        Dimension::Debt => "obligation_shift".to_string(),
        _ => "relational_change".to_string(),
    }
}
```

### TFATD Worked Example: Distortion Chain

**Signal**: Sarah's trust_benevolence toward Adam drops from 0.4 to 0.2 (delta = -0.2).

| Distance | Recipient | Signal Received |
|----------|-----------|----------------|
| 0 | Sarah | Specific: trust_benevolence -0.2 toward Adam |
| 1 | Kate | Reduced: trust toward Adam shifted, perceived as wariness (delta ≈ -0.14) |
| 2 | John | Generalized: "tension in the family" (magnitude 0.06) — below threshold |

Kate would perceive Sarah's wariness if the signal strength (after attenuation and permeability) exceeds threshold. John almost certainly would not — the signal is both attenuated and generalized beyond recognition.

---

## Three-Component Permeability

### Decomposition

Relational permeability has three independent components that multiply together:

```
P(edge) = P_trust(edge) × P_history(edge) × P_opacity(edge)
```

Each component captures a different aspect of how freely information flows:

### Component 1: Trust Permeability

```
P_trust = (trust_competence + 2 × trust_benevolence) / 3
```

Benevolence is weighted double because "will they share honestly?" matters more than "can they understand what I'm sharing?" for information propagation. High benevolence trust = high signal fidelity. Low benevolence trust = signal filtered or distorted by the transmitter.

### Component 2: History Permeability

```
P_history = min(history.depth, 1.0)
```

Deeper relationships transmit more reliably. A relationship at `depth = 0.2` (brief encounter) is 80% less permeable than a bedrock relationship at `depth = 1.0`.

### Component 3: Opacity (Inverse Permeability)

```
P_opacity = 1.0 - opacity
```

Opacity is the only permeability component that can be **intentionally set** by the narrative. A secret-keeper character has high opacity on their outgoing edges — they deliberately prevent signal transmission.

### TFATD Worked Example: Permeability Across the Web

| Edge | P_trust | P_history | P_opacity | Total P |
|------|---------|-----------|-----------|---------|
| Sarah → Kate | (0.7 + 2×0.9)/3 = 0.83 | 0.9 | 1.0 | **0.75** |
| Kate → John | (0.7 + 2×0.9)/3 = 0.83 | 0.8 | **0.3** (Kate keeps secrets) | **0.20** |
| Sarah → Adam | (0.7 + 2×0.4)/3 = 0.50 | 0.2 | 1.0 | **0.10** |
| Tom → Beth | (0.6 + 2×0.9)/3 = 0.80 | 0.7 | 1.0 | **0.56** |
| Adam → Tom | (0.4 + 2×0.0)/3 = 0.13 | 0.3 | 0.5 | **0.02** |

Key observations:
- **Kate → John** has very low permeability despite high trust — Kate's deliberate secret-keeping (opacity 0.7) dominates
- **Adam → Tom** is nearly opaque — Adam has no benevolent trust toward Tom and deliberately withholds information
- **Sarah → Kate** is the highest-permeability edge — high trust, deep history, no secrets

---

## Multi-Path Resolution

### The Problem

When multiple paths exist between a signal source and a destination, each path delivers a different signal strength. Which one determines the effect at the destination?

### Strongest Signal Wins

The path with the maximum delivered signal determines the effect. This is a modified Dijkstra's algorithm where we maximize rather than minimize:

```rust
fn strongest_signal_path(
    web: &RelationalWeb,
    source: EntityId,
    destination: EntityId,
    initial_signal: f32,
    friction_factor: f32,
) -> f32 {
    let mut best_signal: HashMap<EntityId, f32> = HashMap::new();
    let mut queue = BinaryHeap::new();

    best_signal.insert(source, initial_signal);
    queue.push(MaxSignalEntry { entity: source, signal: initial_signal });

    while let Some(current) = queue.pop() {
        if current.entity == destination {
            return current.signal;
        }

        if current.signal < *best_signal.get(&current.entity).unwrap_or(&0.0) {
            continue; // Stale entry
        }

        for neighbor in web.out_neighbors(current.entity) {
            let edge = web.edge(current.entity, neighbor);
            let propagated = current.signal
                * relational_permeability(edge)
                * friction_factor;

            if propagated > *best_signal.get(&neighbor).unwrap_or(&0.0) {
                best_signal.insert(neighbor, propagated);
                queue.push(MaxSignalEntry { entity: neighbor, signal: propagated });
            }
        }
    }

    0.0 // No path found
}
```

### Why Strongest (Not Sum)

We take the maximum rather than summing across paths because:

1. **Information doesn't amplify through multiple channels** — hearing the same news from three people doesn't make it three times more impactful
2. **The best path represents the most reliable channel** — information arriving through a trusted intermediary is more influential than the same information overheard through gossip, regardless of how many gossip channels exist
3. **Summing would violate the significance threshold** — multiple below-threshold signals summing to above-threshold would create phantom propagation

### TFATD Worked Example: Multi-Path Signal

**Signal**: Tom's emotional state shifts (debt toward Beth overwhelms him). Multiple paths to Sarah:

**Path 1: Tom → Sarah** (direct, but Tom is absent/unreachable)
```
signal = 0.3 × P(Tom→Sarah) × F
       = 0.3 × 0.56 × 0.5  (Tom→Sarah has moderate permeability)
       = 0.084
```
Below threshold (0.1). And this path doesn't exist during the quest — Tom is absent.

**Path 2: Tom → Adam → Sarah** (through the Gate)
```
signal = 0.3 × P(Tom→Adam) × F × P(Adam→Sarah) × F
       = 0.3 × 0.02 × 0.5 × 0.10 × 0.5
       = 0.00015
```
Negligible. Adam's near-zero permeability toward Tom blocks essentially everything.

**Result**: Sarah receives no signal from Tom's emotional shift. This is narratively correct — Sarah doesn't know about Beth, about Tom's sacrifice, about any of it. The information boundaries are enforced by the friction model's permeability computation.

---

## Sub-Graph Boundary Friction

### Boundary Permeability

Sub-graph boundaries (from [`tales-within-tales.md`](../tales-within-tales.md)) are a special case of friction. The boundary acts as an additional permeability gate between narrative layers:

```
signal_across_boundary = signal × Ψ(boundary) × category_filter(signal, boundary)
```

Where `Ψ ∈ [0.0, 1.0]` is the boundary permeability and `category_filter` selects which signal categories can cross:

| Ψ Range | What Crosses | Example |
|---------|-------------|---------|
| 0.0 — 0.2 | Thematic resonance only | Fairy tale imagery faintly echoes |
| 0.2 — 0.5 | + Emotional residue | Sub-graph emotional weight colors current scene |
| 0.5 — 0.7 | + Informational content | Facts from sub-graph become available |
| 0.7 — 1.0 | + Relational effects | Sub-graph relationship changes propagate fully |
| 1.0 | Everything (merged) | Boundary dissolved |

### Sigmoid Permeability Dynamics

Permeability change follows a sigmoid curve (from `cross-graph-composition.md`):

```
Ψ(t) = 1 / (1 + e^(-k(t - t₀)))
```

Where `t₀` is the midpoint (when permeability = 0.5) and `k` controls the steepness. Events can shift the curve:

```rust
fn update_permeability(
    boundary: &mut BoundaryPermeability,
    driver: &PermeabilityDriver,
) {
    match driver {
        PermeabilityDriver::NarrativeProgression(delta) => {
            boundary.current += delta;
        }
        PermeabilityDriver::EntityCrossing(_) => {
            boundary.current += 0.05; // Each crossing erodes the boundary slightly
        }
        PermeabilityDriver::ObjectBridge(_) => {
            boundary.current += 0.03; // Bridge objects increase permeability
        }
        PermeabilityDriver::EventDriven(_, magnitude) => {
            boundary.current += magnitude; // Revelation events can spike permeability
        }
    }
    boundary.current = boundary.current.clamp(0.0, 1.0);
}
```

---

## Temporal Friction: Narrative-Chronological Bounds on Propagation

### The Problem

The friction model as described above is purely **structural** — it governs how signals attenuate across graph distance and permeability. But information propagation is also bounded by **time**. When a scene closes and the Storykeeper resolves relational changes, the question is: how far could those changes have propagated before the next scene begins?

The answer depends on the narrative-chronological duration between scenes and the communication affordances available in the setting. If two days pass between scenes, news can travel several relational hops. If five minutes pass — or the next scene picks up immediately from a different point of view — almost nothing has propagated beyond the immediate scene participants.

### Communication Velocity

Information travels at a rate determined by the setting's available communication channels. This is a **setting-level property**, not a per-edge property:

```rust
struct CommunicationAffordance {
    /// Maximum relational hops per narrative time unit
    velocity: f32,
    /// What information categories this channel carries
    categories: Vec<SignalCategory>,
}

struct SettingCommunication {
    /// Baseline: face-to-face, word-of-mouth
    baseline: CommunicationAffordance,
    /// Additional channels available in this setting
    channels: Vec<CommunicationAffordance>,
}
```

| Setting Type | Baseline Velocity | Additional Channels | Notes |
|-------------|------------------|--------------------| ------|
| Medieval village | 0.5 hops/day | Messenger (1.0 hops/day, factual only) | Gossip travels slowly; messengers carry specific news |
| Pre-industrial town | 1.0 hops/day | Post (0.3 hops/day, factual) | Dense social network but slow long-distance |
| Modern suburban | 3.0 hops/day | Phone/text (instant, factual + emotional) | Fast informal channels |
| Contemporary urban | 5.0 hops/day | Social media (instant, factual — distorted) | Many channels but low-fidelity |
| Fantasy with magic | Configurable | Scrying, sending spells (instant, targeted) | Author-defined affordances |

### Temporal Propagation Ceiling

The narrative-chronological duration between scenes imposes a **hard ceiling** on how many hops a signal can traverse:

```rust
fn temporal_propagation_ceiling(
    duration: f32,              // narrative time units between scenes
    setting: &SettingCommunication,
    signal_category: &SignalCategory,
) -> usize {
    // Find the fastest channel that carries this signal category
    let max_velocity = std::iter::once(&setting.baseline)
        .chain(setting.channels.iter())
        .filter(|ch| ch.categories.contains(signal_category))
        .map(|ch| ch.velocity)
        .fold(0.0_f32, f32::max);

    (duration * max_velocity).floor() as usize
}
```

The temporal ceiling interacts with the structural max propagation distance:

```
effective_max_hops = min(structural_max_hops, temporal_ceiling)
```

Where `structural_max_hops = ⌊log(threshold) / log(F)⌋` as defined earlier. The signal propagates no further than the more restrictive bound.

### Resolution Order

When a scene closes, the Storykeeper resolves information propagation in a specific order that accounts for temporal constraints:

```rust
fn resolve_scene_cascade(
    scene: &ClosedScene,
    next_scene: &SceneMetadata,
    web: &mut RelationalWeb,
    setting: &SettingCommunication,
) {
    let duration = next_scene.narrative_time - scene.narrative_time;

    // Step 1: Immediate — resolve relational changes for in-scene characters
    // These characters observed events directly. No friction, no delay.
    for change in &scene.relational_changes {
        web.apply_direct_change(change);
    }

    // Step 2: In-scene communication events — create temporary zero-distance edges
    // A character who sent a text message, cast a sending spell, or called someone
    // creates a direct information channel regardless of physical distance.
    let mut temporary_edges: Vec<TemporaryEdge> = Vec::new();
    for event in &scene.communication_events {
        temporary_edges.push(TemporaryEdge {
            from: event.sender,
            to: event.recipient,
            categories: event.information_categories.clone(),
            // Permeability of the communication channel itself
            permeability: event.channel_fidelity,
        });
    }

    // Step 3: Temporally-bounded cascade — propagate outward up to ceiling
    let ceiling = temporal_propagation_ceiling(
        duration, setting, &SignalCategory::General,
    );
    for signal in &scene.propagating_signals {
        propagate_with_ceiling(web, signal, ceiling, &temporary_edges);
    }

    // Step 4: Decay temporary edges — communication channels close
    // (The text was sent; the spell expired. The information was delivered,
    // but the channel doesn't persist.)
    temporary_edges.clear();
}
```

### In-Scene Communication as Zero-Distance Edges

When a character in-scene communicates with a character off-stage — sending a text, dispatching a messenger, casting a spell — the off-stage character becomes **temporarily adjacent** in the relational graph for that specific information transfer:

```rust
struct TemporaryEdge {
    from: EntityId,
    to: EntityId,
    /// What categories of information this channel carries
    categories: Vec<SignalCategory>,
    /// How faithfully the channel transmits (1.0 = perfect, 0.5 = lossy)
    permeability: f32,
}
```

The temporary edge bypasses temporal friction for the transmitted information — the recipient receives the signal as if at distance 1, regardless of how many structural hops separate them. However:

- The **channel permeability** still applies — a hurried text message has lower fidelity than a long conversation
- The **category filter** still applies — a text can convey facts but not the emotional subtlety of a face-to-face trust shift
- **Onward propagation** from the recipient is still temporally bounded — they received the information, but their ability to spread it further is constrained by the remaining duration

### TFATD Worked Example: Temporal Friction

**Scenario**: Sarah departs Adam's dwelling (end of S2). The next scene (S3, Kate's blessing) begins the following morning — approximately 0.5 days later.

Setting: pre-industrial rural. Baseline velocity = 0.5 hops/day. No additional channels (no phones, no magic).

```
temporal_ceiling = floor(0.5 days × 0.5 hops/day) = floor(0.25) = 0 hops
```

**Result**: With a temporal ceiling of 0, no cascade propagates beyond the immediate scene participants. Sarah's trust shift toward Adam remains entirely private — not because the structural friction blocked it (with F=0.7 it would reach Kate), but because there simply wasn't enough time for word to travel.

**Contrast**: If S3 occurred a week later (7.0 days):
```
temporal_ceiling = floor(7.0 × 0.5) = 3 hops
```

Now the structural limit (3 hops for F=0.5) governs, and Kate could potentially receive the signal through normal social channels — overheard conversations, changes in Sarah's behavior noticed over days.

**Communication event variant**: If Sarah had a magical sending stone and used it to tell Kate about Adam during S2:

```
temporary_edge: Sarah → Kate
    categories: [Factual, Emotional]
    permeability: 0.6 (magical communication loses nuance)

Signal at Kate: 0.25 × 0.6 = 0.15 (above threshold)
```

Kate receives the signal despite the temporal ceiling of 0 — the sending stone bypassed the time constraint. But Kate's ability to propagate this further to John is still temporally bounded: 0 hops of onward propagation before the next scene.

---

## Full Cascade Trace: TFATD Worked Example

### Scenario: Sarah Confronts Adam (Hypothetical S2 Extension)

Sarah's trust in Adam shifts: `trust_benevolence: 0.4 → 0.15` (delta = -0.25).

**Initial signal**: 0.25 (magnitude of shift)
**Friction factor**: F = 0.5
**Significance threshold**: 0.1

**Step 1: Direct edge update** (distance 0)
```
Adam→Sarah edge: Adam senses wariness (immediate cascade, same scene)
Signal at Adam: 0.25 × P(Sarah→Adam observed) = context-dependent
```
Adam is present — he observes the shift directly. No friction needed.

**Step 2: Propagation to Kate** (distance 1, deferred — Kate is not present)
```
Path: Sarah → Kate
Signal: 0.25 × P(Sarah→Kate) × F¹
      = 0.25 × 0.75 × 0.5
      = 0.094
```
Below threshold (0.094 < 0.1). Kate does **not** receive the signal. Sarah's trust shift toward Adam remains a private event.

But with friction factor 0.7 (tight-knit household):
```
Signal: 0.25 × 0.75 × 0.7 = 0.131
```
Above threshold. In a tighter community, Kate would sense something — "Sarah came back from Adam's different."

**Step 3: Propagation to John** (distance 2)
```
Path: Sarah → Kate → John
Signal: 0.131 × P(Kate→John) × F
      = 0.131 × 0.20 × 0.7
      = 0.018
```
Well below threshold. John perceives nothing even in the tight-knit configuration.

**Step 4: Distortion analysis**

If the signal to Kate were above threshold:
- At Kate (distance 1): "Sarah's wariness toward Adam" → perceived as "Sarah is unsettled after visiting Adam"
- At John (distance 2, if it reached): → generalized to "something's off in the family"

**Result**: The friction model correctly isolates Sarah's trust shift. The story's information boundaries — Sarah's private experience of Adam's strangeness — are maintained by the mathematical properties of exponential decay and permeability.

---

## Computational Summary

| Operation | Algorithm | Complexity | Expected Latency |
|-----------|-----------|------------|-----------------|
| Permeability computation | Per-edge formula | O(1) | < 0.01ms |
| Signal propagation (single path) | Product along path | O(path length) | < 0.01ms |
| Strongest signal (all paths) | Modified Dijkstra | O((V+E) log V) | < 2ms |
| Category-shift distortion | Per-hop transformation | O(path length) | < 0.01ms |
| Max propagation distance | Logarithmic formula | O(1) | < 0.01ms |
| Boundary permeability update | Increment + clamp | O(1) | < 0.01ms |
| Temporal ceiling computation | Max velocity × duration | O(channels) | < 0.01ms |
| Scene cascade resolution | 4-step sequential | O((V+E) log V) | < 5ms |

---

## Theoretical Foundations

### What We Borrow

| Concept | Source | Our Adaptation |
|---------|--------|---------------|
| **Exponential decay** | Signal attenuation (physics, network science) | Applied to social signal propagation |
| **Permeability** | Fluid dynamics, membrane transport | Applied to relational edges as information filters |
| **Modified Dijkstra** | Widest path problem (max-min path) | Adapted for max-product path (strongest signal) |
| **Sigmoid dynamics** | Logistic growth (ecology, neural networks) | Applied to boundary permeability evolution |

### What We Invent

| Concept | Novelty |
|---------|---------|
| **Category-shift distortion** | Signals changing semantic category at each hop (specific → general) |
| **Three-component permeability** | Trust + history + opacity decomposition |
| **Opacity as intentional permeability control** | Characters can deliberately reduce their outgoing permeability |
| **Sub-graph boundary as friction layer** | Narrative layer boundaries as permeability gates with sigmoid dynamics |
| **Unified friction comparison** | Comparing attenuation semantics across all four graph types |
| **Inverse friction (amplification)** | The event DAG's amplification as the dual of attenuation |
| **Temporal propagation ceiling** | Narrative-chronological duration bounding cascade reach, with setting-specific communication velocity |
| **In-scene communication as temporary edges** | Off-stage characters becoming temporarily adjacent via communication events |

### What We Defer

- **Frequency-dependent attenuation** — Different signal types (trust, affection, information) propagating with different friction factors. Currently all signals use the same F.
- **Adaptive friction** — Friction factor changing based on recent cascade history (communities that have been through crisis might propagate more freely). Interesting but adds state.
- **Bidirectional cascade** — Currently signals propagate forward only (from source outward). Bidirectional cascade (the destination "pulls" information from the source) is a natural extension but adds complexity.
- **Communication network as persistent graph** — Currently, communication channels are modeled as temporary edges created by in-scene events. A persistent communication network (who has whose phone number, which messengers serve which routes) would allow richer temporal friction modeling but adds a fifth graph to maintain.
