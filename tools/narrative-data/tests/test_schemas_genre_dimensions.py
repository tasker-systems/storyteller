# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for GenreDimensions schema."""

import pytest
from pydantic import ValidationError


def _make_minimal_genre_dimensions(**overrides):
    """Return a minimal valid GenreDimensions dict with all required fields."""
    from narrative_data.schemas.genre_dimensions import (
        AestheticDimensions,
        AgencyDimensions,
        EpistemologicalDimensions,
        GenreDimensions,
        TemporalDimensions,
        ThematicDimensions,
        TonalDimensions,
        WorldAffordances,
    )
    from narrative_data.schemas.shared import ContinuousAxis, WeightedTags

    defaults = {
        "genre_slug": "folk-horror",
        "genre_name": "Folk Horror",
        "classification": "standalone_region",
        "aesthetic": AestheticDimensions(
            sensory_density=ContinuousAxis(value=0.7),
            groundedness=ContinuousAxis(value=0.8),
            aesthetic_register=ContinuousAxis(value=0.4),
            prose_register=["Vernacular"],
        ),
        "tonal": TonalDimensions(
            emotional_contract=ContinuousAxis(value=0.6),
            cynicism_earnestness=ContinuousAxis(value=0.5),
            surface_irony=ContinuousAxis(value=0.2),
            structural_irony=ContinuousAxis(value=0.7),
            intimacy_distance=ContinuousAxis(value=0.4),
        ),
        "temporal": TemporalDimensions(
            time_structure=ContinuousAxis(value=0.5),
            pacing=ContinuousAxis(value=0.4),
            temporal_grounding=ContinuousAxis(value=0.8),
            narrative_span=ContinuousAxis(value=0.3),
        ),
        "thematic": ThematicDimensions(
            power_treatment=WeightedTags(root={"Corruption": 0.8, "Stewardship": 0.2}),
            identity_treatment=WeightedTags(root={"Belonging": 0.9}),
            knowledge_treatment=WeightedTags(root={"ForbiddenKnowledge": 0.7}),
            connection_treatment=WeightedTags(root={"Community": 0.6}),
        ),
        "agency": AgencyDimensions(
            agency_level=ContinuousAxis(value=0.3),
            agency_type="survival",
            triumph_mode=ContinuousAxis(value=0.2),
            competence_relevance=ContinuousAxis(value=0.4),
        ),
        "locus_of_power": ["place", "cosmos"],
        "narrative_structure": ["horror", "tragedy"],
        "world_affordances": WorldAffordances(
            magic=["wild_mythic"],
            technology="pre-industrial",
            violence="ritualistic",
            death="transformative",
            supernatural="immanent",
        ),
        "epistemological": EpistemologicalDimensions(
            knowability=ContinuousAxis(value=0.3),
            knowledge_reward=ContinuousAxis(value=0.4),
            narration_reliability=ContinuousAxis(value=0.6),
        ),
    }
    defaults.update(overrides)
    return GenreDimensions(**defaults)


class TestMinimalValidConstruction:
    def test_all_required_fields_provided(self):
        gd = _make_minimal_genre_dimensions()
        assert gd.genre_slug == "folk-horror"
        assert gd.genre_name == "Folk Horror"
        assert gd.classification == "standalone_region"

    def test_optional_fields_default_correctly(self):
        gd = _make_minimal_genre_dimensions()
        assert gd.constraint_layer_type is None
        assert gd.narrative_contracts == []
        assert gd.active_state_variables == []
        assert gd.boundaries == []
        assert gd.modifies == []
        assert gd.flavor_text is None

    def test_missing_required_slug_rejected(self):
        with pytest.raises((ValidationError, TypeError)):
            _make_minimal_genre_dimensions(genre_slug=None)


class TestRoundTrip:
    def test_construct_dump_validate_equal(self):
        from narrative_data.schemas.genre_dimensions import GenreDimensions

        gd = _make_minimal_genre_dimensions()
        data = gd.model_dump()
        restored = GenreDimensions.model_validate(data)
        assert restored == gd

    def test_round_trip_with_all_optional_fields(self):
        from narrative_data.schemas.genre_dimensions import GenreDimensions, NarrativeContract
        from narrative_data.schemas.shared import GenreBoundary, StateVariableTemplate

        gd = _make_minimal_genre_dimensions(
            narrative_contracts=[
                NarrativeContract(invariant="Must_Resolve_Relationship", enforced=True)
            ],
            active_state_variables=[
                StateVariableTemplate(
                    canonical_id="V1",
                    genre_label="Community_Cohesion",
                    behavior="depleting",
                    initial_value=0.8,
                )
            ],
            boundaries=[
                GenreBoundary(
                    trigger="Protagonist escapes community",
                    drift_target="thriller",
                    description="Genre shifts when protagonist breaks free",
                )
            ],
            modifies=["horror", "gothic"],
            flavor_text="Folk horror roots terror in the communal and the ancient.",
        )
        data = gd.model_dump()
        restored = GenreDimensions.model_validate(data)
        assert restored == gd


class TestContinuousAxisValidation:
    def test_tonal_axis_rejects_value_above_one(self):
        from narrative_data.schemas.genre_dimensions import TonalDimensions
        from narrative_data.schemas.shared import ContinuousAxis

        with pytest.raises(ValidationError):
            TonalDimensions(
                emotional_contract=ContinuousAxis(value=1.5),
                cynicism_earnestness=ContinuousAxis(value=0.5),
                surface_irony=ContinuousAxis(value=0.2),
                structural_irony=ContinuousAxis(value=0.7),
                intimacy_distance=ContinuousAxis(value=0.4),
            )

    def test_aesthetic_axis_rejects_value_below_zero(self):
        from narrative_data.schemas.genre_dimensions import AestheticDimensions
        from narrative_data.schemas.shared import ContinuousAxis

        with pytest.raises(ValidationError):
            AestheticDimensions(
                sensory_density=ContinuousAxis(value=-0.1),
                groundedness=ContinuousAxis(value=0.5),
                aesthetic_register=ContinuousAxis(value=0.5),
                prose_register=[],
            )

    def test_boundary_values_accepted(self):
        from narrative_data.schemas.shared import ContinuousAxis

        axis_zero = ContinuousAxis(value=0.0)
        axis_one = ContinuousAxis(value=1.0)
        assert axis_zero.value == 0.0
        assert axis_one.value == 1.0


class TestWeightedTagsInThematic:
    def test_thematic_weighted_tags_valid(self):
        from narrative_data.schemas.genre_dimensions import ThematicDimensions
        from narrative_data.schemas.shared import WeightedTags

        thematic = ThematicDimensions(
            power_treatment=WeightedTags(root={"Oppression": 0.9, "Liberation": 0.1}),
            identity_treatment=WeightedTags(root={"Outsider": 0.8}),
            knowledge_treatment=WeightedTags(root={"Tradition": 0.6}),
            connection_treatment=WeightedTags(root={"Sacrifice": 0.7}),
        )
        assert thematic.power_treatment.root["Oppression"] == 0.9

    def test_thematic_weighted_tags_out_of_range_rejected(self):
        from narrative_data.schemas.genre_dimensions import ThematicDimensions
        from narrative_data.schemas.shared import WeightedTags

        with pytest.raises(ValidationError):
            ThematicDimensions(
                power_treatment=WeightedTags(root={"Bad": 2.0}),
                identity_treatment=WeightedTags(root={}),
                knowledge_treatment=WeightedTags(root={}),
                connection_treatment=WeightedTags(root={}),
            )


class TestClassificationEnum:
    def test_standalone_region(self):
        gd = _make_minimal_genre_dimensions(classification="standalone_region")
        assert gd.classification == "standalone_region"

    def test_constraint_layer(self):
        gd = _make_minimal_genre_dimensions(classification="constraint_layer")
        assert gd.classification == "constraint_layer"

    def test_hybrid_modifier(self):
        gd = _make_minimal_genre_dimensions(classification="hybrid_modifier")
        assert gd.classification == "hybrid_modifier"

    def test_invalid_classification_rejected(self):
        with pytest.raises(ValidationError):
            _make_minimal_genre_dimensions(classification="not_a_real_classification")


class TestConstraintLayerType:
    def test_constraint_layer_type_present_for_constraint_layer(self):
        gd = _make_minimal_genre_dimensions(
            classification="constraint_layer",
            constraint_layer_type="tonal_emotional",
        )
        assert gd.constraint_layer_type == "tonal_emotional"

    def test_all_constraint_layer_types_valid(self):
        valid_types = ["world_affordance", "tonal_emotional", "agency_outcome", "setting_locus"]
        for ct in valid_types:
            gd = _make_minimal_genre_dimensions(
                classification="constraint_layer",
                constraint_layer_type=ct,
            )
            assert gd.constraint_layer_type == ct

    def test_invalid_constraint_layer_type_rejected(self):
        with pytest.raises(ValidationError):
            _make_minimal_genre_dimensions(
                classification="constraint_layer",
                constraint_layer_type="invalid_type",
            )

    def test_constraint_layer_type_none_by_default(self):
        gd = _make_minimal_genre_dimensions(classification="standalone_region")
        assert gd.constraint_layer_type is None


class TestLocusOfPower:
    def test_locus_of_power_ranked_list(self):
        gd = _make_minimal_genre_dimensions(locus_of_power=["place", "person", "system"])
        assert gd.locus_of_power == ["place", "person", "system"]

    def test_locus_of_power_max_three(self):
        with pytest.raises(ValidationError):
            _make_minimal_genre_dimensions(
                locus_of_power=["place", "person", "system", "relationship"]
            )

    def test_locus_of_power_valid_items(self):
        valid_items = ["place", "person", "system", "relationship", "cosmos"]
        for item in valid_items:
            gd = _make_minimal_genre_dimensions(locus_of_power=[item])
            assert gd.locus_of_power == [item]

    def test_locus_of_power_invalid_item_rejected(self):
        with pytest.raises(ValidationError):
            _make_minimal_genre_dimensions(locus_of_power=["invalid_locus"])


class TestNarrativeStructure:
    def test_narrative_structure_max_three(self):
        with pytest.raises(ValidationError):
            _make_minimal_genre_dimensions(
                narrative_structure=["quest", "mystery", "tragedy", "comedy"]
            )

    def test_valid_narrative_structure_items(self):
        valid_items = ["quest", "mystery", "tragedy", "comedy", "romance", "horror"]
        for item in valid_items:
            gd = _make_minimal_genre_dimensions(narrative_structure=[item])
            assert gd.narrative_structure == [item]

    def test_invalid_narrative_structure_item_rejected(self):
        with pytest.raises(ValidationError):
            _make_minimal_genre_dimensions(narrative_structure=["invalid_structure"])


class TestNarrativeContracts:
    def test_narrative_contracts_empty_by_default(self):
        gd = _make_minimal_genre_dimensions()
        assert gd.narrative_contracts == []

    def test_narrative_contract_construction(self):
        from narrative_data.schemas.genre_dimensions import NarrativeContract

        nc = NarrativeContract(invariant="Must_Resolve_Relationship", enforced=True)
        assert nc.invariant == "Must_Resolve_Relationship"
        assert nc.enforced is True

    def test_narrative_contracts_list(self):
        from narrative_data.schemas.genre_dimensions import NarrativeContract

        gd = _make_minimal_genre_dimensions(
            narrative_contracts=[
                NarrativeContract(invariant="Knowledge_Rewarded", enforced=True),
                NarrativeContract(invariant="Safety_Restored", enforced=False),
            ]
        )
        assert len(gd.narrative_contracts) == 2
        assert gd.narrative_contracts[0].invariant == "Knowledge_Rewarded"

    def test_narrative_contract_round_trip(self):
        from narrative_data.schemas.genre_dimensions import NarrativeContract

        nc = NarrativeContract(invariant="Must_Resolve_Relationship", enforced=True)
        data = nc.model_dump()
        restored = NarrativeContract.model_validate(data)
        assert restored == nc


class TestActiveStateVariables:
    def test_active_state_variables_empty_by_default(self):
        gd = _make_minimal_genre_dimensions()
        assert gd.active_state_variables == []

    def test_state_variable_template_in_list(self):
        from narrative_data.schemas.shared import StateVariableTemplate

        gd = _make_minimal_genre_dimensions(
            active_state_variables=[
                StateVariableTemplate(
                    canonical_id="V_TRUST",
                    genre_label="Community_Trust",
                    behavior="depleting",
                    initial_value=0.9,
                    threshold=0.2,
                    threshold_effect="Ritual violence becomes possible",
                )
            ]
        )
        assert len(gd.active_state_variables) == 1
        assert gd.active_state_variables[0].canonical_id == "V_TRUST"

    def test_state_variable_initial_value_range_enforced(self):
        from narrative_data.schemas.shared import StateVariableTemplate

        with pytest.raises(ValidationError):
            StateVariableTemplate(
                canonical_id="V1",
                genre_label="Test",
                behavior="accumulating",
                initial_value=1.5,
            )


class TestBoundaries:
    def test_boundaries_empty_by_default(self):
        gd = _make_minimal_genre_dimensions()
        assert gd.boundaries == []

    def test_boundary_construction(self):
        from narrative_data.schemas.shared import GenreBoundary

        gb = GenreBoundary(
            trigger="Protagonist escapes rural community",
            drift_target="thriller",
            description="Shift to urban thriller when community is left behind",
        )
        assert gb.trigger == "Protagonist escapes rural community"
        assert gb.drift_target == "thriller"

    def test_boundaries_list(self):
        from narrative_data.schemas.shared import GenreBoundary

        gd = _make_minimal_genre_dimensions(
            boundaries=[
                GenreBoundary(
                    trigger="Supernatural explained rationally",
                    drift_target="gothic",
                    description="Genre softens when mystery is resolved",
                ),
                GenreBoundary(
                    trigger="Urban setting introduced",
                    drift_target="contemporary-horror",
                    description="Displacement from rural roots",
                ),
            ]
        )
        assert len(gd.boundaries) == 2


class TestFullRealisticConstruction:
    """Build a realistic folk-horror genre dimensions object with all fields populated."""

    def test_full_folk_horror(self):
        from narrative_data.schemas.genre_dimensions import (
            AestheticDimensions,
            AgencyDimensions,
            EpistemologicalDimensions,
            GenreDimensions,
            NarrativeContract,
            TemporalDimensions,
            ThematicDimensions,
            TonalDimensions,
            WorldAffordances,
        )
        from narrative_data.schemas.shared import (
            ContinuousAxis,
            GenreBoundary,
            StateVariableTemplate,
            WeightedTags,
        )

        gd = GenreDimensions(
            genre_slug="folk-horror",
            genre_name="Folk Horror",
            classification="standalone_region",
            constraint_layer_type=None,
            aesthetic=AestheticDimensions(
                sensory_density=ContinuousAxis(
                    value=0.75,
                    low_label="Sparse",
                    high_label="Overwhelming",
                    can_be_state_variable=False,
                    flavor_text="The moor smells of rot and old fires.",
                ),
                groundedness=ContinuousAxis(
                    value=0.85,
                    low_label="Abstract",
                    high_label="Visceral",
                ),
                aesthetic_register=ContinuousAxis(
                    value=0.35,
                    low_label="Polished",
                    high_label="Rough",
                ),
                prose_register=["Vernacular", "Incantatory"],
            ),
            tonal=TonalDimensions(
                emotional_contract=ContinuousAxis(
                    value=0.65,
                    low_label="Detached",
                    high_label="Immersive_Dread",
                ),
                cynicism_earnestness=ContinuousAxis(value=0.4),
                surface_irony=ContinuousAxis(value=0.15),
                structural_irony=ContinuousAxis(
                    value=0.8,
                    flavor_text=(
                        "The protagonist seeks belonging in the very thing that destroys them."
                    ),
                ),
                intimacy_distance=ContinuousAxis(value=0.35),
            ),
            temporal=TemporalDimensions(
                time_structure=ContinuousAxis(
                    value=0.45,
                    low_label="Linear",
                    high_label="Cyclical",
                ),
                pacing=ContinuousAxis(value=0.35),
                temporal_grounding=ContinuousAxis(
                    value=0.9,
                    low_label="Timeless",
                    high_label="Seasonal_Anchored",
                ),
                narrative_span=ContinuousAxis(value=0.25),
            ),
            thematic=ThematicDimensions(
                power_treatment=WeightedTags(
                    root={
                        "Corruption": 0.8,
                        "Tradition_as_Control": 0.7,
                        "Communal_Oppression": 0.6,
                    }
                ),
                identity_treatment=WeightedTags(
                    root={
                        "Belonging_vs_Otherness": 0.9,
                        "Blood_and_Soil": 0.7,
                        "Loss_of_Self": 0.5,
                    }
                ),
                knowledge_treatment=WeightedTags(
                    root={
                        "Forbidden_Knowledge": 0.8,
                        "Ancient_Wisdom": 0.7,
                        "Rational_Impotence": 0.6,
                    }
                ),
                connection_treatment=WeightedTags(
                    root={
                        "Community_as_Trap": 0.8,
                        "Ritual_Bond": 0.6,
                        "Outsider_Yearning": 0.5,
                    }
                ),
            ),
            agency=AgencyDimensions(
                agency_level=ContinuousAxis(
                    value=0.25,
                    low_label="Fated",
                    high_label="Self_Determining",
                    can_be_state_variable=True,
                ),
                agency_type="survival",
                triumph_mode=ContinuousAxis(
                    value=0.15,
                    low_label="Pyrrhic_Survival",
                    high_label="Full_Victory",
                ),
                competence_relevance=ContinuousAxis(value=0.3),
            ),
            locus_of_power=["place", "cosmos", "system"],
            narrative_structure=["horror", "tragedy"],
            world_affordances=WorldAffordances(
                magic=["wild_mythic", "immanent_chthonic"],
                technology="pre-industrial",
                violence="ritualistic_and_communal",
                death="transformative_and_sacrificial",
                supernatural="immanent_in_landscape",
            ),
            epistemological=EpistemologicalDimensions(
                knowability=ContinuousAxis(
                    value=0.25,
                    low_label="Ineffable",
                    high_label="Knowable",
                ),
                knowledge_reward=ContinuousAxis(
                    value=0.3,
                    flavor_text="Understanding brings dread, not mastery.",
                ),
                narration_reliability=ContinuousAxis(value=0.7),
            ),
            narrative_contracts=[
                NarrativeContract(
                    invariant="Community_Demands_Sacrifice",
                    enforced=True,
                ),
                NarrativeContract(
                    invariant="Outsider_Cannot_Fully_Escape",
                    enforced=False,
                ),
            ],
            active_state_variables=[
                StateVariableTemplate(
                    canonical_id="V_COMMUNITY_TRUST",
                    genre_label="Community_Cohesion",
                    behavior="depleting",
                    initial_value=0.8,
                    threshold=0.2,
                    threshold_effect="Open ritual violence becomes possible",
                    activation_condition="Scene involves protagonist interacting with locals",
                ),
                StateVariableTemplate(
                    canonical_id="V_DREAD_ACCUMULATION",
                    genre_label="Accumulated_Dread",
                    behavior="accumulating",
                    initial_value=0.0,
                    threshold=0.7,
                    threshold_effect="Full horror mode activates",
                ),
            ],
            boundaries=[
                GenreBoundary(
                    trigger="Supernatural element given rational explanation",
                    drift_target="gothic-horror",
                    description="Rationalisation softens the chthonic into gothic register",
                ),
                GenreBoundary(
                    trigger="Protagonist permanently escapes rural setting",
                    drift_target="urban-horror",
                    description="Displacement from the land severs folk horror's power source",
                ),
            ],
            modifies=[],
            flavor_text=(
                "Terror rooted in the communal, the seasonal, the ancient. The land remembers."
            ),
        )

        # Verify key structural properties
        assert gd.genre_slug == "folk-horror"
        assert gd.classification == "standalone_region"
        assert len(gd.narrative_contracts) == 2
        assert len(gd.active_state_variables) == 2
        assert len(gd.boundaries) == 2
        assert gd.agency.agency_type == "survival"
        assert gd.locus_of_power[0] == "place"
        assert "Corruption" in gd.thematic.power_treatment.root

        # Round-trip
        data = gd.model_dump()
        restored = GenreDimensions.model_validate(data)
        assert restored == gd
