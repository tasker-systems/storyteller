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

    sensory_density: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    groundedness: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    aesthetic_register: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    prose_register: list[str] = Field(default_factory=list, json_schema_extra={"tier": "extended"})


class TonalDimensions(BaseModel):
    """Emotional and ironic register of the genre."""

    emotional_contract: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    cynicism_earnestness: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    surface_irony: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    structural_irony: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    intimacy_distance: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})


class TemporalDimensions(BaseModel):
    """Time structure, pacing, and narrative span."""

    time_structure: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    pacing: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    temporal_grounding: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    narrative_span: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})


class ThematicDimensions(BaseModel):
    """Weighted tag sets for thematic content — not single enums."""

    power_treatment: WeightedTags = Field(..., json_schema_extra={"tier": "core"})
    identity_treatment: WeightedTags = Field(..., json_schema_extra={"tier": "core"})
    knowledge_treatment: WeightedTags = Field(..., json_schema_extra={"tier": "core"})
    connection_treatment: WeightedTags = Field(..., json_schema_extra={"tier": "core"})


class AgencyDimensions(BaseModel):
    """Protagonist agency level, type, and triumph mode."""

    agency_level: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    agency_type: AgencyTypeItem = Field(..., json_schema_extra={"tier": "core"})
    triumph_mode: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    competence_relevance: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})


class WorldAffordances(BaseModel):
    """What the world permits — magic, technology, violence, death, supernatural."""

    magic: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    technology: str = Field(..., json_schema_extra={"tier": "core"})
    violence: str = Field(..., json_schema_extra={"tier": "core"})
    death: str = Field(..., json_schema_extra={"tier": "core"})
    supernatural: str = Field(..., json_schema_extra={"tier": "core"})


class EpistemologicalDimensions(BaseModel):
    """Knowability, knowledge reward, and narration reliability."""

    knowability: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    knowledge_reward: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})
    narration_reliability: ContinuousAxis = Field(..., json_schema_extra={"tier": "core"})


# ---------------------------------------------------------------------------
# Narrative contract
# ---------------------------------------------------------------------------


class NarrativeContract(BaseModel):
    """Hard guarantee the genre makes to the reader."""

    invariant: str = Field(..., json_schema_extra={"tier": "core"})
    enforced: bool = Field(..., json_schema_extra={"tier": "core"})


# ---------------------------------------------------------------------------
# Top-level GenreDimensions
# ---------------------------------------------------------------------------


class GenreDimensions(BaseModel):
    """Full dimensional profile for a single genre region.

    The foundation schema — everything else references genre dimensions.
    26 of 30 genres are constraint layers that modify other genres.
    """

    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    genre_name: str = Field(..., json_schema_extra={"tier": "core"})
    classification: ClassificationItem = Field(..., json_schema_extra={"tier": "core"})
    constraint_layer_type: ConstraintLayerTypeItem | None = Field(None, json_schema_extra={"tier": "core"})

    aesthetic: AestheticDimensions = Field(..., json_schema_extra={"tier": "core"})
    tonal: TonalDimensions = Field(..., json_schema_extra={"tier": "core"})
    temporal: TemporalDimensions = Field(..., json_schema_extra={"tier": "core"})
    thematic: ThematicDimensions = Field(..., json_schema_extra={"tier": "core"})
    agency: AgencyDimensions = Field(..., json_schema_extra={"tier": "core"})

    locus_of_power: list[LocusOfPowerItem] = Field(
        default_factory=list,
        max_length=3,
        description="Ranked list, first = primary. Max 3.",
        json_schema_extra={"tier": "core"},
    )
    narrative_structure: list[NarrativeStructureItem] = Field(
        default_factory=list,
        max_length=3,
        description="Dominant narrative shapes. Max 3.",
        json_schema_extra={"tier": "core"},
    )

    world_affordances: WorldAffordances = Field(..., json_schema_extra={"tier": "core"})
    epistemological: EpistemologicalDimensions = Field(..., json_schema_extra={"tier": "core"})

    narrative_contracts: list[NarrativeContract] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    active_state_variables: list[StateVariableTemplate] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    boundaries: list[GenreBoundary] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    modifies: list[str] = Field(
        default_factory=list,
        description="Genre slugs this constraint layer can modify.",
        json_schema_extra={"tier": "core"},
    )
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})
