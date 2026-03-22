"""Tests for structure orchestration commands (Task 9)."""

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from narrative_data.ollama import OllamaClient


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


# ---------------------------------------------------------------------------
# structure_type() tests
# ---------------------------------------------------------------------------


class TestStructureTypeProcessesGenres:
    def test_produces_json_files(self, mock_client: OllamaClient, tmp_output_dir: Path) -> None:
        """Given .md source files, structure_type() calls run_structuring and produces .json."""
        # Create source .md files
        for genre in ["folk-horror", "cosmic-horror"]:
            genre_dir = tmp_output_dir / "discovery" / "archetypes"
            genre_dir.mkdir(parents=True, exist_ok=True)
            (genre_dir / f"{genre}.md").write_text(f"# Archetypes for {genre}\n\nContent.")

        mock_client.generate_structured.return_value = [_make_archetype_json()]

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            mock_structure.return_value = {"success": True, "output_path": "fake.json"}
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror", "cosmic-horror"],
            )

        assert mock_structure.call_count == 2
        assert result["succeeded"] == 2
        assert result["failed"] == 0
        assert result["skipped"] == 0


class TestStructureTypeSkipsMissingMd:
    def test_skips_gracefully_when_no_md(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """When the .md source file is absent, skip without error."""
        # No files created

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
            )

        mock_structure.assert_not_called()
        assert result["skipped"] == 1
        assert result["succeeded"] == 0
        assert result["failed"] == 0


class TestStructureTypeSkipsCached:
    def test_skips_when_manifest_has_entry(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """When the manifest has a completed entry and force=False, skip the genre."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        (genre_dir / "folk-horror.md").write_text("Content.")

        # Pre-populate manifest with completed entry
        manifest_path = tmp_output_dir / "structure_manifest.json"
        manifest = {"entries": {"archetypes/folk-horror": {"success": True}}}
        manifest_path.write_text(json.dumps(manifest))

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
                force=False,
            )

        mock_structure.assert_not_called()
        assert result["skipped"] == 1


class TestStructureTypeForceOverridesCache:
    def test_force_true_ignores_manifest(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """force=True re-runs structuring even when manifest has a cached entry."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        (genre_dir / "folk-horror.md").write_text("Content.")

        manifest_path = tmp_output_dir / "structure_manifest.json"
        manifest = {"entries": {"archetypes/folk-horror": {"success": True}}}
        manifest_path.write_text(json.dumps(manifest))

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            mock_structure.return_value = {"success": True, "output_path": "fake.json"}
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
                force=True,
            )

        mock_structure.assert_called_once()
        assert result["succeeded"] == 1


# ---------------------------------------------------------------------------
# structure_clusters() tests
# ---------------------------------------------------------------------------


class TestStructureClustersProcessesClusters:
    def test_produces_cluster_json(self, mock_client: OllamaClient, tmp_output_dir: Path) -> None:
        """Given cluster .md files, structure_clusters() invokes run_structuring per cluster."""
        data_dir = tmp_output_dir / "discovery" / "archetypes"
        data_dir.mkdir(parents=True, exist_ok=True)

        from narrative_data.config import GENRE_CLUSTERS

        for cluster_name in GENRE_CLUSTERS:
            (data_dir / f"cluster-{cluster_name}.md").write_text(
                f"Cluster content for {cluster_name}."
            )

        from narrative_data.pipeline.structure_commands import structure_clusters

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            mock_structure.return_value = {"success": True, "output_path": "fake.json"}
            result = structure_clusters(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
            )

        assert mock_structure.call_count == len(GENRE_CLUSTERS)
        assert result["succeeded"] == len(GENRE_CLUSTERS)
        assert result["failed"] == 0


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
    def test_plan_only_prints_but_no_run_structuring(
        self, mock_client: OllamaClient, tmp_output_dir: Path, capsys
    ) -> None:
        """plan_only=True prints what would be done but does not call run_structuring."""
        genre_dir = tmp_output_dir / "discovery" / "archetypes"
        genre_dir.mkdir(parents=True, exist_ok=True)
        (genre_dir / "folk-horror.md").write_text("Content.")

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            result = structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="archetypes",
                genres=["folk-horror"],
                plan_only=True,
            )

        mock_structure.assert_not_called()
        # plan_only results are counted as skipped (or planned)
        assert result["succeeded"] == 0


# ---------------------------------------------------------------------------
# genre-native type path resolution tests (tropes, narrative-shapes)
# ---------------------------------------------------------------------------


class TestGenreNativePathResolution:
    def test_tropes_resolves_to_genre_subdir(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """Tropes live at genres/{genre}/tropes.md."""
        genre_dir = tmp_output_dir / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True, exist_ok=True)
        (genre_dir / "tropes.md").write_text("Folk horror tropes content.")

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            mock_structure.return_value = {"success": True, "output_path": "fake.json"}
            structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="tropes",
                genres=["folk-horror"],
            )

        mock_structure.assert_called_once()
        # Verify the raw_path argument points to the genres dir
        call_kwargs = mock_structure.call_args.kwargs
        assert "genres" in str(call_kwargs["raw_path"])
        assert "tropes.md" in str(call_kwargs["raw_path"])


class TestGenreDimensionsPathResolution:
    def test_genre_dimensions_resolves_to_region_file(
        self, mock_client: OllamaClient, tmp_output_dir: Path
    ) -> None:
        """GenreDimensions live at genres/{genre}/region.md."""
        genre_dir = tmp_output_dir / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True, exist_ok=True)
        (genre_dir / "region.md").write_text("Folk horror region content.")

        from narrative_data.pipeline.structure_commands import structure_type

        with patch("narrative_data.pipeline.structure_commands.run_structuring") as mock_structure:
            mock_structure.return_value = {"success": True, "output_path": "fake.json"}
            structure_type(
                client=mock_client,
                output_base=tmp_output_dir,
                type_slug="genre-dimensions",
                genres=["folk-horror"],
            )

        mock_structure.assert_called_once()
        call_kwargs = mock_structure.call_args.kwargs
        assert "region.md" in str(call_kwargs["raw_path"])
        # genre-dimensions is not a collection
        assert call_kwargs["is_collection"] is False
