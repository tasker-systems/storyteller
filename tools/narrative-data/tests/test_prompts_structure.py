# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for PromptBuilder.build_structure() and build_segment_structure()."""

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


class TestBuildSegmentStructure:
    def test_loads_from_segments_subdirectory(self, tmp_path):
        seg_dir = tmp_path / "structure" / "segments"
        seg_dir.mkdir(parents=True)
        (seg_dir / "genre-region-aesthetic.md").write_text(
            "Extract:\n{raw_content}\nSchema:\n{schema}"
        )
        builder = PromptBuilder(prompts_dir=tmp_path)
        result = builder.build_segment_structure(
            "genre-region-aesthetic", "content here", {"type": "object"}
        )
        assert "content here" in result
        assert '"type": "object"' in result

    def test_missing_template_raises(self, tmp_path):
        builder = PromptBuilder(prompts_dir=tmp_path)
        with pytest.raises(FileNotFoundError):
            builder.build_segment_structure("nonexistent", "content", {})

    def test_all_segment_prompts_load(self):
        """Verify all 17 segment prompt templates load without error."""
        builder = PromptBuilder()
        segment_types = [
            "genre-region-meta",
            "genre-region-aesthetic",
            "genre-region-tonal",
            "genre-region-temporal",
            "genre-region-thematic",
            "genre-region-agency",
            "genre-region-epistemological",
            "genre-region-world-affordances",
            "genre-region-locus-of-power",
            "genre-region-narrative-structure",
            "genre-region-narrative-contracts",
            "genre-region-state-variables",
            "genre-region-boundaries",
            "discovery-entity",
            "discovery-entity-cluster",
            "trope-entity",
            "narrative-shape-entity",
        ]
        for seg_type in segment_types:
            result = builder.build_segment_structure(seg_type, "test content", {"type": "object"})
            assert "{raw_content}" not in result
            assert "{schema}" not in result
            assert "test content" in result
