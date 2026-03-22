"""Tests for the two-stage pipeline (Ollama calls mocked)."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.pipeline.structure import run_structuring
from narrative_data.prompts import PromptBuilder
from narrative_data.schemas.genre import GenreRegion


@pytest.fixture
def mock_ollama() -> OllamaClient:
    return MagicMock(spec=OllamaClient)


@pytest.fixture
def prompt_builder(tmp_path: Path) -> PromptBuilder:
    genre_dir = tmp_path / "genre"
    genre_dir.mkdir()
    (genre_dir / "region.md").write_text("Describe genre: {target_name}")
    (tmp_path / "_commentary.md").write_text("\n---\nCommentary directive.")
    return PromptBuilder(tmp_path)


class TestRunElicitation:
    def test_writes_raw_md(self, mock_ollama, prompt_builder, tmp_path: Path):
        mock_ollama.generate.return_value = "# Folk Horror\n\nRich content here."
        output_dir = tmp_path / "output" / "folk-horror"
        output_dir.mkdir(parents=True)

        result = run_elicitation(
            client=mock_ollama,
            builder=prompt_builder,
            domain="genre",
            category="region",
            target_name="Folk Horror",
            target_slug="folk-horror",
            output_dir=output_dir,
            model="qwen3.5:35b",
        )

        raw_path = output_dir / "region.md"
        assert raw_path.exists()
        assert "Folk Horror" in raw_path.read_text()
        assert result["prompt_hash"] is not None
        assert result["content_digest"].startswith("sha256:")


class TestRunStructuring:
    def test_structures_and_validates(self, mock_ollama, tmp_path: Path):
        raw_path = tmp_path / "region.md"
        raw_path.write_text("# Folk Horror\n\nRural dread content.")

        structured_output = {
            "entity_id": "019d0000-0000-7000-8000-000000000010",
            "name": "Folk Horror",
            "description": "Rural dread, community as threat",
            "provenance": {
                "prompt_hash": "abc",
                "model": "test",
                "generated_at": "2026-03-17T00:00:00Z",
            },
            "aesthetic": [{"dimension": "spare_ornate", "value": -0.3}],
            "tonal": [{"dimension": "dread_wonder", "value": -0.8}],
            "thematic": [],
            "structural": [],
            "world_affordances": {
                "magic": "subtle",
                "technology": "historical",
                "violence": "consequence-laden",
                "death": "permanent",
                "supernatural": "ambiguous",
            },
        }
        mock_ollama.generate_structured.return_value = structured_output

        result = run_structuring(
            client=mock_ollama,
            raw_path=raw_path,
            output_path=tmp_path / "region.json",
            schema_type=GenreRegion,
            model="qwen2.5:3b-instruct",
            is_collection=False,
        )

        assert result["success"] is True
        json_path = tmp_path / "region.json"
        assert json_path.exists()
        data = json.loads(json_path.read_text())
        assert data["name"] == "Folk Horror"

    def test_validation_failure_writes_errors(self, mock_ollama, tmp_path: Path):
        raw_path = tmp_path / "test.md"
        raw_path.write_text("Some content")
        mock_ollama.generate_structured.return_value = {"invalid": True}

        result = run_structuring(
            client=mock_ollama,
            raw_path=raw_path,
            output_path=tmp_path / "test.json",
            schema_type=GenreRegion,
            model="qwen2.5:3b-instruct",
            is_collection=False,
            max_retries=1,
        )

        assert result["success"] is False
        errors_path = tmp_path / "test.errors.json"
        assert errors_path.exists()
