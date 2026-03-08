"""JSONL loading, tokenization, BIO alignment, and HuggingFace Dataset creation.

The critical function is `align_bio_labels()` which converts character-level
entity span annotations to token-level BIO labels using the tokenizer's
offset_mapping. This handles subword tokenization correctly.
"""

import json
from pathlib import Path

from datasets import Dataset
from sklearn.model_selection import train_test_split
from transformers import PreTrainedTokenizerFast

from event_classifier.schema import (
    BIO_LABEL_TO_ID,
    EVENT_KIND_TO_ID,
    IGNORE_INDEX,
    MAX_SEQ_LENGTH,
    NUM_EVENT_KINDS,
)


def load_jsonl(path: str | Path) -> list[dict]:
    """Load and validate training examples from JSONL.

    Each line must have: id, text, register, event_kinds, entities.
    Entities must have: start, end, text, category, role.
    """
    path = Path(path)
    examples = []
    required_top = {"id", "text", "register", "event_kinds", "entities"}
    required_entity = {"start", "end", "text", "category", "role"}

    with open(path) as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)
            missing = required_top - record.keys()
            if missing:
                raise ValueError(f"Line {line_num}: missing fields {missing}")
            for i, ent in enumerate(record["entities"]):
                ent_missing = required_entity - ent.keys()
                if ent_missing:
                    raise ValueError(f"Line {line_num}, entity {i}: missing fields {ent_missing}")
            examples.append(record)

    return examples


def split_data(
    examples: list[dict],
    val_fraction: float = 0.15,
    seed: int = 42,
) -> tuple[list[dict], list[dict]]:
    """Stratified split by primary event kind.

    Uses the first event_kind as the stratification key. Examples with
    the same primary event kind are kept proportionally in train/val.
    """
    primary_kinds = [ex["event_kinds"][0] for ex in examples]
    train_examples, val_examples = train_test_split(
        examples,
        test_size=val_fraction,
        random_state=seed,
        stratify=primary_kinds,
    )
    return train_examples, val_examples


def align_bio_labels(
    entities: list[dict],
    offset_mapping: list[tuple[int, int] | None],
) -> list[int]:
    """Convert character-level entity spans to token-level BIO labels.

    Args:
        entities: Entity annotations with start/end character offsets and category.
        offset_mapping: Token-to-character offset pairs from the tokenizer.
            Special tokens have (0, 0) or None offsets.

    Returns:
        List of BIO label IDs, one per token. Special/padding tokens get IGNORE_INDEX.
    """
    labels = []
    o_id = BIO_LABEL_TO_ID["O"]

    for offset in offset_mapping:
        # Some tokenizers use None for special tokens
        if offset is None:
            labels.append(IGNORE_INDEX)
            continue

        tok_start, tok_end = offset

        # Padding tokens and special tokens (CLS, SEP) have zero-length spans
        if tok_start == tok_end:
            labels.append(IGNORE_INDEX)
            continue

        # Find which entity (if any) this token belongs to
        matched_entity = None
        for ent in entities:
            ent_start = ent["start"]
            ent_end = ent["end"]
            # Token overlaps with entity if there's any intersection
            if tok_start < ent_end and tok_end > ent_start:
                matched_entity = ent
                break

        if matched_entity is None:
            labels.append(o_id)
        else:
            cat = matched_entity["category"]
            # B- tag if this token starts at or before the entity start
            # (i.e., this is the first token of the entity)
            if tok_start <= matched_entity["start"]:
                labels.append(BIO_LABEL_TO_ID[f"B-{cat}"])
            else:
                labels.append(BIO_LABEL_TO_ID[f"I-{cat}"])

    return labels


def _encode_event_labels(event_kinds: list[str]) -> list[float]:
    """Multi-hot encode event kind labels."""
    labels = [0.0] * NUM_EVENT_KINDS
    for kind in event_kinds:
        if kind in EVENT_KIND_TO_ID:
            labels[EVENT_KIND_TO_ID[kind]] = 1.0
    return labels


def create_event_classification_dataset(
    examples: list[dict],
    tokenizer: PreTrainedTokenizerFast,
) -> Dataset:
    """Create a HuggingFace Dataset for event classification (sequence-level).

    Returns a Dataset with columns: input_ids, attention_mask, labels (multi-hot float).
    """
    texts = [ex["text"] for ex in examples]
    encodings = tokenizer(
        texts,
        max_length=MAX_SEQ_LENGTH,
        padding="max_length",
        truncation=True,
        return_tensors=None,
    )

    labels = [_encode_event_labels(ex["event_kinds"]) for ex in examples]

    return Dataset.from_dict(
        {
            "input_ids": encodings["input_ids"],
            "attention_mask": encodings["attention_mask"],
            "labels": labels,
        }
    )


def create_ner_dataset(
    examples: list[dict],
    tokenizer: PreTrainedTokenizerFast,
) -> Dataset:
    """Create a HuggingFace Dataset for NER (token-level BIO classification).

    Returns a Dataset with columns: input_ids, attention_mask, labels (per-token int).
    """
    all_input_ids = []
    all_attention_masks = []
    all_labels = []

    for ex in examples:
        encoding = tokenizer(
            ex["text"],
            max_length=MAX_SEQ_LENGTH,
            padding="max_length",
            truncation=True,
            return_offsets_mapping=True,
            return_tensors=None,
        )

        bio_labels = align_bio_labels(ex["entities"], encoding["offset_mapping"])

        # Pad labels to max_length (padded tokens get IGNORE_INDEX)
        while len(bio_labels) < MAX_SEQ_LENGTH:
            bio_labels.append(IGNORE_INDEX)

        all_input_ids.append(encoding["input_ids"])
        all_attention_masks.append(encoding["attention_mask"])
        all_labels.append(bio_labels)

    return Dataset.from_dict(
        {
            "input_ids": all_input_ids,
            "attention_mask": all_attention_masks,
            "labels": all_labels,
        }
    )
