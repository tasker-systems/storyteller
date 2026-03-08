"""Metric computation for event classification and NER evaluation.

Event classification uses sigmoid + threshold + per-class and macro F1.
NER uses argmax + seqeval entity-level F1 (span-exact match).
"""

import numpy as np
from seqeval.metrics import f1_score as seqeval_f1
from sklearn.metrics import f1_score as sklearn_f1

from event_classifier.schema import ID_TO_BIO_LABEL, ID_TO_EVENT_KIND, IGNORE_INDEX


def compute_event_metrics(eval_pred: tuple) -> dict[str, float]:
    """Compute metrics for multi-label event classification.

    Args:
        eval_pred: (predictions, labels) tuple from HF Trainer.
            predictions: [batch, NUM_EVENT_KINDS] raw logits.
            labels: [batch, NUM_EVENT_KINDS] multi-hot float.

    Returns:
        Dict with per-class F1 and macro F1.
    """
    logits, labels = eval_pred
    probs = 1.0 / (1.0 + np.exp(-logits))  # sigmoid
    preds = (probs > 0.5).astype(int)
    labels = labels.astype(int)

    metrics = {}
    for i, kind in ID_TO_EVENT_KIND.items():
        f1 = sklearn_f1(labels[:, i], preds[:, i], zero_division=0.0)
        metrics[f"f1_{kind}"] = float(f1)

    metrics["macro_f1"] = float(sklearn_f1(labels, preds, average="macro", zero_division=0.0))
    return metrics


def compute_ner_metrics(eval_pred: tuple) -> dict[str, float]:
    """Compute metrics for NER using seqeval entity-level F1.

    Args:
        eval_pred: (predictions, labels) tuple from HF Trainer.
            predictions: [batch, seq_len, NUM_BIO_LABELS] logits.
            labels: [batch, seq_len] integer BIO label IDs.

    Returns:
        Dict with entity-level precision, recall, F1.
    """
    logits, labels = eval_pred
    preds = np.argmax(logits, axis=-1)

    true_sequences = []
    pred_sequences = []

    for pred_seq, label_seq in zip(preds, labels, strict=True):
        true_labels = []
        pred_labels = []
        for pred_id, label_id in zip(pred_seq, label_seq, strict=True):
            if label_id == IGNORE_INDEX:
                continue
            true_labels.append(ID_TO_BIO_LABEL[int(label_id)])
            pred_labels.append(ID_TO_BIO_LABEL[int(pred_id)])
        true_sequences.append(true_labels)
        pred_sequences.append(pred_labels)

    entity_f1 = seqeval_f1(true_sequences, pred_sequences, zero_division=0.0)

    return {
        "entity_f1": float(entity_f1),
    }
