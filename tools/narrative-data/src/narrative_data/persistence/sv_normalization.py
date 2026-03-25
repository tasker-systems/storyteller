# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""State variable slug normalization and resolution.

Normalizes inconsistent state variable references (title case, underscores,
parenthetical suffixes) and resolves them against the canonical set via
exact match, then prefix match.
"""

from __future__ import annotations

import re
from pathlib import Path


def normalize_sv_slug(raw: str) -> str:
    """Normalize a raw state variable reference to a canonical slug form.

    1. Strip whitespace
    2. Lowercase
    3. Strip parenthetical suffixes
    4. Replace underscores and spaces with hyphens
    5. Collapse multiple hyphens
    """
    text = raw.strip().lower()
    text = re.sub(r"\s*\(.*?\)\s*", "", text)
    text = text.replace("_", "-").replace(" ", "-")
    text = re.sub(r"-+", "-", text).strip("-")
    return text


def resolve_sv_slug(
    raw: str,
    canonical_slugs: set[str],
) -> tuple[str, str | None]:
    """Resolve a raw state variable reference against the canonical set.

    Returns:
        Tuple of (resolution_kind, resolved_slug) where resolution_kind is
        one of "exact", "prefix", or "unresolved".
    """
    normalized = normalize_sv_slug(raw)
    if not normalized:
        return ("unresolved", None)

    # Exact match
    if normalized in canonical_slugs:
        return ("exact", normalized)

    # Prefix match: normalized is a prefix of exactly one canonical slug
    prefix_matches = [c for c in canonical_slugs if c.startswith(normalized + "-")]
    if len(prefix_matches) == 1:
        return ("prefix", prefix_matches[0])

    return ("unresolved", None)


def audit_sv_resolution(
    corpus_dir: Path,
    canonical_slugs: set[str],
) -> dict[str, list[dict]]:
    """Scan all entity payloads and classify state variable references.

    Returns dict with keys "exact", "prefix", "unresolved", each mapping to
    a list of dicts with "raw", "normalized", "resolved", "type", "genre", "entity".
    """
    import json

    results: dict[str, list[dict]] = {"exact": [], "prefix": [], "unresolved": []}

    # Only types that have state_variable_interactions, state_variable_expression,
    # or state_variables fields in their entity payloads.
    for type_slug in ("goals", "dynamics", "tropes", "archetypes", "place-entities"):
        type_dir = corpus_dir / "discovery" / type_slug
        if not type_dir.exists():
            # Try genre-native path
            _scan_genre_native(corpus_dir, type_slug, canonical_slugs, results)
            continue

        for json_path in sorted(type_dir.glob("*.json")):
            if json_path.name in ("manifest.json",) or json_path.name.endswith(".errors.json"):
                continue
            try:
                data = json.loads(json_path.read_text())
            except (json.JSONDecodeError, OSError):
                continue
            if not isinstance(data, list):
                continue
            genre_slug = json_path.stem
            for entity in data:
                if not isinstance(entity, dict):
                    continue
                _collect_sv_refs(entity, type_slug, genre_slug, canonical_slugs, results)

    return results


def _scan_genre_native(
    corpus_dir: Path,
    type_slug: str,
    canonical_slugs: set[str],
    results: dict[str, list[dict]],
) -> None:
    import json

    genres_dir = corpus_dir / "genres"
    if not genres_dir.exists():
        return
    for genre_dir in sorted(genres_dir.iterdir()):
        if not genre_dir.is_dir():
            continue
        json_path = genre_dir / f"{type_slug}.json"
        if not json_path.exists():
            continue
        try:
            data = json.loads(json_path.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        if not isinstance(data, list):
            continue
        for entity in data:
            if not isinstance(entity, dict):
                continue
            _collect_sv_refs(entity, type_slug, genre_dir.name, canonical_slugs, results)


def _collect_sv_refs(
    entity: dict,
    type_slug: str,
    genre_slug: str,
    canonical_slugs: set[str],
    results: dict[str, list[dict]],
) -> None:
    entity_name = (
        entity.get("canonical_name") or entity.get("name") or entity.get("default_subject") or "?"
    )
    raw_refs: list[str] = []

    # StateVariableInteraction shape (goals, dynamics, tropes)
    for ix in entity.get("state_variable_interactions", []):
        if isinstance(ix, dict):
            vid = ix.get("variable_id")
            if isinstance(vid, str) and vid.strip():
                raw_refs.append(vid)
    # StateVariableExpression shape (place-entities)
    for expr in entity.get("state_variable_expression", []):
        if isinstance(expr, dict):
            vid = expr.get("variable_id")
            if isinstance(vid, str) and vid.strip():
                raw_refs.append(vid)
    # Bare list shape (archetypes)
    for sv in entity.get("state_variables", []):
        if isinstance(sv, str) and sv.strip():
            raw_refs.append(sv)

    for raw in raw_refs:
        kind, resolved = resolve_sv_slug(raw, canonical_slugs)
        results[kind].append(
            {
                "raw": raw,
                "normalized": normalize_sv_slug(raw),
                "resolved": resolved,
                "type": type_slug,
                "genre": genre_slug,
                "entity": entity_name,
            }
        )
