# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Ontological posture schemas — per-genre and cluster models for genre worldview.

Ontological posture describes how a genre frames the nature of being, selfhood, and
ethical obligation. It answers: what kinds of entities exist here, how permeable are
the boundaries between self and other, and what moral rules govern action in this world?
"""

from typing import Literal

from pydantic import BaseModel, Field

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

    name: str = Field(..., json_schema_extra={"tier": "core"})
    description: str = Field(..., json_schema_extra={"tier": "core"})
    can_have_communicability: bool = Field(False, json_schema_extra={"tier": "core"})


class SelfOtherBoundary(BaseModel):
    """How permeable the boundary between self and other is in this genre."""

    stability: SelfOtherStabilityLiteral = Field(..., json_schema_extra={"tier": "core"})
    crossing_rules: str | None = Field(None, json_schema_extra={"tier": "extended"})
    obligations_across: str | None = Field(None, json_schema_extra={"tier": "extended"})


class EthicalOrientation(BaseModel):
    """What actions are permitted and forbidden in this genre's moral frame."""

    permitted: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    forbidden: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})


class OntologicalPosture(BaseModel):
    """Per-genre ontological posture capturing worldview and ethical frame."""

    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    default_subject: str = Field(..., json_schema_extra={"tier": "core"})
    modes_of_being: list[ModeOfBeing] = Field(..., json_schema_extra={"tier": "core"})
    self_other_boundary: SelfOtherBoundary = Field(..., json_schema_extra={"tier": "core"})
    ethical_orientation: EthicalOrientation = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterOntologicalPosture(BaseModel):
    """Cluster-level ontological posture capturing shared worldview across genres."""

    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ClusterOntologicalPosture",
    "EthicalOrientation",
    "ModeOfBeing",
    "OntologicalPosture",
    "SelfOtherBoundary",
]
