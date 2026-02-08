"""Tests for export.py — ONNX export with round-trip validation."""

from pathlib import Path

import numpy as np
import onnxruntime as ort

from training.export import export_onnx, validate_onnx
from training.feature_schema import (
    ACTION_HEAD_SIZE,
    EMOTION_HEAD_SIZE,
    SPEECH_HEAD_SIZE,
    THOUGHT_HEAD_SIZE,
    TOTAL_INPUT_FEATURES,
)
from training.model import CharacterPredictor


def test_export_creates_file(model: CharacterPredictor, tmp_path: Path):
    output = tmp_path / "test_model.onnx"
    result = export_onnx(model, output)
    assert result.exists()
    assert result.stat().st_size > 0


def test_export_onnx_shapes(model: CharacterPredictor, tmp_path: Path):
    output = tmp_path / "test_model.onnx"
    export_onnx(model, output)

    session = ort.InferenceSession(str(output))

    # Check input
    inputs = session.get_inputs()
    assert len(inputs) == 1
    assert inputs[0].name == "features"
    assert inputs[0].shape[1] == TOTAL_INPUT_FEATURES

    # Check outputs
    outputs = session.get_outputs()
    assert len(outputs) == 4
    names = [o.name for o in outputs]
    assert names == ["action", "speech", "thought", "emotion"]


def test_onnx_roundtrip(model: CharacterPredictor, tmp_path: Path):
    output = tmp_path / "test_model.onnx"
    export_onnx(model, output)

    # Should not raise
    validate_onnx(output, model, atol=1e-5)


def test_onnx_dynamic_batch(model: CharacterPredictor, tmp_path: Path):
    output = tmp_path / "test_model.onnx"
    export_onnx(model, output)

    session = ort.InferenceSession(str(output))

    # Test with different batch sizes
    for batch_size in [1, 4, 16]:
        test_input = np.random.randn(batch_size, TOTAL_INPUT_FEATURES).astype(np.float32)
        results = session.run(None, {"features": test_input})
        assert results[0].shape == (batch_size, ACTION_HEAD_SIZE)
        assert results[1].shape == (batch_size, SPEECH_HEAD_SIZE)
        assert results[2].shape == (batch_size, THOUGHT_HEAD_SIZE)
        assert results[3].shape == (batch_size, EMOTION_HEAD_SIZE)
