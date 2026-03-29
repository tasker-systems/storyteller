# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the fan-out dispatch engine."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from narrative_data.tome.models import FanOutSpec

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def template_dir(tmp_path: Path) -> Path:
    """Create a minimal template directory with a test template."""
    tmpl_dir = tmp_path / "templates"
    tmpl_dir.mkdir(parents=True)
    (tmpl_dir / "place-instance.md").write_text(
        "Generate a place for {genre_slug} world called {setting_slug}.\n"
        "Place type: {place_type}\n"
        "Context: {world_preamble}\n"
    )
    return tmpl_dir


@pytest.fixture()
def basic_spec() -> FanOutSpec:
    """Return a minimal FanOutSpec."""
    return FanOutSpec(
        stage="places",
        index=0,
        template_name="place-instance.md",
        model_role="fan_out_structured",
        context={
            "genre_slug": "folk-horror",
            "setting_slug": "mccallisters-barn",
            "place_type": "dwelling",
            "world_preamble": "A bleak highland settlement.",
        },
    )


@pytest.fixture()
def valid_entity_json() -> str:
    """Return a valid JSON string representing a single entity."""
    return json.dumps(
        {
            "slug": "old-farmhouse",
            "name": "Old Farmhouse",
            "place_type": "dwelling",
            "description": "A crumbling stone farmhouse at the edge of the moor.",
            "tier": 3,
        }
    )


# ---------------------------------------------------------------------------
# Tests: _build_fan_out_prompt
# ---------------------------------------------------------------------------


class TestBuildFanOutPrompt:
    def test_substitutes_context_keys(self, template_dir: Path, basic_spec: FanOutSpec) -> None:
        from narrative_data.tome.fan_out import _build_fan_out_prompt

        result = _build_fan_out_prompt(template_dir, basic_spec)

        assert "folk-horror" in result
        assert "mccallisters-barn" in result
        assert "dwelling" in result
        assert "A bleak highland settlement." in result

    def test_no_remaining_placeholders(self, template_dir: Path, basic_spec: FanOutSpec) -> None:
        from narrative_data.tome.fan_out import _build_fan_out_prompt

        result = _build_fan_out_prompt(template_dir, basic_spec)

        assert "{genre_slug}" not in result
        assert "{setting_slug}" not in result
        assert "{place_type}" not in result
        assert "{world_preamble}" not in result

    def test_loads_correct_template(self, template_dir: Path, basic_spec: FanOutSpec) -> None:
        from narrative_data.tome.fan_out import _build_fan_out_prompt

        result = _build_fan_out_prompt(template_dir, basic_spec)

        # Template text should be present (with substitutions applied)
        assert "Generate a place for" in result

    def test_partial_substitution_leaves_unknown_keys(self, template_dir: Path) -> None:
        """Context keys not present in template are silently ignored."""
        from narrative_data.tome.fan_out import _build_fan_out_prompt

        spec = FanOutSpec(
            stage="places",
            index=0,
            template_name="place-instance.md",
            model_role="fan_out_structured",
            context={
                "genre_slug": "cyberpunk",
                "setting_slug": "neon-depths",
                "place_type": "commercial",
                "world_preamble": "A neon-soaked megacity.",
                "extra_key": "extra_value",  # not in template — silently ignored
            },
        )

        result = _build_fan_out_prompt(template_dir, spec)
        assert "cyberpunk" in result


# ---------------------------------------------------------------------------
# Tests: _parse_fan_out_response
# ---------------------------------------------------------------------------


class TestParseFanOutResponse:
    def test_parses_direct_json_object(self, valid_entity_json: str) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        result = _parse_fan_out_response(valid_entity_json)

        assert result["slug"] == "old-farmhouse"
        assert result["name"] == "Old Farmhouse"

    def test_parses_from_json_code_fence(self, valid_entity_json: str) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        response = f"Here is the entity:\n```json\n{valid_entity_json}\n```\n"
        result = _parse_fan_out_response(response)

        assert result["slug"] == "old-farmhouse"

    def test_parses_from_plain_code_fence(self, valid_entity_json: str) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        response = f"```\n{valid_entity_json}\n```"
        result = _parse_fan_out_response(response)

        assert result["place_type"] == "dwelling"

    def test_parses_by_finding_outermost_braces(self, valid_entity_json: str) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        response = f"Some preamble text\n{valid_entity_json}\nSome trailing text"
        result = _parse_fan_out_response(response)

        assert result["tier"] == 3

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_fan_out_response("this is not json at all")

    def test_raises_on_json_array(self) -> None:
        from narrative_data.tome.fan_out import _parse_fan_out_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_fan_out_response('[{"slug": "a"}, {"slug": "b"}]')


# ---------------------------------------------------------------------------
# Tests: _generate_one
# ---------------------------------------------------------------------------


class TestGenerateOne:
    def test_calls_client_and_returns_parsed_result(
        self, template_dir: Path, basic_spec: FanOutSpec, valid_entity_json: str
    ) -> None:
        from narrative_data.tome.fan_out import _generate_one

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_entity_json

        result = _generate_one(mock_client, template_dir, basic_spec)

        assert result["slug"] == "old-farmhouse"
        mock_client.generate.assert_called_once()

    def test_uses_correct_model_role(
        self, template_dir: Path, basic_spec: FanOutSpec, valid_entity_json: str
    ) -> None:
        from narrative_data.tome.fan_out import _generate_one
        from narrative_data.tome.models import get_model

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_entity_json

        _generate_one(mock_client, template_dir, basic_spec)

        call_kwargs = mock_client.generate.call_args
        assert call_kwargs[1]["model"] == get_model("fan_out_structured")

    def test_retries_on_parse_failure(
        self, template_dir: Path, basic_spec: FanOutSpec, valid_entity_json: str
    ) -> None:
        """First call returns garbage, second returns valid JSON."""
        from narrative_data.tome.fan_out import _generate_one

        mock_client = MagicMock()
        mock_client.generate.side_effect = [
            "this is not valid json",
            valid_entity_json,
        ]

        result = _generate_one(mock_client, template_dir, basic_spec)

        assert result["slug"] == "old-farmhouse"
        assert mock_client.generate.call_count == 2

    def test_retry_prompt_appends_json_hint(
        self, template_dir: Path, basic_spec: FanOutSpec, valid_entity_json: str
    ) -> None:
        """Second attempt should have the JSON-only hint appended to the prompt."""
        from narrative_data.tome.fan_out import _generate_one

        mock_client = MagicMock()
        mock_client.generate.side_effect = [
            "not json",
            valid_entity_json,
        ]

        _generate_one(mock_client, template_dir, basic_spec)

        second_call_prompt = mock_client.generate.call_args_list[1][1]["prompt"]
        assert "Output valid JSON only." in second_call_prompt

    def test_raises_after_two_failures(self, template_dir: Path, basic_spec: FanOutSpec) -> None:
        """Both attempts fail — should raise ValueError."""
        from narrative_data.tome.fan_out import _generate_one

        mock_client = MagicMock()
        mock_client.generate.side_effect = ["garbage", "still garbage"]

        with pytest.raises(ValueError):
            _generate_one(mock_client, template_dir, basic_spec)


# ---------------------------------------------------------------------------
# Tests: fan_out
# ---------------------------------------------------------------------------


class TestFanOut:
    def test_dispatches_all_specs_and_returns_results(
        self, template_dir: Path, valid_entity_json: str
    ) -> None:
        from narrative_data.tome.fan_out import fan_out

        specs = [
            FanOutSpec(
                stage="places",
                index=i,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": "folk-horror",
                    "setting_slug": f"world-{i}",
                    "place_type": "dwelling",
                    "world_preamble": "Bleak.",
                },
            )
            for i in range(3)
        ]

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_entity_json

        results = fan_out(mock_client, template_dir, specs)

        assert len(results) == 3
        assert mock_client.generate.call_count == 3

    def test_results_sorted_by_spec_index(self, template_dir: Path) -> None:
        """Results are returned in spec.index order regardless of completion order."""
        from narrative_data.tome.fan_out import fan_out

        entities = [json.dumps({"slug": f"place-{i}", "index_marker": i}) for i in range(4)]

        specs = [
            FanOutSpec(
                stage="places",
                index=i,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": "folk-horror",
                    "setting_slug": "test",
                    "place_type": "dwelling",
                    "world_preamble": "Bleak.",
                },
            )
            for i in range(4)
        ]

        mock_client = MagicMock()
        mock_client.generate.side_effect = entities

        results = fan_out(mock_client, template_dir, specs)

        assert len(results) == 4
        for i, result in enumerate(results):
            assert result["index_marker"] == i

    def test_skips_failed_instances(self, template_dir: Path, valid_entity_json: str) -> None:
        """Failed instances are skipped; successes are returned."""
        from narrative_data.tome.fan_out import fan_out

        specs = [
            FanOutSpec(
                stage="places",
                index=0,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": "folk-horror",
                    "setting_slug": "test",
                    "place_type": "dwelling",
                    "world_preamble": "Bleak.",
                },
            ),
            FanOutSpec(
                stage="places",
                index=1,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": "folk-horror",
                    "setting_slug": "test",
                    "place_type": "gathering-place",
                    "world_preamble": "Bleak.",
                },
            ),
        ]

        # Spec index=0 succeeds on first attempt; spec index=1 fails both attempts
        mock_client = MagicMock()
        mock_client.generate.side_effect = [
            valid_entity_json,  # spec 0 first attempt
            "garbage",  # spec 1 first attempt
            "still garbage",  # spec 1 second attempt (retry)
        ]

        results = fan_out(mock_client, template_dir, specs)

        assert len(results) == 1
        assert results[0]["slug"] == "old-farmhouse"

    def test_returns_empty_list_when_all_fail(self, template_dir: Path) -> None:
        """Returns empty list when all instances fail."""
        from narrative_data.tome.fan_out import fan_out

        specs = [
            FanOutSpec(
                stage="places",
                index=0,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": "folk-horror",
                    "setting_slug": "test",
                    "place_type": "dwelling",
                    "world_preamble": "Bleak.",
                },
            ),
        ]

        mock_client = MagicMock()
        mock_client.generate.return_value = "always garbage, never json"

        results = fan_out(mock_client, template_dir, specs)

        assert results == []

    def test_empty_specs_returns_empty_list(self, template_dir: Path) -> None:
        from narrative_data.tome.fan_out import fan_out

        mock_client = MagicMock()
        results = fan_out(mock_client, template_dir, [])
        assert results == []


# ---------------------------------------------------------------------------
# Tests: save_instances
# ---------------------------------------------------------------------------


class TestSaveInstances:
    def test_writes_individual_files_to_correct_paths(self, tmp_path: Path) -> None:
        from narrative_data.tome.fan_out import save_instances

        decomposed_dir = tmp_path / "decomposed"
        stage = "places"

        specs = [
            FanOutSpec(
                stage=stage,
                index=0,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={},
            ),
            FanOutSpec(
                stage=stage,
                index=1,
                template_name="place-instance.md",
                model_role="fan_out_structured",
                context={},
            ),
        ]
        results = [
            {"slug": "place-alpha", "name": "Place Alpha"},
            {"slug": "place-beta", "name": "Place Beta"},
        ]

        save_instances(decomposed_dir, stage, specs, results)

        assert (decomposed_dir / "fan-out" / stage / "instance-001.json").exists()
        assert (decomposed_dir / "fan-out" / stage / "instance-002.json").exists()

    def test_files_contain_correct_content(self, tmp_path: Path) -> None:
        from narrative_data.tome.fan_out import save_instances

        decomposed_dir = tmp_path / "decomposed"
        stage = "organizations"

        specs = [
            FanOutSpec(
                stage=stage,
                index=0,
                template_name="org-instance.md",
                model_role="fan_out_structured",
                context={},
            ),
        ]
        results = [{"slug": "the-guild", "name": "The Guild"}]

        save_instances(decomposed_dir, stage, specs, results)

        written = json.loads((decomposed_dir / "fan-out" / stage / "instance-001.json").read_text())
        assert written["slug"] == "the-guild"

    def test_creates_directories_as_needed(self, tmp_path: Path) -> None:
        from narrative_data.tome.fan_out import save_instances

        decomposed_dir = tmp_path / "deep" / "nested" / "decomposed"
        stage = "clusters"

        specs = [
            FanOutSpec(
                stage=stage,
                index=0,
                template_name="cluster-instance.md",
                model_role="fan_out_structured",
                context={},
            ),
        ]
        results = [{"slug": "the-burnsides"}]

        save_instances(decomposed_dir, stage, specs, results)

        assert (decomposed_dir / "fan-out" / stage / "instance-001.json").exists()


# ---------------------------------------------------------------------------
# Tests: aggregate
# ---------------------------------------------------------------------------


class TestAggregate:
    def test_writes_draft_file_as_json_array(self, tmp_path: Path) -> None:
        from narrative_data.tome.fan_out import aggregate

        decomposed_dir = tmp_path / "decomposed"
        stage = "places"

        results = [
            {"slug": "the-barn", "name": "The Barn"},
            {"slug": "the-mill", "name": "The Mill"},
        ]

        aggregate(decomposed_dir, stage, results)

        draft_path = decomposed_dir / f"{stage}-draft.json"
        assert draft_path.exists()

        written = json.loads(draft_path.read_text())
        assert isinstance(written, list)
        assert len(written) == 2
        assert written[0]["slug"] == "the-barn"

    def test_creates_directories_as_needed(self, tmp_path: Path) -> None:
        from narrative_data.tome.fan_out import aggregate

        decomposed_dir = tmp_path / "new" / "decomposed"
        stage = "organizations"

        aggregate(decomposed_dir, stage, [{"slug": "the-guild"}])

        assert (decomposed_dir / f"{stage}-draft.json").exists()

    def test_empty_results_writes_empty_array(self, tmp_path: Path) -> None:
        from narrative_data.tome.fan_out import aggregate

        decomposed_dir = tmp_path / "decomposed"
        stage = "clusters"

        aggregate(decomposed_dir, stage, [])

        written = json.loads((decomposed_dir / f"{stage}-draft.json").read_text())
        assert written == []
