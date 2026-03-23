# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Postprocessing utilities: corpus null-rate audit and field coverage reporting.

Provides `audit_type()` and `audit_corpus()` for scanning structured JSON files
and computing per-field null/empty rates across genre files.
"""

import json
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class AuditResult:
    """Null-rate audit result for a single type across one or more genre files.

    Attributes:
        type_name: The primitive or genre-native type slug (e.g. "dynamics").
        genre: The genre slug if auditing a single file, or None for aggregate.
        total_entities: Total number of entity records scanned.
        file_count: Number of JSON files scanned.
        field_rates: Mapping from dot-separated field path to null rate (0.0–1.0).
                     A rate of 1.0 means every value was null/empty.
        errors: List of error messages encountered during scanning.
    """

    type_name: str
    genre: str | None
    total_entities: int
    file_count: int
    field_rates: dict[str, float]
    errors: list[str] = field(default_factory=list)


def _is_null_or_empty(value: object) -> bool:
    """Return True if the value should be treated as null/empty for audit purposes.

    Treats None, empty strings, and empty lists as null.
    """
    if value is None:
        return True
    if isinstance(value, str) and value.strip() == "":
        return True
    if isinstance(value, list) and len(value) == 0:
        return True
    return False


def _flatten_fields(obj: object, prefix: str = "") -> dict[str, object]:
    """Recursively flatten a nested dict to dot-separated key → leaf value pairs.

    Lists of dicts are not individually flattened — list-typed fields are
    represented by a single entry at the list's key path so that null/empty list
    detection works correctly.  Only plain dicts trigger recursive descent.

    Args:
        obj: The value to flatten (dict, list, or scalar).
        prefix: Dot-separated key prefix accumulated by recursion.

    Returns:
        Dict mapping dot-separated field paths to their leaf values.
    """
    result: dict[str, object] = {}
    if isinstance(obj, dict):
        for k, v in obj.items():
            full_key = f"{prefix}.{k}" if prefix else k
            if isinstance(v, dict):
                result.update(_flatten_fields(v, prefix=full_key))
            else:
                result[full_key] = v
    else:
        result[prefix] = obj
    return result


def _tally_entity(
    entity: dict,
    field_null_counts: dict[str, int],
    field_total_counts: dict[str, int],
) -> None:
    """Update null/total counters for a single entity record.

    Args:
        entity: A single structured entity dict.
        field_null_counts: Mutable counter mapping field path → null count.
        field_total_counts: Mutable counter mapping field path → total count.
    """
    flattened = _flatten_fields(entity)
    for key, value in flattened.items():
        field_total_counts[key] = field_total_counts.get(key, 0) + 1
        if _is_null_or_empty(value):
            field_null_counts[key] = field_null_counts.get(key, 0) + 1


def audit_type(type_name: str, json_path: Path) -> AuditResult:
    """Audit a single JSON file for field null rates.

    The file may contain either a JSON array (discovery types) or a JSON object
    (genre-native or genre region types treated as a single-entity list).

    Args:
        type_name: The primitive type slug (e.g. "dynamics", "region").
        json_path: Path to the JSON file to audit.

    Returns:
        AuditResult with per-field null rates for the file.
    """
    errors: list[str] = []

    if not json_path.exists():
        return AuditResult(
            type_name=type_name,
            genre=json_path.parent.name,
            total_entities=0,
            file_count=0,
            field_rates={},
            errors=[f"File not found: {json_path}"],
        )

    try:
        raw = json.loads(json_path.read_text())
    except (json.JSONDecodeError, OSError) as exc:
        return AuditResult(
            type_name=type_name,
            genre=json_path.parent.name,
            total_entities=0,
            file_count=1,
            field_rates={},
            errors=[f"Failed to load {json_path}: {exc}"],
        )

    # Normalise: treat a single dict as a list of one entity
    if isinstance(raw, dict):
        entities: list[dict] = [raw]
    elif isinstance(raw, list):
        entities = [e for e in raw if isinstance(e, dict)]
        non_dict = len(raw) - len(entities)
        if non_dict:
            errors.append(f"{non_dict} non-dict items skipped in {json_path}")
    else:
        return AuditResult(
            type_name=type_name,
            genre=json_path.parent.name,
            total_entities=0,
            file_count=1,
            field_rates={},
            errors=[f"Unexpected top-level type {type(raw).__name__} in {json_path}"],
        )

    field_null_counts: dict[str, int] = {}
    field_total_counts: dict[str, int] = {}

    for entity in entities:
        _tally_entity(entity, field_null_counts, field_total_counts)

    field_rates: dict[str, float] = {}
    for key, total in field_total_counts.items():
        null_count = field_null_counts.get(key, 0)
        field_rates[key] = null_count / total if total > 0 else 0.0

    # Derive genre from parent directory name (best-effort)
    genre = json_path.parent.name if json_path.parent.name != "." else None

    return AuditResult(
        type_name=type_name,
        genre=genre,
        total_entities=len(entities),
        file_count=1,
        field_rates=field_rates,
        errors=errors,
    )


def audit_corpus(
    corpus_dir: Path,
    types: list[str] | None = None,
    genres: list[str] | None = None,
) -> dict[str, AuditResult]:
    """Aggregate null-rate audit across all genre files for the specified types.

    Discovery types live at:  ``corpus_dir/discovery/{type}/{genre}.json``
    Genre-native types live at: ``corpus_dir/genres/{genre}/{type}.json``

    Args:
        corpus_dir: The ``narrative-data/`` root directory.
        types: List of type slugs to audit.  If None, audits all known types.
        genres: List of genre slugs to include.  If None, includes all found.

    Returns:
        Dict mapping type slug → aggregated AuditResult across all matching files.
    """
    from narrative_data.config import GENRE_NATIVE_TYPES, PRIMITIVE_TYPES

    type_list = types if types is not None else PRIMITIVE_TYPES + GENRE_NATIVE_TYPES

    results: dict[str, AuditResult] = {}

    for type_slug in type_list:
        all_files: list[Path] = []

        if type_slug in GENRE_NATIVE_TYPES:
            # genres/{genre}/{type}.json
            genres_dir = corpus_dir / "genres"
            if genres_dir.exists():
                for genre_dir in sorted(genres_dir.iterdir()):
                    if not genre_dir.is_dir():
                        continue
                    if genres and genre_dir.name not in genres:
                        continue
                    candidate = genre_dir / f"{type_slug}.json"
                    if candidate.exists():
                        all_files.append(candidate)
        else:
            # discovery/{type}/{genre}.json
            type_dir = corpus_dir / "discovery" / type_slug
            if type_dir.exists():
                for json_file in sorted(type_dir.glob("*.json")):
                    # Skip manifest and pipeline metadata files
                    if json_file.name in ("manifest.json",):
                        continue
                    if json_file.name.endswith(".errors.json"):
                        continue
                    if genres and json_file.stem not in genres:
                        continue
                    all_files.append(json_file)

        if not all_files:
            results[type_slug] = AuditResult(
                type_name=type_slug,
                genre=None,
                total_entities=0,
                file_count=0,
                field_rates={},
                errors=[f"No files found for type '{type_slug}' in {corpus_dir}"],
            )
            continue

        # Aggregate across all files
        combined_null: dict[str, int] = {}
        combined_total: dict[str, int] = {}
        total_entities = 0
        all_errors: list[str] = []

        for fp in all_files:
            single = audit_type(type_slug, fp)
            total_entities += single.total_entities
            all_errors.extend(single.errors)
            # Merge per-field tallies by back-computing counts from rates
            # Re-tally directly from raw data to avoid floating-point drift
            try:
                raw = json.loads(fp.read_text())
            except (json.JSONDecodeError, OSError):
                continue

            if isinstance(raw, dict):
                entities: list[dict] = [raw]
            elif isinstance(raw, list):
                entities = [e for e in raw if isinstance(e, dict)]
            else:
                continue

            for entity in entities:
                _tally_entity(entity, combined_null, combined_total)

        field_rates: dict[str, float] = {}
        for key, total in combined_total.items():
            null_count = combined_null.get(key, 0)
            field_rates[key] = null_count / total if total > 0 else 0.0

        results[type_slug] = AuditResult(
            type_name=type_slug,
            genre=None,
            total_entities=total_entities,
            file_count=len(all_files),
            field_rates=field_rates,
            errors=all_errors,
        )

    return results
