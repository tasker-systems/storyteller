# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Graph propagation engine for Tome world-position filling.

Starting from a set of author-provided seed axis positions, propagates
values to the remaining axes by following the Tome mutual-production graph.
The most-determined axis (highest combined incoming edge score from already-set
axes) is filled at each step, iterating until all axes are positioned.
"""

from __future__ import annotations

import random
from pathlib import Path
from typing import Any

from narrative_data.tome.world_position import WorldPosition, load_all_axes, load_graph


# ---------------------------------------------------------------------------
# Edge type multipliers
# ---------------------------------------------------------------------------

_EDGE_TYPE_MULTIPLIERS: dict[str, float] = {
    "produces": 1.0,
    "constrains": 0.8,
    "enables": 0.6,
    "transforms": 0.0,
}


# ---------------------------------------------------------------------------
# build_incoming_index
# ---------------------------------------------------------------------------


def build_incoming_index(graph_data: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    """Build a reverse lookup: to_axis → list of incoming edges.

    Args:
        graph_data: The dict returned by :func:`load_graph`.  Must contain
            an ``"edges"`` key whose value is a list of edge dicts.  Each
            edge dict must have at least ``"from_axis"`` and ``"to_axis"``.

    Returns:
        A dict mapping each target axis slug to the list of all edges that
        point *to* it.  Axes that appear only as sources are absent from the
        index.
    """
    index: dict[str, list[dict[str, Any]]] = {}
    for edge in graph_data.get("edges", []):
        to_axis = edge["to_axis"]
        if to_axis not in index:
            index[to_axis] = []
        index[to_axis].append(edge)
    return index


# ---------------------------------------------------------------------------
# score_candidates
# ---------------------------------------------------------------------------


def score_candidates(
    incoming_edges: list[dict[str, Any]],
    set_positions: dict[str, str],
) -> float:
    """Compute a cumulative determination score for a target axis.

    For each incoming edge whose ``from_axis`` already has a position in
    ``set_positions``, the contribution is ``weight * edge_type_multiplier``.
    Edges whose source axis is not yet set contribute zero.

    Args:
        incoming_edges: All edges that point to the target axis (as produced
            by :func:`build_incoming_index`).
        set_positions: Mapping of already-set axis slugs to their current
            string values.

    Returns:
        The cumulative float score (sum of all active edge contributions).
    """
    total = 0.0
    for edge in incoming_edges:
        if edge["from_axis"] not in set_positions:
            continue
        weight: float = edge.get("weight", 1.0)
        multiplier = _EDGE_TYPE_MULTIPLIERS.get(edge.get("edge_type", ""), 0.0)
        total += weight * multiplier
    return total


# ---------------------------------------------------------------------------
# Value selection helpers
# ---------------------------------------------------------------------------


def _select_value(axis: dict[str, Any]) -> str:
    """Select a random valid value for the given axis definition.

    Handles three families of axis type:

    - ``categorical``, ``ordinal``, ``set``: ``values`` is a list of strings.
    - ``bipolar``: ``values`` is a dict with ``low_label`` / ``high_label``;
      we pick from the abstract positions ``["low", "mid", "high"]``.
    - ``profile``: ``values`` has ``sub_dimensions`` and optional ``levels``;
      we join one sub-dimension with one level per chosen sub-dimension.

    Falls back to ``"unknown"`` for any unrecognised shape.
    """
    axis_type: str = axis.get("axis_type", "categorical")
    values = axis.get("values", [])

    if axis_type in ("categorical", "ordinal", "set"):
        if isinstance(values, list) and values:
            return random.choice(values)
        return "unknown"

    if axis_type == "bipolar":
        return random.choice(["low", "mid", "high"])

    if axis_type == "profile":
        # values is expected to have sub_dimensions list and optional levels list
        if isinstance(values, dict):
            sub_dims: list[str] = values.get("sub_dimensions", [])
            levels: list[str] = values.get("levels", ["low", "moderate", "high"])
            if sub_dims:
                chosen_dim = random.choice(sub_dims)
                chosen_level = random.choice(levels)
                return f"{chosen_dim}:{chosen_level}"
        return "unknown"

    # Unknown axis type — try list fallback, then give up
    if isinstance(values, list) and values:
        return random.choice(values)
    return "unknown"


def _build_justification(
    active_edges: list[dict[str, Any]],
) -> str:
    """Build a human-readable justification string from active incoming edges.

    Produces text of the form::

        geo →produces→ resources (w=0.9); resources →enables→ trade (w=0.6)

    Args:
        active_edges: The subset of incoming edges whose ``from_axis`` was
            already set when this axis was filled.

    Returns:
        A semicolon-separated string summarising all contributing edges.
    """
    parts: list[str] = []
    for edge in active_edges:
        from_axis = edge.get("from_axis", "?")
        to_axis = edge.get("to_axis", "?")
        edge_type = edge.get("edge_type", "?")
        weight = edge.get("weight", 0.0)
        parts.append(f"{from_axis} →{edge_type}→ {to_axis} (w={weight})")
    return "; ".join(parts)


# ---------------------------------------------------------------------------
# propagate
# ---------------------------------------------------------------------------


def propagate(world_position: WorldPosition, data_path: Path) -> WorldPosition:
    """Fill all axis positions in *world_position* via graph propagation.

    Algorithm:

    1. Load all axes and the edge graph from *data_path*.
    2. Build an incoming-edge index.
    3. Iteratively find the unset axis with the highest determination score
       (score > 0), fill it with a random valid value, and record the
       justification.
    4. Once no more axes can be reached via graph edges, fill remaining unset
       axes with a random value at confidence 0.1 ("unreachable" fill).

    Seed positions (source == "seed") are never overwritten.

    Args:
        world_position: A :class:`~narrative_data.tome.world_position.WorldPosition`
            with at least one seed position set.
        data_path: Root of the storyteller-data checkout.

    Returns:
        The same *world_position* object, now fully populated.
    """
    all_axes = load_all_axes(data_path)
    graph_data = load_graph(data_path)
    incoming_index = build_incoming_index(graph_data)
    all_slugs: set[str] = set(all_axes.keys())

    # Phase 1: iterative propagation — fill axes reachable from set positions
    while True:
        unset = world_position.unset_axes(all_slugs)
        if not unset:
            break

        # Build current set_positions map for score computation
        set_positions: dict[str, str] = {
            slug: pos.value for slug, pos in world_position.positions.items()
        }

        # Score every unset axis that has at least one incoming edge from a set axis
        best_slug: str | None = None
        best_score: float = 0.0
        best_active_edges: list[dict[str, Any]] = []

        for slug in unset:
            edges_in = incoming_index.get(slug, [])
            if not edges_in:
                continue
            score = score_candidates(edges_in, set_positions)
            if score > best_score:
                best_score = score
                best_slug = slug
                best_active_edges = [
                    e for e in edges_in if e.get("from_axis") in set_positions
                ]

        if best_slug is None or best_score == 0.0:
            # No more axes are reachable through the graph from current set positions
            break

        axis_def = all_axes.get(best_slug, {})
        value = _select_value(axis_def)
        confidence = min(best_score, 1.0)
        justification = _build_justification(best_active_edges)

        world_position.set_inferred(best_slug, value, confidence, justification)

    # Phase 2: fill unreachable axes at low confidence
    remaining = world_position.unset_axes(all_slugs)
    for slug in remaining:
        axis_def = all_axes.get(slug, {})
        value = _select_value(axis_def)
        world_position.set_inferred(
            slug,
            value,
            confidence=0.1,
            justification="unreachable: no incoming edges from set axes",
        )

    return world_position
