# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Trope family normalization: extracts canonical families from genre_derivation values."""

import json
import logging
from pathlib import Path

log = logging.getLogger(__name__)

_DIMENSION_KEYWORDS: list[tuple[str, str]] = [
    # Multi-word matches first (most specific)
    ("world affordance", "world-affordance"),
    ("world-affordance", "world-affordance"),
    ("locus of power", "locus-of-power"),
    ("locus-of-power", "locus-of-power"),
    ("state variable", "state-variable"),
    ("state-variable", "state-variable"),
    # Single-word matches (ordered by specificity)
    ("epistemological", "epistemological-stance"),
    ("ontological", "ontological-posture"),
    ("aesthetic", "aesthetic-dimension"),
    ("tonal", "tonal-dimension"),
    ("temporal", "temporal-dimension"),
    ("thematic", "thematic-dimension"),
    ("agency", "agency-dimension"),
    ("structural", "structural-dimension"),
]

_SECONDARY_KEYWORDS: list[tuple[str, str]] = [
    ("identity", "thematic-dimension"),
    ("belonging", "thematic-dimension"),
    ("social capital", "thematic-dimension"),
    ("collective", "thematic-dimension"),
    ("solidarity", "thematic-dimension"),
    ("materiality", "thematic-dimension"),
    ("memory", "thematic-dimension"),
    ("mystery", "structural-dimension"),
    ("tragedy", "structural-dimension"),
    ("magic", "world-affordance"),
    ("medical", "world-affordance"),
    ("violence", "world-affordance"),
    ("antagonistic", "locus-of-power"),
    ("power", "locus-of-power"),
]


def normalize_family_name(raw: str) -> str:
    """Normalize a genre_derivation string to a canonical family slug.

    Scans for dimension keywords (primary then secondary) to classify
    the derivation into one of ~11 canonical trope families.
    """
    text = raw.strip()
    if not text:
        return "genre-specific"

    lower = text.lower()

    for keyword, family in _DIMENSION_KEYWORDS:
        if keyword in lower:
            return family

    for keyword, family in _SECONDARY_KEYWORDS:
        if keyword in lower:
            return family

    return "genre-specific"


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
