"""Tests for the two-stage pipeline (Ollama calls mocked)."""

from pathlib import Path
from unittest.mock import MagicMock

import pytest

from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.prompts import PromptBuilder


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
