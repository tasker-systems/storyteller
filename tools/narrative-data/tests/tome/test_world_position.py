# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for WorldPosition data model."""

import json
from pathlib import Path

import pytest

from narrative_data.tome.world_position import (
    AxisPosition,
    WorldPosition,
    load_all_axes,
    load_graph,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_domain_file(domains_dir: Path, domain_slug: str, axes: list[dict]) -> None:
    """Write a minimal domain JSON file into the domains directory."""
    domains_dir.mkdir(parents=True, exist_ok=True)
    data = {
        "domain": {"slug": domain_slug, "name": domain_slug.replace("-", " ").title()},
        "axes": axes,
    }
    (domains_dir / f"{domain_slug}.json").write_text(json.dumps(data))


def _make_edges_file(data_path: Path, edges: dict) -> None:
    """Write a minimal edges JSON file."""
    edges_path = data_path / "narrative-data" / "tome"
    edges_path.mkdir(parents=True, exist_ok=True)
    (edges_path / "edges.json").write_text(json.dumps(edges))


# ---------------------------------------------------------------------------
# Tests: load_all_axes
# ---------------------------------------------------------------------------


def test_load_all_axes(tmp_path: Path) -> None:
    """Axes from all domain files are loaded and keyed by slug."""
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    _make_domain_file(
        domains_dir,
        "social",
        [
            {"slug": "power-distance", "name": "Power Distance"},
            {"slug": "collectivism", "name": "Collectivism vs Individualism"},
        ],
    )
    _make_domain_file(
        domains_dir,
        "temporal",
        [
            {"slug": "time-orientation", "name": "Time Orientation"},
        ],
    )

    axes = load_all_axes(tmp_path)

    assert "power-distance" in axes
    assert "collectivism" in axes
    assert "time-orientation" in axes
    assert axes["power-distance"]["name"] == "Power Distance"
    assert axes["time-orientation"]["name"] == "Time Orientation"


def test_load_all_axes_empty_domains_dir(tmp_path: Path) -> None:
    """Returns empty dict when domains directory has no JSON files."""
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    domains_dir.mkdir(parents=True)

    axes = load_all_axes(tmp_path)

    assert axes == {}


# ---------------------------------------------------------------------------
# Tests: load_graph
# ---------------------------------------------------------------------------


def test_load_graph(tmp_path: Path) -> None:
    """Graph edges file is read and returned as a dict."""
    edges_data = {
        "edges": [
            {"from": "power-distance", "to": "collectivism", "weight": 0.7},
        ],
        "meta": {"version": "1.0"},
    }
    _make_edges_file(tmp_path, edges_data)

    graph = load_graph(tmp_path)

    assert "edges" in graph
    assert len(graph["edges"]) == 1
    assert graph["edges"][0]["from"] == "power-distance"
    assert graph["meta"]["version"] == "1.0"


# ---------------------------------------------------------------------------
# Tests: AxisPosition
# ---------------------------------------------------------------------------


def test_axis_position_creation_seed() -> None:
    """Seed AxisPosition has correct fields and is_seed returns True."""
    pos = AxisPosition(
        axis_slug="power-distance",
        value="high",
        confidence=1.0,
        source="seed",
    )

    assert pos.axis_slug == "power-distance"
    assert pos.value == "high"
    assert pos.confidence == 1.0
    assert pos.source == "seed"
    assert pos.justification is None
    assert pos.is_seed is True


def test_axis_position_creation_inferred() -> None:
    """Inferred AxisPosition has correct fields and is_seed returns False."""
    pos = AxisPosition(
        axis_slug="collectivism",
        value="collective",
        confidence=0.8,
        source="inferred",
        justification="Strong community bonds noted in setting description.",
    )

    assert pos.is_seed is False
    assert pos.justification == "Strong community bonds noted in setting description."


def test_axis_position_to_dict_seed() -> None:
    """to_dict omits justification when it is None."""
    pos = AxisPosition(
        axis_slug="power-distance",
        value="high",
        confidence=1.0,
        source="seed",
    )

    d = pos.to_dict()

    assert d["axis_slug"] == "power-distance"
    assert d["value"] == "high"
    assert d["confidence"] == 1.0
    assert d["source"] == "seed"
    assert "justification" not in d


def test_axis_position_to_dict_inferred() -> None:
    """to_dict includes justification when set."""
    pos = AxisPosition(
        axis_slug="collectivism",
        value="collective",
        confidence=0.8,
        source="inferred",
        justification="Noted.",
    )

    d = pos.to_dict()

    assert d["justification"] == "Noted."


# ---------------------------------------------------------------------------
# Tests: WorldPosition
# ---------------------------------------------------------------------------


def test_world_position_set_and_get() -> None:
    """Seed axis can be set and retrieved."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_seed("power-distance", "high")

    pos = wp.get("power-distance")

    assert pos is not None
    assert pos.value == "high"
    assert pos.source == "seed"
    assert pos.confidence == 1.0
    assert wp.is_set("power-distance") is True


def test_world_position_set_inferred() -> None:
    """Inferred axis can be set and retrieved."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_inferred("collectivism", "collective", 0.75, "Community rites observed.")

    pos = wp.get("collectivism")

    assert pos is not None
    assert pos.value == "collective"
    assert pos.confidence == 0.75
    assert pos.source == "inferred"
    assert pos.justification == "Community rites observed."


def test_world_position_get_missing_returns_none() -> None:
    """get returns None for axes not yet set."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")

    assert wp.get("unknown-axis") is None
    assert wp.is_set("unknown-axis") is False


def test_world_position_unset_axes() -> None:
    """unset_axes returns slugs from all_slugs that have not been set."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_seed("power-distance", "high")
    wp.set_seed("collectivism", "collective")

    all_slugs = {"power-distance", "collectivism", "time-orientation", "uncertainty"}
    unset = wp.unset_axes(all_slugs)

    assert unset == {"time-orientation", "uncertainty"}


def test_world_position_counts() -> None:
    """seed_count and inferred_count track correctly."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_seed("power-distance", "high")
    wp.set_seed("collectivism", "collective")
    wp.set_inferred("time-orientation", "present", 0.7, "Seasonal rites.")

    assert wp.seed_count == 2
    assert wp.inferred_count == 1


def test_world_position_to_dict() -> None:
    """to_dict returns expected structure."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_seed("power-distance", "high")
    wp.set_inferred("collectivism", "collective", 0.8, "Noted.")

    d = wp.to_dict()

    assert d["genre_slug"] == "folk-horror"
    assert d["setting_slug"] == "mcallisters-barn"
    assert d["seed_count"] == 1
    assert d["inferred_count"] == 1
    assert d["total_positions"] == 2
    assert isinstance(d["positions"], list)
    assert len(d["positions"]) == 2


def test_world_position_positions_property_is_copy() -> None:
    """positions property returns a copy so mutations don't affect internal state."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_seed("power-distance", "high")

    positions = wp.positions
    positions["injected"] = "should-not-persist"  # type: ignore[assignment]

    assert "injected" not in wp.positions


def test_world_position_save(tmp_path: Path) -> None:
    """save writes a valid JSON file to the given path."""
    wp = WorldPosition(genre_slug="folk-horror", setting_slug="mcallisters-barn")
    wp.set_seed("power-distance", "high")

    out_path = tmp_path / "worlds" / "folk-horror" / "mcallisters-barn.json"
    wp.save(out_path)

    assert out_path.exists()
    loaded = json.loads(out_path.read_text())
    assert loaded["genre_slug"] == "folk-horror"
    assert loaded["setting_slug"] == "mcallisters-barn"
    assert loaded["total_positions"] == 1
