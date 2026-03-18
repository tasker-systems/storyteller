"""Pydantic schemas for narrative data entities."""

from narrative_data.schemas.genre import (
    GenreArchetype,
    GenreDynamic,
    GenreGoal,
    GenreProfile,
    GenreRegion,
    GenreSetting,
    NarrativeBeat,
    NarrativeShape,
    SubversionPattern,
    Trope,
    WorldAffordances,
)
from narrative_data.schemas.intersections import (
    Enrichment,
    IntersectionSynthesis,
    UpstreamRef,
)
from narrative_data.schemas.shared import (
    DimensionalPosition,
    GenerationProvenance,
    NarrativeEntity,
    ProvenanceEdge,
)
from narrative_data.schemas.spatial import (
    CommunicabilityProfile,
    PlaceEntity,
    SensoryDetail,
    SettingType,
    TonalInheritanceRule,
    TopologyEdge,
)

__all__ = [
    # shared
    "DimensionalPosition",
    "GenerationProvenance",
    "NarrativeEntity",
    "ProvenanceEdge",
    # genre
    "GenreArchetype",
    "GenreDynamic",
    "GenreGoal",
    "GenreProfile",
    "GenreRegion",
    "GenreSetting",
    "NarrativeBeat",
    "NarrativeShape",
    "SubversionPattern",
    "Trope",
    "WorldAffordances",
    # spatial
    "CommunicabilityProfile",
    "PlaceEntity",
    "SensoryDetail",
    "SettingType",
    "TonalInheritanceRule",
    "TopologyEdge",
    # intersections
    "Enrichment",
    "IntersectionSynthesis",
    "UpstreamRef",
]
