# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for database connection management and reference data extraction."""

import json
from pathlib import Path

import pytest

from narrative_data.persistence.connection import get_connection_string
from narrative_data.persistence.reference_data import (
    extract_cluster_metadata,
    extract_dimensions,
    extract_genres,
    extract_state_variables,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_region(genre_dir: Path, slug: str, name: str, state_variables: list[dict],
                  flavor_text: str | None = None) -> None:
    """Write a minimal region.json for a genre directory."""
    genre_dir.mkdir(parents=True, exist_ok=True)
    region = {
        "genre_slug": slug,
        "genre_name": name,
        "classification": "standalone_region",
        "constraint_layer_type": None,
        "active_state_variables": state_variables,
        "flavor_text": flavor_text,
        "modifies": [],
    }
    (genre_dir / "region.json").write_text(json.dumps(region))


# ---------------------------------------------------------------------------
# Connection tests
# ---------------------------------------------------------------------------


class TestConnection:
    def test_reads_database_url_from_env(self, monkeypatch):
        monkeypatch.setenv("DATABASE_URL", "postgres://test:test@localhost:5435/test_db")
        assert get_connection_string() == "postgres://test:test@localhost:5435/test_db"

    def test_raises_without_database_url(self, monkeypatch):
        monkeypatch.delenv("DATABASE_URL", raising=False)
        with pytest.raises(ValueError, match="DATABASE_URL"):
            get_connection_string()


# ---------------------------------------------------------------------------
# Reference data extraction tests
# ---------------------------------------------------------------------------


class TestReferenceDataExtraction:
    def test_extracts_genres_from_corpus(self, tmp_path: Path) -> None:
        """Walking corpus_dir/genres/*/region.json produces genre dicts."""
        corpus_dir = tmp_path / "corpus"
        _write_region(
            corpus_dir / "genres" / "folk-horror",
            slug="folk-horror",
            name="Folk Horror",
            state_variables=[],
            flavor_text="Ancient rural dread.",
        )

        genres = extract_genres(corpus_dir)

        assert len(genres) == 1
        g = genres[0]
        assert g["slug"] == "folk-horror"
        assert g["name"] == "Folk Horror"
        assert g["description"] == "Ancient rural dread."
        assert g["payload"]["genre_slug"] == "folk-horror"

    def test_extracts_multiple_genres_sorted(self, tmp_path: Path) -> None:
        """Multiple genres are returned in directory-sorted order."""
        corpus_dir = tmp_path / "corpus"
        _write_region(corpus_dir / "genres" / "urban-fantasy", "urban-fantasy", "Urban Fantasy", [])
        _write_region(corpus_dir / "genres" / "cozy-mystery", "cozy-mystery", "Cozy Mystery", [])

        genres = extract_genres(corpus_dir)
        slugs = [g["slug"] for g in genres]

        assert slugs == sorted(slugs)
        assert len(genres) == 2

    def test_extracts_genre_with_null_flavor_text(self, tmp_path: Path) -> None:
        """Genres with no flavor_text have description=None."""
        corpus_dir = tmp_path / "corpus"
        _write_region(corpus_dir / "genres" / "cyberpunk", "cyberpunk", "Cyberpunk", [])

        genres = extract_genres(corpus_dir)
        assert genres[0]["description"] is None

    def test_returns_empty_list_when_genres_dir_missing(self, tmp_path: Path) -> None:
        """extract_genres returns [] when genres dir does not exist."""
        genres = extract_genres(tmp_path / "nonexistent")
        assert genres == []

    def test_ignores_non_directory_entries_in_genres(self, tmp_path: Path) -> None:
        """Files directly inside genres/ are skipped."""
        corpus_dir = tmp_path / "corpus"
        genres_dir = corpus_dir / "genres"
        genres_dir.mkdir(parents=True)
        (genres_dir / "README.md").write_text("docs")
        _write_region(corpus_dir / "genres" / "western", "westerns", "Westerns", [])

        genres = extract_genres(corpus_dir)
        assert len(genres) == 1
        assert genres[0]["slug"] == "western"

    def test_extracts_state_variables_deduped(self, tmp_path: Path) -> None:
        """State variables from multiple genres are deduplicated by canonical_id."""
        corpus_dir = tmp_path / "corpus"
        shared_sv = {
            "canonical_id": "trust",
            "genre_label": "Trust",
            "behavior": "fluctuating",
            "initial_value": 0.5,
            "threshold": 0.2,
            "threshold_effect": "Betrayal triggers a crisis.",
            "activation_condition": None,
        }
        unique_sv = {
            "canonical_id": "sanity",
            "genre_label": "Sanity",
            "behavior": "depleting",
            "initial_value": 0.9,
            "threshold": 0.1,
            "threshold_effect": None,
            "activation_condition": None,
        }
        _write_region(corpus_dir / "genres" / "folk-horror", "folk-horror", "Folk Horror",
                      [shared_sv, unique_sv])
        _write_region(corpus_dir / "genres" / "cosmic-horror", "cosmic-horror", "Cosmic Horror",
                      [shared_sv])

        svars = extract_state_variables(corpus_dir)
        slugs = [sv["slug"] for sv in svars]

        assert len(svars) == 2
        assert "trust" in slugs
        assert "sanity" in slugs
        # No duplicates
        assert len(slugs) == len(set(slugs))

    def test_state_variable_dict_shape(self, tmp_path: Path) -> None:
        """Each state variable dict has the expected keys."""
        corpus_dir = tmp_path / "corpus"
        sv = {
            "canonical_id": "safety",
            "genre_label": "Safety",
            "behavior": "depleting",
            "initial_value": 0.8,
            "threshold": 0.1,
            "threshold_effect": "Danger escalates.",
            "activation_condition": None,
        }
        _write_region(corpus_dir / "genres" / "thriller", "thriller", "Thriller", [sv])

        svars = extract_state_variables(corpus_dir)
        assert len(svars) == 1
        s = svars[0]
        assert s["slug"] == "safety"
        assert s["name"] == "Safety"
        assert s["description"] == "Danger escalates."
        assert s["default_range"] == {"initial_value": 0.8, "threshold": 0.1}
        assert s["payload"]["canonical_id"] == "safety"

    def test_state_variable_uses_activation_condition_as_fallback(self, tmp_path: Path) -> None:
        """When threshold_effect is None, activation_condition is used as description."""
        corpus_dir = tmp_path / "corpus"
        sv = {
            "canonical_id": "momentum",
            "genre_label": "Momentum",
            "behavior": "accumulating",
            "initial_value": 0.0,
            "threshold": None,
            "threshold_effect": None,
            "activation_condition": "Triggered after first victory.",
        }
        _write_region(corpus_dir / "genres" / "adventure", "adventure", "Adventure", [sv])

        svars = extract_state_variables(corpus_dir)
        assert svars[0]["description"] == "Triggered after first victory."

    def test_returns_empty_state_variables_when_corpus_missing(self, tmp_path: Path) -> None:
        """extract_state_variables returns [] when genres dir does not exist."""
        result = extract_state_variables(tmp_path / "nonexistent")
        assert result == []

    def test_extracts_cluster_metadata(self) -> None:
        """extract_cluster_metadata returns 6 clusters from config."""
        clusters = extract_cluster_metadata()
        assert len(clusters) == 6
        assert all("slug" in c and "name" in c and "genres" in c for c in clusters)

    def test_cluster_slugs_have_prefix(self) -> None:
        """Each cluster slug starts with 'cluster-'."""
        clusters = extract_cluster_metadata()
        for c in clusters:
            assert c["slug"].startswith("cluster-"), f"Bad slug: {c['slug']}"

    def test_cluster_genres_are_non_empty_lists(self) -> None:
        """Every cluster has at least one member genre."""
        clusters = extract_cluster_metadata()
        for c in clusters:
            assert isinstance(c["genres"], list)
            assert len(c["genres"]) > 0, f"Empty genres for cluster {c['slug']}"

    def test_cluster_contains_known_genres(self) -> None:
        """Fantasy cluster contains cozy-fantasy; horror cluster contains folk-horror."""
        clusters = extract_cluster_metadata()
        by_slug = {c["slug"]: c for c in clusters}
        assert "cozy-fantasy" in by_slug["cluster-fantasy"]["genres"]
        assert "folk-horror" in by_slug["cluster-horror"]["genres"]

    def test_extracts_dimensions(self) -> None:
        """extract_dimensions returns at least 34 dimensions."""
        dims = extract_dimensions()
        assert len(dims) >= 34

    def test_dimensions_have_required_keys(self) -> None:
        """Every dimension dict has slug, name, dimension_group, description."""
        dims = extract_dimensions()
        for d in dims:
            assert "slug" in d, f"Missing slug in {d}"
            assert "name" in d, f"Missing name in {d}"
            assert "dimension_group" in d, f"Missing dimension_group in {d}"
            assert "description" in d, f"Missing description in {d}"

    def test_dimension_groups_include_expected_set(self) -> None:
        """All expected dimension groups appear in the inventory."""
        dims = extract_dimensions()
        groups = {d["dimension_group"] for d in dims}
        expected = {"aesthetic", "tonal", "temporal", "thematic", "agency", "world", "epistemological"}
        for group in expected:
            assert group in groups, f"Missing group: {group}"

    def test_dimension_slugs_are_unique(self) -> None:
        """No two dimensions share a slug."""
        dims = extract_dimensions()
        slugs = [d["slug"] for d in dims]
        assert len(slugs) == len(set(slugs)), "Duplicate dimension slugs found"

    def test_personality_group_absent_uses_tonal_or_structural(self) -> None:
        """Personality axes map to tonal group (cynicism, intimacy etc.) per schema."""
        dims = extract_dimensions()
        groups = {d["dimension_group"] for d in dims}
        # The schema uses 'tonal' for personality-adjacent axes; 'personality' is not a schema group
        assert "tonal" in groups
