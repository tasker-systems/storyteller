# Event Dependency Graph

## Purpose

This document describes the **event dependency graph** — the DAG (directed acyclic graph) that models how events depend on, enable, exclude, and compose with each other across the full scope of a narrative. It addresses a structural problem that becomes apparent when you try to express narrative triggers as queries against the event ledger: combinatorial event patterns — events that depend on the resolution of other events, which themselves depend on player choices that may never occur — cannot be cleanly modeled as predicate trees, recursive CTEs, or nested boolean expressions. They are a graph.

This document is not an implementation plan. It captures a design insight that should shape our thinking as the event system, the ML classifier pipeline, and the eventual story designer authoring tools develop. It establishes the concept and its relationships to existing architecture without committing to specific data structures or APIs.

### Relationship to Other Documents

- **`event-system.md`** describes the two-track classification pipeline, the truth set, and the event ledger. The event dependency graph is a structure *over* the ledger — it models relationships between events, not the events themselves.
- **`tensor-schema-spec.md`** (Decision 5, §Trigger Matching) defines the set-theoretic trigger system: TriggerCondition, TriggerPredicate (All/Any/Not/Difference/Threshold), TemporalPredicate (After/Sequence/Since). The DAG is complementary — it models event *dependencies*, not event *matching*. See "Relationship to the Set-Theoretic Trigger System" below.
- **`event-grammar.md`** (ticket-specs) defines EventAtom, CompoundEvent, and CompositionType. CompoundEvents model within-turn or adjacent-turn composition. The DAG operates at a higher level — cross-scene, cross-arc composition.
- **`scene-model.md`** defines scenes as the unit of play with entry/exit conditions. The DAG connects to scene lifecycle through scene entry assertions and scene exit consequences.
- **`narrative-graph-case-study-tfatd.md`** defines the gravitational landscape of scenes. The DAG is the event-level counterpart — where the narrative graph models *where* the story can go, the event dependency graph models *what can happen* given where it has been.

---

## The Problem: Combinatorial Events

Consider this narrative trigger, authored by a story designer:

> When Nancy discovers that Johnny has been lying to her — as she suspected all along, but not for the reasons she thought (he is not stepping out on her; he has been working second shifts to make money for the new baby) — she falls deeper in love with him and swears she will trust him forever.

This trigger involves:

1. **A precondition**: Nancy is pregnant (authored, may be asserted on scene entry or established in a prior scene).
2. **An emergent state**: Nancy suspects Johnny of infidelity (built up through relational dynamics across multiple turns, possibly multiple scenes — suspicion as accumulated topsoil emotional state).
3. **A factual event**: Johnny has been working extra shifts (may be authored as a given, or may depend on Johnny-as-NPC's resolved actions in prior scenes).
4. **A discovery event**: Nancy learns the truth about the extra shifts (depends on scene affordances, player action, information boundary changes — cannot happen if the player never enters a scene where this information is accessible, or if the player-as-Nancy doesn't pursue it).
5. **A compound consequence**: Nancy's trust deepens at the sediment level, her emotional state shifts, and the relational substrate between Nancy and Johnny transforms.

Each of these is an event condition. They depend on each other: (4) requires (3) and (2); (5) requires (4) and (1). Some are authored givens, some are emergent from play, some may never occur. If the player takes a different path through the narrative graph, (4) may become unreachable — and with it, (5) and everything downstream.

### Why Predicate Trees Don't Work

The natural instinct is to model this as a nested boolean predicate:

```
All(
  nancy_pregnant,
  nancy_suspects_infidelity,
  johnny_working_extra_shifts,
  nancy_discovers_truth
) → trigger_consequence
```

This looks clean until you consider:

- **Temporal ordering matters.** Nancy must suspect *before* she discovers the truth. The discovery's narrative weight depends on the suspicion preceding it. `All()` doesn't capture this.
- **Reachability is structural, not evaluative.** If the player bypassed the scene where discovery is possible, the trigger isn't "false" — it's *unreachable*. The system shouldn't keep evaluating it; it should know it can never fire. Predicate evaluation can't distinguish "not yet true" from "can never be true."
- **Intermediate events have their own dependencies.** "Nancy suspects infidelity" is itself a compound condition — it depends on accumulated relational signals, Johnny's absences, perhaps a specific conversation. These sub-conditions have their own dependency structure. Nesting predicates to express this produces fragile, opaque trees that are difficult to author and debug.
- **Partial resolution is meaningful.** If Nancy suspects Johnny but never discovers the truth, that unresolved suspicion is narratively significant — it could feed into a different trigger (bitterness, withdrawal, confrontation). A predicate tree evaluates to true or false; it doesn't model the narrative significance of partial resolution paths.

### Why Recursive CTEs Don't Work

The instinct after predicates fail is to reach for SQL. The event ledger is in PostgreSQL; surely a recursive CTE can walk the dependency chain:

```sql
WITH RECURSIVE event_chain AS (
  SELECT id, kind, resolved FROM events WHERE ...
  UNION ALL
  SELECT e.id, e.kind, e.resolved
  FROM events e
  JOIN event_dependencies d ON d.depends_on = e.id
  JOIN event_chain ec ON ec.id = d.event_id
  WHERE ...
)
SELECT ...
```

This becomes implausibly complex when dependencies include:

- **Exclusion edges** (event A makes event B impossible — not just "not yet resolved" but "permanently unreachable").
- **Alternative paths** (event B can be reached via A₁ OR A₂, and the path taken affects B's character).
- **Cross-graph references** (event B depends on the player being in a specific region of the narrative graph AND a relational condition holding — mixing event, scene, and relational data in a single CTE).
- **Graceful degradation** (if B is unreachable, what downstream nodes should be pruned? what alternative triggers should be considered?).

The CTE approach also conflates the dependency structure (which is relatively static — it changes when new authored content is added) with the evaluation state (which is dynamic — it changes every turn). You end up re-traversing the entire dependency chain on every evaluation, which is both expensive and conceptually muddled.

The intuition that this "feels bad" as SQL is correct. It feels bad because the data model is wrong. The dependencies between events are not rows in a table with foreign keys — they are edges in a graph.

---

## The Insight: Events as a Directed Acyclic Graph

The simplest and most accurate model for event dependencies is a DAG. Nodes are event conditions. Edges are dependency relationships. Acyclicity is guaranteed by time — an event cannot depend on something that depends on it, because events unfold sequentially in the narrative.

### What a Node Is

A node in the event dependency graph is an **event condition** — a description of something that can happen, has happened, or can no longer happen. Nodes exist at multiple granularities:

- **Authored preconditions**: "Nancy is pregnant." Asserted by the story designer, resolved on scene entry or narrative setup. These are leaf nodes — they have no incoming dependency edges.
- **Atomic event conditions**: "Johnny works a night shift." A single event that either has or hasn't occurred, matchable against the event ledger using the set-theoretic trigger language.
- **Emergent state conditions**: "Nancy suspects infidelity." Not a single event but an accumulated state — the topsoil emotional result of many small events across turns and scenes. Resolved when the state reaches a threshold or when an ML interpretive judgment confirms it.
- **Discovery/interaction conditions**: "Nancy learns the truth." Depends on scene affordances, player action, and information boundary state. May have multiple paths to resolution (multiple scenes where this discovery is possible).
- **Compound consequence nodes**: "Nancy's trust deepens at sediment level." The narrative payoff — resolved when all its dependencies are met. May trigger mandated shifts, echo activations, or relational substrate transformations.

Nodes have a **resolution state**:

- **Unresolved**: The condition has not been met. It may or may not be reachable.
- **Resolved**: The condition has been met. Downstream nodes can now be evaluated.
- **Unreachable**: The condition can no longer be met — the player has passed the point where it was possible, or a predecessor's resolution has excluded it.
- **Orphaned**: A node whose unreachability was inherited from an ancestor. Distinguished from directly-unreachable because orphaning is transitive and automatic — when a node becomes unreachable, all its exclusive descendants are orphaned.

### What an Edge Is

An edge in the DAG represents a dependency relationship between event conditions. Edges are directed: they point from the dependency to the dependent (A → B means "B depends on A").

Several edge types are needed:

- **Requires**: B cannot resolve until A has resolved. The basic temporal dependency. "Nancy discovers truth" requires "Johnny has been working extra shifts."
- **Excludes**: A's resolution makes B permanently unreachable. "Player chooses to leave town" excludes "Player discovers the hidden garden" (if the garden is only in this town).
- **Enables**: A's resolution makes B *possible* but does not guarantee it. B still requires player action or further events. "Nancy enters the scene with the ledger" enables "Nancy discovers the financial records."
- **Amplifies**: A's resolution increases the narrative weight or consequence magnitude of B, but B can resolve without A. "Nancy has suspected Johnny for weeks" amplifies "Nancy discovers the truth" — the discovery hits harder because of the accumulated suspicion, but the discovery could happen without the suspicion (with less narrative impact). Amplification edges carry a weight multiplier.

### Why Acyclic

Events happen in time. A cannot depend on B if B depends on A — that would require time travel. The narrative may contain thematic cycles (echoes, rhymes, callbacks), but the *dependency structure* is strictly acyclic. A character can revisit a location, but the event "character arrives at location the second time" is a different node from "character arrives the first time," and the second depends on the first having occurred.

This acyclicity is what makes the DAG tractable. Topological sort gives evaluation order. Reachability analysis is efficient. Pruning propagates cleanly.

---

## The Four Graphs

The storyteller system now has four distinct graph structures, each modeling a different aspect of the narrative:

### 1. The Narrative Graph

**What it models**: The landscape of scenes and their connections — where the story *can go*.

**Nodes**: Scenes (gravitational, connective, gate, threshold).

**Edges**: Scene transitions with approach vectors (state predicates) and departure trajectories.

**Defined in**: `narrative_graph.md`, `narrative-graph-case-study-tfatd.md`, `scene-model.md`.

### 2. The Relational Web

**What it models**: Relationships between entities — how characters, presences, conditions, and props relate to each other.

**Nodes**: Entities (anything with an `EntityId`).

**Edges**: Directed relational edges carrying five substrate dimensions (trust, affection, debt, history, projection) plus information state and configuration annotation.

**Defined in**: `relational-web-tfatd.md`, `entity-model.md`.

### 3. The Event Ledger

**What it models**: What has happened — the temporal sequence of committed events.

**Structure**: Append-only log (not a graph per se, but queryable by time, scene, turn, entity, and event kind).

**Defined in**: `event-system.md`, `tensor-schema-spec.md`.

### 4. The Event Dependency Graph (this document)

**What it models**: How events depend on each other — what *can happen* given what *has happened* and where the player *is* in the narrative.

**Nodes**: Event conditions at various granularities (preconditions, atomic events, emergent states, compound consequences).

**Edges**: Dependency relationships (requires, excludes, enables, amplifies).

**Relates to the other three**: The event dependency graph is the connective tissue. A node's resolution condition may reference the event ledger ("has event X occurred?"), the relational web ("does the trust between A and B exceed threshold?"), and the narrative graph ("is the player in a scene where this is possible?"). The DAG is the structure that *composes* these references into dependency chains.

---

## Relationship to the Set-Theoretic Trigger System

The set-theoretic trigger system (tensor-schema-spec.md, Decision 5) and the event dependency graph are complementary, not competing. They operate at different levels:

**The trigger system** answers: "Given the current truth set, does this condition match?" It evaluates predicates — `All(CharacterPresent(wolf), TrustAbove(sarah, adam, 0.7))` — against the current state. It is *synchronous* (evaluated at frame computation time or on event receipt) and *stateless* (each evaluation is independent).

**The event dependency graph** answers: "Given the history of play, is this event condition reachable, and have its dependencies been met?" It tracks resolution state across the lifetime of the narrative. It is *persistent* (state evolves over the course of play) and *structural* (the graph itself is part of the authored content).

They compose naturally: a DAG node's resolution condition is expressed *using* the trigger system's predicate language. "Nancy suspects infidelity" might be a node whose resolution condition is `Threshold(0.7, [AccumulatedStress(nancy, jealousy), RelationshipState(nancy, johnny, trust, < 0.4), EventOccurred(johnny_absent_pattern)])`. The trigger system evaluates the predicate; the DAG tracks whether the node has resolved, what that resolution enables or excludes, and whether downstream nodes are now evaluable.

In other words: the trigger system is the *evaluation engine* for individual conditions. The DAG is the *dependency structure* that determines which conditions matter and in what order.

---

## Relationship to CompoundEvent (Event Grammar)

The event grammar's `CompoundEvent` and `CompositionType` (causal, temporal, conditional, thematic) model composition at the **turn level** — atoms within a single turn or adjacent turns that together carry more narrative weight than individually. "She saw the chipped cup and cried" is two atoms with a causal composition detected within one turn.

The event dependency graph models composition at the **arc level** — event conditions that span scenes, spans of turns, and narrative phases. "Across three scenes, Nancy's suspicion grew until she discovered the truth" is a DAG subgraph, not a CompoundEvent.

These are different granularities of the same underlying concern: how events compose to produce meaning. The relationship is hierarchical:

- A `CompoundEvent` within a single turn might resolve a DAG node (the causal composition "she discovered and was shocked" resolves the "discovery" node).
- A DAG node's resolution condition might require detecting a specific `CompositionType` (the "discovery" node requires a causal event where the actor is Nancy and the information content matches the secret).
- The DAG's `Amplifies` edge type is the arc-level analogue of the event grammar's emergent weight — both capture the idea that context increases meaning, but at different timescales.

---

## Evaluation Model

### Incremental Propagation

The DAG is not re-evaluated from scratch on every turn. When events are extracted from a committed turn:

1. **Match**: Newly extracted events and interpretive judgments are matched against unresolved DAG nodes' resolution conditions (using the trigger system).
2. **Resolve**: Any nodes whose conditions are now met transition from Unresolved to Resolved.
3. **Propagate forward**: For each newly resolved node, check its outgoing edges. Downstream nodes whose *all* Requires dependencies are now Resolved become *evaluable* (their resolution conditions begin being checked). Downstream nodes connected by Enables edges become *possible* (they were previously dormant).
4. **Propagate exclusion**: For each newly resolved node, check its Excludes edges. Nodes on the other end transition to Unreachable. Unreachability propagates transitively to their exclusive descendants (Orphaned).
5. **Fire consequences**: Nodes that represent compound consequences — mandated shifts, echo activations, sediment-level changes — fire their effects when resolved.

This is incremental: only the frontier of newly-resolved nodes needs processing. The bulk of the DAG is untouched on any given turn.

### Scene Entry and Exit

Scene transitions interact with the DAG in two ways:

- **Scene entry** may assert precondition nodes as Resolved (authored givens for this scene) and may make previously dormant nodes evaluable (the player has entered a scene where certain events become possible).
- **Scene exit** may mark nodes as Unreachable if the departed scene was the only context where they could resolve (the player left the town without visiting the garden; the "discover the garden" node is now unreachable if there is no return path).

The narrative graph's scene connectivity determines reachability: if a scene is still reachable via some path in the narrative graph, its associated event nodes remain Unresolved (not Unreachable). Only when all paths to a scene are closed do its exclusive event nodes become Unreachable.

### Graceful Degradation

When a subgraph becomes unreachable, the system doesn't simply discard it. Unreachable subgraphs represent *roads not taken* — they may be narratively significant:

- The Storykeeper may note unresolved suspicions (Nancy suspected but never discovered the truth) and adjust the narrative's emotional undertone accordingly.
- Alternative triggers may exist for the same compound consequence via different dependency paths (Nancy discovers the truth through a different scene, or a different character reveals it).
- The story designer may author fallback consequences for unreachable subgraphs (if the player never discovers the secret, the relationship erodes slowly instead of transforming through revelation).

This is not something the DAG evaluation engine needs to handle directly — it surfaces unreachable subgraphs as data, and the Storykeeper and narrative systems interpret them.

---

## Implications for Current Work

### For the Event Grammar (Complete)

The EventAtom and CompoundEvent types are correctly scoped — they model turn-level events. No changes needed. The event grammar's vocabulary (EventKind, ParticipantRole, ImplicationType) provides the language in which DAG node resolution conditions will eventually be expressed. This is a validation criterion: if a narrative trigger concept can't be expressed in terms of EventKind + participants + relational implications, the event grammar vocabulary may need extension.

### For the ML Classifier (Phase C)

The classifier doesn't need to know about the DAG. It classifies individual turns into EventAtoms. But its output feeds into DAG evaluation — each newly extracted event potentially resolves one or more DAG nodes. This means:

- The classifier's EventKind taxonomy must be rich enough that authored resolution conditions can reference it. If a story designer writes "when Nancy discovers the truth," the classifier must be able to produce an EventAtom with `EventKind::InformationTransfer` and appropriate participants, so that the DAG node's condition can match against it.
- Entity extraction must be reliable enough that DAG node conditions referencing specific entities ("Nancy," "the financial records") can match against classifier output. Unresolved entity references in EventAtoms are acceptable in the event grammar, but DAG evaluation needs resolved references (or at least high-confidence matches) to determine whether a node's condition is met.

### For Apache AGE / Storage

The event dependency graph is a natural fit for the graph store already chosen. Event condition nodes, dependency edges with typed relationships, resolution state — all expressible as graph data alongside the narrative graph and relational web. Cypher's pattern matching handles "find all evaluable nodes whose predecessors are all resolved" cleanly. The DAG could share the same PostgreSQL + AGE instance as the other graphs, with a distinct label namespace.

### For Story Designer Authoring (Future)

The DAG is the backing representation for what story designers experience as "narrative triggers." A visual authoring tool would expose:

- Nodes as authored event conditions with resolution criteria.
- Edges as dependency relationships drawn between conditions.
- Resolution state visualized during playtest (green = resolved, gray = unresolved, red = unreachable, dim = orphaned).
- Subgraph templates for common patterns (discovery arcs, accumulating suspicion, branching consequences).

The authoring surface is not this document's concern, but the DAG's structure should be kept authorable: node conditions expressed in a language that maps to designer intent, not implementation detail. "Nancy suspects infidelity" is an authorable condition; `Threshold(0.7, [AccumulatedStress(nancy, jealousy), ...])` is its implementation.

---

## Open Questions

1. **Granularity boundaries.** Where does turn-level CompoundEvent end and arc-level DAG begin? A clear heuristic is needed — perhaps: if the composition spans more than one scene, it's a DAG relationship; if it's within a scene, it's a CompoundEvent. But edge cases exist (multi-turn compositions within a single long scene).

2. **Authored vs. emergent DAG structure.** The examples above assume the DAG is authored by the story designer. But some dependency structures are emergent — the system detects that accumulated events create a dependency chain that wasn't explicitly authored. Should the DAG support both authored (static) and inferred (dynamic) nodes? What produces the inferred nodes?

3. **Amplification semantics.** How exactly does an Amplifies edge affect a downstream node's consequence? Is it a weight multiplier on the mandated shift? A modifier on the emotional intensity? A flag that the Narrator should render the moment with more gravity? The concept is clear; the mechanics need specification.

4. **Reachability computation cost.** Determining whether a DAG node is still reachable requires knowing which scenes are still accessible in the narrative graph. This is a cross-graph query (event DAG × narrative graph). How expensive is this, and when should it be computed? Scene transitions are a natural evaluation point, but mid-scene reachability changes (a character departs, closing an information path) may also matter.

5. **DAG versioning and evolution.** When the story designer adds new content (new scenes, new characters, new arcs), the DAG evolves. How does this interact with in-progress play sessions? Can new nodes be added to a live DAG? Can edges be modified? This is a content deployment problem as much as a data model problem.

6. **Fallback and alternative path authoring.** The "graceful degradation" section describes the concept but not the mechanism. How does a story designer author "if this subgraph becomes unreachable, trigger this alternative consequence instead"? Is this a special edge type? A separate fallback DAG? A Storykeeper heuristic?

7. **Visualization and debugging.** For the story designer, the DAG is the most complex authored artifact in the system. Good tooling for visualizing, navigating, and debugging it is essential — but what does that tooling look like? Graph visualization at narrative scale is a known hard problem.

---

## Design Principles

1. **The DAG models dependencies, not events.** Events live in the ledger. The DAG models the relationships *between* event conditions — what enables what, what excludes what, what must precede what. It is a graph over conditions, not a graph over occurrences.

2. **Evaluation is incremental, not global.** The DAG is evaluated at the frontier — newly resolved nodes propagate forward. The system never re-evaluates the entire graph. This keeps per-turn cost proportional to the number of newly resolved nodes, not the size of the DAG.

3. **Unreachability is as meaningful as resolution.** Roads not taken carry narrative weight. The system should surface unreachable subgraphs as data, not silently discard them.

4. **The DAG is authorable.** Story designers think in terms of "if this happens, then this becomes possible." The DAG's structure should map to that intuition. Implementation-level details (trigger predicates, confidence thresholds, entity resolution) are hidden behind authorable abstractions.

5. **Acyclicity is non-negotiable.** Events depend on prior events. Circular dependencies are not temporal paradoxes to be resolved — they are authoring errors to be rejected at validation time. The DAG must be validated as acyclic when authored content is loaded.
