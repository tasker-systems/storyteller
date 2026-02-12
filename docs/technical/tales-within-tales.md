# Tales Within Tales: Sub-Graph Narrative Architecture

## Purpose

This document extends the knowledge graph domain model to account for **nested narrative structures** — stories within stories, memories within journeys, fairy tales within novels. The existing four-graph model (relational web, narrative graph, setting topology, event dependency DAG) operates on the assumption of a single, flat story-graph. This document defines what happens when that graph contains sub-graphs representing distinct narrative layers, each with their own scenes, entities, information boundaries, and gravitational landscapes.

Tales within tales are among the oldest and most common narrative structures: frame stories, embedded memories, dream sequences, alternate timelines, multiple POVs, prophetic visions, characters telling stories to each other. The storyteller system must support these because the authored content we intend to adapt contains them. Vretil, for instance, interleaves three narrative layers: a contemporary mystery, a prophetic epistle sequence, and a fairy tale that mirrors the protagonist's journey. These layers have discrete boundaries, but they also bleed into each other — information from the fairy tale illuminates the mystery; emotional weight from the epistles inflects the protagonist's choices.

The system already has the architectural primitives to support this. Users are decoupled from players, players from character-POVs, scenes have explicit entry/exit lifecycle, and the Storykeeper manages information boundaries. What we lack is a **topological model** for how narrative layers nest, how entities transit between them, how information and consequence cascade across layer boundaries, and how the graph's state evolves temporally as the story unfolds.

### Relationship to Other Documents

- **`knowledge-graph-domain-model.md`** — Defines the four graph structures. This document extends them with sub-graph nesting and temporal evolution.
- **`storykeeper-api-contract.md`** — Defines the Storykeeper's read/write operations. This document identifies new operations required for sub-graph management.
- **`storykeeper-crate-architecture.md`** — Defines where graph operations live. Sub-graph management maps to existing modules.
- **`narrative-graph-case-study-tfatd.md`** — Single-layer narrative graph. This document extends to multi-layer structures.
- **`event-dependency-graph.md`** — Event DAG for the primary narrative. Sub-graphs introduce scoped DAGs with cross-layer dependencies.

---

## The Concept: Narrative Sub-Graphs

### What a Sub-Graph Is

A **narrative sub-graph** is a bounded narrative layer embedded within a parent story-graph. It contains its own scenes, its own entities (some unique, some shared with the parent), its own relational dynamics, and its own gravitational landscape. It is part of the larger story but operates with distinct narrative rules within its boundaries.

A sub-graph is **not**:
- A separate story entirely (that would be a different story-graph)
- A scene within the parent graph (scenes are atomic; sub-graphs contain scenes)
- A branch in the parent narrative (branches diverge from a common path; sub-graphs are structurally parallel layers)

A sub-graph **is**:
- A coherent narrative layer with its own scene structure
- Connected to the parent graph at defined entry and exit points
- Capable of influencing the parent graph's state upon exit
- Optionally re-enterable (a memory can be revisited; a fairy tale can be re-read)

### Types of Sub-Graphs

The structural type of a sub-graph determines its relationship to the parent narrative, its entity sharing model, and its cascade semantics on exit.

| Type | Description | Entity Model | Temporal Relation | Exit Cascade |
|---|---|---|---|---|
| **Frame Story** | A character tells a story; the player plays through it | Discrete entities, with optional projections of parent entities | Past or timeless | Relational: listener-teller dynamics shift. Informational: listener gains knowledge of told events. |
| **Memory Sequence** | A character recalls or relives an experience | Projections of parent entities at an earlier state | Past (specific) | Informational: rememberer's information state updates. Emotional: accumulated emotional weight from reliving. |
| **Dream/Vision** | A surreal or prophetic experience within the narrative | Mix of discrete and distorted projections | Atemporal | Thematic: symbols and emotional residue carry forward. Informational: prophetic content may resolve future gates. |
| **Parallel POV** | The same events seen from a different character's perspective | Shared entities, different information boundaries | Contemporaneous | Informational: the player now has multiple perspectives. Relational: understanding shifts from seeing the other side. |
| **Embedded Text** | A document, fairy tale, or written work within the story | Fully discrete entities (the fairy tale's characters) | Timeless / allegorical | Thematic: pattern recognition, symbolic resonance. May indirectly resolve event DAG conditions. |
| **Alternate Timeline** | A what-if or counterfactual branch explored within the narrative | Projections of parent entities under different conditions | Hypothetical | Informational: player/character awareness of what could have been. Emotional: weight of paths not taken. |

These types are not exclusive — a single sub-graph might be both a Memory Sequence and a Frame Story (a character tells another character about a memory they relive together). The type primarily determines **defaults** for entity sharing, temporal relationship, and cascade semantics. Story designers can override defaults for specific sub-graphs.

### Special Cases

**Degenerate sub-graphs** — A sub-graph need not contain characters, settings, or events. The minimal sub-graph is pure thematic cascade: a narrative layer whose entire purpose is to inflect the parent graph's emotional approach vectors. Vretil's epistles are the canonical example — prophetic poetry with no named characters, no relational web, no setting topology, no event DAG. They contribute only thematic priming and emotional approach vectors. A degenerate sub-graph has:
- No entity manifest (or an empty one)
- No internal relational web
- No internal event DAG
- Cascade-on-exit that is purely thematic/emotional (informational weight ≈ 0)

The system must not reject a sub-graph for lacking entities or events. Degenerate sub-graphs are valid and useful — they represent the narrative device of a chorus, commentary, or atmospheric frame.

**Hidden sub-graphs** — A sub-graph that the player does not perceive as a sub-graph. The player believes they are in the parent narrative, but the system knows (or later reveals) that a layer boundary exists. The unreliable narrator is the primary pattern: a grief-stricken protagonist experiences events as real that the system models as projected. A hidden sub-graph has:
- No visible portal transition (no boundary marker, no register shift)
- Entity sharing that appears fully continuous but is actually projected
- A **revelation event** that reclassifies the hidden sub-graph, making the boundary visible retroactively
- Cascade semantics that apply retroactively when the hidden layer is revealed

Hidden sub-graphs create dramatic irony when the system knows the layer exists before the player does, and dramatic shock when the revelation event collapses the hidden boundary. Mallory's posthumous presence in Vretil chapters 3-10 is a hidden sub-graph — Chris (and the reader) believe she is physically present, but the system models her as a grief-projection layer that is revealed in chapter 10.

```
HiddenSubGraph {
    sub_graph_id: SubGraphId,
    revelation_event: EventConditionId,   // what triggers the reveal
    pre_revelation_appearance: SubGraphType,  // what it looks like before reveal
    post_revelation_type: SubGraphType,       // what it actually is
    retroactive_cascade: RetroactiveCascadePolicy,  // how past scenes are reinterpreted
}
```

---

## Sub-Graph Topology

### Parent-Child Relationship

Sub-graphs nest within a parent graph. The parent may be the root story-graph or itself a sub-graph (enabling recursive nesting: a fairy tale within a memory within the main story). The nesting depth is unbounded in principle but should be shallow in practice — each layer adds cognitive load for both the player and the system.

```
StoryGraph (root)
├── Scene S1 (main narrative)
├── Scene S2 (main narrative)
├── SubGraph: "The Very Long Journey" (fairy tale)
│   ├── Scene FT-1
│   ├── Scene FT-2
│   └── Scene FT-3
├── Scene S3 (main narrative)
├── SubGraph: "Epistle Visions" (prophetic layer)
│   ├── Scene EP-1
│   └── Scene EP-2
├── Scene S4 (main narrative)
└── SubGraph: "Cabin Memory" (memory sequence)
    ├── Scene MEM-1
    └── Scene MEM-2
```

### Entry and Exit Points

Each sub-graph connects to its parent at defined **portals** — scene-boundary-like transitions that move the player from the parent narrative into the sub-graph and back.

```
SubGraphPortal {
    sub_graph_id: SubGraphId,
    portal_type: PortalType,
    parent_scene: SceneId,           // the scene in the parent graph where this portal exists
    entry_conditions: Vec<TriggerPredicate>,  // what must be true to enter
    exit_conditions: Vec<ExitCondition>,      // what determines when/how we exit
    entry_approach_vectors: Vec<ApproachVector>,  // how entry state affects sub-graph experience
    exit_cascade_policy: CascadePolicy,       // how sub-graph state flows back to parent
}

PortalType =
    | Voluntary      // player chooses to enter (reading a book, asking to hear a story)
    | Triggered      // system initiates based on conditions (flashback triggered by location)
    | Structural     // always entered as part of the parent scene's structure (interleaved layers)
    | Gated          // available only after specific conditions are met

ExitCondition =
    | Completion     // sub-graph has a natural endpoint (fairy tale ends)
    | PlayerChoice   // player can choose to exit at defined points
    | Interruption   // parent-graph event forces exit (being woken from a dream)
    | Exhaustion     // sub-graph's scenes have been explored (memory fades)
```

**Structural portals** deserve special attention. In Vretil, the epistles don't appear as a player choice — they are woven into the narrative structure, appearing between chapters as a parallel voice. The system models these as structural portals that fire automatically at defined narrative positions. The player doesn't choose to "enter the epistle sub-graph" — the system transitions them there as part of the narrative rhythm.

### Sub-Graph Identity

Each sub-graph has a unique identity within its parent story-graph:

```
SubGraphDefinition {
    sub_graph_id: SubGraphId,
    parent_graph: SubGraphId,  // root story-graph for top-level sub-graphs
    sub_graph_type: SubGraphType,
    name: String,
    description: String,
    scenes: Vec<SceneId>,
    entity_manifest: EntityManifest,
    entry_portals: Vec<SubGraphPortal>,
    exit_portals: Vec<SubGraphPortal>,
    tonal_signature: Vec<String>,
    thematic_register: Vec<String>,
    temporal_relation: TemporalRelation,
    narrative_voice: Option<NarrativeVoiceOverride>,
    re_enterable: bool,
    visit_history: Vec<SubGraphVisit>,
}

TemporalRelation =
    | Past(Option<ChronologicalAnchor>)   // happened before main narrative
    | Contemporaneous                      // happening at the same time
    | Future(Option<PropheticWeight>)      // prophetic/anticipated
    | Timeless                             // fairy tales, allegories, myths
    | Hypothetical                         // what-if / counterfactual
```

---

## Entity Transit

### The Entity Sharing Model

When the player enters a sub-graph, entities from the parent graph may appear within it — but not necessarily as themselves. The entity sharing model defines three modes:

**1. Shared Entity** — The same entity, fully continuous. Changes to the entity within the sub-graph are changes to the entity in the parent graph. Used for Parallel POV sub-graphs where we're seeing the same characters from a different angle.

```
SharedEntity {
    entity_id: EntityId,  // same ID in both graphs
    sharing_mode: FullyContinuous,
    // state changes propagate bidirectionally in real-time
}
```

**2. Projected Entity** — A representation of a parent entity at a specific state or from a specific perspective. Changes to the projection do NOT automatically propagate to the parent entity. Used for Memory Sequences (a younger version of a character) and Dreams (a distorted version of a character).

```
ProjectedEntity {
    projection_id: EntityId,         // unique ID for this projection
    source_entity: EntityId,         // the parent entity this projects from
    projection_type: ProjectionType,
    state_snapshot: EntityState,     // the parent entity's state at projection time
    divergence: Vec<EntityDelta>,    // how the projection differs from the snapshot
}

ProjectionType =
    | TemporalProjection(TurnId)    // entity as they were at a specific point in time
    | PerceptualProjection(EntityId) // entity as perceived by another entity
    | DistortedProjection(DistortionSpec) // dream/vision distortion
    | AllegoricalProjection(MappingSpec)  // fairy tale character mapped to parent entity
```

**3. Discrete Entity** — An entity that exists only within the sub-graph. No parent-graph counterpart. The fairy tale's protagonist, a dream figure, a character in an embedded text who has no real-world analogue.

```
DiscreteEntity {
    entity_id: EntityId,
    scope: SubGraphId,  // this entity does not exist outside this sub-graph
    // standard entity properties apply within scope
}
```

**4. Bridge Object** — A symbolic entity that exists simultaneously across multiple narrative layers, maintaining identity through symbolic mapping rather than physical continuity. Bridge objects are not characters — they are objects, places, or motifs whose cross-layer presence functions as the primary cascade mechanism between sub-graphs.

```
BridgeObject {
    bridge_id: EntityId,
    name: String,
    manifestations: Vec<BridgeManifestation>,
    cascade_role: BridgeCascadeRole,
}

BridgeManifestation {
    graph_scope: SubGraphId,
    local_form: String,          // how the object manifests in this layer
    local_entity_id: EntityId,   // the entity ID within this layer
    symbolic_weight: f32,        // how strongly this manifestation carries cross-layer meaning
}

BridgeCascadeRole =
    | IdentityBridge    // same object in multiple layers (the cave is one place)
    | SymbolicBridge    // allegorical correspondence (glass flower ↔ bell-blossom ↔ sacred plant)
    | InstrumentBridge  // same functional role across layers (the knife of sacrifice)
```

Bridge objects are distinct from projected entities. A projected entity is a representation *of* a parent entity; a bridge object is the *same symbolic entity* manifesting across layers with potentially different physical forms. The datura flower in Vretil appears as Isabella's glass sculpture (main narrative), a purple bell-blossom (fairy tale), and a sacred plant (epistles) — three manifestations of one symbolic identity.

When a bridge object's state changes in one layer, the change propagates to other layers through symbolic resonance rather than through entity-state synchronization. This is how the datura's meaning accumulates: each layer's encounter adds symbolic weight that inflects the object's significance in all other layers.

### The Entity Manifest

Each sub-graph declares its entity composition:

```
EntityManifest {
    shared: Vec<SharedEntity>,
    projected: Vec<ProjectedEntity>,
    discrete: Vec<DiscreteEntity>,
    bridges: Vec<BridgeObject>,
    player_role: PlayerSubGraphRole,
}

PlayerSubGraphRole =
    | SameCharacter           // player continues as the same character
    | DifferentCharacter(EntityId)  // player inhabits a different character
    | Observer                // player watches without direct agency
    | Narrator                // player has meta-narrative agency (choosing what to tell)
```

The **player role** within a sub-graph is a critical design decision. When reading a fairy tale, the player might be an Observer (watching the story unfold). When reliving a memory, they might inhabit the Same Character at an earlier point. When a character tells a story, the player might temporarily become the Different Character being described. This role determines what agency the player has within the sub-graph's scenes.

### Retroactive Entity Reclassification

An entity's sharing mode is not always fixed at sub-graph creation. A revelation event may require the system to change an entity's classification after the fact — from Shared to Projected, from Projected to Discrete, or any other transition — with cascading implications for every scene in which the entity appeared.

```
EntityReclassification {
    entity_id: EntityId,
    trigger_event: EventId,             // the revelation that causes reclassification
    previous_mode: EntitySharingMode,   // what the entity was classified as
    new_mode: EntitySharingMode,        // what it should now be classified as
    affected_scenes: Vec<SceneId>,      // all scenes containing this entity
    retroactive_policy: RetroactivePolicy,
}

RetroactivePolicy =
    | Reinterpret    // past scenes remain in the ledger but gain a reinterpretation marker
    | Rewrite        // past scenes' entity state is retroactively modified (dangerous)
    | Layer          // a new interpretation layer is added atop the existing record
```

The `Reinterpret` policy is almost always correct. The system does not delete or modify the original scene records — it adds a reinterpretation annotation that downstream queries can use to present the revised understanding. The original record preserves the player's experience (Mallory was present); the annotation preserves the truth (Mallory was a grief-projection).

The `ProvisionalStatus` model (Hypothesized → Rendered → Committed) provides the mechanism: entity appearances that were Rendered during the hidden sub-graph period are retroactively marked as having been Rendered within a grief-projection layer, not the parent narrative layer. The events are not Uncommitted — they happened in the player's experience — but their provenance is reclassified.

This is a Storykeeper operation, not an engine operation. The Storykeeper maintains the authoritative entity records and applies reclassification as a ledger event with full audit trail.

---

## Information Flow Across Sub-Graph Boundaries

### The Boundary as Information Gate

A sub-graph boundary is fundamentally an **information boundary**. What happens within the sub-graph may or may not be knowable to entities in the parent graph. The boundary's permeability is a design choice that varies by sub-graph type and by specific entity.

**Information boundary dimensions**:

| Dimension | Question | Example |
|---|---|---|
| **Experiential** | Did the parent-graph entity experience the sub-graph events directly? | A character who relived a memory experienced it; a character who was told about it did not. |
| **Observational** | Did the parent-graph entity observe the sub-graph events? | A listener observes a told story; a sleeping character does not observe a dreamer's vision. |
| **Inferential** | Can the parent-graph entity reasonably infer sub-graph events? | If a character returns from a memory sequence shaken, others can infer something happened. |
| **Thematic** | Does the sub-graph's thematic content influence the parent entity's understanding? | Reading a fairy tale about sacrifice might deepen a character's understanding of their own sacrifice, without being "information" in the factual sense. |

### Dynamic Boundary Permeability

Sub-graph boundaries are not static walls — their permeability is a **dynamic property** that evolves as the story progresses. A boundary that starts firm (clear separation between narrative layers) may soften over time as events, entity crossings, and symbolic bridges erode the distinction between layers.

```
BoundaryPermeability {
    boundary: (SubGraphId, SubGraphId),    // the pair of layers this boundary separates
    permeability_curve: Vec<(TurnId, f32)>,  // permeability over time (0.0 → 1.0)
    current_value: f32,                       // 0.0 = impermeable, 1.0 = merged
    drivers: Vec<PermeabilityDriver>,         // what causes permeability to change
}

PermeabilityDriver =
    | NarrativeProgression(f32)   // automatic increase as story advances toward convergence
    | EventDriven(EventId)        // specific events that shift permeability (revelation, ritual)
    | EntityCrossing(EntityId)    // entities appearing in both layers increase permeability
    | ObjectBridge(BridgeId)      // symbolic objects existing in both layers
    | RegisterBleed(SceneId)      // narrative voice absorbing characteristics from the other layer
```

**Permeability affects all four cascade dimensions.** At low permeability, only thematic resonance crosses the boundary. As permeability increases, emotional residue begins to leak, then informational content, then relational effects. At permeability 1.0, the boundary dissolves entirely — the sub-graphs merge (see **Convergence Semantics** below).

The permeability curve is typically authored as a set of waypoints with interpolation between them. Events can push permeability beyond the authored curve (a revelation event might spike permeability) or hold it below (a strong boundary marker might resist erosion). The Storykeeper evaluates the current permeability when processing cascade-on-exit and applies it as a multiplier to cascade weights.

### Cascade-on-Exit Semantics

When the player exits a sub-graph, the Storykeeper evaluates what cascades back to the parent graph. This is analogous to the scene-exit cascade described in `knowledge-graph-domain-model.md`, but with additional considerations for the sub-graph boundary.

```
SubGraphExitCascade {
    // What happened in the sub-graph
    sub_graph_events: Vec<CommittedEvent>,
    sub_graph_relational_changes: Vec<RelationalDelta>,
    sub_graph_information_gained: Vec<Proposition>,

    // What cascades to the parent graph
    parent_relational_effects: Vec<RelationalEffect>,
    parent_information_reveals: Vec<InformationReveal>,
    parent_emotional_residue: EmotionalResidue,
    parent_dag_resolutions: Vec<EventConditionResolution>,
    parent_mass_adjustments: Vec<NarrativeMassAdjustment>,
}
```

**Cascade categories**:

**1. Relational cascade** — Relationships between parent-graph entities shift based on sub-graph experiences. If two characters share a memory sequence together, their relational substrate may change (deepened trust from shared vulnerability, or strained trust from confronting a painful past). The cascade policy determines how much of the sub-graph relational evolution propagates:

```
relational_cascade_factor(sub_graph_type) =
    | FrameStory    → 0.3  // hearing a story affects you less than living it
    | MemorySequence → 0.7  // reliving a memory has substantial emotional weight
    | Dream          → 0.2  // dream-state relationships are tenuous
    | ParallelPOV    → 1.0  // same events, full relational propagation
    | EmbeddedText   → 0.1  // reading about relationships has minimal direct effect
    | Alternate       → 0.4  // knowing what could have been has moderate weight
```

**2. Informational cascade** — Facts learned within the sub-graph may become available in the parent graph. The sub-graph type determines the epistemic status of this information:

| Sub-Graph Type | Information Status on Exit | Epistemic Weight |
|---|---|---|
| Frame Story | "I was told that..." | Secondhand — filtered through teller's reliability |
| Memory Sequence | "I remember that..." | Direct but potentially unreliable (memory distortion) |
| Dream/Vision | "I dreamed/saw that..." | Ambiguous — may be prophetic or meaningless |
| Parallel POV | "I know from experience that..." | Direct, high confidence |
| Embedded Text | "I read that..." | Interpreted, thematic rather than factual |
| Alternate Timeline | "It could have been that..." | Hypothetical, no factual authority |

**3. Emotional residue** — Sub-graph experiences leave emotional weight that persists into the parent graph, affecting character frames and narrative tone:

```
EmotionalResidue {
    intensity: f32,           // how strong the emotional carry-over is
    valence: Vec<String>,     // "grief", "wonder", "unease", "tenderness"
    decay_rate: f32,          // how quickly the residue fades in parent-graph turns
    source_sub_graph: SubGraphId,
    affected_entities: Vec<EntityId>,
}
```

**4. Event DAG cascade** — Sub-graph events may resolve conditions in the parent graph's event dependency DAG. A revelation within a memory sequence might satisfy a DiscoveryCondition in the parent DAG. A fairy tale's thematic pattern might Amplify a CompoundConsequence that was already building:

```
sub_graph_dag_cascade(sub_graph_events, parent_dag) =
    for each event in sub_graph_events:
        for each node in parent_dag.evaluable_nodes():
            if node.resolution_condition.matches(event, sub_graph_boundary_filter):
                mark node as Resolved
                propagate downstream
```

The `sub_graph_boundary_filter` applies the epistemic weight — a fact learned in a dream has lower confidence than one experienced directly, and may only partially satisfy a resolution condition.

### Prophetic Cascade: Temporally Forward Flow

The cascade model above describes how sub-graph state flows to the parent graph **on exit** — backward in the structural sense, from child to parent. But some sub-graphs cascade **forward in time**: their events create approach vectors for parent-graph scenes that haven't been reached yet.

This is **prophetic cascade** — the pattern where a sub-graph's events predict, prime, or structurally prefigure events in the parent narrative. The fairy tale's climax (boy sacrifices himself in a cave) doesn't merely add emotional residue to the current scene; it creates an approach vector for the parent graph's cave scene that may be dozens of turns in the future.

```
PropheticCascade {
    source_sub_graph: SubGraphId,
    source_event: EventId,              // the sub-graph event that generates the prophecy
    target_scene: SceneId,              // the parent-graph scene being primed
    prophecy_type: ProphecyType,
    approach_vector_modifier: ApproachVector,  // how the target scene's approach is modified
    fulfillment_condition: Option<EventConditionId>,  // when is the prophecy "fulfilled"?
    decay_rate: f32,                    // how quickly the prophetic weight fades if unfulfilled
}

ProphecyType =
    | StructuralMirror    // sub-graph event structurally mirrors a future parent event
    | SymbolicPrefigure   // sub-graph symbols appear in future parent scenes
    | EmotionalPriming    // sub-graph emotional weight colors future parent scenes
    | InformationalSeed   // sub-graph reveals information whose significance emerges later
```

Prophetic cascade differs from standard cascade in three ways:
1. **Temporal direction** — It flows forward to unreached scenes, not backward to the parent layer
2. **Deferred resolution** — The cascade effect accumulates over time rather than resolving immediately
3. **Amplification** — Multiple prophetic cascades targeting the same scene compound, increasing the scene's narrative mass

The Storykeeper maintains a registry of active prophetic cascades and evaluates them when computing approach vectors for upcoming scenes. A scene with multiple unfulfilled prophetic cascades pointing at it gains mass — the story bends toward it because multiple layers have primed the reader for it.

### Convergence Semantics: When Boundaries Dissolve

Permeability 1.0 is not a gradual continuation of the permeability curve — it is a qualitative state change. When two sub-graphs reach full permeability, they **merge**: the boundary between them ceases to exist, and the system must handle the consequences.

```
SubGraphConvergence {
    converging_graphs: Vec<SubGraphId>,    // which graphs are merging
    convergence_event: EventId,            // the event that triggers full merge
    entity_resolution: Vec<EntityMerge>,   // how cross-layer entities unify
    voice_resolution: NarrativeVoice,      // the merged narrative voice
    graph_resolution: MergedGraphPolicy,   // how the four graphs combine
}

EntityMerge {
    entities: Vec<(SubGraphId, EntityId)>,  // the entity in each converging graph
    merged_entity: EntityId,                // the unified entity post-convergence
    merge_type: EntityMergeType,
}

EntityMergeType =
    | IdentityCollapse   // projections collapse to source (Chris IS the boy)
    | SettingUnification  // sub-graph settings merge with parent (the cave is one place)
    | VoiceIntegration    // narrative voices merge (fairy tale + main + epistle = unified voice)
```

**What convergence means for each graph:**

- **Relational Web**: Cross-layer edges become same-layer edges. The fairy tale's relational dynamics merge into the parent web.
- **Narrative Graph**: Sub-graph scenes become parent-graph scenes. Gravitational mass unifies — the sub-graph's mass adds to the parent scene's mass.
- **Setting Topology**: Sub-graph settings that correspond to parent settings merge. The fairy tale's cave and the main narrative's cave become one node.
- **Event DAG**: Sub-graph event conditions merge into the parent DAG. Cross-layer resolutions become same-layer resolutions.
- **Narrative Stack**: The converging layers are popped and their state is folded into the layer below. The stack flattens.

Convergence is a valid and intentional end-state, not a failure mode. The system should support it explicitly. It represents the narrative device where parallel story layers are revealed to have been aspects of the same story all along — the moment of anagnorisis where structure collapses into unity.

---

## The Temporal Graph: State as a Function of History

### Event Sourcing and Graph Reconstruction

The user described the temporal model as: "if we imagine a 3D modeling of interrelated graphs where distance is grounded in our concepts of narrative gravity, character trust/affection etc, and then we moved a temporal slider back and forth we would see the 3D graph reflect different operative relationships over time."

This is event sourcing applied to graph state. The graph at any point in time is the result of replaying all committed events from the beginning of the story up to that point. The event ledger is the source of truth; the current graph state is a materialized view.

```
GraphState(t) = fold(initial_state, events[0..t], apply_event)

where:
    initial_state = the authored graph (character relationships, scene structure, etc.)
    events[0..t] = all committed events from story start to time t
    apply_event = the function that updates graph state given an event
```

### What Changes Over Time

Each of the four graphs evolves differently:

**Relational Web** — The most dynamic. Substrate dimensions shift turn-by-turn as characters interact, learn, trust, betray. The relational web at turn 50 may look entirely different from turn 1.

```
RelationalEdge(A→B, t) = authored_base(A→B)
    + Σ(relational_delta(event) for event in events[0..t] where event affects A→B)
```

**Narrative Graph** — Changes at scene boundaries. Gravitational mass is recalculated when scenes complete. Activation states shift (Dormant → Reachable → Attracting → Active → Completed). The gravitational landscape reshapes as the player progresses.

```
SceneMass(scene, t) = authored_base_mass(scene)
    + structural_modifier(scene, graph_state(t))
    + dynamic_adjustment(scene, player_state(t))
```

**Setting Topology** — The most stable. Changes only when world events alter geography. But entity positions update as characters move through the world.

**Event Dependency DAG** — Changes incrementally per turn. Resolution states propagate forward (Unresolved → Resolved or Unreachable). The evaluable frontier advances as conditions are met.

### Temporal Query Patterns

The temporal model enables queries that are impossible with a snapshot-only view:

| Query | Purpose | Example |
|---|---|---|
| `graph_state_at(t)` | Reconstruct graph state at a specific turn | "What was Sarah's trust in Adam before scene S4?" |
| `graph_diff(t1, t2)` | What changed between two points | "How did the relational web change during the memory sequence?" |
| `edge_trajectory(A, B, t_range)` | Direction and rate of change for an edge | "Is Chris's trust in Mallory trending up or down over the last 10 turns?" |
| `mass_trajectory(scene, t_range)` | How a scene's gravitational pull has changed | "Is the climactic scene gaining or losing mass?" |
| `entity_trajectory(entity, t_range)` | Entity's full state evolution | "How has this character's emotional frame evolved across the story?" |
| `sub_graph_impact(sub_graph_id)` | What changed in the parent graph as a result of a sub-graph visit | "What was the net relational effect of reliving that memory?" |
| `convergence_analysis(t)` | Are characters/relationships converging or diverging | "Are the siblings growing closer or further apart?" |

### Temporal Indexing

Efficient temporal queries require indexing by time. Two strategies:

**1. Event-based reconstruction** — Store only events and reconstruct state by replay. Exact but potentially slow for deep history. Suitable for the `InMemoryStorykeeper` where the full event log is in memory.

**2. Checkpoint-based interpolation** — Store periodic snapshots (at scene boundaries, every N turns) and reconstruct by loading the nearest checkpoint and replaying the delta. Faster for point-in-time queries. Suitable for `PostgresStorykeeper` where reconstruction from genesis would require loading all historical events.

The checkpoint model already exists in the architecture (`storykeeper-crate-architecture.md`, `checkpoint/`). The temporal index extends it by adding **graph-state snapshots** at checkpoint boundaries:

```
TemporalCheckpoint {
    checkpoint_id: CheckpointId,
    turn_id: TurnId,
    scene_id: SceneId,
    relational_web_snapshot: RelationalWebState,
    narrative_graph_snapshot: NarrativeGraphState,
    entity_positions: EntityLocationMap,
    dag_resolution_state: DagResolutionSnapshot,
    truth_set_snapshot: TruthSetState,
    active_sub_graphs: Vec<SubGraphState>,
}
```

### The Temporal Slider Visualization

The "3D model with a temporal slider" is a conceptual tool for understanding story evolution, but it maps to concrete operations:

1. **The graph at time t** — A spatial layout where nodes are entities/scenes and edges are relationships/transitions. Distance reflects relational substrate (closer = higher trust/affection), narrative gravity (closer = stronger pull), or spatial proximity (closer = physically nearer).

2. **Moving the slider forward** — Each committed event modifies the graph. Edges lengthen or shorten (trust shifting), nodes gain or lose mass (scene gravity changing), entities appear or disappear (promotion/demotion), and the overall topology deforms.

3. **Sub-graph overlay** — When the player enters a sub-graph, the visualization shows a new layer appearing alongside the parent graph, with portal connections linking them. Within the sub-graph, its own dynamics play out. On exit, the parent graph visibly adjusts in response.

4. **Convergence and divergence** — Over time, certain entity pairs converge (their edges strengthen, they draw closer) while others diverge. The trajectory of convergence/divergence is often the emotional arc of the story.

---

## Implications for the Four Graphs

### Relational Web with Sub-Graphs

Each sub-graph maintains its own relational web for entities within it. Shared entities have edges in both the parent and sub-graph webs simultaneously. Projected entities have edges only in the sub-graph web, but their state is derived from the parent entity at projection time.

**Key considerations**:
- Relational edges within a sub-graph are scoped to that sub-graph. Trust between fairy tale characters does not appear in the parent relational web.
- On sub-graph exit, the cascade policy determines which sub-graph relational changes propagate to parent edges.
- For Memory Sequences, the sub-graph relational web may be a reconstruction of historical relationships — "what was A→B like back then?"

### Narrative Graph with Sub-Graphs

Each sub-graph is effectively a **nested narrative graph** — a gravitational landscape within a landscape. The parent graph's scenes connect to sub-graph portals, and within the sub-graph, scenes have their own mass and transition structure.

**Key considerations**:
- Sub-graph scenes participate in the parent graph's gravitational model but at reduced weight (a fairy tale scene does not pull as hard as a main narrative scene, except thematically)
- The sub-graph itself has a collective mass in the parent narrative graph — "the fairy tale as a whole" has gravitational weight
- Completing a sub-graph may adjust the mass of parent-graph scenes (the knowledge gained from a memory might make a previously low-mass scene suddenly critical)

```
SubGraphCollectiveMass {
    sub_graph_id: SubGraphId,
    base_mass: f32,                  // authored importance of the sub-graph as a unit
    completion_bonus: f32,           // mass gained by completing the sub-graph
    thematic_resonance: f32,         // how much the sub-graph's themes amplify parent scenes
    affected_parent_scenes: Vec<(SceneId, f32)>,  // which parent scenes gain mass from this sub-graph
}
```

### Setting Topology with Sub-Graphs

Sub-graphs may have their own setting topology that is **disconnected** from the parent setting topology. The fairy tale's forest is not the same as the main narrative's forest, even if they share thematic resonance. Memory sequences reconstruct historical settings that may no longer exist in the present-day parent graph.

**Key considerations**:
- Sub-graph settings are scoped to the sub-graph (they don't appear in `setting_topology.adjacent()` queries from the parent)
- Abstract settings (dreams, visions) have no spatial relationship to parent settings
- Portal transitions are not spatial — entering a fairy tale sub-graph doesn't mean the character physically moves to the fairy tale's setting. The transition is narrative, not geographic.

The one exception is **Parallel POV** sub-graphs, where the settings ARE the parent settings seen from a different perspective. In this case, the sub-graph shares the parent's setting topology.

### Event Dependency DAG with Sub-Graphs

Sub-graphs may have their own event DAGs for internal progression (the fairy tale has its own quest conditions). More importantly, sub-graph events may resolve conditions in the parent DAG.

**Key considerations**:
- Sub-graph DAGs are scoped — fairy tale conditions don't appear in the parent evaluable frontier
- Cross-layer resolution uses the epistemic weight model: a discovery in a dream partially satisfies a parent condition; a discovery in lived memory fully satisfies it
- Sub-graph completion may be itself a condition in the parent DAG ("the player has heard the full fairy tale" enables a later revelation)

```
CrossLayerResolution {
    sub_graph_event: EventId,
    parent_condition: EventConditionId,
    epistemic_weight: f32,         // how much this counts toward satisfaction
    resolution_type: CrossLayerResolutionType,
}

CrossLayerResolutionType =
    | Direct         // sub-graph event directly satisfies parent condition
    | Thematic       // sub-graph's themes amplify a parent condition
    | Enabling       // sub-graph completion enables (but doesn't satisfy) parent condition
    | Informational  // sub-graph reveals information needed for parent condition
```

---

## Implications for the Storykeeper

### New Domain Operations

The Storykeeper's trait surface needs extensions for sub-graph management:

**StorykeeperQuery extensions**:

| Operation | Purpose | Inputs | Outputs |
|---|---|---|---|
| `query_active_sub_graphs` | What sub-graphs is the player currently within? | Session context | Stack of active sub-graph IDs with nesting depth |
| `query_sub_graph_portals` | What sub-graphs are available from the current scene? | Scene ID, player state | Available portals with entry conditions and approach vectors |
| `query_sub_graph_state` | What is the current state of a sub-graph? | Sub-graph ID | Visit history, completion state, entity states within |
| `query_cross_layer_gates` | What parent-graph gates could be affected by sub-graph events? | Sub-graph ID, parent DAG state | Gates with potential cross-layer resolution paths |
| `query_temporal_graph_state` | Reconstruct graph state at a historical point | Turn ID or checkpoint ID | Full graph state snapshot |
| `query_graph_diff` | What changed between two temporal points? | Turn range | Relational deltas, mass changes, DAG resolutions |
| `query_edge_trajectory` | Direction and rate of change for a relational edge | Entity pair, turn range | Time series of substrate values |
| `query_boundary_permeability` | Current permeability between two layers | Sub-graph pair | Current permeability value, active drivers, curve position |
| `query_active_prophecies` | Unfulfilled prophetic cascades targeting a scene | Scene ID | Active prophetic cascades with source events and weights |
| `query_bridge_objects` | Bridge objects active across specified layers | Sub-graph IDs | Bridge objects with their manifestations per layer |

**StorykeeperCommit extensions**:

| Operation | Purpose | Inputs | Outputs |
|---|---|---|---|
| `commit_sub_graph_entry` | Record entry into a sub-graph | Sub-graph ID, entry portal, player state | Sub-graph scene load result |
| `commit_sub_graph_exit` | Process exit from a sub-graph with cascade | Sub-graph ID, exit conditions, accumulated state | Cascade result (parent graph effects) |
| `commit_sub_graph_turn` | Commit a turn within a sub-graph | Completed turn, sub-graph context | Sub-graph commit result |
| `commit_permeability_shift` | Record a boundary permeability change | Sub-graph pair, new value, driver | Updated permeability state |
| `commit_entity_reclassification` | Reclassify an entity's sharing mode retroactively | Entity ID, new mode, retroactive policy | Reclassification result with affected scenes |
| `commit_prophetic_cascade` | Register a prophetic cascade from sub-graph to parent scene | Source event, target scene, prophecy type | Registered prophecy with approach vector modifier |

**StorykeeperLifecycle extensions**:

| Operation | Purpose | Inputs | Outputs |
|---|---|---|---|
| `enter_sub_graph` | Full sub-graph entry pipeline (load entities, relational context, set up scenes) | Sub-graph ID, portal, session context | Sub-graph load result |
| `exit_sub_graph` | Full sub-graph exit pipeline (cascade, checkpoint, restore parent state) | Sub-graph context, exit conditions | Exit cascade result |
| `converge_sub_graphs` | Merge sub-graphs when permeability reaches 1.0 | Converging graph IDs, convergence event | Convergence result (merged entities, unified graph state) |
| `reveal_hidden_sub_graph` | Process revelation of a hidden sub-graph | Hidden sub-graph ID, revelation event | Reclassification cascade result |

### Sub-Graph as Scoped Storykeeper

Conceptually, each sub-graph has its own "mini-Storykeeper" — a scoped domain authority that manages the sub-graph's entities, relationships, and events. In implementation, this is the same Storykeeper instance with sub-graph-scoped queries:

```
// Scoping mechanism
storykeeper.with_scope(sub_graph_id).query_relational_context(...)
storykeeper.with_scope(sub_graph_id).commit_turn(...)

// Equivalent to:
storykeeper.query_relational_context(..., scope: Some(sub_graph_id))
```

The scoping ensures that queries within a sub-graph return sub-graph-scoped results (the fairy tale's relational web, not the parent's), while cross-layer queries explicitly span scopes.

---

## Graph Addressing: Where Am I?

### The Narrative Stack

At any point during play, the player exists at a specific position in a potentially nested narrative structure. This position is a **stack** — the root story-graph at the bottom, with any active sub-graphs layered above:

```
NarrativeStack {
    layers: Vec<NarrativeLayer>,
    // layers[0] = root story-graph
    // layers[n] = innermost active sub-graph
}

NarrativeLayer {
    graph_id: SubGraphId,    // root or sub-graph ID
    scene_id: SceneId,       // current scene in this layer
    entry_portal: Option<SubGraphPortal>,  // how we entered (None for root)
    accumulated_state: LayerAccumulatedState,  // what's happened in this layer
}
```

When the player enters a sub-graph, a new layer is pushed onto the stack. When they exit, the top layer is popped and its accumulated state cascades to the layer below. The stack enables:

- **Context-aware queries**: "What scene am I in?" answers differently at each layer
- **Proper scoping**: Entity queries check the current layer's scope first
- **Correct cascade on exit**: Only the top layer's state cascades to the layer below, not to all layers
- **Navigation**: The player can potentially exit multiple layers at once (waking from a dream within a memory)

### Entity Resolution with the Stack

When the Storykeeper resolves an entity reference, it walks the narrative stack:

1. Check the current (topmost) layer for a sub-graph-scoped entity
2. If not found, check for a projected entity from the parent layer
3. If not found, check the parent layer for a shared entity
4. Continue up the stack until the root story-graph

This resolution order means sub-graph entities shadow parent entities of the same conceptual role. The fairy tale's "wolf" is resolved within the fairy tale layer; it doesn't accidentally reference the parent narrative's wolf entity.

---

## The Ledger of Graph Evolution

### Every Edge Has a History

The temporal model requires that every relational edge, every mass value, every resolution state is ledgerable — its full history of changes is reconstructable from the event log.

For the relational web, this means each edge update is an event:

```
RelationalEdgeUpdate {
    event_id: EventId,
    turn_id: TurnId,
    scene_id: SceneId,
    graph_scope: SubGraphId,         // which graph layer this occurred in
    source_entity: EntityId,
    target_entity: EntityId,
    dimension: SubstrateDimension,
    previous_value: f32,
    new_value: f32,
    cause: EdgeUpdateCause,          // which committed event triggered this
}

EdgeUpdateCause =
    | DirectInteraction(EventId)     // entities interacted directly
    | CascadePropagation(CascadeId)  // cascaded from another edge change
    | SubGraphExit(SubGraphId)       // cascaded from sub-graph exit
    | SceneBoundary(SceneId)         // recalculated at scene boundary
    | Authored(String)               // story designer adjustment
```

For the narrative graph, mass changes are similarly ledgered:

```
MassChangeEvent {
    event_id: EventId,
    turn_id: TurnId,
    scene_id: SceneId,
    target_scene: SceneId,
    mass_component: MassComponent,   // authored_base, structural, dynamic
    previous_value: f32,
    new_value: f32,
    cause: MassChangeCause,
}
```

### The Snapshot-Delta Model

For efficient temporal queries, the Storykeeper maintains:

1. **Full snapshots** at major boundaries (scene entry, scene exit, sub-graph entry/exit)
2. **Deltas** between snapshots (individual edge updates, mass changes, resolution events)

To reconstruct state at any point:
```
state_at(t) = nearest_snapshot_before(t) + apply_deltas(snapshot.t, t)
```

This is the same checkpoint + ledger replay pattern described in `storykeeper-crate-architecture.md`, extended to cover all four graphs and sub-graph state.

---

## Design Principles

### 1. Sub-Graphs Are Narrative, Not Technical

A sub-graph is a narrative structure, not a database partition. The decision to create a sub-graph is a story design choice: "this fairy tale is a distinct narrative layer." The implementation details (scoped queries, entity manifests, cascade policies) are invisible to the player and transparent to the story designer.

### 2. Boundaries Are Permeable by Design

Sub-graph boundaries are not firewalls. Information, emotion, and thematic resonance flow across them — attenuated and distorted by the boundary's properties, but flowing. A completely impermeable boundary would make the sub-graph narratively inert. The art is in calibrating permeability.

### 3. The Temporal Model Is a Consequence of Event Sourcing

We don't build temporal queries as a separate system. They emerge naturally from our existing event sourcing architecture. Every committed event already carries a turn ID and scene ID. Graph state at any point in time is reconstructable because we never mutate state — we append events and compute state from them.

### 4. Sub-Graph Scope Does Not Mean Sub-Graph Isolation

Entities within a sub-graph can be influenced by parent-graph state (a memory sequence might be colored by the rememberer's current emotional state). And sub-graph events can influence the parent graph (a revelation in a dream might resolve a waking-world gate). The scope provides namespace isolation for queries, not semantic isolation for narrative.

### 5. The Stack Model Prevents Confusion

At any moment, the system knows exactly where the player is in the narrative hierarchy. There is no ambiguity about which layer's entities, relationships, and events are active. The stack makes nesting explicit rather than implicit.

### 6. Convergence Is a Valid End-State

When sub-graph boundaries dissolve completely (permeability 1.0), the system treats this as a meaningful narrative event — not a model failure. Convergence represents the storytelling device where parallel layers are revealed to have been aspects of the same story. The system supports convergence through explicit entity merges, graph unification, and stack flattening.

### 7. The System Knows More Than the Player

Hidden sub-graphs, prophetic cascades, and entity reclassification all depend on the system maintaining knowledge that the player does not yet have. The Storykeeper's information boundary model is not just about filtering what characters know — it includes filtering what the *player* knows about the narrative structure itself. The player may not know they are in a sub-graph until a revelation event makes the boundary visible.

### 8. Boundaries Are Dynamic, Not Configured

Sub-graph permeability is a function of narrative state, not a static property set at authoring time. Events, entity crossings, symbolic bridges, and narrative progression all drive permeability changes. The authored permeability curve provides waypoints; the actual permeability at any moment is computed from the curve plus accumulated drivers.

---

## What This Document Does Not Cover

This document deliberately does not specify:

- **Cypher/AGE representation of sub-graphs** — That is an implementation concern for the PostgresStorykeeper and the AGE spike.
- **Rust type definitions** — The types shown are domain model pseudocode. Implementation will refine based on what compiles and what Bevy needs.
- **Authoring workflow for sub-graphs** — How story designers define sub-graph structures, entity manifests, and cascade policies. That is a scene authoring concern.
- **Rendering sub-graph transitions** — How the Narrator presents the shift from one narrative layer to another. That is a Narrator voice/style concern, though it depends on `NarrativeVoiceOverride`.
- **Performance implications** — Checkpoint frequency, snapshot storage costs, reconstruction latency for deep history. Implementation concerns for the PostgresStorykeeper.

---

## Appendix: The Vretil Case Study

A companion document (`vretil-case-study.md`) applies this framework to Vretil, mapping the novel's three narrative layers (main narrative, epistles, fairy tale) as concrete sub-graphs with entity manifests, portal definitions, cascade policies, and a realized graph specification.

The Vretil case study serves the same role for the tales-within-tales model that the TFATD case study serves for the single-layer narrative graph: it forces the abstractions into concrete structures and reveals where the model needs engineering. The extensions in this document — dynamic permeability, prophetic cascade, bridge objects, degenerate sub-graphs, convergence semantics, hidden sub-graphs, and retroactive entity reclassification — were all identified through the Vretil analysis and subsequently folded back into this specification.
