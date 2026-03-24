# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Postprocessing utilities: corpus null-rate audit, field coverage reporting, and fills.

Provides:
- ``audit_type()`` and ``audit_corpus()`` for scanning structured JSON files and
  computing per-field null/empty rates across genre files.
- ``fill_spans_scales()`` and ``fill_agency()`` for deterministic field population.
- ``fill_all_deterministic()`` for orchestrated corpus-wide fill runs.
"""

import json
import re
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
    return bool(isinstance(value, list) and len(value) == 0)


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


# ---------------------------------------------------------------------------
# Deterministic fill functions
# ---------------------------------------------------------------------------

# Canonical scale keywords extracted from parentheticals.
_SCALE_KEYWORDS: frozenset[str] = frozenset(["orbital", "arc", "scene"])

# Regex to find a Scale line anywhere in a markdown section.
# Matches:  *   **Scale:** Spanning (Orbital/Scene)
_SCALE_LINE_RE = re.compile(r"\*\s*\*\*Scale:\*\*\s*(.+?)(?:\n|$)", re.IGNORECASE)

# Lookup table: (normalised_friction_type, normalised_directionality_type) → agency
# Directionality type values may use underscores or hyphens — normalise to underscores.
_AGENCY_LOOKUP: dict[tuple[str, str], str] = {
    ("high", "one_way"): "none",
    ("high", "unidirectional"): "none",
    ("high", "bidirectional"): "low",
    ("medium", "bidirectional"): "medium",
    ("low", "bidirectional"): "high",
    ("low", "constrained"): "illusion",
    ("medium", "constrained"): "illusion",
}


def _extract_scale_values(scale_text: str) -> list[str]:
    """Parse scale labels from a ``**Scale:**`` value string.

    Looks for parenthetical tokens matching known scale keywords ("orbital",
    "arc", "scene").  Returns an empty list when fewer than two distinct
    keywords are found (i.e. a single-scale entry does not populate
    ``spans_scales``).

    Examples::

        "Spanning (Orbital/Scene)"        → ["orbital", "scene"]
        "Cross-Scale (Orbital vs. Scene)" → ["orbital", "scene"]
        "Orbital (Primary); Scene (Secondary)" → ["orbital", "scene"]
        "Orbital"                          → []

    Args:
        scale_text: The raw text after ``**Scale:**``.

    Returns:
        List of unique scale keyword strings (lowercased), in canonical order
        (orbital, arc, scene), or an empty list if fewer than two are found.
    """
    lower = scale_text.lower()
    found = [kw for kw in ["orbital", "arc", "scene"] if kw in lower]
    # Only populate when multiple scales are present (i.e. it genuinely spans)
    if len(found) < 2:
        return []
    return found


def _find_entity_section(md_content: str, entity_name: str) -> str:
    """Extract the markdown section nearest to a given entity name.

    Searches for a heading (any level) or bullet containing the entity name,
    then returns the text from that point until the next heading of the same
    or higher level (or end of document).

    Args:
        md_content: Full source markdown text.
        entity_name: The ``canonical_name`` of the entity to locate.

    Returns:
        The relevant markdown section as a string, or an empty string if the
        entity name is not found.
    """
    # Escape for regex but allow flexible whitespace
    escaped = re.escape(entity_name)
    # Find the line that contains the entity name (heading or bullet)
    pattern = re.compile(rf"^.*{escaped}.*$", re.IGNORECASE | re.MULTILINE)
    m = pattern.search(md_content)
    if not m:
        return ""

    start = m.start()
    # Determine heading level of the matched line (if it's a heading)
    line = m.group(0).lstrip()
    heading_match = re.match(r"^(#{1,6})\s", line)
    if heading_match:
        level = len(heading_match.group(1))
        # Next heading at same or higher level ends the section
        end_pattern = re.compile(rf"^#{{1,{level}}}\s", re.MULTILINE)
        end_m = end_pattern.search(md_content, m.end())
        end = end_m.start() if end_m else len(md_content)
    else:
        # Not a heading — take the next 20 lines as the section
        lines = md_content[start:].split("\n")
        section_lines = lines[:20]
        end = start + len("\n".join(section_lines))

    return md_content[start:end]


def fill_spans_scales(entity: dict, md_content: str, entity_name: str) -> dict:
    """Fill the ``spans_scales`` field of a dynamics entity from source markdown.

    Locates the entity's section in *md_content* by *entity_name*, finds the
    first ``**Scale:**`` line, and extracts any recognised scale keywords
    ("orbital", "arc", "scene") from the parenthetical.

    Skips silently if ``spans_scales`` is already non-empty (idempotent).

    Args:
        entity: A dynamics entity dict (will not be mutated).
        md_content: Full source markdown for the genre file.
        entity_name: The ``canonical_name`` to locate in *md_content*.

    Returns:
        A new dict with ``spans_scales`` populated (or unchanged if already set).
    """
    result = dict(entity)
    existing = result.get("spans_scales")
    if isinstance(existing, list) and len(existing) > 0:
        return result

    section = _find_entity_section(md_content, entity_name)
    if not section:
        return result

    scale_match = _SCALE_LINE_RE.search(section)
    if not scale_match:
        return result

    scale_text = scale_match.group(1).strip()
    values = _extract_scale_values(scale_text)
    if values:
        result = dict(result)
        result["spans_scales"] = values
    return result


def _normalise_type(value: str) -> str:
    """Normalise a type string: lowercase and replace hyphens with underscores."""
    return value.lower().replace("-", "_")


def fill_agency(entity: dict) -> dict:
    """Fill the ``agency`` field of a spatial-topology entity.

    Derives agency level from the combination of ``friction.type`` and
    ``directionality.type`` using a fixed lookup table.  Handles both hyphen
    and underscore variants of type values (e.g. ``"one-way"`` = ``"one_way"``).

    Skips silently if ``agency`` is already a non-empty, non-None value.

    Args:
        entity: A spatial-topology entity dict (will not be mutated).

    Returns:
        A new dict with ``agency`` populated (or unchanged if already set or
        lookup fails).
    """
    result = dict(entity)
    existing = result.get("agency")
    # Skip if already populated (not None and not empty string)
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    friction = result.get("friction")
    directionality = result.get("directionality")
    if not isinstance(friction, dict) or not isinstance(directionality, dict):
        return result

    friction_type = friction.get("type")
    dir_type = directionality.get("type")
    if not friction_type or not dir_type:
        return result

    key = (_normalise_type(str(friction_type)), _normalise_type(str(dir_type)))
    agency_value = _AGENCY_LOOKUP.get(key)
    if agency_value is not None:
        result = dict(result)
        result["agency"] = agency_value
    return result


def fill_all_deterministic(
    corpus_dir: Path,
    types: list[str] | None,
    genres: list[str] | None,
    dry_run: bool = False,
) -> dict[str, dict]:
    """Walk the corpus and apply deterministic fill functions per type.

    For ``dynamics``: reads source markdown from
    ``corpus_dir/discovery/dynamics/{genre}.md``, applies ``fill_spans_scales``
    to each entity, writes the updated JSON back (unless *dry_run*).

    For ``spatial-topology``: applies ``fill_agency`` to each entity.

    Only writes files when at least one entity changed.

    Args:
        corpus_dir: The ``narrative-data/`` root directory.
        types: List of type slugs to process.  If None, processes all supported
               fill types (``dynamics``, ``spatial-topology``).
        genres: List of genre slugs to restrict processing to.  If None, all
                found files are processed.
        dry_run: When True, computes changes but does not write any files.

    Returns:
        Dict mapping type slug → summary dict with keys:
        ``files_processed``, ``entities_updated``, ``entities_skipped``.
    """
    supported_fill_types = ["dynamics", "spatial-topology"]
    type_list = types if types is not None else supported_fill_types

    summary: dict[str, dict] = {}

    for type_slug in type_list:
        if type_slug not in supported_fill_types:
            continue

        files_processed = 0
        entities_updated = 0
        entities_skipped = 0

        type_dir = corpus_dir / "discovery" / type_slug
        if not type_dir.exists():
            summary[type_slug] = {
                "files_processed": 0,
                "entities_updated": 0,
                "entities_skipped": 0,
            }
            continue

        for json_path in sorted(type_dir.glob("*.json")):
            if json_path.name in ("manifest.json",):
                continue
            if json_path.name.endswith(".errors.json"):
                continue
            genre_slug = json_path.stem
            if genres and genre_slug not in genres:
                continue

            try:
                raw = json.loads(json_path.read_text())
            except (json.JSONDecodeError, OSError):
                continue

            if not isinstance(raw, list):
                continue

            entities: list[dict] = [e for e in raw if isinstance(e, dict)]
            files_processed += 1

            # For dynamics: load the companion markdown
            md_content: str = ""
            if type_slug == "dynamics":
                md_path = type_dir / f"{genre_slug}.md"
                if md_path.exists():
                    try:
                        md_content = md_path.read_text()
                    except OSError:
                        md_content = ""

            updated_entities: list[dict] = []
            file_changed = False

            for entity in entities:
                if type_slug == "dynamics":
                    entity_name = entity.get("canonical_name", "")
                    filled = fill_spans_scales(entity, md_content, str(entity_name))
                elif type_slug == "spatial-topology":
                    filled = fill_agency(entity)
                else:
                    filled = entity

                if filled != entity:
                    entities_updated += 1
                    file_changed = True
                else:
                    entities_skipped += 1

                updated_entities.append(filled)

            if file_changed and not dry_run:
                json_path.write_text(json.dumps(updated_entities, indent=2))

        summary[type_slug] = {
            "files_processed": files_processed,
            "entities_updated": entities_updated,
            "entities_skipped": entities_skipped,
        }

    return summary
