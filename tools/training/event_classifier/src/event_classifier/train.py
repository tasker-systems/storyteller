"""Training pipeline using HuggingFace Trainer.

Supports both event classification (sequence-level, multi-label) and
NER (token-level, BIO tagging) with a single `train()` entry point.
"""

from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch
from torch import nn
from transformers import (
    AutoTokenizer,
    Trainer,
    TrainingArguments,
)

from event_classifier.dataset import (
    create_event_classification_dataset,
    create_ner_dataset,
    load_jsonl,
    split_data,
)
from event_classifier.metrics import compute_event_metrics, compute_ner_metrics
from event_classifier.models import load_event_classifier, load_ner_model
from event_classifier.schema import BIO_LABEL_TO_ID, IGNORE_INDEX, PRETRAINED_MODEL


@dataclass
class TrainConfig:
    """Configuration for model training."""

    task: str = "event"  # "event" or "ner"
    model_name: str = PRETRAINED_MODEL
    epochs: int = 5
    batch_size: int = 16
    lr: float = 2e-5
    warmup_ratio: float = 0.1
    weight_decay: float = 0.01
    seed: int = 42
    val_fraction: float = 0.15
    output_dir: str = "output"
    fp16: bool = False
    use_cpu: bool = False
    logging_steps: int = 50


class WeightedNerTrainer(Trainer):
    """Trainer subclass that applies inverse-frequency class weights for NER.

    O tokens dominate in NER data, so we weight entity tokens higher.
    """

    def __init__(self, class_weights: torch.Tensor | None = None, **kwargs):
        super().__init__(**kwargs)
        self._class_weights = class_weights

    def compute_loss(self, model, inputs, return_outputs=False, **kwargs):
        labels = inputs.pop("labels")
        outputs = model(**inputs)
        logits = outputs.logits

        if self._class_weights is not None:
            weight = self._class_weights.to(logits.device)
            loss_fn = nn.CrossEntropyLoss(weight=weight, ignore_index=IGNORE_INDEX)
        else:
            loss_fn = nn.CrossEntropyLoss(ignore_index=IGNORE_INDEX)

        # logits: [batch, seq_len, num_labels] → [batch*seq_len, num_labels]
        loss = loss_fn(logits.view(-1, logits.shape[-1]), labels.view(-1))
        return (loss, outputs) if return_outputs else loss


def _compute_ner_class_weights(train_dataset) -> torch.Tensor:
    """Compute inverse-frequency weights for NER classes."""
    all_labels = []
    for example in train_dataset:
        all_labels.extend(label for label in example["labels"] if label != IGNORE_INDEX)

    counts = np.bincount(all_labels, minlength=len(BIO_LABEL_TO_ID))
    # Avoid division by zero for unseen labels
    counts = np.maximum(counts, 1)
    total = counts.sum()
    weights = total / (len(counts) * counts)
    # Cap weights to prevent extreme values
    weights = np.minimum(weights, 10.0)
    return torch.tensor(weights, dtype=torch.float32)


def train(config: TrainConfig, data_path: Path) -> Path:
    """Train an event classification or NER model.

    Returns the path to the saved model directory.
    """
    print(f"Task: {config.task}")
    print(f"Model: {config.model_name}")
    print(f"Data: {data_path}")

    # Load data
    examples = load_jsonl(data_path)
    print(f"Loaded {len(examples)} examples")

    train_examples, val_examples = split_data(
        examples, val_fraction=config.val_fraction, seed=config.seed
    )
    print(f"Train: {len(train_examples)}, Val: {len(val_examples)}")

    # Load tokenizer
    tokenizer = AutoTokenizer.from_pretrained(config.model_name)

    # Create datasets and model
    if config.task == "event":
        train_dataset = create_event_classification_dataset(train_examples, tokenizer)
        val_dataset = create_event_classification_dataset(val_examples, tokenizer)
        model = load_event_classifier(config.model_name)
        compute_metrics = compute_event_metrics
        metric_for_best = "macro_f1"
    elif config.task == "ner":
        train_dataset = create_ner_dataset(train_examples, tokenizer)
        val_dataset = create_ner_dataset(val_examples, tokenizer)
        model = load_ner_model(config.model_name)
        compute_metrics = compute_ner_metrics
        metric_for_best = "entity_f1"
    else:
        raise ValueError(f"Unknown task: {config.task}")

    output_dir = Path(config.output_dir)
    training_args = TrainingArguments(
        output_dir=str(output_dir / "checkpoints"),
        num_train_epochs=config.epochs,
        per_device_train_batch_size=config.batch_size,
        per_device_eval_batch_size=config.batch_size * 2,
        learning_rate=config.lr,
        warmup_ratio=config.warmup_ratio,
        weight_decay=config.weight_decay,
        eval_strategy="epoch",
        save_strategy="epoch",
        load_best_model_at_end=True,
        metric_for_best_model=metric_for_best,
        greater_is_better=True,
        logging_steps=config.logging_steps,
        seed=config.seed,
        fp16=config.fp16,
        use_cpu=config.use_cpu,
        report_to="none",
        save_total_limit=2,
    )

    if config.task == "ner":
        class_weights = _compute_ner_class_weights(train_dataset)
        print(f"NER class weights: {class_weights.tolist()}")
        trainer = WeightedNerTrainer(
            class_weights=class_weights,
            model=model,
            args=training_args,
            train_dataset=train_dataset,
            eval_dataset=val_dataset,
            compute_metrics=compute_metrics,
        )
    else:
        trainer = Trainer(
            model=model,
            args=training_args,
            train_dataset=train_dataset,
            eval_dataset=val_dataset,
            compute_metrics=compute_metrics,
        )

    # Train
    trainer.train()

    # Evaluate
    eval_results = trainer.evaluate()
    print(f"\nEval results: {eval_results}")

    # Save best model + tokenizer
    model_dir = output_dir / "model"
    trainer.save_model(str(model_dir))
    tokenizer.save_pretrained(str(model_dir))
    print(f"Saved model to {model_dir}")

    return model_dir
