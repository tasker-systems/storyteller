"""Tests for configuration and data path resolution."""

from pathlib import Path

import pytest

from narrative_data.config import (
    GENRE_CATEGORIES,
    GENRE_CLUSTERS,
    GENRE_NATIVE_TYPES,
    MODIFIER_REGIONS,
    PRIMITIVE_TYPES,
    SPATIAL_CATEGORIES,
    resolve_data_path,
    resolve_descriptor_dir,
    resolve_output_path,
)


class TestPathResolution:
    def test_resolve_data_path_from_env(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
        desc_dir = tmp_path / "training-data" / "descriptors"
        desc_dir.mkdir(parents=True)
        monkeypatch.setenv("STORYTELLER_DATA_PATH", str(tmp_path))
        assert resolve_data_path() == tmp_path

    def test_resolve_data_path_missing_env(self, monkeypatch: pytest.MonkeyPatch):
        monkeypatch.delenv("STORYTELLER_DATA_PATH", raising=False)
        with pytest.raises(RuntimeError, match="STORYTELLER_DATA_PATH"):
            resolve_data_path()

    def test_resolve_output_path(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
        monkeypatch.setenv("STORYTELLER_DATA_PATH", str(tmp_path))
        output = resolve_output_path()
        assert output == tmp_path / "narrative-data"

    def test_descriptor_dir(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
        desc_dir = tmp_path / "training-data" / "descriptors"
        desc_dir.mkdir(parents=True)
        monkeypatch.setenv("STORYTELLER_DATA_PATH", str(tmp_path))
        assert resolve_descriptor_dir() == desc_dir


class TestConstants:
    def test_genre_categories(self):
        assert "region" in GENRE_CATEGORIES
        assert "archetypes" in GENRE_CATEGORIES
        assert "tropes" in GENRE_CATEGORIES

    def test_spatial_categories(self):
        assert "setting-type" in SPATIAL_CATEGORIES
        assert "place-entities" in SPATIAL_CATEGORIES
        assert "topology" in SPATIAL_CATEGORIES


class TestNewConstants:
    def test_genre_clusters_cover_all_standalone_regions(self):
        from narrative_data.genre.commands import GENRE_REGIONS

        all_clustered = set()
        for genres in GENRE_CLUSTERS.values():
            all_clustered.update(genres)
        standalone = set(GENRE_REGIONS) - set(MODIFIER_REGIONS)
        assert standalone == all_clustered

    def test_primitive_types_are_strings(self):
        assert all(isinstance(t, str) for t in PRIMITIVE_TYPES)
        assert "archetypes" in PRIMITIVE_TYPES
        assert len(PRIMITIVE_TYPES) == 9
        assert "ontological-posture" in PRIMITIVE_TYPES
        assert "archetype-dynamics" in PRIMITIVE_TYPES
        assert "spatial-topology" in PRIMITIVE_TYPES
        assert "place-entities" in PRIMITIVE_TYPES

    def test_genre_native_types(self):
        assert "tropes" in GENRE_NATIVE_TYPES
        assert "narrative-shapes" in GENRE_NATIVE_TYPES
        assert len(GENRE_NATIVE_TYPES) == 2

    def test_modifier_regions(self):
        assert "solarpunk" in MODIFIER_REGIONS
        assert len(MODIFIER_REGIONS) == 4
