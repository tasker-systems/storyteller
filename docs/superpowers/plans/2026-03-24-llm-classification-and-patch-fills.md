# LLM Classification and Patch Fills — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enrich the ground-state narrative corpus with LLM patch fills for dynamics fields, consolidated trope families, state variable normalization, and extended LLM fills for spatial-topology and ontological-posture.

**Architecture:** Four workstreams reusing the existing `llm_patch.py` orchestrator, `OllamaClient`, shell script batch pattern, and ground-state loader. New code is extraction functions, normalization logic, and a CLI audit command. No schema or migration changes.

**Tech Stack:** Python 3.11+, Click CLI, Ollama (qwen2.5:7b-instruct), psycopg, Pydantic, pytest, ruff

**Spec:** `docs/superpowers/specs/2026-03-24-llm-classification-and-patch-fills-design.md`

---

### Task 1: Rewrite trope family normalization with dimension keyword extraction

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/persistence/trope_families.py`
- Test: `tools/narrative-data/tests/test_loader.py` (class `TestTropeFamilyNormalization`)

- [ ] **Step 1: Update tests for dimension-keyword normalization**

Replace the existing `TestTropeFamilyNormalization` test class to assert the new behavior. The key behavioral change: long derivations (> 100 chars) are no longer "unclassified" — they get classified by keyword extraction. Colon-split still works because keyword scan happens on the full string.

```python
class TestTropeFamilyNormalization:
    def test_normalize_simple_dimension_keyword(self) -> None:
        assert normalize_family_name("Temporal Dimensions: Seasonal") == "temporal-dimension"

    def test_normalize_agency_dimension(self) -> None:
        assert normalize_family_name("Agency Dimension") == "agency-dimension"

    def test_normalize_locus_of_power(self) -> None:
        assert normalize_family_name("Locus of Power: Community") == "locus-of-power"

    def test_normalize_world_affordance_with_hyphen(self) -> None:
        assert normalize_family_name("World-Affordance (violence)") == "world-affordance"

    def test_normalize_epistemological_stance(self) -> None:
        assert normalize_family_name("epistemological stance") == "epistemological-stance"

    def test_normalize_ontological_posture(self) -> None:
        assert normalize_family_name("Ontological dimension and agency dimension") == "ontological-posture"

    def test_normalize_state_variable_reference(self) -> None:
        assert normalize_family_name("State Variables (trauma, morale)") == "state-variable"

    def test_long_derivation_classified_by_keyword(self) -> None:
        long_text = (
            "Directly addresses the Agency Dimensions and Boundary Conditions"
            " (Melodrama vs. Tragedy). The hero must be competent to fall from greatness."
        )
        assert normalize_family_name(long_text) == "agency-dimension"

    def test_compound_dimensions_picks_first(self) -> None:
        # "aesthetic" appears before "structural" in keyword scan order
        assert normalize_family_name(
            "Aesthetic dimension (gritty realism) and structural dimension (mystery)"
        ) == "aesthetic-dimension"

    def test_secondary_keyword_identity(self) -> None:
        assert normalize_family_name("Identity and Belonging") == "thematic-dimension"

    def test_secondary_keyword_mystery(self) -> None:
        assert normalize_family_name("Mystery + Tragedy") == "structural-dimension"

    def test_secondary_keyword_magic(self) -> None:
        assert normalize_family_name("Magic is Domesticated / Rule-Bound") == "world-affordance"

    def test_secondary_keyword_social_capital(self) -> None:
        assert normalize_family_name("Social Capital and Collective Success") == "thematic-dimension"

    def test_primary_keyword_takes_precedence_over_secondary(self) -> None:
        # "agency" is a primary keyword, matches before "power" secondary
        assert normalize_family_name("power and its corruption, high agency") == "agency-dimension"

    def test_empty_string_returns_genre_specific(self) -> None:
        assert normalize_family_name("") == "genre-specific"

    def test_whitespace_only_returns_genre_specific(self) -> None:
        assert normalize_family_name("   ") == "genre-specific"

    def test_build_normalization_map_from_corpus(self, tmp_path: Path) -> None:
        corpus_dir = tmp_path / "corpus"
        tropes_dir = corpus_dir / "genres" / "folk-horror"
        tropes_dir.mkdir(parents=True)
        tropes = [
            {"name": "T1", "genre_derivation": "Temporal Dimensions: Seasonal"},
            {"name": "T2", "genre_derivation": "Identity and Belonging"},
            {"name": "T3", "genre_derivation": "Locus of Power: Community"},
        ]
        (tropes_dir / "tropes.json").write_text(json.dumps(tropes))

        nmap = build_normalization_map(corpus_dir)
        assert nmap["Temporal Dimensions: Seasonal"] == "temporal-dimension"
        assert nmap["Identity and Belonging"] == "thematic-dimension"
        assert nmap["Locus of Power: Community"] == "locus-of-power"

    def test_extract_trope_families_deduplicates(self) -> None:
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
        nmap = {"Locus of Power: X": "locus-of-power"}
        families = extract_trope_families(nmap)
        f = families[0]
        assert f["slug"] == "locus-of-power"
        assert f["name"] == "Locus of Power"
        assert "description" in f
```

- [ ] **Step 2: Run tests to verify failures**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestTropeFamilyNormalization -v`
Expected: Multiple failures — `test_long_derivation_classified_by_keyword` returns "unclassified", secondary keyword tests fail.

- [ ] **Step 3: Rewrite `normalize_family_name()` in `trope_families.py`**

Replace the function body with dimension keyword extraction:

```python
_DIMENSION_KEYWORDS: list[tuple[str, str]] = [
    # Multi-word matches first (most specific)
    ("world affordance", "world-affordance"),
    ("world-affordance", "world-affordance"),
    ("locus of power", "locus-of-power"),
    ("locus-of-power", "locus-of-power"),
    ("state variable", "state-variable"),
    ("state-variable", "state-variable"),
    # Single-word matches (ordered by specificity)
    ("epistemological", "epistemological-stance"),
    ("ontological", "ontological-posture"),
    ("aesthetic", "aesthetic-dimension"),
    ("tonal", "tonal-dimension"),
    ("temporal", "temporal-dimension"),
    ("thematic", "thematic-dimension"),
    ("agency", "agency-dimension"),
    ("structural", "structural-dimension"),
]

_SECONDARY_KEYWORDS: list[tuple[str, str]] = [
    ("identity", "thematic-dimension"),
    ("belonging", "thematic-dimension"),
    ("social capital", "thematic-dimension"),
    ("collective", "thematic-dimension"),
    ("solidarity", "thematic-dimension"),
    ("materiality", "thematic-dimension"),
    ("memory", "thematic-dimension"),
    ("mystery", "structural-dimension"),
    ("tragedy", "structural-dimension"),
    ("magic", "world-affordance"),
    ("medical", "world-affordance"),
    ("violence", "world-affordance"),
    ("antagonistic", "locus-of-power"),
    ("power", "locus-of-power"),
]


def normalize_family_name(raw: str) -> str:
    """Normalize a genre_derivation string to a canonical family slug.

    Scans for dimension keywords (primary then secondary) to classify
    the derivation into one of ~11 canonical trope families.
    """
    text = raw.strip()
    if not text:
        return "genre-specific"

    lower = text.lower()

    for keyword, family in _DIMENSION_KEYWORDS:
        if keyword in lower:
            return family

    for keyword, family in _SECONDARY_KEYWORDS:
        if keyword in lower:
            return family

    return "genre-specific"
```

Remove the `_MAX_FAMILY_LENGTH` constant and `import re` — no longer needed (all strings get classified by keyword, no regex used).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py::TestTropeFamilyNormalization -v`
Expected: All PASS

- [ ] **Step 5: Update the DB integration test assertion**

In `TestTropeFamilyLoading.test_upsert_trope_families_extracts_from_corpus`, the `_minimal_corpus` fixture has `genre_derivation: "Temporal: Seasonal"`. With the new logic, "temporal" keyword matches → `"temporal-dimension"`. Update the assertion at line 729:

```python
# Was: assert "temporal" in slugs
assert "temporal-dimension" in slugs
```

- [ ] **Step 6: Run full test suite and lint**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py -v && uv run ruff check .`
Expected: All tests pass, no lint violations

- [ ] **Step 7: Verify against real corpus**

Run: `STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run --project tools/narrative-data python -c "
from narrative_data.persistence.trope_families import build_normalization_map, extract_trope_families
from pathlib import Path
from collections import Counter
corpus_dir = Path('/Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data')
nmap = build_normalization_map(corpus_dir)
families = extract_trope_families(nmap)
slug_counts = Counter(nmap.values())
for f in sorted(families, key=lambda x: -slug_counts[x['slug']]):
    print(f'{slug_counts[f[\"slug\"]]:3d}  {f[\"slug\"]}')
print(f'\n--- {len(families)} families from {len(nmap)} raw derivations ---')
"`

Expected: ~11 families (no "unclassified"), 0 or near-0 "genre-specific".

- [ ] **Step 8: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/trope_families.py tools/narrative-data/tests/test_loader.py
git commit -m "feat: rewrite trope family normalization with dimension keyword extraction

Replaces colon-splitting heuristic with ordered keyword scan against
11 canonical dimension families. Collapses 115 families to ~11.
Secondary keyword table catches 12 edge cases that don't reference
a dimension directly."
```

---

### Task 2: Smoke test dynamics LLM patch fills on one genre

**Files:**
- No code changes (existing infrastructure)

- [ ] **Step 1: Verify Ollama is running with the model loaded**

Run: `curl -s http://localhost:11434/api/tags | python3 -c "import sys,json; tags=json.load(sys.stdin); models=[m['name'] for m in tags.get('models',[])]; print('\n'.join(models)); assert any('7b-instruct' in m for m in models), 'qwen2.5:7b-instruct not found'"`
Expected: Model list includes qwen2.5:7b-instruct

- [ ] **Step 2: Run smoke test on folk-horror dynamics**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data fill --tier llm-patch --type dynamics --genre folk-horror`
Expected: Summary table showing files_processed=1, entities_updated > 0

- [ ] **Step 3: Review the output**

Run: `cd /Users/petetaylor/projects/tasker-systems/storyteller-data && git diff narrative-data/discovery/dynamics/folk-horror.json | head -80`
Expected: Diffs showing `valence`, `currencies`, `scale_manifestations` fields populated with reasonable values. Review quality — valence should be one of the 7 valid values, currencies should be 2-6 short strings, scale_manifestations should have at least one non-null sub-field.

- [ ] **Step 4: If quality is acceptable, proceed to Task 3. If not, iterate on prompts in `llm_patch.py` and re-test.**

---

### Task 3: Create and run dynamics patch fill shell script

**Files:**
- Create: `tools/narrative-data/scripts/run-dynamics-patch-fill.sh`

- [ ] **Step 1: Create the batch script**

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Run LLM patch fills for dynamics type across all 30 genres.
# Batches of 5-6 genres with 30s cooling between batches.
set -euo pipefail

cd "$(dirname "$0")/.."

genre_flags() {
    IFS=',' read -ra GENRES <<< "$1"
    for g in "${GENRES[@]}"; do echo -n "--genre $g "; done
}

BATCH1="folk-horror,cosmic-horror,horror-comedy,high-epic-fantasy,dark-fantasy"
BATCH2="cozy-fantasy,fairy-tale-mythic,urban-fantasy,quiet-contemplative-fantasy,hard-sci-fi"
BATCH3="space-opera,cyberpunk,solarpunk,dystopian,post-apocalyptic"
BATCH4="cozy-mystery,noir,psychological-thriller,spy-thriller,detective-procedural"
BATCH5="contemporary-romance,historical-romance,paranormal-romance,romantasy,literary-fiction"
BATCH6="historical-fiction,southern-gothic,magical-realism,afrofuturism,classical-tragedy,pastoral"

echo "Starting dynamics LLM patch fill — $(date)"

for i in 1 2 3 4 5 6; do
    BATCH_VAR="BATCH$i"
    GENRES="${!BATCH_VAR}"
    echo ""
    echo "=== Batch $i/6: $GENRES ==="
    eval uv run narrative-data fill --tier llm-patch --type dynamics $(genre_flags "$GENRES")
    if [ "$i" -lt 6 ]; then
        echo "Cooling for 30s..."
        sleep 30
    fi
done

echo ""
echo "Dynamics patch fill complete — $(date)"
```

- [ ] **Step 2: Make executable and commit**

```bash
chmod +x tools/narrative-data/scripts/run-dynamics-patch-fill.sh
git add tools/narrative-data/scripts/run-dynamics-patch-fill.sh
git commit -m "feat: add shell script for dynamics LLM patch fill across all genres"
```

- [ ] **Step 3: Kick off in background**

Run in background: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data bash scripts/run-dynamics-patch-fill.sh`

This runs ~873 Ollama calls across 30 genres. Proceed to Task 4 while it runs.

---

### Task 4: Build state variable normalization module

**Files:**
- Create: `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py`
- Create: `tools/narrative-data/tests/test_sv_normalization.py`

- [ ] **Step 1: Write tests for `normalize_sv_slug()`**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for state variable slug normalization."""

from narrative_data.persistence.sv_normalization import (
    normalize_sv_slug,
    resolve_sv_slug,
)


class TestNormalizeSvSlug:
    def test_already_canonical(self) -> None:
        assert normalize_sv_slug("community-trust") == "community-trust"

    def test_title_case(self) -> None:
        assert normalize_sv_slug("Community Trust") == "community-trust"

    def test_underscores(self) -> None:
        assert normalize_sv_slug("moral_stance") == "moral-stance"

    def test_mixed_case_underscores(self) -> None:
        assert normalize_sv_slug("Moral_Stance") == "moral-stance"

    def test_strips_parentheticals(self) -> None:
        assert normalize_sv_slug("Knowledge (Secrets Known)") == "knowledge"

    def test_strips_whitespace(self) -> None:
        assert normalize_sv_slug("  community-trust  ") == "community-trust"

    def test_collapses_hyphens(self) -> None:
        assert normalize_sv_slug("moral--stance") == "moral-stance"

    def test_strips_trailing_hyphens(self) -> None:
        assert normalize_sv_slug("-community-trust-") == "community-trust"

    def test_empty_string(self) -> None:
        assert normalize_sv_slug("") == ""


class TestResolveSvSlug:
    def _canonical(self) -> set[str]:
        return {
            "community-trust",
            "moral-stance",
            "sanctuary-integrity",
            "knowledge-gap",
            "social-capital",
            "energy",
        }

    def test_exact_match_after_normalization(self) -> None:
        kind, slug = resolve_sv_slug("Community Trust", self._canonical())
        assert kind == "exact"
        assert slug == "community-trust"

    def test_prefix_match(self) -> None:
        kind, slug = resolve_sv_slug("sanctuary", self._canonical())
        assert kind == "prefix"
        assert slug == "sanctuary-integrity"

    def test_unresolved(self) -> None:
        kind, slug = resolve_sv_slug("completely-unknown", self._canonical())
        assert kind == "unresolved"
        assert slug is None

    def test_already_canonical_is_exact(self) -> None:
        kind, slug = resolve_sv_slug("energy", self._canonical())
        assert kind == "exact"
        assert slug == "energy"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_sv_normalization.py -v`
Expected: ImportError — module doesn't exist yet

- [ ] **Step 3: Implement `sv_normalization.py`**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""State variable slug normalization and resolution.

Normalizes inconsistent state variable references (title case, underscores,
parenthetical suffixes) and resolves them against the canonical set via
exact match, then prefix match.
"""

from __future__ import annotations

import re


def normalize_sv_slug(raw: str) -> str:
    """Normalize a raw state variable reference to a canonical slug form.

    1. Strip whitespace
    2. Lowercase
    3. Strip parenthetical suffixes
    4. Replace underscores and spaces with hyphens
    5. Collapse multiple hyphens
    """
    text = raw.strip().lower()
    text = re.sub(r"\s*\(.*?\)\s*", "", text)
    text = text.replace("_", "-").replace(" ", "-")
    text = re.sub(r"-+", "-", text).strip("-")
    return text


def resolve_sv_slug(
    raw: str,
    canonical_slugs: set[str],
) -> tuple[str, str | None]:
    """Resolve a raw state variable reference against the canonical set.

    Returns:
        Tuple of (resolution_kind, resolved_slug) where resolution_kind is
        one of "exact", "prefix", or "unresolved".
    """
    normalized = normalize_sv_slug(raw)
    if not normalized:
        return ("unresolved", None)

    # Exact match
    if normalized in canonical_slugs:
        return ("exact", normalized)

    # Prefix match: normalized is a prefix of exactly one canonical slug
    prefix_matches = [c for c in canonical_slugs if c.startswith(normalized + "-")]
    if len(prefix_matches) == 1:
        return ("prefix", prefix_matches[0])

    return ("unresolved", None)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_sv_normalization.py -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/sv_normalization.py tools/narrative-data/tests/test_sv_normalization.py
git commit -m "feat: add state variable slug normalization with exact and prefix matching"
```

---

### Task 5: Run state variable audit against real corpus

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py` (add audit function)
- Modify: `tools/narrative-data/src/narrative_data/cli.py` (add sv-audit command)

- [ ] **Step 1: Add `audit_sv_resolution()` function to `sv_normalization.py`**

```python
def audit_sv_resolution(
    corpus_dir: Path,
    canonical_slugs: set[str],
) -> dict[str, list[dict]]:
    """Scan all entity payloads and classify state variable references.

    Returns:
        Dict with keys "exact", "prefix", "unresolved", each mapping to
        a list of dicts with "raw", "normalized", "resolved", "type", "genre", "entity".
    """
    import json

    results: dict[str, list[dict]] = {"exact": [], "prefix": [], "unresolved": []}

    # Only types that have state_variable_interactions, state_variable_expression,
    # or state_variables fields in their entity payloads.
    for type_slug in ("goals", "dynamics", "tropes", "archetypes", "place-entities"):
        type_dir = corpus_dir / "discovery" / type_slug
        if not type_dir.exists():
            # Try genre-native path
            _scan_genre_native(corpus_dir, type_slug, canonical_slugs, results)
            continue

        for json_path in sorted(type_dir.glob("*.json")):
            if json_path.name in ("manifest.json",) or json_path.name.endswith(".errors.json"):
                continue
            try:
                data = json.loads(json_path.read_text())
            except (json.JSONDecodeError, OSError):
                continue
            if not isinstance(data, list):
                continue
            genre_slug = json_path.stem
            for entity in data:
                if not isinstance(entity, dict):
                    continue
                _collect_sv_refs(entity, type_slug, genre_slug, canonical_slugs, results)

    return results


def _scan_genre_native(
    corpus_dir: Path,
    type_slug: str,
    canonical_slugs: set[str],
    results: dict[str, list[dict]],
) -> None:
    import json

    genres_dir = corpus_dir / "genres"
    if not genres_dir.exists():
        return
    for genre_dir in sorted(genres_dir.iterdir()):
        if not genre_dir.is_dir():
            continue
        json_path = genre_dir / f"{type_slug}.json"
        if not json_path.exists():
            continue
        try:
            data = json.loads(json_path.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        if not isinstance(data, list):
            continue
        for entity in data:
            if not isinstance(entity, dict):
                continue
            _collect_sv_refs(entity, type_slug, genre_dir.name, canonical_slugs, results)


def _collect_sv_refs(
    entity: dict,
    type_slug: str,
    genre_slug: str,
    canonical_slugs: set[str],
    results: dict[str, list[dict]],
) -> None:
    entity_name = entity.get("canonical_name") or entity.get("name") or entity.get("default_subject") or "?"
    raw_refs: list[str] = []

    # StateVariableInteraction shape (goals, dynamics, tropes)
    for ix in entity.get("state_variable_interactions", []):
        if isinstance(ix, dict):
            vid = ix.get("variable_id")
            if isinstance(vid, str) and vid.strip():
                raw_refs.append(vid)
    # StateVariableExpression shape (place-entities)
    for expr in entity.get("state_variable_expression", []):
        if isinstance(expr, dict):
            vid = expr.get("variable_id")
            if isinstance(vid, str) and vid.strip():
                raw_refs.append(vid)
    # Bare list shape (archetypes)
    for sv in entity.get("state_variables", []):
        if isinstance(sv, str) and sv.strip():
            raw_refs.append(sv)

    for raw in raw_refs:
        kind, resolved = resolve_sv_slug(raw, canonical_slugs)
        results[kind].append({
            "raw": raw,
            "normalized": normalize_sv_slug(raw),
            "resolved": resolved,
            "type": type_slug,
            "genre": genre_slug,
            "entity": entity_name,
        })
```

Add `from pathlib import Path` to imports at top.

- [ ] **Step 2: Add `sv-audit` CLI command to `cli.py`**

Add a new command to the CLI group. Find the appropriate location (after the `audit` command):

```python
@cli.command("sv-audit")
def sv_audit() -> None:
    """Audit state variable references against canonical set and report resolution rates."""
    from rich.console import Console
    from rich.table import Table

    from narrative_data.config import resolve_output_path
    from narrative_data.persistence.reference_data import extract_state_variables
    from narrative_data.persistence.sv_normalization import audit_sv_resolution

    console = Console()
    try:
        corpus_dir = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    # Build canonical set from region.json files
    sv_list = extract_state_variables(corpus_dir)
    canonical = {sv["slug"] for sv in sv_list}
    console.print(f"Canonical state variables: {len(canonical)}")

    results = audit_sv_resolution(corpus_dir, canonical)

    exact_n = len(results["exact"])
    prefix_n = len(results["prefix"])
    unresolved_n = len(results["unresolved"])
    total = exact_n + prefix_n + unresolved_n

    console.print(f"\nTotal references scanned: {total}")
    console.print(f"  [green]Exact match:[/green]  {exact_n} ({100*exact_n/total:.1f}%)" if total else "")
    console.print(f"  [yellow]Prefix match:[/yellow] {prefix_n} ({100*prefix_n/total:.1f}%)" if total else "")
    console.print(f"  [red]Unresolved:[/red]   {unresolved_n} ({100*unresolved_n/total:.1f}%)" if total else "")

    if results["unresolved"]:
        console.print(f"\n[bold]Unresolved references ({unresolved_n}):[/bold]")
        table = Table(show_header=True, header_style="bold")
        table.add_column("Raw", style="red")
        table.add_column("Normalized")
        table.add_column("Type")
        table.add_column("Genre")
        table.add_column("Entity")
        seen = set()
        for ref in sorted(results["unresolved"], key=lambda r: r["normalized"]):
            key = (ref["normalized"], ref["type"], ref["genre"])
            if key in seen:
                continue
            seen.add(key)
            table.add_row(ref["raw"], ref["normalized"], ref["type"], ref["genre"], ref["entity"])
        console.print(table)

    if results["prefix"]:
        console.print(f"\n[bold]Prefix matches ({prefix_n}):[/bold]")
        table = Table(show_header=True, header_style="bold")
        table.add_column("Raw", style="yellow")
        table.add_column("Resolved")
        seen = set()
        for ref in sorted(results["prefix"], key=lambda r: r["normalized"]):
            key = ref["normalized"]
            if key in seen:
                continue
            seen.add(key)
            table.add_row(ref["raw"], ref["resolved"])
        console.print(table)
```

- [ ] **Step 3: Run the audit**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data sv-audit`

Analyze output:
- If unresolved < 30: proceed to build `_MANUAL_SV_MAP` in Task 6
- If unresolved >= 30: document findings, defer to next session

- [ ] **Step 4: Lint check**

Run: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check .`

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/sv_normalization.py tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat: add sv-audit command for state variable reference triage"
```

---

### Task 6: Wire state variable normalization into loader

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/persistence/loader.py:858-868`
- Modify: `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py` (add manual map if needed)

This task depends on the audit results from Task 5. If the audit reveals a small number of unresolved references, add a `_MANUAL_SV_MAP` dict. If the number is large, skip the manual map and document.

- [ ] **Step 1: Add manual mapping table (if needed based on audit)**

If unresolved count is manageable, add to `sv_normalization.py`:

```python
# Manual overrides for semantic mismatches that can't be resolved by normalization + prefix.
# Built from sv-audit results. Maps normalized-but-unresolved → canonical slug.
_MANUAL_SV_MAP: dict[str, str] = {
    # Populate from audit results, e.g.:
    # "moral-standing": "moral-stance",
    # "community-bond": "community-trust",
}


def resolve_sv_slug(
    raw: str,
    canonical_slugs: set[str],
) -> tuple[str, str | None]:
    normalized = normalize_sv_slug(raw)
    if not normalized:
        return ("unresolved", None)

    if normalized in canonical_slugs:
        return ("exact", normalized)

    # Manual override
    if normalized in _MANUAL_SV_MAP:
        mapped = _MANUAL_SV_MAP[normalized]
        if mapped in canonical_slugs:
            return ("manual", mapped)

    prefix_matches = [c for c in canonical_slugs if c.startswith(normalized + "-")]
    if len(prefix_matches) == 1:
        return ("prefix", prefix_matches[0])

    return ("unresolved", None)
```

- [ ] **Step 2: Wire into loader**

In `loader.py` at line 858-868, add the normalization pre-lookup:

```python
# At top of file, add import:
from narrative_data.persistence.sv_normalization import resolve_sv_slug

# In _load_state_variable_interactions, replace the direct lookup (line 859):
# Was:
#     sv_id = sv_map.get(ix["variable_slug"])
# Now:
                    raw_slug = ix["variable_slug"]
                    _kind, resolved = resolve_sv_slug(raw_slug, set(sv_map.keys()))
                    sv_id = sv_map.get(resolved) if resolved else None
```

- [ ] **Step 3: Run loader tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_loader.py -v`
Expected: All existing tests pass (normalization is transparent for already-canonical slugs)

- [ ] **Step 4: Commit**

```bash
git add tools/narrative-data/src/narrative_data/persistence/loader.py tools/narrative-data/src/narrative_data/persistence/sv_normalization.py
git commit -m "feat: wire state variable normalization into ground-state loader

resolve_sv_slug() normalizes raw references before lookup in
_load_state_variable_interactions(). Resolves title case, underscores,
parenthetical suffixes, and prefix matches automatically."
```

---

### Task 7: Add spatial-topology and ontological-posture extraction functions

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/llm_patch.py`
- Modify: `tools/narrative-data/tests/test_llm_patch.py`

- [ ] **Step 1: Write tests for spatial-topology extraction functions**

Add to `test_llm_patch.py`:

```python
from narrative_data.pipeline.llm_patch import (
    extract_state_shift,
    extract_directionality_description,
    extract_friction_description,
)


class TestExtractStateShift:
    def test_returns_state_shift_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "The hero transforms from mortal to marked."
        entity = {"source_setting": "A", "target_setting": "B", "state_shift": None}
        result = extract_state_shift(entity, "source markdown", mock_client)
        assert result["state_shift"] == "The hero transforms from mortal to marked."

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"state_shift": "Already filled."}
        result = extract_state_shift(entity, "irrelevant", mock_client)
        mock_client.generate.assert_not_called()
        assert result["state_shift"] == "Already filled."

    def test_does_not_mutate_original(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "Shift description."
        entity = {"source_setting": "A", "target_setting": "B", "state_shift": None}
        result = extract_state_shift(entity, "md", mock_client)
        assert entity["state_shift"] is None
        assert result["state_shift"] == "Shift description."


class TestExtractDirectionalityDescription:
    def test_returns_description_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "One-way passage, no return."
        entity = {"source_setting": "A", "target_setting": "B",
                  "directionality": {"type": "one_way", "description": None}}
        result = extract_directionality_description(entity, "source md", mock_client)
        assert result["directionality"]["description"] == "One-way passage, no return."

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"directionality": {"type": "one_way", "description": "Already here."}}
        result = extract_directionality_description(entity, "md", mock_client)
        mock_client.generate.assert_not_called()

    def test_handles_missing_directionality_dict(self):
        mock_client = MagicMock()
        entity = {"source_setting": "A", "target_setting": "B"}
        result = extract_directionality_description(entity, "md", mock_client)
        mock_client.generate.assert_not_called()
        assert result == entity


class TestExtractFrictionDescription:
    def test_returns_description_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "Environmental resistance, dense fog."
        entity = {"source_setting": "A", "target_setting": "B",
                  "friction": {"type": "environmental", "level": "high", "description": None}}
        result = extract_friction_description(entity, "source md", mock_client)
        assert result["friction"]["description"] == "Environmental resistance, dense fog."

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"friction": {"type": "x", "level": "low", "description": "Filled."}}
        result = extract_friction_description(entity, "md", mock_client)
        mock_client.generate.assert_not_called()
```

- [ ] **Step 2: Write tests for ontological-posture extraction functions**

Add to `test_llm_patch.py`:

```python
from narrative_data.pipeline.llm_patch import (
    extract_crossing_rules,
    extract_obligations_across,
)


class TestExtractCrossingRules:
    def test_returns_crossing_rules_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "Crossing requires sacrifice of innocence."
        entity = {"default_subject": "The Community",
                  "self_other_boundary": {"stability": "rigid", "crossing_rules": None, "obligations_across": None}}
        result = extract_crossing_rules(entity, "source md", mock_client)
        assert result["self_other_boundary"]["crossing_rules"] == "Crossing requires sacrifice of innocence."

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"self_other_boundary": {"crossing_rules": "Already set.", "obligations_across": None}}
        result = extract_crossing_rules(entity, "md", mock_client)
        mock_client.generate.assert_not_called()

    def test_handles_missing_boundary_dict(self):
        mock_client = MagicMock()
        entity = {"default_subject": "X"}
        result = extract_crossing_rules(entity, "md", mock_client)
        mock_client.generate.assert_not_called()


class TestExtractObligationsAcross:
    def test_returns_obligations_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "Reciprocal truth and safety."
        entity = {"default_subject": "The Lovers",
                  "self_other_boundary": {"stability": "fluid", "crossing_rules": None, "obligations_across": None}}
        result = extract_obligations_across(entity, "source md", mock_client)
        assert result["self_other_boundary"]["obligations_across"] == "Reciprocal truth and safety."

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"self_other_boundary": {"obligations_across": "Set.", "crossing_rules": None}}
        result = extract_obligations_across(entity, "md", mock_client)
        mock_client.generate.assert_not_called()
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_llm_patch.py::TestExtractStateShift tests/test_llm_patch.py::TestExtractDirectionalityDescription tests/test_llm_patch.py::TestExtractFrictionDescription tests/test_llm_patch.py::TestExtractCrossingRules tests/test_llm_patch.py::TestExtractObligationsAcross -v`
Expected: ImportError — functions don't exist yet

- [ ] **Step 4: Implement extraction functions in `llm_patch.py`**

Add after the existing `extract_scale_manifestations` function:

```python
# ---------------------------------------------------------------------------
# spatial-topology extraction functions
# ---------------------------------------------------------------------------


def extract_state_shift(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill the ``state_shift`` field of a spatial-topology entity."""
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    existing = result.get("state_shift")
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    source = str(result.get("source_setting", ""))
    target = str(result.get("target_setting", ""))
    section = _find_entity_section(md_content, source) if source else ""
    section_snippet = section[:800] if section else "(no source section found)"

    prompt = (
        f"You are extracting the state shift that occurs when a character traverses "
        f"a spatial transition in a narrative.\n\n"
        f"Transition: {source} → {target}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Describe in 1-2 sentences what changes for the character (ontologically, "
        f"emotionally, or narratively) when they move through this transition. "
        f"If the source doesn't describe a meaningful shift, respond with just 'null'."
    )

    try:
        raw = client.generate(model=STRUCTURING_MODEL, prompt=prompt, temperature=0.1)
        value = raw.strip()
        if value and value.lower() != "null":
            result = dict(result)
            result["state_shift"] = value
    except Exception:
        pass

    return result


def extract_directionality_description(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill ``directionality.description`` of a spatial-topology entity."""
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    directionality = result.get("directionality")
    if not isinstance(directionality, dict):
        return result
    existing = directionality.get("description")
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    source = str(result.get("source_setting", ""))
    target = str(result.get("target_setting", ""))
    dir_type = str(directionality.get("type", ""))
    section = _find_entity_section(md_content, source) if source else ""
    section_snippet = section[:600] if section else "(no source section found)"

    prompt = (
        f"You are describing the directionality of a spatial transition in a narrative.\n\n"
        f"Transition: {source} → {target}\n"
        f"Directionality type: {dir_type}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Describe in 1-2 sentences how this transition's directionality works narratively — "
        f"what makes it one-way, bidirectional, or asymmetric. "
        f"If the source doesn't describe directionality, respond with just 'null'."
    )

    try:
        raw = client.generate(model=STRUCTURING_MODEL, prompt=prompt, temperature=0.1)
        value = raw.strip()
        if value and value.lower() != "null":
            result = dict(result)
            result["directionality"] = dict(directionality)
            result["directionality"]["description"] = value
    except Exception:
        pass

    return result


def extract_friction_description(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill ``friction.description`` of a spatial-topology entity."""
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    friction = result.get("friction")
    if not isinstance(friction, dict):
        return result
    existing = friction.get("description")
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    source = str(result.get("source_setting", ""))
    target = str(result.get("target_setting", ""))
    friction_type = str(friction.get("type", ""))
    friction_level = str(friction.get("level", ""))
    section = _find_entity_section(md_content, source) if source else ""
    section_snippet = section[:600] if section else "(no source section found)"

    prompt = (
        f"You are describing the friction of a spatial transition in a narrative.\n\n"
        f"Transition: {source} → {target}\n"
        f"Friction type: {friction_type}, level: {friction_level}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Describe in 1-2 sentences what creates resistance or difficulty in this transition — "
        f"the nature of the friction characters experience. "
        f"If the source doesn't describe friction, respond with just 'null'."
    )

    try:
        raw = client.generate(model=STRUCTURING_MODEL, prompt=prompt, temperature=0.1)
        value = raw.strip()
        if value and value.lower() != "null":
            result = dict(result)
            result["friction"] = dict(friction)
            result["friction"]["description"] = value
    except Exception:
        pass

    return result


# ---------------------------------------------------------------------------
# ontological-posture extraction functions
# ---------------------------------------------------------------------------


def extract_crossing_rules(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill ``self_other_boundary.crossing_rules`` of an ontological-posture entity."""
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    boundary = result.get("self_other_boundary")
    if not isinstance(boundary, dict):
        return result
    existing = boundary.get("crossing_rules")
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    subject = str(result.get("default_subject", ""))
    stability = str(boundary.get("stability", ""))
    section = _find_entity_section(md_content, "crossing") if md_content else ""
    if not section:
        section = _find_entity_section(md_content, subject) if subject else ""
    section_snippet = section[:800] if section else "(no source section found)"

    prompt = (
        f"You are extracting the rules for crossing the self/other boundary "
        f"in a narrative genre's ontological posture.\n\n"
        f"Subject: {subject}\n"
        f"Boundary stability: {stability}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Describe in 2-3 sentences the rules or mechanics of crossing this boundary — "
        f"what is required, what is risked, what the crossing means narratively. "
        f"If the source doesn't describe crossing rules, respond with just 'null'."
    )

    try:
        raw = client.generate(model=STRUCTURING_MODEL, prompt=prompt, temperature=0.1)
        value = raw.strip()
        if value and value.lower() != "null":
            result = dict(result)
            result["self_other_boundary"] = dict(boundary)
            result["self_other_boundary"]["crossing_rules"] = value
    except Exception:
        pass

    return result


def extract_obligations_across(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill ``self_other_boundary.obligations_across`` of an ontological-posture entity."""
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    boundary = result.get("self_other_boundary")
    if not isinstance(boundary, dict):
        return result
    existing = boundary.get("obligations_across")
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    subject = str(result.get("default_subject", ""))
    stability = str(boundary.get("stability", ""))
    section = _find_entity_section(md_content, "obligation") if md_content else ""
    if not section:
        section = _find_entity_section(md_content, subject) if subject else ""
    section_snippet = section[:800] if section else "(no source section found)"

    prompt = (
        f"You are extracting the obligations that exist across the self/other boundary "
        f"in a narrative genre's ontological posture.\n\n"
        f"Subject: {subject}\n"
        f"Boundary stability: {stability}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Describe in 2-3 sentences what characters owe each other across this boundary — "
        f"what obligations, debts, or reciprocal duties exist. "
        f"If the source doesn't describe obligations, respond with just 'null'."
    )

    try:
        raw = client.generate(model=STRUCTURING_MODEL, prompt=prompt, temperature=0.1)
        value = raw.strip()
        if value and value.lower() != "null":
            result = dict(result)
            result["self_other_boundary"] = dict(boundary)
            result["self_other_boundary"]["obligations_across"] = value
    except Exception:
        pass

    return result
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_llm_patch.py -v`
Expected: All tests pass (old + new)

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/llm_patch.py tools/narrative-data/tests/test_llm_patch.py
git commit -m "feat: add extraction functions for spatial-topology and ontological-posture fields

Five new functions: extract_state_shift, extract_directionality_description,
extract_friction_description, extract_crossing_rules, extract_obligations_across.
All follow the immutable-dict, skip-if-populated, focused-prompt pattern."
```

---

### Task 8: Wire new types into `fill_all_llm_patch` orchestrator

**Note — spec deviation**: The spec (line 222) claims spatial-topology and ontological-posture are "genre-native types" needing a path resolution change in `fill_all_llm_patch`. This is incorrect — both are `PRIMITIVE_TYPES` in `config.py` (lines 28-38), and their `.md`/`.json` files live under `discovery/{type}/`, matching the existing path resolution in `fill_all_llm_patch` (line 335). No path resolution change is needed.

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/llm_patch.py:264-368`
- Modify: `tools/narrative-data/tests/test_llm_patch.py`

- [ ] **Step 1: Write orchestration tests for new types**

Add to the `TestFillAllLlmPatch` class in `test_llm_patch.py`:

```python
    def test_processes_spatial_topology(self, tmp_path: Path):
        corpus = tmp_path / "narrative-data"
        st_dir = corpus / "discovery" / "spatial-topology"
        st_dir.mkdir(parents=True)
        (st_dir / "folk-horror.md").write_text("Source markdown for spatial topology.")
        entities = [
            {"source_setting": "Village", "target_setting": "Forest",
             "state_shift": None,
             "directionality": {"type": "one_way", "description": None},
             "friction": {"type": "environmental", "level": "high", "description": None}}
        ]
        (st_dir / "folk-horror.json").write_text(json.dumps(entities))

        mock_client = MagicMock()
        mock_client.generate.side_effect = ["Shift desc.", "Dir desc.", "Friction desc."]

        summary = fill_all_llm_patch(corpus, mock_client, types=["spatial-topology"], genres=None, dry_run=False)
        assert "spatial-topology" in summary
        assert summary["spatial-topology"]["entities_updated"] >= 1

        written = json.loads((st_dir / "folk-horror.json").read_text())
        assert written[0]["state_shift"] == "Shift desc."

    def test_processes_ontological_posture(self, tmp_path: Path):
        corpus = tmp_path / "narrative-data"
        op_dir = corpus / "discovery" / "ontological-posture"
        op_dir.mkdir(parents=True)
        (op_dir / "folk-horror.md").write_text("Source markdown for ontological posture.")
        entities = [
            {"default_subject": "The Community",
             "self_other_boundary": {"stability": "rigid", "crossing_rules": None, "obligations_across": None}}
        ]
        (op_dir / "folk-horror.json").write_text(json.dumps(entities))

        mock_client = MagicMock()
        mock_client.generate.side_effect = ["Crossing rules desc.", "Obligations desc."]

        summary = fill_all_llm_patch(corpus, mock_client, types=["ontological-posture"], genres=None, dry_run=False)
        assert "ontological-posture" in summary
        assert summary["ontological-posture"]["entities_updated"] >= 1

    def test_spatial_topology_now_supported(self, tmp_path: Path):
        """spatial-topology should no longer be skipped as unsupported."""
        corpus = tmp_path / "narrative-data"
        st_dir = corpus / "discovery" / "spatial-topology"
        st_dir.mkdir(parents=True)
        (st_dir / "folk-horror.md").write_text("")
        (st_dir / "folk-horror.json").write_text("[]")
        mock_client = MagicMock()
        summary = fill_all_llm_patch(corpus, mock_client, types=["spatial-topology"], genres=None, dry_run=False)
        assert "spatial-topology" in summary
```

Update the existing `test_unsupported_type_skipped` test — change the type to something truly unsupported (e.g., `"profiles"`):

```python
    def test_unsupported_type_skipped(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = MagicMock()
        summary = fill_all_llm_patch(
            corpus, mock_client, types=["profiles"], genres=None, dry_run=False
        )
        assert "profiles" not in summary
```

- [ ] **Step 2: Run tests to verify failures**

Run: `cd tools/narrative-data && uv run pytest tests/test_llm_patch.py::TestFillAllLlmPatch::test_processes_spatial_topology tests/test_llm_patch.py::TestFillAllLlmPatch::test_processes_ontological_posture -v`
Expected: FAIL — types not yet in `supported_patch_types`

- [ ] **Step 3: Update `fill_all_llm_patch()` to support new types**

In `llm_patch.py`, modify the orchestrator:

```python
    supported_patch_types = ["dynamics", "spatial-topology", "ontological-posture"]
```

Add type dispatch in the entity processing loop (replace lines 345-349):

```python
            for entity in entities:
                filled = entity
                if type_slug == "dynamics":
                    filled = extract_valence(filled, md_content, client)
                    filled = extract_currencies(filled, md_content, client)
                    filled = extract_scale_manifestations(filled, md_content, client)
                elif type_slug == "spatial-topology":
                    filled = extract_state_shift(filled, md_content, client)
                    filled = extract_directionality_description(filled, md_content, client)
                    filled = extract_friction_description(filled, md_content, client)
                elif type_slug == "ontological-posture":
                    filled = extract_crossing_rules(filled, md_content, client)
                    filled = extract_obligations_across(filled, md_content, client)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_llm_patch.py -v`
Expected: All PASS

- [ ] **Step 5: Lint check**

Run: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check .`

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/llm_patch.py tools/narrative-data/tests/test_llm_patch.py
git commit -m "feat: wire spatial-topology and ontological-posture into fill_all_llm_patch

Adds type dispatch for three spatial-topology fields (state_shift,
directionality.description, friction.description) and two ontological-posture
fields (crossing_rules, obligations_across)."
```

---

### Task 9: Smoke test and batch run spatial-topology and ontological-posture fills

**Files:**
- Create: `tools/narrative-data/scripts/run-spatial-topology-patch-fill.sh`
- Create: `tools/narrative-data/scripts/run-ontological-posture-patch-fill.sh`

- [ ] **Step 1: Smoke test spatial-topology**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data fill --tier llm-patch --type spatial-topology --genre folk-horror`
Review: `cd /Users/petetaylor/projects/tasker-systems/storyteller-data && git diff narrative-data/discovery/spatial-topology/folk-horror.json | head -80`

- [ ] **Step 2: Smoke test ontological-posture**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data fill --tier llm-patch --type ontological-posture --genre folk-horror`
Review: `cd /Users/petetaylor/projects/tasker-systems/storyteller-data && git diff narrative-data/discovery/ontological-posture/folk-horror.json | head -80`

- [ ] **Step 3: Create batch scripts (same pattern as Task 3)**

`run-spatial-topology-patch-fill.sh`:
```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Run LLM patch fills for spatial-topology type across all genres.
set -euo pipefail

cd "$(dirname "$0")/.."

genre_flags() {
    IFS=',' read -ra GENRES <<< "$1"
    for g in "${GENRES[@]}"; do echo -n "--genre $g "; done
}

BATCH1="folk-horror,cosmic-horror,horror-comedy,high-epic-fantasy,dark-fantasy"
BATCH2="cozy-fantasy,fairy-tale-mythic,urban-fantasy,quiet-contemplative-fantasy,hard-sci-fi"
BATCH3="space-opera,cyberpunk,solarpunk,dystopian,post-apocalyptic"
BATCH4="cozy-mystery,noir,psychological-thriller,spy-thriller,detective-procedural"
BATCH5="contemporary-romance,historical-romance,paranormal-romance,romantasy,literary-fiction"
BATCH6="historical-fiction,southern-gothic,magical-realism,afrofuturism,classical-tragedy,pastoral"

echo "Starting spatial-topology LLM patch fill — $(date)"

for i in 1 2 3 4 5 6; do
    BATCH_VAR="BATCH$i"
    GENRES="${!BATCH_VAR}"
    echo ""
    echo "=== Batch $i/6: $GENRES ==="
    eval uv run narrative-data fill --tier llm-patch --type spatial-topology $(genre_flags "$GENRES")
    if [ "$i" -lt 6 ]; then
        echo "Cooling for 30s..."
        sleep 30
    fi
done

echo ""
echo "Spatial-topology patch fill complete — $(date)"
```

`run-ontological-posture-patch-fill.sh` — identical but with `--type ontological-posture` and updated echo messages.

- [ ] **Step 4: Make executable and commit**

```bash
chmod +x tools/narrative-data/scripts/run-spatial-topology-patch-fill.sh tools/narrative-data/scripts/run-ontological-posture-patch-fill.sh
git add tools/narrative-data/scripts/run-spatial-topology-patch-fill.sh tools/narrative-data/scripts/run-ontological-posture-patch-fill.sh
git commit -m "feat: add batch scripts for spatial-topology and ontological-posture patch fills"
```

- [ ] **Step 5: Run batch scripts in background**

Run spatial-topology first, then ontological-posture. Both can be kicked off sequentially in a single background process if preferred, or run one at a time.

---

### Task 10: Final reload and verification

**Files:**
- No code changes

- [ ] **Step 1: Verify all patch fill runs completed**

Check that the dynamics, spatial-topology, and ontological-posture batch scripts finished without errors.

- [ ] **Step 2: Reload ground state**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data load-ground-state`
Expected: LoadReport showing inserts and updates across all types

- [ ] **Step 3: Run audit to verify null rate reduction**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data audit --type dynamics`
Expected: `valence`, `currencies`, `scale_manifestations` null rates significantly reduced.

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data audit --type spatial-topology`
Expected: `state_shift`, `directionality.description`, `friction.description` null rates reduced.

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data audit --type ontological-posture`
Expected: `crossing_rules`, `obligations_across` null rates reduced.

- [ ] **Step 4: Spot-check genre_context query**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run --project tools/narrative-data python -c "
import psycopg, json
conn = psycopg.connect('postgres://storyteller:storyteller@localhost:5435/storyteller_development')
with conn.cursor() as cur:
    cur.execute('SELECT ground_state.genre_context(%s)', ('folk-horror',))
    ctx = cur.fetchone()[0]
    tropes = ctx.get('tropes', [])
    families = set(t.get('family_slug') for t in tropes if t.get('family_slug'))
    print(f'Trope families in folk-horror: {sorted(families)}')
    dynamics = ctx.get('dynamics', [])
    filled = sum(1 for d in dynamics if d.get('data', {}).get('valence'))
    print(f'Dynamics with valence: {filled}/{len(dynamics)}')
conn.close()
"`

- [ ] **Step 5: Run SV audit to document final state**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data sv-audit`
Capture output for session note documentation.

- [ ] **Step 6: Final lint and test pass**

Run: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check . && uv run pytest tests/ -v`
Expected: All pass, no lint violations

- [ ] **Step 7: Save session note**

Run: `temper session save "LLM Classification and Patch Fills" --ticket 2026-03-24-llm-classification-and-patch-fills --state done --project storyteller`
