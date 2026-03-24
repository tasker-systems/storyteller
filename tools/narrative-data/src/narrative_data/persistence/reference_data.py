# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Reference data extraction for ground-state loader.

Extracts canonical reference data from the narrative corpus into
loader-consumable dicts. Covers genres, cluster metadata, state
variables, and the universal dimension inventory.
"""

import json
from pathlib import Path

from narrative_data.config import GENRE_CLUSTERS


def extract_genres(corpus_dir: Path) -> list[dict]:
    """Walk corpus_dir/genres/*/region.json and return genre dicts.

    Each returned dict has:
    - slug: genre directory name (e.g. "folk-horror")
    - name: genre_name field from region.json
    - description: flavor_text if present, else None
    - payload: full parsed region.json content
    """
    genres: list[dict] = []
    genres_dir = corpus_dir / "genres"
    if not genres_dir.exists():
        return genres

    for genre_dir in sorted(genres_dir.iterdir()):
        if not genre_dir.is_dir():
            continue
        region_file = genre_dir / "region.json"
        if not region_file.exists():
            continue

        payload = json.loads(region_file.read_text())
        slug = genre_dir.name
        name = payload.get("genre_name", slug)
        description = payload.get("flavor_text") or None

        genres.append(
            {
                "slug": slug,
                "name": name,
                "description": description,
                "payload": payload,
            }
        )

    return genres


def extract_cluster_metadata() -> list[dict]:
    """Return the 6 genre clusters from GENRE_CLUSTERS config.

    Each returned dict has:
    - slug: cluster key with "cluster-" prefix (e.g. "cluster-fantasy")
    - name: human-readable cluster name (e.g. "Fantasy")
    - genres: list of member genre slugs
    """
    clusters: list[dict] = []
    for cluster_key, genre_slugs in GENRE_CLUSTERS.items():
        # Derive human-readable name from the key (e.g. "mystery-thriller" → "Mystery-Thriller")
        name = " ".join(word.capitalize() for word in cluster_key.split("-"))
        clusters.append(
            {
                "slug": f"cluster-{cluster_key}",
                "name": name,
                "genres": list(genre_slugs),
            }
        )
    return clusters


def extract_state_variables(corpus_dir: Path) -> list[dict]:
    """Scan active_state_variables across all region.json files.

    Deduplicates by canonical_id. The first occurrence of each
    canonical_id is returned; later occurrences are skipped.

    Each returned dict has:
    - slug: canonical_id value
    - name: genre_label from first occurrence
    - description: threshold_effect if present, else activation_condition
    - default_range: dict with initial_value and threshold (may be None)
    - payload: full state variable dict from first occurrence
    """
    seen: dict[str, dict] = {}
    genres_dir = corpus_dir / "genres"
    if not genres_dir.exists():
        return []

    for genre_dir in sorted(genres_dir.iterdir()):
        if not genre_dir.is_dir():
            continue
        region_file = genre_dir / "region.json"
        if not region_file.exists():
            continue

        payload = json.loads(region_file.read_text())
        for sv in payload.get("active_state_variables", []):
            canonical_id = sv.get("canonical_id")
            if not canonical_id or canonical_id in seen:
                continue

            description = sv.get("threshold_effect") or sv.get("activation_condition") or None
            default_range: dict | None = None
            if sv.get("initial_value") is not None or sv.get("threshold") is not None:
                default_range = {
                    "initial_value": sv.get("initial_value"),
                    "threshold": sv.get("threshold"),
                }

            seen[canonical_id] = {
                "slug": canonical_id,
                "name": sv.get("genre_label", canonical_id),
                "description": description,
                "default_range": default_range,
                "payload": sv,
            }

    return list(seen.values())


# ---------------------------------------------------------------------------
# Canonical dimension inventory
# ---------------------------------------------------------------------------
#
# 34 universal dimensions across 7 groups, derived from the comprehensive
# terrain analysis (2026-03-21) and the GenreDimensions schema.
# Groups: aesthetic, tonal, temporal, thematic, agency, world, epistemological

_DIMENSIONS: list[dict] = [
    # --- aesthetic (4) ---
    {
        "slug": "sensory_density",
        "name": "Sensory Density",
        "dimension_group": "aesthetic",
        "description": "How richly the prose evokes the physical senses; sparse vs. lush/ornate.",
    },
    {
        "slug": "groundedness",
        "name": "Groundedness",
        "dimension_group": "aesthetic",
        "description": "How mundane vs. heightened the world's physical reality feels.",
    },
    {
        "slug": "aesthetic_register",
        "name": "Aesthetic Register",
        "dimension_group": "aesthetic",
        "description": "Overall stylistic register from gritty realism to highly stylized.",
    },
    {
        "slug": "prose_register",
        "name": "Prose Register",
        "dimension_group": "aesthetic",
        "description": "Textual style markers characteristic of the genre (list of tags).",
    },
    # --- tonal (5) ---
    {
        "slug": "emotional_contract",
        "name": "Emotional Contract",
        "dimension_group": "tonal",
        "description": "The primary emotional promise to the reader — comfort vs. dread.",
    },
    {
        "slug": "cynicism_earnestness",
        "name": "Cynicism / Earnestness",
        "dimension_group": "tonal",
        "description": "How sincere vs. ironic the narrative stance toward its own content is.",
    },
    {
        "slug": "surface_irony",
        "name": "Surface Irony",
        "dimension_group": "tonal",
        "description": "Degree of ironic distance in scene-level presentation.",
    },
    {
        "slug": "structural_irony",
        "name": "Structural Irony",
        "dimension_group": "tonal",
        "description": "Irony embedded in plot or character arc, not just surface tone.",
    },
    {
        "slug": "intimacy_distance",
        "name": "Intimacy / Distance",
        "dimension_group": "tonal",
        "description": "How close the narrative voice feels to characters and events.",
    },
    # --- temporal (4) ---
    {
        "slug": "time_structure",
        "name": "Time Structure",
        "dimension_group": "temporal",
        "description": "Linear vs. fragmented or circular temporal ordering of events.",
    },
    {
        "slug": "pacing",
        "name": "Pacing",
        "dimension_group": "temporal",
        "description": "Narrative speed — slow and deliberate vs. propulsive.",
    },
    {
        "slug": "temporal_grounding",
        "name": "Temporal Grounding",
        "dimension_group": "temporal",
        "description": "How anchored the story is to a specific historical moment.",
    },
    {
        "slug": "narrative_span",
        "name": "Narrative Span",
        "dimension_group": "temporal",
        "description": "Compressed single event vs. sweeping generational or epic timescale.",
    },
    # --- thematic (4) ---
    {
        "slug": "power_treatment",
        "name": "Power Treatment",
        "dimension_group": "thematic",
        "description": "Weighted tags for how the genre engages with power dynamics.",
    },
    {
        "slug": "identity_treatment",
        "name": "Identity Treatment",
        "dimension_group": "thematic",
        "description": "Weighted tags for how the genre engages with identity and self.",
    },
    {
        "slug": "knowledge_treatment",
        "name": "Knowledge Treatment",
        "dimension_group": "thematic",
        "description": "Weighted tags for how the genre frames knowing and not-knowing.",
    },
    {
        "slug": "connection_treatment",
        "name": "Connection Treatment",
        "dimension_group": "thematic",
        "description": "Weighted tags for how the genre treats bonds between characters.",
    },
    # --- agency (4) ---
    {
        "slug": "agency_level",
        "name": "Agency Level",
        "dimension_group": "agency",
        "description": "How much effective choice protagonists have over their situation.",
    },
    {
        "slug": "agency_type",
        "name": "Agency Type",
        "dimension_group": "agency",
        "description": "Mode of agency: imposition, acceptance, negotiation, sacrifice, survival.",
    },
    {
        "slug": "triumph_mode",
        "name": "Triumph Mode",
        "dimension_group": "agency",
        "description": "How victory or resolution is achieved — pyrrhic vs. clean triumph.",
    },
    {
        "slug": "competence_relevance",
        "name": "Competence Relevance",
        "dimension_group": "agency",
        "description": "How much skill and preparation actually determines outcomes.",
    },
    # --- world (5) ---
    {
        "slug": "world_magic",
        "name": "Magic",
        "dimension_group": "world",
        "description": "What forms of magic the world permits and how they are constrained.",
    },
    {
        "slug": "world_technology",
        "name": "Technology",
        "dimension_group": "world",
        "description": "Technological affordances and constraints of the world.",
    },
    {
        "slug": "world_violence",
        "name": "Violence",
        "dimension_group": "world",
        "description": "How violence is depicted, permitted, and consequential.",
    },
    {
        "slug": "world_death",
        "name": "Death",
        "dimension_group": "world",
        "description": "The nature, permanence, and meaning of death in this world.",
    },
    {
        "slug": "world_supernatural",
        "name": "Supernatural",
        "dimension_group": "world",
        "description": "Presence and rules governing supernatural elements.",
    },
    # --- epistemological (3) ---
    {
        "slug": "knowability",
        "name": "Knowability",
        "dimension_group": "epistemological",
        "description": "Whether the world's secrets can ultimately be understood.",
    },
    {
        "slug": "knowledge_reward",
        "name": "Knowledge Reward",
        "dimension_group": "epistemological",
        "description": "Whether seeking knowledge leads to safety or danger.",
    },
    {
        "slug": "narration_reliability",
        "name": "Narration Reliability",
        "dimension_group": "epistemological",
        "description": "How trustworthy the narrator's account of events is.",
    },
    # --- structural (5) ---
    {
        "slug": "locus_of_power",
        "name": "Locus of Power",
        "dimension_group": "structural",
        "description": "Where power resides: place, person, system, relationship, or cosmos.",
    },
    {
        "slug": "narrative_structure",
        "name": "Narrative Structure",
        "dimension_group": "structural",
        "description": "Dominant narrative shapes: quest, mystery, tragedy, comedy, romance, horror.",
    },
    {
        "slug": "narrative_contracts",
        "name": "Narrative Contracts",
        "dimension_group": "structural",
        "description": "Hard guarantees the genre makes to the reader.",
    },
    {
        "slug": "genre_boundaries",
        "name": "Genre Boundaries",
        "dimension_group": "structural",
        "description": "Triggers that signal genre drift toward adjacent genres.",
    },
    {
        "slug": "modifies",
        "name": "Modifies",
        "dimension_group": "structural",
        "description": "Genre slugs this constraint layer can modify (for modifier genres).",
    },
]


def extract_dimensions() -> list[dict]:
    """Return the canonical universal dimension inventory.

    Returns a list of dicts, each with:
    - slug: machine identifier
    - name: human-readable name
    - dimension_group: one of aesthetic, tonal, temporal, thematic,
      agency, world, epistemological, structural
    - description: brief explanation of the dimension
    """
    return list(_DIMENSIONS)
