"""Tests for PromptBuilder.build_structure()."""

from pathlib import Path

import pytest

from narrative_data.prompts import PromptBuilder


@pytest.fixture
def structure_prompts_dir(tmp_path: Path) -> Path:
    """Create a minimal prompts directory with structure templates."""
    struct_dir = tmp_path / "structure"
    struct_dir.mkdir()
    (struct_dir / "genre-dimensions.md").write_text(
        "Extract genre dimensions from:\n\n{raw_content}\n\nTarget schema:\n{schema}"
    )
    return tmp_path


class TestBuildStructure:
    def test_loads_template_and_injects(self, structure_prompts_dir: Path):
        builder = PromptBuilder(prompts_dir=structure_prompts_dir)
        result = builder.build_structure(
            structure_type="genre-dimensions",
            raw_content="Some raw markdown",
            schema={"type": "object"},
        )
        assert "Some raw markdown" in result
        assert '"type": "object"' in result

    def test_missing_template_raises(self, structure_prompts_dir: Path):
        builder = PromptBuilder(prompts_dir=structure_prompts_dir)
        with pytest.raises(FileNotFoundError):
            builder.build_structure(
                structure_type="nonexistent",
                raw_content="content",
                schema={},
            )

    def test_cluster_template(self, structure_prompts_dir: Path):
        struct_dir = structure_prompts_dir / "structure"
        (struct_dir / "archetypes-cluster.md").write_text(
            "Extract cluster archetypes from:\n\n{raw_content}\n\nSchema:\n{schema}"
        )
        builder = PromptBuilder(prompts_dir=structure_prompts_dir)
        result = builder.build_structure(
            structure_type="archetypes-cluster",
            raw_content="Cluster content",
            schema={"type": "array"},
        )
        assert "Cluster content" in result
