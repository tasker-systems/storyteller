"""Tests for dataset loading, BIO alignment, and HF Dataset creation.

BIO alignment is the critical test surface — subword splits, adjacent
entities, edge positions, and special token handling all need coverage.
"""

import json
from pathlib import Path

import pytest
from transformers import AutoTokenizer

from event_classifier.dataset import (
    align_bio_labels,
    create_event_classification_dataset,
    create_ner_dataset,
    load_jsonl,
    split_data,
)
from event_classifier.schema import (
    BIO_LABEL_TO_ID,
    IGNORE_INDEX,
    MAX_SEQ_LENGTH,
    NUM_EVENT_KINDS,
    PRETRAINED_MODEL,
)


@pytest.fixture(scope="module")
def tokenizer():
    return AutoTokenizer.from_pretrained(PRETRAINED_MODEL)


def _real(labels: list[int]) -> list[int]:
    """Filter out IGNORE_INDEX tokens."""
    return [x for x in labels if x != IGNORE_INDEX]


# ---------------------------------------------------------------------------
# load_jsonl
# ---------------------------------------------------------------------------


def test_load_jsonl(synthetic_jsonl: Path):
    examples = load_jsonl(synthetic_jsonl)
    assert len(examples) == 80
    assert all("text" in ex for ex in examples)


def test_load_jsonl_missing_field(tmp_path: Path):
    path = tmp_path / "bad.jsonl"
    path.write_text(json.dumps({"id": "1", "text": "hello"}) + "\n")
    with pytest.raises(ValueError, match="missing fields"):
        load_jsonl(path)


def test_load_jsonl_missing_entity_field(tmp_path: Path):
    path = tmp_path / "bad_ent.jsonl"
    record = {
        "id": "1",
        "text": "hello",
        "register": "player",
        "event_kinds": ["StateAssertion"],
        # missing category, role
        "entities": [{"start": 0, "end": 5, "text": "hello"}],
    }
    path.write_text(json.dumps(record) + "\n")
    with pytest.raises(ValueError, match="missing fields"):
        load_jsonl(path)


# ---------------------------------------------------------------------------
# split_data
# ---------------------------------------------------------------------------


def test_split_data(synthetic_examples: list[dict]):
    # Need enough examples per class for stratification
    examples = synthetic_examples * 10  # 80 examples
    train, val = split_data(examples, val_fraction=0.2, seed=42)
    assert len(train) + len(val) == len(examples)
    assert len(val) == pytest.approx(len(examples) * 0.2, abs=2)


# ---------------------------------------------------------------------------
# align_bio_labels — the critical test surface
# ---------------------------------------------------------------------------


def test_bio_alignment_simple(tokenizer):
    """Single entity, no subword complexity."""
    text = "Sarah walked"
    entities = [
        {"start": 0, "end": 5, "text": "Sarah", "category": "CHARACTER", "role": "Actor"},
    ]
    encoding = tokenizer(
        text,
        return_offsets_mapping=True,
        return_tensors=None,
    )
    labels = align_bio_labels(entities, encoding["offset_mapping"])
    assert len(labels) == len(encoding["input_ids"])

    b_char_id = BIO_LABEL_TO_ID["B-CHARACTER"]
    o_id = BIO_LABEL_TO_ID["O"]

    rl = _real(labels)
    assert b_char_id in rl
    assert o_id in rl


def test_bio_alignment_multiword_entity(tokenizer):
    """Multi-word entity 'the old man' should get B- then I- tags."""
    text = "The old man sat down"
    entities = [
        {"start": 0, "end": 11, "text": "The old man", "category": "CHARACTER", "role": "Actor"},
    ]
    encoding = tokenizer(
        text,
        return_offsets_mapping=True,
        return_tensors=None,
    )
    labels = align_bio_labels(entities, encoding["offset_mapping"])

    b_id = BIO_LABEL_TO_ID["B-CHARACTER"]
    i_id = BIO_LABEL_TO_ID["I-CHARACTER"]

    rl = _real(labels)
    assert rl[0] == b_id, "First entity token should be B-"
    assert i_id in rl


def test_bio_alignment_multiple_entities(tokenizer):
    """Two separate entities in one sentence."""
    text = "Sarah gave the stone to Adam"
    entities = [
        {"start": 0, "end": 5, "text": "Sarah", "category": "CHARACTER", "role": "Actor"},
        {"start": 11, "end": 20, "text": "the stone", "category": "OBJECT", "role": "Target"},
        {"start": 24, "end": 28, "text": "Adam", "category": "CHARACTER", "role": "Recipient"},
    ]
    encoding = tokenizer(
        text,
        return_offsets_mapping=True,
        return_tensors=None,
    )
    labels = align_bio_labels(entities, encoding["offset_mapping"])

    rl = _real(labels)
    b_char = BIO_LABEL_TO_ID["B-CHARACTER"]
    b_obj = BIO_LABEL_TO_ID["B-OBJECT"]

    assert rl.count(b_char) == 2
    assert rl.count(b_obj) == 1


def test_bio_alignment_no_entities(tokenizer):
    """Text with no entities — all O labels."""
    text = "Something happened"
    encoding = tokenizer(
        text,
        return_offsets_mapping=True,
        return_tensors=None,
    )
    labels = align_bio_labels([], encoding["offset_mapping"])

    o_id = BIO_LABEL_TO_ID["O"]
    rl = _real(labels)
    assert all(x == o_id for x in rl)


def test_bio_alignment_subword_entity(tokenizer):
    """Entity split into subwords should have B- then I- tags."""
    text = "Whisperthorn appeared"
    entities = [
        {"start": 0, "end": 12, "text": "Whisperthorn", "category": "CHARACTER", "role": "Actor"},
    ]
    encoding = tokenizer(
        text,
        return_offsets_mapping=True,
        return_tensors=None,
    )
    labels = align_bio_labels(entities, encoding["offset_mapping"])

    b_id = BIO_LABEL_TO_ID["B-CHARACTER"]
    i_id = BIO_LABEL_TO_ID["I-CHARACTER"]

    rl = _real(labels)
    entity_labels = [x for x in rl if x != BIO_LABEL_TO_ID["O"]]
    assert len(entity_labels) >= 1
    assert entity_labels[0] == b_id
    if len(entity_labels) > 1:
        assert all(x == i_id for x in entity_labels[1:])


def test_bio_alignment_padding_tokens_ignored(tokenizer):
    """Padding tokens should get IGNORE_INDEX."""
    text = "Hello world"
    encoding = tokenizer(
        text,
        return_offsets_mapping=True,
        return_tensors=None,
        padding="max_length",
        max_length=8,
        truncation=True,
    )
    labels = align_bio_labels([], encoding["offset_mapping"])

    o_id = BIO_LABEL_TO_ID["O"]
    for i, mask in enumerate(encoding["attention_mask"]):
        if mask == 1:
            assert labels[i] == o_id
        else:
            assert labels[i] == IGNORE_INDEX


# ---------------------------------------------------------------------------
# create_event_classification_dataset
# ---------------------------------------------------------------------------


def test_create_event_classification_dataset(
    synthetic_examples: list[dict],
    tokenizer,
):
    ds = create_event_classification_dataset(synthetic_examples, tokenizer)
    assert len(ds) == len(synthetic_examples)
    assert "input_ids" in ds.column_names
    assert "attention_mask" in ds.column_names
    assert "labels" in ds.column_names
    assert len(ds[0]["labels"]) == NUM_EVENT_KINDS
    assert len(ds[0]["input_ids"]) == MAX_SEQ_LENGTH


def test_event_labels_multi_hot(tokenizer):
    """Multi-label example should have multiple 1.0s."""
    examples = [
        {
            "id": "multi-1",
            "text": "Sarah cried as she told Adam the truth",
            "register": "narrator",
            "event_kinds": ["EmotionalExpression", "InformationTransfer"],
            "entities": [],
        }
    ]
    ds = create_event_classification_dataset(examples, tokenizer)
    labels = ds[0]["labels"]
    assert sum(labels) == 2.0


# ---------------------------------------------------------------------------
# create_ner_dataset
# ---------------------------------------------------------------------------


def test_create_ner_dataset(synthetic_examples: list[dict], tokenizer):
    ds = create_ner_dataset(synthetic_examples, tokenizer)
    assert len(ds) == len(synthetic_examples)
    assert "labels" in ds.column_names
    assert len(ds[0]["labels"]) == MAX_SEQ_LENGTH


def test_ner_labels_have_entity_tags(tokenizer):
    """NER dataset labels should contain B- tags for entities."""
    examples = [
        {
            "id": "ner-1",
            "text": "Sarah walked to the clearing",
            "register": "narrator",
            "event_kinds": ["SpatialChange"],
            "entities": [
                {"start": 0, "end": 5, "text": "Sarah", "category": "CHARACTER", "role": "Actor"},
                {
                    "start": 19,
                    "end": 31,
                    "text": "the clearing",
                    "category": "LOCATION",
                    "role": "Dest",
                },
            ],
        }
    ]
    ds = create_ner_dataset(examples, tokenizer)
    labels = ds[0]["labels"]
    rl = _real(labels)
    b_char = BIO_LABEL_TO_ID["B-CHARACTER"]
    b_loc = BIO_LABEL_TO_ID["B-LOCATION"]
    assert b_char in rl
    assert b_loc in rl
