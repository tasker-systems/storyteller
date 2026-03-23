# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

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


def format_status(log_path: Path, primitive_type: str) -> str:
    """Format pipeline status for a primitive type as a human-readable string."""
    from narrative_data.config import GENRE_CLUSTERS
    from narrative_data.genre.commands import GENRE_REGIONS

    state = derive_state(log_path, primitive_type)
    n_genres = len(GENRE_REGIONS)
    n_clusters = len(GENRE_CLUSTERS)
    p1 = len(state["phase1_completed"])
    p2 = len(state["phase2_completed"])
    p3 = len(state["phase3_completed"])
    p4 = len(state["phase4_completed"])
    gate = state["phase2_gate"]

    lines = [f"Pipeline Status: {primitive_type}"]
    lines.append(f"  Phase 1 (extract):     {p1}/{n_genres} genres complete")
    lines.append(f"  Phase 2 (synthesize):  {p2}/{n_clusters} clusters complete")

    if gate:
        n_prims = len(gate["primitives"])
        lines.append(f"  Phase 3 (elicit):      {p3}/{n_prims} primitives complete")
    elif p2 >= n_clusters:
        lines.append("  Phase 3 (elicit):      blocked — awaiting review gate")
    else:
        lines.append("  Phase 3 (elicit):      blocked — awaiting Phase 2 completion")

    lines.append(f"  Phase 4 (elaborate):   {p4} pairs complete")
    return "\n".join(lines)
