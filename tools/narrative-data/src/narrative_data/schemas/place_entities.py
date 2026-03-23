# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Place entity schemas — per-genre and cluster models for universal place archetypes.

Place entities are the five universal place archetypes (home, archive, threshold, authority,
climax) that recur across genres with genre-specific variant names. Unlike general settings,
place entities carry communicability profiles across four channels and can evolve narrative
roles within a story.
"""

from typing import Literal

from pydantic import BaseModel, Field, field_validator

from narrative_data.schemas.shared import GenreVariant, StateVariableInteraction


class AtmosphericChannel(BaseModel):
    """Atmospheric communicability of a place."""

    mood: str = Field(..., json_schema_extra={"tier": "core"})
    intensity: float = Field(..., json_schema_extra={"tier": "core"})
    shift_pattern: str | None = Field(None, json_schema_extra={"tier": "extended"})

    @field_validator("intensity")
    @classmethod
    def intensity_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"intensity must be 0.0-1.0, got {v}")
        return v


class SensoryChannel(BaseModel):
    """Dominant and secondary sensory registers of a place."""

    dominant: Literal["olfactory", "tactile", "auditory", "visual", "proprioceptive"] = Field(..., json_schema_extra={"tier": "core"})
    secondary: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    description: str | None = Field(None, json_schema_extra={"tier": "extended"})


class SpatialChannel(BaseModel):
    """How a place communicates through its spatial structure."""

    enclosure: Literal["enclosed_intimate", "vast_exposed", "fluid"] = Field(..., json_schema_extra={"tier": "core"})
    orientation: Literal["vertical", "horizontal", "labyrinthine"] = Field(..., json_schema_extra={"tier": "core"})
    constraint: str | None = Field(None, json_schema_extra={"tier": "extended"})


class TemporalChannel(BaseModel):
    """How a place communicates through its relationship to time."""

    time_model: Literal["cyclical", "linear", "frozen", "compressed", "decaying"] = Field(..., json_schema_extra={"tier": "core"})
    pace_relation: str | None = Field(None, json_schema_extra={"tier": "extended"})


class PlaceCommunicability(BaseModel):
    """Four-channel communicability profile for a place entity."""

    atmospheric: AtmosphericChannel = Field(..., json_schema_extra={"tier": "core"})
    sensory: SensoryChannel = Field(..., json_schema_extra={"tier": "core"})
    spatial: SpatialChannel = Field(..., json_schema_extra={"tier": "core"})
    temporal: TemporalChannel = Field(..., json_schema_extra={"tier": "core"})


class EntityProperties(BaseModel):
    """Structural and role properties of a place entity within a narrative."""

    has_agency: bool = Field(False, json_schema_extra={"tier": "core"})
    is_third_character: bool = Field(False, json_schema_extra={"tier": "core"})
    evolution_pattern: str | None = Field(None, json_schema_extra={"tier": "extended"})
    topological_role: Literal["hub", "endpoint", "connector", "branch", "buffer"] | None = Field(None, json_schema_extra={"tier": "core"})
    role_can_shift: bool = Field(False, json_schema_extra={"tier": "core"})


class StateVariableExpression(BaseModel):
    """How a state variable manifests physically in a place."""

    variable_id: str = Field(..., json_schema_extra={"tier": "core"})
    physical_manifestation: str = Field(..., json_schema_extra={"tier": "extended"})


class PlaceEntity(BaseModel):
    """Per-genre place entity — genre-specific variant of a universal place archetype."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    variant_name: str = Field(..., json_schema_extra={"tier": "core"})
    communicability: PlaceCommunicability = Field(..., json_schema_extra={"tier": "core"})
    entity_properties: EntityProperties = Field(..., json_schema_extra={"tier": "core"})
    state_variable_expression: list[StateVariableExpression] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterPlaceEntity(BaseModel):
    """Cluster-level place entity capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


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
