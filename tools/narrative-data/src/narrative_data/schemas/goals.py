"""Goal schemas — per-genre and cluster models for character motivational patterns.

Goals are the core motivational patterns that drive characters and plots. They operate
at different narrative scales and can create cross-scale tensions when scene-level goals
conflict with existential ones.
"""

from typing import Literal

from pydantic import BaseModel

from narrative_data.schemas.shared import GenreVariant, StateVariableInteraction

ScaleLiteral = Literal["existential", "arc", "scene", "cross_scale"]
UniquenessLiteral = Literal["universal", "cluster_specific", "genre_unique"]
CrossScaleTensionTypeLiteral = Literal[
    "scene_costs_existential",
    "arc_requires_existential_betrayal",
    "mutual_undermining",
]


class CrossScaleTension(BaseModel):
    """A tension between goals operating at different narrative scales."""

    tension_type: CrossScaleTensionTypeLiteral
    description: str | None = None


class Goal(BaseModel):
    """Per-genre goal model capturing motivational patterns and cross-scale tensions."""

    canonical_name: str
    genre_slug: str
    variant_name: str
    scale: ScaleLiteral
    description: str | None = None
    cross_scale_tension: CrossScaleTension | None = None
    archetype_refs: list[str] = []
    state_variable_interactions: list[StateVariableInteraction] = []
    genre_variants: list[GenreVariant] = []
    uniqueness: UniquenessLiteral | None = None
    flavor_text: str | None = None


class ClusterGoal(BaseModel):
    """Cluster-level goal capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    uniqueness: UniquenessLiteral
    flavor_text: str | None = None


__all__ = [
    "ClusterGoal",
    "CrossScaleTension",
    "Goal",
]
