"""Narrative shape schemas — per-genre models for tension arc structures.

Narrative shapes describe how tension moves through a story across its full arc. Each shape
has a tension profile family, load-bearing beats with normalized positions, rest beats that
provide breathing room, and composability signals for multi-shape scenes. Shapes have no
cluster variant — they are genre-native structural patterns.
"""

from typing import Literal

from pydantic import BaseModel, field_validator

TensionFamilyLiteral = Literal[
    "monotonic_build",
    "oscillating",
    "pressure",
    "countdown",
    "spiral",
    "layered",
    "residual",
    "inverted",
]
FlexibilityLiteral = Literal["load_bearing", "ornamental"]
TensionEffectLiteral = Literal[
    "builds", "sustains", "redirects", "releases", "transforms", "peaks"
]
PacingEffectLiteral = Literal["accelerates", "decelerates", "erratic"]
RestBeatTypeLiteral = Literal["ambient", "recovery", "transit", "approach"]
TensionBehaviorLiteral = Literal["releases", "compounds", "redirects", "maintains"]
LayerTypeLiteral = Literal["parallel_tracks", "nested", "separated_axes"]


class TensionProfile(BaseModel):
    """The family and character of a narrative shape's tension movement."""

    family: TensionFamilyLiteral
    description: str | None = None
    distinctive_feature: str | None = None


class Beat(BaseModel):
    """A single load-bearing or ornamental moment in the narrative arc."""

    name: str
    dramatic_function: str | None = None
    position: float
    flexibility: FlexibilityLiteral
    tension_effect: TensionEffectLiteral
    pacing_effect: PacingEffectLiteral | None = None
    state_thresholds: dict[str, float] = {}
    genre_constraints: list[str] = []
    flavor_text: str | None = None

    @field_validator("position")
    @classmethod
    def position_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"position must be 0.0-1.0, got {v}")
        return v

    @field_validator("state_thresholds")
    @classmethod
    def state_thresholds_in_unit_interval(cls, v: dict[str, float]) -> dict[str, float]:
        for key, threshold in v.items():
            if not 0.0 <= threshold <= 1.0:
                raise ValueError(
                    f"state_thresholds['{key}'] must be 0.0-1.0, got {threshold}"
                )
        return v


class RestBeat(BaseModel):
    """A rest moment in the arc that modulates tension without advancing plot."""

    type: RestBeatTypeLiteral
    tension_behavior: TensionBehaviorLiteral
    description: str | None = None
    genre_constraints: list[str] = []


class Composability(BaseModel):
    """How this narrative shape can combine with other shapes."""

    can_layer_with: list[str] = []
    layer_type: LayerTypeLiteral | None = None


class ShapeOverlapSignal(BaseModel):
    """Cross-genre overlap markers for narrative shapes."""

    incompatible_physics: str | None = None
    neighboring_shapes: list[str] = []


class NarrativeShape(BaseModel):
    """Per-genre narrative shape model capturing tension arc structure."""

    name: str
    genre_slug: str
    tension_profile: TensionProfile
    beats: list[Beat]
    rest_beats: list[RestBeat] = []
    composability: Composability | None = None
    overlap_signal: ShapeOverlapSignal | None = None
    flavor_text: str | None = None


__all__ = [
    "Beat",
    "Composability",
    "NarrativeShape",
    "RestBeat",
    "ShapeOverlapSignal",
    "TensionProfile",
]
