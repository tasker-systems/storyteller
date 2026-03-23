# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for shared schema primitives."""

import pytest
from pydantic import ValidationError


class TestContinuousAxis:
    def test_valid_axis(self):
        from narrative_data.schemas.shared import ContinuousAxis

        axis = ContinuousAxis(value=0.7, low_label="Spare", high_label="Lush")
        assert axis.value == 0.7
        assert axis.flavor_text is None

    def test_value_below_zero_rejected(self):
        from narrative_data.schemas.shared import ContinuousAxis

        with pytest.raises(ValidationError):
            ContinuousAxis(value=-0.1)

    def test_value_above_one_rejected(self):
        from narrative_data.schemas.shared import ContinuousAxis

        with pytest.raises(ValidationError):
            ContinuousAxis(value=1.1)

    def test_minimal_axis(self):
        from narrative_data.schemas.shared import ContinuousAxis

        axis = ContinuousAxis(value=0.5)
        assert axis.can_be_state_variable is False

    def test_round_trip(self):
        from narrative_data.schemas.shared import ContinuousAxis

        axis = ContinuousAxis(
            value=0.3,
            low_label="Low",
            high_label="High",
            can_be_state_variable=True,
            flavor_text="test",
        )
        data = axis.model_dump()
        restored = ContinuousAxis.model_validate(data)
        assert restored == axis


class TestWeightedTags:
    def test_valid_tags(self):
        from narrative_data.schemas.shared import WeightedTags

        tags = WeightedTags(root={"Stewardship": 0.7, "Corruption": 0.3})
        assert tags.root["Stewardship"] == 0.7

    def test_value_out_of_range_rejected(self):
        from narrative_data.schemas.shared import WeightedTags

        with pytest.raises(ValidationError):
            WeightedTags(root={"Bad": 1.5})


class TestStateVariableTemplate:
    def test_valid_template(self):
        from narrative_data.schemas.shared import StateVariableTemplate

        sv = StateVariableTemplate(
            canonical_id="V1",
            genre_label="Sanity",
            behavior="depleting",
            initial_value=0.8,
            threshold=0.2,
            threshold_effect="Genre shifts to horror mode",
        )
        assert sv.behavior == "depleting"
        assert sv.initial_value == 0.8

    def test_threshold_out_of_range_rejected(self):
        from narrative_data.schemas.shared import StateVariableTemplate

        with pytest.raises(ValidationError):
            StateVariableTemplate(
                canonical_id="V1", genre_label="X", behavior="depleting", threshold=2.0
            )

    def test_minimal_template(self):
        from narrative_data.schemas.shared import StateVariableTemplate

        sv = StateVariableTemplate(canonical_id="V1", genre_label="Hope", behavior="fluctuating")
        assert sv.initial_value is None
        assert sv.threshold is None


class TestStateVariableInteraction:
    def test_valid_interaction(self):
        from narrative_data.schemas.shared import StateVariableInteraction

        svi = StateVariableInteraction(
            variable_id="V1", operation="consumes", description="Depletes sanity"
        )
        assert svi.operation == "consumes"


class TestOverlapSignal:
    def test_valid_signal(self):
        from narrative_data.schemas.shared import OverlapSignal

        sig = OverlapSignal(
            adjacent_genre="cosmic-horror",
            similar_entity="The Witness",
            differentiator="Cosmic horror variant lacks warmth",
        )
        assert sig.adjacent_genre == "cosmic-horror"


class TestGenreBoundary:
    def test_valid_boundary(self):
        from narrative_data.schemas.shared import GenreBoundary

        gb = GenreBoundary(
            trigger="Hope below 0.3",
            drift_target="horror",
            description="Genre shifts to horror mode",
        )
        assert gb.drift_target == "horror"
