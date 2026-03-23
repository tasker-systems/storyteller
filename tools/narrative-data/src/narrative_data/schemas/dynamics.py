# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Dynamic schemas — per-genre and cluster models for relational dynamics.

Dynamics are the structured relationships and forces that operate between characters
and entities in a narrative. Each dynamic has a canonical name, scale, edge type,
and directionality. Trans-scalar dynamics can operate across multiple scales.
"""

from typing import Literal

from pydantic import BaseModel

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

    role: str
    want: str | None = None
    withhold: str | None = None
    archetype_ref: str | None = None
    is_nonhuman: bool = False


class ScaleManifestations(BaseModel):
    """How a dynamic manifests differently at each narrative scale."""

    orbital: str | None = None
    arc: str | None = None
    scene: str | None = None


class Dynamic(BaseModel):
    """Per-genre dynamic model capturing relational forces between entities."""

    canonical_name: str
    genre_slug: str
    variant_name: str
    scale: ScaleLiteral
    spans_scales: list[ScaleLiteral] = []
    edge_type: str
    directionality: DirectionalityLiteral
    currencies: list[str] = []
    network_position: NetworkPositionLiteral | None = None
    valence: ValenceLiteral | None = None
    role_slots: list[RoleSlot] = []
    evolution_pattern: EvolutionPatternLiteral | None = None
    scale_manifestations: ScaleManifestations | None = None
    state_variable_interactions: list[StateVariableInteraction] = []
    constraints: list[str] = []
    overlap_signal: str | None = None
    flavor_text: str | None = None


class ClusterDynamic(BaseModel):
    """Cluster-level dynamic capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    uniqueness: Literal["universal", "cluster_specific", "genre_unique"]
    flavor_text: str | None = None


__all__ = [
    "ClusterDynamic",
    "Dynamic",
    "RoleSlot",
    "ScaleManifestations",
]
