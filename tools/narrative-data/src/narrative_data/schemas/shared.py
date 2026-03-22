"""Shared primitives for all narrative data schemas.

Design contract: all continuous numeric values are normalized 0.0-1.0 floats.
This is enforced by field validators and applies across all schema types.
"""

from typing import Literal

from pydantic import BaseModel, RootModel, field_validator


class ContinuousAxis(BaseModel):
    """Dimensional positioning with labels and prose."""

    value: float
    low_label: str | None = None
    high_label: str | None = None
    can_be_state_variable: bool = False
    flavor_text: str | None = None

    @field_validator("value")
    @classmethod
    def value_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"value must be 0.0-1.0, got {v}")
        return v


class WeightedTags(RootModel[dict[str, float]]):
    """Weighted tag set, e.g. {"Stewardship": 0.7, "Corruption": 0.3}."""

    @field_validator("root")
    @classmethod
    def values_in_unit_interval(cls, v: dict[str, float]) -> dict[str, float]:
        for tag, weight in v.items():
            if not 0.0 <= weight <= 1.0:
                raise ValueError(f"weight for '{tag}' must be 0.0-1.0, got {weight}")
        return v


class StateVariableTemplate(BaseModel):
    """Dynamic variable configuration per genre."""

    canonical_id: str
    genre_label: str
    behavior: Literal["depleting", "accumulating", "fluctuating", "progression", "countdown"]
    initial_value: float | None = None
    threshold: float | None = None
    threshold_effect: str | None = None
    activation_condition: str | None = None

    @field_validator("initial_value", "threshold")
    @classmethod
    def optional_unit_interval(cls, v: float | None) -> float | None:
        if v is not None and not 0.0 <= v <= 1.0:
            raise ValueError(f"value must be 0.0-1.0, got {v}")
        return v


class StateVariableInteraction(BaseModel):
    """How an entity affects a state variable."""

    variable_id: str
    operation: Literal["consumes", "accumulates", "depletes", "transforms", "gates"]
    description: str | None = None


class OverlapSignal(BaseModel):
    """Cross-genre boundary marker."""

    adjacent_genre: str
    similar_entity: str
    differentiator: str


class GenreBoundary(BaseModel):
    """Genre transition detection trigger."""

    trigger: str
    drift_target: str
    description: str


class GenreVariant(BaseModel):
    """A genre-specific variant of a canonical entity."""

    genre_slug: str
    variant_name: str
    key_differences: str | None = None
