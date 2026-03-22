"""Archetype schemas — per-genre and cluster models for character archetypes.

Archetypes are the load-bearing character roles in a genre. Each archetype has a
canonical name (cross-genre identity), a genre-specific variant, a personality
profile across 7 axes, and optional extended axes for cluster-specific dimensions.
"""

from typing import Literal

from pydantic import BaseModel, field_validator

from narrative_data.schemas.shared import GenreVariant, OverlapSignal, StateVariableInteraction


class PersonalityProfile(BaseModel):
    """Seven-axis personality profile, all values normalized 0.0-1.0."""

    warmth: float
    authority: float
    openness: float
    interiority: float
    stability: float
    agency: float
    morality: float

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

    canonical_name: str
    genre_slug: str
    variant_name: str
    personality_profile: PersonalityProfile
    extended_axes: dict[str, float] = {}
    distinguishing_tension: str
    structural_necessity: str
    overlap_signals: list[OverlapSignal] = []
    state_variables: list[str] = []
    universality: Literal["universal", "cluster_specific", "genre_unique"]
    flavor_text: str | None = None

    @field_validator("extended_axes")
    @classmethod
    def extended_axes_in_unit_interval(cls, v: dict[str, float]) -> dict[str, float]:
        for axis, value in v.items():
            if not 0.0 <= value <= 1.0:
                raise ValueError(f"extended_axes['{axis}'] must be 0.0-1.0, got {value}")
        return v


class ClusterArchetype(BaseModel):
    """Cluster-level archetype capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    uniqueness: Literal["universal", "cluster_specific", "genre_unique"]
    flavor_text: str | None = None


__all__ = [
    "Archetype",
    "ClusterArchetype",
    "PersonalityProfile",
    "StateVariableInteraction",
]
