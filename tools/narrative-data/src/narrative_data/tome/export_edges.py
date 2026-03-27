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
    # Skip separator rows (handles both '---' and ':---:' alignment markers)
    if all(re.match(r"^:?-+:?$", c.replace(" ", "")) for c in cells if c):
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

# Fallback pattern: COMPOUND: [axis-a, axis-b] -> target | type | description
_COMPOUND_RE = re.compile(
    r"COMPOUND:\s*\[([^\]]+)\]\s*->\s*([^\|]+)\s*\|\s*([^\|]+)\s*\|\s*(.+)",
    re.IGNORECASE,
)

# Prose pattern: bold header with arrow (→ or ->), optional backticks and
# parenthetical values on axis names. Captures:
#   group 1: source axes (e.g. "axis-a (Val) + axis-b (Val)")
#   group 2: target axis  (e.g. "target-axis (Val)")
#   group 3: optional inline description after ** (colon-separated or empty)
# Handles numbered ("1. **..."), bulleted ("* **..."), or bare ("**...") lines.
_PROSE_HEADER_RE = re.compile(
    r"^(?:\d+\.\s*|\*\s*|-\s*)?"          # optional list prefix
    r"\*\*"                                 # opening bold
    r"(.+?)"                                # source axes + target (greedy-minimal)
    r"\*\*"                                 # closing bold
    r"[:\s]*(.*)?$",                        # optional inline description
)

# Arrow pattern to split source axes from target within the bold header.
# Matches → (unicode), -> (ascii), or the rare ⟶.
_ARROW_RE = re.compile(r"\s*(?:→|->|⟶)\s*")

_DEFAULT_COMPOUND_WEIGHT = 0.7


def _strip_backticks(s: str) -> str:
    """Remove surrounding backticks from an axis name."""
    return s.strip().strip("`").strip()


def _strip_parenthetical(s: str) -> str:
    """Remove trailing parenthetical value hints like '(Hereditary)'.

    Keeps the bare axis slug: 'authority-legitimation (Hereditary)' -> 'authority-legitimation'
    """
    return re.sub(r"\s*\([^)]*\)\s*$", "", s).strip()


def _clean_axis_name(raw: str) -> str:
    """Normalize an axis name: strip parentheticals, backticks, whitespace.

    Order matters: parentheticals are stripped first so that backticks at the
    boundary (e.g. '`axis-name` (Value)') are exposed for removal.
    """
    return _strip_backticks(_strip_parenthetical(raw))


def _infer_edge_type(text: str) -> str:
    """Infer edge type from description text by keyword search.

    Matches verb stems to handle conjugation (e.g. 'constrain' matches
    'constrains'). Returns the first matching type from _VALID_EDGE_TYPES,
    or 'produces' as the safest default.
    """
    lower = text.lower()
    # Check in priority order: more specific stems first.
    # Each tuple is (stem_to_search, canonical_edge_type).
    for stem, edge_type in (
        ("transform", "transforms"),
        ("constrain", "constrains"),
        ("enabl", "enables"),
        ("produc", "produces"),
    ):
        if stem in lower:
            return edge_type
    return "produces"


def _parse_compound_table(section: str) -> list[dict[str, Any]]:
    """Parse compound edges from a markdown table format.

    Handles varying column headers:
      Source Axes / From | Target Axis / Target / To | Type | Weight | Description
    Source axes are joined with '+' in the cell.
    """
    compounds: list[dict[str, Any]] = []
    in_table = False

    for line in section.splitlines():
        stripped = line.strip()

        # Detect table header row — look for a pipe-delimited line with
        # keywords that indicate a compound table (has "Source", "From", etc.)
        if not in_table:
            if stripped.startswith("|") and (
                "Source" in stripped
                or "From" in stripped
                or "Target" in stripped
                or "To" in stripped
            ):
                in_table = True
            continue

        cells = _parse_table_row(stripped)
        if cells is None:
            # Separator row or non-table line
            if stripped.startswith("|"):
                continue
            # Non-table line after table → table ended
            break

        if len(cells) < 2:
            continue

        # Columns: SourceAxes | Target | Type | Weight | Description
        source_raw = cells[0].strip()
        target_raw = cells[1].strip()
        edge_type = cells[2].strip().lower() if len(cells) > 2 else ""
        weight_raw = cells[3].strip() if len(cells) > 3 else ""
        description = cells[4].strip() if len(cells) > 4 else ""

        if not source_raw or not target_raw:
            continue

        # Split source axes on '+'
        from_axes = [_clean_axis_name(a) for a in source_raw.split("+") if a.strip()]
        to_axis = _clean_axis_name(target_raw)

        if not from_axes or not to_axis:
            continue

        # Validate or infer edge type
        if edge_type not in _VALID_EDGE_TYPES:
            edge_type = _infer_edge_type(description)

        # Parse weight
        try:
            weight = float(weight_raw)
        except (ValueError, TypeError):
            weight = _DEFAULT_COMPOUND_WEIGHT

        compounds.append(
            {
                "type": "compound",
                "from_axes": from_axes,
                "to_axis": to_axis,
                "edge_type": edge_type,
                "weight": weight,
                "description": description,
                "provenance": "systematic",
            }
        )

    return compounds


def _parse_compound_prose(section: str) -> list[dict[str, Any]]:
    """Parse compound edges from prose formats (numbered lists or bold paragraphs).

    Handles two sub-formats:
      - Numbered/bulleted: '1. **axis-a + axis-b → target**: description...'
      - Bold paragraphs:   '**axis-a + axis-b → target**\\ndescription on next line'

    Continuation lines (starting with whitespace or '*') are appended to the
    description of the current edge.
    """
    compounds: list[dict[str, Any]] = []
    lines = section.splitlines()
    i = 0

    while i < len(lines):
        line = lines[i].strip()
        i += 1

        m = _PROSE_HEADER_RE.match(line)
        if not m:
            continue

        header_body = m.group(1).strip()
        inline_desc = (m.group(2) or "").strip()

        # Split header on arrow to get source axes and target
        arrow_parts = _ARROW_RE.split(header_body)
        if len(arrow_parts) < 2:
            continue

        source_part = arrow_parts[0].strip()
        target_part = arrow_parts[-1].strip()

        # Split source axes on '+'
        from_axes = [_clean_axis_name(a) for a in source_part.split("+") if a.strip()]
        to_axis = _clean_axis_name(target_part)

        if not from_axes or not to_axis:
            continue

        # Gather description: inline portion + continuation lines
        desc_parts: list[str] = []
        if inline_desc:
            desc_parts.append(inline_desc)

        # Collect continuation lines (indented, italic, or plain prose that
        # isn't a new bold header, list item, or section heading)
        while i < len(lines):
            next_line = lines[i]
            next_stripped = next_line.strip()
            # Stop at: blank line, new bold header, numbered item, section heading, table row
            if not next_stripped:
                break
            if _PROSE_HEADER_RE.match(next_stripped):
                break
            if re.match(r"^\d+\.\s+\*\*", next_stripped):
                break
            if next_stripped.startswith("## "):
                break
            if next_stripped.startswith("|"):
                break
            # Accept italic continuation (*text*) or plain text
            desc_parts.append(next_stripped.strip("*").strip())
            i += 1

        description = " ".join(desc_parts).strip()

        edge_type = _infer_edge_type(description)

        compounds.append(
            {
                "type": "compound",
                "from_axes": from_axes,
                "to_axis": to_axis,
                "edge_type": edge_type,
                "weight": _DEFAULT_COMPOUND_WEIGHT,
                "description": description,
                "provenance": "systematic",
            }
        )

    return compounds


def _parse_compound_edges(content: str) -> list[dict[str, Any]]:
    """Parse compound edges from the '## Compound Edges' section.

    Supports three LLM-produced formats plus the original COMPOUND: fallback:
      1. Markdown table (pipe-delimited, with Source/From and Target/To columns)
      2. Numbered/bulleted prose with bold header and arrow
      3. Bold prose paragraphs with arrow, description on following line(s)

    For formats without explicit type/weight:
      - Type is inferred from keywords in the description text
      - Weight defaults to 0.7
    """
    start_marker = "## Compound Edges"
    end_marker = "## Cluster Annotation"

    start = content.find(start_marker)
    if start == -1:
        return []
    end = content.find(end_marker, start)
    section = content[start:end] if end != -1 else content[start:]

    compounds: list[dict[str, Any]] = []

    # Strategy 1: Try COMPOUND: lines (original format)
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
                "weight": _DEFAULT_COMPOUND_WEIGHT,
                "description": description.strip(),
                "provenance": "systematic",
            }
        )

    if compounds:
        return compounds

    # Strategy 2: Try markdown table format
    compounds = _parse_compound_table(section)
    if compounds:
        return compounds

    # Strategy 3: Try prose formats (numbered lists or bold paragraphs)
    compounds = _parse_compound_prose(section)
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
