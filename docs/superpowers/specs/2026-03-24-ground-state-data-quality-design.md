# Ground-State Data Quality and Relational Enrichment

**Date:** 2026-03-24
**Ticket:** `2026-03-24-ground-state-data-quality-and-relational-enrichment`
**Branch:** `jcoletaylor/ground-state-data-quality-and-relational-enrichment`
**Scope:** Issues 1-5 (loader hardening, schema corrections, relational enrichment)
**Out of scope:** Spatial topology loader (issue 6), LLM patch fills (issue 7) — carried as context in session notes

## Context

The ground-state persistence layer shipped in PR #29 with ~1500+ entities across 12 primitive types loaded into PostgreSQL via a two-phase Python loader. Post-load review identified 5 data quality issues requiring fixes before the ground-state layer is production-ready for Storykeeper context assembly.

The existing three migrations are edited in-place (not layered) since this is all net-new unreleased work. The database is dropped and recreated on reload.

## Issue Summary

| # | Issue | Category | Fix |
|---|-------|----------|-----|
| 1 | Ontological posture: literal `"null"` strings in entity slugs | Loader bug | Sentinel validation |
| 2 | Place entities: `place_type` column all NULL | Schema/loader mismatch | Rename to `topological_role`, fix extraction |
| 3 | Profiles: `archetype_ref` contains `{}` | Design mismatch | Drop promoted column |
| 4 | State variables: no relational associations | Missing schema | Polymorphic join table |
| 5 | Tropes: `trope_family` as unstructured text | Missing normalization | Lookup table + deterministic normalization |

## Schema Changes

### Migration 1 (`20260323000001_create_ground_state_schema.sql`)

Add two new tables to the reference/join layer.

**`ground_state.trope_families`** — normalized lookup table for trope family classification:

```sql
CREATE TABLE ground_state.trope_families (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            VARCHAR     NOT NULL UNIQUE,
    name            VARCHAR     NOT NULL,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**`ground_state.primitive_state_variable_interactions`** — polymorphic join table linking any primitive entity to state variables it interacts with:

```sql
CREATE TABLE ground_state.primitive_state_variable_interactions (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    primitive_table   VARCHAR     NOT NULL,
    primitive_id      UUID        NOT NULL,
    state_variable_id UUID        NOT NULL REFERENCES ground_state.state_variables(id),
    operation         VARCHAR,
    context           JSONB,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_psvi_primitive
    ON ground_state.primitive_state_variable_interactions(primitive_table, primitive_id);
CREATE INDEX idx_psvi_state_variable
    ON ground_state.primitive_state_variable_interactions(state_variable_id);
```

Design choice: single polymorphic table with `VARCHAR` discriminator instead of per-type join tables. No FK on `primitive_id` (can't FK to multiple tables), but the composite index on `(primitive_table, primitive_id)` gives performant queries. The loader controls all writes from a trusted corpus, so FK enforcement is unnecessary.

### Migration 2 (`20260323000002_create_ground_state_primitive_tables.sql`)

Edit existing primitive table definitions in-place:

- **`place_entities`**: rename `place_type TEXT` to `topological_role VARCHAR` — reflects the actual field in the `EntityProperties` schema (`topological_role: Literal["hub", "endpoint", "connector", "branch", "buffer"]`)
- **`profiles`**: drop `archetype_ref TEXT` column — `provenance` is `list[str]` (not a scalar archetype reference), stays in JSONB payload only
- **`tropes`**: replace `trope_family TEXT` with `trope_family_id UUID REFERENCES ground_state.trope_families(id)` — normalized FK to the new lookup table

### Migration 3 (`20260323000003_create_ground_state_query_functions.sql`)

Update `genre_context()` SQL function:

- Join tropes through `trope_families` to include family name/slug in returned JSONB
- Optionally include state variable interaction summaries per entity where available

## Loader Changes

### Sentinel Validation

New helper in `loader.py`:

```python
_SENTINEL_VALUES = frozenset({"null", "None", "none", "N/A", "n/a", ""})

def _is_valid_value(val: Any) -> bool:
    """Reject sentinel strings that represent missing data."""
    if val is None:
        return False
    if isinstance(val, str) and val.strip() in _SENTINEL_VALUES:
        return False
    return True
```

Applied in:
- `_entity_slug()`: entities with sentinel slugs are skipped with a warning log
- `_promoted_columns()`: sentinel values in promoted fields yield `None`

### Promoted Column Fixes

| Type | Before | After |
|------|--------|-------|
| `ontological-posture` | Extracts `boundary.stability` without validation | Sentinel check on `stability` value |
| `place-entities` | `entity_properties.get("type")` → always None | `entity_properties.get("topological_role")` → column key `topological_role` |
| `profiles` | `entity.get("provenance")` → list into TEXT | Removed — no promoted columns for profiles |
| `tropes` | Colon-split `genre_derivation` → raw text into `trope_family` | Normalize → look up `trope_family_id` UUID from preloaded families |

### Trope Family Normalization

New module: `tools/narrative-data/src/narrative_data/persistence/trope_families.py`

**`build_normalization_map(corpus_path) -> dict[str, str]`**
- Scans all trope files across 30 genres
- Extracts `genre_derivation` values
- Applies normalization: lowercase, strip trailing plurals, collapse known synonyms
- Returns mapping from raw derivation text to canonical family slug

**`extract_trope_families(normalization_map) -> list[dict]`**
- Inverts the normalization map to produce deduplicated canonical families
- Each entry: `{"slug": str, "name": str, "description": str | None}`
- Unclassifiable raw values flagged in loader output for manual review

**Integration**: loader calls `extract_trope_families()` in Phase 1 (alongside genres, clusters, dimensions, state variables), builds a `slug -> UUID` lookup map. Phase 2 trope loading uses this map to resolve `trope_family_id`.

### State Variable Interaction Extraction

New second pass added to Phase 2, after all primitive entities are loaded:

1. Walk each primitive type's loaded entities
2. For entities with `state_variable_interactions` or `state_variable_expression` in JSONB payload, extract variable slug + operation
3. Match slug against the `slug -> UUID` map built from `ground_state.state_variables` in Phase 1
4. Bulk insert into `primitive_state_variable_interactions` with `primitive_table`, `primitive_id`, `state_variable_id`, `operation`
5. Unmatched variable slugs logged as warnings (do not fail the load)

Schema fields that contain state variable references (from exploration):
- `shared.py`: `StateVariableInteraction` — used by multiple types
- `place_entities.py`: `StateVariableExpression` — place-specific variant
- `tropes.py`: `state_variable_interactions: list[StateVariableInteraction]`

## Testing Strategy

### Unit Tests (no DB)

- `test_is_valid_value`: sentinel coverage — `"null"`, `"None"`, `""`, `" null "` (whitespace), plus valid strings pass through
- `test_promoted_columns_place_entities`: extracts `topological_role` correctly from entity dict
- `test_promoted_columns_profiles`: returns empty dict (no promoted columns)
- `test_promoted_columns_tropes`: returns `trope_family_id` UUID after normalization lookup
- `test_promoted_columns_ontological_posture_sentinel`: sentinel `"null"` in `stability` yields None
- `test_build_normalization_map`: known corpus samples collapse correctly (casing, plurals, synonyms)
- `test_extract_trope_families`: produces deduplicated canonical list from normalization map

### DB Integration Tests (real PostgreSQL)

- `test_load_trope_families`: families inserted with unique slugs, queryable by slug
- `test_load_state_variable_interactions`: interactions correctly link primitive entities to state variable UUIDs via composite index
- `test_trope_family_fk`: trope rows reference valid family IDs after load
- `test_unmatched_state_variable_warning`: unknown variable slug logs warning without failing load
- `test_genre_context_includes_families`: `genre_context()` returns trope data with family name/slug attached

### Out of Scope

- Spatial topology loading — not tested because not implemented in this ticket
- LLM patch fills — separate infrastructure, deferred

## Reload Procedure

After all changes:

1. Drop the `storyteller_development` database (or `DROP SCHEMA ground_state CASCADE`)
2. Run sqlx migrations (`cargo make docker-psql` + manual, or sqlx-cli)
3. Run `narrative-data load-ground-state` to reload full corpus with fixes
4. Verify entity counts, spot-check promoted columns, run `genre_context('folk-horror')` to confirm query function works

## Deferred Work (Throughline)

These items are not in scope but inform future direction:

- **Spatial topology loader** (issue 6): entities use `source_setting`/`target_setting` naming. Slug derivation needs to compose from the setting pair. Blocked on understanding whether settings have stable canonical slugs.
- **LLM patch fills** (issue 7): infrastructure built for `valence`, `currencies`, `scale_manifestations`. ~873 LLM calls across full corpus. Worth batching with any future extraction work.
- **Trope family dimensional decomposition**: the ~40-50 canonical families cluster around 9 narrative dimensions. A future pass could add dimension classification to the `trope_families` table for richer queries.
- **Profile-archetype linking**: if profiles should reference archetypes, that's a schema design question for when we understand the Storykeeper's composition layer needs better.
