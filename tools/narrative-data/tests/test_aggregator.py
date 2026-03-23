# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for the aggregator module."""

import json

import pytest
from pydantic import ValidationError

from narrative_data.pipeline.aggregator import (
    aggregate_discovery,
    aggregate_genre_dimensions,
    load_segment_json,
)
from narrative_data.schemas.archetypes import Archetype
from narrative_data.schemas.genre_dimensions import GenreDimensions

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_json(path, data):
    path.write_text(json.dumps(data, indent=2))


def _write_genre_segments(segment_dir):
    """Write all 13 segment files for a minimal valid genre."""
    segment_dir.mkdir(parents=True, exist_ok=True)

    _write_json(
        segment_dir / "segment-meta.json",
        {
            "genre_slug": "test-horror",
            "genre_name": "Test Horror",
            "classification": "constraint_layer",
            "modifies": ["mystery"],
            "flavor_text": "A test genre.",
        },
    )

    # ContinuousAxis dimension groups
    _write_json(
        segment_dir / "segment-aesthetic.json",
        {
            "sensory_density": {"value": 0.7},
            "groundedness": {"value": 0.5},
            "aesthetic_register": {"value": 0.4},
            "prose_register": ["vernacular"],
        },
    )
    _write_json(
        segment_dir / "segment-tonal.json",
        {
            "emotional_contract": {"value": 0.9},
            "cynicism_earnestness": {"value": 0.3},
            "surface_irony": {"value": 0.1},
            "structural_irony": {"value": 0.1},
            "intimacy_distance": {"value": 0.3},
        },
    )
    _write_json(
        segment_dir / "segment-temporal.json",
        {
            "time_structure": {"value": 0.8},
            "pacing": {"value": 0.2},
            "temporal_grounding": {"value": 0.5},
            "narrative_span": {"value": 0.6},
        },
    )
    _write_json(
        segment_dir / "segment-epistemological.json",
        {
            "knowability": {"value": 0.3},
            "knowledge_reward": {"value": 0.1},
            "narration_reliability": {"value": 0.4},
        },
    )

    # WeightedTags
    _write_json(
        segment_dir / "segment-thematic.json",
        {
            "power_treatment": {"Stewardship": 0.7},
            "identity_treatment": {"Subsumed": 1.0},
            "knowledge_treatment": {"Punished": 1.0},
            "connection_treatment": {"Forced": 1.0},
        },
    )

    # AgencyDimensions
    _write_json(
        segment_dir / "segment-agency.json",
        {
            "agency_level": {"value": 0.2},
            "agency_type": "sacrifice",
            "triumph_mode": {"value": 0.1},
            "competence_relevance": {"value": 0.2},
        },
    )

    # WorldAffordances
    _write_json(
        segment_dir / "segment-world-affordances.json",
        {
            "magic": ["ambiguous"],
            "technology": "pre_industrial",
            "violence": "consequence_laden",
            "death": "permanent",
            "supernatural": "ambiguous",
        },
    )

    # Simple lists
    _write_json(
        segment_dir / "segment-locus-of-power.json",
        ["place", "system", "cosmos"],
    )
    _write_json(
        segment_dir / "segment-narrative-structure.json",
        ["tragedy", "mystery"],
    )
    _write_json(
        segment_dir / "segment-narrative-contracts.json",
        [{"invariant": "Must resolve relationship", "enforced": True}],
    )
    _write_json(
        segment_dir / "segment-state-variables.json",
        [{"canonical_id": "integration", "genre_label": "Test", "behavior": "progression"}],
    )
    _write_json(
        segment_dir / "segment-boundaries.json",
        [{"trigger": "Locus shifts", "drift_target": "gothic-horror", "description": "Test drift"}],
    )


# ---------------------------------------------------------------------------
# TestLoadSegmentJson
# ---------------------------------------------------------------------------


class TestLoadSegmentJson:
    def test_loads_dict(self, tmp_path):
        path = tmp_path / "segment-meta.json"
        _write_json(path, {"key": "value", "num": 42})
        result = load_segment_json(path)
        assert result == {"key": "value", "num": 42}

    def test_loads_list(self, tmp_path):
        path = tmp_path / "segment-list.json"
        _write_json(path, ["a", "b", "c"])
        result = load_segment_json(path)
        assert result == ["a", "b", "c"]

    def test_missing_file_raises(self, tmp_path):
        path = tmp_path / "does-not-exist.json"
        with pytest.raises(FileNotFoundError):
            load_segment_json(path)


# ---------------------------------------------------------------------------
# TestAggregateGenreDimensions
# ---------------------------------------------------------------------------


class TestAggregateGenreDimensions:
    def test_assembles_from_segments(self, tmp_path):
        _write_genre_segments(tmp_path)
        result = aggregate_genre_dimensions(tmp_path)
        assert isinstance(result, GenreDimensions)
        assert result.genre_slug == "test-horror"
        assert result.classification == "constraint_layer"
        assert result.aesthetic.sensory_density.value == 0.7
        assert result.locus_of_power == ["place", "system", "cosmos"]
        assert len(result.narrative_contracts) == 1
        assert len(result.boundaries) == 1
        assert result.modifies == ["mystery"]
        assert result.flavor_text == "A test genre."

    def test_missing_segment_raises(self, tmp_path):
        tmp_path.mkdir(exist_ok=True)
        _write_json(
            tmp_path / "segment-meta.json",
            {"genre_slug": "test", "genre_name": "Test", "classification": "constraint_layer"},
        )
        # All other segments are missing
        with pytest.raises(FileNotFoundError):
            aggregate_genre_dimensions(tmp_path)

    def test_invalid_segment_data_raises(self, tmp_path):
        _write_genre_segments(tmp_path)
        # Overwrite aesthetic with an out-of-range axis value
        _write_json(
            tmp_path / "segment-aesthetic.json",
            {
                "sensory_density": {"value": 2.0},  # > 1.0, invalid
                "groundedness": {"value": 0.5},
                "aesthetic_register": {"value": 0.4},
                "prose_register": ["vernacular"],
            },
        )
        with pytest.raises(ValidationError):
            aggregate_genre_dimensions(tmp_path)


# ---------------------------------------------------------------------------
# TestAggregateDiscovery
# ---------------------------------------------------------------------------


class TestAggregateDiscovery:
    def test_collects_entity_segments(self, tmp_path):
        tmp_path.mkdir(exist_ok=True)
        for name in ["guardian", "seeker", "shadow"]:
            _write_json(
                tmp_path / f"segment-the-{name}.json",
                {
                    "canonical_name": f"The {name.title()}",
                    "genre_slug": "test-horror",
                    "variant_name": f"Test {name.title()}",
                    "personality_profile": {
                        "warmth": 0.5,
                        "authority": 0.5,
                        "openness": 0.5,
                        "interiority": 0.5,
                        "stability": 0.5,
                        "agency": 0.5,
                        "morality": 0.5,
                    },
                    "distinguishing_tension": "Test tension",
                    "structural_necessity": "Test necessity",
                    "universality": "universal",
                },
            )
        result = aggregate_discovery(tmp_path, Archetype)
        assert len(result) == 3

    def test_validates_each_entity(self, tmp_path):
        tmp_path.mkdir(exist_ok=True)
        # One valid entity
        _write_json(
            tmp_path / "segment-the-guardian.json",
            {
                "canonical_name": "The Guardian",
                "genre_slug": "test",
                "variant_name": "Test Guardian",
                "personality_profile": {
                    "warmth": 0.5,
                    "authority": 0.5,
                    "openness": 0.5,
                    "interiority": 0.5,
                    "stability": 0.5,
                    "agency": 0.5,
                    "morality": 0.5,
                },
                "distinguishing_tension": "Test",
                "structural_necessity": "Test",
                "universality": "universal",
            },
        )
        # One invalid entity — missing required fields
        _write_json(tmp_path / "segment-the-seeker.json", {"name": "bad"})
        with pytest.raises(ValidationError):
            aggregate_discovery(tmp_path, Archetype)

    def test_excludes_manifest_file(self, tmp_path):
        """segments-manifest.json should not be picked up by the segment-* glob."""
        tmp_path.mkdir(exist_ok=True)
        _write_json(
            tmp_path / "segment-the-guardian.json",
            {
                "canonical_name": "The Guardian",
                "genre_slug": "test",
                "variant_name": "Test Guardian",
                "personality_profile": {
                    "warmth": 0.5,
                    "authority": 0.5,
                    "openness": 0.5,
                    "interiority": 0.5,
                    "stability": 0.5,
                    "agency": 0.5,
                    "morality": 0.5,
                },
                "distinguishing_tension": "Test",
                "structural_necessity": "Test",
                "universality": "universal",
            },
        )
        # This file starts with "segments-" not "segment-", so glob misses it
        _write_json(tmp_path / "segments-manifest.json", {"total": 1, "entries": []})
        result = aggregate_discovery(tmp_path, Archetype)
        assert len(result) == 1
