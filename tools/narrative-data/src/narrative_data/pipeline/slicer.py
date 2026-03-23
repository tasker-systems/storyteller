# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Deterministic markdown slicer for genre region and other structured files.

Splits markdown files into focused segments based on heading levels and bold
dimension group markers. No LLM involved — pure structural parsing.
"""

import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SegmentInfo:
    """Metadata about a produced segment."""

    name: str  # e.g., "aesthetic", "tonal", "meta"
    path: Path  # full path to segment file
    source_path: Path  # path to source .md file
    line_start: int  # 1-indexed line number in source
    line_end: int  # 1-indexed line number in source


# Bold dimension group headers that appear within Dimensional Positions.
# Maps pattern (case-insensitive) to segment name.
_DIMENSION_GROUPS: list[tuple[re.Pattern[str], str]] = [
    (re.compile(r"^\*\*aesthetic\s+dimensions?\*\*", re.IGNORECASE), "aesthetic"),
    (re.compile(r"^\*\*tonal\s+dimensions?\*\*", re.IGNORECASE), "tonal"),
    (re.compile(r"^\*\*temporal\s+dimensions?\*\*", re.IGNORECASE), "temporal"),
    (re.compile(r"^\*\*thematic\s+dimensions?\*\*", re.IGNORECASE), "thematic"),
    (re.compile(r"^\*\*agency\s+dimensions?\*\*", re.IGNORECASE), "agency"),
    (re.compile(r"^\*\*locus\s+of\s+power\*\*", re.IGNORECASE), "locus-of-power"),
    (
        re.compile(r"^\*\*(structural|narrative\s+structure)\s+dimensions?\*\*", re.IGNORECASE),
        "narrative-structure",
    ),
    (re.compile(r"^\*\*world\s+affordances?\*\*", re.IGNORECASE), "world-affordances"),
    (re.compile(r"^\*\*epistemological\s+stance?\*\*", re.IGNORECASE), "epistemological"),
]

# H2 section patterns for non-dimensional content.
_H2_EXCLUSIONS = re.compile(r"^##\s+\d*\.?\s*exclusions?", re.IGNORECASE)
_H2_STATE_VARS = re.compile(r"^##\s+\d*\.?\s*state\s+variables?", re.IGNORECASE)
_H2_TOPOLOGY = re.compile(r"^##\s+\d*\.?\s*genre\s+topology", re.IGNORECASE)
_H2_BOUNDARY = re.compile(r"^##\s+\d*\.?\s*boundary\s+conditions?", re.IGNORECASE)
_H2_EPISTEMOLOGICAL = re.compile(r"^##\s+\d*\.?\s*\d*\.?\s*epistemological", re.IGNORECASE)
_H2_WORLD_AFFORDANCES = re.compile(r"^##\s+\d*\.?\s*world\s+affordances?", re.IGNORECASE)

# Commentary / suggestions markers — content after these is dropped.
_TAIL_MARKERS = re.compile(
    r"^(#{2,3}\s+(_?commentary|_?suggestions|commentary\s+and\s+suggestions))",
    re.IGNORECASE,
)


def _file_hash(path: Path) -> str:
    """SHA-256 hex digest of file contents."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _extract_genre_name(title_line: str) -> str:
    """Extract genre name from '# Genre Region: Foo Bar'."""
    m = re.match(r"^#\s+Genre\s+Region:\s*(.+)", title_line.strip(), re.IGNORECASE)
    return m.group(1).strip() if m else "Unknown"


def _write_segment(
    output_dir: Path,
    name: str,
    lines: list[str],
    source_rel: str,
    line_start: int,
    line_end: int,
    *,
    genre_name: str | None = None,
) -> Path:
    """Write a segment file with YAML frontmatter."""
    path = output_dir / f"segment-{name}.md"
    front = f"---\nsource: {source_rel}\nsegment: {name}\nlines: {line_start}-{line_end}\n"
    if genre_name is not None:
        front += f"genre_name: {genre_name}\n"
    front += "---\n\n"
    path.write_text(front + "\n".join(lines))
    return path


def _match_dimension_group(line: str) -> str | None:
    """Return segment name if line matches a known dimension group header."""
    stripped = line.strip()
    for pattern, name in _DIMENSION_GROUPS:
        if pattern.match(stripped):
            return name
    return None


def _classify_h2(stripped: str) -> str:
    """Classify an H2 heading into a section type string."""
    if _H2_EXCLUSIONS.match(stripped):
        return "exclusions"
    if _H2_STATE_VARS.match(stripped):
        return "state-variables"
    if _H2_TOPOLOGY.match(stripped):
        return "topology"
    if _H2_BOUNDARY.match(stripped):
        return "boundary"
    if _H2_EPISTEMOLOGICAL.match(stripped):
        return "epistemological-h2"
    if _H2_WORLD_AFFORDANCES.match(stripped):
        return "world-affordances-h2"
    return "other"


def slice_genre_region(
    source_path: Path,
    output_dir: Path,
    *,
    force: bool = False,
    source_rel: str | None = None,
) -> list[SegmentInfo]:
    """Slice a genre region markdown file into focused segments.

    Args:
        source_path: Path to the genre region .md file.
        output_dir: Directory to write segment files into.
        force: If True, re-slice even if manifest is fresh.
        source_rel: Relative path to use in frontmatter (defaults to source_path.name).

    Returns:
        List of SegmentInfo describing each produced segment.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "segments-manifest.json"
    current_hash = _file_hash(source_path)

    if source_rel is None:
        source_rel = source_path.name

    # Staleness check
    if not force and manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
        if manifest.get("source_hash") == current_hash:
            return _reconstruct_from_manifest(manifest, output_dir, source_path)

    raw_lines = source_path.read_text().splitlines()

    # Phase 1: Find end-of-content (commentary/suggestions)
    content_end = len(raw_lines)
    for i, line in enumerate(raw_lines):
        if _TAIL_MARKERS.match(line.strip()):
            end = i
            while end > 0 and raw_lines[end - 1].strip() in ("", "---", "***"):
                end -= 1
            content_end = end
            break

    # Phase 2: Identify structural elements
    h1_line = _find_h1(raw_lines, content_end)
    h2_sections = _find_h2_sections(raw_lines, content_end)
    dim_groups = _find_dimension_groups(raw_lines, content_end)

    # Phase 3: Build segments
    segments: list[SegmentInfo] = []

    _emit_meta(segments, raw_lines, h1_line, output_dir, source_rel, source_path)
    _emit_dimension_groups(
        segments,
        raw_lines,
        dim_groups,
        h2_sections,
        content_end,
        output_dir,
        source_rel,
        source_path,
    )
    _emit_h2_sections(
        segments,
        raw_lines,
        h2_sections,
        content_end,
        output_dir,
        source_rel,
        source_path,
    )

    # Write manifest
    manifest = {
        "source_hash": current_hash,
        "source_path": source_rel,
        "segments": {s.name: s.path.name for s in segments},
        "line_ranges": {s.name: [s.line_start, s.line_end] for s in segments},
    }
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")

    return segments


def _reconstruct_from_manifest(
    manifest: dict,
    output_dir: Path,
    source_path: Path,
) -> list[SegmentInfo]:
    """Reconstruct SegmentInfo list from a cached manifest."""
    return [
        SegmentInfo(
            name=name,
            path=output_dir / filename,
            source_path=source_path,
            line_start=manifest.get("line_ranges", {}).get(name, [1, 1])[0],
            line_end=manifest.get("line_ranges", {}).get(name, [1, 1])[1],
        )
        for name, filename in manifest["segments"].items()
    ]


def _find_h1(raw_lines: list[str], content_end: int) -> int:
    """Find the H1 line index, or -1 if not found."""
    for i, line in enumerate(raw_lines[:content_end]):
        if line.startswith("# ") and not line.startswith("## "):
            return i
    return -1


def _find_h2_sections(raw_lines: list[str], content_end: int) -> list[tuple[int, str, str]]:
    """Find all H2 sections as (line_idx, raw_heading, section_type)."""
    sections: list[tuple[int, str, str]] = []
    for i, line in enumerate(raw_lines[:content_end]):
        stripped = line.strip()
        if stripped.startswith("## "):
            sections.append((i, stripped, _classify_h2(stripped)))
    return sections


def _find_dimension_groups(raw_lines: list[str], content_end: int) -> list[tuple[int, str]]:
    """Find bold dimension group headers as (line_idx, segment_name)."""
    groups: list[tuple[int, str]] = []
    for i, line in enumerate(raw_lines[:content_end]):
        stripped = line.strip()
        if not stripped.startswith("## "):
            seg_name = _match_dimension_group(stripped)
            if seg_name is not None:
                groups.append((i, seg_name))
    return groups


def _emit_meta(
    segments: list[SegmentInfo],
    raw_lines: list[str],
    h1_line: int,
    output_dir: Path,
    source_rel: str,
    source_path: Path,
) -> None:
    """Emit the meta segment from the H1 title line."""
    if h1_line < 0:
        return
    genre_name = _extract_genre_name(raw_lines[h1_line])
    meta_path = _write_segment(
        output_dir,
        "meta",
        [raw_lines[h1_line]],
        source_rel,
        h1_line + 1,
        h1_line + 1,
        genre_name=genre_name,
    )
    segments.append(
        SegmentInfo(
            name="meta",
            path=meta_path,
            source_path=source_path,
            line_start=h1_line + 1,
            line_end=h1_line + 1,
        )
    )


def _emit_dimension_groups(
    segments: list[SegmentInfo],
    raw_lines: list[str],
    dim_groups: list[tuple[int, str]],
    h2_sections: list[tuple[int, str, str]],
    content_end: int,
    output_dir: Path,
    source_rel: str,
    source_path: Path,
) -> None:
    """Emit segments for each bold dimension group."""
    # Build sorted boundary list for finding where each group ends
    all_boundaries = sorted(
        [(idx, name) for idx, name in dim_groups] + [(idx, stype) for idx, _, stype in h2_sections]
    )

    for line_idx, seg_name in dim_groups:
        # Find the next boundary after this dim group
        boundary_idx = content_end
        for b_line, _ in all_boundaries:
            if b_line > line_idx:
                boundary_idx = b_line
                break

        # Trim trailing blank lines
        end_idx = boundary_idx
        while end_idx > line_idx + 1 and raw_lines[end_idx - 1].strip() == "":
            end_idx -= 1

        block_lines = raw_lines[line_idx:end_idx]
        seg_path = _write_segment(
            output_dir,
            seg_name,
            block_lines,
            source_rel,
            line_idx + 1,
            end_idx,
        )
        segments.append(
            SegmentInfo(
                name=seg_name,
                path=seg_path,
                source_path=source_path,
                line_start=line_idx + 1,
                line_end=end_idx,
            )
        )


def _emit_h2_sections(
    segments: list[SegmentInfo],
    raw_lines: list[str],
    h2_sections: list[tuple[int, str, str]],
    content_end: int,
    output_dir: Path,
    source_rel: str,
    source_path: Path,
) -> None:
    """Emit segments for H2-level sections (exclusions, state vars, etc.)."""
    for h2_idx, (line_idx, _heading, section_type) in enumerate(h2_sections):
        if section_type == "other":
            continue

        # Find end: next H2 or content_end
        next_h2_line = content_end
        for next_idx, _, _ in h2_sections[h2_idx + 1 :]:
            next_h2_line = next_idx
            break
        section_end = min(next_h2_line, content_end)

        # Trim trailing blank lines and horizontal rules
        while section_end > line_idx + 1 and raw_lines[section_end - 1].strip() in (
            "",
            "---",
            "***",
        ):
            section_end -= 1

        block_lines = raw_lines[line_idx:section_end]

        if section_type == "exclusions":
            _emit_narrative_contracts(
                segments,
                output_dir,
                block_lines,
                source_rel,
                source_path,
                line_idx,
                section_end,
            )
        elif section_type == "state-variables":
            _emit_simple_segment(
                segments,
                output_dir,
                "state-variables",
                block_lines,
                source_rel,
                source_path,
                line_idx,
                section_end,
            )
        elif section_type in ("topology", "boundary"):
            _emit_or_merge_boundaries(
                segments,
                output_dir,
                block_lines,
                source_rel,
                source_path,
                line_idx,
                section_end,
            )
        elif section_type == "epistemological-h2":
            _emit_simple_segment(
                segments,
                output_dir,
                "epistemological",
                block_lines,
                source_rel,
                source_path,
                line_idx,
                section_end,
            )
        elif section_type == "world-affordances-h2":
            _emit_simple_segment(
                segments,
                output_dir,
                "world-affordances",
                block_lines,
                source_rel,
                source_path,
                line_idx,
                section_end,
            )


def _emit_simple_segment(
    segments: list[SegmentInfo],
    output_dir: Path,
    name: str,
    block_lines: list[str],
    source_rel: str,
    source_path: Path,
    line_idx: int,
    section_end: int,
) -> None:
    """Emit a single named segment."""
    seg_path = _write_segment(
        output_dir,
        name,
        block_lines,
        source_rel,
        line_idx + 1,
        section_end,
    )
    segments.append(
        SegmentInfo(
            name=name,
            path=seg_path,
            source_path=source_path,
            line_start=line_idx + 1,
            line_end=section_end,
        )
    )


def _emit_or_merge_boundaries(
    segments: list[SegmentInfo],
    output_dir: Path,
    block_lines: list[str],
    source_rel: str,
    source_path: Path,
    line_idx: int,
    section_end: int,
) -> None:
    """Emit or merge into the 'boundaries' segment (topology + boundary conditions)."""
    existing = next((s for s in segments if s.name == "boundaries"), None)
    if existing is not None:
        old_content = existing.path.read_text()
        parts = old_content.split("---\n", 2)
        old_body = parts[2] if len(parts) >= 3 else old_content
        merged_lines = old_body.strip().split("\n") + [""] + block_lines
        seg_path = _write_segment(
            output_dir,
            "boundaries",
            merged_lines,
            source_rel,
            existing.line_start,
            section_end,
        )
        existing.path = seg_path
        existing.line_end = section_end
    else:
        seg_path = _write_segment(
            output_dir,
            "boundaries",
            block_lines,
            source_rel,
            line_idx + 1,
            section_end,
        )
        segments.append(
            SegmentInfo(
                name="boundaries",
                path=seg_path,
                source_path=source_path,
                line_start=line_idx + 1,
                line_end=section_end,
            )
        )


def _emit_narrative_contracts(
    segments: list[SegmentInfo],
    output_dir: Path,
    block_lines: list[str],
    source_rel: str,
    source_path: Path,
    section_start: int,
    section_end: int,
) -> None:
    """Emit the narrative-contracts segment from the Exclusions section.

    The entire Exclusions section (contract violations, forbidden tropes,
    what breaks the genre contract) maps to narrative-contracts.
    """
    seg_path = _write_segment(
        output_dir,
        "narrative-contracts",
        block_lines,
        source_rel,
        section_start + 1,
        section_end,
    )
    segments.append(
        SegmentInfo(
            name="narrative-contracts",
            path=seg_path,
            source_path=source_path,
            line_start=section_start + 1,
            line_end=section_end,
        )
    )


# ---------------------------------------------------------------------------
# Heading-based slug generation
# ---------------------------------------------------------------------------

_HEADING_PREFIX = re.compile(r"^#{1,6}\s+\d+\.\s*")


def _heading_to_slug(heading_line: str) -> str:
    """Convert a numbered heading line to a kebab-case slug.

    Strips the leading hashes and number prefix, then lowercases and
    replaces spaces with hyphens.

    Examples:
        "#### 1. The Unwilling Vessel" → "the-unwilling-vessel"
        "## 2. The Spiral of Diminishing Certainty" → "the-spiral-of-diminishing-certainty"
    """
    text = heading_line.strip()
    # Remove heading markers and number prefix
    text = _HEADING_PREFIX.sub("", text)
    text = text.strip()
    # Lowercase
    text = text.lower()
    # Replace spaces with hyphens; strip non-alphanumeric except hyphens
    text = re.sub(r"\s+", "-", text)
    text = re.sub(r"[^a-z0-9\-]", "", text)
    # Collapse multiple hyphens
    text = re.sub(r"-{2,}", "-", text)
    return text.strip("-")


# ---------------------------------------------------------------------------
# Heading-level auto-detection
# ---------------------------------------------------------------------------

_ANY_NUMBERED_HEADING = re.compile(r"^(#{1,6})\s+\d+\.\s+")


def _detect_heading_level(source_path: Path) -> int:
    """Scan a file for the first numbered heading and return its level.

    Discovery and cluster files use inconsistent heading levels across the
    corpus (H2, H3, or H4 for entity entries). This function finds the first
    numbered heading (e.g. ``## 1.``, ``### 1.``, ``#### 1.``) and returns its
    level so the slicer can adapt.

    Falls back to 4 if no numbered heading is found.
    """
    for line in source_path.read_text().splitlines():
        m = _ANY_NUMBERED_HEADING.match(line.strip())
        if m:
            return len(m.group(1))
    return 4


# ---------------------------------------------------------------------------
# Generic heading-level slicer (used by discovery, cluster, genre-native)
# ---------------------------------------------------------------------------

_HEADING_PATTERNS: dict[int, re.Pattern[str]] = {
    level: re.compile(r"^" + "#" * level + r"\s+\d+\.\s+") for level in range(1, 7)
}


def _slice_on_heading_level(
    source_path: Path,
    output_dir: Path,
    heading_level: int,
    force: bool = False,
    source_rel: str | None = None,
) -> list[SegmentInfo]:
    """Internal: split a file on numbered headings at the given level (1–6).

    Commentary / suggestions sections (### _commentary, ### _suggestions, etc.)
    are dropped before splitting.

    Args:
        source_path: Path to the source .md file.
        output_dir: Directory to write segment files into.
        heading_level: Number of # characters to match (1 = H1, 2 = H2, …).
        force: Re-slice even if manifest is fresh.
        source_rel: Relative path to use in frontmatter.

    Returns:
        List of SegmentInfo for each entity segment produced.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "segments-manifest.json"
    current_hash = _file_hash(source_path)

    if source_rel is None:
        source_rel = source_path.name

    # Staleness check
    if not force and manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
        if manifest.get("source_hash") == current_hash:
            return _reconstruct_from_manifest(manifest, output_dir, source_path)

    raw_lines = source_path.read_text().splitlines()
    heading_pattern = _HEADING_PATTERNS[heading_level]

    # Find end-of-content (drop commentary/suggestions)
    content_end = len(raw_lines)
    for i, line in enumerate(raw_lines):
        if _TAIL_MARKERS.match(line.strip()):
            end = i
            while end > 0 and raw_lines[end - 1].strip() in ("", "---", "***"):
                end -= 1
            content_end = end
            break

    # Find all numbered heading positions within content
    heading_positions: list[tuple[int, str]] = []
    for i, line in enumerate(raw_lines[:content_end]):
        stripped = line.strip()
        if heading_pattern.match(stripped):
            heading_positions.append((i, stripped))

    segments: list[SegmentInfo] = []

    for pos_idx, (line_idx, heading_text) in enumerate(heading_positions):
        slug = _heading_to_slug(heading_text)

        # End of this entity: start of next heading or content_end
        if pos_idx + 1 < len(heading_positions):
            raw_end = heading_positions[pos_idx + 1][0]
        else:
            raw_end = content_end

        # Trim trailing blank lines and horizontal rules
        end_idx = raw_end
        while end_idx > line_idx + 1 and raw_lines[end_idx - 1].strip() in (
            "",
            "---",
            "***",
        ):
            end_idx -= 1

        block_lines = raw_lines[line_idx:end_idx]
        seg_path = _write_segment(
            output_dir,
            slug,
            block_lines,
            source_rel,
            line_idx + 1,
            end_idx,
        )
        segments.append(
            SegmentInfo(
                name=slug,
                path=seg_path,
                source_path=source_path,
                line_start=line_idx + 1,
                line_end=end_idx,
            )
        )

    # Write manifest
    manifest: dict = {
        "source_hash": current_hash,
        "source_path": source_rel,
        "segments": {s.name: s.path.name for s in segments},
        "line_ranges": {s.name: [s.line_start, s.line_end] for s in segments},
    }
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")

    return segments


# ---------------------------------------------------------------------------
# Public API: per-document-type slicers
# ---------------------------------------------------------------------------


def slice_discovery(
    source_path: Path,
    output_dir: Path,
    force: bool = False,
) -> list[SegmentInfo]:
    """Split a discovery per-genre file on numbered entity headers.

    The heading level is auto-detected from the first numbered heading in the
    file (H2, H3, or H4 depending on how the 35b model structured its output).
    Commentary and suggestions sections are dropped automatically.

    Args:
        source_path: Path to the discovery .md file (e.g.
            ``discovery/archetypes/folk-horror.md``).
        output_dir: Directory to write segment files into.
        force: If True, re-slice even if the manifest is fresh.

    Returns:
        List of SegmentInfo for each entity segment produced.
    """
    level = _detect_heading_level(source_path)
    return _slice_on_heading_level(source_path, output_dir, heading_level=level, force=force)


def slice_cluster(
    source_path: Path,
    output_dir: Path,
    force: bool = False,
) -> list[SegmentInfo]:
    """Split a cluster synthesis file on numbered entity headers.

    The heading level is auto-detected from the first numbered heading in the
    file. Cluster files aggregate genre variants into canonical entries.

    Args:
        source_path: Path to the cluster .md file (e.g.
            ``discovery/archetypes/cluster-horror.md``).
        output_dir: Directory to write segment files into.
        force: If True, re-slice even if the manifest is fresh.

    Returns:
        List of SegmentInfo for each cluster entity segment produced.
    """
    level = _detect_heading_level(source_path)
    return _slice_on_heading_level(source_path, output_dir, heading_level=level, force=force)


def slice_genre_native(
    source_path: Path,
    output_dir: Path,
    heading_level: int,
    force: bool = False,
) -> list[SegmentInfo]:
    """Split a genre-native file on numbered entity headings.

    The ``heading_level`` parameter is used as a fallback; the actual level
    is auto-detected from the first numbered heading in the file to handle
    inconsistent heading levels across the 35b-generated corpus.

    Args:
        source_path: Path to the genre-native .md file.
        output_dir: Directory to write segment files into.
        heading_level: Fallback heading depth if auto-detection finds nothing.
        force: If True, re-slice even if the manifest is fresh.

    Returns:
        List of SegmentInfo for each entity segment produced.
    """
    detected = _detect_heading_level(source_path)
    # _detect_heading_level returns 4 as default when nothing found;
    # use the caller's fallback if detection didn't find anything
    level = detected if detected != 4 else heading_level
    return _slice_on_heading_level(
        source_path, output_dir, heading_level=level, force=force
    )


def slice_file(
    source_path: Path,
    output_dir: Path,
    doc_type: str,
    force: bool = False,
) -> list[SegmentInfo]:
    """Dispatch to the appropriate slicer based on document type.

    Args:
        source_path: Path to the source .md file.
        output_dir: Directory to write segment files into.
        doc_type: One of ``"genre-region"``, ``"discovery"``, ``"cluster"``,
            ``"narrative-shapes"``, ``"tropes"``.
        force: If True, re-slice even if the manifest is fresh.

    Returns:
        List of SegmentInfo for each segment produced.

    Raises:
        ValueError: If ``doc_type`` is not a known type.
    """
    if doc_type == "genre-region":
        return slice_genre_region(source_path, output_dir, force=force)
    if doc_type == "discovery":
        return slice_discovery(source_path, output_dir, force=force)
    if doc_type == "cluster":
        return slice_cluster(source_path, output_dir, force=force)
    if doc_type == "narrative-shapes":
        return slice_genre_native(source_path, output_dir, heading_level=2, force=force)
    if doc_type == "tropes":
        return slice_genre_native(source_path, output_dir, heading_level=4, force=force)
    raise ValueError(f"Unknown doc_type: {doc_type!r}")
