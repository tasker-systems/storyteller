"""Spatial topology schemas — per-genre and cluster models for setting connectivity.

Spatial topology describes the relational structure between settings: how movement between
places is shaped by friction, directionality, tonal contamination, and traversal cost.
Settings are not merely adjacent — they exert narrative forces on each other.
"""

from typing import Literal

from pydantic import BaseModel, field_validator

from narrative_data.schemas.shared import GenreVariant


class TopologyFriction(BaseModel):
    """The resistance that impedes or shapes movement between settings."""

    type: Literal["ontological", "social", "informational", "temporal", "environmental", "tonal"]
    level: Literal["high", "medium", "low"]
    description: str | None = None


class TopologyDirectionality(BaseModel):
    """How movement flows between two connected settings."""

    type: Literal["asymmetric", "one_way", "bidirectional_unequal", "circular", "progressive"]
    forward_cost: str | None = None
    return_cost: str | None = None
    description: str | None = None


class TonalInheritance(BaseModel):
    """How tone bleeds or transfers between connected settings."""

    direction: Literal["inward", "outward", "mutual", "masking", "saturation", "total"]
    resistance: str | None = None
    contamination_threshold: str | None = None
    description: str | None = None


class TraversalCost(BaseModel):
    """The cost to a specific state variable of traversing an edge."""

    variable_id: str
    delta: float
    description: str | None = None

    @field_validator("delta")
    @classmethod
    def delta_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"delta must be 0.0-1.0, got {v}")
        return v


class SpatialTopologyEdge(BaseModel):
    """Per-genre edge between two settings in the spatial topology graph."""

    genre_slug: str
    source_setting: str
    target_setting: str
    friction: TopologyFriction
    directionality: TopologyDirectionality
    agency: Literal["high", "medium", "low", "illusion", "none"] | None = None
    tonal_inheritance: TonalInheritance
    traversal_cost: list[TraversalCost] = []
    state_shift: str | None = None
    flavor_text: str | None = None


class ClusterSpatialTopology(BaseModel):
    """Cluster-level spatial topology capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    flavor_text: str | None = None


__all__ = [
    "ClusterSpatialTopology",
    "SpatialTopologyEdge",
    "TonalInheritance",
    "TopologyDirectionality",
    "TopologyFriction",
    "TraversalCost",
]
