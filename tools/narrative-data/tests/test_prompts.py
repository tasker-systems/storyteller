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
            domain="genre",
            category="region",
            target_name="Folk Horror",
        )
        assert "Folk Horror" in prompt
        assert "_commentary" in prompt
        assert "_suggestions" in prompt

    def test_build_stage1_with_context(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.build_stage1(
            domain="genre",
            category="region",
            target_name="Folk Horror",
            context={"prior_region": '{"name": "Cosmic Horror"}'},
        )
        assert "Cosmic Horror" in prompt


class TestBuildDiscovery:
    def test_injects_genre_content(self, tmp_path):
        prompts_dir = tmp_path / "prompts"
        (prompts_dir / "discovery").mkdir(parents=True)
        (prompts_dir / "discovery" / "extract-archetypes.md").write_text(
            "Extract archetypes for {target_name}.\n\n"
            "## Genre Region Description\n\n{genre_content}"
        )
        builder = PromptBuilder(prompts_dir=prompts_dir)
        result = builder.build_discovery(
            primitive_type="archetypes",
            target_name="Folk Horror",
            genre_content="Rich description of folk horror...",
        )
        assert "Folk Horror" in result
        assert "Rich description of folk horror..." in result

    def test_appends_commentary(self, tmp_path):
        prompts_dir = tmp_path / "prompts"
        (prompts_dir / "discovery").mkdir(parents=True)
        (prompts_dir / "discovery" / "extract-archetypes.md").write_text(
            "Extract for {target_name}.\n{genre_content}"
        )
        (prompts_dir / "_commentary.md").write_text("Add commentary.")
        builder = PromptBuilder(prompts_dir=prompts_dir)
        result = builder.build_discovery("archetypes", "Test", "content")
        assert "Add commentary." in result


class TestBuildSynthesis:
    def test_injects_extractions(self, tmp_path):
        prompts_dir = tmp_path / "prompts"
        (prompts_dir / "discovery").mkdir(parents=True)
        (prompts_dir / "discovery" / "synthesize-archetypes.md").write_text(
            "Synthesize {primitive_type} for {cluster_name}"
            " ({genre_count} genres).\n\n{extractions}"
        )
        builder = PromptBuilder(prompts_dir=prompts_dir)
        extractions = {"folk-horror": "archetype data 1", "cosmic-horror": "archetype data 2"}
        result = builder.build_synthesis(
            primitive_type="archetypes",
            cluster_name="Horror",
            extractions=extractions,
        )
        assert "Horror" in result
        assert "folk-horror" in result
        assert "archetype data 1" in result
        assert "2 genres" in result
