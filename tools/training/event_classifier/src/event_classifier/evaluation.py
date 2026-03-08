"""Evaluation framework for event classification and NER ONNX models.

Loads deployed ONNX models, runs inference on labeled JSONL test data,
and computes per-task metrics. This is lightweight tooling for manual use
when swapping models or training data — not a CI test harness.

Metrics:
- Event classification: per-class precision/recall/F1 + macro F1
- Entity extraction: entity-level F1 (span-exact match via seqeval)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import onnxruntime as ort
from seqeval.metrics import classification_report as seqeval_report
from seqeval.metrics import f1_score as seqeval_f1
from seqeval.metrics import precision_score as seqeval_precision
from seqeval.metrics import recall_score as seqeval_recall
from sklearn.metrics import f1_score, precision_score, recall_score
from transformers import PreTrainedTokenizerFast

from event_classifier.dataset import align_bio_labels, load_jsonl, split_data
from event_classifier.schema import (
    EVENT_KIND_TO_ID,
    EVENT_KINDS,
    ID_TO_BIO_LABEL,
    IGNORE_INDEX,
    MAX_SEQ_LENGTH,
    NUM_EVENT_KINDS,
)

# ---------------------------------------------------------------------------
# Result types
# ---------------------------------------------------------------------------


@dataclass
class EventClassMetrics:
    """Per-class and aggregate metrics for event classification."""

    per_class: dict[str, dict[str, float]] = field(default_factory=dict)
    macro_f1: float = 0.0
    macro_precision: float = 0.0
    macro_recall: float = 0.0


@dataclass
class NerMetrics:
    """Entity-level metrics for NER."""

    entity_f1: float = 0.0
    entity_precision: float = 0.0
    entity_recall: float = 0.0
    per_category: dict[str, dict[str, float]] = field(default_factory=dict)
    seqeval_report: str = ""


@dataclass
class EvaluationResult:
    """Complete evaluation output."""

    model_dir: str
    num_examples: int
    event_metrics: EventClassMetrics | None = None
    ner_metrics: NerMetrics | None = None


# ---------------------------------------------------------------------------
# Model loading
# ---------------------------------------------------------------------------


def load_onnx_session(model_path: Path) -> ort.InferenceSession:
    """Load an ONNX model as an InferenceSession."""
    return ort.InferenceSession(
        str(model_path),
        providers=["CPUExecutionProvider"],
    )


def load_tokenizer(model_dir: Path) -> PreTrainedTokenizerFast:
    """Load the tokenizer from a model directory."""
    return PreTrainedTokenizerFast(tokenizer_file=str(model_dir / "tokenizer.json"))


# ---------------------------------------------------------------------------
# Event classification evaluation
# ---------------------------------------------------------------------------


def evaluate_event_classification(
    session: ort.InferenceSession,
    tokenizer: PreTrainedTokenizerFast,
    examples: list[dict],
    threshold: float = 0.5,
    batch_size: int = 32,
) -> EventClassMetrics:
    """Run event classification on examples and compute per-class metrics.

    Args:
        session: ONNX event classification model session.
        tokenizer: Tokenizer matching the model.
        examples: Labeled examples with event_kinds field.
        threshold: Sigmoid threshold for positive prediction.
        batch_size: Inference batch size.

    Returns:
        EventClassMetrics with per-class and macro scores.
    """
    all_labels = np.zeros((len(examples), NUM_EVENT_KINDS), dtype=np.int32)
    all_preds = np.zeros((len(examples), NUM_EVENT_KINDS), dtype=np.int32)

    for i, ex in enumerate(examples):
        for kind in ex["event_kinds"]:
            if kind in EVENT_KIND_TO_ID:
                all_labels[i, EVENT_KIND_TO_ID[kind]] = 1

    # Run inference in batches
    for batch_start in range(0, len(examples), batch_size):
        batch_end = min(batch_start + batch_size, len(examples))
        batch_texts = [ex["text"] for ex in examples[batch_start:batch_end]]

        encodings = tokenizer(
            batch_texts,
            max_length=MAX_SEQ_LENGTH,
            padding="max_length",
            truncation=True,
            return_tensors="np",
        )

        outputs = session.run(
            None,
            {
                "input_ids": encodings["input_ids"].astype(np.int64),
                "attention_mask": encodings["attention_mask"].astype(np.int64),
            },
        )

        logits = outputs[0]
        probs = 1.0 / (1.0 + np.exp(-logits))  # sigmoid
        preds = (probs > threshold).astype(np.int32)
        all_preds[batch_start:batch_end] = preds

    # Compute per-class metrics
    metrics = EventClassMetrics()
    for i, kind in enumerate(EVENT_KINDS):
        p = float(precision_score(all_labels[:, i], all_preds[:, i], zero_division=0.0))
        r = float(recall_score(all_labels[:, i], all_preds[:, i], zero_division=0.0))
        f = float(f1_score(all_labels[:, i], all_preds[:, i], zero_division=0.0))
        metrics.per_class[kind] = {"precision": p, "recall": r, "f1": f}

    metrics.macro_precision = float(
        precision_score(all_labels, all_preds, average="macro", zero_division=0.0)
    )
    metrics.macro_recall = float(
        recall_score(all_labels, all_preds, average="macro", zero_division=0.0)
    )
    metrics.macro_f1 = float(f1_score(all_labels, all_preds, average="macro", zero_division=0.0))

    return metrics


# ---------------------------------------------------------------------------
# NER evaluation
# ---------------------------------------------------------------------------


def evaluate_ner(
    session: ort.InferenceSession,
    tokenizer: PreTrainedTokenizerFast,
    examples: list[dict],
    batch_size: int = 32,
) -> NerMetrics:
    """Run NER on examples and compute entity-level metrics.

    Args:
        session: ONNX NER model session.
        tokenizer: Tokenizer matching the model.
        examples: Labeled examples with entities field.
        batch_size: Inference batch size.

    Returns:
        NerMetrics with entity-level precision/recall/F1.
    """
    all_true_sequences: list[list[str]] = []
    all_pred_sequences: list[list[str]] = []

    for batch_start in range(0, len(examples), batch_size):
        batch_end = min(batch_start + batch_size, len(examples))
        batch = examples[batch_start:batch_end]

        for ex in batch:
            encoding = tokenizer(
                ex["text"],
                max_length=MAX_SEQ_LENGTH,
                padding="max_length",
                truncation=True,
                return_offsets_mapping=True,
                return_tensors="np",
            )

            # Gold labels
            gold_bio = align_bio_labels(ex["entities"], encoding["offset_mapping"][0].tolist())

            # Inference
            outputs = session.run(
                None,
                {
                    "input_ids": encoding["input_ids"].astype(np.int64),
                    "attention_mask": encoding["attention_mask"].astype(np.int64),
                },
            )

            logits = outputs[0]  # [1, seq_len, num_labels]
            pred_ids = np.argmax(logits[0], axis=-1)

            # Convert to label strings, filtering ignored tokens
            true_labels = []
            pred_labels = []
            for pred_id, gold_id in zip(pred_ids, gold_bio, strict=False):
                if gold_id == IGNORE_INDEX:
                    continue
                true_labels.append(ID_TO_BIO_LABEL[int(gold_id)])
                pred_labels.append(ID_TO_BIO_LABEL[int(pred_id)])

            all_true_sequences.append(true_labels)
            all_pred_sequences.append(pred_labels)

    metrics = NerMetrics()
    metrics.entity_f1 = float(seqeval_f1(all_true_sequences, all_pred_sequences, zero_division=0.0))
    metrics.entity_precision = float(
        seqeval_precision(all_true_sequences, all_pred_sequences, zero_division=0.0)
    )
    metrics.entity_recall = float(
        seqeval_recall(all_true_sequences, all_pred_sequences, zero_division=0.0)
    )

    # Per-category report (seqeval crashes on empty entity sets)
    has_entities = any(
        label != "O" for seq in (all_true_sequences + all_pred_sequences) for label in seq
    )
    if has_entities:
        metrics.seqeval_report = seqeval_report(
            all_true_sequences, all_pred_sequences, zero_division=0.0
        )
    else:
        metrics.seqeval_report = "(no entities in gold or predictions)"

    return metrics


# ---------------------------------------------------------------------------
# Full evaluation pipeline
# ---------------------------------------------------------------------------


def evaluate(
    model_dir: Path,
    data_path: Path,
    *,
    val_fraction: float = 0.15,
    seed: int = 42,
    threshold: float = 0.5,
    use_split: bool = True,
) -> EvaluationResult:
    """Run full evaluation of deployed ONNX models against labeled data.

    Args:
        model_dir: Directory containing ONNX models and tokenizer.json.
        data_path: Path to JSONL labeled data.
        val_fraction: Fraction to hold out for evaluation (if use_split=True).
        seed: Random seed for splitting.
        threshold: Sigmoid threshold for event classification.
        use_split: If True, split data and evaluate on val set only.
            If False, evaluate on all data (for a separate test set).

    Returns:
        EvaluationResult with metrics for available models.
    """
    examples = load_jsonl(data_path)

    if use_split:
        _, eval_examples = split_data(examples, val_fraction=val_fraction, seed=seed)
    else:
        eval_examples = examples

    tokenizer = load_tokenizer(model_dir)
    result = EvaluationResult(model_dir=str(model_dir), num_examples=len(eval_examples))

    # Event classification
    event_model_path = model_dir / "event_classifier.onnx"
    if event_model_path.exists():
        event_session = load_onnx_session(event_model_path)
        result.event_metrics = evaluate_event_classification(
            event_session, tokenizer, eval_examples, threshold=threshold
        )

    # NER
    ner_model_path = model_dir / "ner_classifier.onnx"
    if ner_model_path.exists():
        ner_session = load_onnx_session(ner_model_path)
        result.ner_metrics = evaluate_ner(ner_session, tokenizer, eval_examples)

    return result


# ---------------------------------------------------------------------------
# Report formatting
# ---------------------------------------------------------------------------


def format_report(result: EvaluationResult) -> str:
    """Format evaluation results as a human-readable report."""
    lines = []
    lines.append("=" * 70)
    lines.append("Event Classifier Evaluation Report")
    lines.append("=" * 70)
    lines.append(f"Model directory: {result.model_dir}")
    lines.append(f"Examples evaluated: {result.num_examples}")
    lines.append("")

    if result.event_metrics:
        em = result.event_metrics
        lines.append("-" * 70)
        lines.append("EVENT CLASSIFICATION")
        lines.append("-" * 70)
        lines.append(f"{'EventKind':<25} {'Precision':>10} {'Recall':>10} {'F1':>10}")
        lines.append("-" * 55)
        for kind in EVENT_KINDS:
            if kind in em.per_class:
                m = em.per_class[kind]
                lines.append(
                    f"{kind:<25} {m['precision']:>10.4f} {m['recall']:>10.4f} {m['f1']:>10.4f}"
                )
        lines.append("-" * 55)
        lines.append(
            f"{'MACRO'::<25} {em.macro_precision:>10.4f} {em.macro_recall:>10.4f}"
            f" {em.macro_f1:>10.4f}"
        )
        lines.append("")

    if result.ner_metrics:
        nm = result.ner_metrics
        lines.append("-" * 70)
        lines.append("ENTITY EXTRACTION (NER)")
        lines.append("-" * 70)
        lines.append(f"Entity Precision: {nm.entity_precision:.4f}")
        lines.append(f"Entity Recall:    {nm.entity_recall:.4f}")
        lines.append(f"Entity F1:        {nm.entity_f1:.4f}")
        if nm.seqeval_report:
            lines.append("")
            lines.append("Per-category breakdown:")
            lines.append(nm.seqeval_report)

    lines.append("=" * 70)
    return "\n".join(lines)


def result_to_json(result: EvaluationResult) -> dict:
    """Convert evaluation result to a JSON-serializable dict."""
    out: dict = {
        "model_dir": result.model_dir,
        "num_examples": result.num_examples,
    }
    if result.event_metrics:
        em = result.event_metrics
        out["event_classification"] = {
            "macro_f1": em.macro_f1,
            "macro_precision": em.macro_precision,
            "macro_recall": em.macro_recall,
            "per_class": em.per_class,
        }
    if result.ner_metrics:
        nm = result.ner_metrics
        out["ner"] = {
            "entity_f1": nm.entity_f1,
            "entity_precision": nm.entity_precision,
            "entity_recall": nm.entity_recall,
        }
    return out
