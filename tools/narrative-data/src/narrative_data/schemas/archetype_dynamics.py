# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Archetype dynamics schemas — per-genre and cluster models for archetype pair relationships.

Archetype dynamics describe the structured relational forces between specific archetype
pairings within a genre. Unlike general dynamics (which operate between any entities),
archetype dynamics capture the canonical scene types, edge properties, and shadow inversions
specific to archetypal role relationships.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.dynamics import ScaleManifestations
from narrative_data.schemas.shared import GenreVariant

UniquenessLiteral = Literal["genre_unique", "cluster_specific", "genre_defining"]


class EdgeProperties(BaseModel):
    """Properties of the relational edge between two archetypes."""

    edge_type: str = Field(..., json_schema_extra={"tier": "core"})
    directionality: str = Field(..., json_schema_extra={"tier": "core"})
    currencies: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    network_position: str | None = Field(None, json_schema_extra={"tier": "extended"})
    constraints: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    weight: str | None = Field(None, json_schema_extra={"tier": "extended"})


class CharacteristicScene(BaseModel):
    """A canonical scene that exemplifies the archetype dynamic."""

    title: str = Field(..., json_schema_extra={"tier": "core"})
    opening: str | None = Field(None, json_schema_extra={"tier": "extended"})
    tension_source: str | None = Field(None, json_schema_extra={"tier": "extended"})
    withheld_by_a: str | None = Field(None, json_schema_extra={"tier": "extended"})
    withheld_by_b: str | None = Field(None, json_schema_extra={"tier": "extended"})
    resolution_constraint: str | None = Field(None, json_schema_extra={"tier": "extended"})
    scene_register: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ShadowPairing(BaseModel):
    """The inverted or corrupted form of the archetype dynamic."""

    description: str = Field(..., json_schema_extra={"tier": "extended"})
    inversion_type: (
        Literal["power_inversion", "moral_collapse", "agency_gain", "knowledge_refusal"] | None
    ) = Field(None, json_schema_extra={"tier": "extended"})
    drift_target_genre: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ArchetypeDynamic(BaseModel):
    """Per-genre archetype dynamic model capturing relational forces between archetype pairs."""

    pairing_name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    archetype_a: str = Field(..., json_schema_extra={"tier": "core"})
    archetype_b: str = Field(..., json_schema_extra={"tier": "core"})
    edge_properties: EdgeProperties = Field(..., json_schema_extra={"tier": "core"})
    characteristic_scene: CharacteristicScene | None = Field(None, json_schema_extra={"tier": "extended"})
    shadow_pairing: ShadowPairing | None = Field(None, json_schema_extra={"tier": "extended"})
    scale_properties: ScaleManifestations | None = Field(None, json_schema_extra={"tier": "extended"})
    uniqueness: UniquenessLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterArchetypeDynamic(BaseModel):
    """Cluster-level archetype dynamic capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    uniqueness: UniquenessLiteral = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ArchetypeDynamic",
    "CharacteristicScene",
    "ClusterArchetypeDynamic",
    "EdgeProperties",
    "ShadowPairing",
]
