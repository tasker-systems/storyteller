# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the entity planning module."""

from pathlib import Path

import pytest  # noqa: F401 (used by fixtures implicitly)

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def template_path() -> Path:
    """Return the path to the entity-plan prompt template."""
    return (
        Path(__file__).parent.parent.parent / "prompts" / "tome" / "decomposed" / "entity-plan.md"
    )


@pytest.fixture()
def world_summary() -> dict:
    """Return a minimal world summary dict."""
    return {
        "genre_slug": "folk-horror",
        "setting_slug": "mccallisters-barn",
        "compressed_preamble": (
            "### Material Conditions\n"
            "- geography-climate: temperate-highland [seed]\n"
            "- settlement-pattern: dispersed-rural [seed]\n\n"
            "### Social Forms\n"
            "- kinship-system: clan-tribal [seed]\n"
            "- social-stratification: caste-hereditary\n"
        ),
        "axis_count": 4,
        "seed_count": 3,
    }


@pytest.fixture()
def genre_profile_summary() -> str:
    """Return a minimal genre profile summary string."""
    return (
        "**World Affordances:**\n"
        "- isolation: high\n"
        "- community_cohesion: enforced\n\n"
        "**Tonal Register:**\n"
        "- dread: slow-burn\n"
    )


@pytest.fixture()
def valid_plan() -> dict:
    """Return a valid entity plan dict."""
    return {
        "places": {
            "count": 10,
            "distribution": {
                "infrastructure": 1,
                "institution": 2,
                "dwelling": 2,
                "commercial": 1,
                "liminal": 1,
                "sacred": 1,
                "natural": 1,
                "workshop": 1,
            },
        },
        "organizations": {"count": 4},
        "clusters": {
            "count": 3,
            "basis_hint": "clan-tribal → blood basis (lineages, family names)",
        },
        "characters_mundane": {"q1_count": 5, "q2_count": 3},
        "characters_significant": {"q3_count": 2, "q4_count": 1},
    }


# ---------------------------------------------------------------------------
# Tests: _build_plan_prompt
# ---------------------------------------------------------------------------


class TestBuildPlanPrompt:
    def test_substitutes_genre_slug(
        self, template_path: Path, world_summary: dict, genre_profile_summary: str
    ) -> None:
        from narrative_data.tome.plan_entities import _build_plan_prompt

        result = _build_plan_prompt(template_path, world_summary, genre_profile_summary)

        assert "folk-horror" in result
        assert "{genre_slug}" not in result

    def test_substitutes_setting_slug(
        self, template_path: Path, world_summary: dict, genre_profile_summary: str
    ) -> None:
        from narrative_data.tome.plan_entities import _build_plan_prompt

        result = _build_plan_prompt(template_path, world_summary, genre_profile_summary)

        assert "mccallisters-barn" in result
        assert "{setting_slug}" not in result

    def test_substitutes_compressed_preamble(
        self, template_path: Path, world_summary: dict, genre_profile_summary: str
    ) -> None:
        from narrative_data.tome.plan_entities import _build_plan_prompt

        result = _build_plan_prompt(template_path, world_summary, genre_profile_summary)

        assert "clan-tribal" in result
        assert "{compressed_preamble}" not in result

    def test_substitutes_genre_profile_summary(
        self, template_path: Path, world_summary: dict, genre_profile_summary: str
    ) -> None:
        from narrative_data.tome.plan_entities import _build_plan_prompt

        result = _build_plan_prompt(template_path, world_summary, genre_profile_summary)

        assert "slow-burn" in result
        assert "{genre_profile_summary}" not in result

    def test_no_remaining_placeholders(
        self, template_path: Path, world_summary: dict, genre_profile_summary: str
    ) -> None:
        from narrative_data.tome.plan_entities import _build_plan_prompt

        result = _build_plan_prompt(template_path, world_summary, genre_profile_summary)

        # No curly-brace placeholders should remain
        import re

        remaining = re.findall(r"\{[a-z_]+\}", result)
        assert remaining == [], f"Unreplaced placeholders: {remaining}"


# ---------------------------------------------------------------------------
# Tests: _parse_plan_response
# ---------------------------------------------------------------------------


class TestParsePlanResponse:
    def test_parses_valid_json(self, valid_plan: dict) -> None:
        import json

        from narrative_data.tome.plan_entities import _parse_plan_response

        response = json.dumps(valid_plan)
        result = _parse_plan_response(response)

        assert result["places"]["count"] == 10
        assert result["organizations"]["count"] == 4

    def test_parses_from_json_code_fence(self, valid_plan: dict) -> None:
        import json

        from narrative_data.tome.plan_entities import _parse_plan_response

        inner = json.dumps(valid_plan)
        response = f"Here is the plan:\n```json\n{inner}\n```\n"
        result = _parse_plan_response(response)

        assert result["clusters"]["count"] == 3

    def test_parses_from_plain_code_fence(self, valid_plan: dict) -> None:
        import json

        from narrative_data.tome.plan_entities import _parse_plan_response

        inner = json.dumps(valid_plan)
        response = f"```\n{inner}\n```"
        result = _parse_plan_response(response)

        assert result["characters_mundane"]["q1_count"] == 5

    def test_parses_by_finding_outermost_braces(self, valid_plan: dict) -> None:
        import json

        from narrative_data.tome.plan_entities import _parse_plan_response

        inner = json.dumps(valid_plan)
        response = f"Some preamble text\n{inner}\nSome trailing text"
        result = _parse_plan_response(response)

        assert result["characters_significant"]["q4_count"] == 1

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.plan_entities import _parse_plan_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_plan_response("this is not json at all, no braces here")

    def test_raises_on_json_array(self) -> None:
        from narrative_data.tome.plan_entities import _parse_plan_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_plan_response("[1, 2, 3]")


# ---------------------------------------------------------------------------
# Tests: _validate_plan
# ---------------------------------------------------------------------------


class TestValidatePlan:
    def test_valid_plan_passes(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        # Should not raise
        _validate_plan(valid_plan)

    def test_missing_places_raises(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        del valid_plan["places"]
        with pytest.raises(ValueError, match="places"):
            _validate_plan(valid_plan)

    def test_missing_organizations_raises(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        del valid_plan["organizations"]
        with pytest.raises(ValueError, match="organizations"):
            _validate_plan(valid_plan)

    def test_missing_clusters_raises(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        del valid_plan["clusters"]
        with pytest.raises(ValueError, match="clusters"):
            _validate_plan(valid_plan)

    def test_missing_characters_mundane_raises(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        del valid_plan["characters_mundane"]
        with pytest.raises(ValueError, match="characters_mundane"):
            _validate_plan(valid_plan)

    def test_missing_characters_significant_raises(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        del valid_plan["characters_significant"]
        with pytest.raises(ValueError, match="characters_significant"):
            _validate_plan(valid_plan)

    def test_distribution_normalizes_count_to_match_sum(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        # Distribution sums to 10 but count says 12 — validation normalizes
        valid_plan["places"]["count"] = 12
        _validate_plan(valid_plan)
        assert valid_plan["places"]["count"] == 10

    def test_distribution_filters_zero_counts(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        valid_plan["places"]["distribution"]["liminal"] = 0
        _validate_plan(valid_plan)
        assert "liminal" not in valid_plan["places"]["distribution"]

    def test_places_without_distribution_passes(self, valid_plan: dict) -> None:
        from narrative_data.tome.plan_entities import _validate_plan

        # If there's no distribution key, count alone is fine
        del valid_plan["places"]["distribution"]
        _validate_plan(valid_plan)
