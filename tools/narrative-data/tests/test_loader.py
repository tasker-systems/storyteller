# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for database connection management, reference data extraction, and ground-state loader."""

import json
import os
from pathlib import Path

import pytest

from narrative_data.persistence.connection import get_connection_string
from narrative_data.persistence.loader import (
    LoadReport,
    _entity_slug,
    _promoted_columns,
    _source_hash,
    load_ground_state,
    load_primitive_type,
    load_reference_entities,
)
from narrative_data.persistence.reference_data import (
    extract_cluster_metadata,
    extract_dimensions,
    extract_genres,
    extract_state_variables,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_region(
    genre_dir: Path,
    slug: str,
    name: str,
    state_variables: list[dict],
    flavor_text: str | None = None,
) -> None:
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


def _minimal_corpus(tmp_path: Path) -> Path:
    """Build a minimal corpus directory with two genres and a few primitive files."""
    corpus_dir = tmp_path / "corpus"

    # Genres
    _write_region(
        corpus_dir / "genres" / "folk-horror",
        slug="folk-horror",
        name="Folk Horror",
        state_variables=[
            {
                "canonical_id": "trust",
                "genre_label": "Trust",
                "behavior": "fluctuating",
                "initial_value": 0.5,
                "threshold": 0.2,
                "threshold_effect": "Betrayal triggers crisis.",
                "activation_condition": None,
            }
        ],
        flavor_text="Ancient rural dread.",
    )
    _write_region(
        corpus_dir / "genres" / "cozy-fantasy",
        slug="cozy-fantasy",
        name="Cozy Fantasy",
        state_variables=[],
    )

    # Discovery archetypes for folk-horror
    discovery_dir = corpus_dir / "discovery" / "archetypes"
    discovery_dir.mkdir(parents=True)
    archetypes = [
        {
            "canonical_name": "The Complicit Neighbor",
            "genre_slug": "folk-horror",
            "variant_name": "Friendship vs Self-Preservation",
            "personality_profile": {},
            "extended_axes": {},
            "distinguishing_tension": "Trust vs survival.",
            "structural_necessity": "Essential for community trust.",
            "overlap_signals": [],
            "state_variables": [],
            "universality": "genre_unique",
            "flavor_text": "A test archetype.",
        }
    ]
    (discovery_dir / "folk-horror.json").write_text(json.dumps(archetypes))

    # Genre-native tropes for folk-horror
    tropes_dir = corpus_dir / "genres" / "folk-horror"
    tropes = [
        {
            "name": "The Calendar Trope",
            "genre_slug": "folk-horror",
            "genre_derivation": "Temporal: Seasonal",
            "narrative_function": ["escalating"],
            "variants": {},
            "state_variable_interactions": [],
            "ontological_dimension": None,
            "overlap_signal": None,
            "flavor_text": "A test trope.",
        }
    ]
    (tropes_dir / "tropes.json").write_text(json.dumps(tropes))

    return corpus_dir


# ---------------------------------------------------------------------------
# DB fixture
# ---------------------------------------------------------------------------


def _db_available() -> bool:
    """Return True if DATABASE_URL is set and PostgreSQL is reachable."""
    dsn = os.environ.get("DATABASE_URL")
    if not dsn:
        return False
    try:
        import psycopg

        with psycopg.connect(dsn, connect_timeout=2) as conn:
            conn.execute("SELECT 1")
        return True
    except Exception:
        return False


_skip_db = pytest.mark.skipif(not _db_available(), reason="PostgreSQL not available")


@pytest.fixture
def db_conn():
    """Provide a transactional psycopg connection that rolls back after the test."""
    import psycopg

    dsn = os.environ.get(
        "DATABASE_URL",
        "postgres://storyteller:storyteller@localhost:5435/storyteller_development",
    )
    with psycopg.connect(dsn) as conn:
        conn.autocommit = False
        yield conn
        conn.rollback()


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
        _write_region(
            corpus_dir / "genres" / "folk-horror",
            "folk-horror",
            "Folk Horror",
            [shared_sv, unique_sv],
        )
        _write_region(
            corpus_dir / "genres" / "cosmic-horror",
            "cosmic-horror",
            "Cosmic Horror",
            [shared_sv],
        )

        svars = extract_state_variables(corpus_dir)
        slugs = [sv["slug"] for sv in svars]

        assert len(svars) == 2
        assert "trust" in slugs
        assert "sanity" in slugs
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
        expected = {
            "aesthetic",
            "tonal",
            "temporal",
            "thematic",
            "agency",
            "world",
            "epistemological",
        }
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
        assert "tonal" in groups


# ---------------------------------------------------------------------------
# Unit tests for loader helpers (no DB required)
# ---------------------------------------------------------------------------


class TestLoaderHelpers:
    def test_source_hash_is_sha256_hex(self) -> None:
        """_source_hash returns a 64-char hex string."""
        h = _source_hash(b"hello world")
        assert len(h) == 64
        assert all(c in "0123456789abcdef" for c in h)

    def test_source_hash_deterministic(self) -> None:
        """Same bytes produce same hash."""
        raw = b'{"key": "value"}'
        assert _source_hash(raw) == _source_hash(raw)

    def test_source_hash_differs_for_different_bytes(self) -> None:
        """Different bytes produce different hashes."""
        assert _source_hash(b"a") != _source_hash(b"b")

    def test_entity_slug_from_canonical_name(self) -> None:
        """canonical_name takes priority for slug derivation."""
        entity = {"canonical_name": "The Complicit Neighbor", "name": "Other"}
        assert _entity_slug(entity) == "the-complicit-neighbor"

    def test_entity_slug_from_name_fallback(self) -> None:
        """name is used when canonical_name is absent."""
        entity = {"name": "Calendar Trope"}
        assert _entity_slug(entity) == "calendar-trope"

    def test_entity_slug_from_pairing_name(self) -> None:
        """pairing_name is used for archetype-dynamics entities."""
        entity = {"pairing_name": "Ghost of Escape"}
        assert _entity_slug(entity) == "ghost-of-escape"

    def test_entity_slug_lowercases_and_hyphenates(self) -> None:
        """Slugification: lowercase, spaces→hyphens, underscores→hyphens."""
        entity = {"name": "Blood Line Contract"}
        slug = _entity_slug(entity)
        assert slug == "blood-line-contract"

    def test_entity_slug_returns_none_when_no_name_field(self) -> None:
        """Returns None for entities with no recognised name field."""
        assert _entity_slug({}) is None
        assert _entity_slug({"description": "no name"}) is None

    def test_load_report_addition(self) -> None:
        """LoadReport.__add__ sums all fields."""
        a = LoadReport(inserted=2, updated=1, skipped=3, errors=1)
        b = LoadReport(inserted=1, updated=2, skipped=0, errors=0)
        c = a + b
        assert c.inserted == 3
        assert c.updated == 3
        assert c.skipped == 3
        assert c.errors == 1

    def test_load_report_summary_format(self) -> None:
        """LoadReport.summary() includes all counts."""
        r = LoadReport(inserted=5, updated=2, pruned=1, skipped=3, errors=0)
        s = r.summary()
        assert "inserted=5" in s
        assert "updated=2" in s
        assert "pruned=1" in s
        assert "skipped=3" in s
        assert "errors=0" in s

    def test_promoted_columns_archetypes(self) -> None:
        """Archetypes: archetype_family maps from universality."""
        entity = {"universality": "genre_unique"}
        cols = _promoted_columns("archetypes", entity)
        assert cols["archetype_family"] == "genre_unique"
        assert "primary_scale" in cols

    def test_promoted_columns_goals(self) -> None:
        """Goals: goal_scale maps from scale field."""
        entity = {"scale": "arc"}
        cols = _promoted_columns("goals", entity)
        assert cols["goal_scale"] == "arc"

    def test_promoted_columns_narrative_shapes(self) -> None:
        """Narrative shapes: shape_type from tension_profile.family, beat_count from beats."""
        entity = {
            "tension_profile": {"family": "spiral"},
            "beats": [1, 2, 3, 4, 5, 6],
        }
        cols = _promoted_columns("narrative-shapes", entity)
        assert cols["shape_type"] == "spiral"
        assert cols["beat_count"] == 6

    def test_promoted_columns_ontological_posture(self) -> None:
        """Ontological posture: boundary_stability from self_other_boundary.stability."""
        entity = {"self_other_boundary": {"stability": "firm_defensive"}}
        cols = _promoted_columns("ontological-posture", entity)
        assert cols["boundary_stability"] == "firm_defensive"

    def test_promoted_columns_spatial_topology(self) -> None:
        """Spatial topology: friction_type and directionality_type extracted."""
        entity = {
            "friction": {"type": "environmental", "level": "high"},
            "directionality": {"type": "one_way"},
        }
        cols = _promoted_columns("spatial-topology", entity)
        assert cols["friction_type"] == "environmental"
        assert cols["directionality_type"] == "one_way"

    def test_promoted_columns_archetype_dynamics(self) -> None:
        """Archetype dynamics: archetype_a and archetype_b extracted directly."""
        entity = {"archetype_a": "The Vessel", "archetype_b": "The Exile"}
        cols = _promoted_columns("archetype-dynamics", entity)
        assert cols["archetype_a"] == "The Vessel"
        assert cols["archetype_b"] == "The Exile"

    def test_promoted_columns_unknown_type(self) -> None:
        """Unknown type returns empty dict."""
        cols = _promoted_columns("unknown-type", {"key": "val"})
        assert cols == {}


# ---------------------------------------------------------------------------
# DB integration tests
# ---------------------------------------------------------------------------


class TestLoadReferenceEntities:
    @_skip_db
    def test_genres_loaded_into_db(self, db_conn, tmp_path: Path) -> None:
        """load_reference_entities inserts genres and returns slug→UUID map."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        assert "folk-horror" in slug_map
        assert "cozy-fantasy" in slug_map

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT slug, name FROM ground_state.genres WHERE slug = %s", ("folk-horror",)
            )
            row = cur.fetchone()
        assert row is not None
        assert row["name"] == "Folk Horror"

    @_skip_db
    def test_clusters_loaded_with_members(self, db_conn, tmp_path: Path) -> None:
        """Cluster rows and genre_cluster_members are populated."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.genre_cluster_members")
            row = cur.fetchone()
        assert row["n"] > 0

    @_skip_db
    def test_state_variables_loaded(self, db_conn, tmp_path: Path) -> None:
        """State variables are loaded from region.json files."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT slug, name FROM ground_state.state_variables WHERE slug = %s", ("trust",)
            )
            row = cur.fetchone()
        assert row is not None
        assert row["name"] == "Trust"

    @_skip_db
    def test_dimensions_loaded(self, db_conn, tmp_path: Path) -> None:
        """All 34+ dimensions are loaded."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.dimensions")
            row = cur.fetchone()
        assert row["n"] >= 34

    @_skip_db
    def test_reference_load_is_idempotent(self, db_conn, tmp_path: Path) -> None:
        """Running load_reference_entities twice does not raise or duplicate rows."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()
        load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT COUNT(*) AS n FROM ground_state.genres"
                " WHERE slug IN ('folk-horror', 'cozy-fantasy')"
            )
            row = cur.fetchone()
        assert row["n"] == 2  # Not doubled


class TestLoadPrimitiveType:
    @_skip_db
    def test_archetypes_loaded(self, db_conn, tmp_path: Path) -> None:
        """Archetypes from discovery/archetypes/{genre}.json are loaded."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        report = load_primitive_type(db_conn, "archetypes", corpus_dir, slug_map)

        assert report.errors == 0
        assert report.inserted > 0 or report.updated > 0

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT entity_slug, name FROM ground_state.archetypes "
                "WHERE entity_slug = 'the-complicit-neighbor'"
            )
            row = cur.fetchone()
        assert row is not None
        assert row["name"] == "The Complicit Neighbor"

    @_skip_db
    def test_tropes_loaded_from_genre_native(self, db_conn, tmp_path: Path) -> None:
        """Tropes from genres/{genre}/tropes.json are loaded."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        report = load_primitive_type(db_conn, "tropes", corpus_dir, slug_map)

        assert report.errors == 0

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT entity_slug FROM ground_state.tropes"
                " WHERE entity_slug = 'the-calendar-trope'"
            )
            row = cur.fetchone()
        assert row is not None

    @_skip_db
    def test_load_primitive_type_idempotent(self, db_conn, tmp_path: Path) -> None:
        """Loading the same type twice results in same row count (idempotent)."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        load_primitive_type(db_conn, "archetypes", corpus_dir, slug_map)
        load_primitive_type(db_conn, "archetypes", corpus_dir, slug_map)

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            row = cur.fetchone()
        # Should not double the count
        assert row["n"] == 1

    @_skip_db
    def test_unknown_genre_slug_is_skipped(self, db_conn, tmp_path: Path) -> None:
        """Entities whose genre_slug is not in slug_map are skipped without error."""
        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        # Write a file with an unknown genre
        discovery_dir = corpus_dir / "discovery" / "archetypes"
        discovery_dir.mkdir(parents=True, exist_ok=True)
        unknown_data = [
            {
                "canonical_name": "Ghost Archetype",
                "genre_slug": "nonexistent-genre",
                "universality": "genre_unique",
                "flavor_text": "Should be skipped.",
            }
        ]
        (discovery_dir / "nonexistent-genre.json").write_text(json.dumps(unknown_data))

        report = load_primitive_type(db_conn, "archetypes", corpus_dir, slug_map)
        assert report.errors == 0
        assert report.skipped >= 1


class TestLoadGroundState:
    @_skip_db
    def test_dry_run_does_not_write(self, db_conn, tmp_path: Path) -> None:
        """dry_run=True reports counts without inserting rows."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_ground_state(db_conn, corpus_dir, dry_run=True)

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.genres")
            row = cur.fetchone()
        # In a transactional test session, genres may have been loaded by other tests
        # but the dry-run-specific genres should NOT appear
        # We verify by checking the folk-horror genre was NOT written by this run
        cur.execute("SELECT COUNT(*) AS n FROM ground_state.genres WHERE slug = 'folk-horror'")
        row = cur.fetchone()
        assert row["n"] == 0

    @_skip_db
    def test_refs_only_skips_primitives(self, db_conn, tmp_path: Path) -> None:
        """refs_only=True loads reference entities but skips primitive types."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        report = load_ground_state(db_conn, corpus_dir, refs_only=True)

        # Genres should be present
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT slug FROM ground_state.genres WHERE slug = 'folk-horror'")
            row = cur.fetchone()
        assert row is not None

        # No primitive rows
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            row = cur.fetchone()
        assert row["n"] == 0

        assert report.inserted == 0
        assert report.updated == 0

    @_skip_db
    def test_full_load_then_reload_is_idempotent(self, db_conn, tmp_path: Path) -> None:
        """Loading twice produces the same DB state."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_ground_state(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            n1 = cur.fetchone()["n"]

        load_ground_state(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            n2 = cur.fetchone()["n"]

        assert n1 == n2

    @_skip_db
    def test_type_filter_limits_load(self, db_conn, tmp_path: Path) -> None:
        """types=['archetypes'] loads only archetypes, not tropes."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        load_ground_state(db_conn, corpus_dir, types=["archetypes"])
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            na = cur.fetchone()["n"]
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.tropes")
            nt = cur.fetchone()["n"]

        assert na > 0
        assert nt == 0


# ---------------------------------------------------------------------------
# SQL query function tests (DB-dependent)
# ---------------------------------------------------------------------------


class TestQueryFunctions:
    @_skip_db
    def test_genre_context_returns_all_types(self, db_conn) -> None:
        """genre_context('folk-horror') returns non-null result with genre_slug and archetypes."""
        cur = db_conn.execute("SELECT ground_state.genre_context('folk-horror')")
        result = cur.fetchone()[0]
        assert result is not None
        assert result["genre_slug"] == "folk-horror"
        assert result["genre"] is not None
        assert isinstance(result["archetypes"], list)
        assert len(result["archetypes"]) > 0

    @_skip_db
    def test_genre_context_returns_null_for_unknown(self, db_conn) -> None:
        """genre_context returns NULL for a genre slug that does not exist."""
        cur = db_conn.execute("SELECT ground_state.genre_context('nonexistent')")
        result = cur.fetchone()[0]
        assert result is None

    @_skip_db
    def test_genre_context_wraps_with_slug(self, db_conn) -> None:
        """Each element in the archetypes array has 'slug' and 'data' keys."""
        cur = db_conn.execute("SELECT ground_state.genre_context('folk-horror')")
        result = cur.fetchone()[0]
        first = result["archetypes"][0]
        assert "slug" in first
        assert "data" in first
