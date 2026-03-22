"""Archetype dynamics schemas — per-genre and cluster models for archetype pair relationships.

Archetype dynamics describe the structured relational forces between specific archetype
pairings within a genre. Unlike general dynamics (which operate between any entities),
archetype dynamics capture the canonical scene types, edge properties, and shadow inversions
specific to archetypal role relationships.
"""

from typing import Literal

from pydantic import BaseModel

from narrative_data.schemas.dynamics import ScaleManifestations
from narrative_data.schemas.shared import GenreVariant

UniquenessLiteral = Literal["genre_unique", "cluster_specific", "genre_defining"]


class EdgeProperties(BaseModel):
    """Properties of the relational edge between two archetypes."""

    edge_type: str
    directionality: str
    currencies: list[str] = []
    network_position: str | None = None
    constraints: list[str] = []
    weight: str | None = None


class CharacteristicScene(BaseModel):
    """A canonical scene that exemplifies the archetype dynamic."""

    title: str
    opening: str | None = None
    tension_source: str | None = None
    withheld_by_a: str | None = None
    withheld_by_b: str | None = None
    resolution_constraint: str | None = None
    scene_register: str | None = None


class ShadowPairing(BaseModel):
    """The inverted or corrupted form of the archetype dynamic."""

    description: str
    inversion_type: (
        Literal["power_inversion", "moral_collapse", "agency_gain", "knowledge_refusal"] | None
    ) = None
    drift_target_genre: str | None = None


class ArchetypeDynamic(BaseModel):
    """Per-genre archetype dynamic model capturing relational forces between archetype pairs."""

    pairing_name: str
    genre_slug: str
    archetype_a: str
    archetype_b: str
    edge_properties: EdgeProperties
    characteristic_scene: CharacteristicScene | None = None
    shadow_pairing: ShadowPairing | None = None
    scale_properties: ScaleManifestations | None = None
    uniqueness: UniquenessLiteral | None = None
    genre_variants: list[GenreVariant] = []
    flavor_text: str | None = None


class ClusterArchetypeDynamic(BaseModel):
    """Cluster-level archetype dynamic capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    uniqueness: UniquenessLiteral
    flavor_text: str | None = None


__all__ = [
    "ArchetypeDynamic",
    "CharacteristicScene",
    "ClusterArchetypeDynamic",
    "EdgeProperties",
    "ShadowPairing",
]
