"""Shared fixtures for event classifier tests."""

import json
from pathlib import Path

import pytest


def _ent(start: int, end: int, text: str, category: str, role: str) -> dict:
    """Shorthand for building entity annotation dicts."""
    return {
        "start": start,
        "end": end,
        "text": text,
        "category": category,
        "role": role,
    }


def _make_example(
    text: str,
    event_kinds: list[str],
    entities: list[dict] | None = None,
    register: str = "player",
    action_type: str | None = None,
) -> dict:
    """Helper to build a training example dict."""
    ex = {
        "id": f"test-{hash(text) % 10000:04d}",
        "text": text,
        "register": register,
        "event_kinds": event_kinds,
        "entities": entities or [],
    }
    if action_type:
        ex["action_type"] = action_type
    return ex


@pytest.fixture
def synthetic_examples() -> list[dict]:
    """A small set of synthetic training examples covering all event kinds."""
    return [
        _make_example(
            "I pick up the stone",
            ["ActionOccurrence"],
            [
                _ent(0, 1, "I", "CHARACTER", "Actor"),
                _ent(10, 19, "the stone", "OBJECT", "Target"),
            ],
            action_type="Perform",
        ),
        _make_example(
            "Sarah crossed the river to the clearing",
            ["SpatialChange"],
            [
                _ent(0, 5, "Sarah", "CHARACTER", "Actor"),
                _ent(18, 27, "the river", "LOCATION", "Path"),
                _ent(31, 43, "the clearing", "LOCATION", "Destination"),
            ],
            register="narrator",
        ),
        _make_example(
            "The old man is sitting at the table",
            ["StateAssertion"],
            [
                _ent(0, 11, "The old man", "CHARACTER", "Subject"),
                _ent(27, 36, "the table", "OBJECT", "Location"),
            ],
            register="narrator",
        ),
        _make_example(
            "Tanya begins to cry",
            ["EmotionalExpression"],
            [_ent(0, 5, "Tanya", "CHARACTER", "Experiencer")],
            register="narrator",
        ),
        _make_example(
            "I tell Sarah about the hidden path",
            ["InformationTransfer"],
            [
                _ent(0, 1, "I", "CHARACTER", "Source"),
                _ent(7, 12, "Sarah", "CHARACTER", "Target"),
                _ent(19, 34, "the hidden path", "ABSTRACT", "Content"),
            ],
        ),
        _make_example(
            "Adam whispered a warning",
            ["SpeechAct"],
            [_ent(0, 4, "Adam", "CHARACTER", "Speaker")],
            register="narrator",
        ),
        _make_example(
            "Her trust in Adam deepened",
            ["RelationalShift"],
            [
                _ent(4, 9, "trust", "ABSTRACT", "Dimension"),
                _ent(13, 17, "Adam", "CHARACTER", "Target"),
            ],
            register="narrator",
        ),
        _make_example(
            "Rain begins to fall across the valley",
            ["EnvironmentalChange"],
            [
                _ent(0, 4, "Rain", "SENSORY", "Phenomenon"),
                _ent(27, 37, "the valley", "LOCATION", "Scope"),
            ],
            register="narrator",
        ),
    ]


@pytest.fixture
def synthetic_jsonl(tmp_path: Path, synthetic_examples: list[dict]) -> Path:
    """Write synthetic examples to JSONL, duplicated to reach 80 examples."""
    path = tmp_path / "test_data.jsonl"
    with open(path, "w") as f:
        for i in range(80):
            ex = synthetic_examples[i % len(synthetic_examples)].copy()
            ex["id"] = f"test-{i:04d}"
            f.write(json.dumps(ex) + "\n")
    return path
