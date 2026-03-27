# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Export curated edge markdown files to canonical edges.json.

Reads all annotated (or curated) markdown files in the Tome edges directory,
parses the pair tables, compound edges, and cluster annotations, then writes:

  {data_path}/narrative-data/tome/edges.json          — all non-none edges
  {data_path}/narrative-data/tome/edges-rejected.json — pairs marked 'none'

Usage (via CLI):
    uv run narrative-data tome export-edges
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.utils import now_iso

_VALID_EDGE_TYPES = frozenset({"produces", "constrains", "enables", "transforms"})

# ---------------------------------------------------------------------------
# Table parsing
# ---------------------------------------------------------------------------


def _parse_table_row(line: str) -> list[str] | None:
    """Parse a markdown table row into a list of cell values.

    Returns None for separator rows (---|---) or lines that aren't table rows.
    """
    line = line.strip()
    if not line.startswith("|"):
        return None
    # Strip leading/trailing pipes and split
    cells = [c.strip() for c in line.strip("|").split("|")]
    # Skip separator rows
    if all(re.match(r"^-+$", c.replace(" ", "")) for c in cells if c):
        return None
    return cells


def _parse_edge_assessment(content: str) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Parse the '## Edge Assessment' section into edge and rejected lists.

    Returns:
        (edges, rejected) — two lists of dicts.
    """
    start_marker = "## Edge Assessment"
    end_marker = "## Compound Edges"

    start = content.find(start_marker)
    if start == -1:
        return [], []
    end = content.find(end_marker, start)
    section = content[start:end] if end != -1 else content[start:]

    edges: list[dict[str, Any]] = []
    rejected: list[dict[str, Any]] = []
    in_table = False

    for line in section.splitlines():
        line_stripped = line.strip()

        # Detect table start (header row)
        if line_stripped.startswith("| From") or line_stripped.startswith("|From"):
            in_table = True
            continue

        if not in_table:
            continue

        cells = _parse_table_row(line)
        if cells is None:
            # Separator row — skip
            raw_cells = line_stripped.strip("|").split("|")
            if line_stripped.startswith("|") and all(
                re.match(r"^-+$", c.replace(" ", "")) for c in raw_cells if c.strip()
            ):
                continue
            # Non-table line after table started → table ended
            if not line_stripped.startswith("|"):
                break
            continue

        if len(cells) < 2:
            continue

        # Columns: From | To | Type | Weight | Description
        from_axis = cells[0].strip() if len(cells) > 0 else ""
        to_axis = cells[1].strip() if len(cells) > 1 else ""
        edge_type = cells[2].strip().lower() if len(cells) > 2 else ""
        weight_raw = cells[3].strip() if len(cells) > 3 else ""
        description = cells[4].strip() if len(cells) > 4 else ""

        if not from_axis or not to_axis or not edge_type:
            continue

        # Parse weight
        try:
            weight = float(weight_raw)
        except (ValueError, TypeError):
            weight = 0.5

        if edge_type == "none":
            rejected.append(
                {
                    "from_axis": from_axis,
                    "to_axis": to_axis,
                    "rationale": description,
                }
            )
        elif edge_type in _VALID_EDGE_TYPES:
            edges.append(
                {
                    "from_axis": from_axis,
                    "to_axis": to_axis,
                    "edge_type": edge_type,
                    "weight": weight,
                    "description": description,
                    "provenance": "systematic",
                }
            )

    return edges, rejected


# ---------------------------------------------------------------------------
# Compound edge parsing
# ---------------------------------------------------------------------------

# Pattern: COMPOUND: [axis-a, axis-b] -> target | type | description
_COMPOUND_RE = re.compile(
    r"COMPOUND:\s*\[([^\]]+)\]\s*->\s*([^\|]+)\s*\|\s*([^\|]+)\s*\|\s*(.+)",
    re.IGNORECASE,
)


def _parse_compound_edges(content: str) -> list[dict[str, Any]]:
    """Parse COMPOUND: lines from the '## Compound Edges' section."""
    start_marker = "## Compound Edges"
    end_marker = "## Cluster Annotation"

    start = content.find(start_marker)
    if start == -1:
        return []
    end = content.find(end_marker, start)
    section = content[start:end] if end != -1 else content[start:]

    compounds: list[dict[str, Any]] = []
    for line in section.splitlines():
        m = _COMPOUND_RE.match(line.strip())
        if not m:
            continue
        axes_raw, target_raw, edge_type_raw, description = m.groups()
        from_axes = [a.strip() for a in axes_raw.split(",") if a.strip()]
        target = target_raw.strip()
        edge_type = edge_type_raw.strip().lower()
        if edge_type not in _VALID_EDGE_TYPES:
            continue
        compounds.append(
            {
                "type": "compound",
                "from_axes": from_axes,
                "to_axis": target,
                "edge_type": edge_type,
                "description": description.strip(),
                "provenance": "systematic",
            }
        )
    return compounds


# ---------------------------------------------------------------------------
# Cluster annotation parsing
# ---------------------------------------------------------------------------


def _parse_cluster_annotation(content: str) -> str | None:
    """Extract the cluster annotation prose, or None if it's a placeholder."""
    marker = "## Cluster Annotation"
    idx = content.find(marker)
    if idx == -1:
        return None
    section = content[idx + len(marker):].strip()
    # Strip leading HTML comment markers
    if section.startswith("<!--"):
        return None
    if section.startswith("*("):
        return None
    if not section:
        return None
    return section


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def export_all(data_path: Path) -> None:
    """Export all curated edge files to edges.json and edges-rejected.json.

    Args:
        data_path: Root of the storyteller-data repository.
    """
    from rich.console import Console

    console = Console()

    edges_dir = data_path / "narrative-data" / "tome" / "edges"
    manifest_path = edges_dir / "manifest.json"

    if not manifest_path.exists():
        console.print(f"[red]Manifest not found: {manifest_path}[/red]")
        raise SystemExit(1)

    with manifest_path.open() as f:
        manifest = json.load(f)

    entries: dict[str, dict[str, Any]] = manifest.get("entries", {})

    all_edges: list[dict[str, Any]] = []
    all_rejected: list[dict[str, Any]] = []
    all_compounds: list[dict[str, Any]] = []
    cluster_annotations: dict[str, str] = {}

    skipped = 0
    for chunk_key, entry in entries.items():
        filepath = Path(entry["filepath"])
        if not filepath.exists():
            console.print(f"  [yellow]Missing file: {filepath}[/yellow]")
            skipped += 1
            continue

        status = entry.get("status", "pending")
        if status == "pending":
            console.print(f"  [dim]Skipping {chunk_key} (still pending)[/dim]")
            skipped += 1
            continue

        source_domain = entry.get("source_domain", "")
        target_domain = entry.get("target_domain", "")

        content = filepath.read_text()

        edges, rejected = _parse_edge_assessment(content)
        compounds = _parse_compound_edges(content)
        annotation = _parse_cluster_annotation(content)

        # Attach domain info to edges
        for edge in edges:
            edge["from_domain"] = source_domain
            edge["to_domain"] = target_domain

        for edge in rejected:
            edge["from_domain"] = source_domain
            edge["to_domain"] = target_domain

        for compound in compounds:
            compound["from_domains"] = [source_domain] * len(compound["from_axes"])
            compound["to_domain"] = target_domain

        all_edges.extend(edges)
        all_rejected.extend(rejected)
        all_compounds.extend(compounds)

        if annotation:
            cluster_annotations[chunk_key] = annotation

        console.print(
            f"  [green]✓[/green] {chunk_key}: "
            f"{len(edges)} edges, {len(rejected)} rejected, {len(compounds)} compound"
        )

    # Write edges.json
    output = {
        "exported_at": now_iso(),
        "edge_count": len(all_edges),
        "compound_edge_count": len(all_compounds),
        "edges": all_edges,
        "compound_edges": all_compounds,
        "_cluster_annotations": cluster_annotations,
    }
    edges_json_path = data_path / "narrative-data" / "tome" / "edges.json"
    with edges_json_path.open("w") as f:
        json.dump(output, f, indent=2)
        f.write("\n")

    # Write edges-rejected.json
    rejected_output = {
        "exported_at": now_iso(),
        "rejected_count": len(all_rejected),
        "rejected": all_rejected,
    }
    rejected_json_path = data_path / "narrative-data" / "tome" / "edges-rejected.json"
    with rejected_json_path.open("w") as f:
        json.dump(rejected_output, f, indent=2)
        f.write("\n")

    console.print()
    console.print(
        f"[bold green]Exported {len(all_edges)} edges, "
        f"{len(all_compounds)} compound edges, "
        f"{len(all_rejected)} rejected[/bold green]"
    )
    if skipped:
        console.print(f"[yellow]Skipped {skipped} chunk(s) (pending or missing)[/yellow]")
    console.print(f"[cyan]→[/cyan] {edges_json_path}")
    console.print(f"[cyan]→[/cyan] {rejected_json_path}")
