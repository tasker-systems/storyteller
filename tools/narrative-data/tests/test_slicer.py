"""Tests for genre region slicer."""

import json
from pathlib import Path

from narrative_data.pipeline.slicer import SegmentInfo, slice_genre_region

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
