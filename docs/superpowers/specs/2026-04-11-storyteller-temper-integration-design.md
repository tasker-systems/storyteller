# Storyteller + Temper Integration Design

**Date:** 2026-04-11
**Task:** 2026-04-11-storyteller-and-temper-together
**Context:** storyteller
**Mode/Effort:** plan/medium

## Summary

Design for how storyteller leverages temper as its narrative memory and authored prose
layer. Temper serves two roles: (1) persistence for authored character sketches, world
notes, and authorial decisions during world-building, and (2) persistence and semantic
retrieval for rendered scene prose during play. Storyteller's own PostgreSQL + AGE database
remains the authority for structured data (genre, Tome, Lore, relational graph, event
ledger, state variables). Temper provides the searchable texture layer — the prose that
gives structured data its voice.

## Architectural Principle

**Storyteller owns the geological stack. Temper owns the prose.**

- Genre data (bedrock), Tome data (sediment), and Lore data (topsoil) live in
  storyteller's database as structured, typed records queryable by SQL and Cypher.
- Authored prose (character sketches, world descriptions, authorial decisions) and
  rendered narrative (scene output) live in temper as chunked, embedded, graph-connected
  markdown resources queryable by semantic search and graph traversal.
- The `storyteller-temper` crate is the bridge — it encodes the domain mapping between
  storyteller concepts and temper resources, ensuring both stores are written
  consistently.

Temper never touches the turn-loop critical path. The Storykeeper's contract —
deterministic, bounded, 10-50ms context assembly from structured queries — is preserved
exactly as designed.

## Approach: Temper as Narrative Memory with Domain Awareness

Temper remains a generic knowledge base. All storyteller-specific domain knowledge lives
in the `storyteller-temper` integration crate within storyteller's codebase. Temper's
existing primitives (contexts, doc types, open_meta, knowledge graph edges, combined
search) are flexible enough to carry storyteller's concepts without modification.

### Why not deeper integration

Three alternatives were considered:

- **Approach A (Minimal):** Temper as a dumb document store with search. No knowledge
  graph edges, no domain-aware queries. Simple but loses the graph-expansion retrieval
  that makes "find everything connected to Margaret" possible.

- **Approach B (Recommended, chosen):** Temper as narrative memory with domain awareness
  encoded in `storyteller-temper`. Knowledge graph edges are systematically maintained.
  Combined vector + graph search enables rich narrative retrieval. Temper stays generic.

- **Approach C (Deep):** Temper gains storyteller-specific doc types, edge types, and
  search modes. Cleanest API but couples temper to storyteller — violating temper's
  role as a general-purpose knowledge base and requiring coordinated changes across
  repos for every domain evolution.

## Resource Mapping Convention

Storyteller concepts map to temper's existing doc types via an `open_meta` convention
enforced by the `storyteller-temper` crate. The `storyteller-type` field is the
discriminator.

| Storyteller Concept | Temper Doc Type | open_meta fields | Knowledge Graph Edges |
|---|---|---|---|
| Character sketch | `concept` | `storyteller-type: character`, `character-slug`, `world-slug`, `archetype-ref` | `relates_to` place concepts, other character concepts |
| Place/world description | `concept` | `storyteller-type: place`, `place-slug`, `world-slug` | `relates_to` character concepts set there |
| World-building note | `concept` | `storyteller-type: world-note`, `world-slug`, `domain` (material, economic, political, social, historical, aesthetic) | `relates_to` relevant concepts |
| Authorial decision | `decision` | `storyteller-type: lore-decision`, `world-slug` | `references` the concepts it resolves |
| Story arc | `goal` | `storyteller-type: story`, `world-slug`, `genre-slug` | `parent_of` scene tasks |
| Scene (planned) | `task` | `storyteller-type: scene`, `scene-slug`, `world-slug` | `depends_on` predecessor scenes, `relates_to` character/place concepts |
| Rendered scene (play output) | `session` | `storyteller-type: rendered-scene`, `scene-slug`, `world-slug`, `turn-count` | `preceded_by` prior rendered scene, `relates_to` character/place concepts, `derived_from` planned scene task |
| Authoring session | `session` | `storyteller-type: authoring-session`, `world-slug` | `references` concepts touched during session |
| Tome/Lore reference | `research` | `storyteller-type: tome-reference` or `lore-reference`, `genre-slug`, `domain` | `extends` related research |

Each temper context maps to one story. Temper's cross-context search allows authors
working on multiple stories to discover connections without polluting individual story
contexts.

## Crate Architecture

### storyteller-temper

Systems-level bridge between storyteller and temper. Depends on `temper-client` and
`storyteller-core`. No Bevy dependency — headless integration layer.

**Responsibilities:**

- **Resource builders:** Construct temper resources with correct doc type, open_meta,
  and content from storyteller domain types.
- **Edge builders:** Construct knowledge graph edges when resources are created (e.g.,
  persisting a rendered scene automatically creates `preceded_by`, `relates_to`
  character, `derived_from` scene task edges).
- **Query helpers:** Compose temper search calls with storyteller-meaningful filters
  (search by `storyteller-type`, filter by `world-slug`, scope by character
  connectivity via graph expansion).
- **Context management:** Create/resolve temper contexts for stories.
- **Credential forwarding:** Handle temper-client auth configuration.

**Public API shape:**

```
StorytellerTemper
  .contexts()     -> create/resolve story contexts
  .characters()   -> persist/search character sketches
  .places()       -> persist/search place descriptions
  .scenes()       -> persist planned scenes, persist rendered scenes
  .decisions()    -> persist authorial decisions
  .world_notes()  -> persist world-building notes
  .search()       -> narrative-aware search (vector + graph)
  .reference()    -> persist/retrieve tome and lore references
```

Each sub-API maps storyteller domain types to temper resources and back.

### storyteller-author

Authoring experience crate. Depends on `storyteller-temper` and `storyteller-core`.

**Responsibilities:**

- World-building agent implementation and MCP tool surface
- Co-creation workflows: guide the author through Tome-informed world-building,
  producing dual output (structured -> storyteller DB, prose -> temper)
- Continuity checking: search temper during co-creation for existing authored content
- Lore instantiation: copy-on-write from Tome, creating story-specific Lore records
  in storyteller's DB while capturing authorial prose in temper

**Deferred:** The full authoring agent design (personality, workflow guidance, skill set)
is a separate, multi-session design effort. This spec establishes the crate's role and
data access patterns; the agent's behavior will be designed separately, grounded in real
experience with the tools.

### Dependency graph

```
storyteller-cli --> storyteller-api --> storyteller-engine --> storyteller-core
                                             |                       ^
                                             +-- storyteller-temper --+
                                             |        ^
storyteller-author ---------------------------------------+
      |
      +--> storyteller-core
```

## Data Flow: Authoring Phase

### Story initialization

1. Author starts a new story via the world-building agent.
2. `storyteller-temper` creates a temper context for the story.
3. `storyteller-author` creates the story's Lore layer in storyteller's DB —
   copy-on-write from the appropriate Tome data (genre + world-position select which
   Tome records to fork).
4. A `goal` resource is created in temper representing the story arc.

### Character co-creation (example)

1. Author: "I want to work on the Warden character for this village."
2. World-building agent retrieves Tome data from storyteller's DB: the Earnest Warden
   archetype, axis positions, tensions, genre variant data.
3. Agent presents this as creative grounding: "The genre grammar says the Warden's
   warmth is genuine and their duty demands sacrifice. Tell me about this person."
4. Author: "She's a dairy farmer named Margaret. Third generation on the land..."
5. Agent and author work through the character — agent checks Tome constraints, author
   provides specificity.
6. **Dual write:**
   - Structured: `storyteller-author` writes a Lore character record to storyteller's
     DB — archetype IDs, personal variance, initial state variables, graph edges.
   - Prose: `storyteller-temper` persists a `concept` resource to temper with the full
     character sketch plus open_meta linking to the character slug and world.
   - Edges: `storyteller-temper` creates `relates_to` edges connecting the character
     concept to place concepts, other character concepts.

### Continuity during authoring

When the author works on new content, the world-building agent searches temper for
existing material in the story's context. Temper's combined vector + graph search
enables queries like "what have we established about the village economy?" — semantic
similarity finds relevant prose, graph traversal expands to connected concepts.

## Data Flow: Play Phase

### Scene entry

1. Engine signals scene entry with scene ID, cast list, setting.
2. **Storykeeper** (synchronous, critical path): queries storyteller's DB — character
   records, archetype composition, goals, state variables, relational graph via AGE.
   Assembles the structured context packet. No temper involvement. Deterministic,
   10-50ms.
3. **Dramaturge** (async, parallel): `storyteller-temper` searches for relevant
   narrative history — vector search for semantic relevance, graph expansion for
   connected resources. Results are written to a scene-entry cache structure that
   context assembly reads from after a configured timeout.
4. Context assembly reads the cache: deterministic results (always present) plus
   whatever temper results arrived within the timeout window. Graceful degradation if
   temper results are slow or unavailable.

**Scene entry cache:** Multiple async producers (Storykeeper DB queries, dramaturge
temper search) populate a structured cache in parallel. Assembly reads from the cache
after a timeout, taking whatever is available. The cache mechanism (Dragonfly,
in-process concurrent map, Bevy resource) is an implementation choice deferred to the
engine's scene-entry design work. The principle is: scene entry can afford latency that
the turn loop cannot, and first-render quality benefits from maximum context richness.

### Turn loop

Temper is not on the turn-loop critical path. The Storykeeper assembles context packets
from its own DB every turn. The dramaturge's directive (informed by temper at scene
entry) persists across turns. The event ledger, graph delta updates, state variable
nudges — all within storyteller's DB.

### Scene exit

1. Engine signals scene exit.
2. `storyteller-temper` persists the rendered scene as a `session` resource:
   - Content: full rendered prose of the scene (batched, not per-turn)
   - open_meta: scene metadata, cast list, turn count
   - Edges: `preceded_by` prior rendered scene, `relates_to` character/place concepts,
     `derived_from` planned scene task
3. Persistence is fire-and-forget, out of band from the turn loop.
4. Temper's ingest pipeline chunks and embeds the prose, making it searchable for
   future scene entries.

### Latency budget

| Step | Temper involved? | Blocking? |
|---|---|---|
| Storykeeper context assembly | No | Yes (10-50ms, unchanged) |
| Dramaturge search at scene entry | Yes (vector + graph) | No (async, timeout-gated) |
| Per-turn context assembly | No | Yes (unchanged) |
| Dramaturge per-turn update | Optionally | No (async) |
| Scene exit persistence | Yes (write) | No (fire-and-forget) |

## Boundary: What Stays in Storyteller

| Concern | Location | Rationale |
|---|---|---|
| Genre data (bedrock) | Storyteller DB | Structured, typed, deterministic queries |
| Tome data (sediment) | Storyteller DB | Elicited world-position data, structured records |
| Lore data (topsoil) | Storyteller DB | Copy-on-write from Tome, instanced per-story, graph edges |
| Relational graph (AGE) | Storyteller DB | Character dynamics, narrative topology, runtime state |
| Event ledger | Storyteller DB | Append-only ground truth, command sourcing |
| State variables + deltas | Storyteller DB | Per-turn nudges, narrative gravity accumulation |
| Narrative beat recognition | Storyteller engine | Dramaturge pattern-matches structured data |
| Turn-loop context assembly | Storyteller engine | Storykeeper's deterministic pipeline |

## Boundary: What Lives in Temper

| Concern | Location | Rationale |
|---|---|---|
| Authored prose | Temper | Unstructured, rich, benefits from semantic search |
| Rendered scene prose | Temper | Narrative output, searchable for "what happened" |
| Cross-resource knowledge graph | Temper | Resource connectivity for discovery and continuity |
| Narrative memory for scene entry | Temper | Dramaturge's semantic retrieval source |
| Authoring session records | Temper | Continuity for multi-session world-building |

## Dependencies on Temper

| Requirement | Temper status | Notes |
|---|---|---|
| Create/manage contexts | Exists | No gap |
| Create resources with open_meta | Exists | No gap |
| Semantic search within context | Exists | No gap |
| Knowledge graph edges | R7 — in progress | Blocks graph-expansion queries; degrades gracefully to pure semantic search |
| Combined vector + graph search | R7 — in progress | Same dependency |
| open_meta field filtering in search | Planned | Task created: `2026-04-11-open-meta-field-filtering-in-search` in temper context |
| temper-client as dependency | Needs crates.io publish or path ref | Planned temper task exists |
| Rust client write + search | Exists | No gap |

## Open Questions (Deferred)

### World-building agent design
The `storyteller-author` crate needs an agent personality, skill set, and workflow
guidance — how it presents Tome constraints, handles authorial divergence, manages
dual-write. This is a separate multi-session design effort, best grounded in real
experience with the tools.

### Scene entry cache mechanism
Timeout-gated async assembly with parallel producers is the agreed pattern. The
concrete mechanism (Dragonfly, in-process map, Bevy resource) is deferred to engine
scene-entry design work. The `storyteller-temper` interface needs to write to whatever
cache structure is chosen.

### Temper auth for engine runtime
The engine needs credentials to authenticate with temper's API during play. Likely a
provisioned OAuth machine-to-machine flow through Auth0, consistent with temper's
existing auth patterns. Known-unknown for now; temper will need service account support
regardless.

### Content granularity
Rendered scenes are persisted per-scene (batched turns), not per-turn. This matches the
"scene as unit" principle and keeps the resource count manageable. Per-turn granularity
can be added later if finer-grained search proves necessary.

### Prose mutability
Authored prose in temper is write-once from storyteller's perspective. If an author
updates a character sketch, that's fine — it may influence the story on retrieval for
playtesting — but storyteller does not sync changes back to temper. Narrative evolution
is captured in new rendered scene resources, not by mutating old ones.
