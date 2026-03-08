"""Tests for model.py — architecture and shape verification."""

import torch

from training.feature_schema import (
    ACTION_HEAD_SIZE,
    EMOTION_HEAD_SIZE,
    SPEECH_HEAD_SIZE,
    THOUGHT_HEAD_SIZE,
    TOTAL_INPUT_FEATURES,
)
from training.model import CharacterPredictor


def test_forward_output_shapes(model: CharacterPredictor, sample_features: torch.Tensor):
    outputs = model(sample_features)
    batch = sample_features.shape[0]

    assert outputs["action"].shape == (batch, ACTION_HEAD_SIZE)
    assert outputs["speech"].shape == (batch, SPEECH_HEAD_SIZE)
    assert outputs["thought"].shape == (batch, THOUGHT_HEAD_SIZE)
    assert outputs["emotion"].shape == (batch, EMOTION_HEAD_SIZE)


def test_forward_single_example():
    model = CharacterPredictor()
    x = torch.randn(1, TOTAL_INPUT_FEATURES)
    outputs = model(x)
    assert outputs["action"].shape == (1, 14)
    assert outputs["speech"].shape == (1, 6)
    assert outputs["thought"].shape == (1, 6)
    assert outputs["emotion"].shape == (1, 16)


def test_parameter_count():
    model = CharacterPredictor()
    total = sum(p.numel() for p in model.parameters())
    # Expected ~370K params — allow generous range
    assert 300_000 < total < 500_000, f"Unexpected parameter count: {total}"


def test_custom_architecture():
    model = CharacterPredictor(
        input_dim=100,
        trunk_dims=(128, 64),
        head_hidden=32,
        dropout=0.1,
    )
    x = torch.randn(4, 100)
    outputs = model(x)
    assert outputs["action"].shape == (4, ACTION_HEAD_SIZE)


def test_dropout_active_in_training():
    model = CharacterPredictor(dropout=0.5)
    model.train()
    x = torch.randn(32, TOTAL_INPUT_FEATURES)
    # Run multiple times — with dropout=0.5, outputs should vary
    out1 = model(x)["action"].detach()
    out2 = model(x)["action"].detach()
    # Not a strict test, but with 50% dropout they should almost certainly differ
    assert not torch.allclose(out1, out2, atol=1e-6)


def test_eval_mode_deterministic():
    model = CharacterPredictor()
    model.eval()
    x = torch.randn(4, TOTAL_INPUT_FEATURES)
    with torch.no_grad():
        out1 = model(x)["action"]
        out2 = model(x)["action"]
    assert torch.allclose(out1, out2)
