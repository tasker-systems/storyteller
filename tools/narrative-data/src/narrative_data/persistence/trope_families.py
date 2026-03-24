# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Trope family normalization: extracts canonical families from genre_derivation values."""

import json
import logging
import re
from pathlib import Path

log = logging.getLogger(__name__)

_MAX_FAMILY_LENGTH = 100


def normalize_family_name(raw: str) -> str:
    """Normalize a genre_derivation string to a canonical family slug.

    1. Reject strings > _MAX_FAMILY_LENGTH as 'unclassified'
    2. Take part before first colon
    3. Lowercase
    4. Collapse plurals (dimensions→dimension, affordances→affordance, stances→stance)
    5. Slugify (spaces → hyphens)
    """
    text = raw.strip()
    if len(text) > _MAX_FAMILY_LENGTH:
        return "unclassified"

    if ":" in text:
        text = text.split(":")[0].strip()

    text = text.lower()

    text = re.sub(r"\bdimensions\b", "dimension", text)
    text = re.sub(r"\baffordances\b", "affordance", text)
    text = re.sub(r"\bstances\b", "stance", text)

    slug = text.replace(" ", "-").replace("_", "-")
    slug = re.sub(r"-+", "-", slug).strip("-")

    return slug if slug else "unclassified"


def _slug_to_display_name(slug: str) -> str:
    """Convert slug to display name: 'locus-of-power' → 'Locus of Power'."""
    return " ".join(
        word.capitalize() if i == 0 or len(word) > 2 else word
        for i, word in enumerate(slug.split("-"))
    )


def build_normalization_map(corpus_dir: Path) -> dict[str, str]:
    """Scan all trope files and build raw derivation → canonical slug mapping."""
    nmap: dict[str, str] = {}
    genres_dir = corpus_dir / "genres"
    if not genres_dir.exists():
        return nmap

    for genre_dir in sorted(genres_dir.iterdir()):
        if not genre_dir.is_dir():
            continue
        tropes_file = genre_dir / "tropes.json"
        if not tropes_file.exists():
            continue
        try:
            data = json.loads(tropes_file.read_text())
        except (json.JSONDecodeError, OSError) as exc:
            log.warning("Skipping %s: %s", tropes_file, exc)
            continue

        if not isinstance(data, list):
            continue

        for trope in data:
            derivation = trope.get("genre_derivation")
            if isinstance(derivation, str) and derivation.strip():
                nmap[derivation] = normalize_family_name(derivation)

    return nmap


def extract_trope_families(normalization_map: dict[str, str]) -> list[dict]:
    """Deduplicate canonical families from the normalization map."""
    seen: dict[str, dict] = {}
    for _raw, slug in normalization_map.items():
        if slug not in seen:
            seen[slug] = {
                "slug": slug,
                "name": _slug_to_display_name(slug),
                "description": None,
            }
    return sorted(seen.values(), key=lambda f: f["slug"])
