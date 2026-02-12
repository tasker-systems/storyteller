# Gravitational Context Assembly

## Purpose

This document bridges two ideas that have been theorized separately: **narrative gravity** (the gravitational landscape where scenes have mass and stories bend toward pivotal moments) and **context assembly** (the Storykeeper's construction of the three-tier context window for the one-shot Narrator prompt). The bridge is this: narrative gravity is a retrieval signal. The gravitational landscape is the weighting function that determines what enters the Narrator's context and how prominently it appears.

The system's one-shot Narrator approach depends on context curation, not prompt engineering. The Narrator receives a context window of ~2100-3300 tokens and produces a single narrative beat. It does not know about gravity, sub-graphs, attractor basins, or information boundaries. It knows only what the Storykeeper chose to tell it. The art of the system — its artistic agency — lives entirely in what the Storykeeper selects.

This document specifies the mechanics of that selection.

### Relationship to Other Documents

- **`narrator-architecture.md`** — Defines the three-tier context architecture (preamble + journal + retrieved) and the Narrator's role as sole generative intelligence. This document specifies how the retrieved tier is populated.
- **`knowledge-graph-domain-model.md`** — Defines the four graphs and the gravitational model. This document explains how that model drives retrieval decisions.
- **`storykeeper-api-contract.md`** — Defines `assemble_narrator_context` and `query_entity_relevance`. This document specifies the internal logic of those operations.
- **`tales-within-tales.md`** — Defines sub-graph collective mass and prophetic cascade. This document integrates those concepts into the retrieval ranking.

---

## The Context Window as Curatorial Act

### What the Narrator Receives

The Narrator's input has four components:

| Component | Size | Update Frequency | Content |
|---|---|---|---|
| **Tier 1: Preamble** | ~600-800 tokens | Scene boundaries | Voice, genre, cast list, scene setting, aesthetic constraints |
| **Tier 2: Journal** | ~800-1200 tokens | Per turn (compressed) | What has happened in this scene, at varying resolution |
| **Tier 3: Retrieved** | ~400-800 tokens | Per turn (assembled fresh) | What the Narrator needs to know right now beyond Tiers 1 and 2 |
| **Input** | ~300-500 tokens | Per turn | Classified events, resolved intents, player text |

Tiers 1 and 2 are relatively deterministic — the preamble changes at scene boundaries, the journal compresses according to recency and narrative weight rules. The input is given by the turn cycle pipeline.

**Tier 3 is where gravity lives.** The retrieved context tier is assembled fresh each turn by the Storykeeper, and it is the primary mechanism through which the gravitational landscape, sub-graph resonances, and information boundaries shape what the Narrator produces.

### Why This Matters

The Narrator does not decide what characters do — the prediction model does that. The Narrator does not decide what is possible — the resolver does that. The Narrator does not decide what is known — the Storykeeper does that. The Narrator decides **how to tell it**.

But "how to tell it" depends entirely on what materials the Narrator has. A Narrator that receives only immediate scene facts writes flat, reactive prose. A Narrator that receives contextually rich materials — thematic resonances, approaching dramatic weight, relational subtext, the shadow of prophetic imagery — writes prose that feels inevitable, layered, alive.

The Storykeeper's context assembly is the system's directorial intelligence. Every detail included is a choice; every detail excluded is also a choice. Gravity is the framework that makes those choices coherent rather than arbitrary.

---

## The Retrieval Pipeline

### Overview

When the Storykeeper assembles Tier 3 context, it runs a pipeline that produces **candidate context items**, ranks them by a composite score that incorporates gravitational weight, and trims to the token budget.

```
Classified Player Input + Resolved Intents
    │
    ▼
Candidate Generation (multiple sources, parallel)
    │  entity references → relational context
    │  emotional shifts → self-edge data, awareness annotations
    │  narrative position → gravitational landscape queries
    │  information reveals → revelation context
    │  thematic echoes → echo context
    │  sub-graph resonance → prophetic cascades, bridge objects
    ▼
Candidate Pool (~2000-5000 tokens of candidate material)
    │
    ▼
Gravitational Retrieval Ranking
    │  composite score per candidate
    │  deduplicate, merge overlapping candidates
    ▼
Token Budget Trim (~400-800 tokens)
    │
    ▼
Structural Formatting
    │  structured narrative fact with emotional annotation
    ▼
Tier 3 Retrieved Context
```

### Candidate Generation

Each candidate source produces structured context items with an initial relevance score. Sources run in parallel — they are independent queries against different aspects of the system's state.

**Source 1: Entity-Triggered Retrieval**

When the player input or resolved intents reference specific entities, the Storykeeper retrieves relational context for those entities.

```
trigger: entity reference in classified input
query: query_entity_relevance(entity_id, scene_context, recent_turns)
produces:
    - relational edges to other cast members (bounded by information state)
    - recent events involving this entity (weighted by recency, narrative weight)
    - emotional context (current state, awareness annotations)
    - backstory items permitted by information boundaries
initial_relevance: 1.0 (directly responsive to player input)
```

This is the highest-priority source — it directly responds to what the player did or said. If the player asks about a character, the Narrator needs relational context for that character.

**Source 2: Emotional State Retrieval**

When character predictions produce emotional shifts, the Storykeeper retrieves context that illuminates the shift.

```
trigger: emotional_delta in CharacterIntent predictions
query: self-edge data for shifted character, awareness-level context
produces:
    - what the character is feeling and at what awareness level
    - relational edges that may explain the shift
    - relevant backstory that the emotional state connects to
initial_relevance: 0.8 (active emotional thread)
```

This source ensures the Narrator knows *why* a character is reacting, not just *that* they are. The awareness level annotation is critical — a Defended emotion should be shown through behavior, not stated directly.

**Source 3: Gravitational Landscape Retrieval**

This is the source that makes gravity operational. The Storykeeper queries the narrative graph for the current gravitational landscape and surfaces context related to the strongest attractors.

```
trigger: every turn (always evaluated)
query: gravitational_pull(current_scene, reachable_scenes, player_state)
produces:
    - for each high-pull scene: thematic register, tonal signature, stakes
    - approach vector satisfaction status (how close is the player to this attractor?)
    - gate proximity for gates between current position and attractor
    - cast connections (which entities in the current scene also appear in the attractor scene?)
initial_relevance: f(gravitational_pull)  // proportional to pull strength
```

A scene with high gravitational mass that the player is approaching produces context items about that scene's themes, stakes, and cast connections. The Narrator receives these not as "here is the next scene" (the Narrator has no concept of scenes) but as thematic and relational weight that inflects how the current scene is rendered.

**Example**: The cave in Vretil has mass 1.3 from the flash-forward prologue. As Chris travels south (mass rises to 1.8 after Mallory's death), the Storykeeper surfaces cave-related context more frequently and more prominently: the knife imagery, the old woman's voice, the darkness. The Narrator doesn't know it's being directed toward the cave — it simply has more material about caves, sacrifice, and darkness in its context window, and it writes accordingly.

**Source 4: Sub-Graph Resonance Retrieval**

When sub-graphs are active or have been visited, their collective mass and prophetic cascades influence what the Storykeeper retrieves.

```
trigger: active sub-graphs in narrative stack, or recent sub-graph exits
query: sub-graph collective mass, active prophetic cascades, bridge object state
produces:
    - for high-mass sub-graphs: thematic register that should bleed into current scene
    - for active prophetic cascades: approach vector modifiers for target scenes
    - for bridge objects present in current scene: cross-layer symbolic weight
    - emotional residue from recent sub-graph exits
initial_relevance: f(sub_graph_collective_mass * boundary_permeability)
```

This source implements the key insight: a sub-graph's gravitational pull extends beyond its entry and exit points. The fairy tale's collective mass affects retrieval in main-narrative scenes even when the player is not in the fairy tale. The magnitude of this influence is modulated by boundary permeability — at low permeability, only faint thematic resonance crosses; at high permeability, the sub-graph's imagery, symbols, and emotional weight flood the retrieval pool.

**Source 5: Information Reveal Retrieval**

When the resolver determines that new information is now knowable — a gate condition newly satisfied, an observation newly possible — the Storykeeper retrieves the revelation context.

```
trigger: information boundary change (gate opened, fact revealed, observation possible)
query: revelation context with narrative framing
produces:
    - the revealed fact with epistemic provenance (how was it learned?)
    - emotional weight of the revelation (how much does this matter?)
    - relational implications (how does learning this change what the character knows about relationships?)
initial_relevance: 0.9 (revelations are high-priority narrative events)
```

**Source 6: Thematic Echo Retrieval**

When the event classifier or a dedicated echo detector identifies that the current moment rhymes with an earlier one, the Storykeeper retrieves the echo context.

```
trigger: echo detection from classifier (same entities, similar emotional states, same location, repeated gesture)
query: event ledger search for the echoed moment
produces:
    - the earlier event at compressed resolution
    - what has changed since then (relational shifts, emotional evolution)
    - the rhyme pattern (what makes this moment echo that one)
initial_relevance: 0.6 (thematic resonance, lower priority than direct references)
```

Echo context enables the Narrator to weave resonance — a returned-to gesture, a phrase that mirrors an earlier exchange, a setting that recalls a previous emotional state. Without echo context, the Narrator cannot know that the current moment rhymes with anything.

---

## Gravitational Retrieval Ranking

### The Composite Score

After candidate generation, the Storykeeper has a pool of candidates that typically exceeds the token budget by 3-5x. Ranking determines what makes the cut.

Each candidate receives a **composite score** that combines multiple signals:

```
composite_score(candidate) =
    initial_relevance(candidate)                        // from the generating source
    * gravitational_modifier(candidate)                 // proximity to high-mass attractors
    * information_boundary_factor(candidate)             // is this permitted by boundaries?
    * recency_decay(candidate)                          // how recent is the underlying data?
    * narrative_temperature_modifier(candidate)          // does the scene's current temperature favor this?
```

### Gravitational Modifier

The gravitational modifier amplifies candidates that align with the narrative's current gravitational pull:

```
gravitational_modifier(candidate) =
    if candidate.relates_to(strongest_attractor):
        1.0 + (gravitational_pull(strongest_attractor) * attractor_weight)
    elif candidate.thematically_resonates_with(strongest_attractor):
        1.0 + (gravitational_pull(strongest_attractor) * thematic_weight)
    else:
        1.0  // no gravitational amplification
```

Where `attractor_weight` and `thematic_weight` are tunable parameters (defaults: 0.3 and 0.15 respectively). The modifier is multiplicative — it amplifies existing relevance rather than creating relevance from nothing. A candidate with low initial relevance is not rescued by gravitational proximity alone.

**What "relates to" means**: A candidate relates to an attractor if it involves entities in the attractor's required or contingent cast, settings adjacent to the attractor's setting, or events that satisfy the attractor's approach vectors.

**What "thematically resonates" means**: A candidate thematically resonates with an attractor if the candidate's content shares thematic registers with the attractor scene. If the attractor scene has `thematic_register: ["sacrifice", "transformation"]` and the candidate involves a character discussing duty or loss, the resonance modifier applies. This is a fuzzy match — the system uses the thematic register tags on scenes and the thematic annotations on events, not semantic similarity search.

### Sub-Graph Mass in the Gravitational Modifier

Sub-graph collective mass enters the gravitational modifier through the same mechanism as scene mass. A sub-graph with high collective mass is, gravitationally, equivalent to a high-mass scene — it pulls the retrieval toward its themes, entities, and symbols.

```
gravitational_modifier(candidate) =
    // scene attractors
    scene_component = max(gravitational_pull(scene) for scene in nearby_attractors
                          where candidate.relates_to(scene))
    // sub-graph attractors
    sub_graph_component = max(sub_graph_collective_mass(sg) * boundary_permeability(sg)
                              for sg in active_or_visited_sub_graphs
                              where candidate.resonates_with(sg))

    1.0 + max(scene_component * attractor_weight,
              sub_graph_component * sub_graph_weight)
```

The sub-graph component is modulated by boundary permeability. At permeability 0.1 (firm boundary), a sub-graph with collective mass 0.8 contributes only 0.08 to the retrieval ranking. At permeability 0.7 (bleeding boundary), the same sub-graph contributes 0.56. At permeability 1.0 (merged), the sub-graph's mass applies at full strength — it has become part of the parent landscape.

### Prophetic Cascade in the Gravitational Modifier

Active prophetic cascades are a special case of gravitational pull — they are forward-pointing approach vectors from sub-graph events to unreached parent-graph scenes. In retrieval ranking, prophetic cascades amplify candidates that align with the prophesied scene:

```
prophetic_modifier(candidate) =
    sum(prophecy.approach_vector_modifier.magnitude
        for prophecy in active_prophecies
        where candidate.relates_to(prophecy.target_scene))
```

If the fairy tale's fire-dream (chapter 11) created a prophetic cascade targeting the cave scene, then any candidate related to the cave — its setting, its cast, its thematic register, its bridge objects — receives prophetic amplification in every subsequent retrieval ranking until the prophecy is fulfilled.

Multiple prophetic cascades compound. A scene targeted by three separate prophetic cascades from three sub-graph events is under enormous retrieval pressure — the Storykeeper surfaces its themes consistently and prominently, bending the narrative toward it without the Narrator ever being told to do so.

### Information Boundary Factor

The information boundary factor is binary for hard boundaries and graduated for soft ones:

```
information_boundary_factor(candidate) =
    if candidate.violates_hard_boundary():
        0.0  // absolutely excluded, regardless of other scores
    elif candidate.contains_withheld_information():
        0.0  // Storykeeper guards the mystery
    elif candidate.contains_suspected_information():
        0.5  // can hint at but not state
    else:
        1.0  // fully available
```

This is not a cosmetic filter — it is an integral component of the ranking computation. A high-gravity, high-relevance candidate that violates an information boundary scores zero. The Storykeeper will not reveal information before its time, regardless of how gravitationally appropriate the reveal would be. The boundary is absolute.

### Narrative Temperature Modifier

The narrative temperature modifier adjusts retrieval based on the scene's current dramatic state:

```
narrative_temperature_modifier(candidate) =
    if scene_temperature == High (approaching climax):
        // favor tension, stakes, unresolved conflicts
        if candidate.emotional_valence in ["tension", "stakes", "conflict"]:
            1.2
        elif candidate.emotional_valence in ["calm", "routine", "exposition"]:
            0.7
        else: 1.0
    elif scene_temperature == Low (aftermath, breathing room):
        // favor reflection, connection, quiet revelation
        if candidate.emotional_valence in ["reflection", "connection", "tenderness"]:
            1.2
        elif candidate.emotional_valence in ["tension", "urgency"]:
            0.7
        else: 1.0
    else: 1.0  // neutral temperature, no modification
```

This prevents tonal whiplash — the Storykeeper does not surface cave-sacrifice imagery during a quiet moment of rebuilding, even if the cave's gravitational pull is strong. The pull persists, but the retrieval respects the current scene's dramatic needs.

---

## Token Budget Allocation

### The Retrieved Tier Budget

The retrieved tier has ~400-800 tokens available. This is a hard budget — the Storykeeper must fit its retrieval within it. The budget allocation follows a priority structure:

```
Budget Allocation (400-800 tokens):
    Mandatory:
        Active revelations (if any)           ~50-100 tokens
        Entity context for player references   ~100-200 tokens

    Gravity-Driven:
        Gravitational landscape context        ~100-200 tokens
        Sub-graph resonance                    ~50-150 tokens

    Enrichment:
        Emotional state annotations            ~50-100 tokens
        Thematic echo context                  ~50-100 tokens
```

The mandatory items are always included — they directly respond to what happened this turn. The gravity-driven items fill the remaining budget according to their composite scores. The enrichment items use whatever budget remains.

When the budget is tight (400-token turns), the gravity-driven and enrichment tiers compete. The composite score determines the winners. When the budget is generous (800-token turns), more gravitational and enrichment context survives, producing richer Narrator material.

### What Gravity Displaces

Gravity is a relative force. High-gravity candidates displace low-gravity candidates. In practical terms, this means:

- When a high-mass scene is nearby, its thematic material displaces generic backstory and low-weight relational context
- When a sub-graph has high collective mass and permeable boundaries, its imagery and symbols displace main-narrative-only context
- When multiple prophetic cascades target the same scene, that scene's thematic register dominates the retrieved tier, reducing space for unrelated context

This displacement is the mechanism through which gravity shapes prose. The Narrator writes toward gravitational attractors because their themes, symbols, and relational weight occupy more of its context window. The Narrator writes differently when approaching the cave because the cave's material — darkness, sacrifice, the knife, the old woman's voice — fills more of the retrieved tier as gravitational pull increases.

---

## Narrator Nudging Through Context Curation

### How the Storykeeper Directs Without Directing

The Storykeeper never tells the Narrator what to write. It never says "mention the cave" or "foreshadow the sacrifice." The anti-pattern directives in the preamble explicitly forbid the Narrator from parroting retrieved context. So how does gravitational context curation influence the Narrator's output?

Through **material availability**. The Narrator is a generative model — it creates from the materials available. If the retrieved tier contains:

```
Retrieved (gravitational context):
    Approaching attractor: The cave sequence. Stakes: everything Chris has
    traveled for converges here. The photographs all point to this place.

    Bridge object resonance: The glass datura flower sits on the table in front
    of Chris. Its petals catch light in a way that recalls the fairy tale's
    purple bell-blossom. Isabella's gift has traveled further than she knows.

    Emotional trajectory: Chris's grief has shifted from defended to articulate
    over the last three scenes. The numbness is lifting. What replaces it is
    not comfort but clarity — the kind that makes someone walk into a cave.
```

Then the Narrator has material about caves, flowers, and the evolution from numbness to clarity. It does not have material about Chris's restaurant career, his favorite music, or the weather in New England. The Narrator writes from what it has. The gravitational landscape determined what it has.

This is the Storykeeper's artistic agency. It is not prompt engineering — the system prompt doesn't change. It is not instruction-following — the Narrator is never told to foreshadow. It is **curatorial intelligence**: the art of choosing what to make available, knowing that a skilled renderer will use what it receives.

### The Gradient of Influence

The Storykeeper's influence is not binary (included/excluded). It is a gradient:

| Gravitational Proximity | Retrieval Behavior | Narrator Effect |
|---|---|---|
| **Distant** (low pull, early story) | Attractor themes appear rarely, at low prominence | Faint echoes — a word choice, an image that will matter later |
| **Approaching** (moderate pull) | Attractor themes appear consistently, at moderate prominence | Growing weight — recurring motifs, thickening atmosphere |
| **Imminent** (high pull, approaching climax) | Attractor themes dominate the retrieved tier | Saturated — the prose bends visibly toward the attractor |
| **Active** (player is in the attractor scene) | Attractor context becomes Tier 1/2 material | The scene is happening; gravity is now immediate experience |
| **Post-resolution** (attractor completed) | Attractor mass drops; other attractors rise | The landscape reshapes; new themes emerge in retrieval |

This gradient creates the narrative experience of approach — the feeling that the story is "building toward something" without anyone stating it. The Narrator's prose unconsciously mirrors the gravitational landscape because the landscape is encoded in its context window.

### Sub-Graph Influence Through the Retrieved Tier

Sub-graph collective mass influences the retrieved tier through the same mechanism, modulated by boundary permeability:

**Low permeability** (firm boundary): The fairy tale's themes appear as faint imagery — a character uses a word that echoes the fairy tale's language, an object in the scene recalls a fairy tale image. The Narrator does not know it is doing this; it simply has a small amount of fairy-tale-resonant material in its context.

**Medium permeability** (bleeding boundary): The fairy tale's imagery appears more consistently. Bridge objects in the current scene receive annotation about their cross-layer significance. Emotional residue from recent sub-graph visits colors the retrieved context. The Narrator's prose absorbs the sub-graph's register — not deliberately, but because the material is there.

**High permeability** (boundaries collapsing): The sub-graph's themes dominate. The retrieved tier is saturated with cross-layer resonance. The Narrator writes in a voice that integrates registers from multiple layers, because its context contains material from all of them.

**Full convergence** (permeability 1.0): The sub-graph has merged with the parent. Its material is no longer "sub-graph resonance" — it is the narrative. The retrieved tier draws from a unified graph, and the Narrator writes in a unified voice.

---

## Interaction with the Prediction Pipeline

### Gravity as a Feature for Character Prediction

The gravitational landscape influences not just the Narrator's context but also the character prediction pipeline. When the prediction model computes character intent, the Storykeeper provides relational features that are themselves gravity-influenced:

- **Relational edge selection**: Which edges the prediction model receives for a character depends on which relationships are gravitationally relevant. A character's edge toward someone in a high-mass attractor scene is more prominent in the feature set than an edge toward a peripheral entity.
- **Emotional state framing**: The emotional context provided to the prediction model includes awareness annotations that reflect gravitational proximity. A character approaching a high-mass scene may receive emotional context weighted toward the scene's tonal signature.

This creates coherence between what characters do (prediction) and how the Narrator renders it (context assembly). Both are influenced by the same gravitational landscape, so the Narrator's prose naturally aligns with the characters' behavior.

### Gate Proximity and Prediction

When a gate is close to triggering (`query_gate_proximity` returns high satisfaction percentages), the Storykeeper biases both the prediction features and the retrieved context:

- **Prediction**: Characters who could trigger the gate receive feature emphasis on the relevant relational edges and emotional states
- **Retrieved context**: The Narrator receives framing about the gate's narrative weight — what it means, what changes when it opens — so that the rendering carries appropriate dramatic weight when the gate fires

This ensures that gate triggers feel earned rather than arbitrary. The narrative builds toward them because the gravitational landscape makes gate-adjacent content progressively more prominent in both the prediction pipeline and the Narrator's context.

---

## Practical Implications

### What the Storykeeper Must Compute Per Turn

The retrieval pipeline adds the following to the per-turn context assembly cost:

| Step | Operations | Estimated Latency |
|---|---|---|
| Candidate generation (6 sources, parallel) | 6 graph queries + ledger searches | ~15-40ms |
| Gravitational landscape query | `strongest_attractor`, `gravitational_pull` for reachable scenes | ~5-10ms |
| Sub-graph resonance query | `sub_graph_collective_mass`, `active_prophecies`, `bridge_objects` | ~5-15ms |
| Composite scoring | Score computation for ~50-100 candidates | ~1-2ms |
| Budget allocation and formatting | Sort, trim, format | ~1-2ms |
| **Total** | | **~25-70ms** |

This is within the context assembly budget (~20-80ms) established in `narrator-architecture.md`. The gravitational queries add ~10-25ms over a gravity-unaware retrieval pipeline — a modest cost for significantly richer context curation.

### What Must Be Precomputed

Some gravitational data changes only at scene boundaries and can be precomputed during scene entry:

- **Reachable scenes and their masses** — cached at scene entry, invalidated by gate openings during the scene
- **Strongest attractor** — recomputed only when dynamic mass adjustments cross significance thresholds
- **Active sub-graph collective masses** — cached, updated on sub-graph entry/exit
- **Boundary permeability values** — cached, updated by permeability-shifting events
- **Active prophetic cascades** — cached, updated on sub-graph exit or prophecy fulfillment

Per-turn computation is limited to: scoring candidates against the precomputed landscape, evaluating information boundaries, and formatting the results.

### Tuning Parameters

The retrieval pipeline has several tunable parameters that affect how gravity shapes context:

| Parameter | Default | Effect |
|---|---|---|
| `attractor_weight` | 0.3 | How strongly scene attractors amplify retrieval candidates |
| `thematic_weight` | 0.15 | How strongly thematic resonance (without direct entity connection) amplifies candidates |
| `sub_graph_weight` | 0.25 | How strongly sub-graph collective mass amplifies candidates |
| `prophetic_weight` | 0.2 | How strongly prophetic cascades amplify candidates |
| `temperature_modifier_range` | 0.7-1.2 | How much narrative temperature adjusts candidate scores |
| `minimum_candidate_score` | 0.1 | Below this composite score, candidates are dropped entirely |
| `retrieved_tier_budget` | 400-800 tokens | The hard budget for Tier 3 context |

These parameters are story-configurable. A story with a single dominant attractor (like Vretil's cave) might use higher `attractor_weight` to create a stronger funnel effect. A story with many parallel threads might lower `attractor_weight` and raise `thematic_weight` to create a more distributed gravitational landscape.

---

## Design Principles

### 1. Gravity Is a Retrieval Signal, Not a Directive

The Storykeeper never tells the Narrator "write toward the cave." It curates a context window that contains cave-adjacent material in proportion to the cave's gravitational pull. The Narrator creates from what it has. The directive is implicit in the material selection.

### 2. The Retrieved Tier Is Where Gravity Lives

The preamble is stable (voice, setting, cast). The journal is chronological (what happened recently). The retrieved tier is where the Storykeeper exercises curatorial judgment. Gravity operates almost entirely through Tier 3 — by determining what "additional context the Storykeeper deems relevant" actually contains.

### 3. Information Boundaries Are Absolute

Gravitational pull does not override information boundaries. A high-mass scene whose themes involve a guarded secret does not leak that secret into the retrieved context. The Storykeeper guards the mystery even when gravity pushes toward revelation. The boundary is not a weight in the composite score — it is a hard gate (score = 0.0 for boundary-violating candidates).

### 4. Sub-Graph Mass Is Aggregate, Not Indexed to Portals

A sub-graph's gravitational influence is not limited to scenes adjacent to its entry/exit portals. The sub-graph has collective mass that operates on the entire retrieval landscape. The fairy tale influences context assembly in scenes that have no portal connection to the fairy tale — because the fairy tale's themes, bridge objects, and prophetic cascades have gravitational weight that reaches everywhere, attenuated by boundary permeability.

### 5. The Gradient Creates the Experience

The progression from faint echo to saturated atmosphere is not a discrete transition — it is a continuous gradient driven by the gravitational landscape's evolution. As scenes complete, gates open, and sub-graph visits accumulate weight, the retrieval ranking shifts. The Narrator never notices the shift. The reader/player notices it as the feeling that the story is building toward something.

### 6. Coherence Across Pipelines

The same gravitational landscape that shapes the Narrator's context also shapes the prediction pipeline's features. Characters and Narrator are influenced by the same forces, producing prose where character behavior and narrative rendering align. This coherence is not engineered per-scene — it emerges from shared gravitational input.

---

## What This Document Does Not Cover

This document deliberately does not specify:

- **The compression algorithm for Tier 2 (journal)** — Progressive compression is covered in `narrator-architecture.md`. This document covers only Tier 3 retrieval.
- **The exact format of structured narrative fact** — How retrieved context is formatted for the Narrator (the distinction between prose and structured annotation) is a rendering concern.
- **Tier 1 (preamble) assembly** — Preamble construction at scene boundaries is relatively straightforward and not gravity-dependent.
- **Training the echo detector** — How the classifier identifies thematic echoes is an ML concern, not a retrieval concern.
- **Performance optimization for gravitational queries** — Caching strategies, query plan optimization, and index design are implementation concerns for the Storykeeper implementations (InMemory and Postgres).

---

## Appendix: Worked Example — Vretil, Chapter 16

Chris arrives in New Mexico. The cave scene has maximum gravitational mass (effective_mass = 2.1). The fairy tale sub-graph has high collective mass after three standalone chapters. Boundary permeability is at Phase 4 (boundaries collapsing, ~0.8). One prophetic cascade is active (fire-dream → cave scene). The datura flower bridge object is present (Chris carries Isabella's glass flower).

**Candidate generation produces:**

| Source | Candidate | Initial Relevance |
|---|---|---|
| Entity-triggered | Sarah's relational context (new character) | 1.0 |
| Entity-triggered | The Prophet / Old Woman's accumulated mystery | 0.9 |
| Emotional state | Chris's emotional trajectory: numbness → clarity | 0.8 |
| Gravitational | Cave scene: stakes, thematic register (sacrifice, transformation) | 0.7 |
| Sub-graph resonance | Fairy tale's mountain-crossing → desert imagery | 0.6 |
| Sub-graph resonance | Datura bridge object: glass flower → bell-blossom → sacred plant | 0.7 |
| Prophetic cascade | Fire-dream imagery: sand, cave, seed, sleeping girl | 0.5 |
| Thematic echo | The photographs — first seen chapter 3, now reaching their destination | 0.6 |

**Gravitational modifier applied:**

- Cave-related candidates amplified: `1.0 + (2.1 * 0.3) = 1.63`
- Fairy-tale-resonant candidates amplified: `1.0 + (0.8 * 0.8 * 0.25) = 1.16` (collective mass * permeability * sub_graph_weight)
- Prophetic cascade candidates amplified: `1.0 + (0.5 * 0.2) = 1.10`

**After ranking and budget allocation (~600 tokens):**

1. Sarah's relational context — new character, highest raw relevance (150 tokens)
2. Cave gravitational context — thematic register, stakes, approach (120 tokens)
3. Datura bridge object — cross-layer resonance, amplified by sub-graph mass (80 tokens)
4. Chris's emotional trajectory — approaching clarity, defending nothing (80 tokens)
5. Fire-dream prophetic imagery — sand, cave, seed (70 tokens)
6. The photographs reaching their destination — thematic echo (60 tokens)
7. The Prophet's accumulated weight — orchestrator approaching (40 tokens)

**What the Narrator produces:** A scene where Chris meets Sarah in New Mexico, and the prose naturally carries: the desert's correspondence with the fairy tale's landscape, the glass flower in Chris's pocket catching light, the photographs as a journey finding its endpoint, and an emotional register of terrifying clarity. The cave is not mentioned — but its gravitational field shapes every sentence.
