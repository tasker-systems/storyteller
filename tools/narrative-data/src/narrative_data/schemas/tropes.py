"""Trope schemas — per-genre models for genre-specific narrative tropes.

Tropes are the recognizable narrative patterns that emerge from a genre's dimensional
properties. They carry specific narrative functions, admit multiple variant forms
(straight, inverted, deconstructed, violation), and signal cross-genre overlaps.
Tropes have no cluster variant — they are inherently genre-specific.
"""

from typing import Literal

from pydantic import BaseModel

from narrative_data.schemas.shared import OverlapSignal, StateVariableInteraction

NarrativeFunctionLiteral = Literal[
    "establishing", "connecting", "escalating", "characterizing", "resolving", "subverting"
]


class TropeVariants(BaseModel):
    """The straight, inverted, deconstructed, and violation forms of a trope."""

    straight: str | None = None
    inverted: str | None = None
    deconstructed: str | None = None
    violation: str | None = None


class Trope(BaseModel):
    """Per-genre trope model capturing genre-specific narrative patterns."""

    name: str
    genre_slug: str
    genre_derivation: str
    narrative_function: list[NarrativeFunctionLiteral]
    variants: TropeVariants | None = None
    state_variable_interactions: list[StateVariableInteraction] = []
    ontological_dimension: str | None = None
    overlap_signal: list[OverlapSignal] = []
    flavor_text: str | None = None


__all__ = [
    "Trope",
    "TropeVariants",
]
