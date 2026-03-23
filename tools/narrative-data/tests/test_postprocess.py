# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for pipeline.postprocess — corpus null-rate audit."""

import json
from pathlib import Path

import pytest

from narrative_data.pipeline.postprocess import (
    AuditResult,
    _flatten_fields,
    _is_null_or_empty,
    audit_corpus,
    audit_type,
)

FIXTURES_DIR = Path(__file__).parent / "fixtures"


# ---------------------------------------------------------------------------
# Unit tests for helper functions
# ---------------------------------------------------------------------------


class TestIsNullOrEmpty:
    def test_none_is_null(self):
        assert _is_null_or_empty(None) is True

    def test_empty_string_is_null(self):
        assert _is_null_or_empty("") is True

    def test_whitespace_string_is_null(self):
        assert _is_null_or_empty("   ") is True

    def test_non_empty_string_not_null(self):
        assert _is_null_or_empty("hello") is False

    def test_empty_list_is_null(self):
        assert _is_null_or_empty([]) is True

    def test_non_empty_list_not_null(self):
        assert _is_null_or_empty(["a"]) is False

    def test_zero_is_not_null(self):
        assert _is_null_or_empty(0) is False

    def test_false_is_not_null(self):
        assert _is_null_or_empty(False) is False

    def test_dict_not_null(self):
        assert _is_null_or_empty({}) is False

    def test_integer_not_null(self):
        assert _is_null_or_empty(42) is False


class TestFlattenFields:
    def test_flat_dict(self):
        result = _flatten_fields({"a": 1, "b": "hello"})
        assert result == {"a": 1, "b": "hello"}

    def test_nested_dict(self):
        result = _flatten_fields({"outer": {"inner": "value"}})
        assert result == {"outer.inner": "value"}

    def test_deeply_nested(self):
        result = _flatten_fields({"a": {"b": {"c": 42}}})
        assert result == {"a.b.c": 42}

    def test_list_value_kept_as_leaf(self):
        result = _flatten_fields({"items": [1, 2, 3]})
        assert result == {"items": [1, 2, 3]}

    def test_empty_list_kept_as_leaf(self):
        result = _flatten_fields({"items": []})
        assert result == {"items": []}

    def test_none_value_kept_as_leaf(self):
        result = _flatten_fields({"x": None})
        assert result == {"x": None}

    def test_mixed_nested_and_flat(self):
        result = _flatten_fields({"top": "val", "nested": {"a": 1, "b": None}})
        assert result == {"top": "val", "nested.a": 1, "nested.b": None}

    def test_prefix_applied(self):
        result = _flatten_fields({"x": 1}, prefix="outer")
        assert result == {"outer.x": 1}


# ---------------------------------------------------------------------------
# audit_type — single file
# ---------------------------------------------------------------------------


class TestAuditTypeWithRealFixtures:
    def test_dynamics_fixture_loads(self):
        path = FIXTURES_DIR / "dynamics_folk_horror.json"
        result = audit_type("dynamics", path)
        assert isinstance(result, AuditResult)
        assert result.type_name == "dynamics"
        assert result.total_entities == 3
        assert result.file_count == 1
        assert len(result.errors) == 0

    def test_dynamics_fixture_null_rates_in_range(self):
        path = FIXTURES_DIR / "dynamics_folk_horror.json"
        result = audit_type("dynamics", path)
        for field_name, rate in result.field_rates.items():
            assert 0.0 <= rate <= 1.0, f"Rate out of range for {field_name}: {rate}"

    def test_dynamics_known_null_fields(self):
        """Fields consistently null across all 3 fixture entities should have rate 1.0."""
        path = FIXTURES_DIR / "dynamics_folk_horror.json"
        result = audit_type("dynamics", path)
        # network_position is null in all 3 entities
        assert result.field_rates.get("network_position") == 1.0
        # valence is null in all 3 entities
        assert result.field_rates.get("valence") == 1.0

    def test_dynamics_known_present_fields(self):
        """Fields present in all entities should have rate 0.0."""
        path = FIXTURES_DIR / "dynamics_folk_horror.json"
        result = audit_type("dynamics", path)
        # canonical_name is always present
        assert result.field_rates.get("canonical_name") == 0.0
        # directionality is always present
        assert result.field_rates.get("directionality") == 0.0

    def test_region_fixture_loads(self):
        path = FIXTURES_DIR / "region_folk_horror.json"
        result = audit_type("region", path)
        assert result.total_entities == 1
        assert result.file_count == 1
        assert len(result.errors) == 0

    def test_region_nested_fields_flattened(self):
        path = FIXTURES_DIR / "region_folk_horror.json"
        result = audit_type("region", path)
        # Nested field via dot notation
        assert "aesthetic.sensory_density.value" in result.field_rates
        assert "tonal.emotional_contract.flavor_text" in result.field_rates

    def test_region_null_constraint_layer_type(self):
        path = FIXTURES_DIR / "region_folk_horror.json"
        result = audit_type("region", path)
        # constraint_layer_type is null in fixture
        assert result.field_rates.get("constraint_layer_type") == 1.0


class TestAuditTypeEdgeCases:
    def test_missing_file_returns_error(self, tmp_path: Path):
        path = tmp_path / "nonexistent.json"
        result = audit_type("dynamics", path)
        assert result.total_entities == 0
        assert result.file_count == 0
        assert len(result.errors) == 1
        assert "not found" in result.errors[0].lower()

    def test_invalid_json_returns_error(self, tmp_path: Path):
        bad_file = tmp_path / "bad.json"
        bad_file.write_text("this is not json {{{")
        result = audit_type("dynamics", bad_file)
        assert result.total_entities == 0
        assert len(result.errors) == 1

    def test_empty_array_produces_zero_entities(self, tmp_path: Path):
        empty_file = tmp_path / "empty.json"
        empty_file.write_text("[]")
        result = audit_type("dynamics", empty_file)
        assert result.total_entities == 0
        assert result.file_count == 1
        assert result.field_rates == {}

    def test_single_entity_dict(self, tmp_path: Path):
        entity_file = tmp_path / "single.json"
        entity_file.write_text(json.dumps({"name": "Foo", "value": None}))
        result = audit_type("test-type", entity_file)
        assert result.total_entities == 1
        assert result.field_rates["name"] == 0.0
        assert result.field_rates["value"] == 1.0

    def test_partial_null_rates(self, tmp_path: Path):
        """2 of 4 entities have null in a field → rate should be 0.5."""
        data = [
            {"name": "A", "desc": "present"},
            {"name": "B", "desc": None},
            {"name": "C", "desc": "present"},
            {"name": "D", "desc": None},
        ]
        f = tmp_path / "partial.json"
        f.write_text(json.dumps(data))
        result = audit_type("test-type", f)
        assert result.field_rates["desc"] == pytest.approx(0.5)
        assert result.field_rates["name"] == 0.0

    def test_empty_string_counted_as_null(self, tmp_path: Path):
        data = [{"name": ""}, {"name": "present"}]
        f = tmp_path / "empty_str.json"
        f.write_text(json.dumps(data))
        result = audit_type("test-type", f)
        assert result.field_rates["name"] == pytest.approx(0.5)

    def test_empty_list_counted_as_null(self, tmp_path: Path):
        data = [{"tags": []}, {"tags": ["a", "b"]}]
        f = tmp_path / "empty_list.json"
        f.write_text(json.dumps(data))
        result = audit_type("test-type", f)
        assert result.field_rates["tags"] == pytest.approx(0.5)


# ---------------------------------------------------------------------------
# audit_corpus — aggregate across files
# ---------------------------------------------------------------------------


class TestAuditCorpus:
    def _make_corpus(self, tmp_path: Path) -> Path:
        """Create a minimal corpus directory mimicking the real layout."""
        corpus = tmp_path / "narrative-data"

        # Discovery types: corpus/discovery/{type}/{genre}.json
        dynamics_dir = corpus / "discovery" / "dynamics"
        dynamics_dir.mkdir(parents=True)
        (dynamics_dir / "folk-horror.json").write_text(
            json.dumps(
                [
                    {"canonical_name": "Entity A", "valence": None, "scale": "scene"},
                    {"canonical_name": "Entity B", "valence": "positive", "scale": "arc"},
                ]
            )
        )
        (dynamics_dir / "cosmic-horror.json").write_text(
            json.dumps(
                [
                    {"canonical_name": "Entity C", "valence": None, "scale": "orbital"},
                ]
            )
        )

        # Genre-native types: corpus/genres/{genre}/{type}.json
        folk_horror_dir = corpus / "genres" / "folk-horror"
        folk_horror_dir.mkdir(parents=True)
        (folk_horror_dir / "tropes.json").write_text(
            json.dumps([{"name": "Trope A", "description": "desc A"}])
        )
        cosmic_dir = corpus / "genres" / "cosmic-horror"
        cosmic_dir.mkdir(parents=True)
        (cosmic_dir / "tropes.json").write_text(
            json.dumps([{"name": "Trope B", "description": None}])
        )

        return corpus

    def test_returns_dict_keyed_by_type(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics"])
        assert "dynamics" in results

    def test_aggregate_entity_count(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics"])
        # 2 from folk-horror + 1 from cosmic-horror = 3
        assert results["dynamics"].total_entities == 3

    def test_aggregate_file_count(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics"])
        assert results["dynamics"].file_count == 2

    def test_aggregate_null_rate_valence(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics"])
        # valence: 2 null out of 3 entities = 0.667
        assert results["dynamics"].field_rates["valence"] == pytest.approx(2 / 3)

    def test_genre_native_type_scans_genres_dir(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["tropes"])
        assert results["tropes"].total_entities == 2
        assert results["tropes"].file_count == 2

    def test_genre_native_null_rate(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["tropes"])
        # description: 1 null (Trope B) out of 2 entities = 0.5
        assert results["tropes"].field_rates["description"] == pytest.approx(0.5)

    def test_genre_filter(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics"], genres=["folk-horror"])
        assert results["dynamics"].total_entities == 2
        assert results["dynamics"].file_count == 1

    def test_missing_type_returns_no_files_error(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["goals"])
        assert results["goals"].total_entities == 0
        assert len(results["goals"].errors) >= 1

    def test_multiple_types(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics", "tropes"])
        assert "dynamics" in results
        assert "tropes" in results

    def test_corpus_genre_none_for_aggregate(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        results = audit_corpus(corpus, types=["dynamics"])
        # Aggregate result has no single genre
        assert results["dynamics"].genre is None
