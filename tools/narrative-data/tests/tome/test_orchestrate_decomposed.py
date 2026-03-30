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


# ---------------------------------------------------------------------------
# Tests: _build_compressed_prompt
# ---------------------------------------------------------------------------


class TestBuildCompressedPrompt:
    """Tests that compressed-mode prompts use the compressed preamble, not the full one."""

    def test_places_prompt_uses_compressed_preamble(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_compressed_prompt

        world_dir = data_path / "narrative-data" / "tome" / "worlds" / "test-world"
        world_pos = json.loads((world_dir / "world-position.json").read_text())
        compressed = "### Material Conditions\n- geography-climate: Isolated highland moor [seed]"

        prompt = _build_compressed_prompt("places", world_pos, compressed, data_path, world_dir)

        # The compressed preamble should appear in the prompt
        assert "Isolated highland moor [seed]" in prompt
        # Edge-trace data from world-position (confidence scores, preamble
        # section headers) must not leak through the compressed path.
        assert "(confidence: 0.85)" not in prompt
        assert "(confidence: 0.78)" not in prompt
        assert "Seed Positions (author-provided)" not in prompt
        assert "Inferred Positions (propagated from seeds)" not in prompt
        # The prompt should still have genre and setting
        assert "folk-horror" in prompt
        assert "mccallisters-barn" in prompt

    def test_places_prompt_no_inferred_justifications(self, data_path: Path) -> None:
        """Full preamble includes 'Inferred Positions' header; compressed does not."""
        from narrative_data.tome.orchestrate_decomposed import _build_compressed_prompt

        world_dir = data_path / "narrative-data" / "tome" / "worlds" / "test-world"
        world_pos = json.loads((world_dir / "world-position.json").read_text())
        compressed = "### Material Conditions\n- geography-climate: Isolated highland moor [seed]"

        prompt = _build_compressed_prompt("places", world_pos, compressed, data_path, world_dir)

        assert "Inferred Positions" not in prompt
        assert "Seed Positions" not in prompt

    def test_orgs_prompt_requires_places(self, data_path: Path) -> None:
        """Orgs prompt builder should fail if places.json does not exist."""
        from narrative_data.tome.orchestrate_decomposed import _build_compressed_prompt

        world_dir = data_path / "narrative-data" / "tome" / "worlds" / "test-world"
        world_pos = json.loads((world_dir / "world-position.json").read_text())

        with pytest.raises(FileNotFoundError, match="places.json"):
            _build_compressed_prompt("orgs", world_pos, "preamble", data_path, world_dir)

    def test_orgs_prompt_includes_places_context(self, data_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _build_compressed_prompt

        world_dir = data_path / "narrative-data" / "tome" / "worlds" / "test-world"
        world_pos = json.loads((world_dir / "world-position.json").read_text())

        # Create a places.json prerequisite
        places_data = {
            "places": [
                {
                    "name": "The Hollow",
                    "slug": "the-hollow",
                    "spatial_role": "threshold",
                    "description": "A sunken field.",
                }
            ]
        }
        (world_dir / "places.json").write_text(json.dumps(places_data))

        compressed = "### Material Conditions\n- geography-climate: moor [seed]"
        prompt = _build_compressed_prompt("orgs", world_pos, compressed, data_path, world_dir)

        assert "The Hollow" in prompt
        # Edge-trace data must not appear
        assert "(confidence: 0.85)" not in prompt
        assert "Seed Positions (author-provided)" not in prompt


# ---------------------------------------------------------------------------
# Tests: _parse_compressed_response
# ---------------------------------------------------------------------------


class TestParseCompressedResponse:
    def test_parses_array_stage(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _parse_compressed_response

        response = json.dumps([{"name": "The Hollow", "slug": "the-hollow"}])
        result = _parse_compressed_response("places", response)

        assert isinstance(result, list)
        assert len(result) == 1
        assert result[0]["name"] == "The Hollow"

    def test_parses_substrate_as_dict(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _parse_compressed_response

        response = json.dumps(
            {
                "clusters": [{"name": "Clan A", "slug": "clan-a"}],
                "relationships": [],
            }
        )
        result = _parse_compressed_response("substrate", response)

        assert isinstance(result, dict)
        assert len(result["clusters"]) == 1

    def test_raises_on_bad_json(self) -> None:
        from narrative_data.tome.orchestrate_decomposed import _parse_compressed_response

        with pytest.raises(ValueError):
            _parse_compressed_response("places", "not json at all {{{{")


# ---------------------------------------------------------------------------
# Tests: _save_compressed_output
# ---------------------------------------------------------------------------


class TestSaveCompressedOutput:
    def test_saves_stage_file_and_instance_files(self, tmp_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _save_compressed_output

        decomposed = tmp_path / "decomposed"
        entities = [
            {"name": "Place A", "slug": "place-a"},
            {"name": "Place B", "slug": "place-b"},
        ]

        path = _save_compressed_output(
            decomposed, "places", entities, "test-world", "folk-horror", "barn"
        )

        # Stage file
        assert path.exists()
        data = json.loads(path.read_text())
        assert data["pipeline"] == "compressed"
        assert data["count"] == 2
        assert len(data["places"]) == 2

        # Instance files
        inst_dir = decomposed / "fan-out" / "places"
        assert (inst_dir / "instance-000.json").exists()
        assert (inst_dir / "instance-001.json").exists()
        inst0 = json.loads((inst_dir / "instance-000.json").read_text())
        assert inst0["name"] == "Place A"

    def test_saves_substrate_with_relationships(self, tmp_path: Path) -> None:
        from narrative_data.tome.orchestrate_decomposed import _save_compressed_output

        decomposed = tmp_path / "decomposed"
        parsed = {
            "clusters": [{"name": "Clan A", "slug": "clan-a"}],
            "relationships": [{"cluster_a": "clan-a", "cluster_b": "clan-b"}],
        }

        path = _save_compressed_output(
            decomposed, "substrate", parsed, "test-world", "folk-horror", "barn"
        )

        data = json.loads(path.read_text())
        assert data["cluster_count"] == 1
        assert data["relationship_count"] == 1
        assert len(data["clusters"]) == 1
