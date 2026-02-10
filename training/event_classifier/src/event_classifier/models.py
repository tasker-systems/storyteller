"""Model loading for event classification and NER.

Thin wrappers around HuggingFace AutoModel classes with the correct
configuration for our label vocabularies.
"""

from transformers import (
    AutoModelForSequenceClassification,
    AutoModelForTokenClassification,
    PreTrainedModel,
)

from event_classifier.schema import (
    BIO_LABEL_TO_ID,
    EVENT_KIND_TO_ID,
    ID_TO_BIO_LABEL,
    ID_TO_EVENT_KIND,
    NUM_BIO_LABELS,
    NUM_EVENT_KINDS,
    PRETRAINED_MODEL,
)


def load_event_classifier(
    model_name: str = PRETRAINED_MODEL,
) -> PreTrainedModel:
    """Load a sequence classification model for multi-label event classification.

    Returns a model with output shape [batch, NUM_EVENT_KINDS] and
    problem_type="multi_label_classification" (uses BCEWithLogitsLoss).
    """
    return AutoModelForSequenceClassification.from_pretrained(
        model_name,
        num_labels=NUM_EVENT_KINDS,
        problem_type="multi_label_classification",
        id2label=ID_TO_EVENT_KIND,
        label2id=EVENT_KIND_TO_ID,
    )


def load_ner_model(
    model_name: str = PRETRAINED_MODEL,
) -> PreTrainedModel:
    """Load a token classification model for NER with BIO tagging.

    Returns a model with output shape [batch, seq_len, NUM_BIO_LABELS].
    """
    return AutoModelForTokenClassification.from_pretrained(
        model_name,
        num_labels=NUM_BIO_LABELS,
        id2label=ID_TO_BIO_LABEL,
        label2id=BIO_LABEL_TO_ID,
    )
