"""Tests for model loading and output shapes."""

import pytest
import torch

from event_classifier.models import load_event_classifier, load_ner_model
from event_classifier.schema import (
    MAX_SEQ_LENGTH,
    NUM_BIO_LABELS,
    NUM_EVENT_KINDS,
    PRETRAINED_MODEL,
)


@pytest.fixture(scope="module")
def event_model():
    return load_event_classifier(PRETRAINED_MODEL)


@pytest.fixture(scope="module")
def ner_model():
    return load_ner_model(PRETRAINED_MODEL)


def test_event_classifier_output_shape(event_model):
    """Event classifier should output [batch, NUM_EVENT_KINDS]."""
    batch_size = 2
    dummy_input = torch.randint(0, 1000, (batch_size, MAX_SEQ_LENGTH))
    attention_mask = torch.ones_like(dummy_input)

    with torch.no_grad():
        outputs = event_model(input_ids=dummy_input, attention_mask=attention_mask)

    assert outputs.logits.shape == (batch_size, NUM_EVENT_KINDS)


def test_ner_model_output_shape(ner_model):
    """NER model should output [batch, seq_len, NUM_BIO_LABELS]."""
    batch_size = 2
    dummy_input = torch.randint(0, 1000, (batch_size, MAX_SEQ_LENGTH))
    attention_mask = torch.ones_like(dummy_input)

    with torch.no_grad():
        outputs = ner_model(input_ids=dummy_input, attention_mask=attention_mask)

    assert outputs.logits.shape == (batch_size, MAX_SEQ_LENGTH, NUM_BIO_LABELS)


def test_event_classifier_problem_type(event_model):
    """Event classifier should be configured for multi-label classification."""
    assert event_model.config.problem_type == "multi_label_classification"
