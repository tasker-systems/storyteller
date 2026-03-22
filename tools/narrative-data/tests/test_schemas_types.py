"""Tests for the 6 discovery type schemas: archetypes, dynamics, goals, settings,
scene_profiles, and ontological_posture."""

import pytest
from pydantic import ValidationError

# ---------------------------------------------------------------------------
# Archetypes
# ---------------------------------------------------------------------------


class TestArchetype:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.archetypes import Archetype, PersonalityProfile

        a = Archetype(
            canonical_name="The Mentor",
            genre_slug="epic-fantasy",
            variant_name="The Wise Elder",
            personality_profile=PersonalityProfile(
                warmth=0.8,
                authority=0.7,
                openness=0.6,
                interiority=0.5,
                stability=0.9,
                agency=0.6,
                morality=0.9,
            ),
            distinguishing_tension="Wisdom vs. withholding",
            structural_necessity="Enables protagonist's journey",
            universality="universal",
        )
        assert a.canonical_name == "The Mentor"
        assert a.genre_slug == "epic-fantasy"
        assert a.personality_profile.warmth == 0.8
        assert a.extended_axes == {}
        assert a.overlap_signals == []
        assert a.state_variables == []
        assert a.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.archetypes import Archetype, PersonalityProfile

        a = Archetype(
            canonical_name="The Mentor",
            genre_slug="epic-fantasy",
            variant_name="The Wise Elder",
            personality_profile=PersonalityProfile(
                warmth=0.8,
                authority=0.7,
                openness=0.6,
                interiority=0.5,
                stability=0.9,
                agency=0.6,
                morality=0.9,
            ),
            distinguishing_tension="Wisdom vs. withholding",
            structural_necessity="Enables protagonist's journey",
            universality="universal",
            flavor_text="The guide who must let go.",
        )
        data = a.model_dump()
        restored = Archetype.model_validate(data)
        assert restored == a

    def test_missing_required_field_raises(self):
        from narrative_data.schemas.archetypes import PersonalityProfile

        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.archetypes import Archetype

            Archetype(
                # missing canonical_name
                genre_slug="epic-fantasy",
                variant_name="The Wise Elder",
                personality_profile=PersonalityProfile(
                    warmth=0.8,
                    authority=0.7,
                    openness=0.6,
                    interiority=0.5,
                    stability=0.9,
                    agency=0.6,
                    morality=0.9,
                ),
                distinguishing_tension="Wisdom vs. withholding",
                structural_necessity="Enables protagonist's journey",
                universality="universal",
            )

    def test_cluster_archetype_construction(self):
        from narrative_data.schemas.archetypes import ClusterArchetype
        from narrative_data.schemas.shared import GenreVariant

        ca = ClusterArchetype(
            canonical_name="The Mentor",
            cluster_name="Guidance Cluster",
            core_identity="The one who enables growth at cost to self",
            genre_variants=[
                GenreVariant(
                    genre_slug="epic-fantasy",
                    variant_name="The Wise Elder",
                    key_differences="Explicitly magical lineage",
                ),
                GenreVariant(
                    genre_slug="thriller",
                    variant_name="The Handler",
                    key_differences="Institutional authority replaces mystical",
                ),
            ],
            uniqueness="universal",
        )
        assert ca.canonical_name == "The Mentor"
        assert len(ca.genre_variants) == 2

    def test_personality_profile_axis_out_of_range_rejected(self):
        from narrative_data.schemas.archetypes import PersonalityProfile

        with pytest.raises(ValidationError):
            PersonalityProfile(
                warmth=1.5,  # invalid
                authority=0.7,
                openness=0.6,
                interiority=0.5,
                stability=0.9,
                agency=0.6,
                morality=0.9,
            )

    def test_personality_profile_below_zero_rejected(self):
        from narrative_data.schemas.archetypes import PersonalityProfile

        with pytest.raises(ValidationError):
            PersonalityProfile(
                warmth=-0.1,  # invalid
                authority=0.7,
                openness=0.6,
                interiority=0.5,
                stability=0.9,
                agency=0.6,
                morality=0.9,
            )

    def test_extended_axes_out_of_range_rejected(self):
        from narrative_data.schemas.archetypes import Archetype, PersonalityProfile

        with pytest.raises(ValidationError):
            Archetype(
                canonical_name="The Mentor",
                genre_slug="epic-fantasy",
                variant_name="The Wise Elder",
                personality_profile=PersonalityProfile(
                    warmth=0.8,
                    authority=0.7,
                    openness=0.6,
                    interiority=0.5,
                    stability=0.9,
                    agency=0.6,
                    morality=0.9,
                ),
                distinguishing_tension="Wisdom vs. withholding",
                structural_necessity="Enables protagonist's journey",
                universality="universal",
                extended_axes={"efficacy": 2.0},  # invalid
            )

    def test_universality_enum_invalid_rejected(self):
        from narrative_data.schemas.archetypes import Archetype, PersonalityProfile

        with pytest.raises(ValidationError):
            Archetype(
                canonical_name="The Mentor",
                genre_slug="epic-fantasy",
                variant_name="The Wise Elder",
                personality_profile=PersonalityProfile(
                    warmth=0.8,
                    authority=0.7,
                    openness=0.6,
                    interiority=0.5,
                    stability=0.9,
                    agency=0.6,
                    morality=0.9,
                ),
                distinguishing_tension="Wisdom vs. withholding",
                structural_necessity="Enables protagonist's journey",
                universality="not_a_real_value",  # invalid
            )


# ---------------------------------------------------------------------------
# Dynamics
# ---------------------------------------------------------------------------


class TestDynamic:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.dynamics import Dynamic

        d = Dynamic(
            canonical_name="Mentor-Student Bond",
            genre_slug="epic-fantasy",
            variant_name="The Master's Burden",
            scale="arc",
            edge_type="guidance",
            directionality="unidirectional",
        )
        assert d.canonical_name == "Mentor-Student Bond"
        assert d.scale == "arc"
        assert d.currencies == []
        assert d.role_slots == []
        assert d.spans_scales == []
        assert d.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.dynamics import Dynamic, RoleSlot, ScaleManifestations

        d = Dynamic(
            canonical_name="Rival Bond",
            genre_slug="thriller",
            variant_name="The Nemesis Loop",
            scale="arc",
            edge_type="antagonism",
            directionality="bidirectional_asymmetric",
            currencies=["leverage", "information"],
            role_slots=[
                RoleSlot(role="Hunter", want="Capture", withhold="Mercy"),
                RoleSlot(role="Prey", want="Escape", withhold="Vulnerability"),
            ],
            scale_manifestations=ScaleManifestations(
                orbital="Determines final confrontation",
                arc="Each act escalates stakes",
                scene="Cat-and-mouse exchanges",
            ),
            flavor_text="The chase that defines both.",
        )
        data = d.model_dump()
        restored = Dynamic.model_validate(data)
        assert restored == d

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.dynamics import Dynamic

            Dynamic(
                # missing canonical_name
                genre_slug="thriller",
                variant_name="The Nemesis Loop",
                scale="arc",
                edge_type="antagonism",
                directionality="unidirectional",
            )

    def test_cluster_dynamic_construction(self):
        from narrative_data.schemas.dynamics import ClusterDynamic
        from narrative_data.schemas.shared import GenreVariant

        cd = ClusterDynamic(
            canonical_name="Adversarial Bond",
            cluster_name="Conflict Cluster",
            core_identity="Two opposing forces locked in mutual definition",
            genre_variants=[
                GenreVariant(genre_slug="thriller", variant_name="The Nemesis Loop"),
                GenreVariant(genre_slug="epic-fantasy", variant_name="The Dark Mirror"),
            ],
            uniqueness="universal",
        )
        assert cd.canonical_name == "Adversarial Bond"
        assert len(cd.genre_variants) == 2

    def test_scale_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.dynamics import Dynamic

            Dynamic(
                canonical_name="Test",
                genre_slug="thriller",
                variant_name="Test Variant",
                scale="not_a_scale",  # invalid
                edge_type="antagonism",
                directionality="unidirectional",
            )

    def test_directionality_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.dynamics import Dynamic

            Dynamic(
                canonical_name="Test",
                genre_slug="thriller",
                variant_name="Test Variant",
                scale="arc",
                edge_type="antagonism",
                directionality="invalid_direction",  # invalid
            )


# ---------------------------------------------------------------------------
# Goals
# ---------------------------------------------------------------------------


class TestGoal:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.goals import Goal

        g = Goal(
            canonical_name="Belonging",
            genre_slug="folk-horror",
            variant_name="The Cost of Acceptance",
            scale="arc",
            uniqueness="universal",
        )
        assert g.canonical_name == "Belonging"
        assert g.archetype_refs == []
        assert g.state_variable_interactions == []
        assert g.genre_variants == []
        assert g.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.goals import CrossScaleTension, Goal

        g = Goal(
            canonical_name="Survival",
            genre_slug="thriller",
            variant_name="Staying Alive Against the Odds",
            scale="existential",
            description="The fundamental drive to persist against lethal threat",
            cross_scale_tension=CrossScaleTension(
                tension_type="scene_costs_existential",
                description="Hiding costs long-term credibility",
            ),
            archetype_refs=["The Hunted", "The Pursuer"],
            uniqueness="universal",
            flavor_text="When survival is the only goal, all others become negotiable.",
        )
        data = g.model_dump()
        restored = Goal.model_validate(data)
        assert restored == g

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.goals import Goal

            Goal(
                # missing canonical_name
                genre_slug="thriller",
                variant_name="Staying Alive",
                scale="existential",
            )

    def test_cluster_goal_construction(self):
        from narrative_data.schemas.goals import ClusterGoal
        from narrative_data.schemas.shared import GenreVariant

        cg = ClusterGoal(
            canonical_name="Survival",
            cluster_name="Primal Motivation Cluster",
            core_identity="The drive to persist at all costs",
            genre_variants=[
                GenreVariant(genre_slug="thriller", variant_name="Staying Alive"),
                GenreVariant(genre_slug="folk-horror", variant_name="Escaping the Ritual"),
            ],
            uniqueness="universal",
        )
        assert cg.canonical_name == "Survival"
        assert len(cg.genre_variants) == 2

    def test_scale_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.goals import Goal

            Goal(
                canonical_name="Survival",
                genre_slug="thriller",
                variant_name="Staying Alive",
                scale="not_a_scale",  # invalid
            )

    def test_cross_scale_tension_type_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.goals import CrossScaleTension

            CrossScaleTension(tension_type="not_a_real_tension_type")


# ---------------------------------------------------------------------------
# Settings
# ---------------------------------------------------------------------------


class TestSettings:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.settings import Settings

        s = Settings(
            canonical_name="The Isolated Village",
            genre_slug="folk-horror",
            variant_name="The Ancient Hamlet",
        )
        assert s.canonical_name == "The Isolated Village"
        assert s.atmospheric_palette == []
        assert s.sensory_vocabulary == []
        assert s.narrative_function is None
        assert s.overlap_signals == []
        assert s.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.settings import SettingCommunicability, Settings

        s = Settings(
            canonical_name="The Isolated Village",
            genre_slug="folk-horror",
            variant_name="The Ancient Hamlet",
            atmospheric_palette=["mist", "decay", "firelight"],
            sensory_vocabulary=["peat smoke", "wet stone", "iron bells"],
            narrative_function="Traps protagonist; externalizes community pressure",
            communicability=SettingCommunicability(
                atmospheric="Dread accumulates through sensory saturation",
                sensory="Smell and sound as harbingers",
                spatial="No escape routes apparent",
                temporal="Seasonal cycles enforce ritual time",
            ),
            flavor_text="The village has seen this before.",
        )
        data = s.model_dump()
        restored = Settings.model_validate(data)
        assert restored == s

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.settings import Settings

            Settings(
                # missing canonical_name
                genre_slug="folk-horror",
                variant_name="The Ancient Hamlet",
            )

    def test_cluster_settings_construction(self):
        from narrative_data.schemas.settings import ClusterSettings
        from narrative_data.schemas.shared import GenreVariant

        cs = ClusterSettings(
            canonical_name="The Isolated Place",
            cluster_name="Isolation Settings Cluster",
            core_identity="A place that traps and transforms",
            genre_variants=[
                GenreVariant(
                    genre_slug="folk-horror",
                    variant_name="The Ancient Hamlet",
                    key_differences="Community as threat, not refuge",
                ),
                GenreVariant(
                    genre_slug="gothic",
                    variant_name="The Manor House",
                    key_differences="Architectural oppression replaces communal",
                ),
            ],
            uniqueness="cluster_specific",
        )
        assert cs.canonical_name == "The Isolated Place"
        assert len(cs.genre_variants) == 2

    def test_uniqueness_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.settings import ClusterSettings

            ClusterSettings(
                canonical_name="Test",
                cluster_name="Test Cluster",
                core_identity="Test identity",
                genre_variants=[],
                uniqueness="not_a_valid_uniqueness",  # invalid
            )


# ---------------------------------------------------------------------------
# Scene Profiles
# ---------------------------------------------------------------------------


class TestSceneProfile:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.scene_profiles import SceneDimensionalProperties, SceneProfile

        sp = SceneProfile(
            name="The Revelation Scene",
            genre_slug="thriller",
            core_identity="A scene where hidden truth is forced into the open",
            dimensional_properties=SceneDimensionalProperties(),
        )
        assert sp.name == "The Revelation Scene"
        assert sp.genre_slug == "thriller"
        assert sp.provenance == []
        assert sp.flavor_text is None
        assert sp.uniqueness is None

    def test_round_trip(self):
        from narrative_data.schemas.scene_profiles import SceneDimensionalProperties, SceneProfile

        sp = SceneProfile(
            name="The Confrontation",
            genre_slug="thriller",
            core_identity="Two forces collide with everything at stake",
            dimensional_properties=SceneDimensionalProperties(
                tension_signature="spiking_explosive",
                emotional_register="visceral_overwhelming",
                pacing="rapid_kinetic",
                cast_density="intimate_few",
                physical_dynamism="kinetic",
                information_flow="revealing_spending",
                resolution_tendency="closed_conclusive",
                locus_of_power="The Hunter",
            ),
            uniqueness="genre_unique",
            provenance=["tension_signature", "pacing"],
            flavor_text="The moment everything converges.",
        )
        data = sp.model_dump()
        restored = SceneProfile.model_validate(data)
        assert restored == sp

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.scene_profiles import (
                SceneDimensionalProperties,
                SceneProfile,
            )

            SceneProfile(
                # missing name
                genre_slug="thriller",
                core_identity="A scene",
                dimensional_properties=SceneDimensionalProperties(),
            )

    def test_cluster_scene_profile_construction(self):
        from narrative_data.schemas.scene_profiles import ClusterSceneProfile
        from narrative_data.schemas.shared import GenreVariant

        csp = ClusterSceneProfile(
            canonical_name="The Confrontation",
            cluster_name="High-Stakes Clash Cluster",
            core_identity="Irreversible collision of opposing forces",
            genre_variants=[
                GenreVariant(genre_slug="thriller", variant_name="The Chase Convergence"),
                GenreVariant(genre_slug="epic-fantasy", variant_name="The Final Battle"),
            ],
            uniqueness="cluster_wide",
        )
        assert csp.canonical_name == "The Confrontation"
        assert len(csp.genre_variants) == 2

    def test_tension_signature_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.scene_profiles import SceneDimensionalProperties

            SceneDimensionalProperties(tension_signature="not_a_real_tension")

    def test_uniqueness_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.scene_profiles import (
                SceneDimensionalProperties,
                SceneProfile,
            )

            SceneProfile(
                name="Test",
                genre_slug="thriller",
                core_identity="Test",
                dimensional_properties=SceneDimensionalProperties(),
                uniqueness="not_valid",  # invalid
            )


# ---------------------------------------------------------------------------
# Ontological Posture
# ---------------------------------------------------------------------------


class TestOntologicalPosture:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.ontological_posture import (
            EthicalOrientation,
            ModeOfBeing,
            OntologicalPosture,
            SelfOtherBoundary,
        )

        op = OntologicalPosture(
            genre_slug="folk-horror",
            default_subject="The Outsider",
            modes_of_being=[
                ModeOfBeing(
                    name="The Communal Self",
                    description="Identity defined by belonging or exclusion from the group",
                    can_have_communicability=True,
                )
            ],
            self_other_boundary=SelfOtherBoundary(
                stability="permeable_negotiable",
                crossing_rules="Ritual initiations mark boundary crossings",
                obligations_across="The initiated owe the community silence and service",
            ),
            ethical_orientation=EthicalOrientation(
                permitted=["communal sacrifice", "ritual violence"],
                forbidden=["individual defection", "exposure of secrets"],
            ),
        )
        assert op.genre_slug == "folk-horror"
        assert op.default_subject == "The Outsider"
        assert len(op.modes_of_being) == 1
        assert op.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.ontological_posture import (
            EthicalOrientation,
            ModeOfBeing,
            OntologicalPosture,
            SelfOtherBoundary,
        )

        op = OntologicalPosture(
            genre_slug="folk-horror",
            default_subject="The Outsider",
            modes_of_being=[
                ModeOfBeing(
                    name="The Communal Self",
                    description="Identity as function of belonging",
                )
            ],
            self_other_boundary=SelfOtherBoundary(
                stability="firm_defensive",
            ),
            ethical_orientation=EthicalOrientation(),
            flavor_text="The village decides who you are.",
        )
        data = op.model_dump()
        restored = OntologicalPosture.model_validate(data)
        assert restored == op

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.ontological_posture import (
                EthicalOrientation,
                OntologicalPosture,
                SelfOtherBoundary,
            )

            OntologicalPosture(
                # missing genre_slug
                default_subject="The Outsider",
                modes_of_being=[],
                self_other_boundary=SelfOtherBoundary(stability="firm_defensive"),
                ethical_orientation=EthicalOrientation(),
            )

    def test_cluster_ontological_posture_construction(self):
        from narrative_data.schemas.ontological_posture import ClusterOntologicalPosture
        from narrative_data.schemas.shared import GenreVariant

        cop = ClusterOntologicalPosture(
            cluster_name="Communal Horror Cluster",
            core_identity="Identity as threat or gift of the collective",
            genre_variants=[
                GenreVariant(genre_slug="folk-horror", variant_name="The Ritual Subject"),
                GenreVariant(genre_slug="cosmic-horror", variant_name="The Insignificant Self"),
            ],
        )
        assert cop.cluster_name == "Communal Horror Cluster"
        assert len(cop.genre_variants) == 2

    def test_self_other_boundary_stability_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.ontological_posture import SelfOtherBoundary

            SelfOtherBoundary(stability="not_a_real_stability")

    def test_mode_of_being_defaults(self):
        from narrative_data.schemas.ontological_posture import ModeOfBeing

        m = ModeOfBeing(name="Individual", description="Autonomous selfhood")
        assert m.can_have_communicability is False
