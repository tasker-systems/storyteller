# Tome Phase 3a: Lore Elicitation — Places and Organizations

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the world composition pipeline (seed + graph propagation with genre bias) and lore entity elicitation prompts for places and organizations, then validate with a test world.

**Architecture:** Python scripts in the existing `narrative_data.tome` module. Graph propagation reads `edges.json` + domain JSON files. Genre bias reads bedrock structured data from `storyteller-data`. LLM elicitation via `OllamaClient.generate()` with `qwen3.5:35b`. Output as JSON-with-prose files per world in `storyteller-data/narrative-data/tome/worlds/`.

**Tech Stack:** Python 3.11+, Click CLI, httpx (Ollama), Rich console, existing narrative-data pipeline patterns (manifest, archive, hash tracking).

**Spec:** `docs/superpowers/specs/2026-03-27-tome-phase-3a-lore-elicitation-places-and-orgs-design.md`

---

### Task 1: World Position Data Model

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/world_position.py`
- Test: `tools/narrative-data/tests/tome/test_world_position.py`

- [ ] **Step 1: Create test directory**

```bash
mkdir -p tools/narrative-data/tests/tome
touch tools/narrative-data/tests/tome/__init__.py
```

- [ ] **Step 2: Write tests for WorldPosition and axis loading**

```python
# tools/narrative-data/tests/tome/test_world_position.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for world position data model and axis loading."""

import json
from pathlib import Path

from narrative_data.tome.world_position import (
    AxisPosition,
    WorldPosition,
    load_all_axes,
    load_graph,
)


def _make_domain_file(tmp_path: Path, slug: str, axes: list[dict]) -> None:
    """Write a minimal domain JSON file."""
    domain_dir = tmp_path / "narrative-data" / "tome" / "domains"
    domain_dir.mkdir(parents=True, exist_ok=True)
    data = {
        "domain": {"slug": slug, "name": slug.title(), "description": "test"},
        "axes": axes,
    }
    (domain_dir / f"{slug}.json").write_text(json.dumps(data))


def _make_edges_file(tmp_path: Path, edges: list[dict]) -> None:
    """Write a minimal edges.json."""
    edges_dir = tmp_path / "narrative-data" / "tome"
    edges_dir.mkdir(parents=True, exist_ok=True)
    data = {
        "exported_at": "2026-01-01T00:00:00Z",
        "edge_count": len(edges),
        "compound_edge_count": 0,
        "edges": edges,
        "compound_edges": [],
        "_cluster_annotations": {},
    }
    (edges_dir / "edges.json").write_text(json.dumps(data))


def test_load_all_axes(tmp_path: Path) -> None:
    _make_domain_file(tmp_path, "test-domain", [
        {"slug": "axis-a", "name": "Axis A", "domain": "test-domain",
         "description": "test", "axis_type": "categorical",
         "values": ["low", "mid", "high"]},
        {"slug": "axis-b", "name": "Axis B", "domain": "test-domain",
         "description": "test", "axis_type": "ordinal",
         "values": ["none", "some", "full"]},
    ])
    axes = load_all_axes(tmp_path)
    assert "axis-a" in axes
    assert axes["axis-a"]["domain"] == "test-domain"
    assert axes["axis-a"]["axis_type"] == "categorical"


def test_load_graph(tmp_path: Path) -> None:
    edges = [
        {"from_axis": "a", "to_axis": "b", "edge_type": "produces",
         "weight": 0.9, "description": "a produces b",
         "from_domain": "d1", "to_domain": "d1", "provenance": "systematic"},
    ]
    _make_edges_file(tmp_path, edges)
    graph = load_graph(tmp_path)
    assert graph["edge_count"] == 1
    assert graph["edges"][0]["from_axis"] == "a"


def test_axis_position_creation() -> None:
    pos = AxisPosition(
        axis_slug="geography-climate",
        value="temperate-maritime",
        confidence=1.0,
        source="seed",
        justification=None,
    )
    assert pos.axis_slug == "geography-climate"
    assert pos.is_seed


def test_world_position_set_and_get() -> None:
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="the-village-that-watches")
    wp.set_seed("geography-climate", "temperate-maritime")
    pos = wp.get("geography-climate")
    assert pos is not None
    assert pos.value == "temperate-maritime"
    assert pos.source == "seed"
    assert pos.confidence == 1.0


def test_world_position_unset_axes() -> None:
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="the-village-that-watches")
    wp.set_seed("axis-a", "val-a")
    all_slugs = {"axis-a", "axis-b", "axis-c"}
    unset = wp.unset_axes(all_slugs)
    assert unset == {"axis-b", "axis-c"}


def test_world_position_to_dict() -> None:
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="the-village-that-watches")
    wp.set_seed("geography-climate", "temperate-maritime")
    d = wp.to_dict()
    assert d["genre_slug"] == "folk-horror"
    assert d["setting_slug"] == "the-village-that-watches"
    assert len(d["positions"]) == 1
    assert d["positions"][0]["axis_slug"] == "geography-climate"
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/tome/test_world_position.py -v
```

Expected: FAIL — `ModuleNotFoundError: No module named 'narrative_data.tome.world_position'`

- [ ] **Step 4: Implement WorldPosition data model**

```python
# tools/narrative-data/src/narrative_data/tome/world_position.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""World position data model for Tome lore elicitation.

A WorldPosition is a set of axis positions (one per Tome axis) that
describes a specific world.  Positions are either seeded by the author
or inferred via graph propagation.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass(frozen=True, slots=True)
class AxisPosition:
    """A single axis position within a world."""

    axis_slug: str
    value: str
    confidence: float
    source: str  # "seed" or "inferred"
    justification: str | None = None  # edge chain that produced this

    @property
    def is_seed(self) -> bool:
        return self.source == "seed"

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "axis_slug": self.axis_slug,
            "value": self.value,
            "confidence": self.confidence,
            "source": self.source,
        }
        if self.justification:
            d["justification"] = self.justification
        return d


class WorldPosition:
    """Complete world position: genre + setting + axis positions."""

    def __init__(self, genre_slug: str, setting_slug: str) -> None:
        self.genre_slug = genre_slug
        self.setting_slug = setting_slug
        self._positions: dict[str, AxisPosition] = {}

    def set_seed(self, axis_slug: str, value: str) -> None:
        """Set a seed position (author-provided, confidence 1.0)."""
        self._positions[axis_slug] = AxisPosition(
            axis_slug=axis_slug,
            value=value,
            confidence=1.0,
            source="seed",
        )

    def set_inferred(
        self,
        axis_slug: str,
        value: str,
        confidence: float,
        justification: str,
    ) -> None:
        """Set an inferred position (graph-propagated)."""
        self._positions[axis_slug] = AxisPosition(
            axis_slug=axis_slug,
            value=value,
            confidence=confidence,
            source="inferred",
            justification=justification,
        )

    def get(self, axis_slug: str) -> AxisPosition | None:
        return self._positions.get(axis_slug)

    def is_set(self, axis_slug: str) -> bool:
        return axis_slug in self._positions

    def unset_axes(self, all_slugs: set[str]) -> set[str]:
        """Return axis slugs that don't have positions yet."""
        return all_slugs - set(self._positions.keys())

    @property
    def positions(self) -> dict[str, AxisPosition]:
        return dict(self._positions)

    @property
    def seed_count(self) -> int:
        return sum(1 for p in self._positions.values() if p.is_seed)

    @property
    def inferred_count(self) -> int:
        return sum(1 for p in self._positions.values() if not p.is_seed)

    def to_dict(self) -> dict[str, Any]:
        return {
            "genre_slug": self.genre_slug,
            "setting_slug": self.setting_slug,
            "seed_count": self.seed_count,
            "inferred_count": self.inferred_count,
            "total_positions": len(self._positions),
            "positions": [p.to_dict() for p in self._positions.values()],
        }

    def save(self, path: Path) -> None:
        """Write world position to JSON file."""
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w") as f:
            json.dump(self.to_dict(), f, indent=2)
            f.write("\n")


def load_all_axes(data_path: Path) -> dict[str, dict[str, Any]]:
    """Load all Tome axes keyed by slug from domain JSON files."""
    axes: dict[str, dict[str, Any]] = {}
    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    for f in domains_dir.glob("*.json"):
        with f.open() as fh:
            domain = json.load(fh)
            for axis in domain["axes"]:
                axes[axis["slug"]] = axis
    return axes


def load_graph(data_path: Path) -> dict[str, Any]:
    """Load the canonical edges.json."""
    path = data_path / "narrative-data" / "tome" / "edges.json"
    with path.open() as f:
        return json.load(f)
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/tome/test_world_position.py -v
```

Expected: all 5 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/tome/world_position.py \
        tools/narrative-data/tests/tome/__init__.py \
        tools/narrative-data/tests/tome/test_world_position.py
git commit -m "feat: add WorldPosition data model for Tome lore elicitation"
```

---

### Task 2: Graph Propagation Engine

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/propagation.py`
- Test: `tools/narrative-data/tests/tome/test_propagation.py`

- [ ] **Step 1: Write tests for graph propagation**

```python
# tools/narrative-data/tests/tome/test_propagation.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for mutual production graph propagation with genre bias."""

import json
from pathlib import Path

from narrative_data.tome.propagation import (
    build_incoming_index,
    propagate,
    score_candidates,
)
from narrative_data.tome.world_position import WorldPosition


def _make_test_data(tmp_path: Path) -> Path:
    """Create minimal domain + edge files for propagation testing."""
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    domains_dir.mkdir(parents=True)
    edges_dir = tmp_path / "narrative-data" / "tome"

    # Two domains, 4 axes total
    d1 = {
        "domain": {"slug": "material", "name": "Material", "description": ""},
        "axes": [
            {"slug": "geo", "name": "Geo", "domain": "material",
             "description": "", "axis_type": "categorical",
             "values": ["desert", "temperate", "arctic"]},
            {"slug": "resources", "name": "Resources", "domain": "material",
             "description": "", "axis_type": "categorical",
             "values": ["scarce", "moderate", "abundant"]},
        ],
    }
    d2 = {
        "domain": {"slug": "economic", "name": "Economic", "description": ""},
        "axes": [
            {"slug": "production", "name": "Production", "domain": "economic",
             "description": "", "axis_type": "categorical",
             "values": ["foraging", "agrarian", "industrial"]},
            {"slug": "trade", "name": "Trade", "domain": "economic",
             "description": "", "axis_type": "categorical",
             "values": ["local", "regional", "global"]},
        ],
    }
    (domains_dir / "material.json").write_text(json.dumps(d1))
    (domains_dir / "economic.json").write_text(json.dumps(d2))

    edges = {
        "exported_at": "2026-01-01T00:00:00Z",
        "edge_count": 3,
        "compound_edge_count": 0,
        "edges": [
            {"from_axis": "geo", "to_axis": "resources", "edge_type": "produces",
             "weight": 0.9, "description": "geo produces resources",
             "from_domain": "material", "to_domain": "material",
             "provenance": "systematic"},
            {"from_axis": "resources", "to_axis": "production", "edge_type": "produces",
             "weight": 0.8, "description": "resources produce production",
             "from_domain": "material", "to_domain": "economic",
             "provenance": "systematic"},
            {"from_axis": "production", "to_axis": "trade", "edge_type": "enables",
             "weight": 0.6, "description": "production enables trade",
             "from_domain": "economic", "to_domain": "economic",
             "provenance": "systematic"},
        ],
        "compound_edges": [],
        "_cluster_annotations": {},
    }
    (edges_dir / "edges.json").write_text(json.dumps(edges))
    return tmp_path


def test_build_incoming_index(tmp_path: Path) -> None:
    data_path = _make_test_data(tmp_path)
    edges_data = json.loads(
        (data_path / "narrative-data" / "tome" / "edges.json").read_text()
    )
    incoming = build_incoming_index(edges_data)
    assert "resources" in incoming
    assert len(incoming["resources"]) == 1
    assert incoming["resources"][0]["from_axis"] == "geo"


def test_propagate_fills_all_axes(tmp_path: Path) -> None:
    data_path = _make_test_data(tmp_path)
    wp = WorldPosition(genre_slug="test-genre", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, data_path)
    assert result.get("resources") is not None
    assert result.get("production") is not None
    assert result.get("trade") is not None
    assert result.seed_count == 1
    assert result.inferred_count == 3


def test_propagate_respects_seeds(tmp_path: Path) -> None:
    data_path = _make_test_data(tmp_path)
    wp = WorldPosition(genre_slug="test-genre", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")
    wp.set_seed("resources", "scarce")  # Override what graph would infer

    result = propagate(wp, data_path)
    pos = result.get("resources")
    assert pos is not None
    assert pos.value == "scarce"
    assert pos.source == "seed"


def test_score_candidates_edge_type_multipliers() -> None:
    # produces edge should score higher than enables edge at same weight
    incoming_produces = [
        {"from_axis": "a", "to_axis": "target", "edge_type": "produces",
         "weight": 0.7, "description": ""},
    ]
    incoming_enables = [
        {"from_axis": "a", "to_axis": "target", "edge_type": "enables",
         "weight": 0.7, "description": ""},
    ]
    set_positions = {"a": "some-value"}

    score_p = score_candidates(incoming_produces, set_positions)
    score_e = score_candidates(incoming_enables, set_positions)
    assert score_p > score_e


def test_propagate_records_justification(tmp_path: Path) -> None:
    data_path = _make_test_data(tmp_path)
    wp = WorldPosition(genre_slug="test-genre", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, data_path)
    resources_pos = result.get("resources")
    assert resources_pos is not None
    assert resources_pos.justification is not None
    assert "geo" in resources_pos.justification
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd tools/narrative-data && uv run pytest tests/tome/test_propagation.py -v
```

Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement graph propagation**

```python
# tools/narrative-data/src/narrative_data/tome/propagation.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Mutual production graph propagation with genre-weighted bias.

Given seed axis positions, traverses the edge graph to infer positions
for all remaining axes.  Genre bias acts as a soft prior — shifting
probability distributions without hard-gating values.
"""

from __future__ import annotations

import random
from collections import defaultdict
from pathlib import Path
from typing import Any

from narrative_data.tome.world_position import (
    WorldPosition,
    load_all_axes,
    load_graph,
)

# Edge type multipliers (from spec)
_EDGE_TYPE_MULTIPLIERS: dict[str, float] = {
    "produces": 1.0,
    "constrains": 0.8,
    "enables": 0.6,
    "transforms": 0.0,  # Operates over time, not at composition
}


def build_incoming_index(
    graph_data: dict[str, Any],
) -> dict[str, list[dict[str, Any]]]:
    """Build to_axis → [edge, ...] index for incoming edge lookup."""
    incoming: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for edge in graph_data["edges"]:
        incoming[edge["to_axis"]].append(edge)
    return dict(incoming)


def score_candidates(
    incoming_edges: list[dict[str, Any]],
    set_positions: dict[str, str],
    genre_bias: dict[str, float] | None = None,
) -> float:
    """Score how strongly the already-set axes determine a target axis.

    Returns the cumulative weighted score from all incoming edges whose
    source axis is already set.  Higher score = more determined.
    """
    total = 0.0
    for edge in incoming_edges:
        if edge["from_axis"] not in set_positions:
            continue
        multiplier = _EDGE_TYPE_MULTIPLIERS.get(edge["edge_type"], 0.0)
        weight = edge.get("weight", 0.5)
        total += weight * multiplier
    return total


def _select_value(
    axis: dict[str, Any],
    incoming_edges: list[dict[str, Any]],
    set_positions: dict[str, str],
) -> str:
    """Select a value for an axis based on incoming edge context.

    For now, selects randomly from valid values.  The edge context
    provides the *confidence* that this axis should be set (handled by
    the propagation order), not the specific value — value selection
    is delegated to the LLM during elicitation.

    A future iteration can use constrains edges to eliminate values
    and produces edges to prefer specific values.
    """
    axis_type = axis.get("axis_type", "categorical")
    values = axis.get("values", [])

    if axis_type in ("categorical", "set", "ordinal") and isinstance(values, list):
        return random.choice(values)
    elif axis_type == "bipolar" and isinstance(values, dict):
        return random.choice(["low", "mid", "high"])
    elif axis_type == "profile" and isinstance(values, dict):
        levels = values.get("levels", ["scarce", "moderate", "abundant"])
        subs = values.get("sub_dimensions", [])
        return ", ".join(f"{s}:{random.choice(levels)}" for s in subs)
    return "unresolved"


def _build_justification(
    axis_slug: str,
    incoming_edges: list[dict[str, Any]],
    set_positions: dict[str, str],
) -> str:
    """Build a human-readable justification string from active edges."""
    parts: list[str] = []
    for edge in incoming_edges:
        if edge["from_axis"] not in set_positions:
            continue
        multiplier = _EDGE_TYPE_MULTIPLIERS.get(edge["edge_type"], 0.0)
        if multiplier == 0.0:
            continue
        parts.append(
            f"{edge['from_axis']} →{edge['edge_type']}→ {axis_slug} "
            f"(w={edge['weight']})"
        )
    return "; ".join(parts) if parts else "no incoming edges"


def propagate(
    world_position: WorldPosition,
    data_path: Path,
    genre_bias: dict[str, float] | None = None,
) -> WorldPosition:
    """Propagate seed positions through the mutual production graph.

    Iteratively selects the most-determined unset axis (highest
    cumulative incoming score from already-set axes) and fills it.
    Repeats until all axes are set or no further propagation is possible.

    Args:
        world_position: WorldPosition with seed positions set.
        data_path: Root of storyteller-data repository.
        genre_bias: Optional genre dimensional profile for soft biasing.

    Returns:
        The same WorldPosition, now with inferred positions filled.
    """
    axes = load_all_axes(data_path)
    graph_data = load_graph(data_path)
    incoming = build_incoming_index(graph_data)

    all_slugs = set(axes.keys())
    max_iterations = len(all_slugs)  # Safety bound

    for _ in range(max_iterations):
        unset = world_position.unset_axes(all_slugs)
        if not unset:
            break

        # Build current set_positions lookup
        set_positions = {
            slug: pos.value
            for slug, pos in world_position.positions.items()
        }

        # Score each unset axis by how determined it is
        scored: list[tuple[str, float]] = []
        for slug in unset:
            edges = incoming.get(slug, [])
            score = score_candidates(edges, set_positions, genre_bias)
            if score > 0:
                scored.append((slug, score))

        if not scored:
            # Remaining axes have no incoming edges from set axes.
            # Fill them with random values at low confidence.
            for slug in unset:
                if slug in axes:
                    value = _select_value(axes[slug], [], set_positions)
                    world_position.set_inferred(
                        slug, value, confidence=0.1,
                        justification="no incoming edges from set axes",
                    )
            break

        # Sort by score descending — most determined first
        scored.sort(key=lambda x: -x[1])
        best_slug, best_score = scored[0]

        # Select value and record
        axis = axes[best_slug]
        edges = incoming.get(best_slug, [])
        value = _select_value(axis, edges, set_positions)
        justification = _build_justification(best_slug, edges, set_positions)

        # Confidence is the normalized score (capped at 1.0)
        confidence = min(best_score, 1.0)

        world_position.set_inferred(
            best_slug, value, confidence=confidence,
            justification=justification,
        )

    return world_position
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd tools/narrative-data && uv run pytest tests/tome/test_propagation.py -v
```

Expected: all 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/tome/propagation.py \
        tools/narrative-data/tests/tome/test_propagation.py
git commit -m "feat: add mutual production graph propagation engine"
```

---

### Task 3: World Composition CLI Command

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/compose_world.py`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Write the compose_world module**

```python
# tools/narrative-data/src/narrative_data/tome/compose_world.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Compose a world from genre + setting + seed axis positions.

Runs graph propagation to fill all 51 Tome axes, then writes the
world-position.json to the world's output directory.

Usage (via CLI):
    uv run narrative-data tome compose-world \\
        --genre folk-horror \\
        --setting the-village-that-watches \\
        --seed geography-climate=temperate-maritime \\
        --seed resource-profile=soil-fertility:abundant,potable-water:moderate \\
        --world-slug mccallisters-barn
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from narrative_data.tome.propagation import propagate
from narrative_data.tome.world_position import WorldPosition, load_all_axes


def compose_world(
    data_path: Path,
    genre_slug: str,
    setting_slug: str,
    seeds: dict[str, str],
    world_slug: str,
) -> WorldPosition:
    """Compose a world position from genre + setting + seeds.

    Args:
        data_path: Root of storyteller-data repository.
        genre_slug: Genre region slug (e.g. "folk-horror").
        setting_slug: Setting pattern slug (e.g. "the-village-that-watches").
        seeds: Axis slug → value mapping for seed positions.
        world_slug: Identifier for this world (used for output directory).

    Returns:
        The fully-resolved WorldPosition.
    """
    from rich.console import Console

    console = Console()

    # Validate seeds reference real axes
    axes = load_all_axes(data_path)
    for slug in seeds:
        if slug not in axes:
            console.print(f"[red]Unknown axis: {slug}[/red]")
            console.print(f"[dim]Available: {', '.join(sorted(axes.keys()))}[/dim]")
            raise SystemExit(1)

    # Build world position with seeds
    wp = WorldPosition(genre_slug=genre_slug, setting_slug=setting_slug)
    for slug, value in seeds.items():
        wp.set_seed(slug, value)

    console.print(
        f"[cyan]Composing world[/cyan] [bold]{world_slug}[/bold] "
        f"({genre_slug} × {setting_slug})"
    )
    console.print(f"  Seeds: {wp.seed_count} axes")

    # Propagate through graph
    # TODO: load genre dimensional profile for bias when bedrock query is wired
    wp = propagate(wp, data_path, genre_bias=None)

    console.print(
        f"  Inferred: {wp.inferred_count} axes "
        f"(total: {wp.seed_count + wp.inferred_count})"
    )

    # Write output
    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    world_dir.mkdir(parents=True, exist_ok=True)
    output_path = world_dir / "world-position.json"
    wp.save(output_path)

    console.print(f"[green]✓[/green] {output_path}")
    return wp
```

- [ ] **Step 2: Wire CLI command**

Add to `cli.py` after the `tome_stress_test` command (around line 325):

```python
@tome.command("compose-world")
@click.option("--genre", required=True, help="Genre region slug")
@click.option("--setting", required=True, help="Setting pattern slug")
@click.option(
    "--seed",
    multiple=True,
    help="Seed axis position as axis-slug=value (repeatable)",
)
@click.option("--world-slug", required=True, help="World identifier for output directory")
def tome_compose_world(genre: str, setting: str, seed: tuple[str, ...], world_slug: str) -> None:
    """Compose a world position from genre + setting + seed axes."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.compose_world import compose_world

    data_path = resolve_data_path()
    seeds = {}
    for s in seed:
        if "=" not in s:
            raise click.BadParameter(f"Seed must be axis-slug=value, got: {s}")
        slug, value = s.split("=", 1)
        seeds[slug.strip()] = value.strip()

    compose_world(data_path, genre, setting, seeds, world_slug)
```

- [ ] **Step 3: Test the CLI manually**

```bash
cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome compose-world \
    --genre folk-horror \
    --setting the-village-that-watches \
    --seed geography-climate=temperate-maritime \
    --seed infrastructure-development=roads-and-bridges \
    --seed technological-ceiling=medieval-craft \
    --seed population-density=village-clusters \
    --seed supernatural-permeability=Permeable \
    --world-slug mccallisters-barn
```

Expected: world-position.json written to `storyteller-data/narrative-data/tome/worlds/mccallisters-barn/`.

- [ ] **Step 4: Verify output**

```bash
python3 -c "
import json
with open('/Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data/tome/worlds/mccallisters-barn/world-position.json') as f:
    d = json.load(f)
print(f'Genre: {d[\"genre_slug\"]}')
print(f'Seeds: {d[\"seed_count\"]}')
print(f'Inferred: {d[\"inferred_count\"]}')
print(f'Total: {d[\"total_positions\"]}')
for p in d['positions'][:5]:
    print(f'  {p[\"axis_slug\"]}: {p[\"value\"]} ({p[\"source\"]}, conf={p[\"confidence\"]})')
"
```

Expected: 5 seeds + 46 inferred = 51 total positions.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/tome/compose_world.py \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat: add tome compose-world CLI command"
```

---

### Task 4: Place Elicitation Prompt Template and Script

**Files:**
- Create: `tools/narrative-data/prompts/tome/place-elicitation.md`
- Create: `tools/narrative-data/src/narrative_data/tome/elicit_places.py`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Write the place elicitation prompt template**

```markdown
<!-- tools/narrative-data/prompts/tome/place-elicitation.md -->
You are generating named places for a narrative world. The world has been composed
from a mutual production graph — each axis position was produced, constrained, or
enabled by other axes. The edges below show the causal reasoning.

## World: {genre_slug} × {setting_slug}

{world_preamble}

## Genre Spatial Structure

{spatial_context}

## Task

Generate 6-8 named places that constitute the primary locations where narrative
can occur in this world. Each place must be:

1. **Grounded** in the material conditions — traceable to specific Tome axis positions
2. **Connected** to the genre's spatial topology (center, threshold, periphery)
3. **Distinct** — each place represents a different facet of the world's material reality
4. **Communicable** — has a sensory palette and atmospheric quality that the Narrator can render

For each place, output a JSON object with these fields:
- `slug`: kebab-case identifier
- `name`: evocative proper name
- `place_type`: functional category (settlement, production-site, sacred-site, threshold, wilderness, infrastructure, gathering-place)
- `description`: 2-4 sentences of narrative-rich prose. Specific material details, not generic.
- `grounding.material_axes`: list of "axis-slug:value" strings this place embodies
- `grounding.active_edges`: list of "source →type→ target (weight)" strings showing the causal chain
- `communicability.surface_area`: float 0.0-1.0
- `communicability.translation_friction`: float 0.0-1.0
- `communicability.timescale`: one of momentary, biographical, generational, geological, primordial
- `communicability.atmospheric_palette`: sensory description (smell, sound, light, texture)
- `spatial_role`: one of center, threshold, periphery
- `relational_seeds`: list of "relation:target-slug" strings

Output valid JSON: an array of place objects. No commentary outside the JSON.
```

- [ ] **Step 2: Write the place elicitation script**

```python
# tools/narrative-data/src/narrative_data/tome/elicit_places.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit named places from a composed world position using qwen3.5:35b.

Reads the world-position.json, builds a prompt with axis positions and
active edges (Format B), sends to the LLM, and writes places.json.

Usage (via CLI):
    uv run narrative-data tome elicit-places --world-slug mccallisters-barn
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_ELICITATION_TIMEOUT = 600.0
_ELICITATION_TEMPERATURE = 0.5


def _load_world_position(world_dir: Path) -> dict[str, Any]:
    """Load world-position.json from a world directory."""
    path = world_dir / "world-position.json"
    if not path.exists():
        raise FileNotFoundError(f"No world-position.json in {world_dir}")
    with path.open() as f:
        return json.load(f)


def _build_world_preamble(world_pos: dict[str, Any]) -> str:
    """Build the world preamble with axis positions grouped by domain and edges."""
    lines: list[str] = []
    # Group positions by source (seeds first, then inferred)
    seeds = [p for p in world_pos["positions"] if p["source"] == "seed"]
    inferred = [p for p in world_pos["positions"] if p["source"] == "inferred"]

    lines.append("### Seed Positions (author-selected)")
    for p in seeds:
        lines.append(f"- **{p['axis_slug']}**: {p['value']}")

    lines.append("")
    lines.append("### Inferred Positions (from mutual production graph)")
    for p in inferred:
        justification = p.get("justification", "")
        conf = p.get("confidence", 0)
        lines.append(
            f"- **{p['axis_slug']}**: {p['value']} "
            f"(confidence: {conf:.2f})"
        )
        if justification and justification != "no incoming edges from set axes":
            lines.append(f"  - Via: {justification}")

    return "\n".join(lines)


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    spatial_context: str,
) -> str:
    """Substitute world data into the prompt template."""
    preamble = _build_world_preamble(world_pos)
    return (
        template
        .replace("{genre_slug}", world_pos["genre_slug"])
        .replace("{setting_slug}", world_pos["setting_slug"])
        .replace("{world_preamble}", preamble)
        .replace("{spatial_context}", spatial_context)
    )


def _parse_places_response(response: str) -> list[dict[str, Any]]:
    """Parse the LLM response as a JSON array of place objects.

    Tries multiple extraction strategies since the model may wrap
    the JSON in markdown code fences or add commentary.
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
    if "```" in text:
        for block in text.split("```"):
            block = block.strip()
            if block.startswith("json"):
                block = block[4:].strip()
            try:
                result = json.loads(block)
                if isinstance(result, list):
                    return result
            except json.JSONDecodeError:
                continue

    # Strategy 3: find array boundaries
    start = text.find("[")
    end = text.rfind("]")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(f"Could not parse places from LLM response:\n{text[:500]}")


def elicit_places(
    data_path: Path,
    world_slug: str,
) -> None:
    """Elicit places for a composed world and write places.json.

    Args:
        data_path: Root of storyteller-data repository.
        world_slug: World identifier (must have world-position.json).
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "place-elicitation.md"

    if not template_path.exists():
        console.print(f"[red]Prompt template not found: {template_path}[/red]")
        raise SystemExit(1)

    world_pos = _load_world_position(world_dir)
    template = template_path.read_text()

    # Spatial context placeholder — will be enriched with bedrock data later
    spatial_context = (
        f"Genre: {world_pos['genre_slug']}. "
        f"Setting: {world_pos['setting_slug']}. "
        f"Use the genre's typical spatial structure "
        f"(center/threshold/periphery) to distribute places."
    )

    prompt = _build_prompt(template, world_pos, spatial_context)

    console.print(
        f"[cyan]Eliciting places[/cyan] for [bold]{world_slug}[/bold] "
        f"({world_pos['genre_slug']} × {world_pos['setting_slug']})"
    )

    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_ELICITATION_TIMEOUT,
        temperature=_ELICITATION_TEMPERATURE,
    )

    places = _parse_places_response(response)
    console.print(f"  Generated {len(places)} places")

    output = {
        "world_slug": world_slug,
        "genre_slug": world_pos["genre_slug"],
        "setting_slug": world_pos["setting_slug"],
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "place_count": len(places),
        "places": places,
    }

    output_path = world_dir / "places.json"
    with output_path.open("w") as f:
        json.dump(output, f, indent=2)
        f.write("\n")

    console.print(f"[green]✓[/green] {output_path}")
    for place in places:
        name = place.get("name", place.get("slug", "?"))
        role = place.get("spatial_role", "?")
        console.print(f"    {name} ({role})")
```

- [ ] **Step 3: Wire CLI command**

Add to `cli.py` after the `tome_compose_world` command:

```python
@tome.command("elicit-places")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_places(world_slug: str) -> None:
    """Generate named places for a composed world."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_places import elicit_places

    data_path = resolve_data_path()
    elicit_places(data_path, world_slug)
```

- [ ] **Step 4: Commit**

```bash
git add tools/narrative-data/prompts/tome/place-elicitation.md \
        tools/narrative-data/src/narrative_data/tome/elicit_places.py \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat: add place elicitation prompt and script for Tome lore"
```

---

### Task 5: Organization Elicitation Prompt Template and Script

**Files:**
- Create: `tools/narrative-data/prompts/tome/org-elicitation.md`
- Create: `tools/narrative-data/src/narrative_data/tome/elicit_orgs.py`
- Modify: `tools/narrative-data/src/narrative_data/cli.py`

- [ ] **Step 1: Write the organization elicitation prompt template**

```markdown
<!-- tools/narrative-data/prompts/tome/org-elicitation.md -->
You are generating organizations and institutions for a narrative world. The world
has been composed from a mutual production graph, and places have already been
generated. Organizations exist *in* and *across* these places.

## World: {genre_slug} × {setting_slug}

{world_preamble}

## Generated Places

{places_context}

## Task

Generate 4-6 organizations or institutions that structure power, labor, belief,
or social life in this world. Each organization must be:

1. **Grounded** in political, economic, and social axis positions
2. **Connected** to at least one generated place
3. **Specific** — carries the texture of this world's particular power dynamics
4. **Narratively productive** — the stated/operative gap (where applicable) creates tension

For organizations where authority-legitimation or social-stratification carry
stated/operative duality, surface the gap between the stated function and the
operative reality. This gap is where narrative tension lives.

For each organization, output a JSON object with these fields:
- `slug`: kebab-case identifier
- `name`: proper name of the organization
- `org_type`: functional category (governance, economic, religious, military, social, labor, educational)
- `description`: 2-4 sentences of narrative-rich prose. Specific to this world.
- `grounding.political_axes`: list of "axis-slug:value" strings
- `grounding.economic_axes`: list of "axis-slug:value" strings
- `grounding.social_axes`: list of "axis-slug:value" strings (if applicable)
- `grounding.active_edges`: list of "source →type→ target (weight)" strings
- `authority_basis`: one sentence on how this org legitimates its power
- `membership`: who belongs and how membership is determined
- `place_associations`: list of "place-slug:role" strings
- `stated_vs_operative.stated`: what the organization says it does
- `stated_vs_operative.operative`: what it actually does (may be identical if no gap)
- `relational_seeds`: list of "relation:target-slug" strings

Output valid JSON: an array of organization objects. No commentary outside the JSON.
```

- [ ] **Step 2: Write the organization elicitation script**

```python
# tools/narrative-data/src/narrative_data/tome/elicit_orgs.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit organizations from a composed world position + generated places.

Reads world-position.json and places.json, builds a prompt with
political/economic/social axis emphasis, sends to the LLM, and writes
organizations.json.

Usage (via CLI):
    uv run narrative-data tome elicit-orgs --world-slug mccallisters-barn
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_ELICITATION_TIMEOUT = 600.0
_ELICITATION_TEMPERATURE = 0.5


def _load_world_position(world_dir: Path) -> dict[str, Any]:
    """Load world-position.json."""
    path = world_dir / "world-position.json"
    if not path.exists():
        raise FileNotFoundError(f"No world-position.json in {world_dir}")
    with path.open() as f:
        return json.load(f)


def _load_places(world_dir: Path) -> list[dict[str, Any]]:
    """Load places.json."""
    path = world_dir / "places.json"
    if not path.exists():
        raise FileNotFoundError(
            f"No places.json in {world_dir}. Run elicit-places first."
        )
    with path.open() as f:
        data = json.load(f)
    return data.get("places", [])


def _build_world_preamble(world_pos: dict[str, Any]) -> str:
    """Build world preamble with political/economic/social emphasis."""
    # Reuse the same preamble builder from elicit_places
    from narrative_data.tome.elicit_places import _build_world_preamble

    return _build_world_preamble(world_pos)


def _build_places_context(places: list[dict[str, Any]]) -> str:
    """Summarize generated places for the org elicitation prompt."""
    lines: list[str] = []
    for place in places:
        name = place.get("name", place.get("slug", "?"))
        role = place.get("spatial_role", "?")
        desc = place.get("description", "")
        # Truncate long descriptions
        if len(desc) > 200:
            desc = desc[:200] + "..."
        lines.append(f"- **{name}** ({role}): {desc}")
    return "\n".join(lines)


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    places: list[dict[str, Any]],
) -> str:
    """Substitute world data and places into the prompt template."""
    preamble = _build_world_preamble(world_pos)
    places_context = _build_places_context(places)
    return (
        template
        .replace("{genre_slug}", world_pos["genre_slug"])
        .replace("{setting_slug}", world_pos["setting_slug"])
        .replace("{world_preamble}", preamble)
        .replace("{places_context}", places_context)
    )


def _parse_orgs_response(response: str) -> list[dict[str, Any]]:
    """Parse the LLM response as a JSON array of org objects."""
    text = response.strip()

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if isinstance(result, list):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from code fence
    if "```" in text:
        for block in text.split("```"):
            block = block.strip()
            if block.startswith("json"):
                block = block[4:].strip()
            try:
                result = json.loads(block)
                if isinstance(result, list):
                    return result
            except json.JSONDecodeError:
                continue

    # Strategy 3: find array boundaries
    start = text.find("[")
    end = text.rfind("]")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(f"Could not parse orgs from LLM response:\n{text[:500]}")


def elicit_orgs(
    data_path: Path,
    world_slug: str,
) -> None:
    """Elicit organizations for a composed world and write organizations.json.

    Args:
        data_path: Root of storyteller-data repository.
        world_slug: World identifier (must have world-position.json and places.json).
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "org-elicitation.md"

    if not template_path.exists():
        console.print(f"[red]Prompt template not found: {template_path}[/red]")
        raise SystemExit(1)

    world_pos = _load_world_position(world_dir)
    places = _load_places(world_dir)
    template = template_path.read_text()

    prompt = _build_prompt(template, world_pos, places)

    console.print(
        f"[cyan]Eliciting organizations[/cyan] for [bold]{world_slug}[/bold] "
        f"({world_pos['genre_slug']} × {world_pos['setting_slug']})"
    )
    console.print(f"  Using {len(places)} generated places as context")

    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_ELICITATION_TIMEOUT,
        temperature=_ELICITATION_TEMPERATURE,
    )

    orgs = _parse_orgs_response(response)
    console.print(f"  Generated {len(orgs)} organizations")

    output = {
        "world_slug": world_slug,
        "genre_slug": world_pos["genre_slug"],
        "setting_slug": world_pos["setting_slug"],
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "org_count": len(orgs),
        "organizations": orgs,
    }

    output_path = world_dir / "organizations.json"
    with output_path.open("w") as f:
        json.dump(output, f, indent=2)
        f.write("\n")

    console.print(f"[green]✓[/green] {output_path}")
    for org in orgs:
        name = org.get("name", org.get("slug", "?"))
        otype = org.get("org_type", "?")
        console.print(f"    {name} ({otype})")
```

- [ ] **Step 3: Wire CLI command**

Add to `cli.py` after the `tome_elicit_places` command:

```python
@tome.command("elicit-orgs")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_orgs(world_slug: str) -> None:
    """Generate organizations for a composed world (requires places)."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_orgs import elicit_orgs

    data_path = resolve_data_path()
    elicit_orgs(data_path, world_slug)
```

- [ ] **Step 4: Commit**

```bash
git add tools/narrative-data/prompts/tome/org-elicitation.md \
        tools/narrative-data/src/narrative_data/tome/elicit_orgs.py \
        tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat: add organization elicitation prompt and script for Tome lore"
```

---

### Task 6: First Test World — McCallister's Barn (Folk Horror)

**Files:**
- No new code files. This task runs the pipeline end-to-end.

- [ ] **Step 1: Compose the world**

```bash
cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome compose-world \
    --genre folk-horror \
    --setting the-village-that-watches \
    --seed geography-climate=temperate-maritime \
    --seed resource-profile="soil-fertility:abundant,potable-water:moderate,timber-forestry:moderate,mineral-deposits:scarce,fisheries-marine:scarce,arable-land:abundant,fuel-sources:scarce,animal-husbandry-potential:abundant" \
    --seed infrastructure-development=roads-and-bridges \
    --seed disease-ecology=Clean \
    --seed supernatural-permeability=Permeable \
    --seed technological-ceiling=medieval-craft \
    --seed population-density=village-clusters \
    --world-slug mccallisters-barn
```

- [ ] **Step 2: Review world-position.json**

Inspect the output. Verify:
- All 51 axes have positions
- Seeds are marked with `source: "seed"`, confidence 1.0
- Inferred positions have justification traces
- The inferred political/economic/social positions are plausible for folk-horror × isolated village

- [ ] **Step 3: Elicit places**

```bash
STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome elicit-places --world-slug mccallisters-barn
```

- [ ] **Step 4: Review places.json**

Apply the three review lenses:
- **World-coherence**: each place traceable to axis positions?
- **Distinctiveness**: could these places exist in a cyberpunk megacity? (They shouldn't.)
- **Surfacing path**: would the World Agent find each place useful for scene-entry context?

- [ ] **Step 5: Elicit organizations**

```bash
STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome elicit-orgs --world-slug mccallisters-barn
```

- [ ] **Step 6: Review organizations.json**

Apply the three review lenses:
- **World-coherence**: each org traceable to political/economic/social axes?
- **Stated/operative gap**: present where dual-mode axes apply?
- **Place connections**: each org tied to at least one generated place?

- [ ] **Step 7: Write review document**

Create `storyteller-data/narrative-data/tome/worlds/mccallisters-barn/review.md` documenting findings from the review — what worked, what was too generic, what was incoherent, what needs prompt revision.

- [ ] **Step 8: Commit the test world**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/worlds/mccallisters-barn/
git commit -m "data: first test world — McCallister's Barn (folk-horror)"
```

---

### Task 7: Second Test World — Cyberpunk Megacity (Discriminative Power)

**Files:**
- No new code files. This task validates that different seeds produce a different world.

- [ ] **Step 1: Compose the cyberpunk world**

```bash
cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome compose-world \
    --genre cyberpunk \
    --setting the-vertical-city \
    --seed geography-climate=constructed-artificial \
    --seed resource-profile="mineral-deposits:abundant,potable-water:scarce,soil-fertility:absent,timber-forestry:absent,fisheries-marine:absent,arable-land:absent,fuel-sources:abundant,animal-husbandry-potential:absent" \
    --seed infrastructure-development=integrated-systems \
    --seed technological-ceiling=post-digital \
    --seed biological-plasticity=augmentable \
    --seed population-density=megacity-compressed \
    --seed disease-ecology=Pestilent \
    --world-slug neon-depths
```

- [ ] **Step 2: Elicit places**

```bash
STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome elicit-places --world-slug neon-depths
```

- [ ] **Step 3: Elicit organizations**

```bash
STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data \
    uv run narrative-data tome elicit-orgs --world-slug neon-depths
```

- [ ] **Step 4: Discriminative power comparison**

Compare `mccallisters-barn/places.json` with `neon-depths/places.json`:
- Are the places recognizably different? (They must be.)
- Do the grounding axes differ? (Material conditions should drive different outputs.)
- Do the communicability profiles differ? (Timescales, surface areas, atmospheric palettes.)

Compare organizations similarly:
- Different authority bases?
- Different stated/operative gaps?
- Different place connections?

- [ ] **Step 5: Write review and commit**

Create `storyteller-data/narrative-data/tome/worlds/neon-depths/review.md` and commit.

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/worlds/neon-depths/
git commit -m "data: second test world — Neon Depths (cyberpunk, discriminative power test)"
```

---

### Task 8: Final Commit, Push, and Session Save

- [ ] **Step 1: Run all tests**

```bash
cd tools/narrative-data && uv run pytest tests/tome/ -v
```

Expected: all tests pass.

- [ ] **Step 2: Push storyteller repo**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller && git push
```

- [ ] **Step 3: Push storyteller-data repo**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data && git push
```

- [ ] **Step 4: Save session note**

```bash
temper session save "Tome Phase 3a: Lore Elicitation — Places and Organizations" \
    --ticket 2026-03-27-tome-phase-3-lore-elicitation-methodology-and-entity-structure \
    --project storyteller
```
