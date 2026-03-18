"""Tests for prompt loading and compositional building."""

from pathlib import Path

import pytest

from narrative_data.prompts import PromptBuilder


@pytest.fixture
def prompt_dir(tmp_path: Path) -> Path:
    genre_dir = tmp_path / "genre"
    genre_dir.mkdir()
    (genre_dir / "region.md").write_text("Describe genre region: {target_name}")
    spatial_dir = tmp_path / "spatial"
    spatial_dir.mkdir()
    (spatial_dir / "setting-type.md").write_text("Describe setting type: {target_name}")
    (tmp_path / "_commentary.md").write_text(
        "\n---\nInclude _commentary and _suggestions sections."
    )
    return tmp_path


class TestPromptBuilder:
    def test_load_core_prompt(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.load_core_prompt("genre", "region")
        assert "Describe genre region" in prompt

    def test_load_missing_prompt_raises(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        with pytest.raises(FileNotFoundError):
            builder.load_core_prompt("genre", "nonexistent")

    def test_build_stage1_prompt(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.build_stage1(
            domain="genre", category="region", target_name="Folk Horror",
        )
        assert "Folk Horror" in prompt
        assert "_commentary" in prompt
        assert "_suggestions" in prompt

    def test_build_stage1_with_context(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.build_stage1(
            domain="genre", category="region", target_name="Folk Horror",
            context={"prior_region": '{"name": "Cosmic Horror"}'},
        )
        assert "Cosmic Horror" in prompt

    def test_build_stage2_prompt(self, prompt_dir: Path):  # noqa: ARG002
        raw_content = "# Folk Horror\n\nA genre of rural dread."
        schema = {"type": "object", "properties": {"name": {"type": "string"}}}
        prompt = PromptBuilder.build_stage2(raw_content, schema)
        assert "Folk Horror" in prompt
        assert '"type": "object"' in prompt
        assert "Produce JSON matching this schema" in prompt
