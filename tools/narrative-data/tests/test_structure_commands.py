# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for structure orchestration commands — segment pipeline integration."""

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.slicer import SegmentInfo


@pytest.fixture
def mock_client() -> OllamaClient:
    return MagicMock(spec=OllamaClient)


# ---------------------------------------------------------------------------
# Helper: build a minimal valid Archetype JSON blob
# ---------------------------------------------------------------------------


def _make_archetype_json() -> dict:
    return {
        "canonical_name": "The Mentor",
        "genre_slug": "folk-horror",
        "variant_name": "The Cunning Woman",
        "personality_profile": {
            "warmth": 0.7,
            "authority": 0.8,
            "openness": 0.6,
            "interiority": 0.5,
            "stability": 0.4,
            "agency": 0.9,
            "morality": 0.3,
        },
        "distinguishing_tension": "wisdom vs. culpability",
        "structural_necessity": "transfers forbidden knowledge",
        "universality": "universal",
    }


def _make_cluster_archetype_json() -> dict:
    return {
        "canonical_name": "The Mentor",
        "cluster_name": "horror",
        "core_identity": "The one who knows too much",
        "genre_variants": [],
        "uniqueness": "universal",
    }


def _make_segment_infos(segment_dir: Path, names: list[str], source_path: Path) -> list:
    """Create SegmentInfo list and write stub segment .md files."""
    segments = []
    for name in names:
        seg_path = segment_dir / f"segment-{name}.md"
        seg_path.write_text(f"---\nsegment: {name}\n---\n\nContent for {name}.")
        segments.append(
            SegmentInfo(
                name=name,
                path=seg_path,
                source_path=source_path,
                line_start=1,
                line_end=5,
            )
        )
    return segments


# ---------------------------------------------------------------------------
# structure_type() — segment pipeline tests
# ---------------------------------------------------------------------------


class TestStructureTypeSegmented:
    """Tests for the segment pipeline wiring in structure_type()."""

    def test_slices_and_extracts_segments(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """Full pipeline: slice → extract → aggregate → write."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "folk-horror.md"
        source.write_text("# Archetypes for folk-horror\n\nContent.")

        segment_dir = genre_dir / "folk-horror"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["the-mentor", "the-fool"], source)

        from narrative_data.pipeline.structure_commands import structure_type
        from narrative_data.schemas.archetypes import Archetype

        mock_archetype = MagicMock(spec=Archetype)
        mock_archetype.model_dump.return_value = _make_archetype_json()

        with (
            patch(
                "narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos
            ) as mock_slice,
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ) as mock_extract,
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery",
                return_value=[mock_archetype, mock_archetype],
            ) as mock_aggregate,
        ):
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
            )

        mock_slice.assert_called_once()
        assert mock_extract.call_count == 2
        mock_aggregate.assert_called_once()
        assert result["succeeded"] == 1
        assert result["failed"] == 0
        assert result["skipped"] == 0

        # Verify final JSON was written
        final_json = genre_dir / "folk-horror.json"
        assert final_json.exists()
        data = json.loads(final_json.read_text())
        assert len(data) == 2

    def test_skips_cached_segments(self, mock_client: OllamaClient, tmp_output_dir: Path) -> None:
        """Segments with existing .json files are skipped when not forced."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "folk-horror.md"
        source.write_text("Content.")

        segment_dir = genre_dir / "folk-horror"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["the-mentor", "the-fool"], source)

        # Pre-create .json for one segment (cached)
        seg_infos[0].path.with_suffix(".json").write_text('{"cached": true}')

        from narrative_data.pipeline.structure_commands import structure_type
        from narrative_data.schemas.archetypes import Archetype

        mock_archetype = MagicMock(spec=Archetype)
        mock_archetype.model_dump.return_value = _make_archetype_json()

        with (
            patch("narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos),
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ) as mock_extract,
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery",
                return_value=[mock_archetype],
            ),
        ):
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
            )

        # Only 1 extraction call — the other was cached
        assert mock_extract.call_count == 1
        assert result["succeeded"] == 1

    def test_force_reextracts(self, mock_client: OllamaClient, tmp_output_dir: Path) -> None:
        """force=True re-extracts even when segment .json exists."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "folk-horror.md"
        source.write_text("Content.")

        segment_dir = genre_dir / "folk-horror"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["the-mentor"], source)

        # Pre-create .json (would normally be cached)
        seg_infos[0].path.with_suffix(".json").write_text('{"cached": true}')

        from narrative_data.pipeline.structure_commands import structure_type
        from narrative_data.schemas.archetypes import Archetype

        mock_archetype = MagicMock(spec=Archetype)
        mock_archetype.model_dump.return_value = _make_archetype_json()

        with (
            patch("narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos),
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ) as mock_extract,
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery",
                return_value=[mock_archetype],
            ),
        ):
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
                force=True,
            )

        # Should extract even though .json exists
        assert mock_extract.call_count == 1
        assert result["succeeded"] == 1

    def test_segment_failure_marks_genre_failed(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """If any segment extraction fails, the genre is marked failed."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "folk-horror.md"
        source.write_text("Content.")

        segment_dir = genre_dir / "folk-horror"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["the-mentor", "the-fool"], source)

        from narrative_data.pipeline.structure_commands import structure_type

        # First extraction succeeds, second fails
        extraction_results = [
            {"success": True, "output_path": "fake.json"},
            {"success": False, "errors": ["bad data"]},
        ]

        with (
            patch("narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos),
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                side_effect=extraction_results,
            ),
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery"
            ) as mock_aggregate,
        ):
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
            )

        # Aggregate should NOT have been called
        mock_aggregate.assert_not_called()
        assert result["failed"] == 1
        assert result["succeeded"] == 0

    def test_genre_region_uses_genre_region_aggregator(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """genre-dimensions type uses aggregate_genre_dimensions, not aggregate_discovery."""
        genre_dir = tmp_output_dir / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "region.md"
        source.write_text("# Genre Region: Folk Horror\n\nContent.")

        segment_dir = genre_dir / "region"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["meta", "aesthetic"], source)

        from narrative_data.pipeline.structure_commands import structure_type
        from narrative_data.schemas.genre_dimensions import GenreDimensions

        mock_gd = MagicMock(spec=GenreDimensions)
        mock_gd.model_dump.return_value = {"genre_slug": "folk-horror", "genre_name": "Folk Horror"}

        with (
            patch("narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos),
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ),
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_genre_dimensions",
                return_value=mock_gd,
            ) as mock_agg_gd,
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery"
            ) as mock_agg_disc,
        ):
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="genre-dimensions",
                genres=["folk-horror"],
            )

        mock_agg_gd.assert_called_once()
        mock_agg_disc.assert_not_called()
        assert result["succeeded"] == 1


# ---------------------------------------------------------------------------
# structure_type() — basic behavior tests
# ---------------------------------------------------------------------------


class TestStructureTypeSkipsMissingMd:
    def test_skips_gracefully_when_no_md(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """When the .md source file is absent, skip without error."""
        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.slice_file") as mock_slice:
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
            )

        mock_slice.assert_not_called()
        assert result["skipped"] == 1
        assert result["succeeded"] == 0
        assert result["failed"] == 0


# ---------------------------------------------------------------------------
# structure_clusters() — segment pipeline tests
# ---------------------------------------------------------------------------


class TestStructureClustersSegmented:
    def test_slices_and_extracts_cluster_segments(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """Full cluster pipeline: slice → extract → aggregate → write."""
        data_dir = tmp_output_dir / "discovery" / "archetypes"
        data_dir.mkdir(parents=True, exist_ok=True)

        from narrative_data.config import GENRE_CLUSTERS

        first_cluster = next(iter(GENRE_CLUSTERS))
        source = data_dir / f"cluster-{first_cluster}.md"
        source.write_text(f"Cluster content for {first_cluster}.")

        segment_dir = data_dir / f"cluster-{first_cluster}"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["the-mentor"], source)

        from narrative_data.pipeline.structure_commands import structure_clusters
        from narrative_data.schemas.archetypes import ClusterArchetype

        mock_cluster = MagicMock(spec=ClusterArchetype)
        mock_cluster.model_dump.return_value = _make_cluster_archetype_json()

        with (
            patch(
                "narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos
            ) as mock_slice,
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ) as mock_extract,
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery",
                return_value=[mock_cluster],
            ) as mock_aggregate,
        ):
            result = structure_clusters(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
            )

        # Only the first cluster has a source file
        mock_slice.assert_called_once()
        assert mock_extract.call_count == 1
        mock_aggregate.assert_called_once()
        assert result["succeeded"] == 1

        # Verify final JSON was written
        final_json = data_dir / f"cluster-{first_cluster}.json"
        assert final_json.exists()


class TestStructureClustersRejectsNoClusterType:
    @pytest.mark.parametrize("type_slug", ["tropes", "narrative-shapes", "genre-dimensions"])
    def test_raises_for_no_cluster_schema(
        self, mock_client: OllamaClient, tmp_output_dir: Path, type_slug: str
    ) -> None:
        """Types without cluster schemas raise ValueError."""
        from narrative_data.pipeline.structure_commands import structure_clusters

        with pytest.raises(ValueError, match="cluster schema"):
            structure_clusters(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug=type_slug,
            )


# ---------------------------------------------------------------------------
# plan_only tests
# ---------------------------------------------------------------------------


class TestPlanOnlyNoWrites:
    def test_plan_only_prints_but_no_extraction(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """plan_only=True prints what would be done but does not slice or extract."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        (genre_dir / "folk-horror.md").write_text("Content.")

        from narrative_data.pipeline.structure_commands import structure_type

        with (
            patch("narrative_data.pipeline.structure_commands.slice_file") as mock_slice,
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction"
            ) as mock_extract,
        ):
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
                plan_only=True,
            )

        mock_slice.assert_not_called()
        mock_extract.assert_not_called()
        assert result["succeeded"] == 0
        assert result["skipped"] == 1


# ---------------------------------------------------------------------------
# genre-native type path resolution tests (tropes, narrative-shapes)
# ---------------------------------------------------------------------------


class TestGenreNativePathResolution:
    def test_tropes_resolves_to_genre_subdir(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """Tropes live at genres/{genre}/tropes.md and use tropes doc_type."""
        genre_dir = tmp_output_dir / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "tropes.md"
        source.write_text("Folk horror tropes content.")

        segment_dir = genre_dir / "tropes"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["the-wicker-man"], source)

        from narrative_data.pipeline.structure_commands import structure_type
        from narrative_data.schemas.tropes import Trope

        mock_trope = MagicMock(spec=Trope)
        mock_trope.model_dump.return_value = {"name": "The Wicker Man"}

        with (
            patch(
                "narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos
            ) as mock_slice,
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ),
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_discovery",
                return_value=[mock_trope],
            ),
        ):
            structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="tropes",
                genres=["folk-horror"],
            )

        mock_slice.assert_called_once()
        # Verify slice was called with tropes doc_type
        call_kwargs = mock_slice.call_args
        assert call_kwargs[0][2] == "tropes"  # doc_type positional arg


class TestGenreDimensionsPathResolution:
    def test_genre_dimensions_resolves_to_region_file(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """GenreDimensions live at genres/{genre}/region.md and use genre-region doc_type."""
        genre_dir = tmp_output_dir / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True, exist_ok=True)
        source = genre_dir / "region.md"
        source.write_text("# Genre Region: Folk Horror\n\nRegion content.")

        segment_dir = genre_dir / "region"
        segment_dir.mkdir(parents=True, exist_ok=True)
        seg_infos = _make_segment_infos(segment_dir, ["meta"], source)

        from narrative_data.pipeline.structure_commands import structure_type
        from narrative_data.schemas.genre_dimensions import GenreDimensions

        mock_gd = MagicMock(spec=GenreDimensions)
        mock_gd.model_dump.return_value = {"genre_slug": "folk-horror"}

        with (
            patch(
                "narrative_data.pipeline.structure_commands.slice_file", return_value=seg_infos
            ) as mock_slice,
            patch(
                "narrative_data.pipeline.structure_commands.run_segment_extraction",
                return_value={"success": True, "output_path": "fake.json"},
            ),
            patch(
                "narrative_data.pipeline.structure_commands.aggregate_genre_dimensions",
                return_value=mock_gd,
            ),
        ):
            structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="genre-dimensions",
                genres=["folk-horror"],
            )

        mock_slice.assert_called_once()
        call_kwargs = mock_slice.call_args
        assert call_kwargs[0][2] == "genre-region"  # doc_type positional arg


# ---------------------------------------------------------------------------
# _resolve_segment_config() tests
# ---------------------------------------------------------------------------


class TestResolveSegmentConfig:
    def test_genre_region_known_segment(self) -> None:
        from narrative_data.pipeline.structure_commands import (
            TYPE_REGISTRY,
            _resolve_segment_config,
        )

        config = TYPE_REGISTRY["genre-dimensions"]
        schema, prompt = _resolve_segment_config(config, "aesthetic")
        assert prompt == "genre-region-aesthetic"
        assert "properties" in schema  # It's a Pydantic JSON schema

    def test_genre_region_unknown_segment_raises(self) -> None:
        from narrative_data.pipeline.structure_commands import (
            TYPE_REGISTRY,
            _resolve_segment_config,
        )

        config = TYPE_REGISTRY["genre-dimensions"]
        with pytest.raises(ValueError, match="Unknown genre-region segment"):
            _resolve_segment_config(config, "nonexistent")

    def test_discovery_type(self) -> None:
        from narrative_data.pipeline.structure_commands import (
            TYPE_REGISTRY,
            _resolve_segment_config,
        )

        config = TYPE_REGISTRY["archetypes"]
        schema, prompt = _resolve_segment_config(config, "the-mentor")
        assert prompt == "discovery-entity"
        assert isinstance(schema, dict)

    def test_tropes_type(self) -> None:
        from narrative_data.pipeline.structure_commands import (
            TYPE_REGISTRY,
            _resolve_segment_config,
        )

        config = TYPE_REGISTRY["tropes"]
        schema, prompt = _resolve_segment_config(config, "the-wicker-man")
        assert prompt == "trope-entity"

    def test_narrative_shapes_type(self) -> None:
        from narrative_data.pipeline.structure_commands import (
            TYPE_REGISTRY,
            _resolve_segment_config,
        )

        config = TYPE_REGISTRY["narrative-shapes"]
        schema, prompt = _resolve_segment_config(config, "the-spiral")
        assert prompt == "narrative-shape-entity"
