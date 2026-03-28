# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for mundane character (Q1-Q2) elicitation module."""

import json
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def world_dir(tmp_path: Path) -> Path:
    """Create a minimal world directory with all prerequisite files."""
    world = tmp_path / "narrative-data" / "tome" / "worlds" / "test-world"
    world.mkdir(parents=True)

    world_pos = {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "seed_count": 2,
        "inferred_count": 1,
        "total_positions": 3,
        "positions": [
            {"axis_slug": "kinship-system", "value": "clan-tribal", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "social-stratification", "value": "caste-hereditary", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "community-cohesion", "value": "high", "confidence": 0.8, "source": "inferred"},
        ],
    }
    (world / "world-position.json").write_text(json.dumps(world_pos))

    places = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "places": [
            {"slug": "the-market", "name": "The Market", "tier": 2, "place_type": "gathering-place", "description": "A dusty market.", "spatial_role": "center"},
        ],
    }
    (world / "places.json").write_text(json.dumps(places))

    orgs = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "organizations": [
            {"slug": "parish-council", "name": "Parish Council", "tier": 2, "org_type": "governance", "description": "Local governance."},
        ],
    }
    (world / "organizations.json").write_text(json.dumps(orgs))

    substrate = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "clusters": [
            {"slug": "the-morrows", "name": "The Morrows", "basis": "blood", "hierarchy_position": "dominant"},
            {"slug": "the-hallodays", "name": "The Hallodays", "basis": "blood", "hierarchy_position": "established"},
        ],
        "relationships": [
            {"cluster_a": "the-morrows", "cluster_b": "the-hallodays", "type": "intermarriage-with-tension", "boundary_tension": "Land flows through Morrow blood but Halloday labor works it."},
        ],
    }
    (world / "social-substrate.json").write_text(json.dumps(substrate))

    return world


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestLoadSocialSubstrate:
    def test_loads_substrate(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import _load_social_substrate

        substrate = _load_social_substrate(world_dir)
        assert "clusters" in substrate
        assert len(substrate["clusters"]) == 2

    def test_raises_when_missing(self, tmp_path: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import _load_social_substrate

        empty = tmp_path / "empty"
        empty.mkdir()
        with pytest.raises(FileNotFoundError, match="social-substrate.json"):
            _load_social_substrate(empty)


class TestBuildSubstrateContext:
    def test_formats_clusters_and_relationships(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import (
            _build_social_substrate_context,
            _load_social_substrate,
        )

        substrate = _load_social_substrate(world_dir)
        ctx = _build_social_substrate_context(substrate)
        assert "The Morrows" in ctx
        assert "The Hallodays" in ctx
        assert "intermarriage" in ctx
        assert "Land flows" in ctx


class TestBuildMundanePrompt:
    def test_substitutes_all_placeholders(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import (
            _build_prompt,
            _build_social_substrate_context,
            _load_social_substrate,
        )
        from narrative_data.tome.elicit_places import (
            _build_genre_profile_summary,
            _build_world_preamble,
            _load_world_position,
        )
        from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
        from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs

        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)

        template = (
            "{genre_slug} {setting_slug} {world_preamble} "
            "{genre_profile_summary} {places_context} {orgs_context} "
            "{social_substrate_context}"
        )
        prompt = _build_prompt(
            template=template,
            world_pos=world_pos,
            genre_profile=None,
            places=places,
            orgs=orgs,
            substrate=substrate,
            settings_context="",
        )
        assert "folk-horror" in prompt
        assert "{" not in prompt


class TestParseMundaneResponse:
    def test_parses_valid_array(self) -> None:
        from narrative_data.tome.elicit_characters_mundane import _parse_characters_response

        response = json.dumps([
            {"centrality": "Q1", "slug": "elda", "name": "Elda"},
            {"centrality": "Q2", "slug": "gareth", "name": "Gareth"},
        ])
        result = _parse_characters_response(response)
        assert len(result) == 2

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.elicit_characters_mundane import _parse_characters_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_characters_response("not json")
