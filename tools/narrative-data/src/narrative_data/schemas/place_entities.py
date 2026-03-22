"""Place entity schemas — per-genre and cluster models for universal place archetypes.

Place entities are the five universal place archetypes (home, archive, threshold, authority,
climax) that recur across genres with genre-specific variant names. Unlike general settings,
place entities carry communicability profiles across four channels and can evolve narrative
roles within a story.
"""

from typing import Literal

from pydantic import BaseModel, field_validator

from narrative_data.schemas.shared import GenreVariant, StateVariableInteraction


class AtmosphericChannel(BaseModel):
    """Atmospheric communicability of a place."""

    mood: str
    intensity: float
    shift_pattern: str | None = None

    @field_validator("intensity")
    @classmethod
    def intensity_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"intensity must be 0.0-1.0, got {v}")
        return v


class SensoryChannel(BaseModel):
    """Dominant and secondary sensory registers of a place."""

    dominant: Literal["olfactory", "tactile", "auditory", "visual", "proprioceptive"]
    secondary: list[str] = []
    description: str | None = None


class SpatialChannel(BaseModel):
    """How a place communicates through its spatial structure."""

    enclosure: Literal["enclosed_intimate", "vast_exposed", "fluid"]
    orientation: Literal["vertical", "horizontal", "labyrinthine"]
    constraint: str | None = None


class TemporalChannel(BaseModel):
    """How a place communicates through its relationship to time."""

    time_model: Literal["cyclical", "linear", "frozen", "compressed", "decaying"]
    pace_relation: str | None = None


class PlaceCommunicability(BaseModel):
    """Four-channel communicability profile for a place entity."""

    atmospheric: AtmosphericChannel
    sensory: SensoryChannel
    spatial: SpatialChannel
    temporal: TemporalChannel


class EntityProperties(BaseModel):
    """Structural and role properties of a place entity within a narrative."""

    has_agency: bool = False
    is_third_character: bool = False
    evolution_pattern: str | None = None
    topological_role: Literal["hub", "endpoint", "connector", "branch", "buffer"] | None = None
    role_can_shift: bool = False


class StateVariableExpression(BaseModel):
    """How a state variable manifests physically in a place."""

    variable_id: str
    physical_manifestation: str


class PlaceEntity(BaseModel):
    """Per-genre place entity — genre-specific variant of a universal place archetype."""

    canonical_name: str
    genre_slug: str
    variant_name: str
    communicability: PlaceCommunicability
    entity_properties: EntityProperties
    state_variable_expression: list[StateVariableExpression] = []
    flavor_text: str | None = None


class ClusterPlaceEntity(BaseModel):
    """Cluster-level place entity capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    flavor_text: str | None = None


__all__ = [
    "AtmosphericChannel",
    "ClusterPlaceEntity",
    "EntityProperties",
    "PlaceCommunicability",
    "PlaceEntity",
    "SensoryChannel",
    "SpatialChannel",
    "StateVariableExpression",
    "StateVariableInteraction",
    "TemporalChannel",
]
