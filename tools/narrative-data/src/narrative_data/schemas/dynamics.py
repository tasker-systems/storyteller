# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Dynamic schemas — per-genre and cluster models for relational dynamics.

Dynamics are the structured relationships and forces that operate between characters
and entities in a narrative. Each dynamic has a canonical name, scale, edge type,
and directionality. Trans-scalar dynamics can operate across multiple scales.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.shared import GenreVariant, StateVariableInteraction

ScaleLiteral = Literal["orbital", "arc", "scene"]
DirectionalityLiteral = Literal[
    "unidirectional",
    "bidirectional_asymmetric",
    "symmetrical",
    "convergent",
    "internal",
]
NetworkPositionLiteral = Literal[
    "hub",
    "peripheral",
    "brokerage",
    "bridge",
    "triangulation",
    "enmeshed",
    "parallel",
    "isolation",
    "closed_loop",
]
ValenceLiteral = Literal[
    "sacred",
    "hostile",
    "nurturing",
    "indifferent",
    "parasitic",
    "transactional",
    "protective",
]
EvolutionPatternLiteral = Literal[
    "erosion", "deepening", "crystallization", "inversion", "oscillation", "static"
]


class RoleSlot(BaseModel):
    """A named position within a dynamic that a character can occupy."""

    role: str = Field(..., json_schema_extra={"tier": "core"})
    want: str | None = Field(None, json_schema_extra={"tier": "core"})
    withhold: str | None = Field(None, json_schema_extra={"tier": "core"})
    archetype_ref: str | None = Field(None, json_schema_extra={"tier": "core"})
    is_nonhuman: bool = Field(False, json_schema_extra={"tier": "core"})


class ScaleManifestations(BaseModel):
    """How a dynamic manifests differently at each narrative scale."""

    orbital: str | None = Field(None, json_schema_extra={"tier": "extended"})
    arc: str | None = Field(None, json_schema_extra={"tier": "extended"})
    scene: str | None = Field(None, json_schema_extra={"tier": "extended"})


class Dynamic(BaseModel):
    """Per-genre dynamic model capturing relational forces between entities."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    variant_name: str = Field(..., json_schema_extra={"tier": "core"})
    scale: ScaleLiteral = Field(..., json_schema_extra={"tier": "core"})
    spans_scales: list[ScaleLiteral] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    edge_type: str = Field(..., json_schema_extra={"tier": "core"})
    directionality: DirectionalityLiteral = Field(..., json_schema_extra={"tier": "core"})
    currencies: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    network_position: NetworkPositionLiteral | None = Field(None, json_schema_extra={"tier": "extended"})
    valence: ValenceLiteral | None = Field(None, json_schema_extra={"tier": "extended"})
    role_slots: list[RoleSlot] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    evolution_pattern: EvolutionPatternLiteral | None = Field(None, json_schema_extra={"tier": "extended"})
    scale_manifestations: ScaleManifestations | None = Field(None, json_schema_extra={"tier": "extended"})
    state_variable_interactions: list[StateVariableInteraction] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    constraints: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    overlap_signal: str | None = Field(None, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterDynamic(BaseModel):
    """Cluster-level dynamic capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    uniqueness: Literal["universal", "cluster_specific", "genre_unique"] = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ClusterDynamic",
    "Dynamic",
    "RoleSlot",
    "ScaleManifestations",
]
