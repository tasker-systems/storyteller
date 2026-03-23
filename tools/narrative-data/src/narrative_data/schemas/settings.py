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

from pydantic import BaseModel

from narrative_data.schemas.shared import GenreVariant, OverlapSignal


class SettingCommunicability(BaseModel):
    """How a setting communicates across four channels."""

    atmospheric: str | None = None
    sensory: str | None = None
    spatial: str | None = None
    temporal: str | None = None


class Settings(BaseModel):
    """Per-genre setting model capturing place as narrative entity."""

    canonical_name: str
    genre_slug: str
    variant_name: str
    atmospheric_palette: list[str] = []
    sensory_vocabulary: list[str] = []
    narrative_function: str | None = None
    communicability: SettingCommunicability | None = None
    overlap_signals: list[OverlapSignal] = []
    flavor_text: str | None = None


class ClusterSettings(BaseModel):
    """Cluster-level setting capturing canonical identity and genre variants."""

    canonical_name: str
    cluster_name: str
    core_identity: str
    genre_variants: list[GenreVariant]
    uniqueness: Literal["universal", "cluster_specific", "genre_unique"]
    flavor_text: str | None = None


__all__ = [
    "ClusterSettings",
    "SettingCommunicability",
    "Settings",
]
