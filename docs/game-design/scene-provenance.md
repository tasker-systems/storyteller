# Scene Provenance: Authored, Collaborative, and Runtime-Generated Scenes

## The Problem

The narrative graph as originally designed is **story-scoped and static** — all `:Scene` vertices and `:TRANSITIONS_TO` edges are authored before play begins, and every player sees the same graph structure. This assumption was embedded in the initial AGE persistence model (see [`narrative-graph-age.md`](../technical/age-persistence/narrative-graph-age.md)).

But the gravitational authoring model (see [`authoring-economics.md`](authoring-economics.md)) explicitly anticipates that **story designers author the attractors, and the system generates connective tissue**. The sparse-graph problem — the combinatorial explosion of possible paths between gravitational scenes — is solved not by forcing designers to author every intermediate scene, but by generating scenes dynamically based on identified topological gaps.

This means the narrative graph is not purely static. It has three provenance tiers — but all three are **story-scoped**. The key insight is that topological gaps are structural, not per-player: if there's sparse connectivity between a gate scene and the next attractor, every player traversing that region encounters the same problem. Generated scenes enrich the graph for everyone.

---

## Three Provenance Tiers

### Tier 1: Authored Scenes

Scenes written by the story designer. These form the gravitational skeleton of the narrative — the gates, thresholds, and high-mass attractor scenes that define the story's shape.

| Property | Value |
|----------|-------|
| **Scope** | Story-wide |
| **When created** | Before play begins, during story authoring |
| **Mutability** | Immutable during play |
| **Graph presence** | Always in AGE as `:Scene` vertices |
| **Examples** | "Tom Lies Still and Dying" (S1), "A Mother's Prayer" (S3), "The Other Bank" (S6) |

These are the scenes the designer cares most about. They carry the highest narrative mass, the most detailed scene data, the carefully authored approach vectors and departure conditions.

### Tier 2: Collaborative Scenes

Scenes produced through author-system collaboration during story development or story initialization. The author specifies constraints at a higher level of abstraction — "there should be 2-3 connective scenes between the village and the deep wood, exploring Sarah's growing unease" — and the system generates scene definitions that satisfy those constraints.

| Property | Value |
|----------|-------|
| **Scope** | Story-wide |
| **When created** | During story development or at story-load time |
| **Mutability** | Immutable during play (may be refined between story versions) |
| **Graph presence** | In AGE as `:Scene` vertices, indistinguishable from authored in traversal |
| **Examples** | A connective scene on the path from the village to Adam's dwelling, generated to satisfy pacing constraints |

Collaborative scenes are structurally identical to authored scenes once generated. The distinction is provenance metadata — the system tracks that these were generated, which matters for story versioning and author iteration, but not for gameplay.

### Tier 3: Runtime-Generated Scenes

Scenes created during play in response to identified topological gaps in the narrative graph. These emerge from the system's analysis of:

- **Pacing gaps** — the player is moving too quickly between high-mass scenes and needs a reflective beat
- **Character development needs** — a relationship hasn't had space to develop and needs a scene to grow it
- **Unfilled narrative space** — the gravitational landscape reveals an area between attractors that the player's path traverses but where no scene exists
- **Connective tissue** — a player's chosen exit from scene A leads toward scene B, but the structural/emotional distance is too large for a direct transition

| Property | Value |
|----------|-------|
| **Scope** | Story-wide (enriches the graph for all players) |
| **When created** | At scene boundaries, after event propagation and gravitational recalculation |
| **Mutability** | Created once; not modified after creation |
| **Graph presence** | In AGE as `:Scene` vertices, structurally identical to Tier 1-2 |
| **Examples** | A quiet campfire scene generated because the topology between "Kate's Blessing" and "The Deep Wood" was too sparse for direct transition |

---

## The Core Design Decision: Story-Scoped Generation

An earlier iteration of this design considered **session-scoped** runtime scenes — vertices visible only to the player whose session triggered their creation. This was rejected for three reasons:

**1. The gap is structural, not personal.** If there's insufficient connective tissue between scene A and the next gravitational attractor, that's a property of the graph topology, not of any individual player's session. Every player traversing that region benefits from the enrichment.

**2. Session-scoping introduces a nullable property anti-pattern.** A `session_id` on `:Scene` vertices — null for authored scenes, set for generated ones — adds a filter to every reachability query (`WHERE session_id IS NULL OR session_id = $current`). The nullable field signals an architectural ambiguity: is this vertex part of the graph or not? If it's a scene, it's part of the graph.

**3. Session-specific state already has a home.** What varies between players is not the scene's *existence* but how it *plays* — which characters are present, what relational threads are active, what emotional register the Narrator adopts. This is exactly what the existing session-scoped machinery handles: `scene_activation_states` tracks per-session state, scene instances carry per-session play data, and the turn-level event pipeline adapts content to the player's context. No new scoping mechanism is needed.

**The result**: All three provenance tiers are story-scoped. The `:Scene` vertex carries a `provenance` property (`authored`, `collaborative`, `generated`) for metadata and quality-tier signaling, but no session-scoping. Reachability queries remain unchanged — no filters, no nullable properties, no split-brain between AGE and in-process state.

### Graph Enrichment Over Time

A desirable emergent property: the narrative graph grows richer with play. Early players encounter a sparser topology; later players benefit from connective tissue generated by prior sessions. This is analogous to a trail being worn into a landscape — the path exists because someone walked it, and now others can walk it too.

This also enables a **second-order analysis layer** outside the gameplay loop. An LLM-assisted process can periodically examine the graph for remaining sparse regions — areas between attractors where no connective tissue yet exists — and generate enrichment nodes proactively, before any player encounters the gap.

---

## When Are Runtime Scenes Created?

Scene generation cannot happen until after:

1. The current scene is fully resolved (all events committed)
2. Event propagation has completed (the event DAG's state has been updated)
3. Gravitational recalculation has occurred (effective mass has been recomputed based on new state)
4. The system has identified that the topology toward the next attractor is sparse

This means runtime scene generation happens in the **scene exit pipeline**, after structural modifier recomputation and before next-scene selection:

```
Scene Exit:
  1. Update activation state
  2. Recompute structural modifiers
  3. Analyze narrative topology toward candidate next scenes (NEW)
  4. Generate scenes if topological gaps identified (NEW — create AGE vertices + edges)
  5. Select next scene (may now include the just-generated scene)
```

### What Generates the Scene Content?

A runtime-generated scene needs:

- A scene definition (title, cast, stakes, constraints, setting)
- Narrative mass (low for connective scenes, moderate for character development scenes)
- Approach vectors (derived from the gap the scene is filling)
- Edges to/from existing scenes (structural integration into the graph)
- A PostgreSQL `scenes` row (with `provenance = 'generated'`)

The generation pipeline is a Storykeeper responsibility — it has the gravitational landscape, the player's trajectory, the character development state, and the ability to request scene definitions from an authoring agent or templating system. This is a significant piece of system design that doesn't exist yet.

### Transition Edge Creation

When a generated scene is inserted into the graph, it needs `:TRANSITIONS_TO` edges connecting it to existing scenes. The typical pattern is **interposition** — the new scene is inserted between two existing scenes:

```
Before:  A --[0.8]--> B
After:   A --[0.6]--> G --[0.7]--> B  (G = generated scene)
```

The original A→B edge may be retained (the direct transition remains possible) or removed (the generated scene is now required). This is a generation-pipeline decision based on the gap type — pacing beats are interposed, while alternative-path scenes create new branches.

---

## Implications for Existing Design

### AGE Persistence (`narrative-graph-age.md`)

The `:Scene` vertex schema gains one new property:

| Property | Type | Notes |
|----------|------|-------|
| `provenance` | string | `authored`, `collaborative`, `generated` |

No `session_id`. No new query filters. No additional indexes beyond what already exists.

The `scene_activation_states` table handles session-specific state for generated scenes exactly as it does for authored scenes.

### Scene Lifecycle (`scene-model.md`)

The scene exit pipeline gains new steps (gap analysis, scene generation). The scene entry pipeline must handle scenes whose definition was generated (potentially with lighter scene_data and simpler approach vectors than fully authored scenes).

### Authoring Economics (`authoring-economics.md`)

The sparse-graph problem section already anticipates this: "The system infers what should fill the space between authored attractors." This document makes that concrete by defining the provenance tiers and the generation trigger.

### Storykeeper API Contract

New operations are implied:

- `analyze_narrative_gaps(story_id, current_scene_id) → Vec<NarrativeGap>` — identify where runtime scenes are needed
- `generate_scene(gap: NarrativeGap) → SceneDefinition` — produce a scene definition for a gap
- `integrate_generated_scene(scene: SceneDefinition) → SceneId` — add the scene to AGE + PostgreSQL

### PostgreSQL Schema

The `scenes` table needs a provenance column:

```sql
ALTER TABLE scenes
    ADD COLUMN provenance TEXT NOT NULL DEFAULT 'authored';

CREATE INDEX idx_scenes_provenance ON scenes(provenance);
```

No `session_id` on the scenes table. Generated scenes are story-scoped rows like any other scene.

---

## Cross-Player Resonance (Deferred)

An intriguing future possibility: if the narrative graph enriches over time through play, could the *quality* of that enrichment create a form of cross-player resonance? Player A's journey generates a scene that, because it was played with emotional intensity, acquires higher narrative mass than a typical connective scene. Player B, encountering that same scene later, experiences a gravitationally richer topology — the scene "remembers" that something significant happened here, even though the specific events were Player A's.

This is architecturally natural under story-scoped generation — the generated scene already exists for all players. The question is whether the scene's narrative mass should be updated based on play history (making heavily-played generated scenes more gravitationally significant) or remain at its initially assigned mass.

This is a "What We Defer" item. The provenance model does not preclude it.

---

## What This Document Does NOT Resolve

1. **The scene generation pipeline** — how does the Storykeeper decide what kind of scene to generate? What are the heuristics for "pacing gap" vs "character development need" vs "unfilled narrative space"?
2. **Scene quality tiers** — runtime-generated scenes will have less authored detail than Tier 1 scenes. How does the system compensate? Does the Narrator get different guidance for generated vs authored scenes? Does the `provenance` property signal the Narrator to work harder on establishing atmosphere?
3. **Generation triggers and thresholds** — how sparse is "too sparse"? What metric determines that the topology between two scenes needs enrichment? Is it hop count, narrative distance, pacing analysis, or some combination?
4. **Edge creation policy** — when is a generated scene interposed (replacing a direct edge) vs branched (adding an alternative path)? How does this interact with the gravitational landscape?
5. **Testing and validation** — how do we verify that runtime-generated scenes maintain narrative coherence? What metrics detect a bad generation?
6. **Second-order analysis pipeline** — the out-of-gameplay-loop LLM process for proactive gap analysis. When does it run? What authority does it have to modify the graph?

These are design questions for future work — likely a dedicated milestone after the AGE foundations are built and the scene generation pipeline is designed.

---

## Relationship to Other Documents

| Document | Relationship |
|----------|-------------|
| [`authoring-economics.md`](authoring-economics.md) | Establishes the "authors write attractors, system fills connective tissue" principle |
| [`narrative-graph-age.md`](../technical/age-persistence/narrative-graph-age.md) | AGE schema with `provenance` property on `:Scene` vertices |
| [`scene-model.md`](../technical/scene-model.md) | Scene lifecycle that needs new generation steps |
| [`storykeeper-api-contract.md`](../technical/storykeeper-api-contract.md) | New operations for gap analysis and scene generation |
| [`gravitational-context-assembly.md`](../technical/gravitational-context-assembly.md) | Context assembly that handles generated scenes alongside authored ones |
| [`narrative-gravity.md`](../technical/graph-strategy/narrative-gravity.md) | Gravitational mathematics — runtime scenes have mass that affects the landscape |
