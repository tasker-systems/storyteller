"""Tests for manifest I/O, content digest, and staleness detection."""

from pathlib import Path

from narrative_data.pipeline.invalidation import (
    compute_content_digest,
    compute_intersection_hash,
    compute_prompt_hash,
    is_stale,
    load_manifest,
    save_manifest,
    update_manifest_entry,
)


class TestContentDigest:
    def test_digest_deterministic(self):
        assert compute_content_digest("hello world") == compute_content_digest("hello world")

    def test_digest_changes_with_content(self):
        assert compute_content_digest("hello") != compute_content_digest("world")

    def test_digest_prefix(self):
        assert compute_content_digest("test").startswith("sha256:")


class TestPromptHash:
    def test_prompt_hash_deterministic(self):
        assert compute_prompt_hash("prompt text") == compute_prompt_hash("prompt text")

    def test_prompt_hash_changes(self):
        assert compute_prompt_hash("v1 prompt") != compute_prompt_hash("v2 prompt")


class TestIntersectionHash:
    def test_intersection_hash_from_digests(self):
        h = compute_intersection_hash(["sha256:aaa", "sha256:bbb"])
        assert h.startswith("sha256:")

    def test_intersection_hash_order_sensitive(self):
        h1 = compute_intersection_hash(["sha256:aaa", "sha256:bbb"])
        h2 = compute_intersection_hash(["sha256:bbb", "sha256:aaa"])
        assert h1 != h2


class TestManifest:
    def test_load_empty_manifest(self, tmp_path: Path):
        manifest = load_manifest(tmp_path / "manifest.json")
        assert manifest == {"entries": {}}

    def test_save_and_load_manifest(self, tmp_path: Path):
        path = tmp_path / "manifest.json"
        manifest = {"entries": {"folk-horror": {"entity_id": "abc", "stage": "elicited"}}}
        save_manifest(path, manifest)
        loaded = load_manifest(path)
        assert loaded["entries"]["folk-horror"]["entity_id"] == "abc"

    def test_update_manifest_entry(self, tmp_path: Path):
        path = tmp_path / "manifest.json"
        save_manifest(path, {"entries": {}})
        update_manifest_entry(
            path,
            "folk-horror",
            {
                "entity_id": "abc",
                "prompt_hash": "h1",
                "content_digest": "d1",
                "stage": "elicited",
                "generated_at": "2026-03-17T00:00:00Z",
            },
        )
        manifest = load_manifest(path)
        assert "folk-horror" in manifest["entries"]


class TestArchiveExisting:
    def test_no_archive_when_file_missing(self, tmp_path: Path):
        from narrative_data.pipeline.invalidation import archive_existing

        result = archive_existing(tmp_path / "nonexistent.md")
        assert result is None

    def test_archives_to_v1(self, tmp_path: Path):
        from narrative_data.pipeline.invalidation import archive_existing

        original = tmp_path / "region.md"
        original.write_text("v1 content")
        archived = archive_existing(original)
        assert archived == tmp_path / "region.raw.v1.md"
        assert archived.exists()
        assert archived.read_text() == "v1 content"
        assert not original.exists()

    def test_archives_to_v2_when_v1_exists(self, tmp_path: Path):
        from narrative_data.pipeline.invalidation import archive_existing

        (tmp_path / "region.raw.v1.md").write_text("old v1")
        original = tmp_path / "region.md"
        original.write_text("v2 content")
        archived = archive_existing(original)
        assert archived == tmp_path / "region.raw.v2.md"
        assert archived.exists()
        assert (tmp_path / "region.raw.v1.md").exists()  # v1 untouched

    def test_sequential_archives(self, tmp_path: Path):
        from narrative_data.pipeline.invalidation import archive_existing

        # Simulate three passes
        for i in range(1, 4):
            original = tmp_path / "region.md"
            original.write_text(f"pass {i} content")
            archived = archive_existing(original)
            assert archived == tmp_path / f"region.raw.v{i}.md"
            # Write the "new" version back so next iteration has something to archive
            if i < 3:
                original.write_text(f"pass {i + 1} content")


class TestRunLog:
    def test_write_run_log(self, tmp_path: Path):
        from narrative_data.pipeline.invalidation import write_run_log

        log_path = write_run_log(
            tmp_path,
            {
                "model": "qwen3.5:35b",
                "cells_processed": 5,
                "domain": "genre",
            },
        )
        assert log_path.exists()
        assert log_path.parent.name == "runs"


class TestStaleness:
    def test_not_stale_when_hashes_match(self):
        entry = {"prompt_hash": "h1", "content_digest": "d1"}
        assert not is_stale(entry, current_prompt_hash="h1", upstream_digest="d1")

    def test_stale_when_prompt_changed(self):
        entry = {"prompt_hash": "h1", "content_digest": "d1"}
        assert is_stale(entry, current_prompt_hash="h2", upstream_digest="d1")

    def test_stale_when_upstream_changed(self):
        entry = {"prompt_hash": "h1", "content_digest": "d1"}
        assert is_stale(entry, current_prompt_hash="h1", upstream_digest="d2")

    def test_stale_when_no_entry(self):
        assert is_stale(None, current_prompt_hash="h1", upstream_digest="d1")
