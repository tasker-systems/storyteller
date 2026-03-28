# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""WorldPosition data model for Tome lore elicitation.

Tracks axis positions (seed + inferred) for a genre/setting combination as
the graph propagation engine fills in the world's narrative coordinates.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# AxisPosition
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class AxisPosition:
    """A single axis placement within a WorldPosition.

    Args:
        axis_slug: Identifier matching a Tome axis slug.
        value: The string value placed on this axis.
        confidence: Float in [0.0, 1.0]; seed positions always use 1.0.
        source: Either "seed" (author-provided) or "inferred" (propagated).
        justification: Optional free-text explanation for inferred positions.
    """

    axis_slug: str
    value: str
    confidence: float
    source: str
    justification: str | None = None

    @property
    def is_seed(self) -> bool:
        """True when this position was author-provided (source == "seed")."""
        return self.source == "seed"

    def to_dict(self) -> dict[str, Any]:
        """Serialise to a plain dict, omitting justification when None."""
        d: dict[str, Any] = {
            "axis_slug": self.axis_slug,
            "value": self.value,
            "confidence": self.confidence,
            "source": self.source,
        }
        if self.justification is not None:
            d["justification"] = self.justification
        return d


# ---------------------------------------------------------------------------
# WorldPosition
# ---------------------------------------------------------------------------


class WorldPosition:
    """Accumulates axis positions for a single genre + setting combination.

    Positions are added incrementally — first seeds (author-provided), then
    inferred positions produced by the graph propagation engine.
    """

    def __init__(self, genre_slug: str, setting_slug: str) -> None:
        self.genre_slug = genre_slug
        self.setting_slug = setting_slug
        self._positions: dict[str, AxisPosition] = {}

    # ------------------------------------------------------------------
    # Mutation
    # ------------------------------------------------------------------

    def set_seed(self, axis_slug: str, value: str) -> None:
        """Record an author-provided seed position (confidence = 1.0)."""
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
        """Record a propagated inferred position."""
        self._positions[axis_slug] = AxisPosition(
            axis_slug=axis_slug,
            value=value,
            confidence=confidence,
            source="inferred",
            justification=justification,
        )

    # ------------------------------------------------------------------
    # Query
    # ------------------------------------------------------------------

    def get(self, axis_slug: str) -> AxisPosition | None:
        """Return the AxisPosition for *axis_slug*, or None if not yet set."""
        return self._positions.get(axis_slug)

    def is_set(self, axis_slug: str) -> bool:
        """True when *axis_slug* has been positioned."""
        return axis_slug in self._positions

    def unset_axes(self, all_slugs: set[str]) -> set[str]:
        """Return slugs from *all_slugs* that have not yet been positioned."""
        return all_slugs - self._positions.keys()

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def positions(self) -> dict[str, AxisPosition]:
        """Shallow copy of the internal positions dict."""
        return dict(self._positions)

    @property
    def seed_count(self) -> int:
        """Number of seed (author-provided) positions."""
        return sum(1 for p in self._positions.values() if p.is_seed)

    @property
    def inferred_count(self) -> int:
        """Number of inferred (propagated) positions."""
        return sum(1 for p in self._positions.values() if not p.is_seed)

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_dict(self) -> dict[str, Any]:
        """Serialise to a plain dict suitable for JSON output."""
        return {
            "genre_slug": self.genre_slug,
            "setting_slug": self.setting_slug,
            "seed_count": self.seed_count,
            "inferred_count": self.inferred_count,
            "total_positions": len(self._positions),
            "positions": [p.to_dict() for p in self._positions.values()],
        }

    def save(self, path: Path) -> None:
        """Write JSON representation to *path*, creating parent directories."""
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2))


# ---------------------------------------------------------------------------
# Data loading helpers
# ---------------------------------------------------------------------------


def load_all_axes(data_path: Path) -> dict[str, dict[str, Any]]:
    """Load all Tome axis definitions from the domains directory.

    Args:
        data_path: Root of the storyteller-data checkout (i.e. the value of
            ``STORYTELLER_DATA_PATH``).  The function reads every ``*.json``
            file under ``{data_path}/narrative-data/tome/domains/``.

    Returns:
        A dict keyed by axis slug; values are the raw axis dicts from the
        source files (containing at least ``"slug"`` and ``"name"``).
    """
    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    axes: dict[str, dict[str, Any]] = {}
    for domain_file in sorted(domains_dir.glob("*.json")):
        data = json.loads(domain_file.read_text())
        for axis in data.get("axes", []):
            slug = axis["slug"]
            axes[slug] = axis
    return axes


def load_graph(data_path: Path) -> dict[str, Any]:
    """Load the Tome edge graph from edges.json.

    Args:
        data_path: Root of the storyteller-data checkout.

    Returns:
        The parsed contents of ``{data_path}/narrative-data/tome/edges.json``.
    """
    edges_path = data_path / "narrative-data" / "tome" / "edges.json"
    return json.loads(edges_path.read_text())
