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

from pydantic import BaseModel

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

    tension_signature: TensionSignatureLiteral | None = None
    emotional_register: EmotionalRegisterLiteral | None = None
    pacing: PacingLiteral | None = None
    cast_density: CastDensityLiteral | None = None
    physical_dynamism: PhysicalDynamismLiteral | None = None
    information_flow: InformationFlowLiteral | None = None
    resolution_tendency: ResolutionTendencyLiteral | None = None
    locus_of_power: str | None = None


class SceneProfile(BaseModel):
    """Per-genre scene profile capturing the dimensional properties of a scene type."""

    name: str
    genre_slug: str
    core_identity: str
    dimensional_properties: SceneDimensionalProperties
    uniqueness: UniquenessLiteral | None = None
    provenance: list[str] = []
    flavor_text: str | None = None


class ClusterSceneProfile(BaseModel):
    """Cluster-level scene profile capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    uniqueness: UniquenessLiteral
    flavor_text: str | None = None


__all__ = [
    "ClusterSceneProfile",
    "SceneDimensionalProperties",
    "SceneProfile",
]
