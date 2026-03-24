# Ground-State Data Quality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 5 data quality issues in the ground-state persistence layer so promoted columns are accurate, state variable relationships are queryable, and trope families are normalized.

**Architecture:** Edit 3 existing SQL migrations in-place (new tables + column fixes), add sentinel validation and trope family normalization to the Python loader, extract state variable interactions as a second pass in Phase 2. DB dropped and recreated on reload.

**Tech Stack:** PostgreSQL (sqlx migrations), Python (psycopg, Pydantic), pytest

**Spec:** `docs/superpowers/specs/2026-03-24-ground-state-data-quality-design.md`

---

### File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `crates/storyteller-storykeeper/migrations/20260323000001_create_ground_state_schema.sql` | Add `trope_families` + `primitive_state_variable_interactions` tables |
| Modify | `crates/storyteller-storykeeper/migrations/20260323000002_create_ground_state_primitive_tables.sql` | Rename `place_type` → `topological_role`, drop `archetype_ref`, replace `trope_family` with `trope_family_id` FK |
| Modify | `crates/storyteller-storykeeper/migrations/20260323000003_create_ground_state_query_functions.sql` | Update `genre_context()` to join tropes through `trope_families` |
| Modify | `tools/narrative-data/src/narrative_data/persistence/loader.py` | Sentinel validation, promoted column fixes, state variable interaction extraction |
| Create | `tools/narrative-data/src/narrative_data/persistence/trope_families.py` | Normalization map + family extraction from corpus |
| Modify | `tools/narrative-data/tests/test_loader.py` | All new tests |

---

### Task 1: Sentinel Validation in Loader

**Files:**
- Modify: `tools/narrative-data/tests/test_loader.py`
- Modify: `tools/narrative-data/src/narrative_data/persistence/loader.py:97-103`

- [ ] **Step 1: Write failing tests for `_is_valid_value`**

Add to `TestLoaderHelpers` class in `test_loader.py`:

```python
from narrative_data.persistence.loader import _is_valid_value

class TestSentinelValidation:
    def test_rejects_none(self) -> None:
        assert _is_valid_value(None) is False

    def test_rejects_literal_null_string(self) -> None:
        assert _is_valid_value("null") is False

    def test_rejects_none_string(self) -> None:
        assert _is_valid_value("None") is False

    def test_rejects_lowercase_none(self) -> None:
        assert _is_valid_value("none") is False

    def test_rejects_empty_string(self) -> None:
        assert _is_valid_value("") is False

    def test_rejects_whitespace_padded_sentinel(self) -> None:
        assert _is_valid_value(" null ") is False

    def test_rejects_na(self) -> None:
        assert _is_valid_value("N/A") is False

    def test_accepts_valid_string(self) -> None:
        assert _is_valid_value("firm_defensive") is True

    def test_accepts_non_string_truthy(self) -> None:
        assert _is_valid_value(42) is True

    def test_accepts_list(self) -> None:
        assert _is_valid_value(["a", "b"]) is True
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestSentinelValidation -v`
Expected: ImportError — `_is_valid_value` does not exist yet

- [ ] **Step 3: Implement `_is_valid_value` and update `_entity_slug`**

In `loader.py`, add after `_slugify` (around line 95):

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

Add `Any` to the typing import at top of file.

Update `_entity_slug` to use sentinel check:

```python
def _entity_slug(entity: dict) -> str | None:
    """Derive the entity slug from canonical_name, name, pairing_name, or default_subject."""
    for key in ("canonical_name", "pairing_name", "name", "default_subject"):
        val = entity.get(key)
        if _is_valid_value(val) and isinstance(val, str):
            return _slugify(val)
    return None
```

Similarly update `_entity_name`:

```python
def _entity_name(type_name: str, entity: dict) -> str:
    """Return a human-readable name for the entity."""
    for key in ("canonical_name", "pairing_name", "name", "default_subject"):
        val = entity.get(key)
        if _is_valid_value(val) and isinstance(val, str):
            return val.strip()
    return "unknown"
```

- [ ] **Step 4: Write test for sentinel in entity slug derivation**

Add to `TestLoaderHelpers`:

```python
def test_entity_slug_skips_sentinel_null(self) -> None:
    """Entities with 'null' in primary field fall back to next candidate."""
    entity = {"default_subject": "null", "canonical_name": "The Real Name"}
    assert _entity_slug(entity) == "the-real-name"

def test_entity_slug_returns_none_when_all_sentinels(self) -> None:
    """Returns None when all candidate fields are sentinels."""
    entity = {"default_subject": "null", "name": "None"}
    assert _entity_slug(entity) is None
```

- [ ] **Step 5: Run all tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestSentinelValidation tests/test_loader.py::TestLoaderHelpers -v`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/loader.py tools/narrative-data/tests/test_loader.py
git commit -m "feat: add sentinel validation to ground-state loader"
```

---

### Task 2: Fix Promoted Columns (Issues 1-3)

**Files:**
- Modify: `tools/narrative-data/tests/test_loader.py`
- Modify: `tools/narrative-data/src/narrative_data/persistence/loader.py:111-174`

- [ ] **Step 1: Write failing tests for fixed promoted columns**

Add to or update in `TestLoaderHelpers`:

```python
def test_promoted_columns_ontological_posture_sentinel(self) -> None:
    """Sentinel 'null' in stability yields None."""
    entity = {"self_other_boundary": {"stability": "null"}}
    cols = _promoted_columns("ontological-posture", entity)
    assert cols["boundary_stability"] is None

def test_promoted_columns_place_entities_topological_role(self) -> None:
    """Place entities extract topological_role, not type."""
    entity = {"entity_properties": {"topological_role": "hub", "has_agency": True}}
    cols = _promoted_columns("place-entities", entity)
    assert cols == {"topological_role": "hub"}

def test_promoted_columns_place_entities_missing_role(self) -> None:
    """Place entities with no topological_role get None."""
    entity = {"entity_properties": {"has_agency": False}}
    cols = _promoted_columns("place-entities", entity)
    assert cols == {"topological_role": None}

def test_promoted_columns_profiles_empty(self) -> None:
    """Profiles have no promoted columns (archetype_ref removed)."""
    entity = {"provenance": ["Tarot-derived"], "name": "Foo"}
    cols = _promoted_columns("profiles", entity)
    assert cols == {}
```

- [ ] **Step 2: Run tests to verify failures**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestLoaderHelpers::test_promoted_columns_ontological_posture_sentinel tests/test_loader.py::TestLoaderHelpers::test_promoted_columns_place_entities_topological_role tests/test_loader.py::TestLoaderHelpers::test_promoted_columns_profiles_empty -v`
Expected: FAIL — ontological-posture returns `"null"` not `None`, place-entities still returns `{"place_type": None}`, profiles still returns `{"archetype_ref": ...}`

- [ ] **Step 3: Fix `_promoted_columns` for all three types**

In `loader.py`, update the `_promoted_columns` function:

**Ontological posture** (lines 153-156) — add sentinel check:
```python
if type_name == "ontological-posture":
    boundary = entity.get("self_other_boundary") or {}
    stability = boundary.get("stability") if isinstance(boundary, dict) else None
    if not _is_valid_value(stability):
        stability = None
    return {"boundary_stability": stability}
```

**Place entities** (lines 165-168) — fix field name and column name:
```python
if type_name == "place-entities":
    entity_props = entity.get("entity_properties") or {}
    topological_role = entity_props.get("topological_role") if isinstance(entity_props, dict) else None
    return {"topological_role": topological_role}
```

**Profiles** (lines 137-138) — remove promoted columns:
```python
if type_name == "profiles":
    return {}
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestLoaderHelpers -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/loader.py tools/narrative-data/tests/test_loader.py
git commit -m "fix: correct promoted columns for ontological-posture, place-entities, profiles"
```

---

### Task 3: Migration Changes — Schema Fixes

**Files:**
- Modify: `crates/storyteller-storykeeper/migrations/20260323000001_create_ground_state_schema.sql`
- Modify: `crates/storyteller-storykeeper/migrations/20260323000002_create_ground_state_primitive_tables.sql`

- [ ] **Step 1: Add `trope_families` table to migration 1**

Append before the final line of `20260323000001_create_ground_state_schema.sql` (after `idx_dimensions_group` index):

```sql
-- ---------------------------------------------------------------------------
-- trope_families
-- ---------------------------------------------------------------------------
-- Normalized lookup for trope family classification. Each family maps to
-- a canonical narrative dimension (e.g. "Locus of Power", "Thematic Dimension").
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.trope_families (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            VARCHAR     NOT NULL UNIQUE,
    name            VARCHAR     NOT NULL,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- primitive_state_variable_interactions
-- ---------------------------------------------------------------------------
-- Polymorphic join table: links any primitive entity to state variables it
-- interacts with. primitive_table is the discriminator (e.g. 'tropes',
-- 'dynamics'). No FK on primitive_id (can't FK to multiple tables).
-- ---------------------------------------------------------------------------
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

- [ ] **Step 2: Fix primitive table definitions in migration 2**

In `20260323000002_create_ground_state_primitive_tables.sql`:

**profiles table** (line 121): delete the `archetype_ref  TEXT,` line.

**tropes table** (line 143): replace `trope_family  TEXT,` with:
```sql
    trope_family_id UUID        REFERENCES ground_state.trope_families(id),
```

**place_entities table** (line 233): replace `place_type    TEXT,` with:
```sql
    topological_role VARCHAR,
```

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-storykeeper/migrations/
git commit -m "feat: add trope_families and state variable interactions tables, fix promoted columns"
```

---

### Task 4: Trope Family Normalization Module

**Files:**
- Create: `tools/narrative-data/src/narrative_data/persistence/trope_families.py`
- Modify: `tools/narrative-data/tests/test_loader.py`

- [ ] **Step 1: Write failing tests for trope family normalization**

```python
from narrative_data.persistence.trope_families import (
    build_normalization_map,
    extract_trope_families,
    normalize_family_name,
)


class TestTropeFamilyNormalization:
    def test_normalize_strips_plural_dimensions(self) -> None:
        """'Temporal Dimensions' normalizes to 'temporal-dimension'."""
        assert normalize_family_name("Temporal Dimensions") == "temporal-dimension"

    def test_normalize_strips_plural_affordances(self) -> None:
        """'World Affordances' normalizes to 'world-affordance'."""
        assert normalize_family_name("World Affordances") == "world-affordance"

    def test_normalize_handles_casing(self) -> None:
        """'epistemological stance' normalizes to 'epistemological-stance'."""
        assert normalize_family_name("epistemological stance") == "epistemological-stance"

    def test_normalize_collapses_agency_variants(self) -> None:
        """Both 'Agency Dimension' and 'Agency Dimensions' map to same slug."""
        assert normalize_family_name("Agency Dimension") == "agency-dimension"
        assert normalize_family_name("Agency Dimensions") == "agency-dimension"

    def test_build_normalization_map_from_corpus(self, tmp_path: Path) -> None:
        """Builds mapping from raw genre_derivation to canonical slugs."""
        corpus_dir = tmp_path / "corpus"
        tropes_dir = corpus_dir / "genres" / "folk-horror"
        tropes_dir.mkdir(parents=True)
        tropes = [
            {"name": "T1", "genre_derivation": "Temporal Dimensions: Seasonal"},
            {"name": "T2", "genre_derivation": "Temporal Dimension: Cyclical"},
            {"name": "T3", "genre_derivation": "Locus of Power: Community"},
        ]
        (tropes_dir / "tropes.json").write_text(json.dumps(tropes))

        nmap = build_normalization_map(corpus_dir)

        assert nmap["Temporal Dimensions: Seasonal"] == "temporal-dimension"
        assert nmap["Temporal Dimension: Cyclical"] == "temporal-dimension"
        assert nmap["Locus of Power: Community"] == "locus-of-power"

    def test_extract_trope_families_deduplicates(self) -> None:
        """Extracts unique families from normalization map."""
        nmap = {
            "Temporal Dimensions: Seasonal": "temporal-dimension",
            "Temporal Dimension: Cyclical": "temporal-dimension",
            "Locus of Power: Community": "locus-of-power",
        }
        families = extract_trope_families(nmap)
        slugs = [f["slug"] for f in families]

        assert len(slugs) == 2
        assert "temporal-dimension" in slugs
        assert "locus-of-power" in slugs

    def test_extract_trope_families_have_names(self) -> None:
        """Each family dict has slug, name, description."""
        nmap = {"Locus of Power: X": "locus-of-power"}
        families = extract_trope_families(nmap)
        f = families[0]
        assert f["slug"] == "locus-of-power"
        assert f["name"] == "Locus of Power"
        assert "description" in f

    def test_long_derivation_maps_to_unclassified(self) -> None:
        """Derivation strings >100 chars (LLM prose) map to 'unclassified'."""
        long_text = "Directly addresses the Agency Dimensions and Boundary Conditions (Melodrama vs. Tragedy). The hero must be competent to fall from greatness."
        assert normalize_family_name(long_text) == "unclassified"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestTropeFamilyNormalization -v`
Expected: ImportError — module does not exist

- [ ] **Step 3: Implement `trope_families.py`**

Create `tools/narrative-data/src/narrative_data/persistence/trope_families.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Trope family normalization: extracts canonical families from genre_derivation values."""

import json
import logging
import re
from pathlib import Path

log = logging.getLogger(__name__)

# Threshold: derivation strings longer than this are LLM prose, not family names
_MAX_FAMILY_LENGTH = 100


def normalize_family_name(raw: str) -> str:
    """Normalize a genre_derivation string to a canonical family slug.

    Strategy:
    1. Take the part before the first colon (if present)
    2. Strip whitespace, lowercase
    3. Collapse plural 'dimensions' → 'dimension', 'affordances' → 'affordance'
    4. Slugify (spaces → hyphens)
    5. Strings > _MAX_FAMILY_LENGTH are classified as 'unclassified'
    """
    text = raw.strip()
    if len(text) > _MAX_FAMILY_LENGTH:
        return "unclassified"

    # Take part before colon
    if ":" in text:
        text = text.split(":")[0].strip()

    # Lowercase
    text = text.lower()

    # Normalize plurals
    text = re.sub(r"\bdimensions\b", "dimension", text)
    text = re.sub(r"\baffordances\b", "affordance", text)
    text = re.sub(r"\bstances\b", "stance", text)

    # Slugify
    slug = text.replace(" ", "-").replace("_", "-")
    slug = re.sub(r"-+", "-", slug).strip("-")

    return slug if slug else "unclassified"


def _slug_to_display_name(slug: str) -> str:
    """Convert a slug back to a display name: 'locus-of-power' → 'Locus of Power'."""
    return " ".join(
        word.capitalize() if i == 0 or len(word) > 2 else word
        for i, word in enumerate(slug.split("-"))
    )


def build_normalization_map(corpus_dir: Path) -> dict[str, str]:
    """Scan all trope files and build raw derivation → canonical slug mapping."""
    nmap: dict[str, str] = {}
    genres_dir = corpus_dir / "genres"
    if not genres_dir.exists():
        return nmap

    for genre_dir in sorted(genres_dir.iterdir()):
        if not genre_dir.is_dir():
            continue
        tropes_file = genre_dir / "tropes.json"
        if not tropes_file.exists():
            continue
        try:
            data = json.loads(tropes_file.read_text())
        except (json.JSONDecodeError, OSError) as exc:
            log.warning("Skipping %s: %s", tropes_file, exc)
            continue

        if not isinstance(data, list):
            continue

        for trope in data:
            derivation = trope.get("genre_derivation")
            if isinstance(derivation, str) and derivation.strip():
                nmap[derivation] = normalize_family_name(derivation)

    return nmap


def extract_trope_families(normalization_map: dict[str, str]) -> list[dict]:
    """Deduplicate canonical families from the normalization map."""
    seen: dict[str, dict] = {}
    for raw, slug in normalization_map.items():
        if slug not in seen:
            seen[slug] = {
                "slug": slug,
                "name": _slug_to_display_name(slug),
                "description": None,
            }
    return sorted(seen.values(), key=lambda f: f["slug"])
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestTropeFamilyNormalization -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/trope_families.py tools/narrative-data/tests/test_loader.py
git commit -m "feat: add trope family normalization module"
```

---

### Task 5: Integrate Trope Families into Loader Phase 1

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/persistence/loader.py:191-227`
- Modify: `tools/narrative-data/tests/test_loader.py`

- [ ] **Step 1: Write failing test for trope family loading**

Add to test file (new class or extend `TestLoadReferenceEntities`):

```python
class TestTropeFamilyLoading:
    def test_upsert_trope_families(self, tmp_path: Path) -> None:
        """Trope families extracted from corpus normalization map."""
        corpus_dir = _minimal_corpus(tmp_path)
        # _minimal_corpus already has a trope with genre_derivation "Temporal: Seasonal"

        from narrative_data.persistence.trope_families import build_normalization_map, extract_trope_families
        nmap = build_normalization_map(corpus_dir)
        families = extract_trope_families(nmap)
        assert len(families) >= 1
        assert families[0]["slug"] == "temporal"

    @_skip_db
    def test_load_reference_entities_includes_trope_families(self, db_conn, tmp_path: Path) -> None:
        """load_reference_entities now loads trope families and returns family slug→UUID map."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        # Should have trope family entries in slug_map with tf: prefix
        tf_keys = [k for k in slug_map if k.startswith("tf:")]
        assert len(tf_keys) >= 1

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.trope_families")
            row = cur.fetchone()
        assert row["n"] >= 1
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestTropeFamilyLoading -v`
Expected: FAIL — `load_reference_entities` does not load trope families yet

- [ ] **Step 3: Implement trope family loading in `load_reference_entities`**

In `loader.py`, add import at top:

```python
from narrative_data.persistence.trope_families import (
    build_normalization_map,
    extract_trope_families,
)
```

Add `_upsert_trope_families` function (after `_upsert_dimensions`):

```python
def _upsert_trope_families(
    conn: psycopg.Connection,
    families: list[dict],
    dry_run: bool = False,
) -> dict[str, str]:
    """Upsert trope_families; return slug→UUID map with 'tf:' prefix."""
    slug_map: dict[str, str] = {}
    for fam in families:
        slug = fam["slug"]
        if dry_run:
            slug_map[f"tf:{slug}"] = "dry-run-uuid"
            continue
        with conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                """
                INSERT INTO ground_state.trope_families (slug, name, description)
                VALUES (%s, %s, %s)
                ON CONFLICT (slug) DO UPDATE
                  SET name = EXCLUDED.name,
                      description = EXCLUDED.description,
                      updated_at = now()
                RETURNING id
                """,
                (slug, fam["name"], fam.get("description")),
            )
            if cur.rowcount == 0:
                cur.execute(
                    "SELECT id FROM ground_state.trope_families WHERE slug = %s", (slug,)
                )
            row = cur.fetchone()
            slug_map[f"tf:{row['id']}"] = str(row["id"])
            slug_map[f"tf:{slug}"] = str(row["id"])
    return slug_map
```

Update `load_reference_entities` to call it (after state variables, before dimensions):

```python
def load_reference_entities(
    conn: psycopg.Connection,
    corpus_dir: Path,
    dry_run: bool = False,
) -> dict[str, str]:
    slug_map: dict[str, str] = {}

    genres = extract_genres(corpus_dir)
    slug_map.update(_upsert_genres(conn, genres, dry_run=dry_run))

    clusters = extract_cluster_metadata()
    cluster_map = _upsert_clusters(conn, clusters, slug_map, dry_run=dry_run)
    slug_map.update(cluster_map)

    state_variables = extract_state_variables(corpus_dir)
    _upsert_state_variables(conn, state_variables, dry_run=dry_run)

    # Trope families (scan corpus for genre_derivation values)
    nmap = build_normalization_map(corpus_dir)
    families = extract_trope_families(nmap)
    tf_map = _upsert_trope_families(conn, families, dry_run=dry_run)
    slug_map.update(tf_map)

    dimensions = extract_dimensions()
    _upsert_dimensions(conn, dimensions, dry_run=dry_run)

    if not dry_run:
        conn.commit()

    log.info(
        "Phase 1 complete: %d genres, %d clusters, %d state_variables, %d trope_families, %d dimensions",
        len(genres),
        len(clusters),
        len(state_variables),
        len(families),
        len(dimensions),
    )
    return slug_map
```

Also store the normalization map on the slug_map for Phase 2 trope loading. Add it as a module-level variable or pass it through. Simplest approach: store the normalization map as a JSON-encoded string in slug_map under a special key:

```python
    # Store normalization map for Phase 2 trope promoted column lookup
    slug_map["__trope_nmap__"] = json.dumps(nmap)
```

- [ ] **Step 4: Update trope promoted columns to use family ID**

In `_promoted_columns`, update the tropes case:

```python
if type_name == "tropes":
    derivation = entity.get("genre_derivation")
    if isinstance(derivation, str) and derivation.strip():
        from narrative_data.persistence.trope_families import normalize_family_name
        family_slug = normalize_family_name(derivation)
        # Look up UUID from slug_map (passed via extra context)
        # This will be resolved in load_primitive_type — for now return the slug
        return {"trope_family_id": family_slug}
    return {"trope_family_id": None}
```

Note: The actual UUID resolution happens in `load_primitive_type` where `slug_map` is available. The `_promoted_columns` function needs access to the slug_map. Refactor: pass `slug_map` as an optional parameter to `_promoted_columns`:

Update signature: `def _promoted_columns(type_name: str, entity: dict, slug_map: dict | None = None) -> dict:`

And in the tropes case:
```python
if type_name == "tropes":
    derivation = entity.get("genre_derivation")
    if isinstance(derivation, str) and derivation.strip():
        from narrative_data.persistence.trope_families import normalize_family_name
        family_slug = normalize_family_name(derivation)
        family_id = slug_map.get(f"tf:{family_slug}") if slug_map else None
        return {"trope_family_id": family_id}
    return {"trope_family_id": None}
```

Update the call site in `load_primitive_type` (line 651):
```python
extra = _promoted_columns(type_name, entity, slug_map=slug_map)
```

- [ ] **Step 5: Update existing trope promoted column test**

The existing test at `test_promoted_columns_tropes` (if any) needs updating. The test in `_minimal_corpus` has `genre_derivation: "Temporal: Seasonal"`. Update or add:

```python
def test_promoted_columns_tropes_with_slug_map(self) -> None:
    """Tropes: trope_family_id resolved from slug_map."""
    entity = {"genre_derivation": "Temporal Dimensions: Seasonal"}
    slug_map = {"tf:temporal-dimension": "fake-uuid-123"}
    cols = _promoted_columns("tropes", entity, slug_map=slug_map)
    assert cols["trope_family_id"] == "fake-uuid-123"

def test_promoted_columns_tropes_no_slug_map(self) -> None:
    """Tropes without slug_map get None for family_id."""
    entity = {"genre_derivation": "Temporal: Seasonal"}
    cols = _promoted_columns("tropes", entity)
    assert cols["trope_family_id"] is None
```

- [ ] **Step 6: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py -v -k "trope or TropeFamily"`
Expected: All PASS

- [ ] **Step 7: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/loader.py tools/narrative-data/tests/test_loader.py
git commit -m "feat: integrate trope family normalization into loader Phase 1"
```

---

### Task 6: State Variable Interaction Extraction

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/persistence/loader.py`
- Modify: `tools/narrative-data/tests/test_loader.py`

- [ ] **Step 1: Write failing tests for state variable interaction extraction**

```python
class TestStateVariableInteractionExtraction:
    def test_extract_interactions_from_structured_type(self) -> None:
        """Extracts variable_id and operation from StateVariableInteraction shape."""
        from narrative_data.persistence.loader import _extract_state_variable_interactions

        entity = {
            "state_variable_interactions": [
                {"variable_id": "trust", "operation": "depletes", "description": "Erodes trust."},
                {"variable_id": "safety", "operation": "consumes"},
            ]
        }
        interactions = _extract_state_variable_interactions("tropes", entity)
        assert len(interactions) == 2
        assert interactions[0] == {"variable_slug": "trust", "operation": "depletes", "context": None}
        assert interactions[1] == {"variable_slug": "safety", "operation": "consumes", "context": None}

    def test_extract_interactions_from_expression_type(self) -> None:
        """Extracts variable_id and physical_manifestation from StateVariableExpression shape."""
        from narrative_data.persistence.loader import _extract_state_variable_interactions

        entity = {
            "state_variable_expression": [
                {"variable_id": "trust", "physical_manifestation": "Walls closing in."}
            ]
        }
        interactions = _extract_state_variable_interactions("place-entities", entity)
        assert len(interactions) == 1
        assert interactions[0]["variable_slug"] == "trust"
        assert interactions[0]["operation"] is None
        assert interactions[0]["context"] == {"physical_manifestation": "Walls closing in."}

    def test_extract_interactions_from_bare_slugs(self) -> None:
        """Extracts bare string slugs from archetypes."""
        from narrative_data.persistence.loader import _extract_state_variable_interactions

        entity = {"state_variables": ["trust", "safety"]}
        interactions = _extract_state_variable_interactions("archetypes", entity)
        assert len(interactions) == 2
        assert interactions[0] == {"variable_slug": "trust", "operation": None, "context": None}

    def test_extract_interactions_empty_list(self) -> None:
        """Returns empty list when no interactions present."""
        from narrative_data.persistence.loader import _extract_state_variable_interactions

        entity = {"state_variable_interactions": []}
        interactions = _extract_state_variable_interactions("tropes", entity)
        assert interactions == []

    def test_extract_interactions_missing_field(self) -> None:
        """Returns empty list when field is absent."""
        from narrative_data.persistence.loader import _extract_state_variable_interactions

        entity = {"name": "No interactions"}
        interactions = _extract_state_variable_interactions("tropes", entity)
        assert interactions == []
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestStateVariableInteractionExtraction -v`
Expected: ImportError — `_extract_state_variable_interactions` does not exist

- [ ] **Step 3: Implement `_extract_state_variable_interactions`**

Add to `loader.py`:

```python
def _extract_state_variable_interactions(type_name: str, entity: dict) -> list[dict]:
    """Extract state variable interaction records from an entity's payload.

    Handles three shapes:
    - StateVariableInteraction (tropes, dynamics, goals): variable_id + operation
    - StateVariableExpression (place_entities): variable_id + physical_manifestation
    - Bare list[str] (archetypes): just variable slugs
    """
    results: list[dict] = []

    # Archetypes: bare list[str]
    if type_name == "archetypes":
        slugs = entity.get("state_variables") or []
        if isinstance(slugs, list):
            for slug in slugs:
                if isinstance(slug, str) and slug.strip():
                    results.append({"variable_slug": slug.strip(), "operation": None, "context": None})
        return results

    # Place entities: StateVariableExpression
    if type_name == "place-entities":
        expressions = entity.get("state_variable_expression") or []
        if isinstance(expressions, list):
            for expr in expressions:
                if isinstance(expr, dict):
                    vid = expr.get("variable_id")
                    if vid and isinstance(vid, str):
                        manifestation = expr.get("physical_manifestation")
                        ctx = {"physical_manifestation": manifestation} if manifestation else None
                        results.append({"variable_slug": vid.strip(), "operation": None, "context": ctx})
        return results

    # Default: StateVariableInteraction (tropes, dynamics, goals)
    interactions = entity.get("state_variable_interactions") or []
    if isinstance(interactions, list):
        for ix in interactions:
            if isinstance(ix, dict):
                vid = ix.get("variable_id")
                if vid and isinstance(vid, str):
                    results.append({
                        "variable_slug": vid.strip(),
                        "operation": ix.get("operation"),
                        "context": None,
                    })
    return results
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestStateVariableInteractionExtraction -v`
Expected: All PASS

- [ ] **Step 5: Implement Phase 2 second pass for bulk insertion**

Add to `loader.py`:

```python
# Types that carry state variable references
_SV_TYPES = {"archetypes", "dynamics", "goals", "tropes", "place-entities"}


def _load_state_variable_interactions(
    conn: psycopg.Connection,
    type_name: str,
    corpus_dir: Path,
    slug_map: dict[str, str],
    genre_filter: list[str] | None = None,
    dry_run: bool = False,
) -> LoadReport:
    """Second pass: extract state variable interactions from loaded entities and insert into join table."""
    report = LoadReport()
    table = TABLE_MAP.get(type_name)
    if not table:
        return report

    # Build state variable slug → UUID map
    sv_map: dict[str, str] = {}
    if not dry_run:
        with conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT slug, id FROM ground_state.state_variables")
            for row in cur.fetchall():
                sv_map[row["slug"]] = str(row["id"])

    files = _corpus_files_for_type(type_name, corpus_dir, genre_filter)
    for file_path, genre_slug, cluster_slug in files:
        genre_id = slug_map.get(genre_slug) if genre_slug else None
        if genre_slug and not genre_id:
            continue

        try:
            data = json.loads(file_path.read_bytes())
        except (json.JSONDecodeError, OSError):
            continue

        if not isinstance(data, list):
            continue

        for entity in data:
            if not isinstance(entity, dict):
                continue

            slug = _entity_slug(entity)
            if not slug:
                continue

            interactions = _extract_state_variable_interactions(type_name, entity)
            if not interactions:
                continue

            # Look up the primitive entity's UUID
            if dry_run:
                report.skipped += len(interactions)
                continue

            with conn.cursor(row_factory=dict_row) as cur:
                cur.execute(
                    f"SELECT id FROM ground_state.{table} WHERE genre_id = %s AND entity_slug = %s LIMIT 1",
                    (genre_id, slug),
                )
                row = cur.fetchone()
                if not row:
                    continue
                primitive_id = str(row["id"])

                for ix in interactions:
                    sv_id = sv_map.get(ix["variable_slug"])
                    if not sv_id:
                        log.warning(
                            "Unknown state variable '%s' in %s/%s — skipping",
                            ix["variable_slug"], type_name, slug,
                        )
                        report.skipped += 1
                        continue

                    cur.execute(
                        """
                        INSERT INTO ground_state.primitive_state_variable_interactions
                            (primitive_table, primitive_id, state_variable_id, operation, context)
                        VALUES (%s, %s, %s, %s, %s::jsonb)
                        ON CONFLICT DO NOTHING
                        """,
                        (
                            table,
                            primitive_id,
                            sv_id,
                            ix["operation"],
                            json.dumps(ix["context"]) if ix["context"] else None,
                        ),
                    )
                    report.inserted += 1

        if not dry_run:
            conn.commit()

    return report
```

- [ ] **Step 6: Integrate into `load_ground_state`**

Update `load_ground_state` to run the second pass after primitive types:

```python
    # Phase 2.5: State variable interaction extraction
    sv_total = LoadReport()
    for type_name in [t for t in active_types if t in _SV_TYPES]:
        log.info("Extracting state variable interactions for: %s", type_name)
        sv_report = _load_state_variable_interactions(
            conn, type_name, corpus_dir, slug_map,
            genre_filter=genre_filter, dry_run=dry_run,
        )
        log.info("  %s sv-interactions: %s", type_name, sv_report.summary())
        sv_total = sv_total + sv_report

    log.info("Load complete. Entities: %s | SV interactions: %s", total.summary(), sv_total.summary())
```

- [ ] **Step 7: Write DB integration test**

```python
class TestStateVariableInteractionDB:
    @_skip_db
    def test_interactions_loaded_for_tropes(self, db_conn, tmp_path: Path) -> None:
        """State variable interactions are extracted and inserted for tropes."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus_with_sv_interactions(tmp_path)
        load_ground_state(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT COUNT(*) AS n FROM ground_state.primitive_state_variable_interactions "
                "WHERE primitive_table = 'tropes'"
            )
            row = cur.fetchone()
        assert row["n"] >= 1
```

Add helper `_minimal_corpus_with_sv_interactions` — extend `_minimal_corpus` to include a trope with state_variable_interactions and a matching state variable:

```python
def _minimal_corpus_with_sv_interactions(tmp_path: Path) -> Path:
    """Build corpus with state variable interactions in tropes."""
    corpus_dir = _minimal_corpus(tmp_path)

    # Update tropes to include state variable interactions
    tropes_dir = corpus_dir / "genres" / "folk-horror"
    tropes = [
        {
            "name": "The Calendar Trope",
            "genre_slug": "folk-horror",
            "genre_derivation": "Temporal: Seasonal",
            "narrative_function": ["escalating"],
            "variants": {},
            "state_variable_interactions": [
                {"variable_id": "trust", "operation": "depletes", "description": "Erodes trust."}
            ],
            "ontological_dimension": None,
            "overlap_signal": None,
            "flavor_text": "A test trope.",
        }
    ]
    (tropes_dir / "tropes.json").write_text(json.dumps(tropes))

    return corpus_dir
```

- [ ] **Step 8: Run full test suite**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py -v`
Expected: All PASS

- [ ] **Step 9: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/loader.py tools/narrative-data/tests/test_loader.py
git commit -m "feat: extract state variable interactions into polymorphic join table"
```

---

### Task 7: Update `genre_context()` SQL Function

**Files:**
- Modify: `crates/storyteller-storykeeper/migrations/20260323000003_create_ground_state_query_functions.sql`
- Modify: `tools/narrative-data/tests/test_loader.py`

- [ ] **Step 1: Update genre_context() to join trope families**

In `20260323000003_create_ground_state_query_functions.sql`, replace the tropes subquery (lines 36-38):

```sql
        'tropes', (SELECT jsonb_agg(jsonb_build_object(
                        'slug', t.entity_slug,
                        'data', t.payload,
                        'family_slug', tf.slug,
                        'family_name', tf.name))
                   FROM ground_state.tropes t
                   LEFT JOIN ground_state.trope_families tf ON t.trope_family_id = tf.id
                   WHERE t.genre_id = v_genre_id AND t.cluster_id IS NULL),
```

- [ ] **Step 2: Write DB integration test**

```python
@_skip_db
def test_genre_context_tropes_include_family(self, db_conn, tmp_path: Path) -> None:
    """genre_context returns tropes with family_slug and family_name."""
    corpus_dir = _minimal_corpus(tmp_path)
    load_ground_state(db_conn, corpus_dir)
    db_conn.commit()

    cur = db_conn.execute("SELECT ground_state.genre_context('folk-horror')")
    result = cur.fetchone()[0]
    tropes = result.get("tropes") or []
    if tropes:
        first = tropes[0]
        assert "family_slug" in first
        assert "family_name" in first
```

- [ ] **Step 3: Run DB integration tests**

Run: `cd tools/narrative-data && DATABASE_URL=postgres://storyteller:storyteller@localhost:5435/storyteller_development uv run pytest tests/test_loader.py::TestQueryFunctions -v`
Expected: All PASS (requires DB with updated migrations)

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-storykeeper/migrations/20260323000003_create_ground_state_query_functions.sql tools/narrative-data/tests/test_loader.py
git commit -m "feat: update genre_context() to join trope families"
```

---

### Task 8: Reload and Verify

**Files:** None (verification only)

- [ ] **Step 1: Run ruff lint**

Run: `cd tools/narrative-data && uv run ruff check .`
Expected: No errors

- [ ] **Step 2: Run ruff format**

Run: `cd tools/narrative-data && uv run ruff format --check .`
Expected: No errors (or run `uv run ruff format .` to fix)

- [ ] **Step 3: Run full test suite (unit tests)**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All PASS

- [ ] **Step 4: Drop and recreate database**

```bash
cd crates/storyteller-storykeeper
DATABASE_URL=postgres://storyteller:storyteller@localhost:5435/storyteller_development sqlx database drop -y
DATABASE_URL=postgres://storyteller:storyteller@localhost:5435/storyteller_development sqlx database create
DATABASE_URL=postgres://storyteller:storyteller@localhost:5435/storyteller_development sqlx migrate run
```

Expected: All 3 migrations applied successfully

- [ ] **Step 5: Reload corpus**

Run: `cd tools/narrative-data && DATABASE_URL=postgres://storyteller:storyteller@localhost:5435/storyteller_development uv run narrative-data load-ground-state`
Expected: No errors, entity counts printed, trope families and state variable interactions loaded

- [ ] **Step 6: Verify data quality in psql**

```sql
-- Check no sentinel slugs in ontological_posture
SELECT COUNT(*) FROM ground_state.ontological_posture WHERE entity_slug = 'null';
-- Expected: 0

-- Check topological_role populated
SELECT topological_role, COUNT(*) FROM ground_state.place_entities GROUP BY topological_role;
-- Expected: hub, endpoint, connector, etc. (some NULL is OK)

-- Check no archetype_ref column
SELECT column_name FROM information_schema.columns WHERE table_schema = 'ground_state' AND table_name = 'profiles' AND column_name = 'archetype_ref';
-- Expected: 0 rows

-- Check trope families loaded
SELECT COUNT(*) FROM ground_state.trope_families;
-- Expected: >= 10

-- Check trope_family_id populated
SELECT tf.slug, COUNT(*) FROM ground_state.tropes t JOIN ground_state.trope_families tf ON t.trope_family_id = tf.id GROUP BY tf.slug ORDER BY COUNT(*) DESC;

-- Check state variable interactions
SELECT primitive_table, COUNT(*) FROM ground_state.primitive_state_variable_interactions GROUP BY primitive_table;

-- Check genre_context includes family info
SELECT ground_state.genre_context('folk-horror') -> 'tropes' -> 0;
-- Expected: includes family_slug, family_name
```

- [ ] **Step 7: Run DB integration tests**

Run: `cd tools/narrative-data && DATABASE_URL=postgres://storyteller:storyteller@localhost:5435/storyteller_development uv run pytest tests/test_loader.py -v`
Expected: All PASS

- [ ] **Step 8: Final commit**

```bash
git add -A
git commit -m "chore: verification complete — ground-state data quality fixes validated"
```
