# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Settings schemas — per-genre and cluster models for narrative settings.

Settings are the places where narrative action unfolds. They are not merely backdrops
but active participants — communicating mood, constraining action, and shaping what
kinds of events are possible. Communicability dimensions capture how a setting
expresses itself across atmospheric, sensory, spatial, and temporal channels.
"""

from typing import Literal

from pydantic import BaseModel, Field

from narrative_data.schemas.shared import GenreVariant, OverlapSignal


class SettingCommunicability(BaseModel):
    """How a setting communicates across four channels."""

    atmospheric: str | None = Field(None, json_schema_extra={"tier": "core"})
    sensory: str | None = Field(None, json_schema_extra={"tier": "core"})
    spatial: str | None = Field(None, json_schema_extra={"tier": "core"})
    temporal: str | None = Field(None, json_schema_extra={"tier": "core"})


class Settings(BaseModel):
    """Per-genre setting model capturing place as narrative entity."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    genre_slug: str = Field(..., json_schema_extra={"tier": "core"})
    variant_name: str = Field(..., json_schema_extra={"tier": "core"})
    atmospheric_palette: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    sensory_vocabulary: list[str] = Field(default_factory=list, json_schema_extra={"tier": "core"})
    narrative_function: str | None = Field(None, json_schema_extra={"tier": "core"})
    communicability: SettingCommunicability | None = Field(None, json_schema_extra={"tier": "core"})
    overlap_signals: list[OverlapSignal] = Field(default_factory=list, json_schema_extra={"tier": "extended"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


class ClusterSettings(BaseModel):
    """Cluster-level setting capturing canonical identity and genre variants."""

    canonical_name: str = Field(..., json_schema_extra={"tier": "core"})
    cluster_name: str = Field(..., json_schema_extra={"tier": "core"})
    core_identity: str = Field(..., json_schema_extra={"tier": "core"})
    genre_variants: list[GenreVariant] = Field(..., json_schema_extra={"tier": "extended"})
    uniqueness: Literal["universal", "cluster_specific", "genre_unique"] = Field(..., json_schema_extra={"tier": "core"})
    flavor_text: str | None = Field(None, json_schema_extra={"tier": "extended"})


__all__ = [
    "ClusterSettings",
    "SettingCommunicability",
    "Settings",
]
