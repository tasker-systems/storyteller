# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the graph propagation engine."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from narrative_data.tome.propagation import (
    _select_value,
    _select_value_enriched,
    build_incoming_index,
    propagate,
    score_candidates,
)
from narrative_data.tome.world_position import WorldPosition


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_test_data(tmp_path: Path) -> None:
    """Create 2 domains (material, economic) with 4 axes and 3 edges.

    Domains and axes:
    - material: geo (categorical: desert/temperate/arctic),
                resources (categorical: scarce/moderate/abundant)
    - economic: production (categorical: foraging/agrarian/industrial),
                trade (categorical: local/regional/global)

    Edges:
    - geo → resources (produces, w=0.9)
    - resources → production (produces, w=0.8)
    - production → trade (enables, w=0.6)
    """
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    domains_dir.mkdir(parents=True, exist_ok=True)

    material_domain = {
        "domain": {"slug": "material", "name": "Material Conditions"},
        "axes": [
            {
                "slug": "geo",
                "name": "Geography",
                "axis_type": "categorical",
                "values": ["desert", "temperate", "arctic"],
            },
            {
                "slug": "resources",
                "name": "Resource Profile",
                "axis_type": "categorical",
                "values": ["scarce", "moderate", "abundant"],
            },
        ],
    }
    economic_domain = {
        "domain": {"slug": "economic", "name": "Economic Forms"},
        "axes": [
            {
                "slug": "production",
                "name": "Production Mode",
                "axis_type": "categorical",
                "values": ["foraging", "agrarian", "industrial"],
            },
            {
                "slug": "trade",
                "name": "Trade Networks",
                "axis_type": "categorical",
                "values": ["local", "regional", "global"],
            },
        ],
    }

    (domains_dir / "material.json").write_text(json.dumps(material_domain))
    (domains_dir / "economic.json").write_text(json.dumps(economic_domain))

    edges_data = {
        "exported_at": "2026-01-01T00:00:00Z",
        "edge_count": 3,
        "edges": [
            {
                "from_axis": "geo",
                "to_axis": "resources",
                "edge_type": "produces",
                "weight": 0.9,
                "description": "Geography dictates resource availability.",
            },
            {
                "from_axis": "resources",
                "to_axis": "production",
                "edge_type": "produces",
                "weight": 0.8,
                "description": "Resources shape production mode.",
            },
            {
                "from_axis": "production",
                "to_axis": "trade",
                "edge_type": "enables",
                "weight": 0.6,
                "description": "Production surplus enables trade.",
            },
        ],
    }
    edges_path = tmp_path / "narrative-data" / "tome"
    (edges_path / "edges.json").write_text(json.dumps(edges_data))


# ---------------------------------------------------------------------------
# Tests: build_incoming_index
# ---------------------------------------------------------------------------


def test_build_incoming_index(tmp_path: Path) -> None:
    """build_incoming_index maps to_axis → list of incoming edges."""
    _make_test_data(tmp_path)

    graph_data = {
        "edges": [
            {"from_axis": "geo", "to_axis": "resources", "edge_type": "produces", "weight": 0.9},
            {"from_axis": "resources", "to_axis": "production", "edge_type": "produces", "weight": 0.8},
            {"from_axis": "production", "to_axis": "trade", "edge_type": "enables", "weight": 0.6},
        ]
    }

    index = build_incoming_index(graph_data)

    assert "resources" in index
    assert "production" in index
    assert "trade" in index
    # geo has no incoming edges in this graph
    assert "geo" not in index

    assert len(index["resources"]) == 1
    assert index["resources"][0]["from_axis"] == "geo"
    assert index["resources"][0]["edge_type"] == "produces"
    assert index["resources"][0]["weight"] == 0.9

    assert len(index["production"]) == 1
    assert index["production"][0]["from_axis"] == "resources"

    assert len(index["trade"]) == 1
    assert index["trade"][0]["from_axis"] == "production"
    assert index["trade"][0]["edge_type"] == "enables"


def test_build_incoming_index_multiple_incoming(tmp_path: Path) -> None:
    """Axes with multiple incoming edges accumulate all of them."""
    graph_data = {
        "edges": [
            {"from_axis": "a", "to_axis": "c", "edge_type": "produces", "weight": 0.9},
            {"from_axis": "b", "to_axis": "c", "edge_type": "constrains", "weight": 0.7},
        ]
    }

    index = build_incoming_index(graph_data)

    assert len(index["c"]) == 2
    from_axes = {e["from_axis"] for e in index["c"]}
    assert from_axes == {"a", "b"}


def test_build_incoming_index_empty_graph() -> None:
    """Empty edges list produces an empty index."""
    graph_data = {"edges": []}

    index = build_incoming_index(graph_data)

    assert index == {}


# ---------------------------------------------------------------------------
# Tests: score_candidates
# ---------------------------------------------------------------------------


def test_score_candidates_edge_type_multipliers() -> None:
    """produces > enables at the same weight; multipliers apply correctly."""
    produces_edge = {"from_axis": "geo", "to_axis": "resources", "edge_type": "produces", "weight": 1.0}
    enables_edge = {"from_axis": "geo", "to_axis": "resources", "edge_type": "enables", "weight": 1.0}

    set_positions = {"geo": "temperate"}

    produces_score = score_candidates([produces_edge], set_positions)
    enables_score = score_candidates([enables_edge], set_positions)

    # produces multiplier (1.0) > enables multiplier (0.6)
    assert produces_score > enables_score
    assert produces_score == pytest.approx(1.0)
    assert enables_score == pytest.approx(0.6)


def test_score_candidates_all_edge_types() -> None:
    """All four edge type multipliers produce expected scores at weight=1.0."""
    set_positions = {"src": "some-value"}

    for edge_type, expected in [
        ("produces", 1.0),
        ("constrains", 0.8),
        ("enables", 0.6),
        ("transforms", 0.0),
    ]:
        edge = {"from_axis": "src", "to_axis": "tgt", "edge_type": edge_type, "weight": 1.0}
        score = score_candidates([edge], set_positions)
        assert score == pytest.approx(expected), f"edge_type={edge_type}"


def test_score_candidates_unset_source_not_counted() -> None:
    """Edges whose from_axis is not in set_positions contribute zero."""
    edge = {"from_axis": "missing", "to_axis": "resources", "edge_type": "produces", "weight": 0.9}

    score = score_candidates([edge], set_positions={})

    assert score == pytest.approx(0.0)


def test_score_candidates_cumulative() -> None:
    """Multiple active incoming edges sum their scores."""
    edges = [
        {"from_axis": "a", "to_axis": "c", "edge_type": "produces", "weight": 0.5},
        {"from_axis": "b", "to_axis": "c", "edge_type": "constrains", "weight": 0.5},
    ]
    set_positions = {"a": "x", "b": "y"}

    score = score_candidates(edges, set_positions)

    # 0.5 * 1.0 + 0.5 * 0.8 = 0.5 + 0.4 = 0.9
    assert score == pytest.approx(0.9)


# ---------------------------------------------------------------------------
# Tests: propagate
# ---------------------------------------------------------------------------


def test_propagate_fills_all_axes(tmp_path: Path) -> None:
    """Starting from a single seed, propagate fills all 4 axes."""
    _make_test_data(tmp_path)

    wp = WorldPosition(genre_slug="folk-horror", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, tmp_path)

    all_axes = {"geo", "resources", "production", "trade"}
    for axis in all_axes:
        assert result.is_set(axis), f"axis {axis!r} should be set after propagation"


def test_propagate_respects_seeds(tmp_path: Path) -> None:
    """Seed values are preserved; propagate does not overwrite them."""
    _make_test_data(tmp_path)

    wp = WorldPosition(genre_slug="folk-horror", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")
    wp.set_seed("resources", "scarce")

    result = propagate(wp, tmp_path)

    geo_pos = result.get("geo")
    resources_pos = result.get("resources")

    assert geo_pos is not None and geo_pos.value == "temperate"
    assert geo_pos.source == "seed"
    assert resources_pos is not None and resources_pos.value == "scarce"
    assert resources_pos.source == "seed"


def test_propagate_inferred_values_are_valid(tmp_path: Path) -> None:
    """Inferred values are drawn from the axis's valid values list."""
    _make_test_data(tmp_path)

    wp = WorldPosition(genre_slug="folk-horror", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, tmp_path)

    resources_pos = result.get("resources")
    assert resources_pos is not None
    assert resources_pos.value in ("scarce", "moderate", "abundant")
    assert resources_pos.source == "inferred"

    production_pos = result.get("production")
    assert production_pos is not None
    assert production_pos.value in ("foraging", "agrarian", "industrial")

    trade_pos = result.get("trade")
    assert trade_pos is not None
    assert trade_pos.value in ("local", "regional", "global")


def test_propagate_records_justification(tmp_path: Path) -> None:
    """Inferred axes have a justification string mentioning source axes."""
    _make_test_data(tmp_path)

    wp = WorldPosition(genre_slug="folk-horror", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, tmp_path)

    resources_pos = result.get("resources")
    assert resources_pos is not None
    assert resources_pos.justification is not None
    # Justification should reference the source axis
    assert "geo" in resources_pos.justification


def test_propagate_unreachable_axis_filled_low_confidence(tmp_path: Path) -> None:
    """Axes with no reachable incoming edges are filled with confidence 0.1."""
    _make_test_data(tmp_path)

    # Add an isolated axis that no edge points to
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    isolated_domain = {
        "domain": {"slug": "isolated", "name": "Isolated Domain"},
        "axes": [
            {
                "slug": "isolated-axis",
                "name": "Isolated",
                "axis_type": "categorical",
                "values": ["alpha", "beta"],
            }
        ],
    }
    (domains_dir / "isolated.json").write_text(json.dumps(isolated_domain))

    wp = WorldPosition(genre_slug="folk-horror", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, tmp_path)

    isolated_pos = result.get("isolated-axis")
    assert isolated_pos is not None
    assert isolated_pos.confidence == pytest.approx(0.1)
    assert isolated_pos.value in ("alpha", "beta")


def test_propagate_returns_same_world_position(tmp_path: Path) -> None:
    """propagate returns the WorldPosition object passed in (mutates in-place)."""
    _make_test_data(tmp_path)

    wp = WorldPosition(genre_slug="folk-horror", setting_slug="test-setting")
    wp.set_seed("geo", "temperate")

    result = propagate(wp, tmp_path)

    assert result is wp


def test_propagate_bipolar_axis(tmp_path: Path) -> None:
    """Bipolar axes pick a value from [low, mid, high]."""
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    domains_dir.mkdir(parents=True, exist_ok=True)

    domain_data = {
        "domain": {"slug": "social", "name": "Social"},
        "axes": [
            {
                "slug": "source-ax",
                "name": "Source",
                "axis_type": "categorical",
                "values": ["x"],
            },
            {
                "slug": "bipolar-ax",
                "name": "Bipolar",
                "axis_type": "bipolar",
                "values": {"low_label": "isolated", "high_label": "connected"},
            },
        ],
    }
    (domains_dir / "social.json").write_text(json.dumps(domain_data))

    edges_data = {
        "edges": [
            {
                "from_axis": "source-ax",
                "to_axis": "bipolar-ax",
                "edge_type": "produces",
                "weight": 0.8,
            }
        ]
    }
    edges_path = tmp_path / "narrative-data" / "tome"
    edges_path.mkdir(parents=True, exist_ok=True)
    (edges_path / "edges.json").write_text(json.dumps(edges_data))

    wp = WorldPosition(genre_slug="test", setting_slug="test")
    wp.set_seed("source-ax", "x")

    result = propagate(wp, tmp_path)

    bipolar_pos = result.get("bipolar-ax")
    assert bipolar_pos is not None
    assert bipolar_pos.value in ("low", "mid", "high")


def test_propagate_ordinal_axis(tmp_path: Path) -> None:
    """Ordinal axes pick from their values list just like categorical."""
    domains_dir = tmp_path / "narrative-data" / "tome" / "domains"
    domains_dir.mkdir(parents=True, exist_ok=True)

    domain_data = {
        "domain": {"slug": "power", "name": "Power"},
        "axes": [
            {
                "slug": "root-ax",
                "name": "Root",
                "axis_type": "categorical",
                "values": ["present"],
            },
            {
                "slug": "ordinal-ax",
                "name": "Hierarchy",
                "axis_type": "ordinal",
                "values": ["flat", "moderate", "steep"],
            },
        ],
    }
    (domains_dir / "power.json").write_text(json.dumps(domain_data))

    edges_data = {
        "edges": [
            {
                "from_axis": "root-ax",
                "to_axis": "ordinal-ax",
                "edge_type": "constrains",
                "weight": 0.7,
            }
        ]
    }
    edges_path = tmp_path / "narrative-data" / "tome"
    edges_path.mkdir(parents=True, exist_ok=True)
    (edges_path / "edges.json").write_text(json.dumps(edges_data))

    wp = WorldPosition(genre_slug="test", setting_slug="test")
    wp.set_seed("root-ax", "present")

    result = propagate(wp, tmp_path)

    ordinal_pos = result.get("ordinal-ax")
    assert ordinal_pos is not None
    assert ordinal_pos.value in ("flat", "moderate", "steep")


# ---------------------------------------------------------------------------
# Tests: _select_value_enriched
# ---------------------------------------------------------------------------


def test_select_value_enriched_fallback() -> None:
    """When the OllamaClient raises, _select_value_enriched falls back without crashing."""
    axis = {
        "slug": "geo",
        "name": "Geography",
        "axis_type": "categorical",
        "values": ["desert", "temperate", "arctic"],
    }
    incoming_edges = [
        {
            "from_axis": "climate",
            "to_axis": "geo",
            "edge_type": "produces",
            "weight": 0.9,
            "description": "Climate shapes geography.",
        }
    ]
    set_positions = {"climate": "cold"}

    # Mock client that always raises
    failing_client = MagicMock()
    failing_client.generate_structured.side_effect = RuntimeError("Ollama is unavailable")

    result = _select_value_enriched(
        axis=axis,
        incoming_edges=incoming_edges,
        set_positions=set_positions,
        genre_slug="folk-horror",
        setting_slug="rural-isolation",
        client=failing_client,
    )

    # Should not crash and should return a valid value from the axis
    assert result in ("desert", "temperate", "arctic")
    # The LLM call was attempted exactly once (no retry in _select_value_enriched itself)
    failing_client.generate_structured.assert_called_once()


def test_select_value_enriched_uses_weights() -> None:
    """When the LLM returns valid weighted values, sampling uses those weights."""
    axis = {
        "slug": "resources",
        "name": "Resource Profile",
        "axis_type": "categorical",
        "values": ["scarce", "moderate", "abundant"],
    }
    incoming_edges: list[dict] = []
    set_positions: dict[str, str] = {}

    # Mock client that heavily weights "scarce"
    mock_client = MagicMock()
    mock_client.generate_structured.return_value = [
        {"value": "scarce", "weight": 0.99},
        {"value": "moderate", "weight": 0.005},
        {"value": "abundant", "weight": 0.005},
    ]

    # Run many times; with weight 0.99 on "scarce" it should always win
    results = {
        _select_value_enriched(
            axis=axis,
            incoming_edges=incoming_edges,
            set_positions=set_positions,
            genre_slug="folk-horror",
            setting_slug="test",
            client=mock_client,
        )
        for _ in range(20)
    }

    assert "scarce" in results
    # All results must be valid axis values
    for r in results:
        assert r in ("scarce", "moderate", "abundant")
