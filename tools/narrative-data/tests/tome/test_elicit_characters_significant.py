# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for significant character (Q3-Q4) elicitation module."""

import json
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def data_root(tmp_path: Path) -> Path:
    """Create a full data directory with discovery corpus and world data."""
    # World directory
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

    places = {"world_slug": "test-world", "genre_slug": "folk-horror", "places": [
        {"slug": "the-hall", "name": "The Hall", "tier": 1, "place_type": "infrastructure", "description": "Council hall.", "spatial_role": "center"},
    ]}
    (world / "places.json").write_text(json.dumps(places))

    orgs = {"world_slug": "test-world", "genre_slug": "folk-horror", "organizations": [
        {"slug": "parish-council", "name": "Parish Council", "tier": 2, "org_type": "governance", "description": "Local governance."},
    ]}
    (world / "organizations.json").write_text(json.dumps(orgs))

    substrate = {"world_slug": "test-world", "genre_slug": "folk-horror",
        "clusters": [
            {"slug": "the-morrows", "name": "The Morrows", "basis": "blood", "hierarchy_position": "dominant"},
            {"slug": "the-hallodays", "name": "The Hallodays", "basis": "blood", "hierarchy_position": "established"},
        ],
        "relationships": [
            {"cluster_a": "the-morrows", "cluster_b": "the-hallodays", "type": "intermarriage-with-tension", "boundary_tension": "Land vs labor."},
        ],
    }
    (world / "social-substrate.json").write_text(json.dumps(substrate))

    mundane = {"world_slug": "test-world", "genre_slug": "folk-horror", "characters": [
        {"centrality": "Q1", "slug": "elda-farrow", "name": "Elda Farrow", "role": "mail carrier"},
        {"centrality": "Q2", "slug": "gareth-morrow", "name": "Gareth Morrow", "role": "tithe collector"},
    ]}
    (world / "characters-mundane.json").write_text(json.dumps(mundane))

    # Bedrock archetype data
    arch_dir = tmp_path / "narrative-data" / "discovery" / "archetypes" / "folk-horror"
    arch_dir.mkdir(parents=True)
    archetype = {
        "canonical_name": "The Earnest Warden",
        "genre_slug": "folk-horror",
        "personality_profile": {"warmth": 0.8, "authority": 0.7, "openness": 0.3, "interiority": 0.6, "stability": 0.8, "agency": 0.7, "morality": 0.5},
        "distinguishing_tension": "Genuine care vs. structural complicity",
        "structural_necessity": "Community enforcement through warmth",
    }
    (arch_dir / "the-earnest-warden.json").write_text(json.dumps(archetype))

    # Archetype dynamics data
    dyn_dir = tmp_path / "narrative-data" / "discovery" / "archetype-dynamics" / "folk-horror"
    dyn_dir.mkdir(parents=True)
    dynamic = {
        "pairing_name": "The Warmth That Prepares the Sacrifice",
        "archetype_a": "The Unwilling Vessel",
        "archetype_b": "The Earnest Warden",
        "edge_properties": {"edge_type": "Trust-textured, binding"},
    }
    (dyn_dir / "vessel-warden.json").write_text(json.dumps(dynamic))

    return tmp_path


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestLoadMundaneCharacters:
    def test_loads_characters(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_mundane_characters

        world_dir = data_root / "narrative-data" / "tome" / "worlds" / "test-world"
        chars = _load_mundane_characters(world_dir)
        assert len(chars) == 2

    def test_raises_when_missing(self, tmp_path: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_mundane_characters

        empty = tmp_path / "empty"
        empty.mkdir()
        with pytest.raises(FileNotFoundError, match="characters-mundane.json"):
            _load_mundane_characters(empty)


class TestLoadArchetypes:
    def test_loads_genre_archetypes(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_archetypes

        archetypes = _load_archetypes(data_root, "folk-horror")
        assert len(archetypes) == 1
        assert archetypes[0]["canonical_name"] == "The Earnest Warden"

    def test_returns_empty_for_missing_genre(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_archetypes

        archetypes = _load_archetypes(data_root, "nonexistent-genre")
        assert archetypes == []


class TestLoadArchetypeDynamics:
    def test_loads_genre_dynamics(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_archetype_dynamics

        dynamics = _load_archetype_dynamics(data_root, "folk-horror")
        assert len(dynamics) == 1
        assert dynamics[0]["pairing_name"] == "The Warmth That Prepares the Sacrifice"


class TestBuildArchetypesContext:
    def test_formats_archetypes(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import (
            _build_archetypes_context,
            _load_archetypes,
        )

        archetypes = _load_archetypes(data_root, "folk-horror")
        ctx = _build_archetypes_context(archetypes)
        assert "The Earnest Warden" in ctx
        assert "Genuine care" in ctx
        assert "warmth" in ctx


class TestBuildPrompt:
    def test_substitutes_all_placeholders(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import (
            _build_archetypes_context,
            _build_dynamics_context,
            _build_mundane_characters_context,
            _build_prompt,
            _load_archetypes,
            _load_archetype_dynamics,
            _load_mundane_characters,
        )
        from narrative_data.tome.elicit_places import _load_world_position
        from narrative_data.tome.elicit_orgs import _load_places
        from narrative_data.tome.elicit_social_substrate import _load_orgs
        from narrative_data.tome.elicit_characters_mundane import _load_social_substrate

        world_dir = data_root / "narrative-data" / "tome" / "worlds" / "test-world"
        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)
        mundane = _load_mundane_characters(world_dir)
        archetypes = _load_archetypes(data_root, "folk-horror")
        dynamics = _load_archetype_dynamics(data_root, "folk-horror")

        template = (
            "{genre_slug} {setting_slug} {world_preamble} "
            "{genre_profile_summary} {places_context} {orgs_context} "
            "{social_substrate_context} {mundane_characters_context} "
            "{archetypes_context} {archetype_dynamics_context}"
        )
        prompt = _build_prompt(
            template=template,
            world_pos=world_pos,
            genre_profile=None,
            places=places,
            orgs=orgs,
            substrate=substrate,
            mundane_characters=mundane,
            archetypes=archetypes,
            archetype_dynamics=dynamics,
            settings_context="",
        )
        assert "folk-horror" in prompt
        assert "The Earnest Warden" in prompt
        assert "Elda Farrow" in prompt
        assert "{" not in prompt
