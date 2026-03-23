# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Scene profile schemas — per-genre and cluster models for scene type profiles.

Scene profiles describe the dimensional properties of recurring scene types within a
genre. They capture the tension signature, emotional register, pacing, cast density,
physical dynamism, information flow, and resolution tendency that characterize a
scene type's distinct identity.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.shared import GenreVariant

TensionSignatureLiteral = Literal["ambient_sustained", "spiking_explosive", "oscillating"]
EmotionalRegisterLiteral = Literal[
    "restrained_intellectual", "visceral_overwhelming", "ironic", "earnest"
]
PacingLiteral = Literal["slow_atmospheric", "rapid_kinetic"]
CastDensityLiteral = Literal["intimate_few", "ensemble_crowded", "solitary"]
PhysicalDynamismLiteral = Literal["static_contained", "mobile_expansive", "kinetic"]
InformationFlowLiteral = Literal[
    "withholding_accumulating", "revealing_spending", "distorted_unreliable"
]
ResolutionTendencyLiteral = Literal[
    "closed_conclusive", "open_suspended", "negative", "probabilistic"
]
UniquenessLiteral = Literal["genre_unique", "cluster_wide", "likely_universal"]


class SceneDimensionalProperties(BaseModel):
    """The dimensional fingerprint of a scene type."""

    tension_signature: TensionSignatureLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    emotional_register: EmotionalRegisterLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    pacing: PacingLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    cast_density: CastDensityLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    physical_dynamism: PhysicalDynamismLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    information_flow: InformationFlowLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    resolution_tendency: ResolutionTendencyLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    locus_of_power: str | None = Field(None, json_schema_extra={"tier": "extended"})


class SceneProfile(BaseModel):
    """Per-genre scene profile capturing the dimensional properties of a scene type."""

    name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    dimensional_properties: SceneDimensionalProperties = Field(..., json_schema_extra={"tier": "core"})
    uniqueness: UniquenessLiteral | None = Field(None, json_schema_extra={"tier": "core"})
    provenance: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterSceneProfile(BaseModel):
    """Cluster-level scene profile capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    uniqueness: UniquenessLiteral = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ClusterSceneProfile",
    "SceneDimensionalProperties",
    "SceneProfile",
]
