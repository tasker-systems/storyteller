"""CharacterPredictor — multi-head MLP for character behavior prediction.

Architecture:
    Input (453) → Shared trunk (384→256→256) → 4 prediction heads
    - Action head:  256→64→14  (action type, confidence, target, valence, context)
    - Speech head:  256→64→6   (occurs, register, confidence)
    - Thought head: 256→64→6   (awareness level, dominant emotion)
    - Emotion head: 256→64→16  (intensity deltas, awareness shifts)

All outputs are raw logits — no activation functions in the model.
Softmax/sigmoid/tanh applied at decode time.
"""

from torch import Tensor, nn

from training.feature_schema import (
    ACTION_HEAD_SIZE,
    EMOTION_HEAD_SIZE,
    SPEECH_HEAD_SIZE,
    THOUGHT_HEAD_SIZE,
    TOTAL_INPUT_FEATURES,
)


class CharacterPredictor(nn.Module):
    """Multi-head MLP for predicting character behavior from tensor features."""

    def __init__(
        self,
        input_dim: int = TOTAL_INPUT_FEATURES,
        trunk_dims: tuple[int, ...] = (384, 256, 256),
        head_hidden: int = 64,
        dropout: float = 0.3,
    ) -> None:
        super().__init__()

        # Shared trunk
        trunk_layers: list[nn.Module] = []
        prev_dim = input_dim
        for i, dim in enumerate(trunk_dims):
            trunk_layers.append(nn.Linear(prev_dim, dim))
            trunk_layers.append(nn.ReLU())
            # Dropout on all but the last trunk layer
            if i < len(trunk_dims) - 1:
                trunk_layers.append(nn.Dropout(dropout))
            prev_dim = dim
        self.trunk = nn.Sequential(*trunk_layers)

        trunk_out = trunk_dims[-1]

        # Prediction heads — each is Linear→ReLU→Linear (raw logits)
        self.action_head = nn.Sequential(
            nn.Linear(trunk_out, head_hidden),
            nn.ReLU(),
            nn.Linear(head_hidden, ACTION_HEAD_SIZE),
        )
        self.speech_head = nn.Sequential(
            nn.Linear(trunk_out, head_hidden),
            nn.ReLU(),
            nn.Linear(head_hidden, SPEECH_HEAD_SIZE),
        )
        self.thought_head = nn.Sequential(
            nn.Linear(trunk_out, head_hidden),
            nn.ReLU(),
            nn.Linear(head_hidden, THOUGHT_HEAD_SIZE),
        )
        self.emotion_head = nn.Sequential(
            nn.Linear(trunk_out, head_hidden),
            nn.ReLU(),
            nn.Linear(head_hidden, EMOTION_HEAD_SIZE),
        )

    def forward(self, x: Tensor) -> dict[str, Tensor]:
        shared = self.trunk(x)
        return {
            "action": self.action_head(shared),
            "speech": self.speech_head(shared),
            "thought": self.thought_head(shared),
            "emotion": self.emotion_head(shared),
        }
