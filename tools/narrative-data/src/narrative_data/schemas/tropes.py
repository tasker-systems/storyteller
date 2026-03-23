# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Trope schemas — per-genre models for genre-specific narrative tropes.

Tropes are the recognizable narrative patterns that emerge from a genre's dimensional
properties. They carry specific narrative functions, admit multiple variant forms
(straight, inverted, deconstructed, violation), and signal cross-genre overlaps.
Tropes have no cluster variant — they are inherently genre-specific.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.shared import OverlapSignal, StateVariableInteraction

NarrativeFunctionLiteral = Literal[
    "establishing", "connecting", "escalating", "characterizing", "resolving", "subverting"
]


class TropeVariants(BaseModel):
    """The straight, inverted, deconstructed, and violation forms of a trope."""

    straight: str | None = Field(None, json_schema_extra={"tier": "extended"})
    inverted: str | None = Field(None, json_schema_extra={"tier": "extended"})
    deconstructed: str | None = Field(None, json_schema_extra={"tier": "extended"})
    violation: str | None = Field(None, json_schema_extra={"tier": "extended"})


class Trope(BaseModel):
    """Per-genre trope model capturing genre-specific narrative patterns."""

    name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    genre_derivation: str = Field(..., json_schema_extra={"tier": "core"})
    narrative_function: list[NarrativeFunctionLiteral] = Field(..., json_schema_extra={"tier": "core"})
    variants: TropeVariants | None = Field(None, json_schema_extra={"tier": "extended"})
    state_variable_interactions: list[StateVariableInteraction] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    ontological_dimension: str | None = Field(None, json_schema_extra={"tier": "extended"})
    overlap_signal: list[OverlapSignal] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "Trope",
    "TropeVariants",
]
