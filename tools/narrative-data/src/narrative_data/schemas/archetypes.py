# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Archetype schemas — per-genre and cluster models for character archetypes.

Archetypes are the load-bearing character roles in a genre. Each archetype has a
canonical name (cross-genre identity), a genre-specific variant, a personality
profile across 7 axes, and optional extended axes for cluster-specific dimensions.
"""

from typing import Literal

from pydantic import BaseModel, Field, field_validator

from narrative_data.schemas.shared import GenreVariant, OverlapSignal, StateVariableInteraction


class PersonalityProfile(BaseModel):
    """Seven-axis personality profile, all values normalized 0.0-1.0."""

    warmth: float = Field(..., json_schema_extra={"tier": "core"})
    authority: float = Field(..., json_schema_extra={"tier": "core"})
    openness: float = Field(..., json_schema_extra={"tier": "core"})
    interiority: float = Field(..., json_schema_extra={"tier": "core"})
    stability: float = Field(..., json_schema_extra={"tier": "core"})
    agency: float = Field(..., json_schema_extra={"tier": "core"})
    morality: float = Field(..., json_schema_extra={"tier": "core"})

    @field_validator(
        "warmth", "authority", "openness", "interiority", "stability", "agency", "morality"
    )
    @classmethod
    def axis_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"personality axis value must be 0.0-1.0, got {v}")
        return v


class Archetype(BaseModel):
    """Per-genre archetype model capturing genre-specific variant detail."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    variant_name: str = Field(..., json_schema_extra={"tier": "core"})
    personality_profile: PersonalityProfile = Field(..., json_schema_extra={"tier": "core"})
    extended_axes: dict[str, float] = Field(default_factory=dict, json_schema_extra={"tier": "extended"})
    distinguishing_tension: str = Field(..., json_schema_extra={"tier": "core"})
    structural_necessity: str = Field(..., json_schema_extra={"tier": "core"})
    overlap_signals: list[OverlapSignal] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    state_variables: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    universality: Literal["universal", "cluster_specific", "genre_unique"] = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})

    @field_validator("extended_axes")
    @classmethod
    def extended_axes_in_unit_interval(cls, v: dict[str, float]) -> dict[str, float]:
        for axis, value in v.items():
            if not 0.0 <= value <= 1.0:
                raise ValueError(f"extended_axes['{axis}'] must be 0.0-1.0, got {value}")
        return v


class ClusterArchetype(BaseModel):
    """Cluster-level archetype capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    uniqueness: Literal["universal", "cluster_specific", "genre_unique"] = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "Archetype",
    "ClusterArchetype",
    "PersonalityProfile",
    "StateVariableInteraction",
]
