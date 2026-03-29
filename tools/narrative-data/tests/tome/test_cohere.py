# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the coherence engine."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def template_path(tmp_path: Path) -> Path:
    """Create a minimal coherence prompt template file."""
    tmpl = tmp_path / "coherence-places.md"
    tmpl.write_text(
        "Bind these {genre_slug} places in {setting_slug}.\n"
        "World context: {compressed_preamble}\n"
        "Draft entities:\n{draft_entities}\n"
        "Upstream context:\n{upstream_context}\n"
    )
    return tmpl


@pytest.fixture()
def world_summary() -> dict:
    return {
        "genre_slug": "folk-horror",
        "setting_slug": "mccallisters-barn",
        "compressed_preamble": "A bleak highland settlement where old traditions persist.",
    }


@pytest.fixture()
def draft_entities() -> list[dict]:
    return [
        {"slug": "old-farmhouse", "name": "Old Farmhouse", "place_type": "dwelling"},
        {"slug": "the-mill", "name": "The Mill", "place_type": "infrastructure"},
    ]


@pytest.fixture()
def valid_array_json(draft_entities) -> str:
    return json.dumps(draft_entities)


# ---------------------------------------------------------------------------
# Tests: _build_coherence_prompt
# ---------------------------------------------------------------------------


class TestBuildCoherencePrompt:
    def test_substitutes_standard_placeholders(
        self, template_path: Path, world_summary: dict, draft_entities: list
    ) -> None:
        from narrative_data.tome.cohere import _build_coherence_prompt

        result = _build_coherence_prompt(
            template_path,
            world_summary,
            draft_entities,
            upstream_context="Places from prior stage.",
        )

        assert "folk-horror" in result
        assert "mccallisters-barn" in result
        assert "A bleak highland settlement" in result

    def test_substitutes_draft_entities_as_json(
        self, template_path: Path, world_summary: dict, draft_entities: list
    ) -> None:
        from narrative_data.tome.cohere import _build_coherence_prompt

        result = _build_coherence_prompt(
            template_path,
            world_summary,
            draft_entities,
            upstream_context="",
        )

        assert "old-farmhouse" in result
        assert "Old Farmhouse" in result

    def test_substitutes_upstream_context(
        self, template_path: Path, world_summary: dict, draft_entities: list
    ) -> None:
        from narrative_data.tome.cohere import _build_coherence_prompt

        result = _build_coherence_prompt(
            template_path,
            world_summary,
            draft_entities,
            upstream_context="Prior places: The Barn, The Mill.",
        )

        assert "Prior places: The Barn, The Mill." in result

    def test_substitutes_extra_context_keys(self, tmp_path: Path, world_summary: dict) -> None:
        tmpl = tmp_path / "coherence-custom.md"
        tmpl.write_text(
            "Genre: {genre_slug}\n"
            "Setting: {setting_slug}\n"
            "Preamble: {compressed_preamble}\n"
            "Draft: {draft_entities}\n"
            "Upstream: {upstream_context}\n"
            "Extra: {custom_key}\n"
        )

        from narrative_data.tome.cohere import _build_coherence_prompt

        result = _build_coherence_prompt(
            tmpl,
            world_summary,
            [{"slug": "place-a"}],
            upstream_context="ctx",
            extra_context={"custom_key": "custom_value"},
        )

        assert "custom_value" in result
        assert "{custom_key}" not in result

    def test_no_remaining_standard_placeholders(
        self, template_path: Path, world_summary: dict, draft_entities: list
    ) -> None:
        from narrative_data.tome.cohere import _build_coherence_prompt

        result = _build_coherence_prompt(
            template_path,
            world_summary,
            draft_entities,
            upstream_context="ctx",
        )

        assert "{genre_slug}" not in result
        assert "{setting_slug}" not in result
        assert "{compressed_preamble}" not in result
        assert "{draft_entities}" not in result
        assert "{upstream_context}" not in result


# ---------------------------------------------------------------------------
# Tests: _parse_coherence_response
# ---------------------------------------------------------------------------


class TestParseCoherenceResponse:
    def test_parses_direct_json_array(self, valid_array_json: str) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        result = _parse_coherence_response(valid_array_json)

        assert isinstance(result, list)
        assert len(result) == 2
        assert result[0]["slug"] == "old-farmhouse"

    def test_parses_from_json_code_fence(self, valid_array_json: str) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        response = f"Here are the bound entities:\n```json\n{valid_array_json}\n```\n"
        result = _parse_coherence_response(response)

        assert isinstance(result, list)
        assert result[0]["slug"] == "old-farmhouse"

    def test_parses_by_finding_outermost_brackets(self, valid_array_json: str) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        response = f"Some preamble\n{valid_array_json}\nSome trailing text"
        result = _parse_coherence_response(response)

        assert isinstance(result, list)
        assert len(result) == 2

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.cohere import _parse_coherence_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_coherence_response("this is not json at all")

    def test_accepts_json_object_for_substrate(self) -> None:
        """Substrate coherence returns a dict with clusters and relationships."""
        from narrative_data.tome.cohere import _parse_coherence_response

        substrate = {"clusters": [{"slug": "a"}], "relationships": []}
        result = _parse_coherence_response(json.dumps(substrate))
        assert isinstance(result, dict)
        assert len(result["clusters"]) == 1


# ---------------------------------------------------------------------------
# Tests: cohere
# ---------------------------------------------------------------------------


class TestCohere:
    def test_calls_coherence_model(
        self,
        template_path: Path,
        world_summary: dict,
        draft_entities: list,
        valid_array_json: str,
    ) -> None:
        from narrative_data.tome.cohere import cohere
        from narrative_data.tome.models import get_model

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_array_json

        result = cohere(
            mock_client,
            template_path,
            world_summary,
            draft_entities,
            upstream_context="",
        )

        assert isinstance(result, list)
        mock_client.generate.assert_called_once()
        call_kwargs = mock_client.generate.call_args[1]
        assert call_kwargs["model"] == get_model("coherence")

    def test_returns_parsed_entities(
        self,
        template_path: Path,
        world_summary: dict,
        draft_entities: list,
        valid_array_json: str,
    ) -> None:
        from narrative_data.tome.cohere import cohere

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_array_json

        result = cohere(
            mock_client,
            template_path,
            world_summary,
            draft_entities,
            upstream_context="ctx",
        )

        assert len(result) == 2
        assert result[0]["slug"] == "old-farmhouse"

    def test_passes_elicitation_timeout(
        self,
        template_path: Path,
        world_summary: dict,
        draft_entities: list,
        valid_array_json: str,
    ) -> None:
        from narrative_data.config import ELICITATION_TIMEOUT
        from narrative_data.tome.cohere import cohere

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_array_json

        cohere(
            mock_client,
            template_path,
            world_summary,
            draft_entities,
            upstream_context="",
        )

        call_kwargs = mock_client.generate.call_args[1]
        assert call_kwargs["timeout"] == ELICITATION_TIMEOUT

    def test_passes_temperature_05(
        self,
        template_path: Path,
        world_summary: dict,
        draft_entities: list,
        valid_array_json: str,
    ) -> None:
        from narrative_data.tome.cohere import cohere

        mock_client = MagicMock()
        mock_client.generate.return_value = valid_array_json

        cohere(
            mock_client,
            template_path,
            world_summary,
            draft_entities,
            upstream_context="",
        )

        call_kwargs = mock_client.generate.call_args[1]
        assert call_kwargs["temperature"] == 0.5


# ---------------------------------------------------------------------------
# Tests: save_coherence_output
# ---------------------------------------------------------------------------


class TestSaveCoherenceOutput:
    def test_writes_places_json_with_correct_metadata(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "decomposed"
        entities = [
            {"slug": "old-farmhouse", "name": "Old Farmhouse"},
            {"slug": "the-mill", "name": "The Mill"},
        ]

        path = save_coherence_output(
            decomposed_dir,
            stage="places",
            entities=entities,
            world_slug="test-world",
            genre_slug="folk-horror",
            setting_slug="mccallisters-barn",
        )

        assert path.name == "places.json"
        data = json.loads(path.read_text())
        assert data["world_slug"] == "test-world"
        assert data["genre_slug"] == "folk-horror"
        assert data["setting_slug"] == "mccallisters-barn"
        assert data["pipeline"] == "decomposed"
        assert data["count"] == 2
        assert "generated_at" in data

    def test_places_stage_uses_places_key(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "decomposed"
        entities = [{"slug": "place-a"}]

        path = save_coherence_output(
            decomposed_dir,
            stage="places",
            entities=entities,
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        data = json.loads(path.read_text())
        assert "places" in data
        assert data["places"] == entities

    def test_orgs_stage_uses_organizations_key(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "decomposed"
        entities = [{"slug": "org-a"}]

        path = save_coherence_output(
            decomposed_dir,
            stage="orgs",
            entities=entities,
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        data = json.loads(path.read_text())
        assert "organizations" in data
        assert path.name == "organizations.json"

    def test_substrate_stage_uses_clusters_key(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "decomposed"
        entities = [{"slug": "cluster-a"}]

        path = save_coherence_output(
            decomposed_dir,
            stage="substrate",
            entities=entities,
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        data = json.loads(path.read_text())
        assert "clusters" in data
        assert path.name == "social-substrate.json"

    def test_characters_mundane_stage_uses_characters_key(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "decomposed"
        entities = [{"slug": "char-a"}]

        path = save_coherence_output(
            decomposed_dir,
            stage="characters-mundane",
            entities=entities,
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        data = json.loads(path.read_text())
        assert "characters" in data
        assert path.name == "characters-mundane.json"

    def test_characters_significant_stage_uses_characters_key(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "decomposed"
        entities = [{"slug": "char-sig-a"}]

        path = save_coherence_output(
            decomposed_dir,
            stage="characters-significant",
            entities=entities,
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        data = json.loads(path.read_text())
        assert "characters" in data
        assert path.name == "characters-significant.json"

    def test_creates_directories_as_needed(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        decomposed_dir = tmp_path / "new" / "nested" / "decomposed"
        entities = [{"slug": "place-a"}]

        path = save_coherence_output(
            decomposed_dir,
            stage="places",
            entities=entities,
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        assert path.exists()

    def test_returns_path_object(self, tmp_path: Path) -> None:
        from narrative_data.tome.cohere import save_coherence_output

        path = save_coherence_output(
            tmp_path / "decomposed",
            stage="places",
            entities=[],
            world_slug="w",
            genre_slug="g",
            setting_slug="s",
        )

        assert isinstance(path, Path)
