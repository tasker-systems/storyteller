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
    _extract_state_variable_interactions,
    _is_valid_value,
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
from narrative_data.persistence.trope_families import (
    build_normalization_map,
    extract_trope_families,
    normalize_family_name,
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


class _TransactionIsolatedConn:
    """Proxy wrapper around a psycopg Connection that suppresses commit() and
    autocommit assignments so all writes remain in a single transaction that
    can be rolled back on test teardown.

    Every other attribute access is forwarded to the underlying connection.
    """

    def __init__(self, conn: object) -> None:
        object.__setattr__(self, "_conn", conn)

    def commit(self) -> None:  # no-op — keep writes in the open transaction
        pass

    def rollback(self) -> None:
        object.__getattribute__(self, "_conn").rollback()

    def __getattr__(self, name: str) -> object:
        return getattr(object.__getattribute__(self, "_conn"), name)

    def __setattr__(self, name: str, value: object) -> None:
        if name == "autocommit":
            return  # already set correctly; re-setting fails when INTRANS
        object.__setattr__(object.__getattribute__(self, "_conn"), name, value)


@pytest.fixture
def db_conn():
    """Provide a transactional psycopg connection that rolls back after the test.

    Wraps the connection in _TransactionIsolatedConn so that loader code
    calling conn.commit() or setting conn.autocommit = False doesn't break
    test isolation — all writes stay in the transaction and get rolled back
    on cleanup.
    """
    import psycopg

    dsn = os.environ.get(
        "DATABASE_URL",
        "postgres://storyteller:storyteller@localhost:5435/storyteller_development",
    )
    with psycopg.connect(dsn) as conn:
        conn.autocommit = False
        yield _TransactionIsolatedConn(conn)
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

    def test_entity_slug_skips_sentinel_null(self) -> None:
        """Entities with 'null' in primary field fall back to next candidate."""
        entity = {"default_subject": "null", "canonical_name": "The Real Name"}
        assert _entity_slug(entity) == "the-real-name"

    def test_entity_slug_returns_none_when_all_sentinels(self) -> None:
        """Returns None when all candidate fields are sentinels."""
        entity = {"default_subject": "null", "name": "None"}
        assert _entity_slug(entity) is None

    def test_promoted_columns_ontological_posture_sentinel(self) -> None:
        """Sentinel 'null' in stability yields None."""
        entity = {"self_other_boundary": {"stability": "null"}}
        cols = _promoted_columns("ontological-posture", entity)
        assert cols["boundary_stability"] is None

    def test_promoted_columns_place_entities_topological_role(self) -> None:
        """Place entities extract topological_role, not type."""
        entity = {"entity_properties": {"topological_role": "hub", "has_agency": True}}
        cols = _promoted_columns("place-entities", entity)
        assert cols == {"topological_role": "hub"}

    def test_promoted_columns_place_entities_missing_role(self) -> None:
        """Place entities with no topological_role get None."""
        entity = {"entity_properties": {"has_agency": False}}
        cols = _promoted_columns("place-entities", entity)
        assert cols == {"topological_role": None}

    def test_promoted_columns_profiles_empty(self) -> None:
        """Profiles have no promoted columns (archetype_ref removed)."""
        entity = {"provenance": ["Tarot-derived"], "name": "Foo"}
        cols = _promoted_columns("profiles", entity)
        assert cols == {}

    def test_promoted_columns_tropes_with_slug_map(self) -> None:
        """Tropes: trope_family_id resolved from slug_map."""
        entity = {"genre_derivation": "Temporal Dimensions: Seasonal"}
        slug_map = {"tf:temporal-dimension": "fake-uuid-123"}
        cols = _promoted_columns("tropes", entity, slug_map=slug_map)
        assert cols["trope_family_id"] == "fake-uuid-123"

    def test_promoted_columns_tropes_no_slug_map(self) -> None:
        """Tropes without slug_map get None for family_id."""
        entity = {"genre_derivation": "Temporal: Seasonal"}
        cols = _promoted_columns("tropes", entity)
        assert cols["trope_family_id"] is None

    def test_promoted_columns_tropes_missing_derivation(self) -> None:
        """Tropes with no genre_derivation get None."""
        entity = {"name": "Some trope"}
        cols = _promoted_columns("tropes", entity, slug_map={})
        assert cols["trope_family_id"] is None


# ---------------------------------------------------------------------------
# Sentinel validation tests
# ---------------------------------------------------------------------------


class TestSentinelValidation:
    def test_rejects_none(self) -> None:
        assert _is_valid_value(None) is False

    def test_rejects_literal_null_string(self) -> None:
        assert _is_valid_value("null") is False

    def test_rejects_none_string(self) -> None:
        assert _is_valid_value("None") is False

    def test_rejects_lowercase_none(self) -> None:
        assert _is_valid_value("none") is False

    def test_rejects_empty_string(self) -> None:
        assert _is_valid_value("") is False

    def test_rejects_whitespace_padded_sentinel(self) -> None:
        assert _is_valid_value(" null ") is False

    def test_rejects_na(self) -> None:
        assert _is_valid_value("N/A") is False

    def test_accepts_valid_string(self) -> None:
        assert _is_valid_value("firm_defensive") is True

    def test_accepts_non_string_truthy(self) -> None:
        assert _is_valid_value(42) is True

    def test_accepts_list(self) -> None:
        assert _is_valid_value(["a", "b"]) is True


# ---------------------------------------------------------------------------
# Trope family normalization tests
# ---------------------------------------------------------------------------


class TestTropeFamilyNormalization:
    def test_normalize_strips_plural_dimensions(self) -> None:
        assert normalize_family_name("Temporal Dimensions") == "temporal-dimension"

    def test_normalize_strips_plural_affordances(self) -> None:
        assert normalize_family_name("World Affordances") == "world-affordance"

    def test_normalize_handles_casing(self) -> None:
        assert normalize_family_name("epistemological stance") == "epistemological-stance"

    def test_normalize_collapses_agency_variants(self) -> None:
        assert normalize_family_name("Agency Dimension") == "agency-dimension"
        assert normalize_family_name("Agency Dimensions") == "agency-dimension"

    def test_normalize_takes_part_before_colon(self) -> None:
        assert normalize_family_name("Temporal Dimensions: Seasonal") == "temporal-dimension"

    def test_normalize_locus_of_power(self) -> None:
        assert normalize_family_name("Locus of Power: Community") == "locus-of-power"

    def test_long_derivation_maps_to_unclassified(self) -> None:
        long_text = (
            "Directly addresses the Agency Dimensions and Boundary Conditions"
            " (Melodrama vs. Tragedy). The hero must be competent to fall from greatness."
        )
        assert normalize_family_name(long_text) == "unclassified"

    def test_build_normalization_map_from_corpus(self, tmp_path: Path) -> None:
        corpus_dir = tmp_path / "corpus"
        tropes_dir = corpus_dir / "genres" / "folk-horror"
        tropes_dir.mkdir(parents=True)
        tropes = [
            {"name": "T1", "genre_derivation": "Temporal Dimensions: Seasonal"},
            {"name": "T2", "genre_derivation": "Temporal Dimension: Cyclical"},
            {"name": "T3", "genre_derivation": "Locus of Power: Community"},
        ]
        (tropes_dir / "tropes.json").write_text(json.dumps(tropes))

        nmap = build_normalization_map(corpus_dir)
        assert nmap["Temporal Dimensions: Seasonal"] == "temporal-dimension"
        assert nmap["Temporal Dimension: Cyclical"] == "temporal-dimension"
        assert nmap["Locus of Power: Community"] == "locus-of-power"

    def test_extract_trope_families_deduplicates(self) -> None:
        nmap = {
            "Temporal Dimensions: Seasonal": "temporal-dimension",
            "Temporal Dimension: Cyclical": "temporal-dimension",
            "Locus of Power: Community": "locus-of-power",
        }
        families = extract_trope_families(nmap)
        slugs = [f["slug"] for f in families]
        assert len(slugs) == 2
        assert "temporal-dimension" in slugs
        assert "locus-of-power" in slugs

    def test_extract_trope_families_have_names(self) -> None:
        nmap = {"Locus of Power: X": "locus-of-power"}
        families = extract_trope_families(nmap)
        f = families[0]
        assert f["slug"] == "locus-of-power"
        assert f["name"] == "Locus of Power"
        assert "description" in f


# ---------------------------------------------------------------------------
# DB integration tests
# ---------------------------------------------------------------------------


class TestTropeFamilyLoading:
    def test_upsert_trope_families_extracts_from_corpus(self, tmp_path: Path) -> None:
        """Trope families can be extracted from minimal corpus."""
        corpus_dir = _minimal_corpus(tmp_path)
        nmap = build_normalization_map(corpus_dir)
        families = extract_trope_families(nmap)
        assert len(families) >= 1
        # _minimal_corpus has genre_derivation "Temporal: Seasonal"
        slugs = [f["slug"] for f in families]
        assert "temporal" in slugs

    @_skip_db
    def test_load_reference_entities_includes_trope_families(self, db_conn, tmp_path: Path) -> None:
        """load_reference_entities now loads trope families and includes tf: keys in slug_map."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)
        slug_map = load_reference_entities(db_conn, corpus_dir)
        db_conn.commit()

        tf_keys = [k for k in slug_map if k.startswith("tf:")]
        assert len(tf_keys) >= 1

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.trope_families")
            row = cur.fetchone()
        assert row["n"] >= 1


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
        # inserted or updated or skipped (already present with same hash) — all valid
        assert report.inserted + report.updated + report.skipped > 0

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

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            n1 = cur.fetchone()["n"]

        load_primitive_type(db_conn, "archetypes", corpus_dir, slug_map)

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            n2 = cur.fetchone()["n"]

        # Second load must not increase the count — upsert is idempotent
        assert n1 == n2

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

        # Capture baseline counts before dry-run
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.genres")
            genres_before = cur.fetchone()["n"]
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            archetypes_before = cur.fetchone()["n"]

        load_ground_state(db_conn, corpus_dir, dry_run=True)

        # Row counts must be unchanged — dry_run writes nothing
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.genres")
            genres_after = cur.fetchone()["n"]
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            archetypes_after = cur.fetchone()["n"]

        assert genres_after == genres_before
        assert archetypes_after == archetypes_before

    @_skip_db
    def test_refs_only_skips_primitives(self, db_conn, tmp_path: Path) -> None:
        """refs_only=True loads reference entities but skips primitive types."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus(tmp_path)

        # Capture archetype count before load
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            archetypes_before = cur.fetchone()["n"]

        report = load_ground_state(db_conn, corpus_dir, refs_only=True)

        # Genres should be present (upserted by refs_only load)
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT slug FROM ground_state.genres WHERE slug = 'folk-horror'")
            row = cur.fetchone()
        assert row is not None

        # Primitive tables must be untouched — refs_only skips all primitive types
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            archetypes_after = cur.fetchone()["n"]
        assert archetypes_after == archetypes_before

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

        # Capture trope count before the filtered load
        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.tropes")
            tropes_before = cur.fetchone()["n"]

        load_ground_state(db_conn, corpus_dir, types=["archetypes"])
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.archetypes")
            na = cur.fetchone()["n"]
            cur.execute("SELECT COUNT(*) AS n FROM ground_state.tropes")
            nt = cur.fetchone()["n"]

        # archetypes must be present (at least the minimal corpus archetype or pre-existing)
        assert na > 0
        # tropes must be untouched — type filter excluded them
        assert nt == tropes_before


# ---------------------------------------------------------------------------
# SQL query function tests (DB-dependent)
# ---------------------------------------------------------------------------


# ---------------------------------------------------------------------------
# State variable interaction extraction tests (no DB required)
# ---------------------------------------------------------------------------


class TestStateVariableInteractionExtraction:
    def test_extract_from_structured_type(self) -> None:
        """Extracts variable_id and operation from StateVariableInteraction shape."""
        entity = {
            "state_variable_interactions": [
                {"variable_id": "trust", "operation": "depletes", "description": "Erodes trust."},
                {"variable_id": "safety", "operation": "consumes"},
            ]
        }
        interactions = _extract_state_variable_interactions("tropes", entity)
        assert len(interactions) == 2
        assert interactions[0] == {
            "variable_slug": "trust",
            "operation": "depletes",
            "context": None,
        }
        assert interactions[1] == {
            "variable_slug": "safety",
            "operation": "consumes",
            "context": None,
        }

    def test_extract_from_dynamics(self) -> None:
        """Dynamics use the same shape as tropes."""
        entity = {
            "state_variable_interactions": [{"variable_id": "trust", "operation": "accumulates"}]
        }
        interactions = _extract_state_variable_interactions("dynamics", entity)
        assert len(interactions) == 1
        assert interactions[0]["variable_slug"] == "trust"

    def test_extract_from_goals(self) -> None:
        """Goals use the same shape as tropes."""
        entity = {"state_variable_interactions": [{"variable_id": "safety", "operation": "gates"}]}
        interactions = _extract_state_variable_interactions("goals", entity)
        assert len(interactions) == 1

    def test_extract_from_expression_type(self) -> None:
        """Place entities use StateVariableExpression shape."""
        entity = {
            "state_variable_expression": [
                {"variable_id": "trust", "physical_manifestation": "Walls closing in."}
            ]
        }
        interactions = _extract_state_variable_interactions("place-entities", entity)
        assert len(interactions) == 1
        assert interactions[0]["variable_slug"] == "trust"
        assert interactions[0]["operation"] is None
        assert interactions[0]["context"] == {"physical_manifestation": "Walls closing in."}

    def test_extract_from_bare_slugs(self) -> None:
        """Archetypes use bare list[str]."""
        entity = {"state_variables": ["trust", "safety"]}
        interactions = _extract_state_variable_interactions("archetypes", entity)
        assert len(interactions) == 2
        assert interactions[0] == {
            "variable_slug": "trust",
            "operation": None,
            "context": None,
        }
        assert interactions[1] == {
            "variable_slug": "safety",
            "operation": None,
            "context": None,
        }

    def test_extract_empty_list(self) -> None:
        entity = {"state_variable_interactions": []}
        assert _extract_state_variable_interactions("tropes", entity) == []

    def test_extract_missing_field(self) -> None:
        entity = {"name": "No interactions"}
        assert _extract_state_variable_interactions("tropes", entity) == []

    def test_extract_ignores_non_dict_items(self) -> None:
        entity = {"state_variable_interactions": ["bad", 42]}
        assert _extract_state_variable_interactions("tropes", entity) == []

    def test_extract_ignores_missing_variable_id(self) -> None:
        entity = {"state_variable_interactions": [{"operation": "depletes"}]}
        assert _extract_state_variable_interactions("tropes", entity) == []


# ---------------------------------------------------------------------------
# State variable interaction DB tests
# ---------------------------------------------------------------------------


def _minimal_corpus_with_sv_interactions(tmp_path: Path) -> Path:
    """Build corpus with state variable interactions in tropes."""
    corpus_dir = _minimal_corpus(tmp_path)
    tropes_dir = corpus_dir / "genres" / "folk-horror"
    tropes = [
        {
            "name": "The Calendar Trope",
            "genre_slug": "folk-horror",
            "genre_derivation": "Temporal: Seasonal",
            "narrative_function": ["escalating"],
            "variants": {},
            "state_variable_interactions": [
                {"variable_id": "trust", "operation": "depletes", "description": "Erodes trust."}
            ],
            "ontological_dimension": None,
            "overlap_signal": None,
            "flavor_text": "A test trope.",
        }
    ]
    (tropes_dir / "tropes.json").write_text(json.dumps(tropes))
    return corpus_dir


class TestStateVariableInteractionDB:
    @_skip_db
    def test_interactions_loaded_for_tropes(self, db_conn, tmp_path: Path) -> None:
        """State variable interactions are extracted and inserted for tropes."""
        from psycopg.rows import dict_row

        corpus_dir = _minimal_corpus_with_sv_interactions(tmp_path)
        load_ground_state(db_conn, corpus_dir)
        db_conn.commit()

        with db_conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT COUNT(*) AS n FROM ground_state.primitive_state_variable_interactions "
                "WHERE primitive_table = 'tropes'"
            )
            row = cur.fetchone()
        assert row["n"] >= 1


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

    @_skip_db
    def test_genre_context_tropes_include_family(self, db_conn, tmp_path: Path) -> None:
        """genre_context returns tropes with family_slug and family_name."""
        corpus_dir = _minimal_corpus(tmp_path)
        load_ground_state(db_conn, corpus_dir)
        db_conn.commit()

        cur = db_conn.execute("SELECT ground_state.genre_context('folk-horror')")
        result = cur.fetchone()[0]
        tropes = result.get("tropes") or []
        if tropes:
            first = tropes[0]
            assert "family_slug" in first
            assert "family_name" in first
