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
