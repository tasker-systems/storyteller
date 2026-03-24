# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Goal schemas — per-genre and cluster models for character motivational patterns.

Goals are the core motivational patterns that drive characters and plots. They operate
at different narrative scales and can create cross-scale tensions when scene-level goals
conflict with existential ones.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.shared import GenreVariant, StateVariableInteraction

ScaleLiteral = Literal["existential", "arc", "scene", "cross_scale"]
UniquenessLiteral = Literal["universal", "cluster_specific", "genre_unique"]
CrossScaleTensionTypeLiteral = Literal[
    "scene_costs_existential",
    "arc_requires_existential_betrayal",
    "mutual_undermining",
]


class CrossScaleTension(BaseModel):
    """A tension between goals operating at different narrative scales."""

    tension_type: CrossScaleTensionTypeLiteral = Field(..., json_schema_extra={"tier": "core"})
    description: str | None = Field(None, json_schema_extra={"tier": "extended"})


class Goal(BaseModel):
    """Per-genre goal model capturing motivational patterns and cross-scale tensions."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    variant_name: str = Field(..., json_schema_extra={"tier": "core"})
    scale: ScaleLiteral = Field(..., json_schema_extra={"tier": "core"})
    description: str | None = Field(None, json_schema_extra={"tier": "core"})
    cross_scale_tension: CrossScaleTension | None = Field(None, json_schema_extra={"tier": "extended"})
    archetype_refs: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    state_variable_interactions: list[StateVariableInteraction] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    uniqueness: UniquenessLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterGoal(BaseModel):
    """Cluster-level goal capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    uniqueness: UniquenessLiteral = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ClusterGoal",
    "CrossScaleTension",
    "Goal",
]
