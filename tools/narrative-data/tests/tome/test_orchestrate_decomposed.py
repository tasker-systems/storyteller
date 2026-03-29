# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the decomposed pipeline orchestrator."""

import json
from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def data_path(tmp_path: Path) -> Path:
    """Create a minimal data_path with domains, world-position, and discovery dirs.

    Provides:
    - 2 domain files: material-conditions (geography-climate, resource-profile)
      and social-forms (kinship-system, community-cohesion)
    - world-position.json with positions for those axes
    - Empty archetypes and settings discovery dirs
    """
    nd = tmp_path / "narrative-data"
    nd.mkdir()

    # --- Domains ---
    domains_dir = nd / "domains"
    domains_dir.mkdir()
    (domains_dir / "material-conditions.json").write_text(
        json.dumps(
            {
                "domain": {"slug": "material-conditions", "name": "Material Conditions"},
                "axes": [
                    {"slug": "geography-climate", "name": "Geography & Climate"},
                    {"slug": "resource-profile", "name": "Resource Profile"},
                ],
            }
        )
    )
    (domains_dir / "social-forms.json").write_text(
        json.dumps(
            {
                "domain": {
                    "slug": "social-forms",
                    "name": "Social Forms of Production and Reproduction",
                },
                "axes": [
                    {"slug": "kinship-system", "name": "Kinship System"},
                    {"slug": "community-cohesion", "name": "Community Cohesion"},
                ],
            }
        )
    )

    # --- World directory ---
    world_dir = nd / "tome" / "worlds" / "test-world"
    world_dir.mkdir(parents=True)

    (world_dir / "world-position.json").write_text(
        json.dumps(
            {
                "genre_slug": "folk-horror",
                "setting_slug": "mccallisters-barn",
                "total_positions": 4,
                "genre_profile": {"world_affordances": {"scale": "local"}},
                "positions": [
                    {
                        "axis_slug": "geography-climate",
                        "value": "Isolated highland moor",
                        "source": "seed",
                    },
                    {
                        "axis_slug": "resource-profile",
                        "value": "Subsistence pastoral",
                        "source": "inferred",
                        "confidence": 0.85,
                    },
                    {
                        "axis_slug": "kinship-system",
                        "value": "Extended clan",
                        "source": "inferred",
                        "confidence": 0.78,
                    },
                    {
                        "axis_slug": "community-cohesion",
                        "value": "High internal, suspicious of outsiders",
                        "source": "seed",
                    },
                ],
            }
        )
    )

    # --- Empty discovery dirs ---
    (nd / "discovery" / "archetypes" / "folk-horror").mkdir(parents=True)
    (nd / "discovery" / "settings").mkdir(parents=True)
    (nd / "discovery" / "archetype-dynamics" / "folk-horror").mkdir(parents=True)

    return tmp_path


# ---------------------------------------------------------------------------
# Tests: _ensure_decomposed_dir
# ---------------------------------------------------------------------------


class TestEnsureDecomposedDir:
    def test_creates_expected_directories(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _ensure_decomposed_dir

        result = _ensure_decomposed_dir(data_path, "test-world")

        assert result.exists()
        assert result.name == "decomposed"
        # fan-out subdirs for all 5 stages
        fan_out = result / "fan-out"
        assert fan_out.exists()
        for stage in [
            "places",
            "orgs",
            "substrate",
            "characters-mundane",
            "characters-significant",
        ]:
            assert (fan_out / stage).exists(), f"Missing fan-out/{stage}"

    def test_returns_decomposed_path(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _ensure_decomposed_dir

        result = _ensure_decomposed_dir(data_path, "test-world")

        expected = data_path / "narrative-data" / "tome" / "worlds" / "test-world" / "decomposed"
        assert result == expected

    def test_idempotent(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _ensure_decomposed_dir

        _ensure_decomposed_dir(data_path, "test-world")
        result = _ensure_decomposed_dir(data_path, "test-world")

        assert result.exists()


# ---------------------------------------------------------------------------
# Tests: _build_world_summary_from_path
# ---------------------------------------------------------------------------


class TestBuildWorldSummaryFromPath:
    def test_produces_summary_with_compressed_preamble(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import (
            _build_world_summary_from_path,
        )

        summary = _build_world_summary_from_path(data_path, "test-world")

        assert summary["genre_slug"] == "folk-horror"
        assert summary["setting_slug"] == "mccallisters-barn"
        assert "compressed_preamble" in summary
        assert len(summary["compressed_preamble"]) > 0
        assert summary["axis_count"] == 4
        assert summary["seed_count"] == 2

    def test_no_edge_traces_in_preamble(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import (
            _build_world_summary_from_path,
        )

        summary = _build_world_summary_from_path(data_path, "test-world")

        preamble = summary["compressed_preamble"]
        # Edge traces (justification/confidence) should be stripped
        assert "confidence" not in preamble
        assert "0.85" not in preamble

    def test_includes_genre_profile(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import (
            _build_world_summary_from_path,
        )

        summary = _build_world_summary_from_path(data_path, "test-world")

        assert "genre_profile" in summary
        assert summary["genre_profile"]["world_affordances"]["scale"] == "local"


# ---------------------------------------------------------------------------
# Tests: _build_place_specs
# ---------------------------------------------------------------------------


class TestBuildPlaceSpecs:
    def test_generates_correct_number_of_specs(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_place_specs

        plan = {
            "places": {
                "count": 3,
                "distribution": {
                    "dwelling": 1,
                    "gathering-place": 1,
                    "threshold": 1,
                },
            },
        }

        specs = _build_place_specs(
            plan,
            "### Material Conditions\n- geography-climate: Highland moor [seed]",
            "folk-horror",
            "mccallisters-barn",
        )

        assert len(specs) == 3

    def test_correct_model_role_and_template(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_place_specs

        plan = {
            "places": {
                "count": 1,
                "distribution": {"dwelling": 1},
            },
        }

        specs = _build_place_specs(plan, "preamble", "folk-horror", "mccallisters-barn")

        assert specs[0].model_role == "fan_out_structured"
        assert specs[0].template_name == "place-fanout.md"
        assert specs[0].stage == "places"

    def test_context_includes_genre_and_axes(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_place_specs

        plan = {
            "places": {
                "count": 1,
                "distribution": {"dwelling": 1},
            },
        }

        specs = _build_place_specs(
            plan,
            "### Material Conditions\n- geography-climate: Highland moor [seed]",
            "folk-horror",
            "mccallisters-barn",
        )

        ctx = specs[0].context
        assert ctx["genre_slug"] == "folk-horror"
        assert ctx["setting_slug"] == "mccallisters-barn"
        assert ctx["place_type"] == "dwelling"
        assert "axes_subset" in ctx

    def test_sequential_indices(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_place_specs

        plan = {
            "places": {
                "count": 3,
                "distribution": {"dwelling": 1, "path": 1, "ruin": 1},
            },
        }

        specs = _build_place_specs(plan, "preamble", "folk-horror", "barn")

        indices = [s.index for s in specs]
        assert indices == [0, 1, 2]


# ---------------------------------------------------------------------------
# Tests: _build_significant_character_specs
# ---------------------------------------------------------------------------


class TestBuildSignificantCharacterSpecs:
    def test_generates_q3_plus_q4_specs(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import (
            _build_significant_character_specs,
        )

        plan = {
            "characters_significant": {
                "q3_count": 2,
                "q4_count": 1,
            },
        }

        clusters = [
            {"slug": "old-guard", "name": "Old Guard"},
            {"slug": "newcomers", "name": "Newcomers"},
        ]

        specs = _build_significant_character_specs(
            plan, "preamble", "folk-horror", "barn", clusters, "archetype summary"
        )

        assert len(specs) == 3  # 2 Q3 + 1 Q4

    def test_uses_fan_out_creative_model(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import (
            _build_significant_character_specs,
        )

        plan = {"characters_significant": {"q3_count": 1, "q4_count": 0}}
        clusters = [{"slug": "c1", "name": "C1"}]

        specs = _build_significant_character_specs(
            plan, "preamble", "folk-horror", "barn", clusters, ""
        )

        assert specs[0].model_role == "fan_out_creative"

    def test_assigns_boundary_positions_by_cycling(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import (
            _build_significant_character_specs,
        )

        plan = {"characters_significant": {"q3_count": 3, "q4_count": 0}}
        clusters = [
            {"slug": "a", "name": "A"},
            {"slug": "b", "name": "B"},
            {"slug": "c", "name": "C"},
        ]

        specs = _build_significant_character_specs(
            plan, "preamble", "folk-horror", "barn", clusters, ""
        )

        # Each spec should have a boundary_position with two cluster slugs
        for spec in specs:
            assert "boundary_position" in spec.context
