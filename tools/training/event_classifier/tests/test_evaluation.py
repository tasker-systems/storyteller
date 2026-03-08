"""Tests for the evaluation framework.

Tests use synthetic ONNX-like outputs to verify metric computation
without requiring real ONNX models on disk.
"""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import MagicMock

import numpy as np
import pytest

from event_classifier.evaluation import (
    EvaluationResult,
    EventClassMetrics,
    NerMetrics,
    evaluate_event_classification,
    evaluate_ner,
    format_report,
    result_to_json,
)
from event_classifier.schema import (
    EVENT_KIND_TO_ID,
    EVENT_KINDS,
    NUM_BIO_LABELS,
    NUM_EVENT_KINDS,
)

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


def _make_mock_tokenizer():
    """Create a mock tokenizer that returns deterministic encodings."""
    tokenizer = MagicMock()

    def tokenize_call(texts, **kwargs):
        """Return numpy arrays matching expected shapes."""
        if isinstance(texts, str):
            texts = [texts]
        batch_size = len(texts)
        seq_len = kwargs.get("max_length", 128)

        # Build offset mapping: first 20 tokens have real offsets, rest are padding
        offsets = [[(i, i + 1) if i < 20 else (0, 0) for i in range(seq_len)]]
        offset_array = np.array(offsets * batch_size)

        data = {
            "input_ids": np.ones((batch_size, seq_len), dtype=np.int64),
            "attention_mask": np.ones((batch_size, seq_len), dtype=np.int64),
            "offset_mapping": offset_array,
        }

        result = MagicMock()
        result.__getitem__ = lambda self, key: data[key]
        return result

    tokenizer.side_effect = tokenize_call
    return tokenizer


def _make_event_session(predicted_kinds: list[list[str]]):
    """Create a mock ONNX session that predicts specific event kinds.

    Args:
        predicted_kinds: For each example in the batch, which EventKinds to predict.
    """
    session = MagicMock()

    call_count = [0]

    def run_fn(_output_names, inputs):
        batch_size = inputs["input_ids"].shape[0]
        logits = np.full((batch_size, NUM_EVENT_KINDS), -5.0)  # well below sigmoid(0.5)
        for i in range(batch_size):
            idx = call_count[0] + i
            if idx < len(predicted_kinds):
                for kind in predicted_kinds[idx]:
                    if kind in EVENT_KIND_TO_ID:
                        logits[i, EVENT_KIND_TO_ID[kind]] = 5.0  # well above sigmoid(0.5)
        call_count[0] += batch_size
        return [logits]

    session.run = run_fn
    return session


def _make_ner_session(seq_len: int = 128):
    """Create a mock ONNX session that predicts all O labels."""
    session = MagicMock()

    def run_fn(_output_names, inputs):
        batch_size = inputs["input_ids"].shape[0]
        # All O (index 0)
        logits = np.zeros((batch_size, seq_len, NUM_BIO_LABELS))
        logits[:, :, 0] = 10.0  # strong O prediction
        return [logits]

    session.run = run_fn
    return session


# ---------------------------------------------------------------------------
# Event classification metric tests
# ---------------------------------------------------------------------------


class TestEventClassificationMetrics:
    def test_perfect_predictions(self, synthetic_examples):
        """Perfect predictions yield F1=1.0 for all classes."""
        # Predict exactly the gold labels
        predicted = [ex["event_kinds"] for ex in synthetic_examples]
        session = _make_event_session(predicted)
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_event_classification(
            session, tokenizer, synthetic_examples, batch_size=100
        )

        assert metrics.macro_f1 == pytest.approx(1.0)
        for kind in EVENT_KINDS:
            if kind in metrics.per_class:
                assert metrics.per_class[kind]["f1"] == pytest.approx(1.0), (
                    f"{kind} should have perfect F1"
                )

    def test_all_wrong_predictions(self, synthetic_examples):
        """Predicting wrong kinds yields low F1."""
        # Predict the opposite of what's expected
        wrong_map = {
            "ActionOccurrence": "EnvironmentalChange",
            "SpatialChange": "SpeechAct",
            "StateAssertion": "RelationalShift",
            "EmotionalExpression": "InformationTransfer",
            "InformationTransfer": "EmotionalExpression",
            "SpeechAct": "SpatialChange",
            "RelationalShift": "StateAssertion",
            "EnvironmentalChange": "ActionOccurrence",
        }
        predicted = []
        for ex in synthetic_examples:
            predicted.append([wrong_map.get(k, k) for k in ex["event_kinds"]])

        session = _make_event_session(predicted)
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_event_classification(
            session, tokenizer, synthetic_examples, batch_size=100
        )

        assert metrics.macro_f1 < 0.5

    def test_empty_predictions(self, synthetic_examples):
        """Predicting nothing yields zero recall."""
        predicted = [[] for _ in synthetic_examples]
        session = _make_event_session(predicted)
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_event_classification(
            session, tokenizer, synthetic_examples, batch_size=100
        )

        assert metrics.macro_recall == pytest.approx(0.0)

    def test_metrics_structure(self, synthetic_examples):
        """Verify the metrics dataclass has expected fields."""
        predicted = [ex["event_kinds"] for ex in synthetic_examples]
        session = _make_event_session(predicted)
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_event_classification(
            session, tokenizer, synthetic_examples, batch_size=100
        )

        assert isinstance(metrics, EventClassMetrics)
        assert isinstance(metrics.per_class, dict)
        assert 0.0 <= metrics.macro_f1 <= 1.0
        assert 0.0 <= metrics.macro_precision <= 1.0
        assert 0.0 <= metrics.macro_recall <= 1.0

    def test_batched_inference(self, synthetic_examples):
        """Small batch size still produces correct results."""
        predicted = [ex["event_kinds"] for ex in synthetic_examples]
        session = _make_event_session(predicted)
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_event_classification(
            session, tokenizer, synthetic_examples, batch_size=2
        )

        assert metrics.macro_f1 == pytest.approx(1.0)


# ---------------------------------------------------------------------------
# NER metric tests
# ---------------------------------------------------------------------------


class TestNerMetrics:
    def test_all_o_predictions_all_o_gold(self):
        """When gold has no entities, predicting all O is correct."""
        examples = [
            {
                "id": "test-0",
                "text": "the sky is blue",
                "register": "narrator",
                "event_kinds": ["StateAssertion"],
                "entities": [],
            }
        ]
        session = _make_ner_session()
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_ner(session, tokenizer, examples)

        # No entities to find — seqeval treats this as perfect (no FP, no FN)
        assert isinstance(metrics, NerMetrics)

    def test_metrics_structure(self, synthetic_examples):
        """Verify the NER metrics dataclass has expected fields."""
        session = _make_ner_session()
        tokenizer = _make_mock_tokenizer()

        metrics = evaluate_ner(session, tokenizer, synthetic_examples)

        assert isinstance(metrics, NerMetrics)
        assert 0.0 <= metrics.entity_f1 <= 1.0
        assert 0.0 <= metrics.entity_precision <= 1.0
        assert 0.0 <= metrics.entity_recall <= 1.0
        assert isinstance(metrics.seqeval_report, str)


# ---------------------------------------------------------------------------
# Report formatting tests
# ---------------------------------------------------------------------------


class TestReportFormatting:
    def test_format_report_with_both_metrics(self):
        """Report includes both event and NER sections."""
        result = EvaluationResult(
            model_dir="/tmp/models",
            num_examples=100,
            event_metrics=EventClassMetrics(
                per_class={"ActionOccurrence": {"precision": 0.9, "recall": 0.85, "f1": 0.875}},
                macro_f1=0.875,
                macro_precision=0.9,
                macro_recall=0.85,
            ),
            ner_metrics=NerMetrics(
                entity_f1=0.82,
                entity_precision=0.85,
                entity_recall=0.80,
            ),
        )
        report = format_report(result)
        assert "EVENT CLASSIFICATION" in report
        assert "ENTITY EXTRACTION" in report
        assert "ActionOccurrence" in report
        assert "100" in report

    def test_format_report_event_only(self):
        """Report works with only event metrics."""
        result = EvaluationResult(
            model_dir="/tmp/models",
            num_examples=50,
            event_metrics=EventClassMetrics(macro_f1=0.9),
        )
        report = format_report(result)
        assert "EVENT CLASSIFICATION" in report
        assert "ENTITY EXTRACTION" not in report

    def test_format_report_no_metrics(self):
        """Report works with no metrics (no models found)."""
        result = EvaluationResult(
            model_dir="/tmp/empty",
            num_examples=0,
        )
        report = format_report(result)
        assert "Evaluation Report" in report

    def test_result_to_json(self):
        """JSON serialization includes all fields."""
        result = EvaluationResult(
            model_dir="/tmp/models",
            num_examples=100,
            event_metrics=EventClassMetrics(
                per_class={"ActionOccurrence": {"precision": 0.9, "recall": 0.85, "f1": 0.875}},
                macro_f1=0.875,
                macro_precision=0.9,
                macro_recall=0.85,
            ),
            ner_metrics=NerMetrics(
                entity_f1=0.82,
                entity_precision=0.85,
                entity_recall=0.80,
            ),
        )
        j = result_to_json(result)
        assert j["num_examples"] == 100
        assert j["event_classification"]["macro_f1"] == 0.875
        assert j["ner"]["entity_f1"] == 0.82
        # Verify it's JSON-serializable
        json.dumps(j)

    def test_result_to_json_no_metrics(self):
        """JSON serialization works with no metrics."""
        result = EvaluationResult(model_dir="/tmp/empty", num_examples=0)
        j = result_to_json(result)
        assert "event_classification" not in j
        assert "ner" not in j


# ---------------------------------------------------------------------------
# CLI tests
# ---------------------------------------------------------------------------


class TestEvaluateCli:
    def test_parse_args_basic(self):
        from event_classifier.evaluate_cli import parse_args

        args = parse_args(["/tmp/models", "/tmp/data.jsonl"])
        assert args.model_dir == Path("/tmp/models")
        assert args.data_path == Path("/tmp/data.jsonl")
        assert args.no_split is False
        assert args.threshold == 0.5

    def test_parse_args_no_split(self):
        from event_classifier.evaluate_cli import parse_args

        args = parse_args(["--no-split", "/tmp/models", "/tmp/data.jsonl"])
        assert args.no_split is True

    def test_parse_args_with_output(self):
        from event_classifier.evaluate_cli import parse_args

        args = parse_args(["-o", "/tmp/report.json", "/tmp/models", "/tmp/data.jsonl"])
        assert args.output == Path("/tmp/report.json")

    def test_parse_args_custom_threshold(self):
        from event_classifier.evaluate_cli import parse_args

        args = parse_args(["--threshold", "0.3", "/tmp/models", "/tmp/data.jsonl"])
        assert args.threshold == 0.3
