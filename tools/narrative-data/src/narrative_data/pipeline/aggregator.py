# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Aggregator — reads segment JSON files and assembles validated Pydantic objects.

This is the final deterministic step in the pipeline. No LLM involvement.
Validation happens at assembly time via Pydantic constructors, so errors
point to the specific field (and therefore segment file) that has the problem.
"""

import json
from pathlib import Path

from pydantic import BaseModel

from narrative_data.schemas.genre_dimensions import GenreDimensions


def load_segment_json(path: Path) -> dict | list:
    """Load a segment JSON file, returning raw deserialized data (dict or list).

    Pydantic does the actual type coercion downstream.

    Raises:
        FileNotFoundError: if the path does not exist.
    """
    if not path.exists():
        raise FileNotFoundError(f"Segment file not found: {path}")
    return json.loads(path.read_text())


def aggregate_genre_dimensions(segment_dir: Path) -> GenreDimensions:
    """Read 13 segment JSONs and assemble into a validated GenreDimensions object.

    Args:
        segment_dir: Directory containing all segment-*.json files for one genre.

    Returns:
        Fully validated GenreDimensions instance.

    Raises:
        FileNotFoundError: if any expected segment file is missing.
        pydantic.ValidationError: if any segment contains invalid data.
    """
    meta = load_segment_json(segment_dir / "segment-meta.json")
    return GenreDimensions(
        genre_slug=meta["genre_slug"],
        genre_name=meta["genre_name"],
        classification=meta["classification"],
        constraint_layer_type=meta.get("constraint_layer_type"),
        aesthetic=load_segment_json(segment_dir / "segment-aesthetic.json"),
        tonal=load_segment_json(segment_dir / "segment-tonal.json"),
        temporal=load_segment_json(segment_dir / "segment-temporal.json"),
        thematic=load_segment_json(segment_dir / "segment-thematic.json"),
        agency=load_segment_json(segment_dir / "segment-agency.json"),
        epistemological=load_segment_json(segment_dir / "segment-epistemological.json"),
        world_affordances=load_segment_json(segment_dir / "segment-world-affordances.json"),
        locus_of_power=load_segment_json(segment_dir / "segment-locus-of-power.json"),
        narrative_structure=load_segment_json(segment_dir / "segment-narrative-structure.json"),
        narrative_contracts=load_segment_json(segment_dir / "segment-narrative-contracts.json"),
        active_state_variables=load_segment_json(segment_dir / "segment-state-variables.json"),
        boundaries=load_segment_json(segment_dir / "segment-boundaries.json"),
        modifies=meta.get("modifies", []),
        flavor_text=meta.get("flavor_text"),
    )


def aggregate_discovery(segment_dir: Path, schema: type[BaseModel]) -> list:
    """Collect all segment-*.json files and validate each against the entity schema.

    Note: segments-manifest.json starts with "segments-" (not "segment-"), so
    the glob pattern naturally excludes it.

    Args:
        segment_dir: Directory containing segment-*.json files.
        schema: Pydantic model class to validate each segment against.

    Returns:
        List of validated schema instances, sorted by filename.

    Raises:
        pydantic.ValidationError: if any segment fails schema validation.
    """
    entities = []
    for seg_json in sorted(segment_dir.glob("segment-*.json")):
        data = json.loads(seg_json.read_text())
        entities.append(schema.model_validate(data))
    return entities
