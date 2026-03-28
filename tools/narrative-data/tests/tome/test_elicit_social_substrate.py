# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for social substrate elicitation module."""

import json
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def world_dir(tmp_path: Path) -> Path:
    """Create a minimal world directory with world-position, places, and orgs."""
    world = tmp_path / "narrative-data" / "tome" / "worlds" / "test-world"
    world.mkdir(parents=True)

    world_pos = {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "seed_count": 2,
        "inferred_count": 3,
        "total_positions": 5,
        "positions": [
            {"axis_slug": "kinship-system", "value": "clan-tribal", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "social-stratification", "value": "caste-hereditary", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "community-cohesion", "value": "high", "confidence": 0.8, "source": "inferred", "justification": "kinship-system →produces→ community-cohesion (0.7)"},
            {"axis_slug": "outsider-integration-pattern", "value": "persecutory-expulsive", "confidence": 0.7, "source": "inferred", "justification": "community-cohesion →constrains→ outsider-integration-pattern (0.6)"},
            {"axis_slug": "labor-organization", "value": "household-subsistence", "confidence": 0.6, "source": "inferred", "justification": "kinship-system →produces→ labor-organization (0.5)"},
        ],
    }
    (world / "world-position.json").write_text(json.dumps(world_pos))

    places = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "places": [
            {"slug": "the-market", "name": "The Market", "tier": 2, "place_type": "gathering-place", "description": "A dusty market square."},
            {"slug": "the-hall", "name": "The Hall", "tier": 1, "place_type": "infrastructure", "description": "The council hall.", "spatial_role": "center"},
        ],
    }
    (world / "places.json").write_text(json.dumps(places))

    orgs = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "organizations": [
            {"slug": "parish-council", "name": "Parish Council", "tier": 2, "org_type": "governance", "description": "Local governance."},
            {"slug": "the-keepers", "name": "The Keepers", "tier": 1, "org_type": "religious", "description": "Knowledge suppressors."},
        ],
    }
    (world / "organizations.json").write_text(json.dumps(orgs))

    return world


# ---------------------------------------------------------------------------
# Context loading tests
# ---------------------------------------------------------------------------


class TestLoadOrgs:
    def test_loads_org_list(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import _load_orgs

        orgs = _load_orgs(world_dir)
        assert len(orgs) == 2
        assert orgs[0]["slug"] == "parish-council"

    def test_raises_when_missing(self, tmp_path: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import _load_orgs

        empty = tmp_path / "empty"
        empty.mkdir()
        with pytest.raises(FileNotFoundError, match="organizations.json"):
            _load_orgs(empty)


class TestBuildOrgsContext:
    def test_formats_orgs_as_markdown(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs

        orgs = _load_orgs(world_dir)
        ctx = _build_orgs_context(orgs)
        assert "Parish Council" in ctx
        assert "The Keepers" in ctx
        assert "parish-council" in ctx


class TestBuildSubstratePrompt:
    def test_substitutes_all_placeholders(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import (
            _build_orgs_context,
            _build_prompt,
            _load_orgs,
        )
        from narrative_data.tome.elicit_places import (
            _build_genre_profile_summary,
            _build_world_preamble,
            _load_world_position,
        )

        world_pos = _load_world_position(world_dir)
        orgs = _load_orgs(world_dir)

        # Minimal template with all placeholders
        template = (
            "{genre_slug} {setting_slug} {world_preamble} "
            "{genre_profile_summary} {places_context} {orgs_context} "
            "{kinship_system_value} {stratification_value}"
        )
        prompt = _build_prompt(
            template=template,
            world_pos=world_pos,
            genre_profile=None,
            places=[{"slug": "the-market", "name": "The Market", "spatial_role": "center", "description": "A market."}],
            orgs=orgs,
            settings_context="",
        )
        assert "folk-horror" in prompt
        assert "test-village" in prompt
        assert "clan-tribal" in prompt
        assert "caste-hereditary" in prompt
        assert "{" not in prompt  # No unsubstituted placeholders


class TestParseSubstrateResponse:
    def test_parses_valid_json_object(self) -> None:
        from narrative_data.tome.elicit_social_substrate import _parse_substrate_response

        response = json.dumps({
            "clusters": [{"slug": "the-morrows", "name": "The Morrows"}],
            "relationships": [{"cluster_a": "the-morrows", "cluster_b": "the-others", "type": "rivalry"}],
        })
        result = _parse_substrate_response(response)
        assert "clusters" in result
        assert len(result["clusters"]) == 1

    def test_parses_json_in_code_fence(self) -> None:
        from narrative_data.tome.elicit_social_substrate import _parse_substrate_response

        response = '```json\n{"clusters": [{"slug": "a"}], "relationships": []}\n```'
        result = _parse_substrate_response(response)
        assert len(result["clusters"]) == 1

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.elicit_social_substrate import _parse_substrate_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_substrate_response("This is not JSON at all")
