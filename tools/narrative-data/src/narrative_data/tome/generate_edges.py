# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Generate exhaustive edge combinatorics for the Tome mutual production graph.

Reads all 6 domain JSON files and produces per-domain-pair markdown files
ready for LLM annotation. Each file contains a table of axis pairs with
columns for edge type, weight, and description.

Output: {data_path}/narrative-data/tome/edges/{source-domain}/{target-domain}.md
Manifest: {data_path}/narrative-data/tome/edges/manifest.json
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from itertools import combinations
from pathlib import Path
from typing import Any

DOMAIN_FILES: list[str] = [
    "material-conditions.json",
    "economic-forms.json",
    "political-structures.json",
    "social-forms.json",
    "history-as-force.json",
    "aesthetic-cultural-forms.json",
]

EDGE_TYPES: list[dict[str, str]] = [
    {
        "type": "produces",
        "meaning": "A gives rise to B. If A is present, B is expected.",
    },
    {
        "type": "constrains",
        "meaning": "A limits what B can be. Some B values are implausible given A.",
    },
    {
        "type": "enables",
        "meaning": "A makes B possible but doesn't require it.",
    },
    {
        "type": "transforms",
        "meaning": "A changes B's character over time. A's presence shifts B dynamically.",
    },
    {
        "type": "none",
        "meaning": "No meaningful relationship between these axes.",
    },
]


def _load_domains(domains_dir: Path) -> dict[str, dict[str, Any]]:
    """Load all domain JSON files, keyed by domain slug."""
    domains: dict[str, dict[str, Any]] = {}
    for filename in DOMAIN_FILES:
        filepath = domains_dir / filename
        if not filepath.exists():
            msg = f"Domain file not found: {filepath}"
            raise FileNotFoundError(msg)
        with filepath.open() as f:
            data = json.load(f)
        slug = data["domain"]["slug"]
        domains[slug] = data
    return domains


def _render_axis_values(axis: dict[str, Any]) -> str:
    """Render axis values in a human-readable format based on axis_type."""
    axis_type = axis["axis_type"]
    values = axis["values"]

    if axis_type in ("categorical", "set", "ordinal"):
        if isinstance(values, list):
            return ", ".join(str(v) for v in values)
        return str(values)

    if axis_type == "bipolar":
        low_label = values.get("low_label", "low")
        high_label = values.get("high_label", "high")
        return f"bipolar: {low_label} \u2194 {high_label}"

    if axis_type == "numeric":
        low_label = values.get("low_label", str(values.get("low", "low")))
        high_label = values.get("high_label", str(values.get("high", "high")))
        return f"numeric: {low_label} \u2194 {high_label}"

    if axis_type == "profile":
        sub_dims = values.get("sub_dimensions", [])
        return f"profile: {', '.join(sub_dims)}"

    return str(values)


def _truncate(text: str, max_len: int = 200) -> str:
    """Truncate text to max_len characters, adding ellipsis if needed."""
    if len(text) <= max_len:
        return text
    return text[: max_len - 3] + "..."


def _render_axis_section(axes: list[dict[str, Any]], heading: str) -> str:
    """Render a section listing axes with their metadata."""
    lines = [f"## {heading}\n"]
    for ax in axes:
        desc = _truncate(ax.get("description", ""), 200)
        vals = _render_axis_values(ax)
        lines.append(f"### `{ax['slug']}` ({ax['axis_type']})")
        lines.append(f"")
        lines.append(f"{desc}")
        lines.append(f"")
        lines.append(f"**Values:** {vals}")
        lines.append(f"")
    return "\n".join(lines)


def _render_edge_type_table() -> str:
    """Render the edge type definitions table."""
    lines = [
        "## Edge Type Definitions",
        "",
        "| Type | Meaning |",
        "|------|---------|",
    ]
    for et in EDGE_TYPES:
        lines.append(f"| {et['type']} | {et['meaning']} |")
    lines.append("")
    lines.append(
        "For **compound edges**, note when two or more source axes *jointly* "
        "produce an effect on a target that neither captures alone."
    )
    lines.append("")
    return "\n".join(lines)


def _generate_within_domain(domain_data: dict[str, Any]) -> str:
    """Generate markdown for within-domain edge assessment."""
    domain = domain_data["domain"]
    axes = domain_data["axes"]

    lines = [
        f"# Within-Domain Edge Assessment: {domain['name']}",
        "",
        f"> {domain['description']}",
        "",
    ]

    lines.append(_render_axis_section(axes, "Axes"))
    lines.append(_render_edge_type_table())

    # Pair table: all directed pairs (skip self-pairs)
    lines.append("## Edge Assessment")
    lines.append("")
    lines.append("| From | To | Type | Weight | Description |")
    lines.append("|------|----|------|--------|-------------|")
    for i, source_ax in enumerate(axes):
        for j, target_ax in enumerate(axes):
            if i == j:
                continue
            lines.append(
                f"| {source_ax['slug']} | {target_ax['slug']} |  |  |  |"
            )
    lines.append("")

    lines.append("## Compound Edges")
    lines.append("")
    lines.append("<!-- Note any compound edges here: two or more axes that jointly ")
    lines.append("produce an effect on a target that neither captures alone. -->")
    lines.append("")

    lines.append("## Cluster Annotation")
    lines.append("")
    lines.append("<!-- Note any naturally co-occurring axis clusters within this domain. -->")
    lines.append("")

    return "\n".join(lines)


def _generate_cross_domain(
    source_data: dict[str, Any], target_data: dict[str, Any]
) -> str:
    """Generate markdown for cross-domain edge assessment."""
    source_domain = source_data["domain"]
    target_domain = target_data["domain"]
    source_axes = source_data["axes"]
    target_axes = target_data["axes"]

    lines = [
        f"# Cross-Domain Edge Assessment: {source_domain['name']} \u2192 {target_domain['name']}",
        "",
        f"**Source:** {source_domain['name']} \u2014 {source_domain['description']}",
        "",
        f"**Target:** {target_domain['name']} \u2014 {target_domain['description']}",
        "",
    ]

    lines.append(_render_axis_section(source_axes, "Source Axes"))
    lines.append(_render_axis_section(target_axes, "Target Axes"))
    lines.append(_render_edge_type_table())

    # Pair table: all directed pairs from source to target
    lines.append("## Edge Assessment")
    lines.append("")
    lines.append("| From | To | Type | Weight | Description |")
    lines.append("|------|----|------|--------|-------------|")
    for source_ax in source_axes:
        for target_ax in target_axes:
            lines.append(
                f"| {source_ax['slug']} | {target_ax['slug']} |  |  |  |"
            )
    lines.append("")

    lines.append("## Compound Edges")
    lines.append("")
    lines.append("<!-- Note any compound edges here: two or more source axes that jointly ")
    lines.append("produce an effect on a target that neither captures alone. -->")
    lines.append("")

    lines.append("## Cluster Annotation")
    lines.append("")
    lines.append("<!-- Note any cross-domain axis clusters that naturally co-occur. -->")
    lines.append("")

    return "\n".join(lines)


def _count_pairs_within(axes: list[dict[str, Any]]) -> int:
    """Count directed pairs within a domain (excluding self-pairs)."""
    n = len(axes)
    return n * (n - 1)


def _count_pairs_cross(
    source_axes: list[dict[str, Any]], target_axes: list[dict[str, Any]]
) -> int:
    """Count directed pairs from source to target domain."""
    return len(source_axes) * len(target_axes)


def generate_all(data_path: Path) -> None:
    """Generate all edge combinatoric files and manifest.

    Args:
        data_path: Root of the storyteller-data repository.
    """
    from rich.console import Console

    console = Console()

    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    edges_dir = data_path / "narrative-data" / "tome" / "edges"

    console.print(f"[cyan]Loading domains from:[/cyan] {domains_dir}")
    domains = _load_domains(domains_dir)
    domain_slugs = list(domains.keys())

    console.print(f"[cyan]Found {len(domain_slugs)} domains:[/cyan] {', '.join(domain_slugs)}")

    edges_dir.mkdir(parents=True, exist_ok=True)

    manifest_entries: dict[str, dict[str, Any]] = {}
    total_pairs = 0

    # Within-domain chunks (6)
    for slug in domain_slugs:
        domain_data = domains[slug]
        content = _generate_within_domain(domain_data)

        chunk_dir = edges_dir / slug
        chunk_dir.mkdir(parents=True, exist_ok=True)
        filepath = chunk_dir / "within.md"
        filepath.write_text(content)

        pair_count = _count_pairs_within(domain_data["axes"])
        total_pairs += pair_count

        manifest_entries[f"{slug}/within"] = {
            "source_domain": slug,
            "target_domain": slug,
            "within_domain": True,
            "pair_count": pair_count,
            "status": "pending",
            "filepath": str(filepath),
        }

        console.print(
            f"  [green]\u2713[/green] {slug}/within.md "
            f"({pair_count} pairs)"
        )

    # Cross-domain chunks (15 = C(6,2))
    for source_slug, target_slug in combinations(domain_slugs, 2):
        source_data = domains[source_slug]
        target_data = domains[target_slug]
        content = _generate_cross_domain(source_data, target_data)

        chunk_dir = edges_dir / source_slug
        chunk_dir.mkdir(parents=True, exist_ok=True)
        filepath = chunk_dir / f"{target_slug}.md"
        filepath.write_text(content)

        pair_count = _count_pairs_cross(source_data["axes"], target_data["axes"])
        total_pairs += pair_count

        manifest_entries[f"{source_slug}/{target_slug}"] = {
            "source_domain": source_slug,
            "target_domain": target_slug,
            "within_domain": False,
            "pair_count": pair_count,
            "status": "pending",
            "filepath": str(filepath),
        }

        console.print(
            f"  [green]\u2713[/green] {source_slug}/{target_slug}.md "
            f"({pair_count} pairs)"
        )

    # Write manifest
    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "total_pairs": total_pairs,
        "total_chunks": len(manifest_entries),
        "entries": manifest_entries,
    }
    manifest_path = edges_dir / "manifest.json"
    with manifest_path.open("w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")

    console.print()
    console.print(
        f"[bold green]Generated {len(manifest_entries)} chunks "
        f"with {total_pairs} total pairs[/bold green]"
    )
    console.print(f"[cyan]Manifest:[/cyan] {manifest_path}")
