# PostgreSQL Schema Design

## Purpose

This document defines the relational database schema for the storyteller engine's persistent state. It covers core entity identity tables, the append-only event ledger, turn history, session lifecycle, and narrative sub-graph layers. Graph data (relational web, narrative graph, setting topology) lives in Apache AGE and is designed separately (TAS-244, TAS-245).

The schema exists to support the Storykeeper trait interfaces defined in `storykeeper-api-contract.md`. Every table maps to one or more domain operations in that contract.

### Design Principles

1. **UUIDv7 everywhere.** All primary keys are UUIDv7, providing monotonic insertion order, embedded timestamps, and BTree-friendly storage. PostgreSQL 18 provides native `uuidv7()`.

2. **Provenance by default.** Every record traces back to its origin: `story_id`, `scene_id`, `turn_id`. Denormalized where query efficiency demands it (the event ledger includes `story_id` even though it's derivable from `turn_id → session_id → story_id`).

3. **Columns for identity and lifecycle, JSONB for domain payload.** ID fields, foreign keys, timestamps, enum discriminators, and frequently-filtered scalars are proper columns. Complex nested structures (tensors, predictions, event atoms, relational substrates) are JSONB. This reflects how the Rust types are structured — simple wrapper types for IDs, deeply nested structs for domain data.

4. **Event ledger is immutable.** No UPDATE or DELETE on `event_ledger` rows. The ledger is the system's memory — events are facts about what happened, not mutable state.

5. **Tales-within-tales support.** A `layer_id` column on entities, events, and scenes enables sub-graph queries without a separate table hierarchy. `NULL` means the parent (root) narrative.

6. **PostgreSQL ENUM types for domain discriminators.** All columns that map to Rust enums use PostgreSQL ENUM types, providing database-level type safety and implicit ordinality (enums sort by declaration order). Mapped to Rust via `#[derive(sqlx::Type)]`.

### Relationship to Other Documents

- **`storykeeper-api-contract.md`** — Defines the domain operations this schema supports.
- **`knowledge-graph-domain-model.md`** — Defines the four graph structures. This schema covers the relational foundation; AGE covers the graph data.
- **`tales-within-tales.md`** — Defines nested narrative layers. The `sub_graph_layers` table and `layer_id` columns implement this.
- **`gravitational-context-assembly.md`** — Defines retrieval queries. The schema's indexing strategy supports these access patterns.
- **`infrastructure-architecture.md`** — Defines the data lifecycle and durability model.

---

## Schema Overview

```
stories ──────────────────────────────────┐
  │                                        │
  ├── settings                             │
  │     │                                  │
  ├── scenes (templates) ── sub_graph_layers
  │     │                                  │
  ├── characters (versioned)               │
  │                                        │
  ├── entities ◄──── event_ledger          │
  │     │                  │               │
  └── sessions             │               │
        │                  │               │
        ├── scene_instances (scene × session × POV)
        │     │            │               │
        └─────┴── turns ───┘               │
                    │                      │
                    └── (FK chain) ────────┘
```

---

## Enum Types

All enum types are created before any tables. Variants are declared in semantic order — for ordinal types (priority, promotion_tier, provisional_status), PostgreSQL sorts by declaration order, so `ORDER BY priority` naturally sorts `immediate` first.

```sql
-- Ordinal: sorts immediate → deferred
CREATE TYPE event_priority AS ENUM (
    'immediate', 'high', 'normal', 'low', 'deferred'
);

-- Discriminator: atom vs compound event
CREATE TYPE event_type AS ENUM (
    'atom', 'compound'
);

-- Discriminator: what kind of atomic event (extensible)
CREATE TYPE event_kind AS ENUM (
    'state_assertion', 'action_occurrence', 'spatial_change',
    'relational_shift', 'information_transfer', 'unknown'
);

-- Ordinal state machine: hypothesized → rendered → committed
CREATE TYPE provisional_status AS ENUM (
    'hypothesized', 'rendered', 'committed'
);

-- Discriminator: scene gravitational classification
CREATE TYPE scene_type AS ENUM (
    'gravitational', 'connective', 'gate', 'threshold'
);

-- Discriminator: how an entity entered the narrative
CREATE TYPE entity_origin AS ENUM (
    'authored', 'promoted', 'generated'
);

-- Discriminator: entity lifecycle scope
CREATE TYPE persistence_mode AS ENUM (
    'permanent', 'scene_local', 'ephemeral'
);

-- Ordinal: promotion through lifecycle tiers
CREATE TYPE promotion_tier AS ENUM (
    'unmentioned', 'mentioned', 'referenced', 'tracked', 'persistent'
);

-- Ordinal state machine: session lifecycle
CREATE TYPE session_status AS ENUM (
    'created', 'active', 'suspended', 'ended'
);

-- State machine: scene instance lifecycle
CREATE TYPE scene_instance_status AS ENUM (
    'active', 'completed', 'abandoned'
);

-- Discriminator: sub-graph narrative layer type
CREATE TYPE layer_type AS ENUM (
    'memory', 'dream', 'fairy_tale', 'parallel_pov', 'embedded_text', 'epistle'
);
```

**Rust mapping:** Each enum maps to a Rust enum via `#[derive(sqlx::Type)]` with `#[sqlx(type_name = "...", rename_all = "snake_case")]`. In `query_as!` macros, columns are annotated as `column_name as "column_name: RustEnumType"` for deserialization.

---

## Table Definitions

### stories

Top-level container. A story is an authored narrative world — "The Fair and the Dead," "Bramblehoof's Misadventure," etc. All other tables are scoped to a story.

```sql
CREATE TABLE stories (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    title       TEXT NOT NULL,
    description TEXT,
    metadata    JSONB,           -- extensible story-level configuration
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**Domain mapping:** Stories don't appear explicitly in the current Rust types — the workshop hardcodes a single story. This table is the anchor that makes multi-story support possible.

### settings

Spatial locations that scenes take place in. Settings are reusable — the same farmhouse can appear in multiple scenes. The setting topology graph (AGE) references these IDs as vertices.

```sql
CREATE TABLE settings (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id    UUID NOT NULL REFERENCES stories(id),
    name        TEXT NOT NULL,
    description TEXT NOT NULL,
    spatial_data JSONB,          -- affordances, sensory_details, aesthetic_detail
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_settings_story ON settings(story_id);
```

**Domain mapping:** Maps to `SceneSetting` in `character.rs`. The `spatial_data` JSONB holds `affordances`, `sensory_details`, and `aesthetic_detail` fields.

### sub_graph_layers

Narrative sub-graph layers for tales-within-tales. A layer represents a nested narrative context — a memory, dream, fairy tale, parallel POV, or embedded text. Layers can nest (a dream within a memory).

```sql
CREATE TABLE sub_graph_layers (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    parent_layer_id UUID REFERENCES sub_graph_layers(id),  -- NULL = root narrative
    name            TEXT NOT NULL,
    layer_type      layer_type NOT NULL,
    entry_scene_id  UUID,            -- FK added after scenes table exists
    permeability    REAL NOT NULL DEFAULT 0.0,  -- 0.0 (opaque) to 1.0 (merged)
    metadata        JSONB,           -- type-specific data (boundary drivers, convergence state)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sub_graph_layers_story ON sub_graph_layers(story_id);
CREATE INDEX idx_sub_graph_layers_parent ON sub_graph_layers(parent_layer_id);
```

**Domain mapping:** Maps to `SubGraphLayer`, `SubGraphType`, `BoundaryPermeability` in `tales-within-tales.md`. The `permeability` field tracks the current boundary state (0.0 = established, 1.0 = merged).

### scenes

Scene templates. A scene defines bounded creative constraints — cast, setting, stakes, and gravitational mass. Scenes belong to a story and optionally to a sub-graph layer. Scenes are reusable definitions; specific playthroughs are tracked in `scene_instances`.

```sql
CREATE TABLE scenes (
    id             UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id       UUID NOT NULL REFERENCES stories(id),
    setting_id     UUID REFERENCES settings(id),
    layer_id       UUID REFERENCES sub_graph_layers(id),  -- NULL = root narrative
    title          TEXT NOT NULL,
    scene_type     scene_type NOT NULL,
    narrative_mass JSONB NOT NULL,   -- { authored_base, structural_modifier, dynamic_adjustment }
    scene_data     JSONB NOT NULL,   -- cast, stakes, constraints, emotional_arc, evaluation_criteria
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add deferred FK for sub_graph_layers.entry_scene_id
ALTER TABLE sub_graph_layers
    ADD CONSTRAINT fk_entry_scene FOREIGN KEY (entry_scene_id) REFERENCES scenes(id);

CREATE INDEX idx_scenes_story ON scenes(story_id);
CREATE INDEX idx_scenes_setting ON scenes(setting_id);
CREATE INDEX idx_scenes_layer ON scenes(layer_id);
```

**Domain mapping:** Maps to `SceneData` in `character.rs` and `NarrativeMass` in `narrative.rs`. The `scene_type` column maps to the `SceneType` enum. Cast information is embedded in `scene_data` JSONB rather than a junction table — cast is always loaded as a unit with the scene.

### scene_instances

A specific playthrough of a scene within a session. While `scenes` defines the template (cast, setting, stakes), `scene_instances` tracks a particular activation — which player is present, which character they embody (POV), and whether this is a first visit or a re-entry.

A scene can be instantiated multiple times within a session: re-entering a location, experiencing the same scene from a different character's POV, or returning after new information changes the stakes.

```sql
CREATE TABLE scene_instances (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),
    scene_id         UUID NOT NULL REFERENCES scenes(id),
    session_id       UUID NOT NULL REFERENCES sessions(id),
    player_entity_id UUID NOT NULL REFERENCES entities(id),  -- POV character for this instance
    instance_number  INT NOT NULL,
    status           scene_instance_status NOT NULL DEFAULT 'active',
    entry_conditions JSONB,          -- contextual state at entry: prior knowledge, active tensions
    entered_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    exited_at        TIMESTAMPTZ,
    UNIQUE (session_id, scene_id, instance_number)
);

-- Add deferred FK for sessions.current_scene_instance_id
ALTER TABLE sessions
    ADD CONSTRAINT fk_current_scene_instance FOREIGN KEY (current_scene_instance_id) REFERENCES scene_instances(id);

CREATE INDEX idx_scene_instances_session ON scene_instances(session_id);
CREATE INDEX idx_scene_instances_scene ON scene_instances(scene_id);
CREATE INDEX idx_scene_instances_active ON scene_instances(session_id, status) WHERE status = 'active';
```

**Domain mapping:** Bridges the scene template model and the session lifecycle. The `player_entity_id` establishes which character the player embodies for the duration of this instance — POV is fixed at scene entry and cannot change mid-scene. This simplifies information boundary enforcement: the Storykeeper shapes context based on a single, stable POV character throughout the scene.

**Instance numbering:** `(session_id, scene_id, instance_number)` is unique. Instance 1 is the first entry, instance 2 is the first re-entry, etc. This supports the setting topology model where locations are re-enterable.

**Entry conditions:** The `entry_conditions` JSONB captures the contextual state at the moment of entry — what the POV character knows, which tensions are active, what graph state was loaded. This enables replay and debugging: "what did the system know when it started this scene?"

### characters

Character sheets with versioned tensors. Each row is a snapshot of a character at a point in time. The tensor evolves across scenes — when the Storykeeper commits relational changes at scene exit, a new version is created.

```sql
CREATE TABLE characters (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),  -- version ID, not entity_id
    entity_id   UUID NOT NULL,       -- the character's stable identity (FK → entities)
    story_id    UUID NOT NULL REFERENCES stories(id),
    name        TEXT NOT NULL,
    version     INT NOT NULL,
    sheet       JSONB NOT NULL,      -- full CharacterSheet: tensor, emotional_state, self_edge,
                                     -- triggers, voice, backstory, performance_notes, capabilities
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (entity_id, story_id, version)
);

CREATE INDEX idx_characters_entity ON characters(entity_id);
CREATE INDEX idx_characters_story ON characters(story_id);
-- Composite: latest version lookup for load_cast lateral join
CREATE INDEX idx_characters_entity_version ON characters(entity_id, version DESC);
```

**Domain mapping:** Maps to `CharacterSheet` and `CharacterTensor` in `character.rs`. The `sheet` JSONB is the full `CharacterSheet` struct serialized via serde. The `entity_id` links to the `entities` table but is not a FK because characters may be authored before the entity lifecycle tracking creates the entity row.

**Versioning rationale:** The storyteller system treats character evolution as a geological process — tensors have temporal layers (topsoil, sediment, bedrock). Explicit versioning makes it possible to reconstruct a character's state at any past scene, which is essential for replay and debugging.

### players

Player identity. Minimal for now — the system needs to track who is playing but doesn't need rich player profiles yet.

```sql
CREATE TABLE players (
    id           UUID PRIMARY KEY DEFAULT uuidv7(),
    display_name TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### sessions

Session lifecycle. A session is a player's engagement with a story — from creation through active play, possible suspension, and eventual end. A session tracks which scene instance is currently active, providing the current POV implicitly via the scene instance.

```sql
CREATE TABLE sessions (
    id                         UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id                   UUID NOT NULL REFERENCES stories(id),
    player_id                  UUID NOT NULL REFERENCES players(id),
    current_scene_instance_id  UUID,  -- FK added after scene_instances table exists
    status                     session_status NOT NULL DEFAULT 'created',
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at                   TIMESTAMPTZ
);

CREATE INDEX idx_sessions_story ON sessions(story_id);
CREATE INDEX idx_sessions_player ON sessions(player_id);
CREATE INDEX idx_sessions_status ON sessions(status) WHERE status != 'ended';
```

**Domain mapping:** The session lifecycle state machine (Created → Active → Suspended → Resumed → Ended) is tracked by the `status` column. The partial index on active sessions avoids scanning ended sessions for active-session queries. The `current_scene_instance_id` points to a `scene_instances` row, which in turn provides the scene template, POV character, and instance number — no separate POV tracking is needed on the session itself.

### turns

Turn records. A turn is the atomic unit of play — player input triggers the pipeline, the narrator renders a response, and the turn is eventually committed when the player sends their next input. Turns belong to a scene instance, not directly to a scene template.

```sql
CREATE TABLE turns (
    id                       UUID PRIMARY KEY DEFAULT uuidv7(),  -- this IS the TurnId
    session_id               UUID NOT NULL REFERENCES sessions(id),
    scene_instance_id        UUID NOT NULL REFERENCES scene_instances(id),
    turn_number              INT NOT NULL,
    player_input             TEXT NOT NULL,
    narrator_rendering       TEXT,          -- NULL until rendered
    classification           JSONB,         -- ClassificationOutput from input classification
    committed_classification JSONB,         -- ClassificationOutput from committed-turn classification
    predictions              JSONB,         -- Vec<CharacterPrediction>
    resolver_output          JSONB,         -- ResolverOutput
    provisional_status       provisional_status NOT NULL DEFAULT 'hypothesized',
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    rendered_at              TIMESTAMPTZ,
    committed_at             TIMESTAMPTZ
);

CREATE INDEX idx_turns_session ON turns(session_id);
CREATE INDEX idx_turns_scene_instance ON turns(scene_instance_id);
-- Composite: turns within an instance, ordered
CREATE INDEX idx_turns_instance_order ON turns(scene_instance_id, turn_number);
CREATE UNIQUE INDEX idx_turns_session_number ON turns(session_id, turn_number);
```

**Domain mapping:** Maps directly to `CompletedTurn` in `turn.rs` and `Turn` in `event_grammar.rs`. The `provisional_status` column tracks the `ProvisionalStatus` enum (Hypothesized → Rendered → Committed). JSONB columns hold the complex pipeline outputs that are useful for debugging and replay but not filtered on directly.

**Turn number uniqueness:** `(session_id, turn_number)` is unique — within a session, turn numbers are sequential across all scene instances. This supports the command sourcing guarantee: player input is written to the turns table (with `provisional_status = 'hypothesized'`) before pipeline processing begins. The scene template can be derived via `scene_instance_id → scene_instances.scene_id` when needed.

### event_ledger

The system's memory. Append-only record of every committed event — the atomic facts about what happened in the story. This table is the source of truth for entity weight computation, relational cascade, composition detection, and cross-scene event queries.

```sql
CREATE TABLE event_ledger (
    id                      UUID PRIMARY KEY DEFAULT uuidv7(),  -- this IS the EventId
    story_id                UUID NOT NULL REFERENCES stories(id),
    session_id              UUID NOT NULL REFERENCES sessions(id),
    scene_id                UUID NOT NULL REFERENCES scenes(id),       -- denormalized from scene_instance
    scene_instance_id       UUID REFERENCES scene_instances(id),       -- NULL for system-generated events
    turn_id                 UUID REFERENCES turns(id),                 -- NULL for system-generated events
    layer_id                UUID REFERENCES sub_graph_layers(id),      -- NULL = root narrative
    event_type              event_type NOT NULL,
    event_kind              event_kind NOT NULL DEFAULT 'unknown',
    priority                event_priority NOT NULL,
    participants            JSONB NOT NULL DEFAULT '[]',   -- Vec<Participant>
    relational_implications JSONB NOT NULL DEFAULT '[]',   -- Vec<RelationalImplication>
    source                  JSONB NOT NULL,  -- EventSource (provenance: PlayerInput, TurnExtraction, etc.)
    confidence              JSONB NOT NULL,  -- EventConfidence with evidence
    payload                 JSONB NOT NULL,  -- full EventAtom or CompoundEvent
    committed_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Single-column indexes for simple lookups
CREATE INDEX idx_event_ledger_story ON event_ledger(story_id);
CREATE INDEX idx_event_ledger_turn ON event_ledger(turn_id);

-- Composite indexes for Storykeeper access patterns
CREATE INDEX idx_event_ledger_scene_time ON event_ledger(scene_id, committed_at);
CREATE INDEX idx_event_ledger_instance_time ON event_ledger(scene_instance_id, committed_at);
CREATE INDEX idx_event_ledger_kind_time ON event_ledger(event_kind, committed_at)
    WHERE event_type = 'atom';

-- GIN index for entity participation queries: "all events involving entity X"
CREATE INDEX idx_event_ledger_participants ON event_ledger USING GIN (participants jsonb_path_ops);
```

**Domain mapping:** Maps to `EventAtom` and `CompoundEvent` in `event_grammar.rs`. The `event_type` discriminator separates atoms from compounds. The `payload` JSONB holds the complete serialized event, while frequently-queried fields (`event_kind`, `priority`, `participants`) are extracted as indexed columns or GIN-indexed JSONB.

**Immutability:** The event ledger is append-only by application convention. No `updated_at` column exists — events are never modified after commitment. A database trigger could enforce this but is deferred to keep the schema simple during early development.

**Denormalization:** `story_id`, `session_id`, and `scene_id` are denormalized. `scene_id` is derivable from `scene_instance_id → scene_instances.scene_id` but is denormalized because "all events in this scene template" is a common cross-instance query (e.g., "what has ever happened at the farmhouse across all visits?"). `story_id` and `session_id` are similarly denormalized to eliminate join chains.

### entities

Entity lifecycle tracking. Entities are anything the narrative mentions — characters, objects, locations, concepts. The entity table tracks their promotion through the lifecycle tiers and their accumulated narrative weight.

```sql
CREATE TABLE entities (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),  -- this IS the EntityId
    story_id         UUID NOT NULL REFERENCES stories(id),
    name             TEXT NOT NULL,
    entity_origin    entity_origin NOT NULL,
    persistence_mode persistence_mode NOT NULL,
    promotion_tier   promotion_tier NOT NULL DEFAULT 'unmentioned',
    relational_weight REAL NOT NULL DEFAULT 0.0,
    event_count       INT NOT NULL DEFAULT 0,
    first_seen_turn_id UUID,  -- FK deferred: turns may not exist yet
    last_seen_turn_id  UUID,
    layer_id           UUID REFERENCES sub_graph_layers(id),  -- NULL = root narrative
    metadata           JSONB,          -- entity-specific data (EntityRef context, unresolved mentions)
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_entities_story ON entities(story_id);
CREATE INDEX idx_entities_promotion ON entities(story_id, promotion_tier);
CREATE INDEX idx_entities_layer ON entities(layer_id);
CREATE INDEX idx_entities_name ON entities(story_id, name);
-- Composite: top entities by weight for relevance queries
CREATE INDEX idx_entities_weight ON entities(story_id, relational_weight DESC);
```

**Domain mapping:** Maps to `EntityId`, `EntityOrigin`, `PersistenceMode`, `PromotionTier`, and `RelationalWeight` in `entity.rs`. The `promotion_tier` is a lifecycle state that the entity promotion system updates as entities accumulate narrative weight.

**Weight computation:** `relational_weight` and `event_count` are derived from the event ledger. They are stored here as materialized summaries for efficient querying (the Storykeeper's `query_entity_relevance` operation). The event ledger remains the source of truth.

---

## Access Patterns

The schema is designed to support the Storykeeper's domain operations efficiently. Here are the primary access patterns and how they map to the schema:

### Scene Entry (Phase 1 — "Prepare the World")

| Storykeeper Operation | Query Pattern |
|---|---|
| `create_scene_instance` | `INSERT INTO scene_instances (scene_id, session_id, player_entity_id, instance_number) VALUES (...)` |
| `load_scene` | `SELECT * FROM scenes WHERE id = $1` (via `scene_instances.scene_id`) |
| `load_cast` | `SELECT * FROM characters WHERE entity_id = ANY($1) AND story_id = $2 ORDER BY version DESC LIMIT 1` per entity (or lateral join) |
| `load_entity_histories` | `SELECT * FROM event_ledger WHERE scene_id = $1 AND participants @> $2 ORDER BY committed_at DESC LIMIT $3` |
| `load_information_state` | Derived from event ledger (revelation events) + entity metadata, filtered by POV character |
| `update_session` | `UPDATE sessions SET current_scene_instance_id = $1, status = 'active', updated_at = now() WHERE id = $2` |

### Active Play — Reads (Phase 2)

| Storykeeper Operation | Query Pattern |
|---|---|
| `source_command` | `INSERT INTO turns (session_id, scene_instance_id, turn_number, player_input, provisional_status) VALUES (...)` |
| `query_entity_relevance` | `SELECT * FROM entities WHERE id = $1` + event ledger subquery |
| `assemble_narrator_context` | Composite: scene template + scene instance POV + cast + recent events + entity weights |

### Active Play — Writes (Phase 3)

| Storykeeper Operation | Query Pattern |
|---|---|
| `commit_turn` | `UPDATE turns SET provisional_status = 'committed', committed_at = now(), ... WHERE id = $1` |
| `append_events` | `INSERT INTO event_ledger (...) VALUES (...), (...), ...` (batch insert, includes `scene_instance_id` and denormalized `scene_id`) |
| `update_entity_weights` | `UPDATE entities SET relational_weight = $1, event_count = $2, ... WHERE id = $3` |

### Scene Exit (Phase 4)

| Storykeeper Operation | Query Pattern |
|---|---|
| `snapshot_character` | `INSERT INTO characters (entity_id, story_id, name, version, sheet) VALUES (...)` |
| `complete_scene_instance` | `UPDATE scene_instances SET status = 'completed', exited_at = now() WHERE id = $1` |
| `update_session` | `UPDATE sessions SET current_scene_instance_id = NULL, updated_at = now() WHERE id = $1` |

---

## Design Decisions

### Why PostgreSQL ENUM types?

All columns that map to Rust enums use PostgreSQL ENUM types rather than TEXT with application-level validation. This provides:

1. **Database-level type safety.** Invalid values are rejected at INSERT/UPDATE time, not just in application code.
2. **Implicit ordinality.** Enums sort by declaration order — `ORDER BY priority` naturally sorts `immediate` before `deferred`. No rank column or application-side mapping needed.
3. **Compact storage.** PostgreSQL stores enum values as 4-byte integers internally, smaller than variable-length TEXT.
4. **Cross-repo consistency.** Matches the `#[derive(sqlx::Type)]` pattern established in tasker-core.

The tradeoff is that `ALTER TYPE ... ADD VALUE` cannot run inside a transaction. At our maturity level this is manageable — enum changes are infrequent and always coordinated with Rust code changes. If a migration needs to add a variant, it runs as a separate non-transactional step.

### Why denormalize story_id, session_id, and scene_id on the event ledger?

The event ledger is the highest-volume table and the most-queried. The most common queries are:
1. "All events in this story" (cross-session analysis)
2. "All events in this session" (replay, checkpoint)
3. "All events at this scene template across visits" (context assembly)

Denormalization eliminates join chains (`event_ledger → turn → scene_instance → scene`, `event_ledger → turn → session → story`). At our expected scale (hundreds of events per session, not millions), the storage cost is negligible.

### Why JSONB for the event payload when we also have extracted columns?

The `payload` JSONB column holds the complete `EventAtom` or `CompoundEvent` as serialized by serde. The extracted columns (`event_kind`, `priority`, `participants`) are for indexing and filtering. This is the "wide event" pattern — the full event is always available for deserialization, while hot-path queries use indexed columns.

This also provides forward compatibility. When we add new fields to `EventAtom`, the JSONB payload captures them immediately without schema migration. Extracted columns are added later when query patterns demand them.

### Why version the character table instead of updating in place?

Character evolution is a core narrative mechanic. The tensor changes over scenes — trust erodes, affection grows, new triggers activate. Versioning preserves the complete history:
- **Replay:** Reconstruct what a character was like in a past scene
- **Debugging:** Compare tensor drift across scenes
- **Rollback:** If a scene exit commits bad tensor changes, the previous version is preserved

The cost is one additional row per character per scene transition, which is negligible.

### Why no separate table for cast membership?

Scene cast is stored in `scenes.scene_data` JSONB rather than a junction table. Cast is always loaded as a unit with the scene (the `load_scene` + `load_cast` operations in the Storykeeper API), never queried independently ("which scenes is character X in?" is a graph query, not a relational query). A junction table would add complexity without supporting any current access pattern.

### Why separate scene templates from scene instances?

Scenes (templates) define the authored creative constraints — cast, setting, stakes, scene type. Scene instances track specific activations of that template within a session. This separation supports three key requirements:

1. **Re-enterability:** The setting topology allows players to revisit locations. The farmhouse scene might be entered three times across a session, each time with different tensions active and different information available. Each entry is a distinct instance with its own turn history and events.

2. **POV tracking:** The `player_entity_id` on `scene_instances` establishes which character the player embodies. This is fixed at scene entry — no mid-scene POV switching. Different instances of the same scene can have different POV characters, supporting multi-perspective narratives.

3. **Entry state capture:** The `entry_conditions` JSONB on each instance captures what the system knew at the moment of entry. This is critical for replay and debugging — the Storykeeper's context assembly depends on what information was available, and that changes between visits.

Without this separation, turns and events would reference scene templates directly, losing the distinction between "the farmhouse scene" (a definition) and "Sarah's first visit to the farmhouse" (an experience).

### What about graph data?

The relational web, narrative graph, and setting topology live in Apache AGE, not in these tables. The `entities`, `scenes`, and `settings` tables serve as identity anchors — AGE vertices reference these UUIDs, and joins between relational tables and AGE graph results use these IDs.

The AGE schema is designed separately (TAS-244 spike, TAS-245 implementation).

### What about checkpoints?

Checkpoint snapshots (periodic serialization of Bevy ECS state) are designed in the Checkpoint & Ledger Replay milestone, not here. The checkpoint table will likely be a simple `(id, session_id, bevy_state JSONB, created_at)` structure.

### What about truth set and information boundaries?

These are tracked in-memory during play and reconstructed from the event ledger at scene entry. The event ledger's revelation events (`event_kind = 'information_transfer'`) and gate condition events provide the raw material. Dedicated tables may be added later if materialized views of information state prove necessary.

---

## Migration Strategy

This schema is implemented as sqlx migrations in `storyteller-storykeeper/migrations/`. The `storyteller-storykeeper` crate owns persistence and exposes a `MIGRATOR` static for use in `#[sqlx::test]` tests.

Migration sequence:

1. `20260211000001_create_enum_types.sql` — all 11 PostgreSQL ENUM types
2. `20260211000002_create_stories.sql` — stories table
3. `20260211000003_create_settings.sql` — settings table
4. `20260211000004_create_sub_graph_layers.sql` — sub-graph layers (without entry_scene_id FK)
5. `20260211000005_create_scenes.sql` — scenes table + deferred FK on sub_graph_layers
6. `20260211000006_create_players.sql` — players table
7. `20260211000007_create_entities.sql` — entities table (before scene_instances, which FKs to entities)
8. `20260211000008_create_sessions.sql` — sessions table (without current_scene_instance_id FK)
9. `20260211000009_create_scene_instances.sql` — scene instances + deferred FK on sessions
10. `20260211000010_create_characters.sql` — characters table
11. `20260211000011_create_turns.sql` — turns table + deferred FKs on entities
12. `20260211000012_create_event_ledger.sql` — event ledger with all indexes

Each migration is idempotent and reversible where possible.

---

## Future Considerations

- **Table partitioning:** If the event ledger grows beyond a single story's scale, partition by `story_id`. Not needed until multi-story concurrent sessions exist.
- **Materialized views:** Entity relevance scores, gate proximity computations, and information boundary state could become materialized views refreshed at scene boundaries.
- **Audit columns:** `created_by` / `updated_by` for multi-user scenarios.
- **Soft deletes:** Not planned — the system prefers immutability (event ledger) and versioning (characters) over deletion.
