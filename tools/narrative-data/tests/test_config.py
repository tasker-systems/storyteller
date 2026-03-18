"""Tests for configuration and data path resolution."""

from pathlib import Path

import pytest

from narrative_data.config import (
    GENRE_CATEGORIES,
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
