"""Tests for losses.py — per-head loss decomposition."""

import torch

from training.feature_schema import TOTAL_OUTPUT_FEATURES
from training.losses import MultiHeadLoss
from training.model import CharacterPredictor


def test_loss_returns_all_keys(model: CharacterPredictor, sample_features, sample_labels):
    loss_fn = MultiHeadLoss()
    predictions = model(sample_features)
    losses = loss_fn(predictions, sample_labels)

    assert set(losses.keys()) == {"total", "action", "speech", "thought", "emotion"}


def test_loss_total_is_scalar(model: CharacterPredictor, sample_features, sample_labels):
    loss_fn = MultiHeadLoss()
    predictions = model(sample_features)
    losses = loss_fn(predictions, sample_labels)

    assert losses["total"].dim() == 0
    assert losses["total"].requires_grad


def test_loss_per_head_detached(model: CharacterPredictor, sample_features, sample_labels):
    loss_fn = MultiHeadLoss()
    predictions = model(sample_features)
    losses = loss_fn(predictions, sample_labels)

    for key in ["action", "speech", "thought", "emotion"]:
        assert not losses[key].requires_grad, f"{key} loss should be detached"


def test_loss_decreases_with_matching_labels():
    """Loss should be lower when predictions match labels."""
    model = CharacterPredictor()
    model.eval()

    x = torch.randn(16, 453)
    with torch.no_grad():
        preds = model(x)

    # Construct "perfect" labels from model predictions
    perfect_labels = torch.zeros(16, TOTAL_OUTPUT_FEATURES)
    # Action type: one-hot from argmax
    action_classes = preds["action"][:, :6].argmax(dim=1)
    perfect_labels[:, :6] = torch.nn.functional.one_hot(action_classes, 6).float()
    perfect_labels[:, 6] = preds["action"][:, 6]  # confidence
    perfect_labels[:, 7] = preds["action"][:, 7]  # target
    perfect_labels[:, 8] = preds["action"][:, 8]  # valence
    context_classes = preds["action"][:, 9:14].argmax(dim=1)
    perfect_labels[:, 9:14] = torch.nn.functional.one_hot(context_classes, 5).float()

    # Speech
    perfect_labels[:, 14] = (preds["speech"][:, 0] > 0).float()
    reg_classes = preds["speech"][:, 1:5].argmax(dim=1)
    perfect_labels[:, 15:19] = torch.nn.functional.one_hot(reg_classes, 4).float()
    perfect_labels[:, 19] = preds["speech"][:, 5]

    # Thought
    aware_classes = preds["thought"][:, :5].argmax(dim=1)
    perfect_labels[:, 20:25] = torch.nn.functional.one_hot(aware_classes, 5).float()
    perfect_labels[:, 25] = preds["thought"][:, 5]

    # Emotion
    perfect_labels[:, 26:34] = preds["emotion"][:, :8]
    perfect_labels[:, 34:42] = (preds["emotion"][:, 8:16] > 0).float()

    # Random labels for comparison
    random_labels = torch.randn(16, TOTAL_OUTPUT_FEATURES)

    loss_fn = MultiHeadLoss()
    loss_perfect = loss_fn(preds, perfect_labels)["total"].item()
    loss_random = loss_fn(preds, random_labels)["total"].item()

    assert loss_perfect < loss_random, (
        f"Perfect loss ({loss_perfect:.4f}) should be less than random ({loss_random:.4f})"
    )


def test_custom_head_weights():
    loss_fn = MultiHeadLoss(
        action_weight=1.0,
        speech_weight=0.0,
        thought_weight=0.0,
        emotion_weight=0.0,
    )
    model = CharacterPredictor()
    x = torch.randn(4, 453)
    preds = model(x)
    labels = torch.randn(4, TOTAL_OUTPUT_FEATURES)

    losses = loss_fn(preds, labels)
    # Total should equal just the action loss
    assert torch.allclose(losses["total"], losses["action"], atol=1e-5)
