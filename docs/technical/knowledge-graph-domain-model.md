# Knowledge Graph Domain Model

## Purpose

This document defines the four graph structures in the storyteller system as **domain concepts** — not as database schemas or query implementations, but as the knowledge structures the Storykeeper reasons about. For each graph, we specify: what the vertices and edges mean, what questions the Storykeeper asks, how information propagates through the structure, and what policies govern that propagation.

The existing documentation describes graph *structure* (vertex types, edge labels, property schemas) but not graph *behavior* — how traversal works, what cascade means, how signal attenuates with distance, or how information state maps onto graph topology. This document fills that gap.

### The Problem This Solves

Without a unified domain model, each graph is implemented as an isolated data structure with ad-hoc query patterns. The relational web becomes a table of edges. The narrative graph becomes a table of scene connections. The event dependency DAG becomes a table of preconditions. And the cross-graph queries that the Storykeeper actually needs — "which reachable scenes contain characters with unresolved relational tension?" — become expensive joins between conceptually disconnected stores.

The domain model makes these queries natural by defining each graph as a domain concept with clear semantics, then identifying where graphs interact and what those interactions mean.

### Relationship to Other Documents

- **`storykeeper-api-contract.md`** — Defines the Storykeeper's read/write operations. This document defines the knowledge structures those operations query and mutate.
- **`storykeeper-crate-architecture.md`** — Defines where graph operations live in the workspace. This document defines what those operations do.
- **`relational-web-tfatd.md`** — Concrete case study for the relational web. This document formalizes the patterns observed there.
- **`narrative-graph-case-study-tfatd.md`** — Concrete case study for the narrative graph. This document formalizes the gravitational model.
- **`event-dependency-graph.md`** — Establishes the event DAG concept. This document integrates it into the unified graph domain model.
- **`event-system.md`** — Event taxonomy, truth set, cascade management. This document specifies how cascade policies operate on graph structure.

---

## The Four Graphs

The storyteller system maintains four distinct graph structures. They are not four views of one graph — they have different vertex types, different edge semantics, different update patterns, and different query profiles. But they are deeply interconnected: cross-graph queries are some of the Storykeeper's most important operations.

### Overview

| Graph | Vertices | Edges | Update Frequency | Primary Consumer |
|---|---|---|---|---|
| **Relational Web** | Entities (characters, presences, conditions) | Directed, asymmetric, substrate-dimensioned | Per-turn (cascade from committed events) | Prediction pipeline, context assembly, frame computation |
| **Narrative Graph** | Scenes | Directed, weighted (approach vectors, departure conditions) | Per-scene (mass recalculation at scene exit) | Scene lifecycle, narrative priming, departure evaluation |
| **Setting Topology** | Settings (locations) | Undirected or directed, with traversal cost | Rarely (authored, updated only by world events) | Scene entry, spatial reasoning, entity proximity |
| **Event Dependency DAG** | Event conditions | Directed, typed (Requires/Excludes/Enables/Amplifies) | Per-turn (incremental resolution propagation) | Trigger evaluation, gate checking, narrative planning |

---

## Graph 1: The Relational Web

### Domain Concept

The relational web is the network of relationships between entities. It answers the question: **"How does entity A perceive, feel about, and relate to entity B — and what does entity A know about that relationship?"**

Relationships are directed and asymmetric. Sarah→Adam is a different edge from Adam→Sarah. Each edge carries substrate dimensions that describe the relationship's character, plus information state that describes the factual knowledge asymmetry. Power is never stored — it emerges from substrate configuration and network topology.

### Vertex Type: Entity

Any entity with an `EntityId` can be a vertex in the relational web. In practice, only entities at `PromotionTier::Tracked` or `PromotionTier::Persistent` have relational edges — lower-tier entities haven't accumulated enough relational weight to warrant graph representation.

```
RelationalVertex {
    entity_id: EntityId,
    promotion_tier: PromotionTier,
    topological_role: TopologicalRole,     // Gate, Bridge, Hub, Periphery
    cluster_membership: Option<ClusterId>, // which social cluster this entity belongs to
}
```

Topological role and cluster membership are **computed properties** — derived from the graph's structure, not stored as authored data. They are recomputed when edges change significantly.

### Edge Type: Directed Relational Edge

Each edge carries the five substrate dimensions defined in `storyteller-core/src/types/relational.rs`, plus information state and a configuration annotation.

```
RelationalEdge {
    source: EntityId,          // the entity holding this perspective
    target: EntityId,          // the entity being related to
    substrate: RelationalSubstrate {
        trust_reliability: f32,
        trust_competence: f32,
        trust_benevolence: f32,
        affection: f32,
        debt: f32,
    },
    information_state: InformationState {
        known_facts: Vec<Proposition>,
        beliefs: Vec<WeightedProposition>,
        blind_spots: Vec<Proposition>,
    },
    history: RelationalHistory {
        depth: f32,                     // how long the relationship has existed
        quality_trajectory: String,     // "warm → strained", "passionate → devastating"
        temporal_layer: TemporalLayer,  // topsoil, sediment, or bedrock
        key_events: Vec<EventRef>,      // references to event ledger entries
    },
    projection: Projection {
        description: String,            // what source imagines target to be
        accuracy: ProjectionAccuracy,   // how close projection is to reality
    },
    configuration: String,              // emergent relational dynamic (descriptive)
    last_updated: TurnId,               // when this edge was last modified
    provenance: ProvisionalStatus,      // Hypothesized → Rendered → Committed
}
```

**Design note**: The `history`, `projection`, and `configuration` fields extend beyond the current Rust types in `relational.rs`. The existing `DirectedEdge` struct is the foundation; these extensions are the domain model that the Storykeeper needs for full relational reasoning. The Rust types will grow to match as implementation progresses.

### Edge Properties: The Substrate

The five substrate dimensions are not independent — their *configuration* (how they interact in context) is where relational meaning lives. The Storykeeper reasons about configurations, not individual dimensions:

| Configuration Pattern | Substrate Signature | Example (TFATD) |
|---|---|---|
| **Wary dependence** | High trust_competence, low trust_benevolence, acknowledged debt | Sarah → Adam |
| **Fierce release** | High trust_competence toward target, high affection, zero debt | Kate → Sarah |
| **Devoted unknowing** | High affection, low information, inaccurate projection | John → Kate |
| **Instrumental condescension** | Low trust_competence toward target, deliberate debt creation, high info asymmetry | Adam → Sarah |
| **Desperate sacrifice** | Overwhelming affection + debt, devastating history | Tom → Beth |
| **Grief as force** | Target's state shapes source's behavior without directed agency | Beth → the web |

The configuration field is descriptive text — it captures the qualitative relational dynamic for the psychological frame computation layer and for observability. It is not used in mechanical trigger evaluation (that uses the numeric substrate dimensions and information state).

### Query Patterns

These are the questions the Storykeeper asks of the relational web, expressed as domain operations (not SQL/Cypher):

#### Direct Edge Queries

| Query | Purpose | Called By |
|---|---|---|
| `edge(A, B)` | What is A's relationship toward B? | Prediction pipeline, frame computation |
| `bounded_edge(A, B, observer)` | What does `observer` perceive about A→B? | Context assembly (boundary-aware) |
| `edge_pair(A, B)` | Both A→B and B→A together — the full dyad | Relational analysis, observability |
| `edge_delta(A, B, since: TurnId)` | How has A→B changed since a given turn? | Cascade propagation, narrative priming |

#### Neighborhood Queries

| Query | Purpose | Called By |
|---|---|---|
| `neighborhood(A)` | All edges from/to A | Frame computation (full relational context) |
| `cast_subgraph(entities)` | All edges between a set of entities | Scene entry (load relational context for cast) |
| `dimension_extremes(A, dimension)` | Who does A trust most/least? Owe most? | Prediction features, narrative priming |
| `tension_edges(threshold)` | Edges where substrate dimensions are in conflict | Gate proximity, narrative priming |

#### Structural Queries

| Query | Purpose | Called By |
|---|---|---|
| `shortest_path(A, B)` | Social distance between entities | Cascade attenuation computation |
| `topological_role(A)` | Is A a Gate, Bridge, Hub, or Periphery? | Power dynamics computation |
| `clusters()` | Identify social clusters in the current web | Scene composition, narrative analysis |
| `betweenness(A)` | How much information flows through A? | Structural power computation |
| `triangles(A)` | Triadic patterns involving A (trust transitivity) | Social dynamics, cascade paths |

#### Temporal Queries

| Query | Purpose | Called By |
|---|---|---|
| `edge_history(A, B)` | Full history of A→B across turns/scenes | Observability, training data |
| `edges_changed_since(turn)` | All edges modified since a given turn | Checkpoint delta computation |
| `projection_accuracy_drift(A, B)` | Is A's projection of B becoming more/less accurate? | Narrative priming (collision approaching) |

### Cascade Policy

When a committed event produces relational implications — a trust shift, an information reveal, a debt change — the Storykeeper must decide what propagates through the web and how.

#### Cascade Scope

**Immediate cascade** (within the current turn): Changes to edges between entities present in the current scene. If Sarah's trust in Adam shifts during a scene where both are present, Adam may perceive the shift — the edge Adam→Sarah may adjust its `information_state` to reflect "Adam senses Sarah's wariness."

**Deferred cascade** (scene boundary): Changes to edges involving entities not present. Tom is not in the scene when Sarah's trust in Adam shifts. Any effect on the Tommy-Sarah or Tommy-Adam edges is deferred to scene-exit batch processing.

**Structural cascade** (never automatic): Changes to topological roles and cluster membership. These are recomputed only when the Storykeeper determines that edge changes are structurally significant — not on every turn, but when edges cross significance thresholds.

#### The Friction Model

The friction model governs how relational changes propagate through the web. It has three components:

**1. Attenuation** — signal strength decreases with distance:

```
signal_at_distance(d) = signal_at_source * friction_factor^d

where friction_factor ∈ (0.0, 1.0)
```

Default `friction_factor = 0.5`:
- Distance 0 (source edge): 100% — the direct relationship updates
- Distance 1 (direct neighbor): 50% — neighbors perceive the shift
- Distance 2 (friend-of-friend): 25% — distant awareness
- Distance 3+: Below significance threshold (default 0.1) — dropped

The friction factor is **story-configurable**: a tightly-knit community (low friction, 0.7) propagates more than a society of strangers (high friction, 0.3).

**2. Distortion** — information changes character as it propagates:

| Distance | Information Character | Category Shift |
|---|---|---|
| 0 | Direct observation/experience | Specific substrate dimension (e.g., trust shift) |
| 1 | Secondhand knowledge/perception | Same dimension, reduced specificity |
| 2 | Social inference | Generalized to `history.quality_trajectory` |
| 3+ | Vague awareness | Below threshold — not propagated |

Distortion is modeled as a **category shift**: at each hop, the signal loses specificity. A trust shift at distance 0 remains a trust shift. At distance 2, it becomes a generalized "tension" that affects the `history` dimension rather than `trust` specifically.

**3. Permeability** — not all edges propagate equally:

```
permeability(edge) = base_permeability
    * trust_factor(edge.substrate.trust_competence, edge.substrate.trust_benevolence)
    * history_factor(edge.history.depth)
    * opacity_modifier(edge)

effective_signal = signal * permeability(traversed_edge)
```

- **High-trust edges** propagate more signal — information flows more freely between entities that trust each other
- **Deep history** carries information better than new relationships
- **Opacity markers** explicitly block propagation — a character who keeps secrets has low permeability on outgoing edges

The friction model composes multiplicatively across hops:

```
signal_at_B_via_path(A→X→B) = signal_at_A
    * friction_factor^distance(A, X)
    * permeability(edge_A→X)
    * friction_factor^distance(X, B)
    * permeability(edge_X→B)
```

When multiple paths exist between source and target, the **strongest signal wins** — the path with the least attenuation determines the effect.

---

## Graph 2: The Narrative Graph

### Domain Concept

The narrative graph is the gravitational landscape of the story. It answers the question: **"Where can the story go from here, and where does it want to go?"**

Scenes are not branches on a tree. They are gravitational bodies with mass that attracts the narrative. The player navigates a landscape of attractor basins — regions where accumulated narrative mass pulls the story toward pivotal moments. The story's "natural" direction at any moment is determined by the strongest gravitational pull the player can feel.

### Vertex Type: Scene

```
SceneVertex {
    scene_id: SceneId,
    scene_type: SceneType,           // Gravitational, Connective, Gate, Threshold
    narrative_mass: NarrativeMass {
        authored_base: f32,          // set by story designer
        structural_modifier: f32,    // computed from graph position
        dynamic_adjustment: f32,     // computed from player state
    },
    tonal_signature: Vec<String>,    // "elegiac", "tense", "wondrous"
    thematic_register: Vec<String>,  // "loss", "power_shift", "threshold_crossing"
    temporal_marker: TemporalMarker, // Before (backstory) or Now (active quest)
    required_presences: Vec<EntityId>,
    contingent_presences: Vec<EntityId>,
    information_gates: Vec<InformationGate>,
    setting_id: Option<SettingId>,   // link to setting topology
    activation_state: SceneActivationState,
}

SceneActivationState {
    Dormant,       // not reachable from current position
    Reachable,     // reachable but not yet in attractor basin
    Attracting,    // player is in this scene's attractor basin
    Active,        // player is currently in this scene
    Completed,     // scene has been played
    Revisitable,   // completed but can be re-entered (different approach vector)
}
```

### Edge Type: Scene Transition

```
SceneTransition {
    from_scene: SceneId,
    to_scene: SceneId,
    departure_type: DepartureType,
    approach_vectors: Vec<ApproachVector>,  // state predicates for different entry experiences
    departure_conditions: Vec<DepartureCondition>,  // what must be true to leave this way
    momentum: f32,                          // how strongly this transition pulls
    narrative_weight: f32,                  // how much this transition contributes to story
}

DepartureCondition {
    predicate: TriggerPredicate,   // evaluated against truth set
    description: String,           // designer-readable description
}
```

### The Gravitational Model

Narrative mass determines how strongly a scene attracts the story:

```
effective_mass(scene, player_state) =
    scene.narrative_mass.authored_base
    + scene.narrative_mass.structural_modifier
    + approach_vector_satisfaction(scene, player_state) * 0.2

gravitational_pull(player_position, target_scene) =
    target_scene.effective_mass / narrative_distance(player_position, target_scene)^2
```

**Narrative distance** is not spatial — it is a weighted combination of:

```
narrative_distance(player_position, target_scene) =
    w_info  * information_distance(player.info_state, scene.required_info)
    + w_emo  * emotional_distance(player.emotional_state, scene.optimal_emotional)
    + w_rel  * relational_distance(player.relationships, scene.required_relationships)
    + w_phys * physical_distance(player.setting, scene.setting)
```

Weights are scene-specific — a Gate scene might weight information distance heavily (you need to know things to reach it), while a Gravitational scene might weight emotional distance (you need to feel things for it to land).

### Connective Space

Between major scenes lies connective space — the travel, atmosphere, and small interactions that build texture. Connective space has a **distributed mass model**:

```
connective_mass(time_spent) = base_mass_per_unit * time_spent * diminishing_factor(time_spent)
```

Connective space accumulates mass as the player engages with it, rewarding exploration without punishing speed. The diminishing factor prevents infinite mass accumulation from players who never leave.

### Query Patterns

| Query | Purpose | Called By |
|---|---|---|
| `reachable_scenes(current)` | What scenes can the player reach from here? (N-hop traversal) | Scene lifecycle, departure evaluation |
| `gravitational_pull(current, target)` | How strongly does target scene attract from current position? | Narrative priming, departure ranking |
| `strongest_attractor(current, player_state)` | Which reachable scene has the highest pull right now? | Narrative default direction |
| `attractor_basin(current)` | Which attractor basin is the player currently in? | Narrative trajectory analysis |
| `departure_conditions(current)` | What must be true to leave the current scene by each path? | Departure evaluation |
| `approach_satisfaction(target, player_state)` | How well does the player's current state match the target's approach vectors? | Gravitational pull computation |
| `mass_recalculation(scene, events)` | How do recent events change this scene's mass? | Scene exit batch processing |
| `narrative_trajectory()` | Given current state and gravitational landscape, what is the likely story arc? | Observability, narrative planning |

### Cascade Policy

The narrative graph cascades on a **scene-boundary schedule**: when a scene exits, the Storykeeper recalculates gravitational mass for affected scenes.

- **Events in the current scene** can increase/decrease mass of connected scenes. A revelation in Scene A might make Scene C (three hops away) suddenly the strongest attractor by satisfying approach vector conditions.
- **Gate satisfaction** changes reachability — when an information gate opens, previously-dormant scenes may become reachable.
- **Player trajectory** shifts the dynamic adjustment component of mass — scenes that align with the player's emotional and informational trajectory have higher effective mass.

Mass recalculation is a **deferred operation** (scene-exit batch), not a per-turn operation. The narrative landscape is stable during a scene and shifts between scenes.

---

## Graph 3: The Setting Topology

### Domain Concept

The setting topology is the spatial geography of the story world. It answers the question: **"Where are things, and how do you get between them?"**

Settings are locations — re-enterable places with spatial relationships. Unlike the narrative graph (which is about story structure) and the relational web (which is about entity relationships), the setting topology is about **physical constraints**: what is adjacent, what is traversable, what costs effort to reach.

### Vertex Type: Setting

```
SettingVertex {
    setting_id: SettingId,
    name: String,
    description: String,
    setting_type: SettingType,    // Interior, Exterior, Liminal, Abstract
    accessibility: Accessibility, // Open, Gated, Seasonal, Conditional
    current_entities: Vec<EntityId>,  // who/what is currently here
    scenes_at_setting: Vec<SceneId>,  // narrative graph cross-reference
}

SettingType =
    | Interior        // house, cave, room — enclosed, bounded
    | Exterior        // forest, field, river — open, traversable
    | Liminal         // threshold, crossing, borderland — between places
    | Abstract        // "the dream," "the memory" — non-physical setting
```

### Edge Type: Spatial Connection

```
SpatialConnection {
    from_setting: SettingId,
    to_setting: SettingId,
    directionality: Directionality,  // Bidirectional, OneWay, Conditional
    traversal_cost: TraversalCost {
        time: f32,               // narrative time units
        effort: f32,             // physical difficulty
        risk: f32,               // danger of the journey
    },
    traversal_conditions: Vec<TriggerPredicate>,  // what must be true to traverse
    description: String,         // "through the mist", "across the stream"
}

Directionality =
    | Bidirectional              // can go either way freely
    | OneWay                     // only in the authored direction
    | Conditional {              // direction depends on conditions
        forward_condition: TriggerPredicate,
        reverse_condition: TriggerPredicate,
    }
```

### Query Patterns

| Query | Purpose | Called By |
|---|---|---|
| `adjacent(setting)` | What settings are directly connected? | Scene entry, spatial reasoning |
| `path(from, to)` | Can I reach setting B from A? What's the path? | Pathfinding, entity proximity |
| `reachable(from, max_cost)` | What settings are within traversal budget? | Entity proximity, off-screen reasoning |
| `entities_at(setting)` | Who/what is at this setting? | Scene composition, spatial awareness |
| `entities_nearby(entity, max_cost)` | What entities are within spatial range? | Off-screen character reasoning |
| `scenes_at_reachable(from, max_cost)` | What scenes exist at reachable settings? (cross-graph) | Departure evaluation, narrative planning |

### Cascade Policy

The setting topology is the most stable graph — it changes only when world events alter geography (a bridge collapses, a path opens, a settlement is established). Cascade is minimal:

- **Traversal condition changes** (a gate opens or closes) update edge accessibility but don't propagate
- **Entity movement** updates `current_entities` lists but doesn't change topology
- **World events** (rare) may add/remove settings or connections — these cascade to the narrative graph (new scenes become reachable) and the relational web (entities at newly-connected settings can interact)

---

## Graph 4: The Event Dependency DAG

### Domain Concept

The event dependency DAG models how narrative events depend on each other across the full scope of a story. It answers the question: **"Given what has happened and where the player is, what can happen next — and what can no longer happen?"**

This is not the event ledger (which records what *did* happen). The DAG models what *could* happen — the combinatorial space of narrative possibilities — and tracks which possibilities have been realized, which are still open, and which have been foreclosed.

### Vertex Type: Event Condition

```
EventConditionNode {
    node_id: EventConditionId,
    condition_type: ConditionType,
    resolution_condition: TriggerPredicate,  // evaluated against truth set
    resolution_state: ResolutionState,
    narrative_weight: f32,                   // how much narrative mass this carries when resolved
    consequence: Option<ConsequenceSpec>,     // what happens when this resolves
}

ConditionType =
    | AuthoredPrecondition    // "Nancy is pregnant" — leaf node, asserted by designer
    | AtomicEvent             // "Johnny works a night shift" — single event
    | EmergentState           // "Nancy suspects infidelity" — accumulated threshold
    | DiscoveryCondition      // "Nancy learns the truth" — requires scene + action
    | CompoundConsequence     // "Nancy's trust deepens" — fires when all deps met

ResolutionState =
    | Unresolved              // not yet met, may or may not be reachable
    | Resolved(TurnId)        // condition met at this turn
    | Unreachable             // can no longer be met (scene departed, path closed)
    | Orphaned                // unreachable inherited from ancestor

ConsequenceSpec =
    | MandatedShift { entity: EntityRef, dimension: String, delta: f32 }
    | InformationReveal { gate_id: GateId }
    | TruthSetMutation { proposition: Proposition }
    | NarrativeMassAdjustment { scene_id: SceneId, delta: f32 }
    | CompoundConsequence { effects: Vec<ConsequenceSpec> }
```

### Edge Types: Dependency Relationships

```
DependencyEdge {
    from: EventConditionId,    // the dependency (must resolve first)
    to: EventConditionId,      // the dependent (can resolve after)
    dependency_type: DependencyType,
}

DependencyType =
    | Requires        // B cannot resolve until A has resolved
    | Excludes        // A's resolution makes B permanently unreachable
    | Enables         // A's resolution makes B possible (but not guaranteed)
    | Amplifies(f32)  // A's resolution multiplies B's consequence magnitude
```

Acyclicity is non-negotiable — events depend on prior events. Circular dependencies are authoring errors rejected at validation time.

### Query Patterns

| Query | Purpose | Called By |
|---|---|---|
| `evaluable_nodes()` | Nodes whose all Requires dependencies are Resolved | Turn commitment (what to evaluate) |
| `remaining_conditions(node)` | What must still happen for this node to resolve? | Gate proximity, narrative planning |
| `forward_traversal(node)` | What does resolving this node enable/exclude downstream? | Consequence preview, narrative weight |
| `critical_path(target)` | Longest dependency chain to reach a narrative goal | Pacing analysis, narrative planning |
| `unreachable_subgraph()` | Nodes that can no longer resolve | Road-not-taken analysis, fallback triggers |
| `amplification_context(node)` | What Amplifies edges feed into this node? | Consequence magnitude computation |
| `reachability(node)` | Is this node still achievable given current narrative graph state? (cross-graph) | Pruning, scene-exit reachability analysis |

### Cascade Policy: Incremental Propagation

The event DAG cascades incrementally per-turn, at the frontier of newly-resolved nodes:

1. **Match**: Newly committed events are matched against unresolved nodes' resolution conditions
2. **Resolve**: Matching nodes transition from Unresolved to Resolved
3. **Forward propagate**: Downstream nodes whose all Requires dependencies are Resolved become evaluable
4. **Exclusion propagate**: Nodes connected by Excludes edges to newly-resolved nodes become Unreachable; their exclusive descendants become Orphaned
5. **Fire consequences**: Resolved CompoundConsequence nodes fire their effects (mandated shifts, mass adjustments, information reveals)

Scene transitions interact with the DAG:
- **Scene entry**: May assert AuthoredPrecondition nodes as Resolved; may make DiscoveryCondition nodes evaluable (the player is now in a scene where discovery is possible)
- **Scene exit**: May mark DiscoveryCondition nodes as Unreachable if the departed scene was the only context where they could resolve and no return path exists

Reachability analysis (is a node still achievable?) requires cross-graph consultation with the narrative graph — if all paths to the scene where a DiscoveryCondition can resolve are closed, the node is Unreachable.

---

## Cross-Graph Query Patterns

The Storykeeper's most important queries span multiple graphs. These cross-graph operations are what make the four graphs a unified knowledge system rather than four isolated stores.

### Pattern 1: Scene Composition with Relational Context

**Question**: "What characters are at a nearby setting AND have unresolved relational tension?"

**Graphs**: Setting Topology + Relational Web

**Operation**:
```
1. setting_topology.entities_nearby(player_setting, max_cost)
2. For each nearby entity: relational_web.tension_edges(entity, threshold)
3. Rank by: spatial_proximity * relational_tension_magnitude
```

**Purpose**: When the Storykeeper is priming the narrative for a new scene, it needs to know not just who is spatially available but who would be narratively interesting. Spatial proximity without relational context is a location database; relational tension without spatial grounding is abstract analysis. Together, they answer "who should appear next."

### Pattern 2: Departure Evaluation with Relational Weight

**Question**: "Which reachable scenes contain characters I have unresolved debt with?"

**Graphs**: Narrative Graph + Relational Web

**Operation**:
```
1. narrative_graph.reachable_scenes(current_scene)
2. For each reachable scene: scene.required_presences ∪ scene.contingent_presences
3. For each potential cast member: relational_web.edge(player, cast_member).substrate.debt
4. Rank reachable scenes by: gravitational_pull * max_debt_in_cast
```

**Purpose**: The gravitational model says scenes pull the narrative. But what makes a scene pull harder is not just its authored mass — it's the relational weight of the characters the player would encounter there. A scene with moderate mass but a character the player owes a deep debt to may pull harder than a high-mass scene with strangers.

### Pattern 3: Gate Dependencies with Narrative Reachability

**Question**: "Does reaching a specific gate require passing through a scene with conditions I haven't met?"

**Graphs**: Event Dependency DAG + Narrative Graph + Setting Topology

**Operation**:
```
1. event_dag.remaining_conditions(target_gate)
2. For each remaining condition: identify which scene(s) can resolve it
3. narrative_graph.path(current_scene, resolving_scene)
4. For each path: setting_topology.traversal_cost(path_settings)
5. Answer: ordered list of paths-to-gate-resolution with cost and conditions
```

**Purpose**: When the Storykeeper evaluates gate proximity (`StorykeeperQuery::query_gate_proximity`), it needs to know not just "how close is this gate to triggering?" but "is the path to triggering still open?" A gate might be 90% satisfied but the remaining 10% requires visiting a scene the player has bypassed with no return path — that gate is effectively unreachable.

### Pattern 4: Information Boundary with Cascade Path

**Question**: "If character A learns fact F, through what relational paths could that knowledge propagate to character B?"

**Graphs**: Relational Web (cascade paths) + Event Dependency DAG (trigger consequences)

**Operation**:
```
1. relational_web.shortest_path(A, B)
2. For each path: compute permeability chain (per-edge permeability product)
3. For each viable path: compute attenuation (friction_factor^distance)
4. Apply distortion model: what does fact F become by the time it reaches B?
5. event_dag.check: does B learning (distorted version of) F resolve any DAG nodes?
```

**Purpose**: Information transfer is not just "B learns F." It is "B learns a version of F that has been attenuated, distorted, and filtered through the specific relational path it traveled." This matters because the same fact arriving via different paths produces different narrative effects — learning from a trusted friend versus overhearing from a stranger.

### Pattern 5: Narrative Priming with Projection Accuracy

**Question**: "Are any characters approaching a collision between projection and reality?"

**Graphs**: Relational Web (projection accuracy) + Event Dependency DAG (information reveals)

**Operation**:
```
1. relational_web.all_edges_with(projection.accuracy < threshold)
2. For each low-accuracy edge: event_dag.nodes_that_reveal(correcting_information)
3. For each revealing node: event_dag.remaining_conditions(node)
4. Rank by: projection_inaccuracy * proximity_to_revelation
```

**Purpose**: Dramatic irony lives in the gap between projection and reality. When Adam projects that Sarah will fail (accuracy: very_low), and events are converging toward the moment Sarah succeeds, the Storykeeper should detect this approaching collision and bias narrative priming accordingly — this is a high-mass moment in formation.

### Pattern 6: Entity Off-Screen Propagation

**Question**: "How should absent characters' states evolve given what happened in the current scene?"

**Graphs**: Relational Web (cascade to absent entities) + Setting Topology (where they are) + Event DAG (trigger resolution for absent entities)

**Operation**:
```
1. Identify absent entities connected to present entities in relational_web
2. relational_web.cascade_to_absent(committed_events, friction_model) → deferred changes
3. setting_topology.entities_at(absent_entity_setting) → co-located absent entities
4. For co-located absent groups: simulate relational dynamics (simplified cascade)
5. event_dag.evaluate_for_absent(absent_entity, deferred_changes) → off-screen resolutions
```

**Purpose**: The world doesn't stop when the player isn't looking. Beth grieves whether or not Sarah is present. Tom's condition evolves. The Storykeeper uses setting proximity + relational cascade to simulate off-screen evolution at scene boundaries, keeping the world consistent without full simulation.

---

## Traversal Friction Model: Formal Specification

The friction model from `storykeeper-api-contract.md` applies primarily to the relational web but has analogues in every graph. This section formalizes the model across all four graphs.

### Relational Web Friction (Primary)

Fully specified in the Relational Web section above. Summary:

```
effective_signal(source, target, path) = signal_at_source
    * Π(permeability(edge_i) for edge_i in path)
    * friction_factor^path_length
```

With distortion applied at each hop (category shift from specific substrate dimension to generalized history).

### Narrative Graph Friction

Gravitational pull attenuates with narrative distance (not graph hops):

```
gravitational_pull(from, to) = effective_mass(to) / narrative_distance(from, to)^2
```

There is no per-edge permeability in the narrative graph — scenes don't filter gravitational pull. But approach vector satisfaction acts as a proximity modifier: scenes the player is well-prepared for pull harder (lower effective distance).

### Setting Topology Friction

Spatial connections have traversal cost but not information friction. The setting topology is transparent — if a path exists, it exists equally for all entities (unless gated). Friction in the setting topology is **physical**:

```
effective_traversal_cost(path) = Σ(edge.traversal_cost.time + edge.traversal_cost.effort * physical_factor)
```

Entities with different physical capabilities (the Wolf traverses differently than Sarah) may have different effective costs on the same path.

### Event DAG Friction

The event DAG has no friction in the propagation sense — resolution either happens or it doesn't. But the **Amplifies** edge type functions as inverse friction: it increases signal rather than attenuating it. Amplification compounds along dependency chains:

```
consequence_magnitude(node) = base_magnitude(node)
    * Π(amplification_weight(edge) for edge in amplifying_edges_to(node))
```

A discovery that is amplified by three prior events lands harder than one that is amplified by none.

---

## Unified vs. Separate Storage: Technology Considerations

### The Co-Location Question

Should all four graphs live in the same storage engine (Apache AGE, all in one PostgreSQL database) or in separate stores?

**Arguments for co-location (single AGE instance)**:

1. **Cross-graph queries are first-class operations.** Patterns 1-6 above all span multiple graphs. If each graph is in a separate store, these queries require application-level joins — fetching data from one store, then querying another with those results. Co-location allows these to be single Cypher queries or at least single-transaction operations.

2. **Transactional consistency.** When a turn commits, the Storykeeper updates the relational web AND evaluates event DAG nodes AND may adjust narrative mass. If these are in separate stores, maintaining consistency across updates requires distributed coordination. Co-location provides ACID transactions across all graph updates.

3. **Operational simplicity.** One database to provision, monitor, back up, and tune. For a pre-alpha system, this matters.

**Arguments for separation**:

1. **Different access patterns.** The relational web is read-heavy with frequent small writes (edge updates per turn). The narrative graph is rarely written (scene boundaries only). The event DAG is append-heavy (new nodes authored, resolution state updated). The setting topology is almost static. Different access patterns may benefit from different storage configurations.

2. **Query language fit.** Cypher is excellent for graph traversal (shortest path, neighborhood, pattern matching). But the event DAG's incremental propagation may be more naturally expressed as application logic than as Cypher queries. The truth set's temporal index is a B-tree pattern, not a graph pattern.

3. **Evolution independence.** If we later discover that the relational web needs a different storage strategy than the narrative graph, separation makes migration easier.

**Recommendation**: Co-locate in a single AGE instance, with distinct label namespaces per graph. This gives us cross-graph query capability and transactional consistency from the start. The label namespace separation (`:RelEntity`, `:Scene`, `:Setting`, `:EventCondition` for vertices; `:RELATES_TO`, `:TRANSITIONS_TO`, `:CONNECTS_TO`, `:DEPENDS_ON` for edges) keeps the graphs logically distinct while physically co-located.

If access pattern divergence becomes a problem, the Storykeeper trait abstraction insulates the engine: swapping from one-AGE-instance to separate stores is an implementation change behind the same trait surface.

### Technology Evaluation Criteria for TAS-244 (AGE Spike)

The AGE spike (TAS-244) should evaluate Apache AGE against these specific requirements derived from the domain model:

#### Must-Have Capabilities

| Requirement | Source | Evaluation Criterion |
|---|---|---|
| **Directed, labeled edges** | Relational web (asymmetric), Event DAG (typed) | AGE supports directed edges with labels and properties |
| **Property-rich vertices and edges** | All graphs (substrate dimensions, mass, traversal cost) | AGE supports arbitrary JSON properties on nodes and edges |
| **Shortest path computation** | Relational web (social distance), Setting topology (pathfinding) | AGE provides `shortestPath()` or equivalent Cypher function |
| **Variable-depth traversal** | Relational web (cascade at distance N), Narrative graph (N-hop reachability) | AGE supports `*1..N` variable-length path patterns |
| **Cross-label queries** | All cross-graph patterns above | AGE can query across vertex/edge labels in a single Cypher statement |
| **ACID transactions** | Turn commitment (update multiple graphs atomically) | AGE provides transactional guarantees within PostgreSQL |
| **Coexistence with relational data** | Event ledger (append-only log), truth set (B-tree index) | AGE extends PostgreSQL — relational tables and graph data in the same database |

#### Should-Have Capabilities

| Requirement | Source | Evaluation Criterion |
|---|---|---|
| **Aggregation on paths** | Friction model (product of permeability along path) | Can AGE compute aggregate functions along traversal paths? |
| **Weighted shortest path** | Friction model (path with minimum attenuation) | Does AGE support weighted shortest path (Dijkstra)? |
| **Pattern matching** | Structural queries (triangles, clusters) | How expressive is AGE's Cypher pattern matching? |
| **Temporal properties** | Edge versioning (relational history, turn-stamped updates) | Can AGE efficiently store and query temporal edge versions? |
| **Subgraph extraction** | Scene entry (load cast subgraph) | Can AGE efficiently extract a subgraph by vertex set? |

#### Performance Benchmarks

| Benchmark | Expected Scale | Target Latency |
|---|---|---|
| Direct edge lookup (A→B) | ~100 entities, ~500 edges | < 1ms |
| Cast subgraph (N entities, all edges) | N = 4-8 entities | < 5ms |
| N-hop reachable scenes | N = 3, ~50 scenes | < 10ms |
| Shortest path (social distance) | ~100 entities | < 5ms |
| Evaluable DAG nodes (frontier query) | ~200 DAG nodes | < 5ms |
| Cross-graph: reachable scenes with relational context | ~50 scenes, ~100 entities | < 20ms |
| Full scene entry load (all graphs) | All of the above composed | < 50ms |

These benchmarks assume warm caches and represent the latency budget for scene entry operations. During active play, the Storykeeper reads from hot state — graph queries happen at scene boundaries, not on the per-turn critical path.

#### Spike Methodology

The AGE spike should:

1. **Set up** a PostgreSQL + AGE instance with the TFATD dataset: 6 characters with full relational edges, 8 scenes with transitions, setting topology for the Shadowed Wood, and a representative event DAG (10-20 nodes)
2. **Implement** each query pattern from this document as a Cypher query
3. **Measure** latency for each query at TFATD scale AND at projected production scale (100 entities, 200 scenes, 500 DAG nodes)
4. **Evaluate** cross-graph query ergonomics — are they natural in Cypher or do they require awkward workarounds?
5. **Assess** data loading patterns — how does bulk load at scene entry perform? How does incremental update at turn commitment perform?
6. **Document** Cypher coverage gaps — which domain queries can't be expressed as single Cypher statements and require application-level logic?

---

## Graph Identity and Naming Conventions

For implementation clarity, each graph should use distinct label prefixes in the AGE schema:

### Vertex Labels

| Graph | Vertex Label | Example |
|---|---|---|
| Relational Web | `:RelEntity` | `(:RelEntity {entity_id: '...', tier: 'Tracked'})` |
| Narrative Graph | `:Scene` | `(:Scene {scene_id: '...', type: 'Gravitational'})` |
| Setting Topology | `:Setting` | `(:Setting {setting_id: '...', type: 'Exterior'})` |
| Event DAG | `:EventCondition` | `(:EventCondition {node_id: '...', type: 'DiscoveryCondition'})` |

### Edge Labels

| Graph | Edge Label | Example |
|---|---|---|
| Relational Web | `:RELATES_TO` | `(:RelEntity)-[:RELATES_TO {trust_competence: 0.7, ...}]->(:RelEntity)` |
| Narrative Graph | `:TRANSITIONS_TO` | `(:Scene)-[:TRANSITIONS_TO {momentum: 0.8, ...}]->(:Scene)` |
| Setting Topology | `:CONNECTS_TO` | `(:Setting)-[:CONNECTS_TO {time: 2.0, effort: 0.5}]->(:Setting)` |
| Event DAG | `:REQUIRES` | `(:EventCondition)-[:REQUIRES]->(:EventCondition)` |
| Event DAG | `:EXCLUDES` | `(:EventCondition)-[:EXCLUDES]->(:EventCondition)` |
| Event DAG | `:ENABLES` | `(:EventCondition)-[:ENABLES]->(:EventCondition)` |
| Event DAG | `:AMPLIFIES` | `(:EventCondition)-[:AMPLIFIES {weight: 1.5}]->(:EventCondition)` |

### Cross-Graph Links

Vertices in different graphs are connected through shared identifiers rather than cross-graph edges:

| Link | Mechanism | Example |
|---|---|---|
| Scene ↔ Setting | `scene.setting_id` matches `setting.setting_id` | Scene S4 is at Setting "the_rise" |
| Scene ↔ Entity | `scene.required_presences` contains `entity.entity_id` | Scene S1 requires Sarah |
| EventCondition ↔ Scene | Resolution condition references scene context | DiscoveryCondition resolvable in Scene S6 |
| EventCondition ↔ Entity | Resolution condition references entity state | EmergentState tracks Nancy's suspicion |

These are **referential links**, not graph edges. They are resolved by the Storykeeper's query layer — the application joins graphs using shared identifiers rather than traversing cross-graph edges. This keeps each graph's traversal semantics clean while enabling the cross-graph query patterns described above.

---

## Implications for the Storykeeper Crate

This domain model maps directly to the `storyteller-storykeeper` crate modules defined in `storykeeper-crate-architecture.md`:

| Domain Model Concept | Storykeeper Module | Key Operations |
|---|---|---|
| Relational Web | `graph/relational_web.rs` | Edge CRUD, neighborhood, cast subgraph, tension detection |
| Narrative Graph | `graph/narrative.rs` | Reachability, gravitational pull, mass recalculation |
| Setting Topology | `graph/settings.rs` | Adjacency, pathfinding, entity location |
| Event Dependency DAG | `graph/event_dag.rs` | Resolution propagation, evaluable frontier, reachability |
| Traversal Friction | `friction/cascade.rs`, `friction/permeability.rs` | Attenuation, distortion, permeability computation |
| Information Boundaries | `information/boundaries.rs`, `information/gates.rs` | Boundary checks, gate evaluation, revelation tracking |
| Cross-Graph Queries | Composed at the Storykeeper trait level | Methods on `StorykeeperQuery` that internally query multiple modules |

The `InMemoryStorykeeper` implementation will use in-memory data structures (petgraph or custom adjacency structures) for all four graphs. The `PostgresStorykeeper` implementation will use AGE for graph operations and relational tables for the event ledger, truth set, and information boundaries.

The Storykeeper trait surface (`StorykeeperQuery`, `StorykeeperCommit`, `StorykeeperLifecycle`) is identical regardless of which implementation backs it. The cross-graph query patterns are expressed as Storykeeper methods that internally compose queries across modules — the engine never knows whether a cross-graph query was a single Cypher statement or four separate in-memory lookups.

---

## What This Document Does Not Cover

This document deliberately does not specify:

- **PostgreSQL schema** — That is TAS-242, which should derive table/column structure from the domain operations defined here
- **AGE Cypher queries** — That is TAS-244 (spike) and TAS-245 (schema), which should implement the query patterns defined here
- **Rust type definitions** — The types shown are domain model pseudocode, not final Rust structs. The implementation will refine them based on what compiles, what Bevy needs, and what serde can handle
- **Performance optimization** — Caching strategies, query plan tuning, and index design are implementation concerns for the PostgresStorykeeper

---

## Appendix: TFATD as Validation Dataset

The "Fair and the Dead" case study provides a concrete validation dataset for every graph structure:

### Relational Web Validation

6 characters, 10+ directed edges with full substrate dimensions. Key test cases:
- **Asymmetry**: Sarah→Adam (wary dependence) vs Adam→Sarah (instrumental condescension) — same entities, completely different substrate signatures
- **Structural power**: Adam's Gate role emerges from topology, not stored values
- **Information asymmetry**: Kate knows everything about John; John knows almost nothing about Kate
- **Cascade path**: If Sarah's trust in Adam shifts, does the signal reach Tommy (through Kate? through the web?)

### Narrative Graph Validation

8 scenes (6 authored, 2 implied) with gravitational mass ranging 0.3 to 1.0. Key test cases:
- **Mass computation**: S3 (Mother's Prayer) authored base 0.85 + gates + emotional hinge = 0.95
- **Gate dependency**: S6 (Other Bank) depends on S3's water-blessing gate — does the system track this?
- **Approach vector satisfaction**: S6 has three approach vectors with different emotional textures
- **Connective space**: S5 (Crossing a Stream) has lowest mass (0.3) but is essential texture

### Setting Topology Validation

Settings implied by TFATD Part I: Home, Adam's dwelling, the Shadowed Wood (multiple sub-settings), the stream, the abandoned village, the other bank. Key test cases:
- **Traversal cost**: Home → Adam's dwelling is short; Adam's dwelling → the Witch is long and costly
- **Gated connections**: Entering the Shadowed Wood requires S3's threshold crossing
- **Liminal settings**: The stream (S5/S6) is a liminal boundary between Wood regions

### Event DAG Validation

A representative event DAG for Part I:
- **Authored preconditions**: Tommy is ill, Adam serves the Queen, Kate has otherworldly knowledge
- **Discovery conditions**: Sarah learns Adam is a Gate, Sarah discovers her hidden perception (S6)
- **Gate dependencies**: S6's revelation depends on S3's water-blessing gate
- **Exclusion**: If Sarah never enters the Wood (refuses at S2), the entire Wood subgraph becomes Unreachable

This dataset is small enough to fully validate by hand yet rich enough to exercise every query pattern.
