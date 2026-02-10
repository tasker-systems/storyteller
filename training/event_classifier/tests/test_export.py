"""Tests for ONNX export and validation.

Uses distilbert-base-uncased for test fixtures (DeBERTa's disentangled
attention has dimension constraints that make tiny test models impractical).
Production export uses the actual trained model via Optimum.
"""

import numpy as np
import onnxruntime as ort
import pytest
from transformers import AutoConfig, AutoModelForSequenceClassification, AutoTokenizer

from event_classifier.export import _copy_tokenizer, export_onnx, validate_onnx
from event_classifier.schema import FALLBACK_MODEL, MAX_SEQ_LENGTH, NUM_EVENT_KINDS


@pytest.fixture(scope="module")
def tiny_model_dir(tmp_path_factory):
    """Create a tiny random event classification model for testing export."""
    model_dir = tmp_path_factory.mktemp("tiny_model")

    config = AutoConfig.from_pretrained(
        FALLBACK_MODEL,
        num_labels=NUM_EVENT_KINDS,
        problem_type="multi_label_classification",
    )
    config.n_layers = 1
    config.n_heads = 2
    config.dim = 64
    config.hidden_dim = 128

    model = AutoModelForSequenceClassification.from_config(config)
    model.save_pretrained(str(model_dir))

    tokenizer = AutoTokenizer.from_pretrained(FALLBACK_MODEL)
    tokenizer.save_pretrained(str(model_dir))

    return model_dir


def test_export_onnx_produces_file(tiny_model_dir, tmp_path):
    """export_onnx should produce a valid ONNX file."""
    onnx_path = export_onnx(tiny_model_dir, tmp_path, "event")
    assert onnx_path.exists()

    # Should be loadable by ORT
    session = ort.InferenceSession(str(onnx_path))
    input_names = [inp.name for inp in session.get_inputs()]
    assert "input_ids" in input_names
    assert "attention_mask" in input_names


def test_export_onnx_output_shape(tiny_model_dir, tmp_path):
    """Exported ONNX model should produce correct output shape."""
    onnx_path = export_onnx(tiny_model_dir, tmp_path, "event")
    session = ort.InferenceSession(str(onnx_path))

    batch_size = 2
    dummy_ids = np.ones((batch_size, MAX_SEQ_LENGTH), dtype=np.int64)
    dummy_mask = np.ones((batch_size, MAX_SEQ_LENGTH), dtype=np.int64)

    outputs = session.run(None, {"input_ids": dummy_ids, "attention_mask": dummy_mask})
    assert outputs[0].shape == (batch_size, NUM_EVENT_KINDS)


def test_copy_tokenizer(tiny_model_dir, tmp_path):
    """Tokenizer files should be copied to output directory."""
    _copy_tokenizer(tiny_model_dir, tmp_path)
    assert (tmp_path / "tokenizer_config.json").exists()


def test_validate_onnx_passes(tiny_model_dir, tmp_path):
    """Validation should pass for a freshly exported model."""
    onnx_path = export_onnx(tiny_model_dir, tmp_path, "event")
    # Should not raise
    validate_onnx(onnx_path, tiny_model_dir, "event", atol=1e-4)
