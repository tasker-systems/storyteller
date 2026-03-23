# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

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


# ---------------------------------------------------------------------------
# Archetype Dynamics
# ---------------------------------------------------------------------------


class TestArchetypeDynamic:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.archetype_dynamics import ArchetypeDynamic, EdgeProperties

        ad = ArchetypeDynamic(
            pairing_name="Mentor-Student",
            genre_slug="epic-fantasy",
            archetype_a="The Mentor",
            archetype_b="The Hero",
            edge_properties=EdgeProperties(
                edge_type="guidance",
                directionality="unidirectional",
            ),
        )
        assert ad.pairing_name == "Mentor-Student"
        assert ad.genre_slug == "epic-fantasy"
        assert ad.edge_properties.currencies == []
        assert ad.edge_properties.constraints == []
        assert ad.characteristic_scene is None
        assert ad.shadow_pairing is None
        assert ad.scale_properties is None
        assert ad.uniqueness is None
        assert ad.genre_variants == []
        assert ad.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.archetype_dynamics import (
            ArchetypeDynamic,
            CharacteristicScene,
            EdgeProperties,
            ShadowPairing,
        )
        from narrative_data.schemas.dynamics import ScaleManifestations

        ad = ArchetypeDynamic(
            pairing_name="Hunter-Prey",
            genre_slug="thriller",
            archetype_a="The Pursuer",
            archetype_b="The Hunted",
            edge_properties=EdgeProperties(
                edge_type="antagonism",
                directionality="asymmetric",
                currencies=["leverage", "information"],
                network_position="hub",
                constraints=["protagonist must survive act 1"],
                weight="high",
            ),
            characteristic_scene=CharacteristicScene(
                title="The First Glimpse",
                opening="Hunter detects prey's trail",
                tension_source="Information asymmetry",
                withheld_by_a="Full capability",
                withheld_by_b="Final destination",
                resolution_constraint="No resolution — only escalation",
                scene_register="visceral",
            ),
            shadow_pairing=ShadowPairing(
                description="Prey becomes hunter",
                inversion_type="power_inversion",
                drift_target_genre="revenge-thriller",
            ),
            scale_properties=ScaleManifestations(
                orbital="Determines final confrontation",
                arc="Each act narrows the gap",
                scene="Cat-and-mouse exchanges",
            ),
            uniqueness="genre_defining",
            flavor_text="Every thriller needs a chase.",
        )
        data = ad.model_dump()
        restored = ArchetypeDynamic.model_validate(data)
        assert restored == ad

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.archetype_dynamics import ArchetypeDynamic, EdgeProperties

            ArchetypeDynamic(
                # missing pairing_name
                genre_slug="thriller",
                archetype_a="The Pursuer",
                archetype_b="The Hunted",
                edge_properties=EdgeProperties(
                    edge_type="antagonism",
                    directionality="asymmetric",
                ),
            )

    def test_cluster_archetype_dynamic_construction(self):
        from narrative_data.schemas.archetype_dynamics import ClusterArchetypeDynamic
        from narrative_data.schemas.shared import GenreVariant

        cad = ClusterArchetypeDynamic(
            canonical_name="Adversarial Pairing",
            cluster_name="Conflict Cluster",
            core_identity="Two forces that define each other through opposition",
            genre_variants=[
                GenreVariant(genre_slug="thriller", variant_name="Hunter-Prey"),
                GenreVariant(genre_slug="epic-fantasy", variant_name="Champion-Shadow"),
            ],
            uniqueness="cluster_specific",
        )
        assert cad.canonical_name == "Adversarial Pairing"
        assert len(cad.genre_variants) == 2

    def test_uniqueness_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.archetype_dynamics import ClusterArchetypeDynamic

            ClusterArchetypeDynamic(
                canonical_name="Test",
                cluster_name="Test Cluster",
                core_identity="Test",
                genre_variants=[],
                uniqueness="not_a_real_uniqueness",  # invalid
            )

    def test_shadow_pairing_inversion_type_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.archetype_dynamics import ShadowPairing

            ShadowPairing(
                description="Some inversion",
                inversion_type="not_a_real_inversion",  # invalid
            )


# ---------------------------------------------------------------------------
# Spatial Topology
# ---------------------------------------------------------------------------


class TestSpatialTopology:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.spatial_topology import (
            SpatialTopologyEdge,
            TonalInheritance,
            TopologyDirectionality,
            TopologyFriction,
        )

        edge = SpatialTopologyEdge(
            genre_slug="folk-horror",
            source_setting="The Village",
            target_setting="The Forest",
            friction=TopologyFriction(
                type="ontological",
                level="high",
            ),
            directionality=TopologyDirectionality(
                type="asymmetric",
            ),
            tonal_inheritance=TonalInheritance(
                direction="inward",
            ),
        )
        assert edge.genre_slug == "folk-horror"
        assert edge.source_setting == "The Village"
        assert edge.traversal_cost == []
        assert edge.state_shift is None
        assert edge.agency is None
        assert edge.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.spatial_topology import (
            SpatialTopologyEdge,
            TonalInheritance,
            TopologyDirectionality,
            TopologyFriction,
            TraversalCost,
        )

        edge = SpatialTopologyEdge(
            genre_slug="thriller",
            source_setting="Safe House",
            target_setting="Hostile Territory",
            friction=TopologyFriction(
                type="social",
                level="medium",
                description="Foreign environment with local informants",
            ),
            directionality=TopologyDirectionality(
                type="bidirectional_unequal",
                forward_cost="Significant risk",
                return_cost="Possible compromise",
                description="Easier to enter than exit cleanly",
            ),
            agency="low",
            tonal_inheritance=TonalInheritance(
                direction="outward",
                resistance="Safe house tonal bubble holds for first act",
                contamination_threshold="After second incursion",
                description="Hostile territory bleeds back",
            ),
            traversal_cost=[
                TraversalCost(variable_id="safety", delta=0.3, description="Exposure risk"),
                TraversalCost(variable_id="cover", delta=0.5),
            ],
            state_shift="Safety→Vulnerability",
            flavor_text="You can go in, but coming back clean is another matter.",
        )
        data = edge.model_dump()
        restored = SpatialTopologyEdge.model_validate(data)
        assert restored == edge

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.spatial_topology import (
                SpatialTopologyEdge,
                TonalInheritance,
                TopologyDirectionality,
                TopologyFriction,
            )

            SpatialTopologyEdge(
                # missing genre_slug
                source_setting="Village",
                target_setting="Forest",
                friction=TopologyFriction(type="ontological", level="high"),
                directionality=TopologyDirectionality(type="one_way"),
                tonal_inheritance=TonalInheritance(direction="inward"),
            )

    def test_cluster_spatial_topology_construction(self):
        from narrative_data.schemas.shared import GenreVariant
        from narrative_data.schemas.spatial_topology import ClusterSpatialTopology

        cst = ClusterSpatialTopology(
            canonical_name="Threshold Crossing",
            cluster_name="Boundary Cluster",
            core_identity="The moment between safety and danger",
            genre_variants=[
                GenreVariant(genre_slug="folk-horror", variant_name="Village-to-Forest Edge"),
                GenreVariant(genre_slug="thriller", variant_name="Safe-House Egress"),
            ],
        )
        assert cst.canonical_name == "Threshold Crossing"
        assert len(cst.genre_variants) == 2

    def test_traversal_cost_delta_out_of_range_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.spatial_topology import TraversalCost

            TraversalCost(variable_id="safety", delta=1.5)  # invalid

    def test_traversal_cost_negative_delta_accepted(self):
        from narrative_data.schemas.spatial_topology import TraversalCost

        tc = TraversalCost(variable_id="safety", delta=-0.5)
        assert tc.delta == -0.5  # negative = traversal depletes variable

    def test_traversal_cost_delta_below_negative_one_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.spatial_topology import TraversalCost

            TraversalCost(variable_id="safety", delta=-1.5)  # invalid

    def test_friction_type_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.spatial_topology import TopologyFriction

            TopologyFriction(type="not_a_friction_type", level="high")

    def test_agency_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.spatial_topology import (
                SpatialTopologyEdge,
                TonalInheritance,
                TopologyDirectionality,
                TopologyFriction,
            )

            SpatialTopologyEdge(
                genre_slug="thriller",
                source_setting="A",
                target_setting="B",
                friction=TopologyFriction(type="social", level="low"),
                directionality=TopologyDirectionality(type="one_way"),
                agency="not_a_valid_agency",  # invalid
                tonal_inheritance=TonalInheritance(direction="mutual"),
            )


# ---------------------------------------------------------------------------
# Place Entities
# ---------------------------------------------------------------------------


class TestPlaceEntity:
    def _make_communicability(self):
        from narrative_data.schemas.place_entities import (
            AtmosphericChannel,
            PlaceCommunicability,
            SensoryChannel,
            SpatialChannel,
            TemporalChannel,
        )

        return PlaceCommunicability(
            atmospheric=AtmosphericChannel(mood="dread", intensity=0.8),
            sensory=SensoryChannel(dominant="olfactory"),
            spatial=SpatialChannel(enclosure="enclosed_intimate", orientation="labyrinthine"),
            temporal=TemporalChannel(time_model="cyclical"),
        )

    def test_minimal_valid_construction(self):
        from narrative_data.schemas.place_entities import EntityProperties, PlaceEntity

        pe = PlaceEntity(
            canonical_name="home",
            genre_slug="folk-horror",
            variant_name="The Ancestral Farmhouse",
            communicability=self._make_communicability(),
            entity_properties=EntityProperties(),
        )
        assert pe.canonical_name == "home"
        assert pe.genre_slug == "folk-horror"
        assert pe.state_variable_expression == []
        assert pe.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.place_entities import (
            AtmosphericChannel,
            EntityProperties,
            PlaceCommunicability,
            PlaceEntity,
            SensoryChannel,
            SpatialChannel,
            StateVariableExpression,
            TemporalChannel,
        )

        pe = PlaceEntity(
            canonical_name="threshold",
            genre_slug="folk-horror",
            variant_name="The Village Boundary Stone",
            communicability=PlaceCommunicability(
                atmospheric=AtmosphericChannel(
                    mood="liminal dread",
                    intensity=0.9,
                    shift_pattern="Intensifies at night",
                ),
                sensory=SensoryChannel(
                    dominant="auditory",
                    secondary=["tactile", "olfactory"],
                    description="The sound of the boundary",
                ),
                spatial=SpatialChannel(
                    enclosure="vast_exposed",
                    orientation="horizontal",
                    constraint="Cannot be approached directly",
                ),
                temporal=TemporalChannel(
                    time_model="cyclical",
                    pace_relation="Seasonal crossing rites",
                ),
            ),
            entity_properties=EntityProperties(
                has_agency=True,
                is_third_character=True,
                evolution_pattern="Sanctuary → Trap",
                topological_role="connector",
                role_can_shift=True,
            ),
            state_variable_expression=[
                StateVariableExpression(
                    variable_id="community_pressure",
                    physical_manifestation="Stones seem closer together",
                )
            ],
            flavor_text="The boundary that decides who belongs.",
        )
        data = pe.model_dump()
        restored = PlaceEntity.model_validate(data)
        assert restored == pe

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.place_entities import EntityProperties, PlaceEntity

            PlaceEntity(
                # missing canonical_name
                genre_slug="folk-horror",
                variant_name="The Farmhouse",
                communicability=self._make_communicability(),
                entity_properties=EntityProperties(),
            )

    def test_cluster_place_entity_construction(self):
        from narrative_data.schemas.place_entities import ClusterPlaceEntity
        from narrative_data.schemas.shared import GenreVariant

        cpe = ClusterPlaceEntity(
            canonical_name="threshold",
            cluster_name="Liminal Place Cluster",
            core_identity="The place where ordinary rules no longer apply",
            genre_variants=[
                GenreVariant(genre_slug="folk-horror", variant_name="The Boundary Stone"),
                GenreVariant(genre_slug="epic-fantasy", variant_name="The Worldgate"),
            ],
        )
        assert cpe.canonical_name == "threshold"
        assert len(cpe.genre_variants) == 2

    def test_atmospheric_intensity_out_of_range_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.place_entities import AtmosphericChannel

            AtmosphericChannel(mood="dread", intensity=1.5)  # invalid

    def test_atmospheric_intensity_below_zero_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.place_entities import AtmosphericChannel

            AtmosphericChannel(mood="dread", intensity=-0.1)  # invalid

    def test_sensory_dominant_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.place_entities import SensoryChannel

            SensoryChannel(dominant="not_a_sense")  # invalid

    def test_topological_role_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.place_entities import EntityProperties

            EntityProperties(topological_role="not_a_role")  # invalid


# ---------------------------------------------------------------------------
# Tropes
# ---------------------------------------------------------------------------


class TestTrope:
    def test_minimal_valid_construction(self):
        from narrative_data.schemas.tropes import Trope

        t = Trope(
            name="The Reluctant Witness",
            genre_slug="folk-horror",
            genre_derivation=(
                "Community pressure + forbidden knowledge = character who sees but cannot speak"
            ),
            narrative_function=["establishing", "escalating"],
        )
        assert t.name == "The Reluctant Witness"
        assert t.genre_slug == "folk-horror"
        assert t.variants is None
        assert t.state_variable_interactions == []
        assert t.ontological_dimension is None
        assert t.overlap_signal == []
        assert t.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.shared import OverlapSignal, StateVariableInteraction
        from narrative_data.schemas.tropes import Trope, TropeVariants

        t = Trope(
            name="The Keeper of Secrets",
            genre_slug="gothic",
            genre_derivation="Enclosed space + generational guilt + information asymmetry",
            narrative_function=["characterizing", "connecting", "subverting"],
            variants=TropeVariants(
                straight="The faithful retainer who holds the family shame",
                inverted="The secret-keeper who reveals too early",
                deconstructed="The keeper who never knew what they were keeping",
                violation="The keeper who sells the secret",
            ),
            state_variable_interactions=[
                StateVariableInteraction(
                    variable_id="revelation_pressure",
                    operation="accumulates",
                    description="Each scene adds weight",
                )
            ],
            ontological_dimension="permeable_negotiable",
            overlap_signal=[
                OverlapSignal(
                    adjacent_genre="thriller",
                    similar_entity="The Informant",
                    differentiator="Gothic trope is generational, thriller's is transactional",
                )
            ],
            flavor_text="What is kept shapes the keeper.",
        )
        data = t.model_dump()
        restored = Trope.model_validate(data)
        assert restored == t

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.tropes import Trope

            Trope(
                # missing name
                genre_slug="folk-horror",
                genre_derivation="Some derivation",
                narrative_function=["establishing"],
            )

    def test_narrative_function_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.tropes import Trope

            Trope(
                name="Test Trope",
                genre_slug="folk-horror",
                genre_derivation="Some derivation",
                narrative_function=["not_a_function"],  # invalid
            )

    def test_narrative_function_multiple_values(self):
        from narrative_data.schemas.tropes import Trope

        t = Trope(
            name="Multi-function Trope",
            genre_slug="thriller",
            genre_derivation="Many roles",
            narrative_function=["establishing", "connecting", "escalating", "resolving"],
        )
        assert len(t.narrative_function) == 4


# ---------------------------------------------------------------------------
# Narrative Shapes
# ---------------------------------------------------------------------------


class TestNarrativeShape:
    def _make_minimal_beat(self):
        from narrative_data.schemas.narrative_shapes import Beat

        return Beat(
            name="Inciting Incident",
            position=0.1,
            flexibility="load_bearing",
            tension_effect="builds",
        )

    def test_minimal_valid_construction(self):
        from narrative_data.schemas.narrative_shapes import NarrativeShape, TensionProfile

        ns = NarrativeShape(
            name="The Pressure Cooker",
            genre_slug="thriller",
            tension_profile=TensionProfile(family="pressure"),
            beats=[self._make_minimal_beat()],
        )
        assert ns.name == "The Pressure Cooker"
        assert ns.genre_slug == "thriller"
        assert ns.rest_beats == []
        assert ns.composability is None
        assert ns.overlap_signal is None
        assert ns.flavor_text is None

    def test_round_trip(self):
        from narrative_data.schemas.narrative_shapes import (
            Beat,
            Composability,
            NarrativeShape,
            RestBeat,
            ShapeOverlapSignal,
            TensionProfile,
        )

        ns = NarrativeShape(
            name="The Countdown",
            genre_slug="thriller",
            tension_profile=TensionProfile(
                family="countdown",
                description="Ticking clock drives everything",
                distinctive_feature="Explicit deadline creates irresistible forward momentum",
            ),
            beats=[
                Beat(
                    name="The Clock Starts",
                    dramatic_function="Stakes established",
                    position=0.05,
                    flexibility="load_bearing",
                    tension_effect="builds",
                    pacing_effect="accelerates",
                    state_thresholds={"urgency": 0.3, "danger": 0.2},
                    genre_constraints=["Deadline must be externally imposed"],
                    flavor_text="The moment everything starts running.",
                ),
                Beat(
                    name="False Hope",
                    position=0.6,
                    flexibility="ornamental",
                    tension_effect="redirects",
                    pacing_effect="decelerates",
                ),
                Beat(
                    name="Final Convergence",
                    position=0.9,
                    flexibility="load_bearing",
                    tension_effect="peaks",
                    pacing_effect="erratic",
                ),
            ],
            rest_beats=[
                RestBeat(
                    type="recovery",
                    tension_behavior="releases",
                    description="Brief breathing room before the final act",
                    genre_constraints=["Must not exceed 10% of arc length"],
                )
            ],
            composability=Composability(
                can_layer_with=["oscillating", "spiral"],
                layer_type="parallel_tracks",
            ),
            overlap_signal=ShapeOverlapSignal(
                incompatible_physics="Cannot coexist with residual shapes",
                neighboring_shapes=["pressure", "spiral"],
            ),
            flavor_text="Time is the villain.",
        )
        data = ns.model_dump()
        restored = NarrativeShape.model_validate(data)
        assert restored == ns

    def test_missing_required_field_raises(self):
        with pytest.raises((ValidationError, TypeError)):
            from narrative_data.schemas.narrative_shapes import NarrativeShape, TensionProfile

            NarrativeShape(
                # missing name
                genre_slug="thriller",
                tension_profile=TensionProfile(family="countdown"),
                beats=[],
            )

    def test_beat_position_out_of_range_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import Beat

            Beat(
                name="Bad Beat",
                position=1.5,  # invalid
                flexibility="load_bearing",
                tension_effect="builds",
            )

    def test_beat_position_below_zero_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import Beat

            Beat(
                name="Bad Beat",
                position=-0.1,  # invalid
                flexibility="load_bearing",
                tension_effect="builds",
            )

    def test_state_thresholds_out_of_range_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import Beat

            Beat(
                name="Bad Threshold Beat",
                position=0.5,
                flexibility="load_bearing",
                tension_effect="builds",
                state_thresholds={"urgency": 2.0},  # invalid
            )

    def test_tension_family_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import TensionProfile

            TensionProfile(family="not_a_family")  # invalid

    def test_beat_flexibility_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import Beat

            Beat(
                name="Test",
                position=0.5,
                flexibility="not_a_flexibility",  # invalid
                tension_effect="builds",
            )

    def test_beat_tension_effect_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import Beat

            Beat(
                name="Test",
                position=0.5,
                flexibility="load_bearing",
                tension_effect="not_an_effect",  # invalid
            )

    def test_rest_beat_type_enum_invalid_rejected(self):
        with pytest.raises(ValidationError):
            from narrative_data.schemas.narrative_shapes import RestBeat

            RestBeat(type="not_a_type", tension_behavior="releases")  # invalid
