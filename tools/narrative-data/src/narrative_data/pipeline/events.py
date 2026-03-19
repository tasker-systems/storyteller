"""Pipeline event log: append-only JSONL for tracking pipeline progress."""

import json
from pathlib import Path

from narrative_data.utils import now_iso


def append_event(log_path: Path, *, event: str, phase: int, type: str, **kwargs: object) -> None:
    """Append a single event to the pipeline JSONL log."""
    entry = {"event": event, "phase": phase, "type": type, "timestamp": now_iso(), **kwargs}
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("a") as f:
        f.write(json.dumps(entry, default=str) + "\n")


def read_events(log_path: Path) -> list[dict]:
    """Read all events from the pipeline JSONL log."""
    if not log_path.exists():
        return []
    events = []
    for line in log_path.read_text().strip().split("\n"):
        if line:
            events.append(json.loads(line))
    return events


def derive_state(log_path: Path, primitive_type: str) -> dict:
    """Derive current pipeline state for a primitive type by replaying the event log."""
    events = read_events(log_path)
    state: dict = {
        "phase1_completed": set(),  # genre slugs
        "phase2_completed": set(),  # cluster names
        "phase2_gate": None,  # review gate event dict or None
        "phase3_completed": set(),  # primitive slugs
        "phase4_completed": set(),  # (genre, primitive) tuples
        "phase4_native_completed": set(),  # (genre, native_type) tuples
    }
    for ev in events:
        if ev.get("type") != primitive_type:
            continue
        match ev["event"]:
            case "extract_completed":
                state["phase1_completed"].add(ev["genre"])
            case "synthesize_completed":
                state["phase2_completed"].add(ev["cluster"])
            case "review_gate" if ev.get("phase") == 2 and ev.get("decision") == "approved":
                state["phase2_gate"] = {
                    "decision": ev["decision"],
                    "primitives": ev.get("primitives", []),
                }
            case "elicit_completed" if ev.get("phase") == 3:
                state["phase3_completed"].add(ev["primitive"])
            case "elaborate_completed":
                state["phase4_completed"].add((ev["genre"], ev["primitive"]))
            case "elicit_native_completed":
                state["phase4_native_completed"].add((ev["genre"], ev["native_type"]))
    return state
