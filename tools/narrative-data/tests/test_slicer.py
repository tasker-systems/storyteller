# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for genre region slicer and heading-level slicers."""

import json
from pathlib import Path

import pytest

from narrative_data.pipeline.slicer import (
    SegmentInfo,
    slice_cluster,
    slice_discovery,
    slice_file,
    slice_genre_native,
    slice_genre_region,
)

FIXTURE = Path(__file__).parent / "fixtures" / "sample-region.md"

EXPECTED_SEGMENTS = [
    "meta",
    "aesthetic",
    "tonal",
    "temporal",
    "thematic",
    "agency",
    "locus-of-power",
    "narrative-structure",
    "world-affordances",
    "epistemological",
    "narrative-contracts",
    "state-variables",
    "boundaries",
]


class TestSliceGenreRegion:
    def test_produces_expected_segments(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        names = [s.name for s in segments]
        for expected in EXPECTED_SEGMENTS:
            assert expected in names, f"Missing segment: {expected}"

    def test_segment_count(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        assert len(segments) == 13

    def test_segment_files_exist(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        for seg in segments:
            assert seg.path.exists(), f"Missing file: {seg.path}"
            content = seg.path.read_text()
            assert len(content) > 0, f"Empty file: {seg.path}"

    def test_frontmatter_present(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        for seg in segments:
            content = seg.path.read_text()
            assert content.startswith("---\n"), f"No frontmatter in {seg.name}"
            # Check required fields
            assert "source:" in content
            assert "segment:" in content
            assert "lines:" in content

    def test_commentary_dropped(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        names = [s.name for s in segments]
        assert "commentary" not in names
        assert "suggestions" not in names
        for seg in segments:
            content = seg.path.read_text()
            assert "_commentary" not in content, f"_commentary found in {seg.name}"
            assert "_suggestions" not in content, f"_suggestions found in {seg.name}"

    def test_idempotent(self, tmp_path: Path) -> None:
        segments1 = slice_genre_region(FIXTURE, tmp_path, force=True)
        segments2 = slice_genre_region(FIXTURE, tmp_path, force=True)
        for s1, s2 in zip(segments1, segments2, strict=True):
            assert s1.name == s2.name
            assert s1.path.read_text() == s2.path.read_text()

    def test_skips_when_fresh(self, tmp_path: Path) -> None:
        segments1 = slice_genre_region(FIXTURE, tmp_path)
        # Modify a segment file to detect re-slicing
        marker = "\n<!-- modified -->\n"
        segments1[0].path.write_text(segments1[0].path.read_text() + marker)
        segments2 = slice_genre_region(FIXTURE, tmp_path)
        # Should have returned cached result — the marker file should still be modified
        assert marker in segments2[0].path.read_text()

    def test_force_reslices(self, tmp_path: Path) -> None:
        segments1 = slice_genre_region(FIXTURE, tmp_path)
        marker = "\n<!-- modified -->\n"
        segments1[0].path.write_text(segments1[0].path.read_text() + marker)
        segments2 = slice_genre_region(FIXTURE, tmp_path, force=True)
        # Force should overwrite — marker gone
        assert marker not in segments2[0].path.read_text()

    def test_meta_segment_has_genre_name(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        meta = next(s for s in segments if s.name == "meta")
        content = meta.path.read_text()
        assert "genre_name: Test Fantasy" in content

    def test_segment_info_fields(self, tmp_path: Path) -> None:
        segments = slice_genre_region(FIXTURE, tmp_path)
        for seg in segments:
            assert isinstance(seg, SegmentInfo)
            assert isinstance(seg.name, str)
            assert isinstance(seg.path, Path)
            assert isinstance(seg.source_path, Path)
            assert isinstance(seg.line_start, int)
            assert isinstance(seg.line_end, int)
            assert seg.line_start >= 1
            assert seg.line_end >= seg.line_start

    def test_manifest_written(self, tmp_path: Path) -> None:
        slice_genre_region(FIXTURE, tmp_path)
        manifest_path = tmp_path / "segments-manifest.json"
        assert manifest_path.exists()
        manifest = json.loads(manifest_path.read_text())
        assert "source_hash" in manifest
        assert "source_path" in manifest
        assert "segments" in manifest
        assert len(manifest["segments"]) == 13

    def test_no_overlapping_lines(self, tmp_path: Path) -> None:
        """Segments should not have overlapping line ranges."""
        segments = slice_genre_region(FIXTURE, tmp_path)
        # Sort by line_start, excluding meta which is just line 1
        non_meta = sorted(
            [s for s in segments if s.name != "meta"],
            key=lambda s: s.line_start,
        )
        for i in range(len(non_meta) - 1):
            assert non_meta[i].line_end < non_meta[i + 1].line_start, (
                f"Overlap: {non_meta[i].name} ends at {non_meta[i].line_end}, "
                f"{non_meta[i + 1].name} starts at {non_meta[i + 1].line_start}"
            )


# ---------------------------------------------------------------------------
# Fixtures for new slicers
# ---------------------------------------------------------------------------

_FIXTURES = Path(__file__).parent / "fixtures"
ARCHETYPES_FIXTURE = _FIXTURES / "sample-archetypes.md"
CLUSTER_FIXTURE = _FIXTURES / "sample-cluster-archetypes.md"
SHAPES_FIXTURE = _FIXTURES / "sample-narrative-shapes.md"


# ---------------------------------------------------------------------------
# TestSliceDiscovery
# ---------------------------------------------------------------------------


class TestSliceDiscovery:
    def test_splits_on_h4_headers(self, tmp_path: Path) -> None:
        segments = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        assert len(segments) == 3

    def test_slug_generation(self, tmp_path: Path) -> None:
        segments = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        names = [s.name for s in segments]
        assert "the-guardian" in names
        assert "the-seeker" in names
        assert "the-shadow" in names

    def test_drops_commentary(self, tmp_path: Path) -> None:
        segments = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        for seg in segments:
            content = seg.path.read_text()
            assert "_commentary" not in content, f"_commentary in {seg.name}"
            assert "_suggestions" not in content, f"_suggestions in {seg.name}"

    def test_frontmatter(self, tmp_path: Path) -> None:
        segments = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        for seg in segments:
            content = seg.path.read_text()
            assert content.startswith("---\n"), f"No frontmatter in {seg.name}"
            assert "source:" in content
            assert "segment:" in content
            assert "lines:" in content

    def test_manifest(self, tmp_path: Path) -> None:
        slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        manifest_path = tmp_path / "segments-manifest.json"
        assert manifest_path.exists()
        manifest = json.loads(manifest_path.read_text())
        assert "source_hash" in manifest
        assert "source_path" in manifest
        assert "segments" in manifest
        assert len(manifest["segments"]) == 3

    def test_segment_info_fields(self, tmp_path: Path) -> None:
        segments = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        for seg in segments:
            assert isinstance(seg, SegmentInfo)
            assert seg.line_start >= 1
            assert seg.line_end >= seg.line_start

    def test_idempotent(self, tmp_path: Path) -> None:
        segments1 = slice_discovery(ARCHETYPES_FIXTURE, tmp_path, force=True)
        segments2 = slice_discovery(ARCHETYPES_FIXTURE, tmp_path, force=True)
        for s1, s2 in zip(segments1, segments2, strict=True):
            assert s1.name == s2.name
            assert s1.path.read_text() == s2.path.read_text()

    def test_skips_when_fresh(self, tmp_path: Path) -> None:
        segments1 = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        marker = "\n<!-- modified -->\n"
        segments1[0].path.write_text(segments1[0].path.read_text() + marker)
        segments2 = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        assert marker in segments2[0].path.read_text()

    def test_force_reslices(self, tmp_path: Path) -> None:
        segments1 = slice_discovery(ARCHETYPES_FIXTURE, tmp_path)
        marker = "\n<!-- modified -->\n"
        segments1[0].path.write_text(segments1[0].path.read_text() + marker)
        segments2 = slice_discovery(ARCHETYPES_FIXTURE, tmp_path, force=True)
        assert marker not in segments2[0].path.read_text()


# ---------------------------------------------------------------------------
# TestSliceCluster
# ---------------------------------------------------------------------------


class TestSliceCluster:
    def test_splits_on_h3_headers(self, tmp_path: Path) -> None:
        segments = slice_cluster(CLUSTER_FIXTURE, tmp_path)
        assert len(segments) == 2

    def test_slug_generation(self, tmp_path: Path) -> None:
        segments = slice_cluster(CLUSTER_FIXTURE, tmp_path)
        names = [s.name for s in segments]
        assert "the-guardian" in names
        assert "the-seeker" in names

    def test_frontmatter(self, tmp_path: Path) -> None:
        segments = slice_cluster(CLUSTER_FIXTURE, tmp_path)
        for seg in segments:
            content = seg.path.read_text()
            assert content.startswith("---\n"), f"No frontmatter in {seg.name}"
            assert "source:" in content
            assert "segment:" in content
            assert "lines:" in content

    def test_manifest(self, tmp_path: Path) -> None:
        slice_cluster(CLUSTER_FIXTURE, tmp_path)
        manifest_path = tmp_path / "segments-manifest.json"
        assert manifest_path.exists()
        manifest = json.loads(manifest_path.read_text())
        assert len(manifest["segments"]) == 2

    def test_segment_content_preserved(self, tmp_path: Path) -> None:
        segments = slice_cluster(CLUSTER_FIXTURE, tmp_path)
        guardian = next(s for s in segments if s.name == "the-guardian")
        content = guardian.path.read_text()
        assert "Canonical Name" in content
        assert "Genre Variants" in content


# ---------------------------------------------------------------------------
# TestSliceGenreNative
# ---------------------------------------------------------------------------


class TestSliceGenreNative:
    def test_narrative_shapes_split_on_h2(self, tmp_path: Path) -> None:
        segments = slice_genre_native(SHAPES_FIXTURE, tmp_path, heading_level=2)
        assert len(segments) == 2

    def test_narrative_shapes_slug_generation(self, tmp_path: Path) -> None:
        segments = slice_genre_native(SHAPES_FIXTURE, tmp_path, heading_level=2)
        names = [s.name for s in segments]
        assert "the-rising-spiral" in names
        assert "the-descent" in names

    def test_narrative_shapes_includes_h3_subsections(self, tmp_path: Path) -> None:
        segments = slice_genre_native(SHAPES_FIXTURE, tmp_path, heading_level=2)
        spiral = next(s for s in segments if s.name == "the-rising-spiral")
        content = spiral.path.read_text()
        assert "Why this genre produces it" in content
        assert "Key Beats" in content
        assert "Tension Profile" in content

    def test_tropes_reuse_h4_pattern(self, tmp_path: Path) -> None:
        # Tropes share the same H4 structure as discovery/archetypes
        segments = slice_genre_native(ARCHETYPES_FIXTURE, tmp_path, heading_level=4)
        assert len(segments) == 3
        names = [s.name for s in segments]
        assert "the-guardian" in names

    def test_frontmatter(self, tmp_path: Path) -> None:
        segments = slice_genre_native(SHAPES_FIXTURE, tmp_path, heading_level=2)
        for seg in segments:
            content = seg.path.read_text()
            assert content.startswith("---\n")
            assert "source:" in content
            assert "segment:" in content
            assert "lines:" in content

    def test_manifest(self, tmp_path: Path) -> None:
        slice_genre_native(SHAPES_FIXTURE, tmp_path, heading_level=2)
        manifest_path = tmp_path / "segments-manifest.json"
        assert manifest_path.exists()
        manifest = json.loads(manifest_path.read_text())
        assert len(manifest["segments"]) == 2


# ---------------------------------------------------------------------------
# TestSliceFileDispatch
# ---------------------------------------------------------------------------


class TestSliceFileDispatch:
    @pytest.mark.parametrize(
        "doc_type,fixture_name,expected_count",
        [
            ("genre-region", "sample-region.md", 13),
            ("discovery", "sample-archetypes.md", 3),
            ("cluster", "sample-cluster-archetypes.md", 2),
            ("narrative-shapes", "sample-narrative-shapes.md", 2),
            ("tropes", "sample-archetypes.md", 3),
        ],
    )
    def test_dispatches_correctly(
        self,
        doc_type: str,
        fixture_name: str,
        expected_count: int,
        tmp_path: Path,
    ) -> None:
        src = _FIXTURES / fixture_name
        dest = tmp_path / "source.md"
        dest.write_text(src.read_text())
        segments = slice_file(dest, tmp_path / "segments", doc_type)
        assert len(segments) == expected_count

    def test_unknown_type_raises(self, tmp_path: Path) -> None:
        with pytest.raises(ValueError, match="Unknown doc_type"):
            slice_file(tmp_path / "dummy.md", tmp_path / "out", "nonexistent")
