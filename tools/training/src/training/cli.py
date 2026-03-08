"""CLI entry point for train-character-model."""

import argparse
from pathlib import Path

import torch

from training.dataset import load_jsonl
from training.export import export_onnx, validate_onnx
from training.feature_schema import TOTAL_INPUT_FEATURES, TOTAL_OUTPUT_FEATURES
from training.model import CharacterPredictor
from training.train import TrainConfig, train


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="train-character-model",
        description="Train the character prediction model and export to ONNX.",
    )
    parser.add_argument(
        "data_path",
        type=Path,
        help="Path to training data JSONL file",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("character_predictor.onnx"),
        help="Output ONNX model path (default: character_predictor.onnx)",
    )
    parser.add_argument("--epochs", type=int, default=100)
    parser.add_argument("--batch-size", type=int, default=256)
    parser.add_argument("--lr", type=float, default=1e-3)
    parser.add_argument("--patience", type=int, default=10)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--dropout", type=float, default=0.3)
    parser.add_argument("--action-weight", type=float, default=0.35)
    parser.add_argument("--speech-weight", type=float, default=0.20)
    parser.add_argument("--thought-weight", type=float, default=0.20)
    parser.add_argument("--emotion-weight", type=float, default=0.25)
    parser.add_argument(
        "--export-only",
        type=Path,
        metavar="CHECKPOINT",
        help="Skip training, export ONNX from existing checkpoint",
    )
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="Only validate data dimensions, don't train",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)

    if args.validate_only:
        print(f"Validating data: {args.data_path}")
        features, labels, cells = load_jsonl(args.data_path)
        unique_cells = len(set(cells))
        print(f"  Examples: {features.shape[0]}")
        print(f"  Features: {features.shape[1]} (expected {TOTAL_INPUT_FEATURES})")
        print(f"  Labels: {labels.shape[1]} (expected {TOTAL_OUTPUT_FEATURES})")
        print(f"  Unique cells: {unique_cells}")
        print("Validation passed!")
        return

    if args.export_only:
        print(f"Loading checkpoint: {args.export_only}")
        checkpoint = torch.load(args.export_only, weights_only=True)
        model = CharacterPredictor(dropout=checkpoint.get("config", {}).get("dropout", 0.3))
        model.load_state_dict(checkpoint["model_state_dict"])
        print(f"  Loaded model from epoch {checkpoint.get('epoch', '?')}")
        print(f"  Val loss: {checkpoint.get('val_loss', '?')}")

        export_onnx(model, args.output)
        validate_onnx(args.output, model)
        return

    # Full training pipeline
    config = TrainConfig(
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr=args.lr,
        patience=args.patience,
        seed=args.seed,
        dropout=args.dropout,
        action_weight=args.action_weight,
        speech_weight=args.speech_weight,
        thought_weight=args.thought_weight,
        emotion_weight=args.emotion_weight,
    )

    checkpoint_path = train(config, args.data_path)

    # Export to ONNX
    print(f"\nExporting to ONNX: {args.output}")
    checkpoint = torch.load(checkpoint_path, weights_only=True)
    model = CharacterPredictor(dropout=config.dropout)
    model.load_state_dict(checkpoint["model_state_dict"])

    export_onnx(model, args.output)
    validate_onnx(args.output, model)
    print("\nDone!")


if __name__ == "__main__":
    main()
