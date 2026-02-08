"""Multi-head loss with per-region loss functions.

Each head's output is sliced from the flat 42-element label vector and
matched with the appropriate loss function:
- CrossEntropyLoss for categorical one-hot regions (argmax to class index)
- BCEWithLogitsLoss for binary outputs
- MSELoss for continuous values
"""

from torch import Tensor, nn

from training.feature_schema import (
    ACTION_CONFIDENCE_IDX,
    ACTION_CONTEXT_SLICE,
    ACTION_TARGET_IDX,
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


def _onehot_to_class(labels: Tensor, s: slice) -> Tensor:
    """Convert one-hot slice of labels to class indices for CrossEntropyLoss."""
    return labels[:, s].argmax(dim=1)


class MultiHeadLoss(nn.Module):
    """Combined loss across all four prediction heads."""

    def __init__(
        self,
        action_weight: float = 0.35,
        speech_weight: float = 0.20,
        thought_weight: float = 0.20,
        emotion_weight: float = 0.25,
    ) -> None:
        super().__init__()
        self.action_weight = action_weight
        self.speech_weight = speech_weight
        self.thought_weight = thought_weight
        self.emotion_weight = emotion_weight

        self.ce = nn.CrossEntropyLoss()
        self.bce = nn.BCEWithLogitsLoss()
        self.mse = nn.MSELoss()

    def forward(self, predictions: dict[str, Tensor], labels: Tensor) -> dict[str, Tensor]:
        # --- Action head (offset 0, length 14) ---
        action_pred = predictions["action"]
        action_type_loss = self.ce(action_pred[:, :6], _onehot_to_class(labels, ACTION_TYPE_SLICE))
        action_conf_loss = self.mse(action_pred[:, 6], labels[:, ACTION_CONFIDENCE_IDX])
        action_target_loss = self.mse(action_pred[:, 7], labels[:, ACTION_TARGET_IDX])
        action_valence_loss = self.mse(action_pred[:, 8], labels[:, ACTION_VALENCE_IDX])
        action_context_loss = self.ce(
            action_pred[:, 9:14], _onehot_to_class(labels, ACTION_CONTEXT_SLICE)
        )
        action_loss = (
            action_type_loss
            + action_conf_loss
            + action_target_loss
            + action_valence_loss
            + action_context_loss
        )

        # --- Speech head (offset 14, length 6) ---
        speech_pred = predictions["speech"]
        speech_occurs_loss = self.bce(speech_pred[:, 0], labels[:, SPEECH_OCCURS_IDX])
        speech_register_loss = self.ce(
            speech_pred[:, 1:5], _onehot_to_class(labels, SPEECH_REGISTER_SLICE)
        )
        speech_conf_loss = self.mse(speech_pred[:, 5], labels[:, SPEECH_CONFIDENCE_IDX])
        speech_loss = speech_occurs_loss + speech_register_loss + speech_conf_loss

        # --- Thought head (offset 20, length 6) ---
        thought_pred = predictions["thought"]
        awareness_loss = self.ce(
            thought_pred[:, :5], _onehot_to_class(labels, AWARENESS_LEVEL_SLICE)
        )
        dominant_emotion_loss = self.mse(thought_pred[:, 5], labels[:, DOMINANT_EMOTION_IDX])
        thought_loss = awareness_loss + dominant_emotion_loss

        # --- Emotion head (offset 26, length 16) ---
        emotion_pred = predictions["emotion"]
        delta_loss = self.mse(emotion_pred[:, :8], labels[:, INTENSITY_DELTA_SLICE])
        shift_loss = self.bce(emotion_pred[:, 8:16], labels[:, AWARENESS_SHIFT_SLICE])
        emotion_loss = delta_loss + shift_loss

        # Weighted total
        total = (
            self.action_weight * action_loss
            + self.speech_weight * speech_loss
            + self.thought_weight * thought_loss
            + self.emotion_weight * emotion_loss
        )

        return {
            "total": total,
            "action": action_loss.detach(),
            "speech": speech_loss.detach(),
            "thought": thought_loss.detach(),
            "emotion": emotion_loss.detach(),
        }
