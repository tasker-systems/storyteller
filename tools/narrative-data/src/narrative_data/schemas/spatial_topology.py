# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Spatial topology schemas — per-genre and cluster models for setting connectivity.

Spatial topology describes the relational structure between settings: how movement between
places is shaped by friction, directionality, tonal contamination, and traversal cost.
Settings are not merely adjacent — they exert narrative forces on each other.
"""

from typing import Literal

from pydantic import BaseModel, Field, field_validator

from narrative_data.schemas.shared import GenreVariant


class TopologyFriction(BaseModel):
    """The resistance that impedes or shapes movement between settings."""

    type: Literal["ontological", "social", "informational", "temporal", "environmental", "tonal"] = Field(..., json_schema_extra={"tier": "core"})
    level: Literal["high", "medium", "low"] = Field(..., json_schema_extra={"tier": "core"})
    description: str | None = Field(None, json_schema_extra={"tier": "extended"})


class TopologyDirectionality(BaseModel):
    """How movement flows between two connected settings."""

    type: Literal["asymmetric", "one_way", "bidirectional_unequal", "circular", "progressive"] = Field(..., json_schema_extra={"tier": "core"})
    forward_cost: str | None = Field(None, json_schema_extra={"tier": "extended"})
    return_cost: str | None = Field(None, json_schema_extra={"tier": "extended"})
    description: str | None = Field(None, json_schema_extra={"tier": "extended"})


class TonalInheritance(BaseModel):
    """How tone bleeds or transfers between connected settings."""

    direction: Literal["inward", "outward", "mutual", "masking", "saturation", "total"] = Field(..., json_schema_extra={"tier": "core"})
    resistance: str | None = Field(None, json_schema_extra={"tier": "extended"})
    contamination_threshold: str | None = Field(None, json_schema_extra={"tier": "extended"})
    description: str | None = Field(None, json_schema_extra={"tier": "extended"})


class TraversalCost(BaseModel):
    """The cost to a specific state variable of traversing an edge."""

    variable_id: str = Field(..., json_schema_extra={"tier": "core"})
    delta: float = Field(..., json_schema_extra={"tier": "core"})
    description: str | None = Field(None, json_schema_extra={"tier": "extended"})

    @field_validator("delta")
    @classmethod
    def delta_in_unit_interval(cls, v: float) -> float:
        if not -1.0 <= v <= 1.0:
            raise ValueError(f"delta must be -1.0 to 1.0, got {v}")
        return v


class SpatialTopologyEdge(BaseModel):
    """Per-genre edge between two settings in the spatial topology graph."""

    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    source_setting: str = Field(..., json_schema_extra={"tier": "core"})
    target_setting: str = Field(..., json_schema_extra={"tier": "core"})
    friction: TopologyFriction = Field(..., json_schema_extra={"tier": "core"})
    directionality: TopologyDirectionality = Field(..., json_schema_extra={"tier": "core"})
    agency: Literal["high", "medium", "low", "illusion", "none"] | None = Field(None, json_schema_extra={"tier": "core"})
    tonal_inheritance: TonalInheritance = Field(..., json_schema_extra={"tier": "core"})
    traversal_cost: list[TraversalCost] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    state_shift: str | None = Field(None, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterSpatialTopology(BaseModel):
    """Cluster-level spatial topology capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ClusterSpatialTopology",
    "SpatialTopologyEdge",
    "TonalInheritance",
    "TopologyDirectionality",
    "TopologyFriction",
    "TraversalCost",
]
