# BedrockQuery Trait Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `BedrockQuery` trait — the read-only query surface for narrative grammar data — with PostgreSQL backing, including the bedrock rename, dimensional extraction, and typed Record structs.

**Architecture:** Types and trait in `storyteller-core`, query implementation in `storyteller-storykeeper`. SQL schema renamed from `ground_state` to `bedrock`. Python loader updated for rename and dimensional extraction. All queries are slug-based and read-only.

**Tech Stack:** Rust (sqlx, async-trait, serde, uuid, chrono), PostgreSQL, Python (psycopg)

**Spec:** `docs/superpowers/specs/2026-03-25-bedrock-query-trait-design.md`

---

## File Structure

### Phase 1: Bedrock Rename

| Action | File |
|--------|------|
| Rename+Edit | `crates/storyteller-storykeeper/migrations/20260323000001_create_ground_state_schema.sql` → `20260323000001_create_bedrock_schema.sql` |
| Rename+Edit | `crates/storyteller-storykeeper/migrations/20260323000002_create_ground_state_primitive_tables.sql` → `20260323000002_create_bedrock_primitive_tables.sql` |
| Rename+Edit | `crates/storyteller-storykeeper/migrations/20260323000003_create_ground_state_query_functions.sql` → `20260323000003_create_bedrock_query_functions.sql` |
| Edit | `tools/narrative-data/src/narrative_data/persistence/loader.py` — all `ground_state` → `bedrock` |
| Edit | `tools/narrative-data/src/narrative_data/persistence/trope_families.py` — schema refs |
| Edit | `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py` — schema refs |
| Edit | `tools/narrative-data/src/narrative_data/cli.py` — command name + function refs |
| Edit | `tools/narrative-data/tests/test_loader.py` — schema refs |

### Phase 2: Dimensional Extraction

| Action | File |
|--------|------|
| Create | `crates/storyteller-storykeeper/migrations/20260325000001_create_bedrock_dimension_values.sql` |
| Create | `tools/narrative-data/src/narrative_data/persistence/dimension_extraction.py` |
| Edit | `tools/narrative-data/src/narrative_data/persistence/loader.py` — call dimension extraction after entity upsert |
| Create | `tools/narrative-data/tests/test_dimension_extraction.py` |

### Phase 3: Core Types and Traits

| Action | File |
|--------|------|
| Create | `crates/storyteller-core/src/types/bedrock.rs` — all Record structs, BedrockEntity<T>, GenreContext |
| Edit | `crates/storyteller-core/src/types/mod.rs` — add `pub mod bedrock;` |
| Create | `crates/storyteller-core/src/traits/bedrock.rs` — BedrockQuery trait |
| Edit | `crates/storyteller-core/src/traits/mod.rs` — add `pub mod bedrock;` + re-export |

### Phase 4: Storykeeper Query Logic

| Action | File |
|--------|------|
| Create | `crates/storyteller-storykeeper/src/bedrock/mod.rs` — PostgresBedrock struct + trait impl |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/mod.rs` — re-exports |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/genre_context.rs` |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/by_genre.rs` |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/by_slug.rs` |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/dimensions.rs` |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/state_variables.rs` |
| Create | `crates/storyteller-storykeeper/src/bedrock/queries/reference.rs` |
| Edit | `crates/storyteller-storykeeper/src/lib.rs` — add `pub mod bedrock;` + re-export |
| Create | `crates/storyteller-storykeeper/tests/bedrock_integration.rs` — feature-gated integration tests |
| Edit | `crates/storyteller-storykeeper/Cargo.toml` — add `test-db` feature |

---

## Tasks

### Task 1: Rename Migrations (ground_state → bedrock)

**Files:**
- Edit: `crates/storyteller-storykeeper/migrations/20260323000001_create_ground_state_schema.sql`
- Edit: `crates/storyteller-storykeeper/migrations/20260323000002_create_ground_state_primitive_tables.sql`
- Edit: `crates/storyteller-storykeeper/migrations/20260323000003_create_ground_state_query_functions.sql`

- [ ] **Step 1: Rename migration files**

```bash
cd crates/storyteller-storykeeper/migrations
mv 20260323000001_create_ground_state_schema.sql 20260323000001_create_bedrock_schema.sql
mv 20260323000002_create_ground_state_primitive_tables.sql 20260323000002_create_bedrock_primitive_tables.sql
mv 20260323000003_create_ground_state_query_functions.sql 20260323000003_create_bedrock_query_functions.sql
```

- [ ] **Step 2: Replace ground_state with bedrock in all three migration files**

In each file, replace all occurrences of `ground_state` with `bedrock`. This includes:
- Schema name: `CREATE SCHEMA IF NOT EXISTS bedrock;`
- All table names: `bedrock.genres`, `bedrock.archetypes`, etc.
- All index names: update any that reference `ground_state`
- SQL function: `bedrock.genre_context()`
- Comments referencing the schema name

Additionally, **rewrite the `genre_context()` SQL function** to return full rows (all columns per entity), not just the `payload` JSONB. The current function returns `jsonb_build_object('slug', entity_slug, 'data', payload)` which only includes the JSONB payload — but the Rust Record structs need promoted columns too (`id`, `genre_id`, `name`, `archetype_family`, etc.). Change each subquery to build a full-row JSON object, e.g.:
```sql
jsonb_build_object(
    'id', a.id, 'genre_id', a.genre_id, 'cluster_id', a.cluster_id,
    'entity_slug', a.entity_slug, 'name', a.name,
    'archetype_family', a.archetype_family, 'primary_scale', a.primary_scale,
    'payload', a.payload, 'source_hash', a.source_hash,
    'created_at', a.created_at, 'updated_at', a.updated_at
)
```
Do this for all 12 primitive types and the genre itself in the function. The Rust deserialization in Task 9 depends on this.

- [ ] **Step 3: Verify migrations parse correctly**

```bash
# Check no ground_state references remain
grep -r "ground_state" crates/storyteller-storykeeper/migrations/
# Should return nothing
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/migrations/
git commit -m "refactor: rename ground_state schema to bedrock in migrations

Aligns with geological data layer naming: bedrock (grammar),
sediment (vocabulary), topsoil (live state)."
```

### Task 2: Rename Python Loader (ground_state → bedrock)

**Files:**
- Edit: `tools/narrative-data/src/narrative_data/persistence/loader.py`
- Edit: `tools/narrative-data/src/narrative_data/persistence/trope_families.py`
- Edit: `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py`
- Edit: `tools/narrative-data/src/narrative_data/cli.py`
- Edit: `tools/narrative-data/tests/test_loader.py`

- [ ] **Step 1: Replace ground_state with bedrock in loader.py**

Replace all occurrences of `ground_state` with `bedrock` in SQL strings and schema references. Also rename the exported function `load_ground_state()` → `load_bedrock()`.

- [ ] **Step 2: Replace ground_state in trope_families.py and sv_normalization.py**

These files contain SQL queries referencing `ground_state.*` tables. Replace schema prefix in all SQL strings.

- [ ] **Step 3: Update CLI command in cli.py**

Find the `@cli.command("load-ground-state")` decorator and rename to `@cli.command("load-bedrock")`. Update the function name and any internal references.

- [ ] **Step 4: Update test_loader.py**

Replace all `ground_state` references in test SQL queries and function calls.

- [ ] **Step 5: Verify no ground_state references remain in Python**

```bash
grep -r "ground_state" tools/narrative-data/ --include="*.py"
# Should return nothing
```

- [ ] **Step 6: Run Python tests**

```bash
cd tools/narrative-data && uv run pytest tests/ -v
```

- [ ] **Step 7: Commit**

```bash
git add tools/narrative-data/
git commit -m "refactor: rename ground_state to bedrock in Python loader

Updates all SQL schema references, CLI command name, and tests."
```

### Task 3: Drop and Reload Database

**Files:** None (database operations only)

- [ ] **Step 1: Start the database if not running**

```bash
cargo make docker-up
```

- [ ] **Step 2: Drop the old ground_state schema**

```bash
cargo make docker-psql -- -c "DROP SCHEMA IF EXISTS ground_state CASCADE;"
```

- [ ] **Step 3: Run sqlx migrations to create bedrock schema**

```bash
cd crates/storyteller-storykeeper
DATABASE_URL="postgres://storyteller:storyteller@localhost:5435/storyteller_development" sqlx migrate run
```

- [ ] **Step 4: Reload corpus data via Python loader**

```bash
cd tools/narrative-data
uv run narrative-data load-bedrock
```

- [ ] **Step 5: Verify data loaded correctly**

```bash
cargo make docker-psql -- -c "SELECT slug FROM bedrock.genres ORDER BY slug LIMIT 5;"
cargo make docker-psql -- -c "SELECT COUNT(*) FROM bedrock.archetypes;"
cargo make docker-psql -- -c "SELECT bedrock.genre_context('folk-horror') IS NOT NULL AS loaded;"
```

- [ ] **Step 6: Commit** (no file changes, but checkpoint)

```bash
git status  # Should be clean — migration renames already committed
```

### Task 4: Create dimension_values Migration

**Files:**
- Create: `crates/storyteller-storykeeper/migrations/20260325000001_create_bedrock_dimension_values.sql`

- [ ] **Step 1: Write the migration**

Create the `bedrock.dimension_values` table as defined in the spec:

```sql
-- Dimensional value extraction from primitive entity payloads.
-- One row per entity-dimension pair. Polymorphic storage with typed columns.
-- Populated by the Python narrative-data loader during phase 2.

CREATE TABLE bedrock.dimension_values (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    primitive_table   VARCHAR     NOT NULL,
    primitive_id      UUID        NOT NULL,
    genre_id          UUID        NOT NULL REFERENCES bedrock.genres(id),
    dimension_slug    TEXT        NOT NULL,
    dimension_group   TEXT        NOT NULL,
    value_type        VARCHAR     NOT NULL,
    numeric_value     REAL,
    categorical_value TEXT,
    complex_value     JSONB,
    source_path       TEXT,
    tier              VARCHAR     NOT NULL DEFAULT 'core',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dv_entity ON bedrock.dimension_values(primitive_table, primitive_id);
CREATE INDEX idx_dv_dimension ON bedrock.dimension_values(dimension_slug);
CREATE INDEX idx_dv_genre_dimension ON bedrock.dimension_values(genre_id, dimension_slug);
CREATE INDEX idx_dv_value_type ON bedrock.dimension_values(value_type);
CREATE INDEX idx_dv_complex ON bedrock.dimension_values USING gin (complex_value) WHERE complex_value IS NOT NULL;
CREATE UNIQUE INDEX idx_dv_natural_key ON bedrock.dimension_values(primitive_table, primitive_id, dimension_slug);
```

- [ ] **Step 2: Run migration**

```bash
cd crates/storyteller-storykeeper
DATABASE_URL="postgres://storyteller:storyteller@localhost:5435/storyteller_development" sqlx migrate run
```

- [ ] **Step 3: Verify table exists**

```bash
cargo make docker-psql -- -c "\d bedrock.dimension_values"
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/migrations/20260325000001_create_bedrock_dimension_values.sql
git commit -m "feat: add bedrock.dimension_values table for dimensional extraction

Polymorphic storage: numeric_value (normalized/bipolar), categorical_value
(enums), complex_value (weighted tags/sets). One row per entity-dimension pair."
```

### Task 5: Implement Python Dimension Extraction

**Files:**
- Create: `tools/narrative-data/src/narrative_data/persistence/dimension_extraction.py`
- Create: `tools/narrative-data/tests/test_dimension_extraction.py`
- Edit: `tools/narrative-data/src/narrative_data/persistence/loader.py`

- [ ] **Step 1: Write extraction rule tests**

Create `tools/narrative-data/tests/test_dimension_extraction.py` with tests for:
- Extracting normalized dimensions from an archetype payload (personality_profile axes)
- Extracting categorical dimensions from a profile payload (scene dimensions)
- Extracting complex/weighted_tags from genre_dimensions payload (thematic treatments)
- Extracting set dimensions from dynamics payload (currencies)
- Handling missing/null fields gracefully

Test against known fixture data from the corpus (e.g., folk-horror archetype payloads).

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/test_dimension_extraction.py -v
```

- [ ] **Step 3: Implement dimension_extraction.py**

Create `tools/narrative-data/src/narrative_data/persistence/dimension_extraction.py` with:

- A `DimensionRule` dataclass: `(source_path: str, dimension_slug: str, dimension_group: str, value_type: str, tier: str)`
- Per-type extraction rule registries (dictionaries mapping primitive_table → list of DimensionRules)
- An `extract_dimensions(primitive_table: str, entity_id: UUID, genre_id: UUID, payload: dict) -> list[dict]` function that walks the payload using the rules
- A `upsert_dimension_values(conn, rows: list[dict])` function that writes to `bedrock.dimension_values` with ON CONFLICT on the natural key

Start with core-tier dimensions for: archetypes (7 personality axes), profiles (7 scene dimensions), dynamics (edge_type, directionality, valence, evolution_pattern, currencies, network_position), place_entities (communicability channels), spatial_topology (friction, directionality, agency), genre_dimensions (34 dimensions).

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/test_dimension_extraction.py -v
```

- [ ] **Step 5: Integrate extraction into loader.py**

In `loader.py`, after each `_upsert_primitive_row()` call, invoke `extract_dimensions()` and `upsert_dimension_values()`. This goes in the phase 2 loop that walks primitive types per genre.

- [ ] **Step 6: Reload database with dimensional extraction**

```bash
cd tools/narrative-data && uv run narrative-data load-bedrock
```

- [ ] **Step 7: Verify dimensions loaded**

```bash
cargo make docker-psql -- -c "SELECT COUNT(*) FROM bedrock.dimension_values;"
cargo make docker-psql -- -c "SELECT dimension_group, COUNT(*) FROM bedrock.dimension_values GROUP BY dimension_group ORDER BY count DESC;"
cargo make docker-psql -- -c "SELECT primitive_table, dimension_slug, numeric_value FROM bedrock.dimension_values WHERE dimension_slug = 'warmth' LIMIT 5;"
```

- [ ] **Step 8: Commit**

```bash
git add tools/narrative-data/
git commit -m "feat: add dimensional extraction from bedrock payloads

Extracts typed dimension values (normalized, bipolar, categorical,
weighted_tags, set) from primitive entity payloads into
bedrock.dimension_values. Core-tier dimensions for archetypes, profiles,
dynamics, place_entities, spatial_topology, and genre_dimensions."
```

### Task 6: Define Record Structs in storyteller-core

**Files:**
- Create: `crates/storyteller-core/src/types/bedrock.rs`
- Edit: `crates/storyteller-core/src/types/mod.rs`

- [ ] **Step 1: Create bedrock.rs with all Record structs**

Create `crates/storyteller-core/src/types/bedrock.rs` with SPDX header and the following structs, each deriving `Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize`:

**Reference Records:**
- `GenreRecord` — id, slug, name, description, payload, source_hash, created_at, updated_at
- `GenreClusterRecord` — id, slug, name, description
- `StateVariableRecord` — id, slug, name, description, default_range (Option<serde_json::Value>)
- `TropeFamilyRecord` — id, slug, name, description
- `DimensionRecord` — id, slug, name, dimension_group, description

**Primitive Records (12 types):**
- `ArchetypeRecord` — id, genre_id, cluster_id, entity_slug, name, archetype_family, primary_scale, payload, source_hash, created_at, updated_at
- `DynamicRecord` — id, genre_id, cluster_id, entity_slug, name, edge_type, scale, payload, source_hash, created_at, updated_at
- `SettingRecord` — id, genre_id, cluster_id, entity_slug, name, setting_type, payload, source_hash, created_at, updated_at
- `GoalRecord` — id, genre_id, cluster_id, entity_slug, name, goal_scale, payload, source_hash, created_at, updated_at
- `ProfileRecord` — id, genre_id, cluster_id, entity_slug, name, payload, source_hash, created_at, updated_at
- `TropeRecord` — id, genre_id, cluster_id, entity_slug, name, trope_family_id, payload, source_hash, created_at, updated_at
- `NarrativeShapeRecord` — id, genre_id, cluster_id, entity_slug, name, shape_type, beat_count, payload, source_hash, created_at, updated_at
- `OntologicalPostureRecord` — id, genre_id, cluster_id, entity_slug, name, boundary_stability, payload, source_hash, created_at, updated_at
- `SpatialTopologyRecord` — id, genre_id, cluster_id, entity_slug, name, friction_type, directionality_type, payload, source_hash, created_at, updated_at
- `PlaceEntityRecord` — id, genre_id, cluster_id, entity_slug, name, topological_role, payload, source_hash, created_at, updated_at
- `ArchetypeDynamicRecord` — id, genre_id, cluster_id, entity_slug, name, archetype_a, archetype_b, payload, source_hash, created_at, updated_at
- `GenreDimensionRecord` — id, genre_id, payload, source_hash, created_at, updated_at

**Query Result Records:**
- `DimensionValueRecord` — id, primitive_table, primitive_id, genre_id, dimension_slug, dimension_group, value_type, numeric_value, categorical_value, complex_value, source_path, tier, created_at
- `StateVariableInteractionRecord` — state_variable_slug, state_variable_name, operation, context (Option<serde_json::Value>), primitive_table, primitive_id

**Envelope:**
- `BedrockEntity<T>` — id (Uuid), genre_slug (String), entity_slug (String), record (T). Derives Debug, Clone, Serialize, Deserialize. Bounded: `T: Debug + Clone`.

**Composite:**
- `GenreContext` — genre (GenreRecord), archetypes (Vec<ArchetypeRecord>), dynamics (Vec<DynamicRecord>), settings (Vec<SettingRecord>), goals (Vec<GoalRecord>), profiles (Vec<ProfileRecord>), tropes (Vec<TropeRecord>), narrative_shapes (Vec<NarrativeShapeRecord>), ontological_posture (Vec<OntologicalPostureRecord>), spatial_topology (Vec<SpatialTopologyRecord>), place_entities (Vec<PlaceEntityRecord>), archetype_dynamics (Vec<ArchetypeDynamicRecord>), dimensions (Option<GenreDimensionRecord>). Derives Debug, Clone, Serialize, Deserialize.

All `Option` types map to nullable SQL columns. All `Uuid` fields use `uuid::Uuid`. All timestamps use `chrono::DateTime<chrono::Utc>`. The `payload` field on all primitive records is `serde_json::Value` (deprecated — retained as safety net, see spec decision #11).

- [ ] **Step 2: Register the module in types/mod.rs**

Add `pub mod bedrock;` to `crates/storyteller-core/src/types/mod.rs`.

- [ ] **Step 3: Verify compilation**

```bash
cargo check --all-features -p storyteller-core
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-core/src/types/bedrock.rs crates/storyteller-core/src/types/mod.rs
git commit -m "feat: add bedrock Record structs and GenreContext to storyteller-core

12 primitive type records, 5 reference records, DimensionValueRecord,
StateVariableInteractionRecord, BedrockEntity<T> envelope, and GenreContext
composite. All derive sqlx::FromRow + serde for dual deserialization."
```

### Task 7: Define BedrockQuery Trait

**Files:**
- Create: `crates/storyteller-core/src/traits/bedrock.rs`
- Edit: `crates/storyteller-core/src/traits/mod.rs`

- [ ] **Step 1: Create the trait file**

Create `crates/storyteller-core/src/traits/bedrock.rs` with SPDX header and the `BedrockQuery` trait as defined in the spec. All 33+ async methods organized by section: bulk, by-genre (12), by-slug (10), dimensional (4), state variables (2), reference (2).

Import Record types from `crate::types::bedrock::*` and `StorytellerResult` from `crate::errors`.

- [ ] **Step 2: Register the module in traits/mod.rs**

Add to `crates/storyteller-core/src/traits/mod.rs`:
```rust
pub mod bedrock;
pub use bedrock::BedrockQuery;
```

- [ ] **Step 3: Verify compilation and object safety**

```bash
cargo check --all-features -p storyteller-core
```

Add a compile-time assertion in the trait file (or a test):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Verify object safety — this must compile
    fn _assert_object_safe(_: &dyn BedrockQuery) {}
    // Verify Send + Sync
    fn _assert_send_sync<T: Send + Sync>() {}
    fn _assert_bounds() { _assert_send_sync::<Box<dyn BedrockQuery>>(); }
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-core/src/traits/bedrock.rs crates/storyteller-core/src/traits/mod.rs
git commit -m "feat: add BedrockQuery trait for read-only grammar queries

33 async methods: genre_context (bulk), 12 by-genre, 10 by-slug,
4 dimensional, 2 state-variable, 2 reference. Object-safe, Send + Sync."
```

### Task 8: Implement PostgresBedrock — Scaffold and Reference Queries

**Files:**
- Create: `crates/storyteller-storykeeper/src/bedrock/mod.rs`
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/mod.rs`
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/reference.rs`
- Edit: `crates/storyteller-storykeeper/src/lib.rs`

- [ ] **Step 1: Create module structure**

Create `crates/storyteller-storykeeper/src/bedrock/mod.rs`:
- `PostgresBedrock` struct wrapping `PgPool`
- `pub mod queries;`
- Begin `impl BedrockQuery for PostgresBedrock` (initially with `todo!()` for all methods except reference queries)

Create `crates/storyteller-storykeeper/src/bedrock/queries/mod.rs` with all sub-module declarations.

- [ ] **Step 2: Implement reference queries**

Create `crates/storyteller-storykeeper/src/bedrock/queries/reference.rs`:
- `pub async fn genres(pool: &PgPool) -> StorytellerResult<Vec<GenreRecord>>`
- `pub async fn trope_families(pool: &PgPool) -> StorytellerResult<Vec<TropeFamilyRecord>>`

Wire these into the trait impl.

- [ ] **Step 3: Update lib.rs**

Add to `crates/storyteller-storykeeper/src/lib.rs`:
```rust
pub mod bedrock;
pub use bedrock::PostgresBedrock;
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check --all-features -p storyteller-storykeeper
```

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-storykeeper/src/bedrock/ crates/storyteller-storykeeper/src/lib.rs
git commit -m "feat: scaffold PostgresBedrock with reference queries

PostgresBedrock struct wrapping PgPool. genres() and trope_families()
implemented. Remaining trait methods stubbed with todo!()."
```

### Task 9: Implement genre_context Query

**Files:**
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/genre_context.rs`
- Edit: `crates/storyteller-storykeeper/src/bedrock/mod.rs`

- [ ] **Step 1: Implement the SQL function wrapper**

Create `genre_context.rs` with a function that calls `SELECT bedrock.genre_context($1)`, receives a single `serde_json::Value`, and deserializes it into `GenreContext`. Handle the NULL return (unknown genre) by returning `StorytellerError::EntityNotFound(format!("genre not found: {genre_slug}"))`.

The SQL function returns JSONB with keys matching primitive type names. Each value is an array of `{"slug": ..., "data": ...}` objects. Parse each array into the corresponding `Vec<Record>` by extracting fields from the `data` object.

Note: The `GenreRecord` for the genre itself comes from the `genre` key in the JSONB. The `dimensions` key is a bare JSONB object (not wrapped in slug/data), parsed into `GenreDimensionRecord`.

- [ ] **Step 2: Wire into trait impl**

Replace the `todo!()` for `genre_context()` in the trait impl with delegation to the query function.

- [ ] **Step 3: Verify compilation**

```bash
cargo check --all-features -p storyteller-storykeeper
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/src/bedrock/
git commit -m "feat: implement genre_context() via bedrock SQL function

Single round-trip query returning all 12 primitive types. Returns
NotFound for unknown genre slugs."
```

### Task 10: Implement by_genre Queries (11 types)

**Files:**
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/by_genre.rs`
- Edit: `crates/storyteller-storykeeper/src/bedrock/mod.rs`

- [ ] **Step 1: Implement all 11 by_genre query functions**

Each function follows the pattern:
```rust
pub async fn archetypes_by_genre(pool: &PgPool, genre_slug: &str) -> StorytellerResult<Vec<ArchetypeRecord>> {
    let records = sqlx::query_as::<_, ArchetypeRecord>(
        "SELECT a.* FROM bedrock.archetypes a
         JOIN bedrock.genres g ON a.genre_id = g.id
         WHERE g.slug = $1 AND a.cluster_id IS NULL
         ORDER BY a.entity_slug"
    )
    .bind(genre_slug)
    .fetch_all(pool)
    .await
    .map_err(|e| StorytellerError::Database(e.to_string()))?;
    Ok(records)
}
```

Implement for all 12 types: archetypes, dynamics, settings, goals, profiles, tropes, narrative_shapes, ontological_posture, spatial_topology, place_entities, archetype_dynamics. Each uses the correct table name and column list.

- [ ] **Step 2: Wire all into trait impl**

Replace the `todo!()` stubs for all `*_by_genre()` methods.

- [ ] **Step 3: Verify compilation**

```bash
cargo check --all-features -p storyteller-storykeeper
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/src/bedrock/
git commit -m "feat: implement 12 by_genre query methods

Each queries bedrock.{table} joined to genres on slug, filtered to
genre-specific entities (cluster_id IS NULL)."
```

### Task 11: Implement by_slug Queries

**Files:**
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/by_slug.rs`
- Edit: `crates/storyteller-storykeeper/src/bedrock/mod.rs`

- [ ] **Step 1: Implement all 10 by_slug query functions**

Same pattern as by_genre but with `fetch_optional` and an additional `entity_slug` bind:
```rust
pub async fn archetype_by_slug(pool: &PgPool, genre_slug: &str, entity_slug: &str) -> StorytellerResult<Option<ArchetypeRecord>> {
    let record = sqlx::query_as::<_, ArchetypeRecord>(
        "SELECT a.* FROM bedrock.archetypes a
         JOIN bedrock.genres g ON a.genre_id = g.id
         WHERE g.slug = $1 AND a.entity_slug = $2 AND a.cluster_id IS NULL"
    )
    .bind(genre_slug)
    .bind(entity_slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| StorytellerError::Database(e.to_string()))?;
    Ok(record)
}
```

Implement for: archetypes, dynamics, settings, goals, profiles, tropes, narrative_shapes, ontological_posture, spatial_topology, place_entities (10 types — archetype_dynamics excluded per spec).

- [ ] **Step 2: Wire into trait impl**

- [ ] **Step 3: Verify compilation**

```bash
cargo check --all-features -p storyteller-storykeeper
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/src/bedrock/
git commit -m "feat: implement 10 by_slug query methods

Entity lookup by genre_slug + entity_slug. Returns Option<Record>."
```

### Task 12: Implement Dimensional and State Variable Queries

**Files:**
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/dimensions.rs`
- Create: `crates/storyteller-storykeeper/src/bedrock/queries/state_variables.rs`
- Edit: `crates/storyteller-storykeeper/src/bedrock/mod.rs`

- [ ] **Step 1: Implement dimensional queries**

In `dimensions.rs`:
- `genre_dimensions(pool, genre_slug)` — joins genre_dimensions to genres
- `dimensions_for_entity(pool, primitive_table, entity_slug, genre_slug)` — subquery to resolve entity_slug→id, then select from dimension_values
- `entities_by_dimension(pool, dimension_slug, genre_slug)` — select from dimension_values joined to genres
- `dimensional_intersection(pool, dimension_slugs, genre_slug)` — GROUP BY/HAVING COUNT pattern

- [ ] **Step 2: Implement state variable queries**

In `state_variables.rs`:
- `state_variables(pool)` — simple SELECT from bedrock.state_variables
- `state_variable_interactions(pool, genre_slug, state_variable_slug)` — join through primitive_state_variable_interactions to state_variables, filtering by genre. The `primitive_state_variable_interactions` table uses `primitive_table` as a discriminator and `primitive_id` as a soft FK. To filter by genre, use an `IN` subquery per known primitive table, e.g.:
  ```sql
  SELECT sv.slug AS state_variable_slug, sv.name AS state_variable_name,
         psvi.operation, psvi.context, psvi.primitive_table, psvi.primitive_id
  FROM bedrock.primitive_state_variable_interactions psvi
  JOIN bedrock.state_variables sv ON psvi.state_variable_id = sv.id
  WHERE sv.slug = $2
    AND (
      (psvi.primitive_table = 'archetypes' AND psvi.primitive_id IN (SELECT id FROM bedrock.archetypes WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1)))
      OR (psvi.primitive_table = 'dynamics' AND psvi.primitive_id IN (SELECT id FROM bedrock.dynamics WHERE genre_id = (SELECT id FROM bedrock.genres WHERE slug = $1)))
      -- ... repeat for each primitive table that has state variable interactions
    )
  ```
  This is verbose but avoids the need for dynamic SQL. The genre subquery is the same each time and PostgreSQL will plan it efficiently.

- [ ] **Step 3: Wire all into trait impl, replacing remaining todo!() stubs**

- [ ] **Step 4: Verify compilation — no todo!() stubs should remain**

```bash
cargo check --all-features -p storyteller-storykeeper
# Also verify no todo!() remain:
grep -r "todo!()" crates/storyteller-storykeeper/src/bedrock/
```

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-storykeeper/src/bedrock/
git commit -m "feat: implement dimensional and state variable queries

genre_dimensions, dimensions_for_entity, entities_by_dimension,
dimensional_intersection, state_variables, state_variable_interactions.
All BedrockQuery trait methods now implemented."
```

### Task 13: Integration Tests

**Files:**
- Create: `crates/storyteller-storykeeper/tests/bedrock_integration.rs`
- Edit: `crates/storyteller-storykeeper/Cargo.toml`

- [ ] **Step 1: Add test-db feature to Cargo.toml**

Add to `crates/storyteller-storykeeper/Cargo.toml`:
```toml
[features]
test-db = []
```

- [ ] **Step 2: Write integration tests**

Create `crates/storyteller-storykeeper/tests/bedrock_integration.rs` with `#![cfg(feature = "test-db")]`:

- `setup()` helper that connects to DATABASE_URL or default, returns `PostgresBedrock`
- `genre_context_returns_all_types()` — folk-horror context has non-empty archetypes, dynamics, tropes, etc.
- `genre_context_not_found_for_unknown_slug()` — returns NotFound error
- `archetypes_by_genre_returns_expected_entities()` — folk-horror archetypes not empty
- `archetype_by_slug_returns_known_entity()` — look up a specific known archetype
- `archetype_by_slug_returns_none_for_unknown()` — None for nonexistent slug
- `genre_dimensions_returns_profile()` — folk-horror has dimension data
- `state_variables_returns_canonical_set()` — at least 8 state variables
- `dimensions_for_entity_returns_values()` — known archetype has personality dimensions
- `genres_returns_all_30()` — reference query returns 30 genres
- Contract test: `Arc<dyn BedrockQuery>` compiles

- [ ] **Step 3: Run integration tests**

```bash
cd crates/storyteller-storykeeper
DATABASE_URL="postgres://storyteller:storyteller@localhost:5435/storyteller_development" cargo test --features test-db -- --nocapture
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/tests/bedrock_integration.rs crates/storyteller-storykeeper/Cargo.toml
git commit -m "feat: add integration tests for BedrockQuery

11 tests covering genre_context, by_genre, by_slug, dimensions,
state variables, reference queries, and trait object safety.
Feature-gated behind test-db."
```

### Task 14: Full Workspace Verification

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace check**

```bash
cargo check --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
cargo test --all-features
```

- [ ] **Step 2: Run integration tests**

```bash
cd crates/storyteller-storykeeper
DATABASE_URL="postgres://storyteller:storyteller@localhost:5435/storyteller_development" cargo test --features test-db -v
```

- [ ] **Step 3: Run Python tests**

```bash
cd tools/narrative-data && uv run pytest -v
```

- [ ] **Step 4: Final commit if any fixes needed**

```bash
# Only if clippy/fmt required changes
git add -A && git commit -m "fix: address clippy and formatting issues"
```
