# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for dimension extraction from primitive entity payloads."""

from uuid import uuid4

import pytest

from narrative_data.persistence.dimension_extraction import (
    _walk_path,
    extract_dimensions,
)

# ---------------------------------------------------------------------------
# Fixtures: realistic payloads from the corpus
# ---------------------------------------------------------------------------

ENTITY_ID = uuid4()
GENRE_ID = uuid4()


def _archetype_payload() -> dict:
    """Realistic archetype payload (The Complicit Neighbor, folk-horror)."""
    return {
        "canonical_name": "The Complicit Neighbor",
        "genre_slug": "folk-horror",
        "variant_name": "Friendship vs. Self-Preservation",
        "personality_profile": {
            "warmth": 0.3,
            "authority": 0.2,
            "openness": 0.1,
            "interiority": 0.4,
            "stability": 0.6,
            "agency": 0.5,
            "morality": 0.7,
        },
        "extended_axes": {},
        "state_variables": ["community-trust"],
        "universality": "genre_unique",
    }


def _profile_payload() -> dict:
    """Realistic profile payload (The Breakdown Where Modernity Fails, folk-horror)."""
    return {
        "name": "The Breakdown Where Modernity Fails",
        "genre_slug": "folk-horror",
        "core_identity": (
            "A scene where the protagonist's reliance on modern technology"
            " is stripped away."
        ),
        "dimensional_properties": {
            "tension_signature": "spiking_explosive",
            "emotional_register": "visceral_overwhelming",
            "pacing": "slow_atmospheric",
            "cast_density": "solitary",
            "physical_dynamism": "static_contained",
            "information_flow": "withholding_accumulating",
            "resolution_tendency": "negative",
            "locus_of_power": None,
        },
        "uniqueness": "genre_unique",
    }


def _dynamics_payload() -> dict:
    """Realistic dynamics payload (Blood-Line Contract, folk-horror)."""
    return {
        "canonical_name": "Blood-Line Contract",
        "genre_slug": "folk-horror",
        "variant_name": "Orbital",
        "scale": "orbital",
        "edge_type": "Information-symmetric within the kin, asymmetric to the outsider.",
        "directionality": "bidirectional_asymmetric",
        "currencies": ["Bloodline Loyalty", "Ancestral Debt"],
        "network_position": None,
        "valence": "sacred",
        "evolution_pattern": "static",
    }


def _place_entity_payload() -> dict:
    """Realistic place_entities payload (The Ancestral Hearth, folk-horror)."""
    return {
        "canonical_name": "The Ancestral Hearth",
        "genre_slug": "folk-horror",
        "communicability": {
            "atmospheric": {
                "mood": "Warm but Claustrophobic",
                "intensity": 0.8,
                "shift_pattern": None,
            },
            "sensory": {
                "dominant": "olfactory",
                "secondary": ["auditory"],
                "description": "Cooking smells, smoke, animals.",
            },
            "spatial": {
                "enclosure": "enclosed_intimate",
                "orientation": "horizontal",
                "constraint": None,
            },
            "temporal": {
                "time_model": "linear",
                "pace_relation": "Linear but Repetitive",
            },
        },
        "entity_properties": {
            "has_agency": False,
            "is_third_character": True,
            "topological_role": "hub",
        },
    }


def _spatial_topology_payload() -> dict:
    """Realistic spatial_topology payload (folk-horror)."""
    return {
        "genre_slug": "folk-horror",
        "source_setting": "The Incubator House",
        "target_setting": "The Sentient Edge",
        "friction": {
            "type": "environmental",
            "level": "high",
            "description": "High friction transition.",
        },
        "directionality": {
            "type": "one_way",
            "forward_cost": None,
            "return_cost": None,
        },
        "agency": None,
    }


# ---------------------------------------------------------------------------
# _walk_path tests
# ---------------------------------------------------------------------------


class TestWalkPath:
    def test_simple_key(self):
        assert _walk_path({"a": 1}, "a") == 1

    def test_nested_key(self):
        assert _walk_path({"a": {"b": {"c": 42}}}, "a.b.c") == 42

    def test_missing_key_returns_none(self):
        assert _walk_path({"a": 1}, "b") is None

    def test_missing_nested_returns_none(self):
        assert _walk_path({"a": {"b": 1}}, "a.c") is None

    def test_non_dict_intermediate_returns_none(self):
        assert _walk_path({"a": "string"}, "a.b") is None

    def test_empty_payload(self):
        assert _walk_path({}, "a.b.c") is None


# ---------------------------------------------------------------------------
# Archetype extraction (normalized dimensions)
# ---------------------------------------------------------------------------


class TestArchetypeExtraction:
    def test_extracts_all_seven_personality_axes(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        assert len(rows) == 7

    def test_all_rows_are_normalized(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        assert all(r["value_type"] == "normalized" for r in rows)

    def test_all_rows_in_personality_group(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        assert all(r["dimension_group"] == "personality" for r in rows)

    def test_warmth_value(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        warmth = next(r for r in rows if r["dimension_slug"] == "warmth")
        assert warmth["numeric_value"] == pytest.approx(0.3)
        assert warmth["categorical_value"] is None
        assert warmth["complex_value"] is None

    def test_source_paths(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        slugs = {r["dimension_slug"]: r["source_path"] for r in rows}
        assert slugs["warmth"] == "personality_profile.warmth"
        assert slugs["morality"] == "personality_profile.morality"

    def test_entity_references(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        for r in rows:
            assert r["primitive_table"] == "archetypes"
            assert r["primitive_id"] == ENTITY_ID
            assert r["genre_id"] == GENRE_ID


# ---------------------------------------------------------------------------
# Profile extraction (categorical dimensions)
# ---------------------------------------------------------------------------


class TestProfileExtraction:
    def test_extracts_seven_scene_dimensions(self):
        rows = extract_dimensions("profiles", ENTITY_ID, GENRE_ID, _profile_payload())
        assert len(rows) == 7

    def test_all_rows_are_categorical(self):
        rows = extract_dimensions("profiles", ENTITY_ID, GENRE_ID, _profile_payload())
        assert all(r["value_type"] == "categorical" for r in rows)

    def test_tension_signature_value(self):
        rows = extract_dimensions("profiles", ENTITY_ID, GENRE_ID, _profile_payload())
        ts = next(r for r in rows if r["dimension_slug"] == "tension_signature")
        assert ts["categorical_value"] == "spiking_explosive"
        assert ts["numeric_value"] is None

    def test_null_field_skipped(self):
        """Null resolution_tendency is skipped; locus_of_power has no rule."""
        payload = _profile_payload()
        payload["dimensional_properties"]["resolution_tendency"] = None
        rows = extract_dimensions("profiles", ENTITY_ID, GENRE_ID, payload)
        slugs = {r["dimension_slug"] for r in rows}
        assert "resolution_tendency" not in slugs
        assert len(rows) == 6


# ---------------------------------------------------------------------------
# Dynamics extraction (categorical + set dimensions)
# ---------------------------------------------------------------------------


class TestDynamicsExtraction:
    def test_extracts_expected_dimensions(self):
        rows = extract_dimensions("dynamics", ENTITY_ID, GENRE_ID, _dynamics_payload())
        slugs = {r["dimension_slug"] for r in rows}
        assert "edge_type" in slugs
        assert "currencies" in slugs
        assert "valence" in slugs

    def test_currencies_is_set_type(self):
        rows = extract_dimensions("dynamics", ENTITY_ID, GENRE_ID, _dynamics_payload())
        currencies = next(r for r in rows if r["dimension_slug"] == "currencies")
        assert currencies["value_type"] == "set"
        assert currencies["complex_value"] == ["Bloodline Loyalty", "Ancestral Debt"]
        assert currencies["numeric_value"] is None
        assert currencies["categorical_value"] is None

    def test_null_network_position_skipped(self):
        rows = extract_dimensions("dynamics", ENTITY_ID, GENRE_ID, _dynamics_payload())
        slugs = {r["dimension_slug"] for r in rows}
        assert "network_position" not in slugs

    def test_count_excludes_nulls(self):
        """network_position is null → 5 of 6 rules produce rows."""
        rows = extract_dimensions("dynamics", ENTITY_ID, GENRE_ID, _dynamics_payload())
        assert len(rows) == 5


# ---------------------------------------------------------------------------
# Place-entities extraction (nested communicability)
# ---------------------------------------------------------------------------


class TestPlaceEntityExtraction:
    def test_extracts_six_communicability_dimensions(self):
        rows = extract_dimensions("place_entities", ENTITY_ID, GENRE_ID, _place_entity_payload())
        assert len(rows) == 6

    def test_atmospheric_mood_is_categorical(self):
        rows = extract_dimensions("place_entities", ENTITY_ID, GENRE_ID, _place_entity_payload())
        mood = next(r for r in rows if r["dimension_slug"] == "atmospheric_mood")
        assert mood["value_type"] == "categorical"
        assert mood["categorical_value"] == "Warm but Claustrophobic"

    def test_atmospheric_intensity_is_normalized(self):
        rows = extract_dimensions("place_entities", ENTITY_ID, GENRE_ID, _place_entity_payload())
        intensity = next(r for r in rows if r["dimension_slug"] == "atmospheric_intensity")
        assert intensity["value_type"] == "normalized"
        assert intensity["numeric_value"] == pytest.approx(0.8)

    def test_deep_nested_paths(self):
        rows = extract_dimensions("place_entities", ENTITY_ID, GENRE_ID, _place_entity_payload())
        enclosure = next(r for r in rows if r["dimension_slug"] == "spatial_enclosure")
        assert enclosure["source_path"] == "communicability.spatial.enclosure"
        assert enclosure["categorical_value"] == "enclosed_intimate"


# ---------------------------------------------------------------------------
# Spatial topology extraction
# ---------------------------------------------------------------------------


class TestSpatialTopologyExtraction:
    def test_extracts_friction_and_directionality(self):
        rows = extract_dimensions(
            "spatial_topology", ENTITY_ID, GENRE_ID, _spatial_topology_payload()
        )
        slugs = {r["dimension_slug"] for r in rows}
        assert "friction_type" in slugs
        assert "friction_level" in slugs
        assert "directionality_type" in slugs

    def test_null_agency_skipped(self):
        rows = extract_dimensions(
            "spatial_topology", ENTITY_ID, GENRE_ID, _spatial_topology_payload()
        )
        slugs = {r["dimension_slug"] for r in rows}
        assert "agency" not in slugs
        assert len(rows) == 3


# ---------------------------------------------------------------------------
# Genre-dimensions extraction (special)
# ---------------------------------------------------------------------------


class TestGenreDimensionsExtraction:
    def test_extracts_grouped_dimensions(self):
        payload = {
            "aesthetic": {
                "descriptive_density": 0.8,
                "color_palette": "earthy_muted",
            },
            "tonal": {
                "earnestness": 0.9,
                "emotional_contract": "dread",
            },
        }
        rows = extract_dimensions("genre_dimensions", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 4

    def test_numeric_becomes_normalized(self):
        payload = {"aesthetic": {"descriptive_density": 0.8}}
        rows = extract_dimensions("genre_dimensions", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        r = rows[0]
        assert r["value_type"] == "normalized"
        assert r["numeric_value"] == pytest.approx(0.8)
        assert r["dimension_slug"] == "aesthetic_descriptive_density"
        assert r["dimension_group"] == "aesthetic"

    def test_string_becomes_categorical(self):
        payload = {"tonal": {"emotional_contract": "dread"}}
        rows = extract_dimensions("genre_dimensions", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        r = rows[0]
        assert r["value_type"] == "categorical"
        assert r["categorical_value"] == "dread"

    def test_list_becomes_set(self):
        payload = {"identity": {"core_axes": ["autonomy", "belonging"]}}
        rows = extract_dimensions("genre_dimensions", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        assert rows[0]["value_type"] == "set"
        assert rows[0]["complex_value"] == ["autonomy", "belonging"]

    def test_skips_null_values(self):
        payload = {"aesthetic": {"color_palette": None, "density": 0.5}}
        rows = extract_dimensions("genre_dimensions", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        assert rows[0]["dimension_slug"] == "aesthetic_density"

    def test_skips_non_dict_groups(self):
        payload = {"genre_slug": "folk-horror", "aesthetic": {"density": 0.5}}
        rows = extract_dimensions("genre_dimensions", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_missing_personality_profile(self):
        """Archetype with no personality_profile → empty list."""
        payload = {"canonical_name": "Incomplete", "genre_slug": "test"}
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, payload)
        assert rows == []

    def test_partial_personality_profile(self):
        """Only warmth present → 1 row."""
        payload = {"personality_profile": {"warmth": 0.5}}
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        assert rows[0]["dimension_slug"] == "warmth"

    def test_unknown_primitive_table(self):
        """Unknown table name → empty list."""
        rows = extract_dimensions("unknown_table", ENTITY_ID, GENRE_ID, {"a": 1})
        assert rows == []

    def test_non_dict_payload(self):
        """Non-dict payload → empty list."""
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, "not a dict")
        assert rows == []

    def test_sentinel_values_skipped(self):
        """Sentinel strings like 'N/A' are treated as missing."""
        payload = {"personality_profile": {"warmth": "N/A", "authority": 0.5}}
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        assert rows[0]["dimension_slug"] == "authority"

    def test_empty_currencies_skipped(self):
        """Empty list for a set dimension → skipped."""
        payload = {"currencies": [], "edge_type": "test"}
        rows = extract_dimensions("dynamics", ENTITY_ID, GENRE_ID, payload)
        assert len(rows) == 1
        assert rows[0]["dimension_slug"] == "edge_type"

    def test_tier_defaults_to_core(self):
        rows = extract_dimensions("archetypes", ENTITY_ID, GENRE_ID, _archetype_payload())
        assert all(r["tier"] == "core" for r in rows)
