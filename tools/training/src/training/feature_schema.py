"""Feature encoding schema — shared contract between Rust and Python.

Constants transliterated from storyteller-ml/src/feature_schema.rs (lines 40-113).
Both the Rust inference path (ort) and the Python training path (PyTorch) must
agree on this encoding.
"""

# ---------------------------------------------------------------------------
# Input dimension constants
# ---------------------------------------------------------------------------

MAX_TENSOR_AXES = 16
FEATURES_PER_AXIS = 4 + 4 + 5  # AxisValue(4) + TemporalLayer(4) + Provenance(5) = 13

MAX_EDGES = 5
FEATURES_PER_EDGE = 5 * 4 + 4  # 5 substrate dims × AxisValue(4) + TopologicalRole(4) = 24

NUM_PRIMARIES = 8
FEATURES_PER_PRIMARY = 1 + 5  # intensity(1) + AwarenessLevel(5) = 6

SELF_EDGE_FEATURES = 7  # trust(3) + affection + debt + history_weight + projection_accuracy

SCENE_FEATURES = 6  # SceneType(4) + cast_size + tension

EVENT_FEATURES = 16  # EventType(7) + EmotionalRegister(7) + confidence + target_count

FEATURES_PER_HISTORY = 16  # ActionType(6) + SpeechRegister(4) + AwarenessLevel(5) + valence
HISTORY_DEPTH = 3

TOTAL_INPUT_FEATURES = (
    MAX_TENSOR_AXES * FEATURES_PER_AXIS  # 208
    + NUM_PRIMARIES * FEATURES_PER_PRIMARY  # 48
    + SELF_EDGE_FEATURES  # 7
    + MAX_EDGES * FEATURES_PER_EDGE  # 120
    + SCENE_FEATURES  # 6
    + EVENT_FEATURES  # 16
    + HISTORY_DEPTH * FEATURES_PER_HISTORY  # 48
)  # Total: 453

# ---------------------------------------------------------------------------
# Output dimension constants
# ---------------------------------------------------------------------------

NUM_ACTION_TYPES = 6
NUM_ACTION_CONTEXTS = 5
NUM_SPEECH_REGISTERS = 4
NUM_AWARENESS_LEVELS = 5

TOTAL_OUTPUT_FEATURES = (
    NUM_ACTION_TYPES
    + 1
    + 1
    + 1
    + NUM_ACTION_CONTEXTS  # 14: action head
    + 1
    + NUM_SPEECH_REGISTERS
    + 1  # 6: speech head
    + NUM_AWARENESS_LEVELS
    + 1  # 6: thought head
    + NUM_PRIMARIES
    + NUM_PRIMARIES  # 16: emotion head
)  # Total: 42

# Per-head output sizes (for model architecture)
ACTION_HEAD_SIZE = NUM_ACTION_TYPES + 1 + 1 + 1 + NUM_ACTION_CONTEXTS  # 14
SPEECH_HEAD_SIZE = 1 + NUM_SPEECH_REGISTERS + 1  # 6
THOUGHT_HEAD_SIZE = NUM_AWARENESS_LEVELS + 1  # 6
EMOTION_HEAD_SIZE = NUM_PRIMARIES + NUM_PRIMARIES  # 16

# ---------------------------------------------------------------------------
# Output label slicing — absolute offsets into the 42-element label vector
# ---------------------------------------------------------------------------

# Action head (offset 0, length 14)
ACTION_TYPE_SLICE = slice(0, 6)
ACTION_CONFIDENCE_IDX = 6
ACTION_TARGET_IDX = 7
ACTION_VALENCE_IDX = 8
ACTION_CONTEXT_SLICE = slice(9, 14)

# Speech head (offset 14, length 6)
SPEECH_OCCURS_IDX = 14
SPEECH_REGISTER_SLICE = slice(15, 19)
SPEECH_CONFIDENCE_IDX = 19

# Thought head (offset 20, length 6)
AWARENESS_LEVEL_SLICE = slice(20, 25)
DOMINANT_EMOTION_IDX = 25

# Emotion head (offset 26, length 16)
INTENSITY_DELTA_SLICE = slice(26, 34)
AWARENESS_SHIFT_SLICE = slice(34, 42)

# ---------------------------------------------------------------------------
# Enum name lists — for human-readable reporting
# ---------------------------------------------------------------------------

ACTION_TYPES = ["Perform", "Speak", "Move", "Examine", "Wait", "Resist"]
ACTION_CONTEXTS = [
    "SharedHistory",
    "CurrentScene",
    "EmotionalReaction",
    "RelationalDynamic",
    "WorldResponse",
]
SPEECH_REGISTERS = ["Whisper", "Conversational", "Declamatory", "Internal"]
AWARENESS_LEVELS = ["Articulate", "Recognizable", "Preconscious", "Defended", "Structural"]
EMOTIONAL_PRIMARIES = [
    "Joy",
    "Trust",
    "Fear",
    "Surprise",
    "Sadness",
    "Disgust",
    "Anger",
    "Anticipation",
]

# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------


def verify_dimensions(features: list | tuple, labels: list | tuple) -> None:
    """Assert that feature and label vectors match the expected schema dimensions."""
    assert len(features) == TOTAL_INPUT_FEATURES, (
        f"Feature vector has {len(features)} elements, expected {TOTAL_INPUT_FEATURES}"
    )
    assert len(labels) == TOTAL_OUTPUT_FEATURES, (
        f"Label vector has {len(labels)} elements, expected {TOTAL_OUTPUT_FEATURES}"
    )
