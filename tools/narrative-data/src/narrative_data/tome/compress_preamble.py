# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Preamble compression for the Tome elicitation pipeline.

Reduces world-position.json (typically ~36 KB with edge-traversal justifications)
to a compact domain-grouped markdown summary (~3-4 KB) for injection into LLM prompts.
Justifications, confidence values, and edge traces are dropped; seed provenance is
preserved with a [seed] label.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# build_domain_index
# ---------------------------------------------------------------------------


def build_domain_index(domains_dir: Path) -> dict[str, str]:
    """Build a mapping from axis slug to domain display name.

    Reads all JSON files in ``domains_dir``.  Each file has the structure::

        {
            "domain": {"slug": "...", "name": "Material Conditions"},
            "axes": [{"slug": "geography-climate"}, ...]
        }

    Args:
        domains_dir: Path to the directory containing domain JSON files.

    Returns:
        Mapping of axis_slug → domain display name.  Returns an empty dict
        when the directory does not exist or contains no JSON files.
    """
    if not domains_dir.exists():
        return {}

    index: dict[str, str] = {}
    for domain_file in sorted(domains_dir.glob("*.json")):
        try:
            data = json.loads(domain_file.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        domain_name: str = data.get("domain", {}).get("name", domain_file.stem)
        for axis in data.get("axes", []):
            slug = axis.get("slug")
            if slug:
                index[slug] = domain_name

    return index


# ---------------------------------------------------------------------------
# compress_preamble
# ---------------------------------------------------------------------------


def compress_preamble(world_pos: dict[str, Any], domains_dir: Path) -> str:
    """Compress a world-position dict into a domain-grouped markdown summary.

    Drops justifications, confidence values, and edge traces.  Seeds are
    labelled with ``[seed]``; inferred positions carry no extra marker.
    Known domains are sorted alphabetically; axes not belonging to any domain
    appear under an ``### Other`` section at the end.

    Args:
        world_pos: Parsed world-position.json dict containing a "positions" list.
        domains_dir: Path to the directory containing domain JSON files.

    Returns:
        Markdown string with ``### Domain Name`` section headers.
    """
    domain_index = build_domain_index(domains_dir)
    positions: list[dict[str, Any]] = world_pos.get("positions", [])

    if not positions:
        return ""

    # Group positions by domain name
    grouped: dict[str, list[str]] = {}
    other: list[str] = []

    for pos in positions:
        axis_slug: str = pos.get("axis_slug", "")
        value: str = pos.get("value", "")
        source: str = pos.get("source", "inferred")

        label = f"- {axis_slug}: {value}"
        if source == "seed":
            label += " [seed]"

        domain_name = domain_index.get(axis_slug)
        if domain_name:
            grouped.setdefault(domain_name, []).append(label)
        else:
            other.append(label)

    # Build output — known domains sorted alphabetically, Other last
    sections: list[str] = []
    for domain_name in sorted(grouped.keys()):
        lines = [f"### {domain_name}"] + grouped[domain_name]
        sections.append("\n".join(lines))

    if other:
        lines = ["### Other"] + other
        sections.append("\n".join(lines))

    return "\n\n".join(sections)


# ---------------------------------------------------------------------------
# subset_axes
# ---------------------------------------------------------------------------


def subset_axes(compressed_preamble: str, domain_names: list[str]) -> str:
    """Extract specific domain sections from a compressed preamble string.

    Args:
        compressed_preamble: Output of :func:`compress_preamble`.
        domain_names: Display names of domains to include (e.g. ``["Material Conditions"]``).

    Returns:
        Markdown string containing only the requested sections.  Returns an
        empty string when the preamble is empty or none of the domains match.
    """
    if not compressed_preamble:
        return ""

    wanted = set(domain_names)
    # Split on section boundaries; keep the header with each body
    raw_sections = compressed_preamble.split("### ")
    matched: list[str] = []

    for section in raw_sections:
        if not section.strip():
            continue
        # First line is the domain name
        first_newline = section.find("\n")
        header = section.strip() if first_newline == -1 else section[:first_newline].strip()

        if header in wanted:
            matched.append(f"### {section.rstrip()}")

    return "\n\n".join(matched)


# ---------------------------------------------------------------------------
# build_world_summary
# ---------------------------------------------------------------------------


def build_world_summary(world_pos: dict[str, Any], domains_dir: Path) -> dict[str, Any]:
    """Build a compact summary dict for a world-position.

    Args:
        world_pos: Parsed world-position.json dict.
        domains_dir: Path to the directory containing domain JSON files.

    Returns:
        Dict with keys: ``genre_slug``, ``setting_slug``, ``compressed_preamble``,
        ``axis_count``, ``seed_count``.
    """
    positions: list[dict[str, Any]] = world_pos.get("positions", [])
    seed_count = sum(1 for p in positions if p.get("source") == "seed")

    return {
        "genre_slug": world_pos.get("genre_slug", ""),
        "setting_slug": world_pos.get("setting_slug", ""),
        "compressed_preamble": compress_preamble(world_pos, domains_dir),
        "axis_count": len(positions),
        "seed_count": seed_count,
    }
