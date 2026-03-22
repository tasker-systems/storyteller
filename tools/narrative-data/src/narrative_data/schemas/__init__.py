"""Narrative data schemas for Stage 2 JSON structuring.

All schema types are re-exported here so consumers can write:
    from narrative_data.schemas import Archetype, Dynamic, NarrativeShape, ...
"""

# ---------------------------------------------------------------------------
# Shared primitives
# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# Remaining types (5 types)
# ---------------------------------------------------------------------------
from narrative_data.schemas.archetype_dynamics import (
    ArchetypeDynamic,
    CharacteristicScene,
    ClusterArchetypeDynamic,
    EdgeProperties,
    ShadowPairing,
)

# ---------------------------------------------------------------------------
# Discovery types (6 types)
# ---------------------------------------------------------------------------
from narrative_data.schemas.archetypes import (
    Archetype,
    ClusterArchetype,
    PersonalityProfile,
)
from narrative_data.schemas.dynamics import (
    ClusterDynamic,
    Dynamic,
    RoleSlot,
    ScaleManifestations,
)

# ---------------------------------------------------------------------------
# Genre dimensions (foundation schema)
# ---------------------------------------------------------------------------
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
from narrative_data.schemas.goals import (
    ClusterGoal,
    CrossScaleTension,
    Goal,
)
from narrative_data.schemas.narrative_shapes import (
    Beat,
    Composability,
    NarrativeShape,
    RestBeat,
    ShapeOverlapSignal,
    TensionProfile,
)
from narrative_data.schemas.ontological_posture import (
    ClusterOntologicalPosture,
    EthicalOrientation,
    ModeOfBeing,
    OntologicalPosture,
    SelfOtherBoundary,
)
from narrative_data.schemas.place_entities import (
    AtmosphericChannel,
    ClusterPlaceEntity,
    EntityProperties,
    PlaceCommunicability,
    PlaceEntity,
    SensoryChannel,
    SpatialChannel,
    StateVariableExpression,
    TemporalChannel,
)
from narrative_data.schemas.scene_profiles import (
    ClusterSceneProfile,
    SceneDimensionalProperties,
    SceneProfile,
)
from narrative_data.schemas.settings import (
    ClusterSettings,
    SettingCommunicability,
    Settings,
)
from narrative_data.schemas.shared import (
    ContinuousAxis,
    GenreBoundary,
    GenreVariant,
    OverlapSignal,
    StateVariableInteraction,
    StateVariableTemplate,
    WeightedTags,
)
from narrative_data.schemas.spatial_topology import (
    ClusterSpatialTopology,
    SpatialTopologyEdge,
    TonalInheritance,
    TopologyDirectionality,
    TopologyFriction,
    TraversalCost,
)
from narrative_data.schemas.tropes import (
    Trope,
    TropeVariants,
)

__all__ = [
    # shared
    "ContinuousAxis",
    "GenreBoundary",
    "GenreVariant",
    "OverlapSignal",
    "StateVariableInteraction",
    "StateVariableTemplate",
    "WeightedTags",
    # genre dimensions
    "AestheticDimensions",
    "AgencyDimensions",
    "EpistemologicalDimensions",
    "GenreDimensions",
    "NarrativeContract",
    "TemporalDimensions",
    "ThematicDimensions",
    "TonalDimensions",
    "WorldAffordances",
    # archetypes
    "Archetype",
    "ClusterArchetype",
    "PersonalityProfile",
    # dynamics
    "ClusterDynamic",
    "Dynamic",
    "RoleSlot",
    "ScaleManifestations",
    # goals
    "ClusterGoal",
    "CrossScaleTension",
    "Goal",
    # ontological posture
    "ClusterOntologicalPosture",
    "EthicalOrientation",
    "ModeOfBeing",
    "OntologicalPosture",
    "SelfOtherBoundary",
    # scene profiles
    "ClusterSceneProfile",
    "SceneDimensionalProperties",
    "SceneProfile",
    # settings
    "ClusterSettings",
    "SettingCommunicability",
    "Settings",
    # archetype dynamics
    "ArchetypeDynamic",
    "CharacteristicScene",
    "ClusterArchetypeDynamic",
    "EdgeProperties",
    "ShadowPairing",
    # narrative shapes
    "Beat",
    "Composability",
    "NarrativeShape",
    "RestBeat",
    "ShapeOverlapSignal",
    "TensionProfile",
    # place entities
    "AtmosphericChannel",
    "ClusterPlaceEntity",
    "EntityProperties",
    "PlaceCommunicability",
    "PlaceEntity",
    "SensoryChannel",
    "SpatialChannel",
    "StateVariableExpression",
    "TemporalChannel",
    # spatial topology
    "ClusterSpatialTopology",
    "SpatialTopologyEdge",
    "TonalInheritance",
    "TopologyDirectionality",
    "TopologyFriction",
    "TraversalCost",
    # tropes
    "Trope",
    "TropeVariants",
]
