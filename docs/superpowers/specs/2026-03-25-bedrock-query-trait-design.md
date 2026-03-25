# BedrockQuery Trait with PostgreSQL Backing — Design Spec

**Date**: 2026-03-25
**Ticket**: `2026-03-24-implement-storykeeperquery-trait-with-ground-state-backing`
**Milestone**: Storykeeper Ground-State Integration
**Branch**: `jcoletaylor/implement-storykeeperquery-trait-with-ground-state-backing`

## Summary

Implement a `BedrockQuery` trait — the read-only query surface for the narrative grammar corpus — backed by PostgreSQL. This is the grammar layer that agents query to understand *how narrative works* within a genre. The Tome/Lore vocabulary layer (world-specific content) will compose against this surface via a separate `SedimentQuery` / `TomeLoreQuery` trait in future work.

### Nomenclature: Geological Data Layers

The data architecture adopts the geological temporal layering metaphor already used for character tensors:

| Layer | Content | Schema | Mutability |
|-------|---------|--------|------------|
| **Bedrock** | Narrative grammar — genres, archetypes, dynamics, tropes, etc. | `bedrock.*` | Read-only from Rust; Python loader owns writes |
| **Sediment** | World vocabulary — Tome (authored world) and Lore (agent-filtered views) | TBD | Authored, accumulates |
| **Topsoil** | Live story state — events, turns, entity weights, sessions | `public.*` | Churns with active play |

This rename replaces all `ground_state` references across SQL, Python, and Rust.

## Architecture

### Crate Ownership

```
storyteller-core (owns contracts + types)
├── src/types/bedrock.rs        → BedrockEntity<T>, GenreContext, all Record structs
└── src/traits/bedrock.rs       → BedrockQuery trait

storyteller-storykeeper (owns implementations)
├── src/bedrock/mod.rs          → PostgresBedrock struct + trait impl
└── src/bedrock/queries/        → Query functions organized by pattern
    ├── genre_context.rs        → Bulk genre_context() via SQL function
    ├── by_genre.rs             → All *_by_genre() queries
    ├── by_slug.rs              → All *_by_slug() queries
    ├── dimensions.rs           → genre_dimensions() + dimensional queries
    ├── state_variables.rs      → state_variables(), interactions
    └── reference.rs            → genres(), trope_families()
```

**Why this split**: Core owns all types and trait contracts. Record structs derive `sqlx::FromRow`, which requires sqlx as a dependency in core — but only for the derive macro and Postgres type mappings (`Uuid`, `DateTime<Utc>`, `serde_json::Value`). No runtime, no connection pool, no TLS. Core knows *what shape data takes*; storykeeper knows *how to get it from PostgreSQL*.

This keeps the dependency graph clean: any crate (engine, api, future agents) can reference Record types and the `BedrockQuery` trait through core without depending on storykeeper. `GenreContext` lives in core alongside the Record types it composes, and the trait returns fully typed results with no `serde_json::Value` indirection.

### Type System

#### Record Structs (storyteller-core)

One `Record` struct per table, living in `storyteller-core/src/types/bedrock.rs`. Each derives `sqlx::FromRow`, `Debug`, `Clone`, `Serialize`, `Deserialize`. Named `*Record` (not `*Row`) because query results may come from joins or aggregates, not strictly single rows. Living in core means any crate can reference these types without depending on storykeeper.

```rust
/// Archetype entity from bedrock.archetypes.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ArchetypeRecord {
    pub id: Uuid,
    pub genre_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub entity_slug: String,
    pub name: String,
    // Promoted columns (type-specific)
    pub archetype_family: Option<String>,
    pub primary_scale: Option<String>,
    // Full payload
    pub payload: serde_json::Value,
    pub source_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Same pattern for all 12 primitive types plus reference types (`GenreRecord`, `StateVariableRecord`, `TropeFamilyRecord`, `GenreDimensionRecord`, `DimensionValueRecord`, `StateVariableInteractionRecord`).

#### BedrockEntity Envelope (storyteller-core)

Generic wrapper carrying common metadata alongside the typed payload:

```rust
/// A bedrock entity with common metadata.
/// T is the typed record (e.g., ArchetypeRecord).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockEntity<T> {
    pub id: Uuid,
    pub genre_slug: String,
    pub entity_slug: String,
    pub record: T,
}
```

#### GenreContext Composite (storyteller-core)

Bulk result from `genre_context()` — all 12 types for a genre. Lives in core alongside the Record types it composes:

```rust
/// Complete bedrock context for a single genre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreContext {
    pub genre: GenreRecord,
    pub archetypes: Vec<ArchetypeRecord>,
    pub dynamics: Vec<DynamicRecord>,
    pub settings: Vec<SettingRecord>,
    pub goals: Vec<GoalRecord>,
    pub profiles: Vec<ProfileRecord>,
    pub tropes: Vec<TropeRecord>,
    pub narrative_shapes: Vec<NarrativeShapeRecord>,
    pub ontological_posture: Vec<OntologicalPostureRecord>,
    pub spatial_topology: Vec<SpatialTopologyRecord>,
    pub place_entities: Vec<PlaceEntityRecord>,
    pub archetype_dynamics: Vec<ArchetypeDynamicRecord>,
    pub dimensions: Option<GenreDimensionRecord>,
}
```

### BedrockQuery Trait

Async trait, object-safe, `Send + Sync + Debug`. WET-first design — explicit per-type methods that can later be consolidated behind shared traits (`AsGenreScopedRecord`, `AsSluggableRecord`) as usage patterns emerge.

All queries take `genre_slug: &str` (semantic names, never UUIDs). The impl resolves slug→UUID internally.

```rust
#[async_trait]
pub trait BedrockQuery: Send + Sync + Debug {

    // ── Bulk ──────────────────────────────────────────────
    /// Full genre context — all 12 primitive types in one call.
    /// Primary scene-entry hydration path.
    /// Returns NotFound if the genre slug doesn't exist.
    async fn genre_context(&self, genre_slug: &str) -> StorytellerResult<GenreContext>;

    // ── By genre (type-within-genre) ──────────────────────
    async fn archetypes_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<ArchetypeRecord>>;
    async fn dynamics_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<DynamicRecord>>;
    async fn settings_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<SettingRecord>>;
    async fn goals_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<GoalRecord>>;
    async fn profiles_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<ProfileRecord>>;
    async fn tropes_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<TropeRecord>>;
    async fn narrative_shapes_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<NarrativeShapeRecord>>;
    async fn ontological_posture_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<OntologicalPostureRecord>>;
    async fn spatial_topology_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<SpatialTopologyRecord>>;
    async fn place_entities_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<PlaceEntityRecord>>;
    async fn archetype_dynamics_by_genre(&self, genre_slug: &str) -> StorytellerResult<Vec<ArchetypeDynamicRecord>>;

    // ── By slug (specific entity lookup) ──────────────────
    async fn archetype_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<ArchetypeRecord>>;
    async fn dynamic_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<DynamicRecord>>;
    async fn setting_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<SettingRecord>>;
    async fn goal_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<GoalRecord>>;
    async fn profile_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<ProfileRecord>>;
    async fn trope_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<TropeRecord>>;
    async fn narrative_shape_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<NarrativeShapeRecord>>;
    async fn ontological_posture_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<OntologicalPostureRecord>>;
    async fn spatial_topology_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<SpatialTopologyRecord>>;
    async fn place_entity_by_slug(&self, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<PlaceEntityRecord>>;

    // ── Dimensional ───────────────────────────────────────
    /// Genre dimensional profile — all 34 dimensions.
    async fn genre_dimensions(&self, genre_slug: &str) -> StorytellerResult<Option<GenreDimensionRecord>>;
    /// All dimension values for a specific entity.
    async fn dimensions_for_entity(&self, primitive_table: &str, entity_slug: &str, genre_slug: &str) -> StorytellerResult<Vec<DimensionValueRecord>>;
    /// All entities sharing a dimension within a genre, crossing primitive types.
    async fn entities_by_dimension(&self, dimension_slug: &str, genre_slug: &str) -> StorytellerResult<Vec<DimensionValueRecord>>;
    /// Dimensional intersection: entities sharing multiple dimensions.
    async fn dimensional_intersection(&self, dimension_slugs: &[&str], genre_slug: &str) -> StorytellerResult<Vec<DimensionValueRecord>>;

    // ── State variables ───────────────────────────────────
    /// All state variables in the canonical registry.
    async fn state_variables(&self) -> StorytellerResult<Vec<StateVariableRecord>>;
    /// Entities that interact with a state variable within a genre.
    async fn state_variable_interactions(&self, genre_slug: &str, state_variable_slug: &str) -> StorytellerResult<Vec<StateVariableInteractionRecord>>;

    // ── Reference data ────────────────────────────────────
    /// All genres in bedrock.
    async fn genres(&self) -> StorytellerResult<Vec<GenreRecord>>;
    /// All trope families.
    async fn trope_families(&self) -> StorytellerResult<Vec<TropeFamilyRecord>>;
}
```

**Return conventions**:
- By-genre: `Vec<T>` — empty if genre not found (for individual type queries) or `NotFound` error (for `genre_context()`)
- By-slug: `Option<T>` — `None` if entity doesn't exist in this genre
- `genre_context()`: `NotFound` error for unknown genre (missing genre is a configuration error, not an expected empty result)

**Future direction** (documented, not implemented):
- Cross-genre queries: `archetype_across_genres(entity_slug)` — shape TBD
- Relational queries: `dynamics_for_archetype(genre_slug, archetype_slug)` — shape TBD

### PostgresBedrock Implementation

```rust
/// Bedrock query implementation backed by PostgreSQL.
///
/// Read-only — all writes happen via the Python narrative-data loader.
/// Wraps a shared PgPool; callers construct with an existing pool.
#[derive(Debug, Clone)]
pub struct PostgresBedrock {
    pool: PgPool,
}

impl PostgresBedrock {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
```

Deliberately thin — no caching, no connection management. Takes a `PgPool` from whoever owns the connection lifecycle. Clone is cheap (PgPool is Arc internally).

The trait impl delegates to free functions in `queries/` modules, keeping the impl thin and the SQL in focused, testable units.

### Query Patterns

| Pattern | SQL Strategy | Return |
|---------|-------------|--------|
| `genre_context` | `SELECT bedrock.genre_context($1)` — single JSONB round-trip via SQL function | Deserialize into `GenreContext` |
| `*_by_genre` | `SELECT t.* FROM bedrock.{table} t JOIN bedrock.genres g ON t.genre_id = g.id WHERE g.slug = $1 AND t.cluster_id IS NULL` | `Vec<Record>` via `sqlx::query_as` |
| `*_by_slug` | Same join + `AND t.entity_slug = $2` | `Option<Record>` via `fetch_optional` |
| `dimensions_for_entity` | `SELECT * FROM bedrock.dimension_values WHERE primitive_table = $1 AND primitive_id = (subquery)` | `Vec<DimensionValueRecord>` |
| `entities_by_dimension` | `SELECT * FROM bedrock.dimension_values dv JOIN bedrock.genres g ON dv.genre_id = g.id WHERE g.slug = $1 AND dv.dimension_slug = $2` | `Vec<DimensionValueRecord>` |
| `dimensional_intersection` | `SELECT dv.* FROM bedrock.dimension_values dv JOIN bedrock.genres g ON dv.genre_id = g.id WHERE g.slug = $1 AND dv.dimension_slug = ANY($2) GROUP BY dv.primitive_table, dv.primitive_id HAVING COUNT(DISTINCT dv.dimension_slug) = $3` — entities sharing all requested dimensions | `Vec<DimensionValueRecord>` |
| `state_variable_interactions` | Joins through `primitive_table`/`primitive_id` to resolve genre: `SELECT sv.slug, sv.name, psvi.operation, psvi.context FROM bedrock.primitive_state_variable_interactions psvi JOIN bedrock.state_variables sv ON psvi.state_variable_id = sv.id WHERE psvi.primitive_id IN (SELECT id FROM bedrock.{table} WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1)) AND sv.slug = $2` — note: requires iterating primitive tables or a UNION approach | `Vec<StateVariableInteractionRecord>` |
| reference | `SELECT * FROM bedrock.{table}` | `Vec<Record>` |

All by-genre and by-slug queries filter `cluster_id IS NULL` by default — genre-specific entities only. Cluster-scoped queries are future direction.

The `genre_context()` SQL function is the primary path for bulk loading. It evolves as needs demand (additional indexes, query plan optimization). Individual typed queries via `sqlx::query_as` provide the per-type path for targeted lookups.

## Dimensional Extraction

### Problem

JSONB payloads contain structured dimensional data across all 12 primitive types. These dimensions have typed semantics (normalized, bipolar, categorical) and shared axes that need to be queryable across primitive types for compositional use by downstream consumers.

### Value Types

| Type | Range | Examples | Storage |
|------|-------|----------|---------|
| Normalized | `[0.0, 1.0]` | warmth, authority, sensory_density, knowability | `REAL` |
| Bipolar | `[-1.0, 1.0]` | traversal_cost deltas, state variable operations | `REAL` |
| Categorical | enum values | tension_signature, enclosure, edge_type, valence | `TEXT` |
| Weighted tags | dict\<str, float\> | power_treatment, identity_treatment | `JSONB` |
| Set | list\<str\> | currencies, magic types, locus_of_power | `JSONB` |

### Schema: bedrock.dimension_values

```sql
CREATE TABLE bedrock.dimension_values (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Entity reference
    primitive_table   VARCHAR     NOT NULL,
    primitive_id      UUID        NOT NULL,
    genre_id          UUID        NOT NULL REFERENCES bedrock.genres(id),
    -- Dimension identity
    dimension_slug    TEXT        NOT NULL,
    dimension_group   TEXT        NOT NULL,
    value_type        VARCHAR     NOT NULL,  -- 'normalized', 'bipolar', 'categorical', 'weighted_tags', 'set'
    -- Typed value columns (one populated per row)
    numeric_value     REAL,
    categorical_value TEXT,
    complex_value     JSONB,
    -- Provenance
    source_path       TEXT,
    tier              VARCHAR     NOT NULL DEFAULT 'core',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary query paths
CREATE INDEX idx_dv_entity ON bedrock.dimension_values(primitive_table, primitive_id);
CREATE INDEX idx_dv_dimension ON bedrock.dimension_values(dimension_slug);
CREATE INDEX idx_dv_genre_dimension ON bedrock.dimension_values(genre_id, dimension_slug);
CREATE INDEX idx_dv_value_type ON bedrock.dimension_values(value_type);
CREATE INDEX idx_dv_complex ON bedrock.dimension_values USING gin (complex_value) WHERE complex_value IS NOT NULL;
```

### Population

The Python loader's phase 2 is extended: after upserting each primitive entity, walk its payload to extract dimensional values. Extraction rules are defined per primitive type in Python (not SQL). Upsert on `(primitive_table, primitive_id, dimension_slug)`.

### Dimensional Inventory

| Primitive Type | Dimension Groups | Approx Count |
|---------------|-----------------|-------------|
| archetypes | personality (7 core + extended_axes) | ~9+ |
| dynamics | edge_type, directionality, valence, evolution_pattern, currencies, network_position | ~6 |
| profiles | tension_signature, emotional_register, pacing, cast_density, physical_dynamism, information_flow, resolution_tendency | 7 |
| goals | goal_scale, state variable interactions | ~2+ |
| place_entities | communicability 4-channel (atmospheric, sensory, spatial, temporal), topological_role | ~9 |
| spatial_topology | friction, directionality, agency, tonal_inheritance, traversal_cost | ~6 |
| ontological_posture | boundary_stability, worldview axes | ~3 |
| genre_dimensions | 34 dimensions across 8 groups | 34 |
| tropes | trope_family, uniqueness | ~2 |
| narrative_shapes | shape_type, beat_count, spans_scales | ~3 |
| settings | setting_type | ~1 |

### Scope Boundary

**In scope**: Schema, extraction rules for core-tier dimensions, population in Python loader, Rust `DimensionValueRecord`, three dimensional query methods on `BedrockQuery`.

**Out of scope**: Compositional semantics (how dimension values combine mathematically across types), extended-tier dimension extraction, candidate dimensions N1-N10. These belong to the consumers (Dramaturge, context assembly, `TomeLoreQuery`).

## Rename: ground_state → bedrock

All references to `ground_state` are renamed to `bedrock`:

| Layer | Before | After |
|-------|--------|-------|
| PostgreSQL schema | `ground_state.*` | `bedrock.*` |
| SQL function | `ground_state.genre_context()` | `bedrock.genre_context()` |
| Migrations | `create_ground_state_*` | Edit in place → `create_bedrock_*` |
| Python loader | `ground_state` schema refs | `bedrock` schema refs |
| Rust trait | `GroundStateQuery` | `BedrockQuery` |
| Rust impl | `PostgresGroundState` | `PostgresBedrock` |
| Rust envelope | `GroundStateEntity<T>` | `BedrockEntity<T>` |
| Module paths | `src/ground_state/` | `src/bedrock/` |

**Strategy**: Edit migrations in place, drop and reload via Python loader. No backwards compatibility needed — bedrock data is regenerated from the narrative-data corpus.

## Testing

### Three Tiers

**Unit tests** (always run, no dependencies):
- Record struct serialization round-trips
- GenreContext deserialization from known JSONB fixtures
- BedrockEntity envelope construction
- Error mapping (sqlx errors → StorytellerError)

**Integration tests** (feature-gated `test-db`, require PostgreSQL + loaded corpus):
- `genre_context()` returns all 12 types for a known genre
- `*_by_genre()` returns expected counts
- `*_by_slug()` returns correct entity
- `state_variable_interactions()` crosses primitive types
- `dimensions_for_entity()` returns extracted dimensions
- `entities_by_dimension()` crosses primitive types
- Unknown genre returns `NotFound` error
- Unknown slug returns `None`

**Contract tests** (compile-time):
- `PostgresBedrock` implements `BedrockQuery`
- Object safety: `Arc<dyn BedrockQuery>` compiles
- `Send + Sync` bounds satisfied

### Feature Gate

```toml
[features]
test-db = []  # Integration tests requiring PostgreSQL + loaded corpus
```

Test setup connects to `DATABASE_URL` or falls back to the development default.

## Key Design Decisions

1. **Types and trait in core, impl in storykeeper** — Record structs, GenreContext, BedrockEntity, and BedrockQuery all live in core so any crate can reference them. Core adds sqlx as a lightweight dependency (derive macro + type mappings only, no runtime). Storykeeper owns query execution via PostgresBedrock.
2. **Record not Row** — query results may come from joins/aggregates, not strictly single rows
3. **WET-first** — explicit per-type methods; extract `AsGenreScopedRecord` / `AsSluggableRecord` when patterns emerge in practice
4. **SQL function for bulk** — `bedrock.genre_context()` avoids N+1, enables query plan optimization and index tuning
5. **Slug-based API** — callers work with semantic names, never raw UUIDs
6. **NotFound for missing genres** — a missing genre is a configuration error, not an expected empty result
7. **cluster_id IS NULL default** — genre-specific entities only; cluster-scoped queries are future direction
8. **Dimensional extraction as infrastructure** — structure dimensions now, defer compositional semantics to consumers
9. **Bedrock/sediment/topsoil naming** — unifies geological metaphor across entity modeling and data architecture
10. **Read-only from Rust** — Python loader owns the write path; no C-UD concern in the storykeeper crate for bedrock data
