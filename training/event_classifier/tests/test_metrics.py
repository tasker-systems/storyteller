"""Tests for metric computation."""

import numpy as np

from event_classifier.metrics import compute_event_metrics, compute_ner_metrics
from event_classifier.schema import (
    BIO_LABEL_TO_ID,
    IGNORE_INDEX,
    NUM_BIO_LABELS,
    NUM_EVENT_KINDS,
)


def test_event_metrics_perfect_prediction():
    """Perfect predictions should give F1 = 1.0."""
    # One example per class so all classes have support
    labels = np.eye(NUM_EVENT_KINDS, dtype=float)
    logits = np.where(labels == 1, 5.0, -5.0)
    metrics = compute_event_metrics((logits, labels))
    assert metrics["macro_f1"] == 1.0


def test_event_metrics_wrong_prediction():
    """Completely wrong predictions should give F1 = 0.0."""
    labels = np.eye(NUM_EVENT_KINDS, dtype=float)
    # Predict the opposite
    logits = np.where(labels == 1, -5.0, 5.0)
    metrics = compute_event_metrics((logits, labels))
    assert metrics["macro_f1"] == 0.0


def test_event_metrics_has_per_class_f1():
    """Metrics should include per-class F1 scores."""
    labels = np.ones((4, NUM_EVENT_KINDS), dtype=float)
    logits = np.ones((4, NUM_EVENT_KINDS)) * 5.0
    metrics = compute_event_metrics((logits, labels))
    assert "f1_ActionOccurrence" in metrics
    assert "f1_SpeechAct" in metrics


def test_ner_metrics_perfect():
    """Perfect NER predictions should give entity_f1 = 1.0."""
    seq_len = 10
    # Build labels: B-CHARACTER, I-CHARACTER, O, O, B-LOCATION, O, ...
    b_char = BIO_LABEL_TO_ID["B-CHARACTER"]
    i_char = BIO_LABEL_TO_ID["I-CHARACTER"]
    o = BIO_LABEL_TO_ID["O"]
    b_loc = BIO_LABEL_TO_ID["B-LOCATION"]

    ign = IGNORE_INDEX
    labels = np.array([[b_char, i_char, o, o, b_loc, o, ign, ign, ign, ign]])
    # Perfect logits: one-hot for the correct label
    logits = np.full((1, seq_len, NUM_BIO_LABELS), -10.0)
    for i in range(seq_len):
        if labels[0, i] != IGNORE_INDEX:
            logits[0, i, labels[0, i]] = 10.0
        else:
            logits[0, i, o] = 10.0  # doesn't matter, will be ignored

    metrics = compute_ner_metrics((logits, labels))
    assert metrics["entity_f1"] == 1.0


def test_ner_metrics_ignores_special_tokens():
    """IGNORE_INDEX tokens should not affect NER metrics."""
    seq_len = 5
    o = BIO_LABEL_TO_ID["O"]
    labels = np.array([[IGNORE_INDEX, o, o, o, IGNORE_INDEX]])
    logits = np.full((1, seq_len, NUM_BIO_LABELS), -10.0)
    for i in range(seq_len):
        logits[0, i, o] = 10.0

    metrics = compute_ner_metrics((logits, labels))
    # No entities to detect, but no errors either
    assert "entity_f1" in metrics
