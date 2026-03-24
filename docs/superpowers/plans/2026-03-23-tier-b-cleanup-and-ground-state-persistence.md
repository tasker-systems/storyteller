# Tier B Cleanup and Ground-State Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the Tier B narrative data corpus (fill extractable null fields, tier the schemas) and persist it as queryable ground-state reference data in PostgreSQL.

**Architecture:** Two sequential workstreams in Python. Workstream 1 (Tasks 1-5) adds audit, fill, and schema tiering to the existing `narrative-data` CLI. Workstream 2 (Tasks 6-10) adds PostgreSQL migrations, a two-phase loader, and SQL query functions. The `narrative-data` package gains `psycopg[binary]` as a new dependency for database connectivity.

**Tech Stack:** Python 3.11+, Pydantic 2.x, Click, psycopg 3 (PostgreSQL adapter), Rich (terminal UI), pytest. PostgreSQL 18 + Apache AGE on port 5435.

**Spec:** `docs/superpowers/specs/2026-03-23-tier-b-cleanup-and-ground-state-persistence-design.md`

---

## File Map

### New files

| File | Responsibility |
|------|---------------|
| `tools/narrative-data/src/narrative_data/pipeline/postprocess.py` | Audit + deterministic fill functions |
| `tools/narrative-data/src/narrative_data/pipeline/llm_patch.py` | LLM-targeted fill functions (valence, currencies, scale_manifestations) |
| `tools/narrative-data/src/narrative_data/persistence/__init__.py` | Persistence subpackage |
| `tools/narrative-data/src/narrative_data/persistence/connection.py` | Database connection management |
| `tools/narrative-data/src/narrative_data/persistence/loader.py` | Two-phase ground-state loader (upsert, drift, prune) |
| `tools/narrative-data/src/narrative_data/persistence/reference_data.py` | State variable + dimension canonical data extraction |
| `tools/narrative-data/sql/migrations/001_create_ground_state_schema.sql` | Schema + reference entity tables |
| `tools/narrative-data/sql/migrations/002_create_primitive_type_tables.sql` | 12 primitive type tables + indexes |
| `tools/narrative-data/sql/migrations/003_create_query_functions.sql` | SQL query functions (genre_context, etc.) |
| `tools/narrative-data/tests/test_postprocess.py` | Tests for audit + deterministic fills |
| `tools/narrative-data/tests/test_llm_patch.py` | Tests for LLM patch fills |
| `tools/narrative-data/tests/test_loader.py` | Tests for ground-state loader |
| `tools/narrative-data/tests/fixtures/dynamics_folk_horror.json` | Fixture: dynamics entities with known nulls |
| `tools/narrative-data/tests/fixtures/dynamics_folk_horror.md` | Fixture: dynamics source markdown |
| `tools/narrative-data/tests/fixtures/region_folk_horror.json` | Fixture: region.json for folk-horror |

### Modified files

| File | Changes |
|------|---------|
| `tools/narrative-data/pyproject.toml` | Add `psycopg[binary]` dependency |
| `tools/narrative-data/src/narrative_data/cli.py` | Add `audit`, `fill`, `load-ground-state`, `migrate` commands |
| `tools/narrative-data/src/narrative_data/config.py` | Add `resolve_database_url()`, reference data constants |
| `tools/narrative-data/src/narrative_data/schemas/*.py` | Add `json_schema_extra={"tier": ...}` to all Field definitions (12 files) |

---

## Task 1: Audit Command — Corpus Null Rate Scanner

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/postprocess.py`
- Create: `tools/narrative-data/tests/test_postprocess.py`
- Create: `tools/narrative-data/tests/fixtures/dynamics_folk_horror.json`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`
- Modify: `tools/narrative-data/src/narrative_data/config.py`

- [ ] **Step 1: Create test fixtures**

Create a minimal dynamics JSON fixture with known null patterns (some entities have `spans_scales: []`, `currencies: []`, `valence: null`, etc.) and a region.json fixture. Use real data structure from the corpus but with 3-4 entities to keep tests fast.

```bash
# Copy and trim a real file for the fixture
cd tools/narrative-data
```

- [ ] **Step 2: Write failing tests for audit functions**

```python
# tests/test_postprocess.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

import json
from pathlib import Path

import pytest

from narrative_data.pipeline.postprocess import audit_type, audit_corpus, AuditResult


@pytest.fixture
def fixtures_dir() -> Path:
    return Path(__file__).parent / "fixtures"


class TestAuditType:
    def test_reports_null_rates(self, fixtures_dir: Path):
        result = audit_type("dynamics", fixtures_dir / "dynamics_folk_horror.json")
        assert isinstance(result, AuditResult)
        assert result.type_name == "dynamics"
        assert result.record_count > 0
        assert "genre_slug" in result.field_rates
        assert result.field_rates["genre_slug"] == 0.0  # never null

    def test_detects_empty_lists_as_null(self, fixtures_dir: Path):
        result = audit_type("dynamics", fixtures_dir / "dynamics_folk_horror.json")
        # spans_scales should show high null rate (empty lists count as null)
        assert result.field_rates["spans_scales"] > 0.5

    def test_handles_nested_fields(self, fixtures_dir: Path):
        result = audit_type("dynamics", fixtures_dir / "dynamics_folk_horror.json")
        # scale_manifestations.orbital should be tracked
        assert "scale_manifestations.orbital" in result.field_rates


class TestAuditCorpus:
    def test_walks_discovery_and_genre_dirs(self, fixtures_dir: Path, tmp_path: Path):
        # Set up minimal corpus structure
        discovery_dir = tmp_path / "discovery" / "dynamics"
        discovery_dir.mkdir(parents=True)
        src = fixtures_dir / "dynamics_folk_horror.json"
        (discovery_dir / "folk-horror.json").write_text(src.read_text())

        results = audit_corpus(tmp_path, types=["dynamics"])
        assert len(results) == 1
        assert results[0].type_name == "dynamics"
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py -v
```
Expected: FAIL — `narrative_data.pipeline.postprocess` does not exist.

- [ ] **Step 4: Implement audit functions**

```python
# src/narrative_data/pipeline/postprocess.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Audit and deterministic fill pipeline for Tier B narrative data corpus."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

from narrative_data.config import PRIMITIVE_TYPES, GENRE_NATIVE_TYPES


@dataclass
class AuditResult:
    """Null rate report for a single primitive type."""
    type_name: str
    record_count: int
    field_rates: dict[str, float]  # field_name → fraction null (0.0-1.0)
    file_paths: list[str] = field(default_factory=list)


def _is_null_or_empty(value: object) -> bool:
    """Treat None, empty strings, and empty lists as null."""
    if value is None:
        return True
    if isinstance(value, str) and value.strip() == "":
        return True
    if isinstance(value, list) and len(value) == 0:
        return True
    return False


def _flatten_fields(obj: dict, prefix: str = "") -> dict[str, object]:
    """Flatten nested dict to dot-separated keys, one level deep for sub-models."""
    result = {}
    for key, value in obj.items():
        full_key = f"{prefix}{key}" if not prefix else f"{prefix}.{key}"
        if isinstance(value, dict):
            result.update(_flatten_fields(value, full_key))
        else:
            result[full_key] = value
    return result


def audit_type(type_name: str, json_path: Path) -> AuditResult:
    """Compute null rates for every field in a single type's JSON file."""
    data = json.loads(json_path.read_text())
    entities = data if isinstance(data, list) else [data]

    field_counts: dict[str, int] = {}  # field → count of nulls
    field_totals: dict[str, int] = {}  # field → total occurrences

    for entity in entities:
        flat = _flatten_fields(entity)
        for key, value in flat.items():
            field_totals[key] = field_totals.get(key, 0) + 1
            if _is_null_or_empty(value):
                field_counts[key] = field_counts.get(key, 0) + 1

    field_rates = {
        k: field_counts.get(k, 0) / field_totals[k]
        for k in sorted(field_totals)
    }

    return AuditResult(
        type_name=type_name,
        record_count=len(entities),
        field_rates=field_rates,
        file_paths=[str(json_path)],
    )


def audit_corpus(
    corpus_dir: Path,
    types: list[str] | None = None,
    genres: list[str] | None = None,
) -> list[AuditResult]:
    """Walk the corpus and produce audit results per type."""
    results: list[AuditResult] = []
    target_types = types or PRIMITIVE_TYPES + GENRE_NATIVE_TYPES

    # Discovery types: corpus_dir/discovery/{type}/{genre}.json
    discovery_dir = corpus_dir / "discovery"
    for type_name in target_types:
        if type_name in GENRE_NATIVE_TYPES:
            continue  # handled below
        type_dir = discovery_dir / type_name
        if not type_dir.is_dir():
            continue
        # Aggregate across all genre files for this type
        merged_rates: dict[str, list[float]] = {}
        total_records = 0
        file_paths = []
        for json_file in sorted(type_dir.glob("*.json")):
            if json_file.name.startswith("cluster-"):
                continue  # skip cluster files for per-genre audit
            genre_slug = json_file.stem
            if genres and genre_slug not in genres:
                continue
            result = audit_type(type_name, json_file)
            total_records += result.record_count
            file_paths.append(str(json_file))
            for field_name, rate in result.field_rates.items():
                merged_rates.setdefault(field_name, []).append(
                    rate * result.record_count
                )

        if total_records > 0:
            avg_rates = {
                k: sum(v) / total_records for k, v in merged_rates.items()
            }
            results.append(AuditResult(
                type_name=type_name,
                record_count=total_records,
                field_rates=avg_rates,
                file_paths=file_paths,
            ))

    # Genre-native types: corpus_dir/genres/{genre}/{type}.json
    genres_dir = corpus_dir / "genres"
    for type_name in target_types:
        if type_name not in GENRE_NATIVE_TYPES:
            continue
        merged_rates: dict[str, list[float]] = {}
        total_records = 0
        file_paths = []
        if not genres_dir.is_dir():
            continue
        for genre_dir in sorted(genres_dir.iterdir()):
            if not genre_dir.is_dir():
                continue
            if genres and genre_dir.name not in genres:
                continue
            json_file = genre_dir / f"{type_name}.json"
            if not json_file.exists():
                continue
            result = audit_type(type_name, json_file)
            total_records += result.record_count
            file_paths.append(str(json_file))
            for field_name, rate in result.field_rates.items():
                merged_rates.setdefault(field_name, []).append(
                    rate * result.record_count
                )

        if total_records > 0:
            avg_rates = {
                k: sum(v) / total_records for k, v in merged_rates.items()
            }
            results.append(AuditResult(
                type_name=type_name,
                record_count=total_records,
                field_rates=avg_rates,
                file_paths=file_paths,
            ))

    return results
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py -v
```
Expected: PASS

- [ ] **Step 6: Add audit CLI command**

In `cli.py`, add:

```python
@cli.command("audit")
@click.option("--type", "types", multiple=True, help="Filter to specific types")
@click.option("--genre", "genres", multiple=True, help="Filter to specific genres")
@click.option("--output", "output_path", type=click.Path(), help="Save JSON report")
def audit_cmd(types: tuple[str, ...], genres: tuple[str, ...], output_path: str | None) -> None:
    """Audit null rates across the narrative data corpus."""
    from narrative_data.config import resolve_output_path
    from narrative_data.pipeline.postprocess import audit_corpus
    from rich.console import Console
    from rich.table import Table

    corpus_dir = resolve_output_path()
    results = audit_corpus(
        corpus_dir,
        types=list(types) or None,
        genres=list(genres) or None,
    )

    console = Console()
    for result in results:
        table = Table(title=f"{result.type_name} ({result.record_count} records)")
        table.add_column("Field", style="cyan")
        table.add_column("Null %", justify="right")
        table.add_column("Status")

        for field_name, rate in sorted(result.field_rates.items(), key=lambda x: -x[1]):
            pct = f"{rate * 100:.1f}%"
            if rate == 0:
                status = "[green]complete[/green]"
            elif rate < 0.2:
                status = "[green]good[/green]"
            elif rate < 0.5:
                status = "[yellow]partial[/yellow]"
            else:
                status = "[red]sparse[/red]"
            table.add_row(field_name, pct, status)

        console.print(table)
        console.print()

    if output_path:
        import json
        report = {r.type_name: {"record_count": r.record_count, "field_rates": r.field_rates} for r in results}
        Path(output_path).write_text(json.dumps(report, indent=2))
        console.print(f"Report saved to {output_path}")
```

- [ ] **Step 7: Smoke test audit against real corpus**

```bash
cd tools/narrative-data && uv run narrative-data audit --type dynamics --type spatial-topology
```
Expected: Rich tables showing null rates. Verify `spans_scales` shows >70%, `agency` shows >75%.

- [ ] **Step 8: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/postprocess.py \
        tools/narrative-data/tests/test_postprocess.py \
        tools/narrative-data/tests/fixtures/ \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(narrative-data): add corpus audit command with null rate reporting"
```

---

## Task 2: Deterministic Fill Functions

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/postprocess.py`
- Create: `tools/narrative-data/tests/fixtures/dynamics_folk_horror.md`
- Modify: `tools/narrative-data/tests/test_postprocess.py`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Write failing tests for spans_scales fill**

```python
# In tests/test_postprocess.py

from narrative_data.pipeline.postprocess import fill_spans_scales


class TestFillSpansScales:
    def test_extracts_spanning_scale(self, tmp_path: Path):
        md_content = "## Dynamic 1\n*   **Scale:** Spanning (Orbital/Scene)\n"
        entity = {"canonical_name": "Test", "spans_scales": [], "scale": "orbital"}
        result = fill_spans_scales(entity, md_content, "Test")
        assert result["spans_scales"] == ["orbital", "scene"]

    def test_skips_populated_field(self):
        entity = {"canonical_name": "Test", "spans_scales": ["arc"], "scale": "orbital"}
        result = fill_spans_scales(entity, "irrelevant", "Test")
        assert result["spans_scales"] == ["arc"]  # unchanged

    def test_extracts_cross_scale(self, tmp_path: Path):
        md_content = "## Dynamic 1\n*   **Scale:** Cross-Scale (Orbital vs. Scene)\n"
        entity = {"canonical_name": "Test", "spans_scales": [], "scale": "orbital"}
        result = fill_spans_scales(entity, md_content, "Test")
        assert "orbital" in result["spans_scales"]
        assert "scene" in result["spans_scales"]
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py::TestFillSpansScales -v
```

- [ ] **Step 3: Implement fill_spans_scales**

Add to `postprocess.py`:

```python
import re

# Scale label patterns from dynamics source markdown
_SCALE_PATTERN = re.compile(
    r"\*\s*\*\*Scale:\*\*\s*(.+?)(?:\n|$)", re.IGNORECASE
)
_SCALE_LABELS = {"orbital", "arc", "scene"}


def fill_spans_scales(
    entity: dict, md_content: str, entity_name: str
) -> dict:
    """Fill spans_scales from **Scale:** patterns in source markdown."""
    if entity.get("spans_scales"):
        return entity  # already populated

    # Find the section for this entity in the markdown
    # Search for the entity name and the Scale pattern nearby
    matches = _SCALE_PATTERN.findall(md_content)
    for match in matches:
        match_lower = match.lower()
        # Check for spanning/cross-scale indicators
        if any(kw in match_lower for kw in ["spanning", "cross-scale", "multi-scale"]):
            # Extract scale names from parenthetical
            scales = []
            for label in _SCALE_LABELS:
                if label in match_lower:
                    scales.append(label)
            if scales:
                entity["spans_scales"] = sorted(scales)
                return entity

    return entity
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py::TestFillSpansScales -v
```

- [ ] **Step 5: Write failing tests for agency fill**

```python
class TestFillAgency:
    def test_infers_from_friction_and_directionality(self):
        entity = {
            "agency": None,
            "friction": {"type": "high", "level": "impassable"},
            "directionality": {"type": "one-way"},
        }
        result = fill_agency(entity)
        assert result["agency"] == "none"

    def test_bidirectional_low_friction_is_high(self):
        entity = {
            "agency": None,
            "friction": {"type": "low", "level": "permeable"},
            "directionality": {"type": "bidirectional"},
        }
        result = fill_agency(entity)
        assert result["agency"] == "high"

    def test_skips_populated(self):
        entity = {"agency": "medium", "friction": {"type": "low"}, "directionality": {"type": "bidirectional"}}
        result = fill_agency(entity)
        assert result["agency"] == "medium"
```

- [ ] **Step 6: Implement fill_agency**

```python
# Lookup table: (friction_type, directionality_type) → agency
_AGENCY_LOOKUP: dict[tuple[str, str], str] = {
    ("high", "one-way"): "none",
    ("high", "unidirectional"): "none",
    ("high", "bidirectional"): "low",
    ("high", "asymmetric"): "low",
    ("medium", "one-way"): "low",
    ("medium", "unidirectional"): "low",
    ("medium", "bidirectional"): "medium",
    ("medium", "asymmetric"): "medium",
    ("low", "one-way"): "medium",
    ("low", "unidirectional"): "medium",
    ("low", "bidirectional"): "high",
    ("low", "bidirectional_asymmetric"): "medium",
    ("low", "asymmetric"): "medium",
    ("none", "bidirectional"): "high",
    ("none", "one-way"): "medium",
    # Constrained choice patterns
    ("medium", "constrained"): "illusion",
    ("low", "constrained"): "illusion",
}


def fill_agency(entity: dict) -> dict:
    """Infer agency from friction.type + directionality.type lookup."""
    if entity.get("agency") is not None:
        return entity

    friction = entity.get("friction", {})
    directionality = entity.get("directionality", {})
    friction_type = friction.get("type", "").lower()
    dir_type = directionality.get("type", "").lower()

    key = (friction_type, dir_type)
    if key in _AGENCY_LOOKUP:
        entity["agency"] = _AGENCY_LOOKUP[key]

    return entity
```

- [ ] **Step 7: Run tests**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py::TestFillAgency -v
```

- [ ] **Step 8: Implement fill_all_deterministic and fill CLI command**

Add the orchestration function that walks the corpus and applies fills per type, plus the
`fill` CLI command with `--tier deterministic` and `--dry-run` options. The fill function
reads each JSON file, applies the appropriate fill functions, and writes back only if changes
were made.

Add to `cli.py`:

```python
@cli.command("fill")
@click.option("--tier", type=click.Choice(["deterministic", "llm-patch"]), required=True)
@click.option("--type", "types", multiple=True)
@click.option("--genre", "genres", multiple=True)
@click.option("--dry-run", is_flag=True, help="Report changes without writing")
def fill_cmd(tier: str, types: tuple[str, ...], genres: tuple[str, ...], dry_run: bool) -> None:
    """Fill null fields in the corpus."""
    ...
```

- [ ] **Step 9: Smoke test fills against real corpus**

```bash
cd tools/narrative-data && uv run narrative-data fill --tier deterministic --type dynamics --dry-run
cd tools/narrative-data && uv run narrative-data fill --tier deterministic --type spatial-topology --dry-run
```
Expected: Dry-run output showing which entities would be updated and which fields filled.

- [ ] **Step 10: Run fills for real, then re-audit**

```bash
cd tools/narrative-data && uv run narrative-data fill --tier deterministic --type dynamics
cd tools/narrative-data && uv run narrative-data fill --tier deterministic --type spatial-topology
cd tools/narrative-data && uv run narrative-data audit --type dynamics --type spatial-topology
```
Expected: `spans_scales` and `agency` null rates should decrease significantly.

- [ ] **Step 11: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/postprocess.py \
        tools/narrative-data/tests/test_postprocess.py \
        tools/narrative-data/tests/fixtures/ \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(narrative-data): add deterministic fill functions (spans_scales, agency)"
```

---

## Task 3: LLM Patch Fills

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/llm_patch.py`
- Create: `tools/narrative-data/tests/test_llm_patch.py`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Write failing tests for LLM patch functions**

```python
# tests/test_llm_patch.py
from unittest.mock import MagicMock

from narrative_data.pipeline.llm_patch import extract_valence, extract_currencies


class TestExtractValence:
    def test_returns_valence_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "hostile"
        entity = {"canonical_name": "The Debt", "edge_type": "debt-laden", "valence": None}
        result = extract_valence(entity, "source markdown content", mock_client)
        assert result["valence"] in {"hostile", "protective", "nurturing", "sacred", "transgressive", "ambivalent"}

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"canonical_name": "Test", "valence": "hostile"}
        result = extract_valence(entity, "irrelevant", mock_client)
        assert result["valence"] == "hostile"
        mock_client.generate.assert_not_called()
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/test_llm_patch.py -v
```

- [ ] **Step 3: Implement LLM patch functions**

Create `llm_patch.py` with `extract_valence()`, `extract_currencies()`, and
`extract_scale_manifestations()`. Each function:
- Checks if field is already populated (skip if so)
- Builds a focused prompt with the entity data + source markdown context
- Calls the Ollama client for structured extraction
- Returns the entity with the field filled

Use the existing `OllamaClient` from `narrative_data.ollama` and the existing
`STRUCTURING_MODEL` from config. Keep prompts minimal — one field at a time.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/test_llm_patch.py -v
```

- [ ] **Step 5: Wire LLM patch tier into the fill CLI command**

Update the `fill_cmd` in `cli.py` to handle `--tier llm-patch`. This tier initializes the
Ollama client and passes it to the LLM patch functions.

- [ ] **Step 6: Smoke test LLM fills on a single genre**

```bash
cd tools/narrative-data && uv run narrative-data fill --tier llm-patch --type dynamics --genre folk-horror --dry-run
```
Expected: Shows which entities would get `valence`, `currencies`, `scale_manifestations` filled.

- [ ] **Step 7: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/llm_patch.py \
        tools/narrative-data/tests/test_llm_patch.py \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(narrative-data): add LLM patch fill functions (valence, currencies, scale_manifestations)"
```

---

## Task 4: Schema Tiering Annotations

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/schemas/archetypes.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/dynamics.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/goals.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/settings.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/place_entities.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/scene_profiles.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/ontological_posture.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/spatial_topology.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/archetype_dynamics.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/genre_dimensions.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/tropes.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/narrative_shapes.py`
- Modify: `tools/narrative-data/tests/test_postprocess.py`

- [ ] **Step 1: Run the audit against the full corpus to get post-cleanup null rates**

```bash
cd tools/narrative-data && uv run narrative-data audit --output /tmp/audit-report.json
```

Review the report. Fields with >80% population → core. Fields with <80% → extended.
Identity fields (slugs, names, types) → always core regardless of rate.

- [ ] **Step 2: Write a test that validates tier annotations exist on all schema fields**

```python
# In tests/test_postprocess.py

from narrative_data.schemas import archetypes, dynamics, goals, settings
# ... import all schema modules


class TestSchemaTiering:
    @pytest.mark.parametrize("model_cls", [
        archetypes.Archetype,
        dynamics.Dynamic,
        goals.Goal,
        settings.Settings,
        # ... all 12 per-genre models
    ])
    def test_all_fields_have_tier_annotation(self, model_cls):
        for field_name, field_info in model_cls.model_fields.items():
            extra = field_info.json_schema_extra or {}
            assert "tier" in extra, (
                f"{model_cls.__name__}.{field_name} missing tier annotation"
            )
            assert extra["tier"] in ("core", "extended"), (
                f"{model_cls.__name__}.{field_name} has invalid tier: {extra['tier']}"
            )
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py::TestSchemaTiering -v
```
Expected: FAIL — no `json_schema_extra` on any fields yet.

- [ ] **Step 4: Add tier annotations to all 12 schema files**

For each schema file, add `json_schema_extra={"tier": "core"}` or `{"tier": "extended"}` to
every `Field()` definition. For fields that use bare type annotations without `Field()`,
convert them to use `Field(...)` or `Field(default=...)`.

**Tier assignment guide** (informed by audit results + spec criteria):

Core (always): `canonical_name`, `genre_slug`, `variant_name`, `cluster_name`, `entity_slug`,
`name`, `scale`, `edge_type`, `directionality`, `universality`, `uniqueness`

Core (high population): Fields with >80% population that the Storykeeper needs for
filtering — `archetype_family`, `setting_type`, `goal_scale`, `friction_type`,
`directionality_type`, `boundary_stability`, `place_type`, `archetype_ref`,
`trope_family`, `shape_type`

Extended: Everything else — `flavor_text`, `overlap_signals`, `genre_variants`,
`scale_manifestations`, `currencies`, `valence`, `network_position`, `crossing_rules`, etc.

- [ ] **Step 5: Run test to verify it passes**

```bash
cd tools/narrative-data && uv run pytest tests/test_postprocess.py::TestSchemaTiering -v
```

- [ ] **Step 6: Run full test suite to verify no regressions**

```bash
cd tools/narrative-data && uv run pytest -v
```

- [ ] **Step 7: Commit**

```bash
git add tools/narrative-data/src/narrative_data/schemas/*.py \
        tools/narrative-data/tests/test_postprocess.py
git commit -m "feat(narrative-data): add core/extended tier annotations to all 12 schema models"
```

---

## Task 5: Evaluate Speculative Fields + Run Full Fills

**Files:**
- No new files — this is an investigation + execution task

- [ ] **Step 1: Evaluate spatial-topology.agency**

Check source markdown across 3+ genres for extractable agency content:

```bash
cd tools/narrative-data && uv run narrative-data audit --type spatial-topology --output /tmp/spatial-audit.json
```

Read 3-4 spatial-topology source markdown files to see if agency is discussed. If the
deterministic fill from Task 2 reliably infers agency for most records, keep as core. If
it's unreliable or the inference is speculative, move to extended tier.

- [ ] **Step 2: Run deterministic fills across the full corpus**

```bash
cd tools/narrative-data && uv run narrative-data fill --tier deterministic
```

- [ ] **Step 3: Run LLM patch fills across the full corpus**

```bash
cd tools/narrative-data && uv run narrative-data fill --tier llm-patch
```

Note: This requires a running Ollama instance. If unavailable, document what would be
filled and skip to the next task.

- [ ] **Step 4: Run full audit and save post-fill report**

```bash
cd tools/narrative-data && uv run narrative-data audit --output docs/superpowers/specs/2026-03-23-post-cleanup-audit.json
```

Compare with pre-fill audit. Document the improvement.

- [ ] **Step 5: Commit any corpus changes**

The fills modify JSON files in `storyteller-data/`. These should be committed in that repo:

```bash
cd ../storyteller-data && git add narrative-data/ && git commit -m "data: Tier B cleanup — deterministic and LLM patch fills"
```

- [ ] **Step 6: Commit audit report**

```bash
git add docs/superpowers/specs/2026-03-23-post-cleanup-audit.json
git commit -m "docs: add post-cleanup audit report"
```

---

## Task 6: Database Dependency + Connection Management

**Files:**
- Modify: `tools/narrative-data/pyproject.toml`
- Create: `tools/narrative-data/src/narrative_data/persistence/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/persistence/connection.py`
- Modify: `tools/narrative-data/src/narrative_data/config.py`

- [ ] **Step 1: Add psycopg dependency**

In `pyproject.toml`, add to `[project.dependencies]`:

```toml
"psycopg[binary]>=3.1.0",
```

Run `uv sync --dev` to install.

- [ ] **Step 2: Write failing test for connection management**

```python
# tests/test_loader.py
# SPDX-License-Identifier: AGPL-3.0-only

import pytest
from narrative_data.persistence.connection import get_connection_string


class TestConnection:
    def test_reads_database_url_from_env(self, monkeypatch):
        monkeypatch.setenv("DATABASE_URL", "postgres://test:test@localhost:5435/test_db")
        assert get_connection_string() == "postgres://test:test@localhost:5435/test_db"

    def test_raises_without_database_url(self, monkeypatch):
        monkeypatch.delenv("DATABASE_URL", raising=False)
        with pytest.raises(ValueError, match="DATABASE_URL"):
            get_connection_string()
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestConnection -v
```

- [ ] **Step 4: Implement connection module**

```python
# src/narrative_data/persistence/__init__.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

# src/narrative_data/persistence/connection.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Database connection management for ground-state persistence."""

from __future__ import annotations

import os


def get_connection_string() -> str:
    """Read DATABASE_URL from environment."""
    url = os.environ.get("DATABASE_URL")
    if not url:
        raise ValueError(
            "DATABASE_URL environment variable is required. "
            "Set it to your PostgreSQL connection string "
            "(e.g., postgres://storyteller:storyteller@localhost:5435/storyteller_development)"
        )
    return url
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestConnection -v
```

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/pyproject.toml \
        tools/narrative-data/src/narrative_data/persistence/ \
        tools/narrative-data/tests/test_loader.py
git commit -m "feat(narrative-data): add psycopg dependency and connection management"
```

---

## Task 7: SQL Migrations — Ground-State Schema

**Files:**
- Create: `tools/narrative-data/sql/migrations/001_create_ground_state_schema.sql`
- Create: `tools/narrative-data/sql/migrations/002_create_primitive_type_tables.sql`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

Note: These migrations live in the `narrative-data` tool, not in `storyteller-storykeeper/migrations/`,
because ground-state tables are managed by the Python loader, not by sqlx. The `migrate` CLI
command applies them via psycopg.

- [ ] **Step 1: Write migration 001 — schema + reference entity tables**

```sql
-- tools/narrative-data/sql/migrations/001_create_ground_state_schema.sql
-- Ground-State Reference Data Schema
-- Managed by narrative-data Python tooling, not sqlx.

CREATE SCHEMA IF NOT EXISTS ground_state;
-- NOTE: ground_state.settings (genre-level setting archetypes from Tier B)
-- is distinct from public.settings (per-story authored locations).

-- Genres (source: region.json per genre)
CREATE TABLE ground_state.genres (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    payload       JSONB NOT NULL,
    source_hash   TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Genre clusters (6 semantic groupings)
CREATE TABLE ground_state.genre_clusters (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    payload       JSONB,
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

-- Canonical state variables (12)
CREATE TABLE ground_state.state_variables (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    default_range JSONB,
    payload       JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Universal dimensions (34 across 7 groups)
CREATE TABLE ground_state.dimensions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    dimension_group TEXT NOT NULL,
    description   TEXT,
    payload       JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dimensions_group ON ground_state.dimensions (dimension_group);
```

Note: Use `gen_random_uuid()` instead of `uuidv7()` if the UUIDv7 extension is not available.
Check with the existing migration pattern — the storyteller-storykeeper migrations use `uuidv7()`.
If the AGE image has UUIDv7 support, use that. Otherwise fall back to `gen_random_uuid()` and
document the difference.

- [ ] **Step 2: Write migration 002 — primitive type tables**

Create `002_create_primitive_type_tables.sql` with all 12 primitive type tables. Each table
follows the pattern from the spec (section 3.4): `id`, `genre_id` FK, `cluster_id` FK,
`entity_slug`, `name`, promoted core columns (vary per type), `payload` JSONB, `source_hash`,
timestamps.

For each table, create:
- The unique index on `(genre_id, entity_slug, COALESCE(cluster_id, ...))`
- FK index on `genre_id`
- Partial index on `cluster_id WHERE cluster_id IS NOT NULL`
- GIN index on `payload`

`genre_dimensions` is the special case — one row per genre, no `entity_slug` or `cluster_id`.
Its unique constraint is just `genre_id`.

- [ ] **Step 3: Add migrate CLI command**

```python
@cli.command("migrate")
@click.option("--dry-run", is_flag=True, help="Show SQL without executing")
def migrate_cmd(dry_run: bool) -> None:
    """Apply ground-state SQL migrations."""
    ...
```

The command reads SQL files from `tools/narrative-data/sql/migrations/` in order, tracks
which have been applied (via a `ground_state._migrations` table), and applies new ones.

- [ ] **Step 4: Test migration against local PostgreSQL**

```bash
cd tools/narrative-data && uv run narrative-data migrate --dry-run
cd tools/narrative-data && uv run narrative-data migrate
```
Expected: Schema and tables created. Verify with:
```bash
cargo make docker-psql
\dn ground_state
\dt ground_state.*
```

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/sql/ \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(narrative-data): add ground_state SQL migrations (schema, reference tables, primitive tables)"
```

---

## Task 8: Reference Data Extraction

**Files:**
- Create: `tools/narrative-data/src/narrative_data/persistence/reference_data.py`
- Modify: `tools/narrative-data/tests/test_loader.py`

The loader's phase 1 needs canonical lists of state variables and dimensions. These live
in the terrain analysis documents and region.json files, not as pre-existing JSON arrays.
This task extracts them into loader-consumable form.

- [ ] **Step 1: Write failing tests for reference data extraction**

```python
# In tests/test_loader.py

from narrative_data.persistence.reference_data import (
    extract_state_variables,
    extract_dimensions,
    extract_cluster_metadata,
)


class TestReferenceDataExtraction:
    def test_extracts_state_variables_from_regions(self, fixtures_dir: Path):
        region_data = json.loads((fixtures_dir / "region_folk_horror.json").read_text())
        variables = extract_state_variables([region_data])
        assert len(variables) > 0
        assert all("slug" in v and "name" in v for v in variables)
        # Canonical IDs should be deduplicated across genres
        slugs = [v["slug"] for v in variables]
        assert len(slugs) == len(set(slugs))

    def test_extracts_cluster_metadata(self):
        clusters = extract_cluster_metadata()
        assert len(clusters) == 6  # 6 known clusters
        assert all("slug" in c and "name" in c for c in clusters)
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestReferenceDataExtraction -v
```

- [ ] **Step 3: Implement reference data extraction**

```python
# src/narrative_data/persistence/reference_data.py
# SPDX-License-Identifier: AGPL-3.0-only
```

Functions:
- `extract_state_variables(region_data_list)` — scan `active_state_variables` across all
  region.json files, deduplicate by `canonical_id`, return list of `{slug, name, description,
  default_range}` dicts
- `extract_dimensions()` — return the 34 universal dimensions with their groups, sourced from
  the config or a dedicated JSON file. If no structured source exists, create a minimal
  `reference_data.json` from the terrain analysis.
- `extract_cluster_metadata()` — return the 6 clusters from `config.GENRE_CLUSTERS` with
  slug, name, and genre membership

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestReferenceDataExtraction -v
```

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/reference_data.py \
        tools/narrative-data/tests/test_loader.py
git commit -m "feat(narrative-data): add reference data extraction (state variables, dimensions, clusters)"
```

---

## Task 9: Ground-State Loader

**Files:**
- Create: `tools/narrative-data/src/narrative_data/persistence/loader.py`
- Modify: `tools/narrative-data/tests/test_loader.py`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Write failing tests for the loader**

```python
# In tests/test_loader.py

import psycopg

from narrative_data.persistence.loader import (
    load_reference_entities,
    load_primitive_type,
    LoadReport,
)


@pytest.fixture
def db_conn():
    """Connect to test database. Skip if unavailable."""
    try:
        conn = psycopg.connect(
            "postgres://storyteller:storyteller@localhost:5435/storyteller_development"
        )
        yield conn
        conn.close()
    except psycopg.OperationalError:
        pytest.skip("PostgreSQL not available on port 5435")


class TestLoader:
    def test_load_reference_entities(self, db_conn, fixtures_dir: Path):
        report = load_reference_entities(db_conn, fixtures_dir)
        assert report.inserted > 0
        assert report.errors == 0

    def test_idempotent_load(self, db_conn, fixtures_dir: Path):
        report1 = load_reference_entities(db_conn, fixtures_dir)
        report2 = load_reference_entities(db_conn, fixtures_dir)
        assert report2.updated == 0  # no changes on second load
        assert report2.skipped == report1.inserted  # all skipped

    def test_drift_detection(self, db_conn, tmp_path: Path, fixtures_dir: Path):
        # Load original
        load_reference_entities(db_conn, tmp_path)
        # Modify a file
        # Reload — should detect drift
        ...
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestLoader -v
```

- [ ] **Step 3: Implement the loader**

```python
# src/narrative_data/persistence/loader.py
# SPDX-License-Identifier: AGPL-3.0-only
```

Key components:
- `LoadReport` dataclass: `inserted`, `updated`, `pruned`, `skipped`, `errors`
- `load_reference_entities(conn, corpus_dir)` — Phase 1: loads genres, clusters, membership,
  state variables, dimensions. Returns slug→UUID lookup maps.
- `load_primitive_type(conn, type_name, corpus_dir, slug_map, dry_run=False)` — Phase 2:
  walks corpus for a single type, validates through Pydantic, upserts.
- `_compute_source_hash(path)` — SHA256 of raw file bytes (before Pydantic processing).
- `_upsert_entity(conn, table_name, entity, genre_id, cluster_id, source_hash)` — single
  entity upsert with drift detection.
- `_prune_stale(conn, table_name, valid_keys)` — delete rows not in the current corpus.

The loader uses `psycopg`'s `conn.execute()` for individual upserts (not batch — the corpus
is small enough that individual inserts are fine and simplify error reporting).

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestLoader -v
```

- [ ] **Step 5: Add load-ground-state CLI command**

```python
@cli.command("load-ground-state")
@click.option("--dry-run", is_flag=True)
@click.option("--type", "types", multiple=True)
@click.option("--genre", "genres", multiple=True)
@click.option("--skip-prune", is_flag=True)
@click.option("--refs-only", is_flag=True, help="Load only reference entities (phase 1)")
def load_ground_state_cmd(...) -> None:
    """Load narrative data corpus into PostgreSQL ground_state tables."""
    ...
```

- [ ] **Step 6: Smoke test loader against real corpus**

```bash
cd tools/narrative-data && uv run narrative-data load-ground-state --refs-only
cd tools/narrative-data && uv run narrative-data load-ground-state --type archetypes
cd tools/narrative-data && uv run narrative-data load-ground-state
```

Verify with:
```bash
cargo make docker-psql
SELECT count(*) FROM ground_state.genres;
SELECT count(*) FROM ground_state.archetypes;
SELECT count(*) FROM ground_state.dynamics;
```

- [ ] **Step 7: Test idempotency — run loader twice**

```bash
cd tools/narrative-data && uv run narrative-data load-ground-state
```
Expected: `inserted 0, updated 0, pruned 0, skipped N`

- [ ] **Step 8: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/loader.py \
        tools/narrative-data/tests/test_loader.py \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(narrative-data): add ground-state loader with upsert, drift detection, and pruning"
```

---

## Task 10: SQL Query Functions + Smoke Test

**Files:**
- Create: `tools/narrative-data/sql/migrations/003_create_query_functions.sql`
- Modify: `tools/narrative-data/tests/test_loader.py`

- [ ] **Step 1: Write migration 003 — query functions**

Create `003_create_query_functions.sql` containing:

```sql
-- genre_context: returns all ground-state data for a genre
CREATE OR REPLACE FUNCTION ground_state.genre_context(p_genre_slug TEXT)
RETURNS JSONB AS $$
...
$$ LANGUAGE plpgsql STABLE;
```

Use the function definition from the spec (section 3.7), which wraps each entity as
`{"slug": entity_slug, "data": payload}` and resolves slug → genre_id internally.

- [ ] **Step 2: Apply migration**

```bash
cd tools/narrative-data && uv run narrative-data migrate
```

- [ ] **Step 3: Write test for genre_context function**

```python
class TestQueryFunctions:
    def test_genre_context_returns_all_types(self, db_conn):
        # Assumes loader has already populated data
        cur = db_conn.execute(
            "SELECT ground_state.genre_context('folk-horror')"
        )
        result = cur.fetchone()[0]
        assert result is not None
        assert result["genre_slug"] == "folk-horror"
        assert result["genre"] is not None
        assert isinstance(result["archetypes"], list)
        assert isinstance(result["dynamics"], list)
        assert len(result["archetypes"]) > 0

    def test_genre_context_returns_null_for_unknown(self, db_conn):
        cur = db_conn.execute(
            "SELECT ground_state.genre_context('nonexistent-genre')"
        )
        result = cur.fetchone()[0]
        assert result is None

    def test_genre_context_wraps_entities_with_slug(self, db_conn):
        cur = db_conn.execute(
            "SELECT ground_state.genre_context('folk-horror')"
        )
        result = cur.fetchone()[0]
        first_archetype = result["archetypes"][0]
        assert "slug" in first_archetype
        assert "data" in first_archetype
```

- [ ] **Step 4: Run full end-to-end test**

```bash
cd tools/narrative-data && uv run narrative-data migrate
cd tools/narrative-data && uv run narrative-data load-ground-state
cd tools/narrative-data && uv run pytest tests/test_loader.py::TestQueryFunctions -v
```

- [ ] **Step 5: Manual verification via psql**

```bash
cargo make docker-psql
SELECT ground_state.genre_context('folk-horror') ->> 'genre_slug';
SELECT jsonb_array_length(ground_state.genre_context('folk-horror') -> 'archetypes');
SELECT jsonb_array_length(ground_state.genre_context('folk-horror') -> 'dynamics');
```

- [ ] **Step 6: Run full test suite**

```bash
cd tools/narrative-data && uv run pytest -v
cd tools/narrative-data && uv run ruff check .
```

- [ ] **Step 7: Commit**

```bash
git add tools/narrative-data/sql/migrations/003_create_query_functions.sql \
        tools/narrative-data/tests/test_loader.py
git commit -m "feat(narrative-data): add genre_context SQL function and end-to-end smoke tests"
```

---

## Post-Implementation

After all tasks are complete:

1. Run the full test suite: `cd tools/narrative-data && uv run pytest -v`
2. Run linting: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check .`
3. Verify ground-state data is loaded: `cargo make docker-psql` → `SELECT count(*) FROM ground_state.genres;`
4. Save a session note: `temper session save`
5. Use the finishing-a-development-branch skill to decide on merge/PR strategy
