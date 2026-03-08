"""Shared fixtures for training tests."""

import json
import random
from pathlib import Path

import pytest
import torch

from training.feature_schema import TOTAL_INPUT_FEATURES, TOTAL_OUTPUT_FEATURES
from training.model import CharacterPredictor


@pytest.fixture
def model():
    """A fresh CharacterPredictor instance."""
    return CharacterPredictor()


@pytest.fixture
def sample_features():
    """Random feature tensor [batch=8, 453]."""
    return torch.randn(8, TOTAL_INPUT_FEATURES)


@pytest.fixture
def sample_labels():
    """Random label tensor [batch=8, 42]."""
    return torch.randn(8, TOTAL_OUTPUT_FEATURES)


def _make_cell(i: int) -> dict:
    archetypes = ["guardian", "trickster", "mentor", "innocent", "rebel"]
    dynamics = ["trust_building", "betrayal", "reunion"]
    profiles = ["high_tension", "low_tension", "neutral"]
    return {
        "archetype_a": archetypes[i % len(archetypes)],
        "archetype_b": archetypes[(i + 1) % len(archetypes)],
        "dynamic": dynamics[i % len(dynamics)],
        "a_is_role_a": i % 2 == 0,
        "profile": profiles[i % len(profiles)],
        "genre": "low_fantasy_folklore",
    }


@pytest.fixture
def synthetic_jsonl(tmp_path: Path) -> Path:
    """Create a small synthetic JSONL file with 100 examples from 10 cells."""
    rng = random.Random(42)
    path = tmp_path / "synthetic.jsonl"
    with open(path, "w") as f:
        for i in range(100):
            cell_idx = i % 10
            cell = _make_cell(cell_idx)
            features = [rng.uniform(-1, 1) for _ in range(TOTAL_INPUT_FEATURES)]
            labels = [rng.uniform(-1, 1) for _ in range(TOTAL_OUTPUT_FEATURES)]
            record = {
                "id": f"synth-{i:04d}",
                "cell": cell,
                "variation": i // 10,
                "features": features,
                "labels": labels,
                "coherence_score": rng.uniform(0.5, 1.0),
                "content_hash": f"hash-{i:04d}",
            }
            f.write(json.dumps(record) + "\n")
    return path
