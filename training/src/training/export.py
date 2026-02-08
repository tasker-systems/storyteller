"""ONNX export with named I/O and validation against PyTorch output."""

from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
import torch
from torch import Tensor, nn

from training.feature_schema import (
    ACTION_HEAD_SIZE,
    EMOTION_HEAD_SIZE,
    SPEECH_HEAD_SIZE,
    THOUGHT_HEAD_SIZE,
    TOTAL_INPUT_FEATURES,
)
from training.model import CharacterPredictor


class _ExportWrapper(nn.Module):
    """Wraps CharacterPredictor to produce tuple output for ONNX export."""

    def __init__(self, model: CharacterPredictor) -> None:
        super().__init__()
        self.model = model

    def forward(self, x: Tensor) -> tuple[Tensor, Tensor, Tensor, Tensor]:
        out = self.model(x)
        return out["action"], out["speech"], out["thought"], out["emotion"]


def export_onnx(
    model: CharacterPredictor,
    output_path: str | Path,
    opset_version: int = 18,
) -> Path:
    """Export model to ONNX with named I/O and dynamic batch axis."""
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    model.eval()
    wrapper = _ExportWrapper(model)
    wrapper.eval()
    dummy_input = torch.randn(1, TOTAL_INPUT_FEATURES)

    batch = torch.export.Dim("batch", min=1)
    torch.onnx.export(
        wrapper,
        (dummy_input,),
        str(output_path),
        input_names=["features"],
        output_names=["action", "speech", "thought", "emotion"],
        dynamic_shapes={"x": {0: batch}},
        opset_version=opset_version,
    )

    # Validate ONNX model structure
    onnx_model = onnx.load(str(output_path))
    onnx.checker.check_model(onnx_model)

    print(f"Exported ONNX model to {output_path}")
    print(f"  Input: features [{TOTAL_INPUT_FEATURES}]")
    print(
        f"  Outputs: action [{ACTION_HEAD_SIZE}], speech [{SPEECH_HEAD_SIZE}], "
        f"thought [{THOUGHT_HEAD_SIZE}], emotion [{EMOTION_HEAD_SIZE}]"
    )
    return output_path


def validate_onnx(
    onnx_path: str | Path,
    model: CharacterPredictor,
    atol: float = 1e-5,
) -> None:
    """Compare PyTorch and onnxruntime outputs for random input."""
    model.eval()

    # Generate random input
    test_input = torch.randn(4, TOTAL_INPUT_FEATURES)

    # PyTorch output
    with torch.no_grad():
        pt_out = model(test_input)

    # ONNX Runtime output
    session = ort.InferenceSession(str(onnx_path))
    ort_inputs = {"features": test_input.numpy()}
    ort_outputs = session.run(None, ort_inputs)

    # Compare
    head_names = ["action", "speech", "thought", "emotion"]
    for i, name in enumerate(head_names):
        pt_array = pt_out[name].numpy()
        ort_array = ort_outputs[i]
        max_diff = np.abs(pt_array - ort_array).max()
        if not np.allclose(pt_array, ort_array, atol=atol):
            raise ValueError(f"ONNX validation failed for {name} head: max diff = {max_diff:.2e}")
        print(f"  {name}: max diff = {max_diff:.2e} (OK)")

    print("ONNX validation passed!")
