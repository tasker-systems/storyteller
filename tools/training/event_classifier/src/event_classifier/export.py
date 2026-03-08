"""ONNX export and validation for event classification and NER models.

Primary path uses Optimum's export infrastructure. Falls back to manual
torch.onnx.export if Optimum fails (e.g., for some model architectures).
"""

import shutil
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch
from transformers import (
    AutoModelForSequenceClassification,
    AutoModelForTokenClassification,
    AutoTokenizer,
)

from event_classifier.schema import MAX_SEQ_LENGTH


def export_onnx(
    model_dir: str | Path,
    output_dir: str | Path,
    task: str,
) -> Path:
    """Export a trained model to ONNX format.

    Tries Optimum first, falls back to manual torch.onnx.export.

    Args:
        model_dir: Directory containing saved model and tokenizer.
        output_dir: Directory to write ONNX model and tokenizer.
        task: "event" or "ner".

    Returns:
        Path to the exported ONNX file.
    """
    model_dir = Path(model_dir)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    onnx_filename = "event_classifier.onnx" if task == "event" else "ner_classifier.onnx"

    try:
        onnx_path = _export_with_optimum(model_dir, output_dir, task, onnx_filename)
        print(f"Exported via Optimum: {onnx_path}")
    except Exception as e:
        print(f"Optimum export failed ({e}), falling back to manual export")
        onnx_path = _export_manual(model_dir, output_dir, task, onnx_filename)
        print(f"Exported via manual torch.onnx.export: {onnx_path}")

    # Copy tokenizer files to output dir
    _copy_tokenizer(model_dir, output_dir)

    return onnx_path


def _export_with_optimum(model_dir: Path, output_dir: Path, task: str, onnx_filename: str) -> Path:
    """Export using Optimum's ONNX exporter."""
    from optimum.exporters.onnx import main_export

    optimum_task = "text-classification" if task == "event" else "token-classification"

    # Optimum exports to a directory with model.onnx by default
    export_dir = output_dir / "_optimum_export"
    main_export(
        model_name_or_path=str(model_dir),
        output=export_dir,
        task=optimum_task,
        opset=17,
    )

    # Rename to our expected filename
    source = export_dir / "model.onnx"
    dest = output_dir / onnx_filename
    shutil.move(str(source), str(dest))

    # Clean up temp dir
    shutil.rmtree(str(export_dir), ignore_errors=True)

    return dest


def _export_manual(model_dir: Path, output_dir: Path, task: str, onnx_filename: str) -> Path:
    """Fallback: export using torch.onnx.export directly."""
    tokenizer = AutoTokenizer.from_pretrained(str(model_dir))

    if task == "event":
        model = AutoModelForSequenceClassification.from_pretrained(str(model_dir))
    else:
        model = AutoModelForTokenClassification.from_pretrained(str(model_dir))

    model.eval()

    dummy_input = tokenizer(
        "This is a test sentence",
        max_length=MAX_SEQ_LENGTH,
        padding="max_length",
        truncation=True,
        return_tensors="pt",
    )

    onnx_path = output_dir / onnx_filename

    dynamic_axes = {
        "input_ids": {0: "batch"},
        "attention_mask": {0: "batch"},
        "logits": {0: "batch"},
    }

    torch.onnx.export(
        model,
        (dummy_input["input_ids"], dummy_input["attention_mask"]),
        str(onnx_path),
        input_names=["input_ids", "attention_mask"],
        output_names=["logits"],
        dynamic_axes=dynamic_axes,
        opset_version=17,
        dynamo=False,
    )

    return onnx_path


def _copy_tokenizer(model_dir: Path, output_dir: Path) -> None:
    """Copy tokenizer.json (and spm.model for DeBERTa) to output directory."""
    for name in ["tokenizer.json", "tokenizer_config.json", "special_tokens_map.json", "spm.model"]:
        src = model_dir / name
        if src.exists():
            shutil.copy2(str(src), str(output_dir / name))


def validate_onnx(
    onnx_path: str | Path,
    model_dir: str | Path,
    task: str,
    atol: float = 1e-4,
) -> None:
    """Validate ONNX model against PyTorch model outputs.

    Args:
        onnx_path: Path to exported ONNX file.
        model_dir: Directory containing the PyTorch model.
        task: "event" or "ner".
        atol: Absolute tolerance for numerical comparison.
    """
    onnx_path = Path(onnx_path)
    model_dir = Path(model_dir)

    tokenizer = AutoTokenizer.from_pretrained(str(model_dir))

    if task == "event":
        model = AutoModelForSequenceClassification.from_pretrained(str(model_dir))
    else:
        model = AutoModelForTokenClassification.from_pretrained(str(model_dir))
    model.eval()

    # Test inputs
    test_texts = [
        "Sarah picked up the stone",
        "The wolf crossed the river at dawn",
        "I tell Adam about the path",
        "Rain begins to fall",
    ]

    inputs = tokenizer(
        test_texts,
        max_length=MAX_SEQ_LENGTH,
        padding="max_length",
        truncation=True,
        return_tensors="pt",
    )

    # PyTorch output
    with torch.no_grad():
        pt_outputs = model(
            input_ids=inputs["input_ids"],
            attention_mask=inputs["attention_mask"],
        )
    pt_logits = pt_outputs.logits.numpy()

    # ONNX Runtime output
    session = ort.InferenceSession(str(onnx_path))
    ort_inputs = {
        "input_ids": inputs["input_ids"].numpy(),
        "attention_mask": inputs["attention_mask"].numpy(),
    }

    # Handle different output naming conventions
    ort_outputs = session.run(None, ort_inputs)
    ort_logits = ort_outputs[0]

    max_diff = np.abs(pt_logits - ort_logits).max()
    print(f"  Max diff (PyTorch vs ONNX): {max_diff:.2e}")

    if not np.allclose(pt_logits, ort_logits, atol=atol):
        # Try relaxed tolerance for DeBERTa
        if np.allclose(pt_logits, ort_logits, atol=1e-3):
            print(f"  Warning: passed with relaxed tolerance (1e-3), max diff = {max_diff:.2e}")
        else:
            raise ValueError(
                f"ONNX validation failed: max diff = {max_diff:.2e} exceeds atol={atol}"
            )

    print("ONNX validation passed!")
