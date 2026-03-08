"""Tests for label vocabularies and model constants."""

from event_classifier.schema import (
    BIO_LABEL_TO_ID,
    BIO_LABELS,
    EVENT_KIND_TO_ID,
    EVENT_KINDS,
    ID_TO_BIO_LABEL,
    ID_TO_EVENT_KIND,
    NER_CATEGORIES,
    NUM_BIO_LABELS,
    NUM_EVENT_KINDS,
)


def test_event_kinds_count():
    assert NUM_EVENT_KINDS == 8


def test_event_kind_round_trip():
    for kind in EVENT_KINDS:
        idx = EVENT_KIND_TO_ID[kind]
        assert ID_TO_EVENT_KIND[idx] == kind


def test_ner_categories_count():
    assert len(NER_CATEGORIES) == 7


def test_bio_labels_count():
    # O + 7 * (B + I) = 15
    assert NUM_BIO_LABELS == 15


def test_bio_labels_start_with_O():
    assert BIO_LABELS[0] == "O"


def test_bio_label_round_trip():
    for label in BIO_LABELS:
        idx = BIO_LABEL_TO_ID[label]
        assert ID_TO_BIO_LABEL[idx] == label


def test_bio_labels_structure():
    """Each category should have a B- and I- tag, in order."""
    for i, cat in enumerate(NER_CATEGORIES):
        b_label = BIO_LABELS[1 + i * 2]
        i_label = BIO_LABELS[1 + i * 2 + 1]
        assert b_label == f"B-{cat}"
        assert i_label == f"I-{cat}"


def test_event_kinds_match_rust():
    """Event kinds must match the Rust EventKind enum variants."""
    expected = {
        "StateAssertion",
        "ActionOccurrence",
        "SpatialChange",
        "EmotionalExpression",
        "InformationTransfer",
        "SpeechAct",
        "RelationalShift",
        "EnvironmentalChange",
    }
    assert set(EVENT_KINDS) == expected
