# Tome Phase 3c: Elicitation Prompt Decomposition for Scale — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single-model sequential elicitation pipeline with a fan-out/fan-in architecture using model tiering (7b/9b for parallel entity generation, 35b for per-stage coherence passes) to achieve 2x+ speedup while maintaining output quality.

**Architecture:** Each entity stage (places, orgs, substrate, characters) splits into a parallel fan-out phase (small model generates one entity per call) and a coherence phase (large model binds entities relationally). A pure-Python preamble compression step reduces the 36.5 KB world preamble to ~3-4 KB by dropping edge-traversal traces and grouping axes by domain. An agent-backed planning call determines entity counts and distribution.

**Tech Stack:** Python 3.11+, Click CLI, httpx (Ollama), ThreadPoolExecutor, pytest, ruff

**Spec:** `docs/superpowers/specs/2026-03-29-tome-phase-3c-elicitation-prompt-decomposition-for-scale-design.md`

---

## File Structure

### New Files

```
tools/narrative-data/src/narrative_data/tome/
    compress_preamble.py       # Pure Python: world-position.json → world-summary.json
    plan_entities.py           # 7b planning call: world-summary → entity-plan.json
    fan_out.py                 # Parallel dispatch: specs → individual entity instances
    cohere.py                  # 35b coherence calls: drafts → finals per stage
    orchestrate_decomposed.py  # Pipeline orchestrator: ties all stages together
    models.py                  # Model registry and FanOutSpec dataclass

tools/narrative-data/prompts/tome/decomposed/
    entity-plan.md                         # Planning call template
    place-fanout.md                        # Single place generation
    org-fanout.md                          # Single org generation
    substrate-fanout.md                    # Single cluster generation
    character-mundane-fanout.md            # Single Q1/Q2 character generation
    character-significant-fanout.md        # Single Q3/Q4 skeleton generation
    places-coherence.md                    # Place relational binding
    orgs-coherence.md                      # Org-place binding
    substrate-coherence.md                 # Pairwise cluster relationships
    characters-mundane-coherence.md        # Cluster distribution review
    characters-significant-coherence.md    # Deep relational binding (design principle)

tools/narrative-data/tests/tome/
    test_compress_preamble.py
    test_plan_entities.py
    test_fan_out.py
    test_cohere.py
    test_orchestrate_decomposed.py
```

### Modified Files

```
tools/narrative-data/src/narrative_data/config.py      # Add CREATIVE_MODEL constant
tools/narrative-data/src/narrative_data/cli.py          # Add elicit-decomposed command
```

---

## Task 1: Model Registry and FanOutSpec Dataclass

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/models.py`
- Modify: `tools/narrative-data/src/narrative_data/config.py`
- Test: `tools/narrative-data/tests/tome/test_models.py`

- [ ] **Step 1: Write the failing test**

Create `tools/narrative-data/tests/tome/test_models.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for model registry and FanOutSpec dataclass."""

import pytest


class TestModelRegistry:
    def test_has_fan_out_structured(self) -> None:
        from narrative_data.tome.models import MODEL_REGISTRY

        assert "fan_out_structured" in MODEL_REGISTRY
        assert MODEL_REGISTRY["fan_out_structured"] == "qwen2.5:7b-instruct"

    def test_has_fan_out_creative(self) -> None:
        from narrative_data.tome.models import MODEL_REGISTRY

        assert "fan_out_creative" in MODEL_REGISTRY
        assert MODEL_REGISTRY["fan_out_creative"] == "qwen3.5:9b"

    def test_has_coherence(self) -> None:
        from narrative_data.tome.models import MODEL_REGISTRY

        assert "coherence" in MODEL_REGISTRY
        assert MODEL_REGISTRY["coherence"] == "qwen3.5:35b"

    def test_get_model_valid(self) -> None:
        from narrative_data.tome.models import get_model

        assert get_model("coherence") == "qwen3.5:35b"

    def test_get_model_invalid_raises(self) -> None:
        from narrative_data.tome.models import get_model

        with pytest.raises(KeyError):
            get_model("nonexistent")


class TestFanOutSpec:
    def test_construction(self) -> None:
        from narrative_data.tome.models import FanOutSpec

        spec = FanOutSpec(
            stage="places",
            index=0,
            template_name="place-fanout.md",
            model_role="fan_out_structured",
            context={"axes": "test", "type_hint": "infrastructure"},
        )
        assert spec.stage == "places"
        assert spec.index == 0
        assert spec.model_role == "fan_out_structured"

    def test_output_filename(self) -> None:
        from narrative_data.tome.models import FanOutSpec

        spec = FanOutSpec(
            stage="places",
            index=3,
            template_name="place-fanout.md",
            model_role="fan_out_structured",
            context={},
        )
        assert spec.output_filename == "instance-004.json"

    def test_output_filename_zero_padded(self) -> None:
        from narrative_data.tome.models import FanOutSpec

        spec = FanOutSpec(
            stage="orgs",
            index=11,
            template_name="org-fanout.md",
            model_role="fan_out_structured",
            context={},
        )
        assert spec.output_filename == "instance-012.json"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_models.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'narrative_data.tome.models'`

- [ ] **Step 3: Add CREATIVE_MODEL to config.py**

In `tools/narrative-data/src/narrative_data/config.py`, after line 81 (`ELICITATION_MODEL = "qwen3.5:35b"`), add:

```python
CREATIVE_MODEL = "qwen3.5:9b"
```

- [ ] **Step 4: Write the implementation**

Create `tools/narrative-data/src/narrative_data/tome/models.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Model registry and FanOutSpec dataclass for the decomposed elicitation pipeline."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from narrative_data.config import CREATIVE_MODEL, ELICITATION_MODEL, STRUCTURING_MODEL

MODEL_REGISTRY: dict[str, str] = {
    "fan_out_structured": STRUCTURING_MODEL,
    "fan_out_creative": CREATIVE_MODEL,
    "coherence": ELICITATION_MODEL,
}


def get_model(role: str) -> str:
    """Look up a model name by its pipeline role.

    Args:
        role: One of 'fan_out_structured', 'fan_out_creative', 'coherence'.

    Returns:
        The Ollama model identifier string.

    Raises:
        KeyError: If the role is not in the registry.
    """
    return MODEL_REGISTRY[role]


@dataclass
class FanOutSpec:
    """Specification for a single fan-out LLM call.

    Attributes:
        stage: Pipeline stage name (places, orgs, substrate, characters-mundane,
            characters-significant).
        index: Zero-based index within the stage's entity list.
        template_name: Filename of the prompt template under prompts/tome/decomposed/.
        model_role: Key into MODEL_REGISTRY.
        context: Dict of placeholder values for the prompt template.
    """

    stage: str
    index: int
    template_name: str
    model_role: str
    context: dict[str, Any] = field(default_factory=dict)

    @property
    def output_filename(self) -> str:
        """Zero-padded instance filename (1-indexed)."""
        return f"instance-{self.index + 1:03d}.json"
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_models.py -v`
Expected: All 7 tests PASS

- [ ] **Step 6: Lint**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/models.py tests/tome/test_models.py && uv run ruff format --check src/narrative_data/tome/models.py tests/tome/test_models.py`
Expected: Clean

- [ ] **Step 7: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/models.py \
        tools/narrative-data/tests/tome/test_models.py \
        tools/narrative-data/src/narrative_data/config.py
git commit -m "feat(tome): add model registry and FanOutSpec dataclass for decomposed pipeline"
```

---

## Task 2: Preamble Compression

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/compress_preamble.py`
- Test: `tools/narrative-data/tests/tome/test_compress_preamble.py`

- [ ] **Step 1: Write the failing test**

Create `tools/narrative-data/tests/tome/test_compress_preamble.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for preamble compression — world-position.json to domain-grouped summary."""

import json
from pathlib import Path

import pytest


@pytest.fixture()
def domains_dir(tmp_path: Path) -> Path:
    """Create a minimal domains directory with two domains."""
    domains = tmp_path / "narrative-data" / "tome" / "domains"
    domains.mkdir(parents=True)

    material = {
        "domain": {"slug": "material-conditions", "name": "Material Conditions"},
        "axes": [
            {"slug": "geography-climate"},
            {"slug": "resource-profile"},
            {"slug": "disease-ecology"},
        ],
    }
    (domains / "material-conditions.json").write_text(json.dumps(material))

    social = {
        "domain": {"slug": "social-forms", "name": "Social Forms"},
        "axes": [
            {"slug": "kinship-system"},
            {"slug": "community-cohesion"},
        ],
    }
    (domains / "social-forms.json").write_text(json.dumps(social))

    return tmp_path


@pytest.fixture()
def world_pos() -> dict:
    return {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "positions": [
            {
                "axis_slug": "geography-climate",
                "value": "temperate-maritime",
                "source": "seed",
                "confidence": 1.0,
            },
            {
                "axis_slug": "resource-profile",
                "value": "soil-fertility:abundant,potable-water:scarce",
                "source": "seed",
                "confidence": 1.0,
            },
            {
                "axis_slug": "disease-ecology",
                "value": "Endemic",
                "source": "inferred",
                "confidence": 1.0,
                "justification": "geography-climate →produces→ disease-ecology (w=0.8); resource-profile →enables→ disease-ecology (w=0.5)",
            },
            {
                "axis_slug": "kinship-system",
                "value": "clan-tribal",
                "source": "seed",
                "confidence": 1.0,
            },
            {
                "axis_slug": "community-cohesion",
                "value": "high",
                "source": "inferred",
                "confidence": 0.8,
                "justification": "kinship-system →produces→ community-cohesion (w=0.7)",
            },
        ],
    }


class TestBuildDomainIndex:
    def test_maps_axes_to_domains(self, domains_dir: Path) -> None:
        from narrative_data.tome.compress_preamble import build_domain_index

        index = build_domain_index(domains_dir)
        assert index["geography-climate"] == "Material Conditions"
        assert index["kinship-system"] == "Social Forms"
        assert index["disease-ecology"] == "Material Conditions"

    def test_returns_empty_for_missing_dir(self, tmp_path: Path) -> None:
        from narrative_data.tome.compress_preamble import build_domain_index

        index = build_domain_index(tmp_path / "nonexistent")
        assert index == {}


class TestCompressPreamble:
    def test_groups_by_domain(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble

        result = compress_preamble(world_pos, domains_dir)
        assert "### Material Conditions" in result
        assert "### Social Forms" in result

    def test_drops_justifications(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble

        result = compress_preamble(world_pos, domains_dir)
        assert "→produces→" not in result
        assert "→enables→" not in result
        assert "(w=" not in result

    def test_labels_seeds(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble

        result = compress_preamble(world_pos, domains_dir)
        assert "geography-climate: temperate-maritime [seed]" in result
        assert "disease-ecology: Endemic" in result
        # Inferred positions should NOT have [seed] label
        assert "disease-ecology: Endemic [seed]" not in result

    def test_preserves_all_axis_values(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble

        result = compress_preamble(world_pos, domains_dir)
        assert "temperate-maritime" in result
        assert "soil-fertility:abundant,potable-water:scarce" in result
        assert "clan-tribal" in result
        assert "high" in result
        assert "Endemic" in result

    def test_ungrouped_axes_in_other_section(self, domains_dir: Path) -> None:
        """Axes not found in any domain file go to an 'Other' section."""
        from narrative_data.tome.compress_preamble import compress_preamble

        world_pos = {
            "positions": [
                {"axis_slug": "unknown-axis", "value": "mystery", "source": "inferred", "confidence": 1.0},
            ],
        }
        result = compress_preamble(world_pos, domains_dir)
        assert "### Other" in result
        assert "unknown-axis: mystery" in result


class TestSubsetAxes:
    def test_returns_subset_for_domain_names(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble, subset_axes

        full = compress_preamble(world_pos, domains_dir)
        subset = subset_axes(full, ["Material Conditions"])
        assert "### Material Conditions" in subset
        assert "### Social Forms" not in subset
        assert "geography-climate" in subset

    def test_returns_multiple_domains(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble, subset_axes

        full = compress_preamble(world_pos, domains_dir)
        subset = subset_axes(full, ["Material Conditions", "Social Forms"])
        assert "### Material Conditions" in subset
        assert "### Social Forms" in subset

    def test_returns_empty_for_unknown_domain(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import compress_preamble, subset_axes

        full = compress_preamble(world_pos, domains_dir)
        subset = subset_axes(full, ["Nonexistent Domain"])
        assert subset.strip() == ""


class TestBuildWorldSummary:
    def test_produces_complete_summary(self, domains_dir: Path, world_pos: dict) -> None:
        from narrative_data.tome.compress_preamble import build_world_summary

        summary = build_world_summary(world_pos, domains_dir)
        assert summary["genre_slug"] == "folk-horror"
        assert summary["setting_slug"] == "test-village"
        assert "### Material Conditions" in summary["compressed_preamble"]
        assert isinstance(summary["axis_count"], int)
        assert summary["axis_count"] == 5
        assert summary["seed_count"] == 3
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_compress_preamble.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write the implementation**

Create `tools/narrative-data/src/narrative_data/tome/compress_preamble.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Compress world-position.json into a domain-grouped, values-only summary.

Drops edge-traversal justifications (~36 KB) and groups axes by their Tome
domain (~3-4 KB). Used by the decomposed elicitation pipeline to reduce
prompt sizes by ~90%.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def build_domain_index(domains_dir: Path) -> dict[str, str]:
    """Build a mapping from axis slug to domain display name.

    Args:
        domains_dir: Path to the tome/domains/ directory containing domain JSON files.

    Returns:
        Dict mapping axis_slug → domain name (e.g. "geography-climate" → "Material Conditions").
    """
    if not domains_dir.exists():
        return {}

    index: dict[str, str] = {}
    for f in sorted(domains_dir.glob("*.json")):
        try:
            data = json.loads(f.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        domain_meta = data.get("domain", {})
        domain_name = domain_meta.get("name", f.stem) if isinstance(domain_meta, dict) else f.stem
        for axis in data.get("axes", []):
            slug = axis.get("slug")
            if slug:
                index[slug] = domain_name
    return index


def compress_preamble(world_pos: dict[str, Any], domains_dir: Path) -> str:
    """Compress a world position into a domain-grouped, values-only markdown string.

    Args:
        world_pos: Parsed world-position.json dict.
        domains_dir: Path to the tome/domains/ directory.

    Returns:
        Markdown string with axes grouped by domain, seeds labeled, no justifications.
    """
    index = build_domain_index(domains_dir)
    positions = world_pos.get("positions", [])

    # Group positions by domain name
    groups: dict[str, list[str]] = {}
    for pos in positions:
        slug = pos.get("axis_slug", "unknown")
        value = pos.get("value", "?")
        source = pos.get("source", "inferred")
        domain = index.get(slug, "Other")

        line = f"- {slug}: {value}"
        if source == "seed":
            line += " [seed]"

        groups.setdefault(domain, []).append(line)

    # Render in a stable order: known domains first (sorted), then Other
    lines: list[str] = []
    known = sorted(k for k in groups if k != "Other")
    for domain in known:
        lines.append(f"### {domain}")
        lines.extend(groups[domain])
        lines.append("")

    if "Other" in groups:
        lines.append("### Other")
        lines.extend(groups["Other"])
        lines.append("")

    return "\n".join(lines).rstrip()


def subset_axes(compressed_preamble: str, domain_names: list[str]) -> str:
    """Extract specific domain sections from a compressed preamble.

    Args:
        compressed_preamble: Full compressed preamble from compress_preamble().
        domain_names: List of domain display names to include.

    Returns:
        Markdown string containing only the requested domain sections.
    """
    sections = compressed_preamble.split("### ")
    result_lines: list[str] = []
    for section in sections:
        if not section.strip():
            continue
        header = section.split("\n", 1)[0].strip()
        if header in domain_names:
            result_lines.append(f"### {section.rstrip()}")
            result_lines.append("")

    return "\n".join(result_lines).rstrip()


def build_world_summary(
    world_pos: dict[str, Any], domains_dir: Path
) -> dict[str, Any]:
    """Build a complete world summary dict for the decomposed pipeline.

    Args:
        world_pos: Parsed world-position.json dict.
        domains_dir: Path to the tome/domains/ directory.

    Returns:
        Dict with genre_slug, setting_slug, compressed_preamble, axis_count, seed_count.
    """
    positions = world_pos.get("positions", [])
    seed_count = sum(1 for p in positions if p.get("source") == "seed")

    return {
        "genre_slug": world_pos.get("genre_slug", "unknown"),
        "setting_slug": world_pos.get("setting_slug", "unknown"),
        "compressed_preamble": compress_preamble(world_pos, domains_dir),
        "axis_count": len(positions),
        "seed_count": seed_count,
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_compress_preamble.py -v`
Expected: All 11 tests PASS

- [ ] **Step 5: Lint**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/compress_preamble.py tests/tome/test_compress_preamble.py && uv run ruff format --check src/narrative_data/tome/compress_preamble.py tests/tome/test_compress_preamble.py`
Expected: Clean

- [ ] **Step 6: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/compress_preamble.py \
        tools/narrative-data/tests/tome/test_compress_preamble.py
git commit -m "feat(tome): add preamble compression — domain-grouped axes without edge traces"
```

---

## Task 3: Entity Planning Call

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/plan_entities.py`
- Create: `tools/narrative-data/prompts/tome/decomposed/entity-plan.md`
- Test: `tools/narrative-data/tests/tome/test_plan_entities.py`

- [ ] **Step 1: Write the prompt template**

Create `tools/narrative-data/prompts/tome/decomposed/entity-plan.md`:

```markdown
You are a world-building planner. Given a world's axis positions and genre profile,
determine how many entities of each type this world needs and how they should be distributed.

The world's material conditions, social structures, and genre constraints should drive the
distribution. A resource-scarce mountainous world needs more infrastructure places and fewer
gathering places than a fertile village. A world with high institutional density needs more
organizations than one with low enforcement capacity.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Genre Profile

{genre_profile_summary}

## Task

Output a JSON object with entity counts and distribution. Use these ranges:

- places: 8-15 total, distributed across types (infrastructure, gathering-place, production-site, settlement, landmark, threshold)
- organizations: 3-8 total
- clusters: 3-6 total (social substrate groups)
- characters_mundane: q1_count (4-8 background) + q2_count (3-5 community)
- characters_significant: q3_count (2-4 tension-bearing) + q4_count (1-2 scene-driving)

For places, provide a distribution object mapping place_type to count. The types should
reflect what the material conditions and genre require — not every type needs to appear.

For clusters, provide a basis hint: one of "blood", "occupation", "belief", "geography",
"affiliation". The kinship-system axis should drive this choice:
- clan-tribal or lineage-hereditary → blood
- guild-professional → occupation
- theocratic or syncretic-plural → belief
- chosen-elective or network-loose → affiliation
- Other patterns → geography

Output valid JSON only. No commentary.

```json
{
  "places": {
    "count": 12,
    "distribution": {
      "infrastructure": 3,
      "gathering-place": 4,
      "production-site": 3,
      "settlement": 2
    }
  },
  "organizations": {
    "count": 5
  },
  "clusters": {
    "count": 4,
    "basis": "blood"
  },
  "characters_mundane": {
    "q1_count": 6,
    "q2_count": 4
  },
  "characters_significant": {
    "q3_count": 3,
    "q4_count": 2
  }
}
```
```

- [ ] **Step 2: Write the failing test**

Create `tools/narrative-data/tests/tome/test_plan_entities.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for entity planning call."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest


SAMPLE_PLAN = {
    "places": {
        "count": 10,
        "distribution": {"infrastructure": 3, "gathering-place": 3, "production-site": 2, "settlement": 2},
    },
    "organizations": {"count": 5},
    "clusters": {"count": 4, "basis": "blood"},
    "characters_mundane": {"q1_count": 6, "q2_count": 4},
    "characters_significant": {"q3_count": 3, "q4_count": 2},
}


@pytest.fixture()
def template_dir(tmp_path: Path) -> Path:
    """Create a minimal prompt template directory."""
    d = tmp_path / "prompts" / "tome" / "decomposed"
    d.mkdir(parents=True)
    (d / "entity-plan.md").write_text(
        "Genre: {genre_slug}\nSetting: {setting_slug}\n"
        "{compressed_preamble}\n{genre_profile_summary}\n"
    )
    return d


class TestBuildPlanPrompt:
    def test_substitutes_placeholders(self, template_dir: Path) -> None:
        from narrative_data.tome.plan_entities import _build_plan_prompt

        world_summary = {
            "genre_slug": "folk-horror",
            "setting_slug": "test-village",
            "compressed_preamble": "### Material Conditions\n- geography-climate: temperate",
        }
        genre_profile_summary = "**Tonal Register:** dread"

        prompt = _build_plan_prompt(
            template_dir / "entity-plan.md", world_summary, genre_profile_summary
        )
        assert "folk-horror" in prompt
        assert "test-village" in prompt
        assert "geography-climate: temperate" in prompt
        assert "dread" in prompt


class TestParsePlanResponse:
    def test_parses_valid_json(self) -> None:
        from narrative_data.tome.plan_entities import _parse_plan_response

        result = _parse_plan_response(json.dumps(SAMPLE_PLAN))
        assert result["places"]["count"] == 10
        assert result["clusters"]["basis"] == "blood"

    def test_parses_json_in_code_fence(self) -> None:
        from narrative_data.tome.plan_entities import _parse_plan_response

        result = _parse_plan_response(f"```json\n{json.dumps(SAMPLE_PLAN)}\n```")
        assert result["places"]["count"] == 10

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.plan_entities import _parse_plan_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_plan_response("this is not json at all")


class TestValidatePlan:
    def test_valid_plan_passes(self) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        _validate_plan(SAMPLE_PLAN)  # should not raise

    def test_missing_places_raises(self) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        with pytest.raises(ValueError, match="places"):
            _validate_plan({"organizations": {"count": 5}})

    def test_distribution_sum_mismatch_raises(self) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        bad = {
            **SAMPLE_PLAN,
            "places": {"count": 10, "distribution": {"infrastructure": 1}},
        }
        with pytest.raises(ValueError, match="distribution.*count"):
            _validate_plan(bad)
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_plan_entities.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 4: Write the implementation**

Create `tools/narrative-data/src/narrative_data/tome/plan_entities.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Entity planning call — determines entity counts and distribution for a world.

A single small-model call reads the compressed world summary and outputs an entity
plan that drives the fan-out phase of the decomposed pipeline.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.ollama import OllamaClient
from narrative_data.tome.models import get_model

_PLAN_TIMEOUT = 120.0
_PLAN_TEMPERATURE = 0.3

_REQUIRED_KEYS = ["places", "organizations", "clusters", "characters_mundane", "characters_significant"]


def _build_plan_prompt(
    template_path: Path,
    world_summary: dict[str, Any],
    genre_profile_summary: str,
) -> str:
    """Substitute placeholders into the entity-plan template.

    Args:
        template_path: Path to entity-plan.md.
        world_summary: Dict from build_world_summary().
        genre_profile_summary: Formatted genre profile string.

    Returns:
        Fully substituted prompt string.
    """
    template = template_path.read_text()
    return (
        template.replace("{genre_slug}", world_summary.get("genre_slug", "unknown"))
        .replace("{setting_slug}", world_summary.get("setting_slug", "unknown"))
        .replace("{compressed_preamble}", world_summary.get("compressed_preamble", ""))
        .replace("{genre_profile_summary}", genre_profile_summary)
    )


def _parse_plan_response(response: str) -> dict[str, Any]:
    """Parse LLM response as a JSON entity plan.

    Args:
        response: Raw LLM response text.

    Returns:
        Parsed plan dict.

    Raises:
        ValueError: If parsing fails.
    """
    text = response.strip()

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if isinstance(result, dict):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from code fence
    fence_match = re.search(r"```(?:json)?\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 3: find outermost { ... }
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(
        f"Could not parse entity plan response as JSON. Response began with: {text[:200]!r}"
    )


def _validate_plan(plan: dict[str, Any]) -> None:
    """Validate that the plan has required keys and consistent counts.

    Args:
        plan: Parsed entity plan dict.

    Raises:
        ValueError: If required keys are missing or distribution doesn't sum to count.
    """
    for key in _REQUIRED_KEYS:
        if key not in plan:
            raise ValueError(f"Entity plan missing required key: '{key}'")

    places = plan["places"]
    if "distribution" in places and "count" in places:
        dist_sum = sum(places["distribution"].values())
        if dist_sum != places["count"]:
            raise ValueError(
                f"Place distribution sum ({dist_sum}) doesn't match count ({places['count']})"
            )


def plan_entities(
    client: OllamaClient,
    template_path: Path,
    world_summary: dict[str, Any],
    genre_profile_summary: str,
) -> dict[str, Any]:
    """Run the entity planning call and return a validated plan.

    Args:
        client: Ollama client instance.
        template_path: Path to entity-plan.md.
        world_summary: Dict from build_world_summary().
        genre_profile_summary: Formatted genre profile string.

    Returns:
        Validated entity plan dict.
    """
    prompt = _build_plan_prompt(template_path, world_summary, genre_profile_summary)
    model = get_model("fan_out_structured")
    response = client.generate(
        model=model,
        prompt=prompt,
        timeout=_PLAN_TIMEOUT,
        temperature=_PLAN_TEMPERATURE,
    )
    plan = _parse_plan_response(response)
    _validate_plan(plan)
    return plan
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_plan_entities.py -v`
Expected: All 7 tests PASS

- [ ] **Step 6: Lint**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/plan_entities.py tests/tome/test_plan_entities.py && uv run ruff format --check src/narrative_data/tome/plan_entities.py tests/tome/test_plan_entities.py`
Expected: Clean

- [ ] **Step 7: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/plan_entities.py \
        tools/narrative-data/tests/tome/test_plan_entities.py \
        tools/narrative-data/prompts/tome/decomposed/entity-plan.md
git commit -m "feat(tome): add entity planning call — world-driven entity count distribution"
```

---

## Task 4: Fan-Out Dispatch Engine

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/fan_out.py`
- Test: `tools/narrative-data/tests/tome/test_fan_out.py`

- [ ] **Step 1: Write the failing test**

Create `tools/narrative-data/tests/tome/test_fan_out.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for fan-out dispatch engine."""

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from narrative_data.tome.models import FanOutSpec


def _make_spec(stage: str = "places", index: int = 0, context: dict | None = None) -> FanOutSpec:
    return FanOutSpec(
        stage=stage,
        index=index,
        template_name="place-fanout.md",
        model_role="fan_out_structured",
        context=context or {"axes": "test"},
    )


@pytest.fixture()
def template_dir(tmp_path: Path) -> Path:
    d = tmp_path / "prompts" / "tome" / "decomposed"
    d.mkdir(parents=True)
    (d / "place-fanout.md").write_text("Generate a {place_type} place.\nAxes: {axes}")
    return d


@pytest.fixture()
def output_dir(tmp_path: Path) -> Path:
    d = tmp_path / "decomposed" / "fan-out" / "places"
    d.mkdir(parents=True)
    return tmp_path / "decomposed"


class TestBuildFanOutPrompt:
    def test_substitutes_context_keys(self, template_dir: Path) -> None:
        from narrative_data.tome.fan_out import _build_fan_out_prompt

        spec = _make_spec(context={"place_type": "infrastructure", "axes": "geo: mountain"})
        prompt = _build_fan_out_prompt(template_dir, spec)
        assert "infrastructure" in prompt
        assert "geo: mountain" in prompt


class TestParseFanOutResponse:
    def test_parses_json_object(self) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        obj = {"slug": "the-well", "name": "The Well", "place_type": "infrastructure"}
        result = _parse_fan_out_response(json.dumps(obj))
        assert result["slug"] == "the-well"

    def test_parses_json_in_fence(self) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        obj = {"slug": "the-well"}
        result = _parse_fan_out_response(f"```json\n{json.dumps(obj)}\n```")
        assert result["slug"] == "the-well"

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_fan_out_response("not json")


class TestGenerateOne:
    def test_calls_client_and_returns_parsed(self, template_dir: Path) -> None:
        from narrative_data.tome.fan_out import _generate_one

        client = MagicMock()
        client.generate.return_value = json.dumps({"slug": "the-well", "name": "The Well"})

        spec = _make_spec(context={"place_type": "infra", "axes": "test"})
        result = _generate_one(client, template_dir, spec)
        assert result["slug"] == "the-well"
        client.generate.assert_called_once()

    def test_retries_on_parse_failure(self, template_dir: Path) -> None:
        from narrative_data.tome.fan_out import _generate_one

        client = MagicMock()
        client.generate.side_effect = [
            "bad output {{{",
            json.dumps({"slug": "the-well"}),
        ]

        spec = _make_spec(context={"place_type": "infra", "axes": "test"})
        result = _generate_one(client, template_dir, spec)
        assert result["slug"] == "the-well"
        assert client.generate.call_count == 2


class TestFanOut:
    def test_dispatches_all_specs_and_returns_results(self, template_dir: Path) -> None:
        from narrative_data.tome.fan_out import fan_out

        client = MagicMock()
        client.generate.return_value = json.dumps({"slug": "place-x"})

        specs = [_make_spec(index=i, context={"place_type": "test", "axes": "a"}) for i in range(3)]
        results = fan_out(client, template_dir, specs)
        assert len(results) == 3
        assert all(r["slug"] == "place-x" for r in results)

    def test_skips_failed_instances(self, template_dir: Path) -> None:
        from narrative_data.tome.fan_out import fan_out

        client = MagicMock()
        # All calls return garbage — should skip all
        client.generate.return_value = "not json at all"

        specs = [_make_spec(index=0, context={"place_type": "test", "axes": "a"})]
        results = fan_out(client, template_dir, specs)
        assert len(results) == 0


class TestSaveInstances:
    def test_writes_individual_files(self, output_dir: Path) -> None:
        from narrative_data.tome.fan_out import save_instances

        specs = [_make_spec(index=0), _make_spec(index=1)]
        results = [{"slug": "a"}, {"slug": "b"}]
        save_instances(output_dir, "places", specs, results)

        f1 = output_dir / "fan-out" / "places" / "instance-001.json"
        f2 = output_dir / "fan-out" / "places" / "instance-002.json"
        assert f1.exists()
        assert f2.exists()
        assert json.loads(f1.read_text())["slug"] == "a"


class TestAggregate:
    def test_writes_draft_file(self, output_dir: Path) -> None:
        from narrative_data.tome.fan_out import aggregate

        results = [{"slug": "a"}, {"slug": "b"}]
        aggregate(output_dir, "places", results)

        draft = output_dir / "places-draft.json"
        assert draft.exists()
        data = json.loads(draft.read_text())
        assert len(data) == 2
        assert data[0]["slug"] == "a"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_fan_out.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write the implementation**

Create `tools/narrative-data/src/narrative_data/tome/fan_out.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Fan-out dispatch engine — parallel single-entity LLM generation.

Each FanOutSpec produces one entity via one LLM call. Calls are dispatched
concurrently via ThreadPoolExecutor. Failed instances are logged and skipped;
the coherence pass works with N-1 entities.
"""

from __future__ import annotations

import json
import re
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any

from narrative_data.config import STRUCTURING_TIMEOUT
from narrative_data.ollama import OllamaClient
from narrative_data.tome.models import FanOutSpec, get_model

_MAX_WORKERS = 4
_FAN_OUT_TEMPERATURE = 0.5


def _build_fan_out_prompt(template_dir: Path, spec: FanOutSpec) -> str:
    """Load and substitute the prompt template for a fan-out spec.

    Args:
        template_dir: Path to prompts/tome/decomposed/ directory.
        spec: The FanOutSpec with template_name and context.

    Returns:
        Fully substituted prompt string.
    """
    template = (template_dir / spec.template_name).read_text()
    result = template
    for key, value in spec.context.items():
        result = result.replace(f"{{{key}}}", str(value))
    return result


def _parse_fan_out_response(response: str) -> dict[str, Any]:
    """Parse LLM response as a single JSON object.

    Args:
        response: Raw LLM response text.

    Returns:
        Parsed entity dict.

    Raises:
        ValueError: If parsing fails.
    """
    text = response.strip()

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if isinstance(result, dict):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from code fence
    fence_match = re.search(r"```(?:json)?\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 3: find outermost { ... }
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(
        f"Could not parse fan-out response as JSON object. Response began with: {text[:200]!r}"
    )


def _generate_one(
    client: OllamaClient,
    template_dir: Path,
    spec: FanOutSpec,
) -> dict[str, Any]:
    """Generate a single entity, with one retry on parse failure.

    Args:
        client: Ollama client instance.
        template_dir: Path to prompts/tome/decomposed/.
        spec: The FanOutSpec to execute.

    Returns:
        Parsed entity dict.

    Raises:
        ValueError: If both attempts fail to parse.
    """
    model = get_model(spec.model_role)
    prompt = _build_fan_out_prompt(template_dir, spec)

    for attempt in range(2):
        response = client.generate(
            model=model,
            prompt=prompt if attempt == 0 else prompt + "\n\nOutput valid JSON only.",
            timeout=STRUCTURING_TIMEOUT,
            temperature=_FAN_OUT_TEMPERATURE,
        )
        try:
            return _parse_fan_out_response(response)
        except ValueError:
            if attempt == 1:
                raise
    raise RuntimeError("unreachable")


def fan_out(
    client: OllamaClient,
    template_dir: Path,
    specs: list[FanOutSpec],
) -> list[dict[str, Any]]:
    """Dispatch all fan-out specs concurrently and collect results.

    Failed instances are logged and skipped.

    Args:
        client: Ollama client instance.
        template_dir: Path to prompts/tome/decomposed/.
        specs: List of FanOutSpec objects to execute.

    Returns:
        List of successfully parsed entity dicts (may be shorter than specs).
    """
    results: list[tuple[int, dict[str, Any]]] = []

    with ThreadPoolExecutor(max_workers=_MAX_WORKERS) as executor:
        futures = {
            executor.submit(_generate_one, client, template_dir, spec): spec
            for spec in specs
        }
        for future in as_completed(futures):
            spec = futures[future]
            try:
                result = future.result()
                results.append((spec.index, result))
            except (ValueError, Exception):  # noqa: BLE001
                # Log and skip — coherence works with N-1
                pass

    # Sort by index to maintain deterministic order
    results.sort(key=lambda x: x[0])
    return [r for _, r in results]


def save_instances(
    decomposed_dir: Path,
    stage: str,
    specs: list[FanOutSpec],
    results: list[dict[str, Any]],
) -> None:
    """Save individual fan-out results to instance files.

    Args:
        decomposed_dir: Path to the decomposed/ output directory.
        stage: Stage name (places, orgs, substrate, etc.).
        specs: The specs that produced results (for filename generation).
        results: Parsed entity dicts, aligned with specs.
    """
    stage_dir = decomposed_dir / "fan-out" / stage
    stage_dir.mkdir(parents=True, exist_ok=True)

    for spec, result in zip(specs, results):
        path = stage_dir / spec.output_filename
        path.write_text(json.dumps(result, indent=2))


def aggregate(
    decomposed_dir: Path,
    stage: str,
    results: list[dict[str, Any]],
) -> None:
    """Write aggregated draft file from fan-out results.

    Args:
        decomposed_dir: Path to the decomposed/ output directory.
        stage: Stage name.
        results: List of entity dicts.
    """
    draft_path = decomposed_dir / f"{stage}-draft.json"
    draft_path.write_text(json.dumps(results, indent=2))
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_fan_out.py -v`
Expected: All 10 tests PASS

- [ ] **Step 5: Lint**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/fan_out.py tests/tome/test_fan_out.py && uv run ruff format --check src/narrative_data/tome/fan_out.py tests/tome/test_fan_out.py`
Expected: Clean

- [ ] **Step 6: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/fan_out.py \
        tools/narrative-data/tests/tome/test_fan_out.py
git commit -m "feat(tome): add fan-out dispatch engine — parallel single-entity generation"
```

---

## Task 5: Coherence Engine

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/cohere.py`
- Test: `tools/narrative-data/tests/tome/test_cohere.py`

- [ ] **Step 1: Write the failing test**

Create `tools/narrative-data/tests/tome/test_cohere.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for coherence engine — 35b relational binding calls."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest


@pytest.fixture()
def template_dir(tmp_path: Path) -> Path:
    d = tmp_path / "prompts" / "tome" / "decomposed"
    d.mkdir(parents=True)
    (d / "places-coherence.md").write_text(
        "World: {genre_slug} / {setting_slug}\n"
        "{compressed_preamble}\n"
        "{draft_entities}\n"
        "{upstream_context}\n"
    )
    (d / "characters-significant-coherence.md").write_text(
        "World: {genre_slug} / {setting_slug}\n"
        "{compressed_preamble}\n"
        "{draft_entities}\n"
        "{upstream_context}\n"
        "{archetypes_context}\n"
        "{archetype_dynamics_context}\n"
        "{design_principle}\n"
    )
    return d


@pytest.fixture()
def decomposed_dir(tmp_path: Path) -> Path:
    d = tmp_path / "decomposed"
    d.mkdir()
    return d


class TestBuildCoherencePrompt:
    def test_substitutes_all_placeholders(self, template_dir: Path) -> None:
        from narrative_data.tome.cohere import _build_coherence_prompt

        prompt = _build_coherence_prompt(
            template_path=template_dir / "places-coherence.md",
            world_summary={"genre_slug": "folk-horror", "setting_slug": "test", "compressed_preamble": "axes here"},
            draft_entities=[{"slug": "the-well"}, {"slug": "the-hall"}],
            upstream_context="Places: the-well, the-hall",
        )
        assert "folk-horror" in prompt
        assert "axes here" in prompt
        assert "the-well" in prompt
        assert "the-hall" in prompt


class TestParseCoherenceResponse:
    def test_parses_json_array(self) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        entities = [{"slug": "a"}, {"slug": "b"}]
        result = _parse_coherence_response(json.dumps(entities))
        assert len(result) == 2

    def test_parses_from_code_fence(self) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        entities = [{"slug": "a"}]
        result = _parse_coherence_response(f"```json\n{json.dumps(entities)}\n```")
        assert len(result) == 1

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_coherence_response("nope")


class TestCohere:
    def test_calls_35b_and_returns_parsed(self, template_dir: Path) -> None:
        from narrative_data.tome.cohere import cohere

        client = MagicMock()
        entities = [{"slug": "the-well", "name": "The Well"}]
        client.generate.return_value = json.dumps(entities)

        result = cohere(
            client=client,
            template_path=template_dir / "places-coherence.md",
            world_summary={"genre_slug": "folk-horror", "setting_slug": "t", "compressed_preamble": "a"},
            draft_entities=[{"slug": "the-well"}],
            upstream_context="",
        )
        assert len(result) == 1
        assert result[0]["slug"] == "the-well"
        # Verify it used the coherence model
        call_kwargs = client.generate.call_args
        assert call_kwargs[1]["model"] == "qwen3.5:35b" or call_kwargs[0][0] == "qwen3.5:35b"


class TestSaveCoherenceOutput:
    def test_writes_final_file_with_metadata(self, decomposed_dir: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        entities = [{"slug": "the-well"}, {"slug": "the-hall"}]
        save_coherence_output(
            decomposed_dir=decomposed_dir,
            stage="places",
            entities=entities,
            world_slug="test-world",
            genre_slug="folk-horror",
            setting_slug="test-village",
        )

        path = decomposed_dir / "places.json"
        assert path.exists()
        data = json.loads(path.read_text())
        assert data["world_slug"] == "test-world"
        assert data["genre_slug"] == "folk-horror"
        assert len(data["places"]) == 2

    def test_writes_characters_with_correct_key(self, decomposed_dir: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        chars = [{"slug": "mara", "centrality": "Q3"}]
        save_coherence_output(
            decomposed_dir=decomposed_dir,
            stage="characters-significant",
            entities=chars,
            world_slug="test-world",
            genre_slug="folk-horror",
            setting_slug="test-village",
        )

        path = decomposed_dir / "characters-significant.json"
        data = json.loads(path.read_text())
        assert "characters" in data
        assert data["characters"][0]["slug"] == "mara"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_cohere.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write the implementation**

Create `tools/narrative-data/src/narrative_data/tome/cohere.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Coherence engine — 35b relational binding calls per entity stage.

Takes aggregated fan-out drafts and upstream context, prompts the coherence
model (35b) to bind entities relationally, and writes the final per-stage
output files.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_TIMEOUT
from narrative_data.ollama import OllamaClient
from narrative_data.tome.models import get_model
from narrative_data.utils import now_iso

_COHERENCE_TEMPERATURE = 0.5

# Maps stage names to the JSON key used for the entity array in output files
_STAGE_ENTITY_KEYS: dict[str, str] = {
    "places": "places",
    "orgs": "organizations",
    "substrate": "clusters",
    "characters-mundane": "characters",
    "characters-significant": "characters",
}

# Maps stage names to the output filename
_STAGE_OUTPUT_FILES: dict[str, str] = {
    "places": "places.json",
    "orgs": "organizations.json",
    "substrate": "social-substrate.json",
    "characters-mundane": "characters-mundane.json",
    "characters-significant": "characters-significant.json",
}


def _build_coherence_prompt(
    template_path: Path,
    world_summary: dict[str, Any],
    draft_entities: list[dict[str, Any]],
    upstream_context: str,
    extra_context: dict[str, str] | None = None,
) -> str:
    """Substitute placeholders into a coherence template.

    Args:
        template_path: Path to the coherence prompt template.
        world_summary: Dict from build_world_summary().
        draft_entities: List of entity dicts from the fan-out phase.
        upstream_context: Formatted summary of upstream stage outputs.
        extra_context: Additional placeholder substitutions (e.g., archetypes_context).

    Returns:
        Fully substituted prompt string.
    """
    template = template_path.read_text()
    draft_json = json.dumps(draft_entities, indent=2)

    result = (
        template.replace("{genre_slug}", world_summary.get("genre_slug", "unknown"))
        .replace("{setting_slug}", world_summary.get("setting_slug", "unknown"))
        .replace("{compressed_preamble}", world_summary.get("compressed_preamble", ""))
        .replace("{draft_entities}", draft_json)
        .replace("{upstream_context}", upstream_context)
    )

    if extra_context:
        for key, value in extra_context.items():
            result = result.replace(f"{{{key}}}", value)

    return result


def _parse_coherence_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of entities.

    Args:
        response: Raw LLM response text.

    Returns:
        List of entity dicts.

    Raises:
        ValueError: If parsing fails.
    """
    text = response.strip()

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if isinstance(result, list):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from code fence
    fence_match = re.search(r"```(?:json)?\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 3: find outermost [ ... ]
    start = text.find("[")
    end = text.rfind("]")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(
        f"Could not parse coherence response as JSON array. Response began with: {text[:200]!r}"
    )


def cohere(
    client: OllamaClient,
    template_path: Path,
    world_summary: dict[str, Any],
    draft_entities: list[dict[str, Any]],
    upstream_context: str,
    extra_context: dict[str, str] | None = None,
) -> list[dict[str, Any]]:
    """Run a coherence call — 35b relational binding for one entity stage.

    Args:
        client: Ollama client instance.
        template_path: Path to the coherence prompt template.
        world_summary: Dict from build_world_summary().
        draft_entities: List of entity dicts from fan-out.
        upstream_context: Formatted summary of upstream stages.
        extra_context: Additional placeholder substitutions.

    Returns:
        List of coherence-enhanced entity dicts.
    """
    prompt = _build_coherence_prompt(
        template_path, world_summary, draft_entities, upstream_context, extra_context
    )
    model = get_model("coherence")
    response = client.generate(
        model=model,
        prompt=prompt,
        timeout=ELICITATION_TIMEOUT,
        temperature=_COHERENCE_TEMPERATURE,
    )
    return _parse_coherence_response(response)


def save_coherence_output(
    decomposed_dir: Path,
    stage: str,
    entities: list[dict[str, Any]],
    world_slug: str,
    genre_slug: str,
    setting_slug: str,
) -> Path:
    """Write the coherence output as a final per-stage JSON file.

    The output schema matches the current pipeline format for backward compatibility.

    Args:
        decomposed_dir: Path to the decomposed/ output directory.
        stage: Stage name.
        entities: List of coherence-enhanced entity dicts.
        world_slug: World identifier.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.

    Returns:
        Path to the written file.
    """
    entity_key = _STAGE_ENTITY_KEYS.get(stage, stage)
    output_filename = _STAGE_OUTPUT_FILES.get(stage, f"{stage}.json")

    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "pipeline": "decomposed",
        f"{entity_key.rstrip('s')}_count" if entity_key != "characters" else "character_count": len(entities),
        entity_key: entities,
    }

    path = decomposed_dir / output_filename
    path.write_text(json.dumps(output, indent=2))
    return path
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_cohere.py -v`
Expected: All 8 tests PASS

- [ ] **Step 5: Lint and fix**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/cohere.py tests/tome/test_cohere.py && uv run ruff format --check src/narrative_data/tome/cohere.py tests/tome/test_cohere.py`
Expected: Clean

- [ ] **Step 6: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/cohere.py \
        tools/narrative-data/tests/tome/test_cohere.py
git commit -m "feat(tome): add coherence engine — 35b relational binding per entity stage"
```

---

## Task 6: Fan-Out Prompt Templates

**Files:**
- Create: `tools/narrative-data/prompts/tome/decomposed/place-fanout.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/org-fanout.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/substrate-fanout.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/character-mundane-fanout.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/character-significant-fanout.md`

These are the small, focused prompts for individual entity generation. Each generates one entity.

- [ ] **Step 1: Create place-fanout.md**

Create `tools/narrative-data/prompts/tome/decomposed/place-fanout.md`:

```markdown
You are generating a single named place for a narrative world.

Genre: {genre_slug}
Setting: {setting_slug}
Place type: {place_type}

## Relevant World Axes

{axes_subset}

## Genre Spatial Function

This place should serve this narrative role: {spatial_function}

## Task

Generate ONE place of type "{place_type}" that is grounded in the material conditions
above. The place must feel like it belongs in this specific world — not a generic version
of its type. Name it something evocative that reflects the world's cultural register.

Output valid JSON only. No commentary.

```json
{
  "slug": "kebab-case-identifier",
  "name": "The Place Name",
  "place_type": "{place_type}",
  "tier": 1,
  "description": "2-3 sentences grounded in specific axis values.",
  "spatial_role": "What narrative function this place serves.",
  "grounding": {
    "material_axes": ["axis-slug:value", "axis-slug:value"]
  }
}
```
```

- [ ] **Step 2: Create org-fanout.md**

Create `tools/narrative-data/prompts/tome/decomposed/org-fanout.md`:

```markdown
You are generating a single organization or institution for a narrative world.

Genre: {genre_slug}
Setting: {setting_slug}

## Relevant World Axes

{axes_subset}

## Existing Places

{places_summary}

## Task

Generate ONE organization that is grounded in the economic, political, and social
conditions above. It should have a clear relationship to at least one existing place.
The organization must feel like a product of this world's material conditions — not
a generic institution.

Output valid JSON only. No commentary.

```json
{
  "slug": "kebab-case-identifier",
  "name": "The Organization Name",
  "org_type": "governance|economic|religious|military|cultural|professional",
  "tier": 1,
  "description": "2-3 sentences grounded in specific axis values.",
  "place_associations": ["place-slug"],
  "grounding": {
    "axes": ["axis-slug:value"]
  }
}
```
```

- [ ] **Step 3: Create substrate-fanout.md**

Create `tools/narrative-data/prompts/tome/decomposed/substrate-fanout.md`:

```markdown
You are generating a single social cluster (kinship group, faction, lineage) for a
narrative world.

Genre: {genre_slug}
Setting: {setting_slug}
Cluster basis: {cluster_basis}

## Relevant World Axes

{axes_subset}

## Existing Places and Organizations

{upstream_summary}

## Task

Generate ONE social cluster based on "{cluster_basis}" bonds. The cluster should be
grounded in specific material conditions and connected to existing places or organizations.
Every cluster carries narrative weight — the productive tension is between clusters, not
within tier categories.

Output valid JSON only. No commentary.

```json
{
  "slug": "kebab-case-identifier",
  "name": "The Cluster Name",
  "basis": "{cluster_basis}",
  "description": "2-3 sentences describing this group's identity and material foundation.",
  "place_associations": ["place-slug"],
  "org_associations": ["org-slug"],
  "grounding": {
    "axes": ["axis-slug:value"]
  }
}
```
```

- [ ] **Step 4: Create character-mundane-fanout.md**

Create `tools/narrative-data/prompts/tome/decomposed/character-mundane-fanout.md`:

```markdown
You are generating a single mundane character for a narrative world. Mundane characters
form the demographic reality of the world — the people who make daily life legible.

Genre: {genre_slug}
Setting: {setting_slug}
Centrality: {centrality}
Cluster: {cluster_name} ({cluster_slug})

## Relevant World Axes

{axes_subset}

## Task

Generate ONE {centrality} character who belongs to the "{cluster_name}" cluster.

If centrality is Q1 (background): minimal characterization — name, role, cluster, brief
description. These are the people you'd see but not notice.

If centrality is Q2 (community): light personality sketch with a distinguishing quirk.
These are the people you'd recognize and greet.

The character must feel like a product of this world's material conditions and their
cluster membership — not a generic archetype.

Output valid JSON only. No commentary.

```json
{
  "centrality": "{centrality}",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their daily function",
  "cluster_membership": "{cluster_slug}",
  "description": "1-3 sentences, grounded in world position.",
  "place_associations": ["place-slug"]
}
```
```

- [ ] **Step 5: Create character-significant-fanout.md**

Create `tools/narrative-data/prompts/tome/decomposed/character-significant-fanout.md`:

```markdown
You are generating a skeleton for a narratively significant character. This skeleton will
be enriched in a subsequent coherence pass — focus on the character's structural position,
not their full relational web.

Genre: {genre_slug}
Setting: {setting_slug}
Centrality: {centrality}
Boundary position: {primary_cluster} / {boundary_cluster}

## Relevant World Axes

{axes_subset}

## Boundary Context

Primary cluster: {primary_cluster_desc}
Boundary cluster: {boundary_cluster_desc}

## Available Archetypes

{archetypes_summary}

## Task

Generate ONE {centrality} character skeleton positioned at the boundary between
"{primary_cluster}" and "{boundary_cluster}". Their archetype should emerge from their
boundary position — it is how they cope with that position, not a personality assigned
from a catalog.

If centrality is Q3 (tension-bearing): lives in the gap between stated and operative
reality. Include archetype mapping and boundary tension.

If centrality is Q4 (scene-driving): the genre expressing itself through a specific
person. Include archetype mapping, boundary tension, and initial personality axis hints.

Output valid JSON only. No commentary.

```json
{
  "centrality": "{centrality}",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their position or function",
  "description": "3-4 sentences. Situated at the boundary.",
  "archetype": {
    "primary": "The Archetype Name",
    "shadow": "The Shadow Archetype Name",
    "genre_inflection": "How this archetype expresses at THIS boundary."
  },
  "cluster_membership": {
    "primary": "{primary_cluster}",
    "boundary_with": "{boundary_cluster}",
    "boundary_tension": "What makes this boundary productive."
  },
  "place_associations": ["place-slug"]
}
```
```

- [ ] **Step 6: Commit all templates**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/prompts/tome/decomposed/place-fanout.md \
        tools/narrative-data/prompts/tome/decomposed/org-fanout.md \
        tools/narrative-data/prompts/tome/decomposed/substrate-fanout.md \
        tools/narrative-data/prompts/tome/decomposed/character-mundane-fanout.md \
        tools/narrative-data/prompts/tome/decomposed/character-significant-fanout.md
git commit -m "feat(tome): add fan-out prompt templates for decomposed entity generation"
```

---

## Task 7: Coherence Prompt Templates

**Files:**
- Create: `tools/narrative-data/prompts/tome/decomposed/places-coherence.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/orgs-coherence.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/substrate-coherence.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/characters-mundane-coherence.md`
- Create: `tools/narrative-data/prompts/tome/decomposed/characters-significant-coherence.md`

- [ ] **Step 1: Create places-coherence.md**

Create `tools/narrative-data/prompts/tome/decomposed/places-coherence.md`:

```markdown
You are reviewing and enriching a set of individually-generated places for a narrative world.
Each place was generated independently — your job is to bind them into a coherent spatial
landscape.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Genre Profile

{genre_profile_summary}

## Draft Places

These places were generated individually. Review and enrich them:

{draft_entities}

## Task

For each place, review and adjust:
1. **Spatial relationships**: How does this place relate to others? Add adjacency, visibility,
   access path hints to the description where relevant.
2. **Grounding review**: Does the description reference specific axis values? Strengthen
   connections to material conditions.
3. **Naming consistency**: Do the names fit the world's cultural register? Adjust if needed.
4. **Narrative function**: Does each place serve a distinct narrative role? Flag or merge
   redundancies.

You may adjust descriptions, names, spatial roles, and grounding. Do not remove places.
Do not add new places.

Output the complete array of enriched place objects as valid JSON. Same schema as the input.
No commentary outside the JSON.
```

- [ ] **Step 2: Create orgs-coherence.md**

Create `tools/narrative-data/prompts/tome/decomposed/orgs-coherence.md`:

```markdown
You are reviewing and enriching a set of individually-generated organizations for a narrative
world. Each organization was generated independently — your job is to bind them into a
coherent institutional landscape.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Existing Places

{upstream_context}

## Draft Organizations

{draft_entities}

## Task

For each organization, review and adjust:
1. **Place binding**: Does each org have a clear relationship to at least one place? Add or
   strengthen place_associations.
2. **Power structure**: How do these organizations relate to each other? Adjust descriptions
   to reflect institutional hierarchy, competition, or cooperation.
3. **Axis grounding**: Are descriptions specific to this world's economic, political, and
   social conditions?
4. **No redundancy**: Do any orgs overlap in function? Differentiate or merge.

Output the complete array of enriched organization objects as valid JSON. Same schema as input.
No commentary outside the JSON.
```

- [ ] **Step 3: Create substrate-coherence.md**

Create `tools/narrative-data/prompts/tome/decomposed/substrate-coherence.md`:

```markdown
You are reviewing and enriching a set of individually-generated social clusters for a
narrative world. Each cluster was generated independently — your job is to bind them into
a coherent social substrate with pairwise relationships.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Existing Places and Organizations

{upstream_context}

## Draft Clusters

{draft_entities}

## Task

1. **Review each cluster**: Adjust descriptions, place/org associations, and grounding for
   consistency with the world's material conditions and social structure.

2. **Generate pairwise relationships**: For each pair of clusters, determine the nature of
   their boundary. Relationships should reflect material tensions — resource competition,
   knowledge asymmetry, institutional allegiance, ritual obligation.

   Use this format for each relationship:
   ```json
   {
     "from": "cluster-a-slug",
     "to": "cluster-b-slug",
     "relationship_type": "tension|cooperation|dependency|rivalry|deference",
     "description": "What connects or divides these groups."
   }
   ```

3. **Boundary richness**: The productive tension in the social substrate lives at boundaries
   between clusters. Significant characters will later be positioned at these boundaries.
   Make the boundaries specific and materially grounded.

Output valid JSON:
```json
{
  "clusters": [...enriched cluster objects...],
  "relationships": [...pairwise relationship objects...]
}
```

No commentary outside the JSON.
```

- [ ] **Step 4: Create characters-mundane-coherence.md**

Create `tools/narrative-data/prompts/tome/decomposed/characters-mundane-coherence.md`:

```markdown
You are reviewing and enriching a set of individually-generated mundane characters (Q1-Q2)
for a narrative world. Each character was generated independently — your job is to ensure
they form a coherent demographic reality.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Social Substrate

{upstream_context}

## Draft Characters

{draft_entities}

## Task

This is an editorial review — adjust, don't reinvent:

1. **Cluster distribution**: Are clusters evenly populated? If all characters ended up in
   one cluster, redistribute some.
2. **Naming consistency**: Do names fit the world's cultural register? A clan-tribal society
   with sacred-ritual gender system should have names that reflect those patterns.
3. **No duplicate roles**: Two grain weighers in the same cluster is redundant. Diversify
   occupational roles within clusters.
4. **Place associations**: Does each character connect to at least one relevant place?
5. **Q1/Q2 distinction**: Q1 characters should be minimal (name, role, cluster). Q2
   characters should have a distinguishing quirk or personality note. Ensure the distinction
   is clear.

Output the complete array of adjusted character objects as valid JSON. Same schema as input.
No commentary outside the JSON.
```

- [ ] **Step 5: Create characters-significant-coherence.md**

Create `tools/narrative-data/prompts/tome/decomposed/characters-significant-coherence.md`:

```markdown
You are performing the deepest coherence pass in the world-building pipeline. These
significant characters (Q3-Q4) are the heart of expressed agency — the locus at which
entanglement drives the rendering of multivalent intention and desire across scene,
arc, and orbital scales.

## Design Principle

Ascending centrality simultaneously increases capacity to act and constraints on action.
The more a character can do, the more the world can do to them. Entanglement is the price
of agency.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{compressed_preamble}

## Genre Profile

{genre_profile_summary}

## Social Substrate

{substrate_context}

## Mundane Characters (Q1-Q2)

{mundane_context}

## Existing Places and Organizations

{upstream_context}

## Bedrock Archetypes

{archetypes_context}

## Archetype Dynamics

{archetype_dynamics_context}

## Draft Significant Character Skeletons

{draft_entities}

## Task

Enrich each character skeleton into a fully realized significant character. This is
generative work, not editorial:

### For ALL Q3-Q4 characters:

1. **Relational seed binding** (verb:slug format): Bind each character to specific mundane
   characters, organizations, clusters, and other significant characters. Use concrete verbs:
   supervises, fears, married-into, protects, controls, answers-to, distrusts, depends-on.
   Q3 characters need 4+ seeds. Q4 characters need 5+ seeds. Each must reference at least
   one Q1-Q2 character slug and one organization or cluster slug.

2. **Stated/operative gap**: Ground in social position. The gap is structural — produced by
   the character's position in the social web, not a personal character flaw. What they claim
   to do versus what the network requires them to actually do.

3. **Place associations**: 1-2 place slugs with role context for Q4 (e.g., "the-hall:authority").

4. **Arc-scale goals**: What changes over the story for this character.

### Additionally for Q4 characters:

5. **Personality profile**: 7-axis numeric (warmth, authority, openness, interiority, stability,
   agency, morality) each 0.0-1.0. Start from the bedrock archetype's profile and adjust for
   this character's specific world-position and social entanglement.

6. **Communicability**: surface_area (0.0-1.0), translation_friction (0.0-1.0),
   timescale (momentary|biographical|generational|geological|primordial),
   atmospheric_palette (sensory string).

7. **Multi-scale goals**: existential (what they'd die for), arc (what changes), scene (what
   they want right now). All three constrained by social position.

### Cross-character verification:

8. Do the significant characters create productive tension with each other, not just with
   the substrate? The relational web should have cross-links between Q3-Q4 characters.

## Output Schema

Output valid JSON: a single array containing ALL significant characters (Q3 first, then Q4).
Use the same schema as the Phase 3b character-significant-elicitation output. No commentary
outside the JSON.
```

- [ ] **Step 6: Commit all coherence templates**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/prompts/tome/decomposed/places-coherence.md \
        tools/narrative-data/prompts/tome/decomposed/orgs-coherence.md \
        tools/narrative-data/prompts/tome/decomposed/substrate-coherence.md \
        tools/narrative-data/prompts/tome/decomposed/characters-mundane-coherence.md \
        tools/narrative-data/prompts/tome/decomposed/characters-significant-coherence.md
git commit -m "feat(tome): add coherence prompt templates for per-stage relational binding"
```

---

## Task 8: Pipeline Orchestrator

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/orchestrate_decomposed.py`
- Test: `tools/narrative-data/tests/tome/test_orchestrate_decomposed.py`

This is the central module that ties all stages together. It's the largest single task
because it coordinates compress_preamble → plan → fan-out → cohere for each stage.

- [ ] **Step 1: Write the failing test**

Create `tools/narrative-data/tests/tome/test_orchestrate_decomposed.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the decomposed pipeline orchestrator."""

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest


@pytest.fixture()
def data_path(tmp_path: Path) -> Path:
    """Create a minimal data directory structure."""
    # Domains
    domains = tmp_path / "narrative-data" / "tome" / "domains"
    domains.mkdir(parents=True)

    material = {
        "domain": {"slug": "material-conditions", "name": "Material Conditions"},
        "axes": [{"slug": "geography-climate"}, {"slug": "resource-profile"}],
    }
    (domains / "material-conditions.json").write_text(json.dumps(material))

    social = {
        "domain": {"slug": "social-forms", "name": "Social Forms"},
        "axes": [{"slug": "kinship-system"}, {"slug": "community-cohesion"}],
    }
    (domains / "social-forms.json").write_text(json.dumps(social))

    # World directory
    world = tmp_path / "narrative-data" / "tome" / "worlds" / "test-world"
    world.mkdir(parents=True)

    world_pos = {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "genre_profile": {"aesthetic": {"sensory_density": {"value": 0.9}}},
        "positions": [
            {"axis_slug": "geography-climate", "value": "temperate", "source": "seed", "confidence": 1.0},
            {"axis_slug": "resource-profile", "value": "moderate", "source": "seed", "confidence": 1.0},
            {"axis_slug": "kinship-system", "value": "clan-tribal", "source": "seed", "confidence": 1.0},
            {"axis_slug": "community-cohesion", "value": "high", "source": "inferred", "confidence": 0.8, "justification": "kinship →produces→ cohesion (0.7)"},
        ],
    }
    (world / "world-position.json").write_text(json.dumps(world_pos))

    # Genre data (empty archetypes and dynamics dirs)
    (tmp_path / "narrative-data" / "discovery" / "archetypes" / "folk-horror").mkdir(parents=True)
    (tmp_path / "narrative-data" / "discovery" / "archetype-dynamics" / "folk-horror").mkdir(parents=True)
    (tmp_path / "narrative-data" / "discovery" / "settings").mkdir(parents=True)

    return tmp_path


class TestBuildWorldSummaryFromPath:
    def test_produces_summary_with_compressed_preamble(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_world_summary_from_path

        summary = _build_world_summary_from_path(data_path, "test-world")
        assert summary["genre_slug"] == "folk-horror"
        assert "Material Conditions" in summary["compressed_preamble"]
        assert "→produces→" not in summary["compressed_preamble"]


class TestBuildFanOutSpecs:
    def test_generates_place_specs_from_plan(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_place_specs

        plan = {
            "places": {
                "count": 3,
                "distribution": {"infrastructure": 1, "gathering-place": 1, "settlement": 1},
            },
        }
        compressed_preamble = "### Material Conditions\n- geo: temperate"
        specs = _build_place_specs(plan, compressed_preamble, "folk-horror", "test")

        assert len(specs) == 3
        assert specs[0].stage == "places"
        assert specs[0].model_role == "fan_out_structured"
        # Each spec should have the place_type in its context
        types = [s.context["place_type"] for s in specs]
        assert "infrastructure" in types
        assert "gathering-place" in types

    def test_generates_character_significant_specs(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_significant_character_specs

        plan = {"characters_significant": {"q3_count": 2, "q4_count": 1}}
        clusters = [
            {"slug": "name-bound", "name": "Name-Bound", "description": "Blood lineage"},
            {"slug": "silt-bound", "name": "Silt-Bound", "description": "Agricultural"},
        ]
        specs = _build_significant_character_specs(
            plan, "axes here", "folk-horror", "test", clusters, "archetype summaries"
        )

        assert len(specs) == 3
        # First 2 are Q3, last is Q4
        assert specs[0].context["centrality"] == "Q3"
        assert specs[2].context["centrality"] == "Q4"
        # Q3-Q4 use the creative model
        assert specs[0].model_role == "fan_out_creative"


class TestDecomposedDirectory:
    def test_creates_expected_structure(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _ensure_decomposed_dir

        decomposed = _ensure_decomposed_dir(data_path, "test-world")
        assert decomposed.exists()
        assert (decomposed / "fan-out").exists()
        expected_stages = ["places", "orgs", "substrate", "characters-mundane", "characters-significant"]
        for stage in expected_stages:
            assert (decomposed / "fan-out" / stage).exists()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_orchestrate_decomposed.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write the implementation**

Create `tools/narrative-data/src/narrative_data/tome/orchestrate_decomposed.py`:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Pipeline orchestrator for the decomposed fan-out/fan-in elicitation pipeline.

Coordinates: compress preamble → plan entities → [fan-out → cohere] per stage → compose.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from narrative_data.ollama import OllamaClient
from narrative_data.tome.cohere import cohere, save_coherence_output
from narrative_data.tome.compress_preamble import (
    build_world_summary,
    subset_axes,
)
from narrative_data.tome.elicit_characters_significant import (
    _build_archetypes_context,
    _build_dynamics_context,
    _load_archetype_dynamics,
    _load_archetypes,
)
from narrative_data.tome.elicit_places import (
    _build_genre_profile_summary,
    _build_settings_context,
    _load_world_position,
)
from narrative_data.tome.fan_out import aggregate, fan_out, save_instances
from narrative_data.tome.models import FanOutSpec
from narrative_data.tome.plan_entities import plan_entities

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_DECOMPOSED_PROMPTS = _PROMPTS_DIR / "tome" / "decomposed"

_STAGES = ["places", "orgs", "substrate", "characters-mundane", "characters-significant"]

# Domain subsets per entity type for fan-out axis injection
_STAGE_DOMAINS: dict[str, list[str]] = {
    "places": ["Material Conditions", "Aesthetic and Cultural Forms"],
    "orgs": ["Economic Forms", "Political Structures", "Social Forms of Production and Reproduction"],
    "substrate": ["Social Forms of Production and Reproduction", "Material Conditions"],
    "characters-mundane": ["Social Forms of Production and Reproduction"],
    "characters-significant": [
        "Social Forms of Production and Reproduction",
        "Material Conditions",
    ],
}


# ---------------------------------------------------------------------------
# Directory setup
# ---------------------------------------------------------------------------


def _ensure_decomposed_dir(data_path: Path, world_slug: str) -> Path:
    """Create the decomposed output directory structure.

    Args:
        data_path: Root of storyteller-data.
        world_slug: World identifier.

    Returns:
        Path to the decomposed/ directory.
    """
    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    decomposed = world_dir / "decomposed"
    decomposed.mkdir(exist_ok=True)
    (decomposed / "fan-out").mkdir(exist_ok=True)
    for stage in _STAGES:
        (decomposed / "fan-out" / stage).mkdir(exist_ok=True)
    return decomposed


# ---------------------------------------------------------------------------
# World summary
# ---------------------------------------------------------------------------


def _build_world_summary_from_path(
    data_path: Path, world_slug: str
) -> dict[str, Any]:
    """Load world position and build compressed summary.

    Args:
        data_path: Root of storyteller-data.
        world_slug: World identifier.

    Returns:
        World summary dict with compressed_preamble.
    """
    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    world_pos = _load_world_position(world_dir)
    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    summary = build_world_summary(world_pos, domains_dir)
    summary["genre_profile"] = world_pos.get("genre_profile")
    return summary


# ---------------------------------------------------------------------------
# Spec builders
# ---------------------------------------------------------------------------


def _build_place_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
) -> list[FanOutSpec]:
    """Build FanOutSpec list for place generation.

    Args:
        plan: Entity plan from planning call.
        compressed_preamble: Full compressed preamble.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.

    Returns:
        List of FanOutSpec, one per place.
    """
    axes_subset = subset_axes(compressed_preamble, _STAGE_DOMAINS["places"])
    distribution = plan["places"].get("distribution", {})

    specs: list[FanOutSpec] = []
    index = 0
    for place_type, count in distribution.items():
        for _ in range(count):
            specs.append(
                FanOutSpec(
                    stage="places",
                    index=index,
                    template_name="place-fanout.md",
                    model_role="fan_out_structured",
                    context={
                        "genre_slug": genre_slug,
                        "setting_slug": setting_slug,
                        "place_type": place_type,
                        "axes_subset": axes_subset,
                        "spatial_function": "",
                    },
                )
            )
            index += 1
    return specs


def _build_org_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    places_summary: str,
) -> list[FanOutSpec]:
    """Build FanOutSpec list for organization generation."""
    axes_subset = subset_axes(compressed_preamble, _STAGE_DOMAINS["orgs"])
    count = plan["organizations"]["count"]

    return [
        FanOutSpec(
            stage="orgs",
            index=i,
            template_name="org-fanout.md",
            model_role="fan_out_structured",
            context={
                "genre_slug": genre_slug,
                "setting_slug": setting_slug,
                "axes_subset": axes_subset,
                "places_summary": places_summary,
            },
        )
        for i in range(count)
    ]


def _build_substrate_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    upstream_summary: str,
) -> list[FanOutSpec]:
    """Build FanOutSpec list for social substrate cluster generation."""
    axes_subset = subset_axes(compressed_preamble, _STAGE_DOMAINS["substrate"])
    cluster_plan = plan["clusters"]
    count = cluster_plan["count"]
    basis = cluster_plan.get("basis", "blood")

    return [
        FanOutSpec(
            stage="substrate",
            index=i,
            template_name="substrate-fanout.md",
            model_role="fan_out_structured",
            context={
                "genre_slug": genre_slug,
                "setting_slug": setting_slug,
                "axes_subset": axes_subset,
                "cluster_basis": basis,
                "upstream_summary": upstream_summary,
            },
        )
        for i in range(count)
    ]


def _build_mundane_character_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    clusters: list[dict[str, Any]],
) -> list[FanOutSpec]:
    """Build FanOutSpec list for mundane character generation."""
    axes_subset = subset_axes(compressed_preamble, _STAGE_DOMAINS["characters-mundane"])
    q1_count = plan["characters_mundane"]["q1_count"]
    q2_count = plan["characters_mundane"]["q2_count"]

    specs: list[FanOutSpec] = []
    index = 0

    for i in range(q1_count):
        cluster = clusters[i % len(clusters)] if clusters else {"slug": "unknown", "name": "Unknown"}
        specs.append(
            FanOutSpec(
                stage="characters-mundane",
                index=index,
                template_name="character-mundane-fanout.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "centrality": "Q1",
                    "cluster_name": cluster.get("name", "Unknown"),
                    "cluster_slug": cluster.get("slug", "unknown"),
                    "axes_subset": axes_subset,
                },
            )
        )
        index += 1

    for i in range(q2_count):
        cluster = clusters[i % len(clusters)] if clusters else {"slug": "unknown", "name": "Unknown"}
        specs.append(
            FanOutSpec(
                stage="characters-mundane",
                index=index,
                template_name="character-mundane-fanout.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "centrality": "Q2",
                    "cluster_name": cluster.get("name", "Unknown"),
                    "cluster_slug": cluster.get("slug", "unknown"),
                    "axes_subset": axes_subset,
                },
            )
        )
        index += 1

    return specs


def _build_significant_character_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    clusters: list[dict[str, Any]],
    archetypes_summary: str,
) -> list[FanOutSpec]:
    """Build FanOutSpec list for significant character skeleton generation."""
    axes_subset = subset_axes(compressed_preamble, _STAGE_DOMAINS["characters-significant"])
    q3_count = plan["characters_significant"]["q3_count"]
    q4_count = plan["characters_significant"]["q4_count"]

    specs: list[FanOutSpec] = []
    index = 0

    # Assign boundary positions by cycling through cluster pairs
    cluster_pairs = []
    if len(clusters) >= 2:
        for i in range(len(clusters)):
            for j in range(i + 1, len(clusters)):
                cluster_pairs.append((clusters[i], clusters[j]))

    for i in range(q3_count):
        pair = cluster_pairs[i % len(cluster_pairs)] if cluster_pairs else (
            {"slug": "a", "name": "A", "description": ""},
            {"slug": "b", "name": "B", "description": ""},
        )
        specs.append(
            FanOutSpec(
                stage="characters-significant",
                index=index,
                template_name="character-significant-fanout.md",
                model_role="fan_out_creative",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "centrality": "Q3",
                    "primary_cluster": pair[0].get("slug", ""),
                    "boundary_cluster": pair[1].get("slug", ""),
                    "primary_cluster_desc": pair[0].get("description", ""),
                    "boundary_cluster_desc": pair[1].get("description", ""),
                    "axes_subset": axes_subset,
                    "archetypes_summary": archetypes_summary,
                },
            )
        )
        index += 1

    for i in range(q4_count):
        pair_idx = (q3_count + i) % len(cluster_pairs) if cluster_pairs else 0
        pair = cluster_pairs[pair_idx] if cluster_pairs else (
            {"slug": "a", "name": "A", "description": ""},
            {"slug": "b", "name": "B", "description": ""},
        )
        specs.append(
            FanOutSpec(
                stage="characters-significant",
                index=index,
                template_name="character-significant-fanout.md",
                model_role="fan_out_creative",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "centrality": "Q4",
                    "primary_cluster": pair[0].get("slug", ""),
                    "boundary_cluster": pair[1].get("slug", ""),
                    "primary_cluster_desc": pair[0].get("description", ""),
                    "boundary_cluster_desc": pair[1].get("description", ""),
                    "axes_subset": axes_subset,
                    "archetypes_summary": archetypes_summary,
                },
            )
        )
        index += 1

    return specs


# ---------------------------------------------------------------------------
# Context summarizers
# ---------------------------------------------------------------------------


def _summarize_places(places: list[dict[str, Any]]) -> str:
    """Format places as a brief summary for downstream context."""
    lines = []
    for p in places:
        lines.append(f"- **{p.get('name', '?')}** `{p.get('slug', '?')}` ({p.get('place_type', '?')})")
    return "\n".join(lines) if lines else "No places generated."


def _summarize_orgs(orgs: list[dict[str, Any]]) -> str:
    """Format organizations as a brief summary for downstream context."""
    lines = []
    for o in orgs:
        lines.append(f"- **{o.get('name', '?')}** `{o.get('slug', '?')}` ({o.get('org_type', '?')})")
    return "\n".join(lines) if lines else "No organizations generated."


def _summarize_clusters(clusters: list[dict[str, Any]]) -> str:
    """Format clusters as a brief summary for downstream context."""
    lines = []
    for c in clusters:
        lines.append(
            f"- **{c.get('name', '?')}** `{c.get('slug', '?')}` ({c.get('basis', '?')}): "
            f"{c.get('description', '')[:150]}"
        )
    return "\n".join(lines) if lines else "No clusters generated."


def _summarize_mundane_characters(chars: list[dict[str, Any]]) -> str:
    """Format mundane characters as a brief summary for downstream context."""
    lines = []
    for c in chars:
        lines.append(
            f"- [{c.get('centrality', '?')}] **{c.get('name', '?')}** `{c.get('slug', '?')}` "
            f"— {c.get('role', '?')} ({c.get('cluster_membership', '?')})"
        )
    return "\n".join(lines) if lines else "No mundane characters generated."


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def orchestrate_world(
    data_path: Path,
    world_slug: str,
    stage: str | None = None,
    coherence_only: bool = False,
) -> None:
    """Run the decomposed fan-out/fan-in pipeline for a world.

    Args:
        data_path: Root of storyteller-data.
        world_slug: World identifier.
        stage: If set, run only this stage (uses existing upstream outputs).
        coherence_only: If True, skip fan-out and use existing draft files.
    """
    from rich.console import Console

    console = Console()
    client = OllamaClient()

    # Setup
    console.print(f"[bold]Decomposed pipeline for[/bold] [cyan]{world_slug}[/cyan]")
    decomposed = _ensure_decomposed_dir(data_path, world_slug)
    world_summary = _build_world_summary_from_path(data_path, world_slug)
    genre_slug = world_summary["genre_slug"]
    setting_slug = world_summary["setting_slug"]
    compressed = world_summary["compressed_preamble"]

    # Save world summary
    (decomposed / "world-summary.json").write_text(json.dumps(world_summary, indent=2))
    console.print(f"  Compressed preamble: [dim]{len(compressed)} chars[/dim] (from ~36,500)")

    # Genre profile summary
    genre_profile = world_summary.get("genre_profile")
    genre_profile_summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    # Planning call
    stages_to_run = [stage] if stage else _STAGES

    if not coherence_only and (stage is None or stage == _STAGES[0]):
        console.print("[bold]Running entity planning call…[/bold]")
        plan = plan_entities(
            client,
            _DECOMPOSED_PROMPTS / "entity-plan.md",
            world_summary,
            genre_profile_summary,
        )
        (decomposed / "entity-plan.json").write_text(json.dumps(plan, indent=2))
        console.print(f"  Plan: {json.dumps(plan, indent=None)}")
    else:
        plan = json.loads((decomposed / "entity-plan.json").read_text())

    # Track outputs for downstream context
    places_final: list[dict[str, Any]] = []
    orgs_final: list[dict[str, Any]] = []
    clusters_final: list[dict[str, Any]] = []
    relationships_final: list[dict[str, Any]] = []
    mundane_final: list[dict[str, Any]] = []

    # ---------------------------------------------------------------------------
    # Stage 1: Places
    # ---------------------------------------------------------------------------
    if "places" in stages_to_run:
        console.print("\n[bold blue]═══ Stage 1: Places ═══[/bold blue]")
        if not coherence_only:
            specs = _build_place_specs(plan, compressed, genre_slug, setting_slug)
            console.print(f"  Fan-out: {len(specs)} place specs")
            results = fan_out(client, _DECOMPOSED_PROMPTS, specs)
            save_instances(decomposed, "places", specs, results)
            aggregate(decomposed, "places", results)
            console.print(f"  [green]{len(results)}[/green] instances generated")
        else:
            results = json.loads((decomposed / "places-draft.json").read_text())

        console.print("  Running coherence pass…")
        places_final = cohere(
            client,
            _DECOMPOSED_PROMPTS / "places-coherence.md",
            world_summary,
            results,
            "",
            {"genre_profile_summary": genre_profile_summary},
        )
        save_coherence_output(decomposed, "places", places_final, world_slug, genre_slug, setting_slug)
        console.print(f"  [green]{len(places_final)}[/green] places finalized")
    else:
        places_data = json.loads((decomposed / "places.json").read_text())
        places_final = places_data.get("places", [])

    # ---------------------------------------------------------------------------
    # Stage 2: Organizations
    # ---------------------------------------------------------------------------
    if "orgs" in stages_to_run:
        console.print("\n[bold blue]═══ Stage 2: Organizations ═══[/bold blue]")
        places_summary = _summarize_places(places_final)
        if not coherence_only:
            specs = _build_org_specs(plan, compressed, genre_slug, setting_slug, places_summary)
            console.print(f"  Fan-out: {len(specs)} org specs")
            results = fan_out(client, _DECOMPOSED_PROMPTS, specs)
            save_instances(decomposed, "orgs", specs, results)
            aggregate(decomposed, "orgs", results)
            console.print(f"  [green]{len(results)}[/green] instances generated")
        else:
            results = json.loads((decomposed / "orgs-draft.json").read_text())

        console.print("  Running coherence pass…")
        orgs_final = cohere(
            client,
            _DECOMPOSED_PROMPTS / "orgs-coherence.md",
            world_summary,
            results,
            places_summary,
        )
        save_coherence_output(decomposed, "orgs", orgs_final, world_slug, genre_slug, setting_slug)
        console.print(f"  [green]{len(orgs_final)}[/green] organizations finalized")
    else:
        orgs_data = json.loads((decomposed / "organizations.json").read_text())
        orgs_final = orgs_data.get("organizations", [])

    # ---------------------------------------------------------------------------
    # Stage 3: Social Substrate
    # ---------------------------------------------------------------------------
    if "substrate" in stages_to_run:
        console.print("\n[bold blue]═══ Stage 3: Social Substrate ═══[/bold blue]")
        upstream = _summarize_places(places_final) + "\n\n" + _summarize_orgs(orgs_final)
        if not coherence_only:
            specs = _build_substrate_specs(plan, compressed, genre_slug, setting_slug, upstream)
            console.print(f"  Fan-out: {len(specs)} cluster specs")
            results = fan_out(client, _DECOMPOSED_PROMPTS, specs)
            save_instances(decomposed, "substrate", specs, results)
            aggregate(decomposed, "substrate", results)
            console.print(f"  [green]{len(results)}[/green] instances generated")
        else:
            results = json.loads((decomposed / "substrate-draft.json").read_text())

        console.print("  Running coherence pass…")
        substrate_response = cohere(
            client,
            _DECOMPOSED_PROMPTS / "substrate-coherence.md",
            world_summary,
            results,
            upstream,
        )
        # Substrate coherence returns {clusters: [...], relationships: [...]}
        # or a flat list — handle both
        if isinstance(substrate_response, list):
            clusters_final = substrate_response
            relationships_final = []
        elif isinstance(substrate_response, dict):
            clusters_final = substrate_response.get("clusters", [])
            relationships_final = substrate_response.get("relationships", [])
        else:
            clusters_final = []
            relationships_final = []

        # Save substrate with relationships
        substrate_output: dict[str, Any] = {
            "world_slug": world_slug,
            "genre_slug": genre_slug,
            "setting_slug": setting_slug,
            "pipeline": "decomposed",
            "cluster_count": len(clusters_final),
            "relationship_count": len(relationships_final),
            "clusters": clusters_final,
            "relationships": relationships_final,
        }
        (decomposed / "social-substrate.json").write_text(json.dumps(substrate_output, indent=2))
        console.print(
            f"  [green]{len(clusters_final)}[/green] clusters, "
            f"[green]{len(relationships_final)}[/green] relationships finalized"
        )
    else:
        substrate_data = json.loads((decomposed / "social-substrate.json").read_text())
        clusters_final = substrate_data.get("clusters", [])
        relationships_final = substrate_data.get("relationships", [])

    # ---------------------------------------------------------------------------
    # Stage 4a: Mundane Characters
    # ---------------------------------------------------------------------------
    if "characters-mundane" in stages_to_run:
        console.print("\n[bold blue]═══ Stage 4a: Mundane Characters ═══[/bold blue]")
        substrate_summary = _summarize_clusters(clusters_final)
        if not coherence_only:
            specs = _build_mundane_character_specs(
                plan, compressed, genre_slug, setting_slug, clusters_final
            )
            console.print(f"  Fan-out: {len(specs)} character specs")
            results = fan_out(client, _DECOMPOSED_PROMPTS, specs)
            save_instances(decomposed, "characters-mundane", specs, results)
            aggregate(decomposed, "characters-mundane", results)
            console.print(f"  [green]{len(results)}[/green] instances generated")
        else:
            results = json.loads((decomposed / "characters-mundane-draft.json").read_text())

        console.print("  Running coherence pass…")
        mundane_final = cohere(
            client,
            _DECOMPOSED_PROMPTS / "characters-mundane-coherence.md",
            world_summary,
            results,
            substrate_summary,
        )
        save_coherence_output(
            decomposed, "characters-mundane", mundane_final, world_slug, genre_slug, setting_slug
        )
        q1 = [c for c in mundane_final if c.get("centrality") == "Q1"]
        q2 = [c for c in mundane_final if c.get("centrality") == "Q2"]
        console.print(f"  [green]{len(q1)}[/green] Q1 + [green]{len(q2)}[/green] Q2 finalized")
    else:
        mundane_data = json.loads((decomposed / "characters-mundane.json").read_text())
        mundane_final = mundane_data.get("characters", [])

    # ---------------------------------------------------------------------------
    # Stage 4b: Significant Characters
    # ---------------------------------------------------------------------------
    if "characters-significant" in stages_to_run:
        console.print("\n[bold blue]═══ Stage 4b: Significant Characters (DEEP) ═══[/bold blue]")

        # Load archetype data
        archetypes = _load_archetypes(data_path, genre_slug)
        dynamics = _load_archetype_dynamics(data_path, genre_slug)
        archetypes_context = _build_archetypes_context(archetypes)
        dynamics_context = _build_dynamics_context(dynamics)
        console.print(
            f"  Archetypes: [green]{len(archetypes)}[/green], "
            f"Dynamics: [green]{len(dynamics)}[/green]"
        )

        if not coherence_only:
            specs = _build_significant_character_specs(
                plan, compressed, genre_slug, setting_slug, clusters_final, archetypes_context
            )
            console.print(f"  Fan-out: {len(specs)} character specs (using 9b)")
            results = fan_out(client, _DECOMPOSED_PROMPTS, specs)
            save_instances(decomposed, "characters-significant", specs, results)
            aggregate(decomposed, "characters-significant", results)
            console.print(f"  [green]{len(results)}[/green] skeletons generated")
        else:
            results = json.loads((decomposed / "characters-significant-draft.json").read_text())

        # Build rich upstream context for the deep coherence pass
        upstream_parts = [
            "## Places\n" + _summarize_places(places_final),
            "## Organizations\n" + _summarize_orgs(orgs_final),
        ]

        console.print("  Running [bold]deep[/bold] coherence pass…")
        significant_final = cohere(
            client,
            _DECOMPOSED_PROMPTS / "characters-significant-coherence.md",
            world_summary,
            results,
            "\n\n".join(upstream_parts),
            extra_context={
                "genre_profile_summary": genre_profile_summary,
                "substrate_context": _summarize_clusters(clusters_final),
                "mundane_context": _summarize_mundane_characters(mundane_final),
                "archetypes_context": archetypes_context,
                "archetype_dynamics_context": dynamics_context,
                "design_principle": (
                    "Ascending centrality simultaneously increases capacity to act and "
                    "constraints on action. The more a character can do, the more the "
                    "world can do to them. Entanglement is the price of agency."
                ),
            },
        )
        save_coherence_output(
            decomposed, "characters-significant", significant_final,
            world_slug, genre_slug, setting_slug,
        )
        q3 = [c for c in significant_final if c.get("centrality") == "Q3"]
        q4 = [c for c in significant_final if c.get("centrality") == "Q4"]
        console.print(f"  [green]{len(q3)}[/green] Q3 + [green]{len(q4)}[/green] Q4 finalized")
    else:
        sig_data = json.loads((decomposed / "characters-significant.json").read_text())
        significant_final = sig_data.get("characters", [])

    # ---------------------------------------------------------------------------
    # Compose world.json
    # ---------------------------------------------------------------------------
    console.print("\n[bold blue]═══ Composing world.json ═══[/bold blue]")
    world_json: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "pipeline": "decomposed",
        "places": places_final,
        "organizations": orgs_final,
        "social_substrate": {
            "clusters": clusters_final,
            "relationships": relationships_final,
        },
        "characters_mundane": mundane_final,
        "characters_significant": significant_final,
    }
    world_path = decomposed / "world.json"
    world_path.write_text(json.dumps(world_json, indent=2))
    console.print(f"[bold green]Written:[/bold green] {world_path}")
    console.print(f"  Total: {len(places_final)} places, {len(orgs_final)} orgs, "
                  f"{len(clusters_final)} clusters, {len(mundane_final)} mundane chars, "
                  f"{len(significant_final)} significant chars")
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_orchestrate_decomposed.py -v`
Expected: All 4 tests PASS

- [ ] **Step 5: Lint**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/orchestrate_decomposed.py tests/tome/test_orchestrate_decomposed.py && uv run ruff format --check src/narrative_data/tome/orchestrate_decomposed.py tests/tome/test_orchestrate_decomposed.py`
Expected: Clean (fix any issues)

- [ ] **Step 6: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/orchestrate_decomposed.py \
        tools/narrative-data/tests/tome/test_orchestrate_decomposed.py
git commit -m "feat(tome): add decomposed pipeline orchestrator — fan-out/coherence per stage"
```

---

## Task 9: CLI Command Registration

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Add the elicit-decomposed command**

In `tools/narrative-data/src/narrative_data/cli.py`, after the `tome_elicit_characters_significant` command (around line 420), add:

```python
@tome.command("elicit-decomposed")
@click.option("--world-slug", required=True, help="World slug under tome/worlds/.")
@click.option("--stage", default=None, help="Run only this stage (places, orgs, substrate, characters-mundane, characters-significant).")
@click.option("--coherence-only", is_flag=True, default=False, help="Skip fan-out, use existing drafts.")
def tome_elicit_decomposed(world_slug: str, stage: str | None, coherence_only: bool) -> None:
    """Run the decomposed fan-out/fan-in elicitation pipeline for a world."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.orchestrate_decomposed import orchestrate_world

    data_path = resolve_data_path()
    orchestrate_world(data_path, world_slug, stage=stage, coherence_only=coherence_only)
```

- [ ] **Step 2: Verify CLI registration**

Run: `cd tools/narrative-data && uv run narrative-data tome elicit-decomposed --help`
Expected: Shows help with `--world-slug`, `--stage`, `--coherence-only` options

- [ ] **Step 3: Commit**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(tome): register elicit-decomposed CLI command"
```

---

## Task 10: Full Test Suite Validation

- [ ] **Step 1: Run all tome tests**

Run: `cd tools/narrative-data && uv run pytest tests/tome/ -v`
Expected: All existing tests PASS + all new tests PASS

- [ ] **Step 2: Run ruff on all new and modified files**

Run: `cd tools/narrative-data && uv run ruff check src/narrative_data/tome/ tests/tome/ && uv run ruff format --check src/narrative_data/tome/ tests/tome/`
Expected: Clean

- [ ] **Step 3: Run ruff format (fix any issues)**

Run: `cd tools/narrative-data && uv run ruff format src/narrative_data/tome/ tests/tome/`

- [ ] **Step 4: Commit formatting if needed**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add -u
git commit -m "style: apply ruff formatting to Phase 3c decomposed pipeline modules"
```

---

## Task 11: Ablation Test on McCallister's Barn

This task requires a running Ollama instance with qwen2.5:7b-instruct, qwen3.5:9b, and
qwen3.5:35b available.

- [ ] **Step 1: Run the decomposed pipeline**

Run: `cd tools/narrative-data && uv run narrative-data tome elicit-decomposed --world-slug mccallisters-barn`

Monitor the output for:
- Planning call produces reasonable counts
- Fan-out instances generate successfully (watch for JSON parse failures)
- Coherence passes complete without timeout
- Total wall time (target: <20 minutes)

- [ ] **Step 2: Inspect outputs**

Check the decomposed directory:
```bash
ls -la storyteller-data/narrative-data/tome/worlds/mccallisters-barn/decomposed/
ls -la storyteller-data/narrative-data/tome/worlds/mccallisters-barn/decomposed/fan-out/places/
```

Compare entity counts against baseline:
```bash
python3 -c "
import json
from pathlib import Path
base = Path('storyteller-data/narrative-data/tome/worlds/mccallisters-barn')
decomp = base / 'decomposed'
for name in ['places', 'organizations', 'social-substrate', 'characters-mundane', 'characters-significant']:
    bfile = base / f'{name}.json'
    dfile = decomp / f'{name}.json'
    if bfile.exists() and dfile.exists():
        b = json.loads(bfile.read_text())
        d = json.loads(dfile.read_text())
        # Find the entity array key
        for key in ['places', 'organizations', 'clusters', 'characters']:
            if key in b and key in d:
                print(f'{name}: baseline={len(b[key])}, decomposed={len(d[key])}')
                break
"
```

- [ ] **Step 3: Quality comparison**

Manually compare 2-3 entities from each stage against the baseline for:
1. Axis grounding (do descriptions reference material conditions?)
2. Inter-entity references (do orgs cite places by slug?)
3. Material specificity (distinct to this world or generic?)
4. Relational density (Q3-Q4 verb:slug seed count and diversity)
5. Stated/operative gap quality (grounded in position or generic?)

- [ ] **Step 4: Document results**

Record timing, entity counts, and quality observations. If quality is on par, proceed to
remaining worlds.

- [ ] **Step 5: Commit any prompt adjustments**

If ablation reveals prompt issues, fix the relevant template and re-run the affected stage
using `--stage <name> --coherence-only`.
