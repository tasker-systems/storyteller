"""Tests for pipeline event log: append, read, and state derivation."""

import json
from pathlib import Path

from narrative_data.pipeline.events import append_event, derive_state, read_events


class TestAppendEvent:
    def test_creates_file_on_first_append(self, tmp_path: Path) -> None:
        log_path = tmp_path / "pipeline.jsonl"
        append_event(
            log_path, event="extract_started", phase=1, type="archetypes", genre="folk-horror"
        )
        assert log_path.exists()
        lines = log_path.read_text().strip().split("\n")
        assert len(lines) == 1
        data = json.loads(lines[0])
        assert data["event"] == "extract_started"
        assert data["phase"] == 1
        assert data["type"] == "archetypes"
        assert data["genre"] == "folk-horror"
        assert "timestamp" in data

    def test_appends_to_existing_file(self, tmp_path: Path) -> None:
        log_path = tmp_path / "pipeline.jsonl"
        append_event(
            log_path, event="extract_started", phase=1, type="archetypes", genre="folk-horror"
        )
        append_event(
            log_path,
            event="extract_completed",
            phase=1,
            type="archetypes",
            genre="folk-horror",
            output="discovery/archetypes/folk-horror.raw.md",
            content_digest="sha256:abc",
        )
        lines = log_path.read_text().strip().split("\n")
        assert len(lines) == 2

    def test_extra_fields_preserved(self, tmp_path: Path) -> None:
        log_path = tmp_path / "pipeline.jsonl"
        append_event(
            log_path,
            event="review_gate",
            phase=2,
            type="archetypes",
            decision="approved",
            primitives=["mentor", "trickster"],
            note="looks good",
        )
        data = json.loads(log_path.read_text().strip())
        assert data["primitives"] == ["mentor", "trickster"]
        assert data["note"] == "looks good"


class TestReadEvents:
    def test_empty_file(self, tmp_path: Path) -> None:
        log_path = tmp_path / "pipeline.jsonl"
        log_path.touch()
        assert read_events(log_path) == []

    def test_nonexistent_file(self, tmp_path: Path) -> None:
        log_path = tmp_path / "nonexistent.jsonl"
        assert read_events(log_path) == []

    def test_reads_all_events(self, tmp_path: Path) -> None:
        log_path = tmp_path / "pipeline.jsonl"
        append_event(
            log_path, event="extract_started", phase=1, type="archetypes", genre="folk-horror"
        )
        append_event(
            log_path,
            event="extract_completed",
            phase=1,
            type="archetypes",
            genre="folk-horror",
            output="x.md",
            content_digest="sha256:abc",
        )
        events = read_events(log_path)
        assert len(events) == 2
        assert events[0]["event"] == "extract_started"
        assert events[1]["event"] == "extract_completed"


class TestDeriveState:
    def _log(self, tmp_path: Path, events_data: list[dict]) -> Path:
        """Helper to write multiple events."""
        log_path = tmp_path / "pipeline.jsonl"
        for ev in events_data:
            append_event(log_path, **ev)
        return log_path

    def test_empty_state(self, tmp_path: Path) -> None:
        log_path = tmp_path / "pipeline.jsonl"
        state = derive_state(log_path, "archetypes")
        assert state["phase1_completed"] == set()
        assert state["phase2_completed"] == set()
        assert state["phase2_gate"] is None
        assert state["phase3_completed"] == set()
        assert state["phase4_completed"] == set()

    def test_phase1_progress(self, tmp_path: Path) -> None:
        log_path = self._log(
            tmp_path,
            [
                {
                    "event": "extract_completed",
                    "phase": 1,
                    "type": "archetypes",
                    "genre": "folk-horror",
                    "output": "x.md",
                    "content_digest": "sha256:a",
                },
                {
                    "event": "extract_completed",
                    "phase": 1,
                    "type": "archetypes",
                    "genre": "cosmic-horror",
                    "output": "y.md",
                    "content_digest": "sha256:b",
                },
            ],
        )
        state = derive_state(log_path, "archetypes")
        assert state["phase1_completed"] == {"folk-horror", "cosmic-horror"}

    def test_filters_by_type(self, tmp_path: Path) -> None:
        log_path = self._log(
            tmp_path,
            [
                {
                    "event": "extract_completed",
                    "phase": 1,
                    "type": "archetypes",
                    "genre": "folk-horror",
                    "output": "x.md",
                    "content_digest": "sha256:a",
                },
                {
                    "event": "extract_completed",
                    "phase": 1,
                    "type": "dynamics",
                    "genre": "folk-horror",
                    "output": "y.md",
                    "content_digest": "sha256:b",
                },
            ],
        )
        state = derive_state(log_path, "archetypes")
        assert state["phase1_completed"] == {"folk-horror"}
        state_dyn = derive_state(log_path, "dynamics")
        assert state_dyn["phase1_completed"] == {"folk-horror"}

    def test_review_gate_captured(self, tmp_path: Path) -> None:
        log_path = self._log(
            tmp_path,
            [
                {
                    "event": "review_gate",
                    "phase": 2,
                    "type": "archetypes",
                    "decision": "approved",
                    "primitives": ["mentor", "trickster"],
                },
            ],
        )
        state = derive_state(log_path, "archetypes")
        assert state["phase2_gate"] == {
            "decision": "approved",
            "primitives": ["mentor", "trickster"],
        }

    def test_phase4_tracks_genre_primitive_pairs(self, tmp_path: Path) -> None:
        log_path = self._log(
            tmp_path,
            [
                {
                    "event": "elaborate_completed",
                    "phase": 4,
                    "type": "archetypes",
                    "genre": "folk-horror",
                    "primitive": "mentor",
                    "output": "x.md",
                    "content_digest": "sha256:a",
                },
            ],
        )
        state = derive_state(log_path, "archetypes")
        assert ("folk-horror", "mentor") in state["phase4_completed"]
