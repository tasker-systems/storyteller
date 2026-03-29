# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for MODEL_REGISTRY, get_model(), and FanOutSpec."""

import pytest

from narrative_data.tome.models import MODEL_REGISTRY, FanOutSpec, get_model

# ---------------------------------------------------------------------------
# Tests: MODEL_REGISTRY
# ---------------------------------------------------------------------------


def test_model_registry_has_all_roles() -> None:
    """MODEL_REGISTRY contains entries for all three expected roles."""
    assert "fan_out_structured" in MODEL_REGISTRY
    assert "fan_out_creative" in MODEL_REGISTRY
    assert "coherence" in MODEL_REGISTRY


def test_model_registry_values_are_nonempty_strings() -> None:
    """All registry values are non-empty strings."""
    for role, model in MODEL_REGISTRY.items():
        assert isinstance(model, str), f"Role {role!r} value is not a string"
        assert model, f"Role {role!r} value is an empty string"


# ---------------------------------------------------------------------------
# Tests: get_model
# ---------------------------------------------------------------------------


def test_get_model_returns_correct_value() -> None:
    """get_model() returns the same value as direct registry lookup."""
    for role in MODEL_REGISTRY:
        assert get_model(role) == MODEL_REGISTRY[role]


def test_get_model_raises_on_unknown_role() -> None:
    """get_model() raises KeyError for an unrecognised role."""
    with pytest.raises(KeyError):
        get_model("nonexistent_role")


# ---------------------------------------------------------------------------
# Tests: FanOutSpec
# ---------------------------------------------------------------------------


def test_fanout_spec_construction() -> None:
    """FanOutSpec can be constructed with required fields."""
    spec = FanOutSpec(
        stage="places",
        index=0,
        template_name="places_template",
        model_role="fan_out_creative",
        context={"world": "test-world"},
    )
    assert spec.stage == "places"
    assert spec.index == 0
    assert spec.template_name == "places_template"
    assert spec.model_role == "fan_out_creative"
    assert spec.context == {"world": "test-world"}


def test_fanout_spec_output_filename_zero_indexed() -> None:
    """output_filename is 1-indexed and zero-padded to 3 digits."""
    spec = FanOutSpec(
        stage="places",
        index=0,
        template_name="t",
        model_role="fan_out_creative",
        context={},
    )
    assert spec.output_filename == "instance-001.json"


def test_fanout_spec_output_filename_multiple_indices() -> None:
    """output_filename correctly maps index to 1-based zero-padded filename."""
    cases = [
        (0, "instance-001.json"),
        (1, "instance-002.json"),
        (9, "instance-010.json"),
        (99, "instance-100.json"),
    ]
    for idx, expected in cases:
        spec = FanOutSpec(
            stage="s",
            index=idx,
            template_name="t",
            model_role="coherence",
            context={},
        )
        assert spec.output_filename == expected, f"index={idx}"
