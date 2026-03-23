# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""GenreDimensions schema — the foundation schema for all narrative data.

34 dimensions organized into 8 groups plus meta-fields. Converts the genre
region markdown corpus into queryable, typed JSON.

Design note: `flavor_text` is a plain string (not the object form from the
JSON scaffolding), matching the task spec and consistent with other schemas.
The JSON schema scaffolding's object form (commentary/suggestions) is a
divergence noted in schema comparison.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.shared import (
    ContinuousAxis,
    GenreBoundary,
    StateVariableTemplate,
    WeightedTags,
)

# ---------------------------------------------------------------------------
# Dimensional sub-models
# ---------------------------------------------------------------------------

LocusOfPowerItem = Literal["place", "person", "system", "relationship", "cosmos"]
NarrativeStructureItem = Literal["quest", "mystery", "tragedy", "comedy", "romance", "horror"]
AgencyTypeItem = Literal["imposition", "acceptance", "negotiation", "sacrifice", "survival"]
ClassificationItem = Literal["standalone_region", "constraint_layer", "hybrid_modifier"]
ConstraintLayerTypeItem = Literal[
    "world_affordance", "tonal_emotional", "agency_outcome", "setting_locus"
]


class AestheticDimensions(BaseModel):
    """Sensory and stylistic character of the genre."""

    sensory_density: ContinuousAxis
    groundedness: ContinuousAxis
    aesthetic_register: ContinuousAxis
    prose_register: list[str] = Field(default_factory=list)


class TonalDimensions(BaseModel):
    """Emotional and ironic register of the genre."""

    emotional_contract: ContinuousAxis
    cynicism_earnestness: ContinuousAxis
    surface_irony: ContinuousAxis
    structural_irony: ContinuousAxis
    intimacy_distance: ContinuousAxis


class TemporalDimensions(BaseModel):
    """Time structure, pacing, and narrative span."""

    time_structure: ContinuousAxis
    pacing: ContinuousAxis
    temporal_grounding: ContinuousAxis
    narrative_span: ContinuousAxis


class ThematicDimensions(BaseModel):
    """Weighted tag sets for thematic content — not single enums."""

    power_treatment: WeightedTags
    identity_treatment: WeightedTags
    knowledge_treatment: WeightedTags
    connection_treatment: WeightedTags


class AgencyDimensions(BaseModel):
    """Protagonist agency level, type, and triumph mode."""

    agency_level: ContinuousAxis
    agency_type: AgencyTypeItem
    triumph_mode: ContinuousAxis
    competence_relevance: ContinuousAxis


class WorldAffordances(BaseModel):
    """What the world permits — magic, technology, violence, death, supernatural."""

    magic: list[str] = Field(default_factory=list)
    technology: str
    violence: str
    death: str
    supernatural: str


class EpistemologicalDimensions(BaseModel):
    """Knowability, knowledge reward, and narration reliability."""

    knowability: ContinuousAxis
    knowledge_reward: ContinuousAxis
    narration_reliability: ContinuousAxis


# ---------------------------------------------------------------------------
# Narrative contract
# ---------------------------------------------------------------------------


class NarrativeContract(BaseModel):
    """Hard guarantee the genre makes to the reader."""

    invariant: str
    enforced: bool


# ---------------------------------------------------------------------------
# Top-level GenreDimensions
# ---------------------------------------------------------------------------


class GenreDimensions(BaseModel):
    """Full dimensional profile for a single genre region.

    The foundation schema — everything else references genre dimensions.
    26 of 30 genres are constraint layers that modify other genres.
    """

    genre_slug: str
    genre_name: str
    classification: ClassificationItem
    constraint_layer_type: ConstraintLayerTypeItem | None = None

    aesthetic: AestheticDimensions
    tonal: TonalDimensions
    temporal: TemporalDimensions
    thematic: ThematicDimensions
    agency: AgencyDimensions

    locus_of_power: list[LocusOfPowerItem] = Field(
        default_factory=list,
        max_length=3,
        description="Ranked list, first = primary. Max 3.",
    )
    narrative_structure: list[NarrativeStructureItem] = Field(
        default_factory=list,
        max_length=3,
        description="Dominant narrative shapes. Max 3.",
    )

    world_affordances: WorldAffordances
    epistemological: EpistemologicalDimensions

    narrative_contracts: list[NarrativeContract] = Field(default_factory=list)
    active_state_variables: list[StateVariableTemplate] = Field(default_factory=list)
    boundaries: list[GenreBoundary] = Field(default_factory=list)
    modifies: list[str] = Field(
        default_factory=list,
        description="Genre slugs this constraint layer can modify.",
    )
    flavor_text: str | None = None
