"""Ontological posture schemas — per-genre and cluster models for genre worldview.

Ontological posture describes how a genre frames the nature of being, selfhood, and
ethical obligation. It answers: what kinds of entities exist here, how permeable are
the boundaries between self and other, and what moral rules govern action in this world?
"""

from typing import Literal

from pydantic import BaseModel

from narrative_data.schemas.shared import GenreVariant

SelfOtherStabilityLiteral = Literal[
    "firm_defensive",
    "permeable_negotiable",
    "permeable_toxic",
    "illusory_dissolving",
    "weaponized",
]


class ModeOfBeing(BaseModel):
    """A mode of existence available to entities in this genre."""

    name: str
    description: str
    can_have_communicability: bool = False


class SelfOtherBoundary(BaseModel):
    """How permeable the boundary between self and other is in this genre."""

    stability: SelfOtherStabilityLiteral
    crossing_rules: str | None = None
    obligations_across: str | None = None


class EthicalOrientation(BaseModel):
    """What actions are permitted and forbidden in this genre's moral frame."""

    permitted: list[str] = []
    forbidden: list[str] = []


class OntologicalPosture(BaseModel):
    """Per-genre ontological posture capturing worldview and ethical frame."""

    genre_slug: str
    default_subject: str
    modes_of_being: list[ModeOfBeing]
    self_other_boundary: SelfOtherBoundary
    ethical_orientation: EthicalOrientation
    flavor_text: str | None = None


class ClusterOntologicalPosture(BaseModel):
    """Cluster-level ontological posture capturing shared worldview across genres."""

    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    flavor_text: str | None = None


__all__ = [
    "ClusterOntologicalPosture",
    "EthicalOrientation",
    "ModeOfBeing",
    "OntologicalPosture",
    "SelfOtherBoundary",
]
