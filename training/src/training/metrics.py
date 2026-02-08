"""Per-head metric accumulation and reporting."""

import torch
from torch import Tensor

from training.feature_schema import (
    ACTION_CONFIDENCE_IDX,
    ACTION_CONTEXT_SLICE,
    ACTION_TYPE_SLICE,
    ACTION_VALENCE_IDX,
    AWARENESS_LEVEL_SLICE,
    AWARENESS_SHIFT_SLICE,
    DOMINANT_EMOTION_IDX,
    INTENSITY_DELTA_SLICE,
    SPEECH_CONFIDENCE_IDX,
    SPEECH_OCCURS_IDX,
    SPEECH_REGISTER_SLICE,
)


class MetricsAccumulator:
    """Accumulates predictions and labels across batches for per-head metrics."""

    def __init__(self) -> None:
        self._action_preds: list[Tensor] = []
        self._speech_preds: list[Tensor] = []
        self._thought_preds: list[Tensor] = []
        self._emotion_preds: list[Tensor] = []
        self._labels: list[Tensor] = []

    def update(self, predictions: dict[str, Tensor], labels: Tensor) -> None:
        self._action_preds.append(predictions["action"].detach().cpu())
        self._speech_preds.append(predictions["speech"].detach().cpu())
        self._thought_preds.append(predictions["thought"].detach().cpu())
        self._emotion_preds.append(predictions["emotion"].detach().cpu())
        self._labels.append(labels.detach().cpu())

    def compute(self) -> dict[str, float]:
        if not self._labels:
            return {}

        action = torch.cat(self._action_preds)
        speech = torch.cat(self._speech_preds)
        thought = torch.cat(self._thought_preds)
        emotion = torch.cat(self._emotion_preds)
        labels = torch.cat(self._labels)

        metrics: dict[str, float] = {}

        # --- Action head ---
        # Type accuracy (argmax over first 6 logits vs argmax of one-hot label)
        pred_types = action[:, :6].argmax(dim=1)
        true_types = labels[:, ACTION_TYPE_SLICE].argmax(dim=1)
        metrics["action_type_acc"] = (pred_types == true_types).float().mean().item()

        # Context accuracy
        pred_ctx = action[:, 9:14].argmax(dim=1)
        true_ctx = labels[:, ACTION_CONTEXT_SLICE].argmax(dim=1)
        metrics["action_context_acc"] = (pred_ctx == true_ctx).float().mean().item()

        # Confidence MSE
        metrics["action_confidence_mse"] = (
            (action[:, 6] - labels[:, ACTION_CONFIDENCE_IDX]).pow(2).mean().item()
        )

        # Valence MSE
        metrics["action_valence_mse"] = (
            (action[:, 8] - labels[:, ACTION_VALENCE_IDX]).pow(2).mean().item()
        )

        # --- Speech head ---
        # Occurs binary accuracy
        pred_occurs = (speech[:, 0] > 0).float()
        true_occurs = (labels[:, SPEECH_OCCURS_IDX] > 0.5).float()
        metrics["speech_occurs_acc"] = (pred_occurs == true_occurs).float().mean().item()

        # Register accuracy
        pred_reg = speech[:, 1:5].argmax(dim=1)
        true_reg = labels[:, SPEECH_REGISTER_SLICE].argmax(dim=1)
        metrics["speech_register_acc"] = (pred_reg == true_reg).float().mean().item()

        # Confidence MSE
        metrics["speech_confidence_mse"] = (
            (speech[:, 5] - labels[:, SPEECH_CONFIDENCE_IDX]).pow(2).mean().item()
        )

        # --- Thought head ---
        # Awareness accuracy
        pred_aware = thought[:, :5].argmax(dim=1)
        true_aware = labels[:, AWARENESS_LEVEL_SLICE].argmax(dim=1)
        metrics["thought_awareness_acc"] = (pred_aware == true_aware).float().mean().item()

        # Dominant emotion MAE
        metrics["thought_emotion_mae"] = (
            (thought[:, 5] - labels[:, DOMINANT_EMOTION_IDX]).abs().mean().item()
        )

        # --- Emotion head ---
        # Delta MSE (mean across 8 primaries)
        metrics["emotion_delta_mse"] = (
            (emotion[:, :8] - labels[:, INTENSITY_DELTA_SLICE]).pow(2).mean().item()
        )

        # Shift binary accuracy (mean across 8 primaries)
        pred_shifts = (emotion[:, 8:16] > 0).float()
        true_shifts = (labels[:, AWARENESS_SHIFT_SLICE] > 0.5).float()
        metrics["emotion_shift_acc"] = (pred_shifts == true_shifts).float().mean().item()

        return metrics

    def reset(self) -> None:
        self._action_preds.clear()
        self._speech_preds.clear()
        self._thought_preds.clear()
        self._emotion_preds.clear()
        self._labels.clear()
