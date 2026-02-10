"""Label vocabularies and model constants for event classification and NER.

Must stay aligned with:
- Rust `EventKind` in storyteller-core/src/types/event_grammar.rs
- Rust `NerCategory` in storyteller-ml/src/event_templates/mod.rs
- Training data JSONL from storyteller-ml generate_event_training_data
"""

# ---------------------------------------------------------------------------
# Event classification labels (sequence-level, multi-label)
# ---------------------------------------------------------------------------

# The 8 classifiable EventKinds. SceneLifecycle and EntityLifecycle are
# system-generated events, not extracted from text by the classifier.
EVENT_KINDS: list[str] = [
    "StateAssertion",
    "ActionOccurrence",
    "SpatialChange",
    "EmotionalExpression",
    "InformationTransfer",
    "SpeechAct",
    "RelationalShift",
    "EnvironmentalChange",
]

NUM_EVENT_KINDS: int = len(EVENT_KINDS)

EVENT_KIND_TO_ID: dict[str, int] = {kind: i for i, kind in enumerate(EVENT_KINDS)}
ID_TO_EVENT_KIND: dict[int, str] = dict(enumerate(EVENT_KINDS))

# ---------------------------------------------------------------------------
# NER labels (token-level, BIO tagging)
# ---------------------------------------------------------------------------

# 7 entity categories from storyteller-ml NerCategory enum.
NER_CATEGORIES: list[str] = [
    "CHARACTER",
    "OBJECT",
    "LOCATION",
    "GESTURE",
    "SENSORY",
    "ABSTRACT",
    "COLLECTIVE",
]

# BIO label scheme: O + B-{cat} + I-{cat} for each category = 1 + 7*2 = 15
BIO_LABELS: list[str] = ["O"]
for _cat in NER_CATEGORIES:
    BIO_LABELS.append(f"B-{_cat}")
    BIO_LABELS.append(f"I-{_cat}")

NUM_BIO_LABELS: int = len(BIO_LABELS)

BIO_LABEL_TO_ID: dict[str, int] = {label: i for i, label in enumerate(BIO_LABELS)}
ID_TO_BIO_LABEL: dict[int, str] = dict(enumerate(BIO_LABELS))

# ---------------------------------------------------------------------------
# Model constants
# ---------------------------------------------------------------------------

PRETRAINED_MODEL: str = "microsoft/deberta-v3-small"
FALLBACK_MODEL: str = "distilbert-base-uncased"

MAX_SEQ_LENGTH: int = 128

# Special label ID for tokens that should be ignored in loss computation
# (CLS, SEP, PAD, subword continuations in NER).
IGNORE_INDEX: int = -100
