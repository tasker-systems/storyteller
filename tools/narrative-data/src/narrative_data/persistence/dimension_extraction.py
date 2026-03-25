# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Dimension extraction: walks primitive entity payloads and extracts typed dimensional values.

Produces rows for ``bedrock.dimension_values`` — one row per entity-dimension pair —
so that dimensional data becomes queryable and composable without JSONB path traversal.
"""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from typing import Any
from uuid import UUID

import psycopg

log = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# DimensionRule
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class DimensionRule:
    """Describes how to extract a single dimension from an entity payload.

    Attributes:
        source_path: Dot-separated JSONB path (e.g. ``personality_profile.warmth``).
        dimension_slug: Canonical slug for the dimension
            (maps to ``dimension_values.dimension_slug``).
        dimension_group: Grouping key (``personality``, ``scene_dimensions``, ``relational``, etc.).
        value_type: One of ``normalized``, ``bipolar``, ``categorical``, ``weighted_tags``, ``set``.
        tier: Extraction tier — ``core`` by default.
    """

    source_path: str
    dimension_slug: str
    dimension_group: str
    value_type: str
    tier: str = "core"


# ---------------------------------------------------------------------------
# Per-type extraction rule registries
# ---------------------------------------------------------------------------

ARCHETYPE_RULES: list[DimensionRule] = [
    DimensionRule("personality_profile.warmth", "warmth", "personality", "normalized"),
    DimensionRule("personality_profile.authority", "authority", "personality", "normalized"),
    DimensionRule("personality_profile.openness", "openness", "personality", "normalized"),
    DimensionRule("personality_profile.interiority", "interiority", "personality", "normalized"),
    DimensionRule("personality_profile.stability", "stability", "personality", "normalized"),
    DimensionRule("personality_profile.agency", "agency", "personality", "normalized"),
    DimensionRule("personality_profile.morality", "morality", "personality", "normalized"),
]

PROFILE_RULES: list[DimensionRule] = [
    DimensionRule(
        "dimensional_properties.tension_signature",
        "tension_signature",
        "scene_dimensions",
        "categorical",
    ),
    DimensionRule(
        "dimensional_properties.emotional_register",
        "emotional_register",
        "scene_dimensions",
        "categorical",
    ),
    DimensionRule("dimensional_properties.pacing", "pacing", "scene_dimensions", "categorical"),
    DimensionRule(
        "dimensional_properties.cast_density", "cast_density", "scene_dimensions", "categorical"
    ),
    DimensionRule(
        "dimensional_properties.physical_dynamism",
        "physical_dynamism",
        "scene_dimensions",
        "categorical",
    ),
    DimensionRule(
        "dimensional_properties.information_flow",
        "information_flow",
        "scene_dimensions",
        "categorical",
    ),
    DimensionRule(
        "dimensional_properties.resolution_tendency",
        "resolution_tendency",
        "scene_dimensions",
        "categorical",
    ),
]

DYNAMICS_RULES: list[DimensionRule] = [
    DimensionRule("edge_type", "edge_type", "relational", "categorical"),
    DimensionRule("directionality", "directionality", "relational", "categorical"),
    DimensionRule("valence", "valence", "relational", "categorical"),
    DimensionRule("evolution_pattern", "evolution_pattern", "relational", "categorical"),
    DimensionRule("currencies", "currencies", "relational", "set"),
    DimensionRule("network_position", "network_position", "relational", "categorical"),
]

PLACE_ENTITY_RULES: list[DimensionRule] = [
    DimensionRule(
        "communicability.atmospheric.mood", "atmospheric_mood", "communicability", "categorical"
    ),
    DimensionRule(
        "communicability.atmospheric.intensity",
        "atmospheric_intensity",
        "communicability",
        "normalized",
    ),
    DimensionRule(
        "communicability.sensory.dominant", "sensory_dominant", "communicability", "categorical"
    ),
    DimensionRule(
        "communicability.spatial.enclosure", "spatial_enclosure", "communicability", "categorical"
    ),
    DimensionRule(
        "communicability.spatial.orientation",
        "spatial_orientation",
        "communicability",
        "categorical",
    ),
    DimensionRule(
        "communicability.temporal.time_model",
        "temporal_time_model",
        "communicability",
        "categorical",
    ),
]

SPATIAL_TOPOLOGY_RULES: list[DimensionRule] = [
    DimensionRule("friction.type", "friction_type", "spatial", "categorical"),
    DimensionRule("friction.level", "friction_level", "spatial", "categorical"),
    DimensionRule("directionality.type", "directionality_type", "spatial", "categorical"),
    DimensionRule("agency", "agency", "spatial", "categorical"),
]

RULE_REGISTRY: dict[str, list[DimensionRule]] = {
    "archetypes": ARCHETYPE_RULES,
    "profiles": PROFILE_RULES,
    "dynamics": DYNAMICS_RULES,
    "place_entities": PLACE_ENTITY_RULES,
    "spatial_topology": SPATIAL_TOPOLOGY_RULES,
}


# ---------------------------------------------------------------------------
# Payload traversal
# ---------------------------------------------------------------------------


def _walk_path(payload: dict, path: str) -> Any:
    """Walk a dot-separated path into a nested dict, returning the value or None."""
    current: Any = payload
    for segment in path.split("."):
        if not isinstance(current, dict):
            return None
        current = current.get(segment)
        if current is None:
            return None
    return current


# ---------------------------------------------------------------------------
# Value coercion helpers
# ---------------------------------------------------------------------------

_SENTINEL_VALUES = frozenset({"null", "None", "none", "N/A", "n/a", "Null.", "NULL", ""})


def _is_valid(val: Any) -> bool:
    """Check if a value is non-null and not a sentinel string."""
    if val is None:
        return False
    return not (isinstance(val, str) and val.strip() in _SENTINEL_VALUES)


def _coerce_row(
    rule: DimensionRule,
    raw_value: Any,
    primitive_table: str,
    primitive_id: UUID,
    genre_id: UUID,
) -> dict | None:
    """Convert a raw extracted value into a dimension_values row dict.

    Returns None if the value is empty or invalid.
    """
    if not _is_valid(raw_value):
        return None

    numeric_value: float | None = None
    categorical_value: str | None = None
    complex_value: Any | None = None

    vtype = rule.value_type

    if vtype in ("normalized", "bipolar"):
        try:
            numeric_value = float(raw_value)
        except (TypeError, ValueError):
            return None

    elif vtype == "categorical":
        if isinstance(raw_value, str):
            categorical_value = raw_value.strip()
            if not categorical_value:
                return None
        else:
            # Coerce non-string scalars (int, bool, etc.)
            categorical_value = str(raw_value)

    elif vtype in ("set", "weighted_tags"):
        if isinstance(raw_value, (list, dict)):
            if not raw_value:
                return None
            complex_value = raw_value
        elif isinstance(raw_value, str):
            # Try to parse as JSON
            try:
                parsed = json.loads(raw_value)
                if not parsed:
                    return None
                complex_value = parsed
            except (json.JSONDecodeError, TypeError):
                complex_value = [raw_value]
        else:
            return None

    else:
        log.warning("Unknown value_type %r for %s — skipping", vtype, rule.dimension_slug)
        return None

    return {
        "primitive_table": primitive_table,
        "primitive_id": primitive_id,
        "genre_id": genre_id,
        "dimension_slug": rule.dimension_slug,
        "dimension_group": rule.dimension_group,
        "value_type": vtype,
        "numeric_value": numeric_value,
        "categorical_value": categorical_value,
        "complex_value": complex_value,
        "source_path": rule.source_path,
        "tier": rule.tier,
    }


# ---------------------------------------------------------------------------
# Genre-dimensions extraction (special: one row per genre, 34 dimensions)
# ---------------------------------------------------------------------------


def _extract_genre_dimensions(
    primitive_id: UUID,
    genre_id: UUID,
    payload: dict,
) -> list[dict]:
    """Extract all dimensions from a genre_dimensions payload.

    The payload is structured as groups (``aesthetic``, ``tonal``, ``temporal``, etc.)
    containing named dimensions with numeric or categorical values. Each becomes its
    own ``dimension_values`` row.
    """
    rows: list[dict] = []
    for group_name, group_value in payload.items():
        if not isinstance(group_value, dict):
            continue
        for dim_name, dim_value in group_value.items():
            if not _is_valid(dim_value):
                continue

            source_path = f"{group_name}.{dim_name}"
            dimension_slug = f"{group_name}_{dim_name}"

            # Determine value_type from Python type
            if isinstance(dim_value, (int, float)):
                rows.append(
                    {
                        "primitive_table": "genre_dimensions",
                        "primitive_id": primitive_id,
                        "genre_id": genre_id,
                        "dimension_slug": dimension_slug,
                        "dimension_group": group_name,
                        "value_type": "normalized",
                        "numeric_value": float(dim_value),
                        "categorical_value": None,
                        "complex_value": None,
                        "source_path": source_path,
                        "tier": "core",
                    }
                )
            elif isinstance(dim_value, str):
                rows.append(
                    {
                        "primitive_table": "genre_dimensions",
                        "primitive_id": primitive_id,
                        "genre_id": genre_id,
                        "dimension_slug": dimension_slug,
                        "dimension_group": group_name,
                        "value_type": "categorical",
                        "numeric_value": None,
                        "categorical_value": dim_value,
                        "complex_value": None,
                        "source_path": source_path,
                        "tier": "core",
                    }
                )
            elif isinstance(dim_value, (list, dict)) and dim_value:
                vtype = "set" if isinstance(dim_value, list) else "weighted_tags"
                rows.append(
                    {
                        "primitive_table": "genre_dimensions",
                        "primitive_id": primitive_id,
                        "genre_id": genre_id,
                        "dimension_slug": dimension_slug,
                        "dimension_group": group_name,
                        "value_type": vtype,
                        "numeric_value": None,
                        "categorical_value": None,
                        "complex_value": dim_value,
                        "source_path": source_path,
                        "tier": "core",
                    }
                )
    return rows


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def extract_dimensions(
    primitive_table: str,
    entity_id: UUID,
    genre_id: UUID,
    payload: dict,
) -> list[dict]:
    """Extract typed dimensional values from an entity payload.

    Looks up extraction rules for the given ``primitive_table``, walks the payload
    using dot-separated paths, and returns a list of dicts ready for insertion into
    ``bedrock.dimension_values``.

    For ``genre_dimensions``, uses the special genre-dimensions extractor that
    walks all groups and dimensions dynamically.
    """
    if not isinstance(payload, dict):
        return []

    if primitive_table == "genre_dimensions":
        return _extract_genre_dimensions(entity_id, genre_id, payload)

    rules = RULE_REGISTRY.get(primitive_table)
    if not rules:
        return []

    rows: list[dict] = []
    for rule in rules:
        raw_value = _walk_path(payload, rule.source_path)
        row = _coerce_row(rule, raw_value, primitive_table, entity_id, genre_id)
        if row is not None:
            rows.append(row)

    return rows


def upsert_dimension_values(conn: psycopg.Connection, rows: list[dict]) -> int:
    """Write dimension value rows to ``bedrock.dimension_values``.

    Uses ON CONFLICT on the natural key (primitive_table, primitive_id, dimension_slug)
    to update existing rows.

    Returns the number of rows upserted.
    """
    if not rows:
        return 0

    count = 0
    with conn.cursor() as cur:
        for row in rows:
            cur.execute(
                """
                INSERT INTO bedrock.dimension_values
                    (primitive_table, primitive_id, genre_id,
                     dimension_slug, dimension_group, value_type,
                     numeric_value, categorical_value, complex_value,
                     source_path, tier)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s::jsonb, %s, %s)
                ON CONFLICT (primitive_table, primitive_id, dimension_slug)
                DO UPDATE SET
                    numeric_value = EXCLUDED.numeric_value,
                    categorical_value = EXCLUDED.categorical_value,
                    complex_value = EXCLUDED.complex_value,
                    value_type = EXCLUDED.value_type
                """,
                (
                    row["primitive_table"],
                    str(row["primitive_id"]),
                    str(row["genre_id"]),
                    row["dimension_slug"],
                    row["dimension_group"],
                    row["value_type"],
                    row["numeric_value"],
                    row["categorical_value"],
                    json.dumps(row["complex_value"]) if row["complex_value"] is not None else None,
                    row["source_path"],
                    row["tier"],
                ),
            )
            count += cur.rowcount

    return count
