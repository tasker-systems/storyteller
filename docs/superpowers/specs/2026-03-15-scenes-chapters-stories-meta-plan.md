# Scenes, Chapters, and Stories — Meta-Plan

**Date:** 2026-03-15
**Branch:** `jcoletaylor/scene-setting-data-and-intent-driving`
**Status:** Design
**Roadmap:** `docs/technical/roadmap/2026-03-16-scenes-chapters-and-stories.md`

## Purpose

This is a **meta-spec** — it decomposes the six-part roadmap into three tiers of work, defines each tier's work units and completion criteria, establishes dependency ordering, and specifies what kind of design artifact each tier produces. It does not contain the designs themselves; each work unit gets its own spec → plan → implementation cycle.

## Roadmap Summary

The roadmap document identifies six interconnected concerns discovered during Phase 3 workshop conversion and playtesting:

| Part | Title | Core concern |
|------|-------|-------------|
| I | Pragmatic Preamble | Sequential RPCs, streaming channels, turn 0, player intent review |
| II | The Dramaturge | Async agent for turn-level dramatic direction |
| III | Genre as Hyperplane | Multidimensional genre modeling replacing flat taxonomy |
| IV | Data Acquisition and Provenance | Structured extraction from public domain, CC-BY-SA, and LLM sources with ethical attribution |
| V | Narrative Spatial Topology | Place-as-entity, tonal inheritance, World Agent lifecycle |
| VI | Scene Resolution | Possibility foreclosure, convergence signals, scene-to-scene handoff |

## Three-Tier Decomposition

### Tier Classification

Each roadmap part falls into one of three tiers based on the nature of the work required:

| Tier | Nature | Roadmap parts | Output type |
|------|--------|---------------|-------------|
| **A** | Bounded engineering | I (§1, §2, §4, §5; §3 deferred) | Specs → plans → implementation |
| **B** | Exploratory research | III, IV, V (data needs) | Python tooling, generated data, proposed schemas, analysis docs |
| **C** | Architecture-then-engineering | II, V (agent/topology), VI | Specs → plans → implementation, grounded in Tier B's data |

### Dependency Ordering

```
Tier A ──→ Tier B ──→ Tier C
(engineering)  (exploration)  (architecture)
```

**A → B → C is the operational execution order.** Tiers A and B are technically independent — different languages (Rust vs. Python), different codebases, different concerns. Sequential execution is an operational constraint (single developer with limited evening/weekend hours across multiple projects), not a technical dependency. Context-switching between tiers is safe if desired.

**Tier B feeds Tier C:** The exploratory work produces data specifications and proposed schemas that Tier C's architectural decisions are grounded in. Without real generated data to validate against, Tier C risks premature formalization.

**Tier A feeds Tier C:** The streaming channel infrastructure (A.3) and turn lifecycle formalization are prerequisites for the dramaturge's integration with context assembly and the gameplay event sequence.

---

## Tier A — Bounded Engineering

Tier A addresses the pragmatic near-term concerns from Part I of the roadmap. These are well-defined engineering tasks that improve the current workshop and lay infrastructure for later tiers.

### Work Unit A.1: Sequential Descriptor RPCs

**Roadmap reference:** Part I, §1

**Problem:** Phase 3 introduced a `GetGenreOptions` combined RPC that flattens the wizard's per-step intersection and fallback logic into incommensurate option lists.

**Scope:**
- Replace `GetGenreOptions` with per-step RPCs that accept prior selections as input: `GetGenres()`, `GetArchetypes(genre)`, `GetProfiles(genre, archetype)`, `GetDynamics(genre, archetype, profile)`
- Server-side: intersection and fallback logic moves into each RPC handler
- Workshop: `SceneSetup.svelte` wizard steps call RPCs sequentially

**Dependencies:** None. Independent of other A work units.

**Spec scope:** Small-medium. The logic existed in the old `commands.rs`; this restores it in a new location.

### Work Unit A.2: Turn 0 Opening Narration

**Roadmap reference:** Part I, §4

**Problem:** Scenes start with player input, not narrator-rendered opening prose. Human players would see a blank scene with an input prompt and no context.

**Scope:**
- Engine/server: on scene start, narrator renders an opening passage before player input is accepted
- `turns.jsonl`: turn 0 recorded with `player_input: null`
- Workshop: opening narration displays before input bar activates
- Session resume: turn 0 replayed as part of history

**Dependencies:** None. Independent of other A work units.

**Spec scope:** Small-medium. Touches the turn cycle, event persistence, and workshop rendering.

### Work Unit A.3: Gameplay Streaming Channel + Player Intent Review + Turn Lifecycle

**Roadmap reference:** Part I, §2, §5 (§3 annotated as deferred)

**Problem:** Scene rendering collects the full narrator response and returns it as a single gRPC response. Player character intentions are generated but never shown to the player. Turn boundaries are implicitly defined by narrator response completion, which breaks once rendering becomes a token stream.

**Scope:**
- `workshop:gameplay` event channel infrastructure alongside existing `workshop:debug` and `workshop:logs`
- Narrator token streaming over this channel (replaces request-response for narrator rendering)
- Player intent review step: after composition, before opening narration, player sees/modifies their character's generated intentions
- Event sequence: `SceneComposed` → `IntentProposal` → player interaction → `IntentConfirmed` → `SceneOpening` → gameplay
- **Turn lifecycle formalization**: define what signals constitute a complete turn, in what order, and what event marks the system as ready for the next player input. Narrator `NarratorComplete` becomes one signal among several (event persistence, ML predictions queued, downstream processing) rather than the implicit turn boundary.
- `workshop:media` annotated as a future addition to this channel, **deferred until Tier B provides aesthetic/genre data** — with sparse descriptions and limited genre-hyperplane semantics, image generation would produce lower-value outputs

**Dependencies:** Depends on A.2 (turn 0 must exist for the intent review → opening narration sequence to work).

**Spec scope:** Large. New gRPC streaming infrastructure, narrator rendering refactor, new UI interaction step, and turn lifecycle architecture.

### Tier A Execution Order

A.1 and A.2 can be done in either order (or combined into one spec if that feels natural). A.3 follows, as it depends on A.2 and is the largest unit.

### Tier A Completion Criteria

- Sequential RPCs working in workshop wizard
- Turn 0 renders on scene start and session resume
- Gameplay channel streams narrator tokens to frontend
- Player intent review step functional between composition and opening narration
- Turn lifecycle formalized with clear boundary semantics distinct from narrator render completion
- `workshop:media` annotated as deferred in code and documentation

---

## Tier B — Exploratory Research

Tier B produces **data and proposed schemas**, not running system code. Its output is the foundation Tier C architects against. All tooling lives in a new Python package at `tools/narrative-data/` (pyproject.toml, uv, ruff, pytest — same pattern as existing Python packages). Primary LLM: qwen3.5:35b via Ollama for structured elicitation.

### Stream B.1: Genre / Trope / Narrative Structure

**Roadmap reference:** Parts III, IV

**Scope:**
- Identify ~20 genre regions that cover meaningful narrative space (folk horror, nordic noir, romantasy, cozy fantasy, hard sci-fi, literary fiction, magical realism, etc.)
- For each region, elicit via structured prompts to qwen3.5:35b:
  - Dimensional position (aesthetic, tonal, thematic, structural, world affordances)
  - **Genre-contextualized expressions of all existing descriptor categories**: archetypes, settings, dynamics, profiles, goals, cross-dimensions, axis vocabulary. A "mentor" in nordic noir is a different character than a "mentor" in cozy fantasy — the current flat descriptors don't capture this.
  - Trope inventory with reinforcement/subversion markers and narrative function
  - Narrative shapes and beat structures (what does "rising tension," "revelation arc," "descent" look like in this genre?)
- Cross-genre analysis: dimensional overlap, natural clusters, discriminating dimensions, false boundaries
- Provenance graph strawman: how multi-source attribution would work with this data shape

**Output:**
- Python scripts for structured LLM elicitation (repeatable, parameterized — not one-shot)
- Generated JSON data across ~20 regions × all descriptor categories
- Proposed schemas for `GenreRegion`, `Trope`, `NarrativeShape`, `ProvenanceEdge`
- Analysis doc: patterns observed, surprises, recommendations for Tier C

### Stream B.2: Spatial / Setting / Place-Entity

**Roadmap reference:** Part V (data needs)

**Scope:**
- For ~5-6 setting types (gothic mansion, pastoral village, urban noir streetscape, fantasy wilderness, sci-fi station, fairy-tale forest, etc.), elicit via structured prompts:
  - Plausible place-entity inventories (what spaces exist in this setting type)
  - Adjacency patterns and topological structure
  - Per-place communicability profiles (atmospheric, sensory, spatial, temporal dimensions)
  - Tonal inheritance rules: how tone propagates across adjacency boundaries
  - Narrative function assignments: which spaces naturally serve as sanctuary, threshold, arena, labyrinth, forbidden zone
- Cross-setting analysis: universal patterns vs. genre-specific patterns
- Friction model exploration: examples of low/high/uncanny friction transitions with tonal descriptions

**Output:**
- Python scripts for structured spatial elicitation (repeatable, parameterized)
- Generated JSON place-entity data across ~5-6 setting types
- Proposed schemas for place-entities, topology graph edges, communicability profiles
- Analysis doc: patterns, tonal inheritance rules, friction observations

### Stream B.3: Cross-Pollination and Edge Discovery

**Roadmap reference:** Emergent from the intersection of Parts III-V

**Depends on:** B.1 and B.2 both reaching initial completion.

**Scope:**
- Review genre findings against spatial findings for mutual enrichment:
  - What narrative themes are implied by settings but not yet captured in genre data?
  - What genre-specific tonal patterns need spatial expressions that weren't generated?
  - Which settings might evoke narrative space possibilities not yet captured?
  - Which narrative space possibilities are best expressed in particular settings?
- This is **aesthetic judgement, not formal logic** — human-guided, LLM-assisted. Review both data sets, identify gaps and resonances, use the same elicitation tooling to fill them in.
- May produce new genre regions, new setting types, or enriched entries in existing ones.

**Output:**
- Updated data sets from B.1 and B.2
- Revised schemas if patterns demand it
- Brief "what we learned" synthesis document that Tier C should read before designing

### Tier B Execution Order

B.1 and B.2 can run in either order or be interleaved. B.3 follows both.

### Tier B Completion Criteria

- ~20 genre regions generated with full descriptor variants across all categories
- ~5-6 setting types with place-entity topologies, communicability profiles, and tonal data
- Cross-pollination pass complete with updated data sets
- Proposed schemas validated against generated data (schemas describe what we actually have, not what we hoped to have)
- Analysis docs written for each stream plus the cross-pollination synthesis
- All tooling in `tools/narrative-data/` is repeatable and parameterized

---

## Tier C — Architecture-Then-Engineering

Tier C designs and builds the runtime systems that transform storyteller from a scene renderer into a narrative engine. Each work unit gets its own spec → plan → implementation cycle, grounded in Tier B's data and schemas.

### Work Unit C.1: The Dramaturge Agent

**Roadmap reference:** Part II

**Depends on:**
- Tier A.3 — gameplay channel and turn lifecycle formalization (the dramaturge writes directives consumed by context assembly and must understand turn boundaries)
- Tier B.1 — genre data and narrative shapes (the dramaturge reasons about genre-specific arc patterns, beat structures, trope expectations)
- Tier B.3 — cross-pollination synthesis (edge-case genre/setting interactions inform directive vocabulary)

**Scope:**
- Async MPSC channel actor: receives turn data, does not block the turn cycle
- Pre-invoked Storykeeper queries for bounded context assembly
- `DramaticDirective` structure: arc position, active narrative forces, turn-level guidance, staleness flags, mutual turn/event ID pointers
- Integration with narrator context assembly pipeline (directives folded into preamble and retrieved context)
- Staleness detection: identifying echo-stagnation when dynamics have been static across multiple turns
- Arc position tracking: mapping player actions against expected narrative shapes from Tier B data
- Description ledger: tracking rendered sensory details and character descriptions to prevent narrator repetition

**Spec scope:** Large. New agent with its own LLM call, context assembly, persistence, and deep integration with the existing turn cycle.

### Work Unit C.2: World Agent Spatial Lifecycle

**Roadmap reference:** Part V (agent architecture)

**Depends on:**
- Tier B.2 — spatial data and place-entity schemas (the World Agent generates and persists this data at runtime)
- Tier B.3 — genre-informed tonal inheritance rules
- C.1 is a soft dependency — the World Agent responds to dramaturge signals, but could be designed in parallel if the directive interface is agreed upon

**Scope:**
- Place-as-entity model: extending the compositional entity framework with spatial communicability dimensions (atmospheric, sensory, spatial, temporal)
- Scaffold phase: parallel with scene composition, structured LLM generation of place-entity topology for the scene's setting
- Enrichment phase: async during play, deepening sensory palettes, responding to dramaturge signals, tracking player attention
- Synchronous consultation points: movement validation, physical interaction feasibility, boundary-crossing events
- Topology graph persistence (likely Apache AGE, aligned with existing relational web patterns)
- Tonal inheritance and friction model implementation
- Authored vs. dynamic convergence: same entity model for pre-authored and generated place-entities

**Spec scope:** Large. New entity types, graph structures, async lifecycle, World Agent system refactor.

### Work Unit C.3: Scene Resolution and Narrative State Progression

**Roadmap reference:** Part VI

**Depends on:**
- C.1 — the dramaturge produces the `ResolutionAssessment`
- C.2 — spatial state is part of what gets evaluated for resolution
- Tier B.1 — narrative shapes define what "resolved" means for a given scene type

**Scope:**
- Convergence signal model: event decomposition + ML predictions + dramaturge arc tracking + intent fulfillment → resolution gradient
- `ResolutionAssessment` structure: dramatic completion, foreclosed/opened narrative paths, active tensions, resolution signal (continuing → approaching → ready → overdue)
- Exit mode signaling to narrator: fade, cut, denouement, rupture
- Possibility foreclosure tracking: threshold events (discrete state changes), gradual foreclosure (relational erosion), rapid foreclosure (sharp relational shifts)
- `NarrativeStateDelta`: what a resolved scene contributes to the narrative graph
- Scene-to-scene handoff: carry-forward tensions, foreclosed paths, opened paths, relational state changes feed the next scene's composition

**Spec scope:** Medium-large. Conceptually rich but builds substantially on C.1's infrastructure. Much of the implementation extends the dramaturge's evaluation rather than building new standalone systems.

### Tier C Execution Order

C.1 first — it is foundational, as both C.2 and C.3 reference the dramaturge's directive and assessment structures. C.2 follows (spatial state must be modeled before scene resolution can evaluate it). C.3 last, building on both.

### Tier C Completion Criteria

- Dramaturge agent operational: produces directives that visibly improve turn-over-turn scene progression compared to current system
- World Agent scaffolds spatial topology at scene start and enriches place-entities during play
- Scene resolution signals scene endings with appropriate exit modes based on convergence of event, relational, and arc-tracking signals
- A playtest demonstrates multi-turn directionality, spatial coherence in unscripted spaces, and scene resolution — capabilities the current system lacks
- `NarrativeStateDelta` produced on scene resolution, demonstrating scene-to-scene state handoff

---

## What This Spec Does Not Contain

- **Detailed designs** for any individual work unit — those are their own specs, produced when work begins on that unit
- **Implementation plans** — those follow from individual specs via the standard spec → plan → implementation cycle
- **Timeline estimates** — operational constraints (single developer, evening/weekend availability, multiple active projects) make calendar estimates unreliable. The sequencing is defined; the pacing is adaptive.
- **`workshop:media` design** — deferred until Tier B provides the aesthetic/genre data needed for meaningful image generation. The streaming channel infrastructure from Tier A will be in place; wiring up media becomes a well-scoped addition at that point.

## Relationship to Prior Work

Several prior design documents touch on concerns addressed by this roadmap:

| Prior design | Relationship |
|---|---|
| `dramaturgy-of-tension-design.md` (Mar 10) | Explored player character tension rendering. Tier C.1 (dramaturge) subsumes and extends this into a full agent architecture. |
| `scene-goals-and-character-intentions-design.md` (Mar 11) | Established scene goals and per-character intentions. Tier A.3 (player intent review) surfaces these to the player. Tier C.1 (dramaturge) uses them as input to arc tracking. |
| `narrator-anti-closure-and-emergent-agency-design.md` (Mar 11) | Addressed narrator drift and echo-stagnation. Tier C.1 (dramaturge) provides the architectural solution via dramatic directives and staleness detection. |
| `engine-server-and-playtest-harness-design.md` (Mar 13) | Established the server/client architecture. Tier A builds on this infrastructure with streaming channels and turn lifecycle formalization. |
| `workshop-conversion-phase3-design.md` (Mar 14) | Converted workshop to thin gRPC client. Tier A.1 corrects the combined RPC decision. Tier A.3 adds the deferred `workshop:gameplay` channel. |
