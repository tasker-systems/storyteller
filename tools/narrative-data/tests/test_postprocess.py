# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for pipeline.postprocess — corpus null-rate audit and deterministic fills."""

import json
from pathlib import Path

import pytest

from narrative_data.pipeline.postprocess import (
    AuditResult,
    _flatten_fields,
    _is_null_or_empty,
    audit_corpus,
    audit_type,
    fill_agency,
    fill_all_deterministic,
    fill_spans_scales,
)
from narrative_data.schemas import (
    archetype_dynamics,
    archetypes,
    dynamics,
    genre_dimensions,
    goals,
    narrative_shapes,
    ontological_posture,
    place_entities,
    scene_profiles,
    settings,
    spatial_topology,
    tropes,
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


# ---------------------------------------------------------------------------
# fill_spans_scales — extract Scale patterns from source markdown
# ---------------------------------------------------------------------------


FOLK_HORROR_MD = (Path(__file__).parent / "fixtures" / "dynamics_folk_horror.md").read_text()


class TestFillSpansScales:
    def _entity(self, spans_scales: list) -> dict:
        return {"canonical_name": "Blood-Line Contract", "spans_scales": spans_scales}

    def test_spanning_orbital_scene(self):
        entity = self._entity([])
        result = fill_spans_scales(entity, FOLK_HORROR_MD, "Blood-Line Contract")
        assert result["spans_scales"] == ["orbital", "scene"]

    def test_cross_scale_orbital_vs_scene(self):
        entity = {"canonical_name": "The Dissolution of Self", "spans_scales": []}
        result = fill_spans_scales(entity, FOLK_HORROR_MD, "The Dissolution of Self")
        assert "orbital" in result["spans_scales"]
        assert "scene" in result["spans_scales"]

    def test_primary_secondary_pattern(self):
        entity = {"canonical_name": "The Hearth's Trap", "spans_scales": []}
        result = fill_spans_scales(entity, FOLK_HORROR_MD, "The Hearth's Trap")
        assert "orbital" in result["spans_scales"]
        assert "scene" in result["spans_scales"]

    def test_skip_when_already_populated(self):
        entity = {"canonical_name": "Blood-Line Contract", "spans_scales": ["arc"]}
        result = fill_spans_scales(entity, FOLK_HORROR_MD, "Blood-Line Contract")
        # Already populated — must not change
        assert result["spans_scales"] == ["arc"]

    def test_idempotent(self):
        entity = {"canonical_name": "Blood-Line Contract", "spans_scales": []}
        first = fill_spans_scales(entity, FOLK_HORROR_MD, "Blood-Line Contract")
        second = fill_spans_scales(first, FOLK_HORROR_MD, "Blood-Line Contract")
        assert first["spans_scales"] == second["spans_scales"]

    def test_no_match_returns_entity_unchanged(self):
        entity = {"canonical_name": "Unknown Dynamic", "spans_scales": []}
        result = fill_spans_scales(entity, FOLK_HORROR_MD, "Unknown Dynamic")
        assert result["spans_scales"] == []

    def test_single_scale_no_spans(self):
        md = "# Single Thing\n*   **Scale:** Orbital\n"
        entity = {"canonical_name": "Single Thing", "spans_scales": []}
        result = fill_spans_scales(entity, md, "Single Thing")
        # A single scale with no parenthetical — should not populate spans_scales
        # (it doesn't span multiple scales)
        assert result["spans_scales"] == []

    def test_returns_new_dict_not_mutated(self):
        entity = {"canonical_name": "Blood-Line Contract", "spans_scales": []}
        original_spans = entity["spans_scales"]
        result = fill_spans_scales(entity, FOLK_HORROR_MD, "Blood-Line Contract")
        # Original list must not be mutated
        assert original_spans == []
        assert result is not entity or result["spans_scales"] is not original_spans


# ---------------------------------------------------------------------------
# fill_agency — infer agency from friction + directionality
# ---------------------------------------------------------------------------


class TestFillAgency:
    def _entity(self, friction_type: str, directionality_type: str, agency=None) -> dict:
        return {
            "agency": agency,
            "friction": {"type": friction_type, "level": "severe", "description": None},
            "directionality": {
                "type": directionality_type,
                "forward_cost": None,
                "reverse_cost": None,
            },
        }

    def test_high_friction_one_way_is_none(self):
        entity = self._entity("high", "one_way")
        result = fill_agency(entity)
        assert result["agency"] == "none"

    def test_high_friction_unidirectional_is_none(self):
        entity = self._entity("high", "unidirectional")
        result = fill_agency(entity)
        assert result["agency"] == "none"

    def test_high_friction_bidirectional_is_low(self):
        entity = self._entity("high", "bidirectional")
        result = fill_agency(entity)
        assert result["agency"] == "low"

    def test_medium_friction_bidirectional_is_medium(self):
        entity = self._entity("medium", "bidirectional")
        result = fill_agency(entity)
        assert result["agency"] == "medium"

    def test_low_friction_bidirectional_is_high(self):
        entity = self._entity("low", "bidirectional")
        result = fill_agency(entity)
        assert result["agency"] == "high"

    def test_low_friction_constrained_is_illusion(self):
        entity = self._entity("low", "constrained")
        result = fill_agency(entity)
        assert result["agency"] == "illusion"

    def test_medium_friction_constrained_is_illusion(self):
        entity = self._entity("medium", "constrained")
        result = fill_agency(entity)
        assert result["agency"] == "illusion"

    def test_hyphen_variant_one_way(self):
        entity = self._entity("high", "one-way")
        result = fill_agency(entity)
        assert result["agency"] == "none"

    def test_skip_when_already_populated(self):
        entity = self._entity("high", "one_way", agency="high")
        result = fill_agency(entity)
        # Pre-existing value must be preserved
        assert result["agency"] == "high"

    def test_skip_when_agency_is_empty_string(self):
        # Empty string is treated as "not populated" — fill it
        entity = self._entity("high", "one_way")
        entity["agency"] = ""
        result = fill_agency(entity)
        assert result["agency"] == "none"

    def test_unknown_combo_returns_none_agency(self):
        entity = self._entity("extreme", "wormhole")
        result = fill_agency(entity)
        # No lookup match — agency remains None
        assert result["agency"] is None

    def test_missing_friction_key_returns_entity_unchanged(self):
        entity = {"agency": None, "directionality": {"type": "bidirectional"}}
        result = fill_agency(entity)
        assert result["agency"] is None


# ---------------------------------------------------------------------------
# fill_all_deterministic — orchestration with fixture corpus
# ---------------------------------------------------------------------------


class TestFillAllDeterministic:
    def _make_corpus(self, tmp_path: Path) -> Path:
        """Build minimal corpus with dynamics and spatial-topology files."""
        corpus = tmp_path / "narrative-data"

        # dynamics source markdown + JSON
        dynamics_dir = corpus / "discovery" / "dynamics"
        dynamics_dir.mkdir(parents=True)
        (dynamics_dir / "folk-horror.md").write_text(FOLK_HORROR_MD)
        dynamics_entities = [
            {
                "canonical_name": "Blood-Line Contract",
                "spans_scales": [],
                "scale": "orbital",
            },
            {
                "canonical_name": "The Dissolution of Self",
                "spans_scales": [],
                "scale": "arc",
            },
        ]
        (dynamics_dir / "folk-horror.json").write_text(json.dumps(dynamics_entities))

        # spatial-topology JSON
        spatial_dir = corpus / "discovery" / "spatial-topology"
        spatial_dir.mkdir(parents=True)
        spatial_entities = [
            {
                "agency": None,
                "friction": {"type": "high", "level": "severe", "description": None},
                "directionality": {"type": "one_way", "forward_cost": None},
            },
            {
                "agency": None,
                "friction": {"type": "low", "level": "mild", "description": None},
                "directionality": {"type": "bidirectional", "forward_cost": None},
            },
        ]
        (spatial_dir / "folk-horror.json").write_text(json.dumps(spatial_entities))

        return corpus

    def test_dynamics_spans_scales_filled(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        summary = fill_all_deterministic(corpus, types=["dynamics"], genres=None, dry_run=False)
        # At least one entity should have been updated
        assert summary["dynamics"]["entities_updated"] >= 1

    def test_spatial_topology_agency_filled(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        summary = fill_all_deterministic(
            corpus, types=["spatial-topology"], genres=None, dry_run=False
        )
        assert summary["spatial-topology"]["entities_updated"] >= 1

    def test_dry_run_does_not_write(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        spatial_path = corpus / "discovery" / "spatial-topology" / "folk-horror.json"
        original = spatial_path.read_text()

        fill_all_deterministic(corpus, types=["spatial-topology"], genres=None, dry_run=True)

        assert spatial_path.read_text() == original

    def test_dry_run_still_reports_changes(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        summary = fill_all_deterministic(
            corpus, types=["spatial-topology"], genres=None, dry_run=True
        )
        assert summary["spatial-topology"]["entities_updated"] >= 1

    def test_genre_filter_limits_files(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        # Add a second genre file that would also be touched
        spatial_dir = corpus / "discovery" / "spatial-topology"
        extra = [
            {"agency": None, "friction": {"type": "high"}, "directionality": {"type": "one_way"}}
        ]
        (spatial_dir / "cosmic-horror.json").write_text(json.dumps(extra))

        summary = fill_all_deterministic(
            corpus, types=["spatial-topology"], genres=["folk-horror"], dry_run=False
        )
        # Only folk-horror processed — cosmic-horror not touched
        assert summary["spatial-topology"]["files_processed"] == 1

    def test_idempotent_second_run_no_changes(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        fill_all_deterministic(corpus, types=["spatial-topology"], genres=None, dry_run=False)
        summary2 = fill_all_deterministic(
            corpus, types=["spatial-topology"], genres=None, dry_run=False
        )
        assert summary2["spatial-topology"]["entities_updated"] == 0

    def test_summary_keys_present(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        summary = fill_all_deterministic(corpus, types=["dynamics"], genres=None, dry_run=False)
        assert "dynamics" in summary
        result = summary["dynamics"]
        assert "files_processed" in result
        assert "entities_updated" in result
        assert "entities_skipped" in result


# ---------------------------------------------------------------------------
# TestSchemaTiering — every field in every per-genre model has a tier annotation
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "model_cls",
    [
        # archetypes
        archetypes.Archetype,
        archetypes.ClusterArchetype,
        archetypes.PersonalityProfile,
        # dynamics
        dynamics.Dynamic,
        dynamics.ClusterDynamic,
        dynamics.RoleSlot,
        dynamics.ScaleManifestations,
        # goals
        goals.Goal,
        goals.ClusterGoal,
        goals.CrossScaleTension,
        # settings
        settings.Settings,
        settings.ClusterSettings,
        settings.SettingCommunicability,
        # place_entities
        place_entities.PlaceEntity,
        place_entities.ClusterPlaceEntity,
        place_entities.AtmosphericChannel,
        place_entities.SensoryChannel,
        place_entities.SpatialChannel,
        place_entities.TemporalChannel,
        place_entities.PlaceCommunicability,
        place_entities.EntityProperties,
        place_entities.StateVariableExpression,
        # scene_profiles
        scene_profiles.SceneProfile,
        scene_profiles.ClusterSceneProfile,
        scene_profiles.SceneDimensionalProperties,
        # ontological_posture
        ontological_posture.OntologicalPosture,
        ontological_posture.ClusterOntologicalPosture,
        ontological_posture.ModeOfBeing,
        ontological_posture.SelfOtherBoundary,
        ontological_posture.EthicalOrientation,
        # spatial_topology
        spatial_topology.SpatialTopologyEdge,
        spatial_topology.ClusterSpatialTopology,
        spatial_topology.TopologyFriction,
        spatial_topology.TopologyDirectionality,
        spatial_topology.TonalInheritance,
        spatial_topology.TraversalCost,
        # archetype_dynamics
        archetype_dynamics.ArchetypeDynamic,
        archetype_dynamics.ClusterArchetypeDynamic,
        archetype_dynamics.EdgeProperties,
        archetype_dynamics.CharacteristicScene,
        archetype_dynamics.ShadowPairing,
        # genre_dimensions
        genre_dimensions.GenreDimensions,
        genre_dimensions.AestheticDimensions,
        genre_dimensions.TonalDimensions,
        genre_dimensions.TemporalDimensions,
        genre_dimensions.ThematicDimensions,
        genre_dimensions.AgencyDimensions,
        genre_dimensions.WorldAffordances,
        genre_dimensions.EpistemologicalDimensions,
        genre_dimensions.NarrativeContract,
        # tropes
        tropes.Trope,
        tropes.TropeVariants,
        # narrative_shapes
        narrative_shapes.NarrativeShape,
        narrative_shapes.TensionProfile,
        narrative_shapes.Beat,
        narrative_shapes.RestBeat,
        narrative_shapes.Composability,
        narrative_shapes.ShapeOverlapSignal,
    ],
)
class TestSchemaTiering:
    def test_all_fields_have_tier_annotation(self, model_cls):
        for field_name, field_info in model_cls.model_fields.items():
            extra = field_info.json_schema_extra or {}
            assert "tier" in extra, (
                f"{model_cls.__name__}.{field_name} missing tier annotation"
            )
            assert extra["tier"] in ("core", "extended"), (
                f"{model_cls.__name__}.{field_name} tier must be 'core' or 'extended', "
                f"got {extra['tier']!r}"
            )
