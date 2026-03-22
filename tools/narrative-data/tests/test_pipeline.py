# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the two-stage pipeline (Ollama calls mocked)."""

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest
from pydantic import BaseModel

from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.pipeline.structure import (
    ERROR_MARKER,
    _strip_frontmatter,
    run_segment_extraction,
    run_structuring,
)
from narrative_data.prompts import PromptBuilder


@pytest.fixture
def mock_ollama() -> OllamaClient:
    return MagicMock(spec=OllamaClient)


@pytest.fixture
def mock_client() -> OllamaClient:
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


class SimpleItem(BaseModel):
    name: str
    value: int


class TestRunStructuring:
    """Tests for run_structuring() — PromptBuilder is patched to avoid filesystem deps."""

    def _make_raw(self, tmp_path: Path, content: str = "Some raw narrative content.") -> Path:
        raw_path = tmp_path / "raw.md"
        raw_path.write_text(content)
        return raw_path

    def test_retry_replaces_error_section(self, mock_client, tmp_path: Path):
        """Errors are replaced on retry to protect the 7b model's context window."""
        raw_path = self._make_raw(tmp_path)
        output_path = tmp_path / "output.json"

        # Call 1: missing required field 'value' → triggers ValidationError
        # Call 2: missing required field 'name' → triggers ValidationError
        # Call 3: valid data → success
        mock_client.generate_structured.side_effect = [
            [{"name": "item1"}],  # missing 'value' → invalid
            [{"value": 42}],  # missing 'name' → invalid
            [{"name": "item3", "value": 3}],  # valid
        ]

        with patch("narrative_data.pipeline.structure.PromptBuilder") as MockPB:
            MockPB.return_value.build_structure.return_value = "BASE PROMPT"
            result = run_structuring(
                client=mock_client,
                raw_path=raw_path,
                output_path=output_path,
                schema_type=SimpleItem,
                structure_type="test-type",
                max_retries=3,
            )

        assert result["success"] is True
        calls = mock_client.generate_structured.call_args_list
        assert len(calls) == 3

        prompt_1 = calls[0].kwargs["prompt"]
        prompt_2 = calls[1].kwargs["prompt"]
        prompt_3 = calls[2].kwargs["prompt"]

        # First call has no error section
        assert ERROR_MARKER not in prompt_1

        # Second call has exactly one error section
        assert prompt_2.count(ERROR_MARKER) == 1

        # Third call has exactly one error section (replaced, not appended)
        assert prompt_3.count(ERROR_MARKER) == 1

        # Each error section references only that retry's preceding error.
        # Call 1 failed because 'value' was missing → call 2's error section mentions 'value'.
        error_section_2 = prompt_2.split(ERROR_MARKER, 1)[1]
        assert "value" in error_section_2

        # Call 2 failed because 'name' was missing → call 3's error section mentions 'name'.
        error_section_3 = prompt_3.split(ERROR_MARKER, 1)[1]
        assert "name" in error_section_3

        # Call 3 must NOT contain the call-1 error ('value') mixed into the call-2 error section.
        # If errors were accumulated, both 'value' and 'name' would appear in separate markers.
        assert prompt_3.count(ERROR_MARKER) == 1  # only one marker, not two

    def test_success_on_first_attempt(self, mock_client, tmp_path: Path):
        """No error section when first call succeeds."""
        raw_path = self._make_raw(tmp_path, "Content.")
        output_path = tmp_path / "output.json"

        mock_client.generate_structured.return_value = [{"name": "x", "value": 1}]

        with patch("narrative_data.pipeline.structure.PromptBuilder") as MockPB:
            MockPB.return_value.build_structure.return_value = "BASE PROMPT"
            result = run_structuring(
                client=mock_client,
                raw_path=raw_path,
                output_path=output_path,
                schema_type=SimpleItem,
                structure_type="test-type",
            )

        assert result["success"] is True
        assert mock_client.generate_structured.call_count == 1
        prompt = mock_client.generate_structured.call_args.kwargs["prompt"]
        assert ERROR_MARKER not in prompt

    def test_exhausted_retries_writes_errors_json(self, mock_client, tmp_path: Path):
        """When all retries fail, writes .errors.json and returns success=False."""
        raw_path = self._make_raw(tmp_path, "Content.")
        output_path = tmp_path / "output.json"

        mock_client.generate_structured.return_value = [{"name": "bad"}]  # always missing 'value'

        with patch("narrative_data.pipeline.structure.PromptBuilder") as MockPB:
            MockPB.return_value.build_structure.return_value = "BASE PROMPT"
            result = run_structuring(
                client=mock_client,
                raw_path=raw_path,
                output_path=output_path,
                schema_type=SimpleItem,
                structure_type="test-type",
                max_retries=3,
            )

        assert result["success"] is False
        assert "errors_path" in result
        errors_path = Path(result["errors_path"])
        assert errors_path.exists()
        assert len(result["errors"]) == 3  # one error recorded per attempt


class TestStripFrontmatter:
    def test_strips_yaml_frontmatter(self):
        content = (
            "---\nsource: test.md\nsegment: aesthetic\nlines: 1-5\n---\n\n"
            "**Aesthetic dimensions**\nContent here."
        )
        result = _strip_frontmatter(content)
        assert result.startswith("\n**Aesthetic dimensions**")
        assert "source:" not in result

    def test_no_frontmatter_unchanged(self):
        content = "**Aesthetic dimensions**\nContent here."
        assert _strip_frontmatter(content) == content

    def test_incomplete_frontmatter_unchanged(self):
        content = "---\nincomplete frontmatter\nno closing marker"
        assert _strip_frontmatter(content) == content


class TestRunSegmentExtraction:
    def test_extracts_segment_to_json(self, tmp_path):
        """Given a segment .md and a mock client, produces a .json file."""
        segment_path = tmp_path / "segment-aesthetic.md"
        segment_path.write_text(
            "---\nsource: test.md\nsegment: aesthetic\nlines: 1-5\n---\n\n**Aesthetic**\nContent."
        )

        output_path = tmp_path / "segment-aesthetic.json"

        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"sensory_density": {"value": 0.7}}

        with patch("narrative_data.pipeline.structure.PromptBuilder") as MockPB:
            MockPB.return_value.build_segment_structure.return_value = "test prompt"
            result = run_segment_extraction(
                client=mock_client,
                segment_path=segment_path,
                output_path=output_path,
                schema={"type": "object"},
                segment_prompt_slug="genre-region-aesthetic",
            )

        assert result["success"] is True
        assert output_path.exists()
        data = json.loads(output_path.read_text())
        assert data["sensory_density"]["value"] == 0.7

    def test_uses_segment_prompt_builder(self, tmp_path):
        """Verifies build_segment_structure is called, not build_structure."""
        segment_path = tmp_path / "segment-tonal.md"
        segment_path.write_text("---\nsource: test.md\nsegment: tonal\nlines: 1-5\n---\n\nContent.")
        output_path = tmp_path / "segment-tonal.json"

        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"result": True}

        with patch("narrative_data.pipeline.structure.PromptBuilder") as MockPB:
            mock_pb_instance = MockPB.return_value
            mock_pb_instance.build_segment_structure.return_value = "segment prompt"
            run_segment_extraction(
                client=mock_client,
                segment_path=segment_path,
                output_path=output_path,
                schema={"type": "object"},
                segment_prompt_slug="genre-region-tonal",
            )
            mock_pb_instance.build_segment_structure.assert_called_once()
            # Verify it was NOT build_structure
            mock_pb_instance.build_structure.assert_not_called()

    def test_replace_on_retry(self, tmp_path):
        """Same replace-not-append behavior as run_structuring."""
        segment_path = tmp_path / "segment-test.md"
        segment_path.write_text("Content.")
        output_path = tmp_path / "segment-test.json"

        mock_client = MagicMock()
        # First call returns non-dict/list (trigger retry), second succeeds
        mock_client.generate_structured.side_effect = ["bad string", {"result": True}]

        with patch("narrative_data.pipeline.structure.PromptBuilder") as MockPB:
            MockPB.return_value.build_segment_structure.return_value = "prompt"
            result = run_segment_extraction(
                client=mock_client,
                segment_path=segment_path,
                output_path=output_path,
                schema={"type": "object"},
                segment_prompt_slug="test-slug",
            )

        assert result["success"] is True
