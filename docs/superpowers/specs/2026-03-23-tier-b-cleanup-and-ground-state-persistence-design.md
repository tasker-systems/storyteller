# Tier B Cleanup and Ground-State Persistence Design

**Date:** 2026-03-23
**Branch:** `jcoletaylor/tier-b-cleanup-and-storykeeper-persistence`
**Ticket:** `2026-03-23-tier-b-cleanup-and-storykeeper-api-and-persistence`

## Context

Tier B narrative data extraction is complete: 2,038 structured entities across 12 primitive
types, 30 genres, 54 clusters, 3.6MB of validated JSON (PR #28 merged). However, the corpus
has systematic null fields — some extractable from source markdown with deterministic methods,
others requiring cross-genre reasoning or prompt re-engineering. Additionally, the structured
corpus has no persistence path — it exists only as JSON files, unreachable by the Storykeeper
at runtime.

This design addresses two workstreams:

1. **Tier B Cleanup** — fill extractable null fields, evaluate speculative schema fields, and
   annotate schemas with core/extended tiering
2. **Ground-State Persistence** — model the narrative data corpus as read-only reference tables
   in PostgreSQL, with a loader that supports idempotent upsert and drift management

## Scope Boundaries

**In scope:**
- Audit tooling for null rates across the corpus
- Deterministic fill pipeline (regex/heuristic extraction from source markdown)
- Quick LLM patch fills for fields extractable with targeted re-extraction
- Pydantic schema tiering (core vs extended field annotations)
- Evaluation of speculative fields (e.g., `spatial-topology.agency`)
- PostgreSQL `ground_state` schema with hybrid relational + JSONB tables
- Python loader with upsert, drift detection, and pruning
- SQL query functions for Storykeeper consumption

**Out of scope:**
- Story-composition overlay tables (the layer that overrides/extends ground-state per-story)
- AGE graph structures (story-instance runtime, not ground-state reference)
- Full `PostgresStorykeeper` implementation
- Event ledger, checkpoint, or session persistence
- Cross-genre cluster reasoning fills (documented as deferred work)
- Visualization implementation

## Part 1: Tier B Cleanup Pipeline

### 1.1 Audit Command

New CLI subcommand: `narrative-data audit`

Walks all 12 types across 30 genres + clusters. For each type, reports null/empty rates per
field. Output is both a JSON summary (machine-readable, consumed by the tier annotation step)
and a human-readable table.

```
$ narrative-data audit

dynamics (292 records)
  genre_slug       0.0% null   [core]
  edge_type        0.0% null   [core]
  spans_scales    71.2% null   → deterministic fill candidate
  currencies      63.4% null   → LLM patch candidate
  valence         42.1% null   → LLM patch candidate
  ...

spatial-topology (247 records)
  agency          75.3% null   → evaluate: deterministic or speculative?
  state_shift     89.0% null   → LLM patch candidate
  ...
```

The audit command is run before and after fills to measure progress. It also produces a
machine-readable summary used by the schema tiering step.

### 1.2 Deterministic Fills

New module: `tools/narrative-data/src/narrative_data/pipeline/postprocess.py`

Pure Python, no LLM calls. Each fill function:
- Reads existing JSON + source markdown for the same genre/type
- Fills only null/empty fields (never overwrites populated data)
- Writes back to the same JSON file
- Is idempotent: running twice produces the same result

**Fill functions:**

| Field | Type | Method |
|---|---|---|
| `spans_scales` | dynamics | Regex for `**Scale:**` patterns in source markdown. Parse "Spanning", "Cross-Scale", "Orbital", "Arc", "Scene" labels into list. |
| `agency` | spatial-topology | Lookup table from `friction.type` + `directionality.type` → agency level. One-way + high-friction = "none", bidirectional + low-friction = "high", asymmetric + choice = "medium", constrained options = "illusion". |
| `network_position` | archetype-dynamics | Infer from role slot count + symmetry. 1 primary + 2+ secondary = "hub", bilateral symmetric = "bridge", triangulated = "triangulation", isolated = "peripheral". |
| Description fields | various | Extract from clearly delineated source markdown sections where content exists but wasn't captured during initial extraction. |

**CLI:** `narrative-data fill --tier deterministic [--dry-run] [--type dynamics] [--genre folk-horror]`

The `--dry-run` flag reports what would change without writing. Optional `--type` and `--genre`
filters allow targeted runs.

### 1.3 Quick LLM Patch Fills

Targeted re-extraction for specific fields where the data exists in source markdown but isn't
regex-extractable. Uses the existing pipeline's Ollama integration with focused prompts.

| Field | Type | Method |
|---|---|---|
| `valence` | dynamics | Edge semantics → positive/negative/mixed. Cluster synthesis files discuss this explicitly. Short prompt per entity. |
| `currencies` | dynamics | Listed in dynamics markdown in varied prose formats. Extraction prompt targeting currency mentions. |
| `scale_manifestations` | dynamics | Orbital/arc/scene breakdowns present in source markdown sections 6-7. Structured extraction into the three sub-fields. |

**CLI:** `narrative-data fill --tier llm-patch [--dry-run] [--type dynamics] [--genre folk-horror]`

Same idempotency and filtering semantics as deterministic fills.

### 1.4 Deferred Work (Documented, Not Implemented)

These fields require approaches beyond this session's scope:

| Field | Type | Why Deferred |
|---|---|---|
| `constraint_layer_type` | genre-dimensions | Requires cross-genre cluster reasoning — which genres are constraint layers vs standalone regions |
| `modifies` | genre-dimensions | Which genres a constraint layer modifies — derivable from cluster synthesis but needs dedicated prompt engineering |
| `crossing_rules` | ontological-posture | Requires deep ontological context per-genre, possibly new segmentation |
| `obligations_across` | ontological-posture | Same as crossing_rules |

### 1.5 Speculative Field Evaluation

**`spatial-topology.agency`** requires specific investigation:
- Check source markdown across multiple genres for extractable content
- If the field is genuinely not present in source material, it may be a speculative addition
  to the schema that should move to extended tier or be dropped entirely
- The deterministic fill (1.2) proposes an inference heuristic from friction + directionality;
  if this proves unreliable across genres, the field moves to deferred

## Part 2: Schema Tiering

### 2.1 Tier Annotations

Add tier metadata to Pydantic `Field` definitions using `json_schema_extra`:

```python
class DynamicsEdge(BaseModel):
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    edge_type: str = Field(..., json_schema_extra={"tier": "core"})
    spans_scales: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    currencies: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    valence: str | None = Field(None, json_schema_extra={"tier": "extended"})
```

### 2.2 Tier Assignment Criteria

- **Core**: identity fields (slugs, names, types), fields with >80% post-cleanup population
  rate, fields the Storykeeper needs for filtering or indexing at query time
- **Extended**: fields with <80% post-cleanup population, fields only useful as payload
  content, fields pending cross-genre reasoning or future extraction improvement

### 2.3 Audit-Driven Assignment

Tier assignments are made *after* running the audit and fill commands, so tiering reflects the
post-cleanup state of the corpus. The audit JSON output can be consumed programmatically to
suggest tier assignments, but final assignment is a human decision informed by which fields
the Storykeeper will need to filter on.

### 2.4 Downstream Consumption

The tier annotations serve as the contract between the Python extraction pipeline and the
ground-state persistence layer:
- **Core** fields become relational columns in the PostgreSQL tables (for indexing, filtering,
  joining)
- **Extended** fields remain in the JSONB payload column only
- The Pydantic models are the single source of truth for both extraction validation and
  persistence modeling

## Part 3: Ground-State Persistence

### 3.1 Data Architecture: Three Layers

The ground-state data participates in a three-layer model, though only layer one is built now:

1. **Ground-state reference** (this design) — read-only canonical data from Tier B, loaded
   into PostgreSQL. The narrative grammar library.
2. **Story-composition overlay** (future) — per-story customizations expressed as join metadata
   that override, extend, suppress, or ignore ground-state records. Does not mutate layer 1.
3. **Story-instance runtime** (future) — AGE graph structures for a specific playthrough,
   informed by layers 1+2. Event ledger, relational web, narrative graph.

The Storykeeper resolves precedence at query time: story-composition overlays take priority
over ground-state defaults. The ground-state tables are never mutated by downstream layers.

### 3.2 PostgreSQL Schema

A dedicated `ground_state` schema, namespaced away from story-instance tables:

```sql
CREATE SCHEMA IF NOT EXISTS ground_state;
```

### 3.3 Reference Entity Tables

The analytical scaffolding that situates the entire corpus — genres, clusters, state variables,
and dimensions — are first-class relational entities with their own UUIDv7 identifiers. All
primitive type tables reference these via foreign keys rather than denormalized text slugs.

```sql
-- The genre as a modeled entity (source: region.json per genre)
CREATE TABLE ground_state.genres (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    payload       JSONB NOT NULL,      -- full region.json content
    source_hash   TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Genre clusters (the 6 semantic groupings)
CREATE TABLE ground_state.genre_clusters (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    payload       JSONB,               -- cluster synthesis metadata
    source_hash   TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Genre ↔ cluster membership
CREATE TABLE ground_state.genre_cluster_members (
    genre_id      UUID NOT NULL REFERENCES ground_state.genres(id) ON DELETE CASCADE,
    cluster_id    UUID NOT NULL REFERENCES ground_state.genre_clusters(id) ON DELETE CASCADE,
    PRIMARY KEY (genre_id, cluster_id)
);

-- Canonical state variables (the 12 that emerged from axis inventory)
CREATE TABLE ground_state.state_variables (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    default_range JSONB,               -- { "min": 0.0, "max": 1.0 } or similar
    payload       JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Universal dimensions (34 dimensions across 7 groups from terrain analysis)
CREATE TABLE ground_state.dimensions (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    dimension_group TEXT NOT NULL,      -- e.g., "narrative-structure", "relational", "world"
    description   TEXT,
    payload       JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dimensions_group ON ground_state.dimensions (dimension_group);
```

These tables are loaded first, before any primitive type tables, since they provide the FK
targets. The loader resolves slugs to UUIDs during the primitive type load phase.

### 3.4 Primitive Type Table Structure

Each of the 12 primitive types gets a table following a shared pattern, now with FK references
to the scaffolding entities:

```sql
CREATE TABLE ground_state.archetypes (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    genre_id      UUID NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID REFERENCES ground_state.genre_clusters(id),
    entity_slug   TEXT NOT NULL,
    name          TEXT NOT NULL,
    -- promoted core fields (type-specific, driven by tier annotations)
    archetype_family  TEXT,
    primary_scale     TEXT,
    -- full payload
    payload       JSONB NOT NULL,
    -- housekeeping
    source_hash   TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Natural key for upsert (functional index for nullable cluster_id)
CREATE UNIQUE INDEX idx_archetypes_natural_key
    ON ground_state.archetypes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));

CREATE INDEX idx_archetypes_genre ON ground_state.archetypes (genre_id);
CREATE INDEX idx_archetypes_cluster ON ground_state.archetypes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_archetypes_payload ON ground_state.archetypes USING gin (payload);
```

**Key design choices:**

- **FK to `genres` and `genre_clusters`** — proper relational integrity instead of
  denormalized text slugs. The loader resolves `genre_slug` → `genre_id` via lookup during
  the load phase.
- **Functional unique index** with `COALESCE` on `cluster_id` — PostgreSQL doesn't support
  nullable columns in inline `UNIQUE` constraints with the semantics we need. A functional
  index handles this correctly, and serves as the upsert conflict target.
- **`source_hash`** — SHA256 of the source JSON file content. The loader compares hashes to
  detect drift: changed file = upsert, identical = skip.
- **`payload` JSONB** — the full entity including both core and extended fields. Core fields
  are duplicated into columns for indexing, not removed from payload. The Storykeeper reads
  payload as a whole; columns exist for filtering.
- **GIN index on payload** — supports `@>` containment queries for ad-hoc JSONB filtering
  without requiring column promotion.
- **Promoted core columns** vary per type, driven by tier annotations. Only fields the
  Storykeeper would plausibly `WHERE` or `JOIN` on get promoted.

### 3.5 Table Inventory

**Reference entities (scaffolding):**

| Table | Natural Key | Est. Records |
|---|---|---|
| `genres` | slug | 30 |
| `genre_clusters` | slug | 6 |
| `genre_cluster_members` | (genre_id, cluster_id) | ~30 |
| `state_variables` | slug | 12 |
| `dimensions` | slug | 34 |

**Primitive type tables:**

| Table | Promoted Core Columns | Est. Records |
|---|---|---|
| `archetypes` | genre_id, entity_slug, archetype_family, primary_scale | ~240 |
| `settings` | genre_id, entity_slug, setting_type, communicability_profile | ~210 |
| `dynamics` | genre_id, entity_slug, edge_type, scale | ~290 |
| `goals` | genre_id, entity_slug, goal_scale | ~200 |
| `profiles` | genre_id, entity_slug, archetype_ref | ~230 |
| `tropes` | genre_id, entity_slug, trope_family | ~300 |
| `narrative_shapes` | genre_id, entity_slug, shape_type, beat_count | ~280 |
| `ontological_posture` | genre_id, entity_slug, boundary_stability | ~210 |
| `spatial_topology` | genre_id, entity_slug, friction_type, directionality_type | ~250 |
| `place_entities` | genre_id, entity_slug, place_type | ~200 |
| `archetype_dynamics` | genre_id, entity_slug, archetype_a, archetype_b | ~230 |
| `genre_dimensions` | genre_id | 30 |

Record counts are approximate, based on current corpus size. The promoted core columns listed
are initial candidates — final selection depends on post-cleanup audit results and tier
annotation decisions.

### 3.6 Loader Design

Python script in the narrative-data package, integrated as a CLI subcommand:
`narrative-data load-ground-state`

**Two-phase load:**

**Phase 1: Reference entities** (must complete before phase 2)

1. Load `genres` from `region.json` files — one per genre directory
2. Load `genre_clusters` from cluster synthesis file metadata
3. Load `genre_cluster_members` from cluster membership data in region files
4. Load `state_variables` from the canonical list (sourced from terrain analysis / region data)
5. Load `dimensions` from the 34 universal dimensions (sourced from terrain analysis)

Phase 1 establishes the FK targets. The loader builds an in-memory slug→UUID lookup map
for use in phase 2.

**Phase 2: Primitive type entities**

1. Walk the corpus directory structure (`storyteller-data/narrative-data/`)
2. For each JSON file, deserialize through the Pydantic schema (validation gate — reuses the
   same schemas as extraction)
3. Resolve `genre_slug` → `genre_id` and `cluster_slug` → `cluster_id` via the phase 1
   lookup map
4. Compute `source_hash` (SHA256 of file content)
5. Upsert via `INSERT ... ON CONFLICT DO UPDATE`:
   ```sql
   INSERT INTO ground_state.archetypes (genre_id, entity_slug, name, ..., payload, source_hash)
   VALUES ($1, $2, $3, ..., $4, $5)
   ON CONFLICT ON CONSTRAINT idx_archetypes_natural_key
   DO UPDATE SET
       payload = EXCLUDED.payload,
       source_hash = EXCLUDED.source_hash,
       updated_at = now()
   WHERE archetypes.source_hash != EXCLUDED.source_hash;
   ```
6. **Pruning pass**: after load, delete rows whose `(genre_id, entity_slug, cluster_id)`
   combination no longer exists in the corpus. Matches on the full composite key to avoid
   accidentally deleting cluster-level entities when per-genre entities are removed (or
   vice versa).
7. **Report**: `inserted N, updated M, pruned P, skipped Q (unchanged)`

**Flags:**
- `--dry-run` — report what would change without writing
- `--type <type>` — load only a specific primitive type
- `--genre <genre>` — load only a specific genre
- `--skip-prune` — skip the pruning pass (useful during incremental development)
- `--refs-only` — load only reference entities (phase 1), useful for bootstrapping

**Database connection:** Uses `DATABASE_URL` from `.env` (same as storyteller engine),
targeting the existing PostgreSQL 18 + Apache AGE instance on port 5435.

### 3.7 SQL Query Functions

Purpose-built SQL functions encapsulate join logic and return pre-assembled payloads. The Rust
Storykeeper calls these via sqlx, receiving JSONB results deserializable into domain types.

```sql
-- Get all ground-state data for a genre, pre-joined across types
-- Accepts slug (human-readable) and resolves to genre_id internally
CREATE OR REPLACE FUNCTION ground_state.genre_context(p_genre_slug TEXT)
RETURNS JSONB AS $$
DECLARE
    v_genre_id UUID;
BEGIN
    SELECT id INTO v_genre_id FROM ground_state.genres WHERE slug = p_genre_slug;
    IF v_genre_id IS NULL THEN
        RETURN NULL;
    END IF;

    RETURN jsonb_build_object(
        'genre_slug', p_genre_slug,
        'genre', (SELECT payload FROM ground_state.genres WHERE id = v_genre_id),
        'archetypes', (SELECT jsonb_agg(payload) FROM ground_state.archetypes
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'dynamics',   (SELECT jsonb_agg(payload) FROM ground_state.dynamics
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'settings',   (SELECT jsonb_agg(payload) FROM ground_state.settings
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'goals',      (SELECT jsonb_agg(payload) FROM ground_state.goals
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'profiles',   (SELECT jsonb_agg(payload) FROM ground_state.profiles
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'tropes',     (SELECT jsonb_agg(payload) FROM ground_state.tropes
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'narrative_shapes', (SELECT jsonb_agg(payload) FROM ground_state.narrative_shapes
                            WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'ontological_posture', (SELECT jsonb_agg(payload) FROM ground_state.ontological_posture
                               WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'spatial_topology', (SELECT jsonb_agg(payload) FROM ground_state.spatial_topology
                            WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'place_entities', (SELECT jsonb_agg(payload) FROM ground_state.place_entities
                          WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'archetype_dynamics', (SELECT jsonb_agg(payload) FROM ground_state.archetype_dynamics
                              WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'genre_dimensions', (SELECT payload FROM ground_state.genre_dimensions
                            WHERE genre_id = v_genre_id LIMIT 1)
    );
END;
$$ LANGUAGE plpgsql STABLE;
```

Additional functions will be defined as Storykeeper query patterns emerge:
- `ground_state.archetype_context(p_genre_slug, p_archetype_slug)` — single archetype with
  associated profiles, dynamics, goals
- `ground_state.cluster_context(p_cluster_slug)` — cluster-level synthesis data across all
  member genres
- `ground_state.entity_by_state_variable(p_variable_slug)` — cross-type lookup for entities
  referencing a specific state variable (joins through JSONB payload containment)

These are single-trip queries — one round-trip returns everything the Storykeeper needs for
context assembly. Query complexity lives in SQL; the Rust side deserializes `JsonValue`.
Functions accept human-readable slugs and resolve to UUIDs internally, keeping the calling
code clean.

## Part 4: Composition and Test Strategy

### 4.1 Execution Sequence

```
Tier B Cleanup                    Ground-State Persistence
─────────────                    ────────────────────────
1. audit command
   ↓ (null rates inform)
2. deterministic fills
3. LLM patch fills
   ↓ (post-cleanup rates)
4. schema tiering
   ↓ (tier annotations)
                                 5. migration: ground_state schema + tables
                                 6. loader (upsert + drift + prune)
                                 7. SQL query functions
                                 8. smoke test: load corpus, query back
```

Steps 1-4 are sequential. Steps 5-8 depend on step 4 (tier annotations drive column
promotion) but are otherwise independent of each other.

### 4.2 Test Strategy

**Cleanup tests** (pytest):
- Unit tests for each fill function with fixture JSON + markdown data
- Integration test: audit → fill → re-audit, assert null rates decreased for targeted fields
- Idempotency test: fill → fill again, assert no changes on second run

**Persistence tests** (pytest, requires PostgreSQL on port 5435):
- Loader idempotency: load twice, assert same row count and content
- Drift detection: load, modify a source JSON, reload, assert row updated with new hash
- Pruning: load, remove a source JSON, reload with prune, assert row deleted
- SQL functions: load fixture data, call `genre_context()`, assert returned JSONB contains
  expected structure and entity counts

### 4.3 Deliverables

- `tools/narrative-data/src/narrative_data/pipeline/postprocess.py` — audit + fill module
- CLI subcommands: `audit`, `fill`, `load-ground-state`
- Tier annotations across all 12 Pydantic schema models
- SQL migration: `ground_state` schema, reference entity tables (genres, clusters, state
  variables, dimensions), 12 primitive type tables, indexes
- SQL query functions (at minimum `genre_context()`)
- Python loader with upsert/drift/prune
- Tests for cleanup and persistence
- Session documentation of tiering decisions

## References

- Session note: `sessions/storyteller/2026-03-23 — P3/P4 Complete — Full Stage 2 Extraction Across 12 Types.md`
- Milestone: `milestones/tier-betwixt-grammar-to-vocabulary.md`
- Prior persistence ticket: `tickets/storyteller/2026-02-10-implement-sqlx-migrations-and-storykeeper-persistence-traits.md` (done — produced InMemoryStorykeeper)
- Storykeeper API contract: `docs/technical/storykeeper-api-contract.md`
- Knowledge graph domain model: `docs/technical/knowledge-graph-domain-model.md`
- PostgreSQL schema design: `docs/technical/postgresql-schema-design.md`
- Storykeeper crate architecture: `docs/technical/storykeeper-crate-architecture.md`
- Comprehensive terrain analysis: `storyteller-data/narrative-data/analysis/2026-03-21-comprehensive-terrain-analysis.md`
