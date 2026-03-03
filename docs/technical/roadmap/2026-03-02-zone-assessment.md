# Zone Assessment: What's Grounded, What's Designed, What's Open

**Date**: March 2, 2026
**Context**: TAS-243 in progress (Tier 1 migrations + Storykeeper traits). 26 open tickets in Linear across 11 milestones. Rich documentation across foundation, technical, and game-design layers.

## The Problem This Document Addresses

The project has wide, rich, and deep documentation — foundational philosophy, technical architecture, graph strategy, game design theory, domain modeling, and a working prototype with 336+ Rust tests. The persistence layer is materializing (16 tables, 16 enums, event DAG functions). But it's hard to see the boundary between what's grounded enough to build against, what's designed but untested against reality, and what's still genuinely open ideation.

This matters for picking the next right thing to work on. The Linear backlog (TAS-244 through TAS-264, plus newer tickets) implies a sequential march through milestones. But several of those tickets assume we know things we don't yet know, or prescribe implementations for problems we haven't actually encountered. The question isn't "what's the next ticket?" — it's "what would we learn the most from building next?"

---

## Three Zones

### Zone 1: Grounded and Operational

Things with concrete code, tested behavior, and verified data structures. You can build on these today with confidence.

| Component | Evidence | Key Files |
|-----------|----------|-----------|
| **Turn cycle pipeline** | 8-stage Bevy ECS state machine, tested end-to-end via `play-scene` | `storyteller-engine/src/systems/turn_cycle.rs`, `rendering.rs` |
| **TurnCycleStage + ProvisionalStatus** | Defined enums, `run_if` gating, working state transitions | `storyteller-core/src/types/message.rs` |
| **ML classification pipeline** | DistilBERT ONNX event classifier + NER, character behavior predictor | `storyteller-engine/src/inference/` |
| **Three-tier context assembly** | Preamble + journal + retrieved, token budget management | `storyteller-engine/src/agents/narrator.rs` |
| **Async narrator bridge** | Oneshot polling for LLM call, graceful degradation | `storyteller-engine/src/systems/rendering.rs` |
| **Entity promotion** | Weight computation, 4 resolution strategies, configurable thresholds | `storyteller-core/src/types/entity.rs` |
| **Event grammar** | 10 event kinds, typed payloads, relational implications | `storyteller-core/src/types/event.rs` |
| **PostgreSQL schema** | 16 tables, 16 enums, 38 FK relationships, applied to dev DB | `storyteller-storykeeper/migrations/` |
| **Event DAG tables + functions** | 3 tables, 3 recursive CTE functions, 1 view — applied | Migrations 014, 015 |
| **Storykeeper trait interfaces** | StorykeeperQuery, StorykeeperCommit, StorykeeperLifecycle | `storyteller-core/src/traits/storykeeper.rs` |
| **InMemoryStorykeeper** | Full trait implementation with tests | `storyteller-storykeeper/src/in_memory.rs` |
| **Docker dev environment** | PostgreSQL 18 + AGE 1.7.0 on port 5435, cargo-make tasks | `docker/docker-compose.dev.yml` |

**What this means**: The within-turn loop works. Player input enters, ML classifies and predicts, context assembles, narrator renders, journal compresses. The persistence schema is ready for writes. The Storykeeper trait boundary is defined.

### Zone 2: Well-Designed but Unexercised

Things with detailed specifications and clear architectural intent, but that haven't been tested against actual gameplay. These designs are internally consistent — they could still discover contradictions when they meet reality.

| Component | What's Specified | What's Untested | Key Docs |
|-----------|-----------------|-----------------|----------|
| **Scene entry pipeline** | 6-step process (selection, cast, frames, entities, warming, presentation) | No code runs this pipeline. Frame computation is hardcoded in workshop. Cast assembly is manual. | `scene-model.md` §Scene Lifecycle |
| **Scene exit conditions** | 6 exit types listed (GoalCompletion, TrajectorySelection, NarrativeExhaustion, TemporalAdvance, PlayerInitiated, StorykeepersJudgment) | No evaluation mechanism. What algorithm detects "narrative exhaustion"? What signal triggers "Storykeeper's judgment"? | `scene-model.md` §Exit |
| **Scene-to-scene transition** | Exit → checkpoint → gap analysis → next-scene selection → entry | The transition as a continuous flow is sketched but not specified as a system. The roadmap calls it Phase 3, but it's the least specified Phase 3 item. | `scene-model.md`, `scene-provenance.md` |
| **Graduated stasis response** | 5-level escalation (narrative invitation → character initiative → world pressure → gravitational escalation → narrative contraction) | Each level implies agent behaviors and signals that don't exist. What does the Storykeeper emit? What does the Narrator receive? | `scene-model.md` §Unsuccessful Engagement |
| **Event DAG runtime integration** | Tables and SQL functions deployed. Evaluable frontier function works. | When in the turn cycle does evaluation happen? What system consumes the frontier? How does condition resolution feed back into scene state? | `event-dag-age.md`, migration 015 |
| **Warmed data / context warming** | Detailed struct with per-agent data packages (Narrator, Storykeeper, World Agent, Character Agents) | Context warming is described as struct definitions but the assembly logic — what queries run, what gets compressed, what gets omitted — is open. | `scene-model.md` §Warmed Data |
| **Agent message catalog** | Typed messages (MSG-SK01 through MSG-R01) with token budgets | Partially superseded by narrator-centric pivot. Which messages survive? The Storykeeper→Narrator channel is clear; others are uncertain. | `agent-message-catalog.md` |
| **Relational web semantics** | 5 substrate dimensions, asymmetric directed edges, tensor evolution | The math is specified (graph-strategy docs). The AGE schema is designed. No queries have run against real data. | `relational-web-tfatd.md`, `relational-web-age.md` |
| **Narrative gravity computation** | Effective mass = authored + structural + dynamic modifiers | The formula is conceptual. What function computes it? When does it run? What consumes the output? | `narrative-gravity.md` |

**What this means**: The scene model is the richest document in the project. It describes what happens inside a scene with care and precision. But the *boundaries* of scenes — how one ends, how the next begins, how the system decides — are where the design meets the unknown. These are the decisions that would get forced by trying to run a two-scene sequence.

### Zone 3: Rich Ideation Awaiting Structural Commitment

Things where the thinking is genuinely valuable but the gap between idea and implementation is widest. These are *correctly* open — they shouldn't be decided yet. But it helps to name them as open so you don't feel pressure to implement against them.

| Component | What Exists | Why It's Open | Key Docs |
|-----------|-------------|--------------|----------|
| **Scene provenance tiers** | Authored/Collaborative/Generated taxonomy. `provenance` enum in schema. | Tier 2 and Tier 3 generation pipelines are explicitly deferred. The taxonomy is useful now; the generation machinery is future work. | `scene-provenance.md` |
| **Pacing grammar** | `PacingGrammar`, `DensityExpectations`, `SequentialRule` structs. Low fantasy reference implementation. | Presupposes a sparsity analysis system and a graph with enough scenes to analyze. Neither exists. | `narrative-pacing.md` |
| **Connective scene generation** | Templates, genre patterns, procedural atmosphere. Referenced everywhere. | Specified nowhere as an algorithm. Correctly deferred — you need to know what connective space *feels like* in play before you can generate it. | `scene-model.md` §Connective Scenes |
| **Gravitational context assembly** | "Gravity as retrieval signal" — mass-weighted context selection. | Beautiful idea. Requires a working graph with mass values and a retrieval system to weight. | `gravitational-context-assembly.md` |
| **Cross-graph composition** | Single Cypher query traversing relational web + setting topology + narrative graph. | Requires all three AGE graphs to exist. TAS-244 (AGE spike) hasn't happened. | `cross-graph-composition.md` |
| **Purpose vectors** | 7-dimension weighted composition per scene. Static + runtime analysis. | Enriches the scene model but no scene currently has a purpose vector. The concept is ahead of the data. | `narrative-pacing.md` |
| **Traversal friction** | Communication velocity, sigmoid permeability, temporal friction. | Graph-strategy math that operates on graph structure that doesn't exist yet. | `traversal-friction.md` |
| **Character complexity tiers** | Full/reduced/minimal tensor by role importance. | A performance optimization. Premature until we have performance data. | `open_questions.md` |
| **Sub-graph pacing** | Tales-within-tales may have distinct pacing grammars. | Requires both pacing grammars and sub-graph navigation to exist. Two layers of dependency. | `narrative-pacing.md` |

**What this means**: Zone 3 items are not blocked or broken — they're *correctly deferred*. The risk is not that they're unfinished; it's that their presence in the documentation can make it feel like they need to be solved before you can proceed. They don't.

---

## What the Linear Backlog Implies vs. What's Actually Needed

The 26 open tickets across 11 milestones were written to decompose the roadmap's 7 phases into actionable work. That decomposition was reasonable at the time, but it encoded assumptions:

**What the backlog assumes**:
- That milestones are sequential (Entity Persistence → AGE Foundations → Shared Async → State Machines → Graph Queries → ...)
- That AGE graph work (TAS-244, TAS-245) follows immediately after relational persistence
- That state machine formalization (TAS-248) is its own milestone, separate from scene lifecycle
- That shared async services (TAS-246, TAS-247) are a prerequisite for graph work

**What we actually know now**:
- The within-turn loop works and doesn't need the backlog items to continue functioning
- The persistence schema is broader than what's needed for the next learning step
- AGE graph work is substantial and its value is gated on TAS-244 (Cypher coverage spike) — which hasn't been prioritized because it's not clear it's the next thing that would teach us the most
- The scene lifecycle is the critical unknown, and no ticket currently addresses "make two scenes play in sequence"

**The honest gap**: There is no ticket for "implement scene-to-scene transition." TAS-248 ("Formalize scene and entity lifecycle state machines") gestures at it, but it's framed as formalization of something that doesn't exist yet. TAS-264 ("Build content loader and multi-scene support") is in the last milestone. The thing that would teach us the most about how the system actually works is scattered across the backlog as assumed infrastructure for other work.

---

## What Would Teach Us the Most

The question is not "what's the next milestone?" but "what's the smallest thing we could build that would force real decisions about Zone 2?"

### The Two-Scene Loop

If we could play Scene A, trigger an exit, transition, and enter Scene B — even with hardcoded scenes, simplified exit logic, and no persistence — we would learn:

1. **What data flows at scene boundaries.** The warmed data struct is specified. But building it means deciding what queries run (even against in-memory data), what gets compressed, what the Narrator actually needs.

2. **What triggers scene exit.** The 6 exit conditions are named. Building even one (PlayerInitiated: the player says "I leave") forces decisions about how exit detection works mechanically.

3. **What "next scene selection" means.** Even a hardcoded choice between two exits forces the question: what data does the Storykeeper need to pick the next scene? What does the transition message look like?

4. **What resets and what persists.** Character Agents are ephemeral. TurnContext resets. But the event ledger carries forward. The information state carries forward. Building the boundary forces you to enumerate what crosses it.

5. **Whether the scene model's complexity is right-sized.** The WarmedData struct has ~15 fields. Maybe you need 5 of them for a working transition and the rest are enrichment. You can't know until you try.

### What This Doesn't Require

A two-scene loop does NOT require:
- PostgreSQL persistence (in-memory is fine for learning)
- Apache AGE graphs (no graph queries needed for two hardcoded scenes)
- Shared async services (no multiplexed writes yet)
- Pacing grammar or sparsity analysis (two scenes, no topology to analyze)
- Scene generation (both scenes are authored)
- gRPC server or TUI (play-scene binary is sufficient)

It DOES require:
- A second workshop scene (alongside the existing Flute Kept scene)
- Scene lifecycle states in the engine (at minimum: active, transitioning)
- An exit detection system (even rudimentary — player types "/leave" or scene goal completes)
- A scene entry system (loads new scene data, recomputes what the narrator needs)
- Decisions about what the Storykeeper does at scene boundaries

### Relationship to Existing Tickets

This work cuts across several existing tickets rather than mapping cleanly to one:

| What's Needed | Closest Ticket | Gap |
|---------------|---------------|-----|
| Scene lifecycle states | TAS-248 (State Machine Formalization) | TAS-248 is broader; we need the scene part only |
| Exit detection system | None | Implied by scene-model.md but not ticketed |
| Scene entry system (real, not workshop) | TAS-264 (Content loader + multi-scene) | TAS-264 is in the last milestone; we need the core mechanic now |
| Second workshop scene | None | Could be extracted from TFATD |
| Storykeeper boundary behavior | TAS-243 (current — traits exist) | Traits exist; the boundary logic doesn't |

---

## Recommendations

### Near Term: Ground Zone 2 Through the Two-Scene Loop

The highest-leverage work right now is implementing a minimal scene-to-scene transition in the engine. This teaches more about the system's real behavior than any amount of additional specification.

This doesn't need a new milestone. It needs a focused spike (possibly a single ticket) that says: "Make `play-scene` support transitioning between two authored scenes. Discover what decisions this forces."

### Medium Term: Let the Loop Inform the Backlog

Once a two-scene loop works, several backlog tickets become either:
- **Clearly next** — because the loop revealed what's missing
- **Clearly premature** — because the loop showed they aren't needed yet
- **Clearly wrong** — because the loop forced a different approach than what was assumed

The backlog should be pruned and re-scoped after the loop works, not before.

### Ongoing: Protect Zone 3's Openness

The game design documents (narrative-pacing.md, scene-provenance.md, authoring-economics.md) and graph strategy documents are doing important work by existing as thinking. They don't need to be implemented to be valuable. They inform judgment calls that will happen during Zone 2 work. Resist the urge to ticket them or schedule them — let them be the reference material they are.

---

## Appendix: Current Ticket Inventory by Zone

### Zone 1 Tickets (Grounded — Build Against These)

| Ticket | Status | Notes |
|--------|--------|-------|
| TAS-243 | In Progress | Tier 1 migrations + Storykeeper traits. Nearly complete. |

### Zone 2 Tickets (Design Exists — Need Reality Testing)

| Ticket | Status | What It Gets Right | What's Uncertain |
|--------|--------|-------------------|-----------------|
| TAS-248 | Backlog | Scene + entity lifecycle need formalization | Scope is too broad; scene lifecycle is the critical part |
| TAS-249 | Backlog | Session lifecycle with suspend/resume | Correct but premature — single session works first |
| TAS-246 | Backlog | Event ledger writer as actor service | Pattern is right; timing depends on whether persistence is needed for the loop |
| TAS-244 | Backlog | AGE Cypher coverage spike | Important but not blocking the next learning step |
| TAS-245 | Backlog | Graph schema for relational web + narrative graph | Depends on TAS-244; both are after the two-scene loop |
| TAS-250 | Backlog | Relational web + narrative graph queries | Requires graphs to exist |
| TAS-251 | Backlog | Wire graph retrieval into Tier 3 context | Requires queries to exist |

### Zone 3 Tickets (Ideation — Correctly Deferred)

| Ticket | Status | Why It's Correctly Deferred |
|--------|--------|---------------------------|
| TAS-252 | Backlog | Cross-scene compound events — needs the event DAG running at runtime first |
| TAS-253 | Backlog | Entity-event-graph lookups — needs graph + event data flowing |
| TAS-254 | Backlog | Command sourcing + checkpoints — enrichment on working persistence |
| TAS-255 | Backlog | Ledger replay — requires the ledger to exist and be populated |
| TAS-256 | Backlog | TUI scaffold — play-scene binary is sufficient for now |
| TAS-257 | Backlog | TUI observability panels — after TUI exists |
| TAS-258 | Backlog | Session dump schema — after sessions persist |
| TAS-259 | Backlog | Import-to-state injection — after dumps exist |
| TAS-260 | Backlog | Proto file + storyteller-proto crate — API contract is premature |
| TAS-261 | Backlog | WebSocket/SSE strategy — deployment decisions are future |
| TAS-262 | Backlog | gRPC server — after the engine loop is solid |
| TAS-263 | Backlog | Scene + character authoring formats — after we know what scenes need |
| TAS-264 | Backlog | Content loader + multi-scene — the core mechanic is needed now, the full system later |
| TAS-274 | Backlog | Character study: Bartender — creative work, do when it's useful |
| TAS-275 | Backlog | Chapters + transition points — design thinking, correctly open |

### Untracked Work (Zone 2 Gaps)

| Need | Why It's Not Ticketed | Priority |
|------|----------------------|----------|
| Scene-to-scene transition in engine | Falls between TAS-248 and TAS-264; assumed as infrastructure | **High — this is the next learning step** |
| Second workshop scene | Test data; not considered a deliverable | High (enables the loop) |
| Exit detection system | Implied by scene-model.md but never decomposed into work | High (part of the loop) |
| Storykeeper boundary logic | Traits exist (TAS-243); what happens at boundaries doesn't | High (discovered during the loop) |

---

*This assessment is a snapshot. It should be revisited after the two-scene loop is implemented — many Zone 2 items will move to Zone 1 or reveal new unknowns.*
