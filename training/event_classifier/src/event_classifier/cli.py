"""CLI entry point for train-event-classifier."""

import argparse
from pathlib import Path

from event_classifier.dataset import load_jsonl, split_data
from event_classifier.export import export_onnx, validate_onnx
from event_classifier.schema import PRETRAINED_MODEL
from event_classifier.train import TrainConfig, train


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="train-event-classifier",
        description="Train event classification or NER models and export to ONNX.",
    )
    parser.add_argument(
        "data_path",
        type=Path,
        help="Path to training data JSONL file",
    )
    parser.add_argument(
        "--task",
        choices=["event", "ner"],
        default="event",
        help="Task: event classification or NER (default: event)",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("output"),
        help="Output directory (default: output/)",
    )
    parser.add_argument(
        "--model",
        type=str,
        default=PRETRAINED_MODEL,
        help=f"Pretrained model name (default: {PRETRAINED_MODEL})",
    )
    parser.add_argument("--epochs", type=int, default=5)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--warmup-ratio", type=float, default=0.1)
    parser.add_argument("--weight-decay", type=float, default=0.01)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--val-fraction", type=float, default=0.15)
    parser.add_argument("--fp16", action="store_true", help="Use mixed precision")
    parser.add_argument(
        "--cpu",
        action="store_true",
        help="Force CPU training (recommended for DeBERTa on Apple Silicon)",
    )
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="Only validate data, don't train",
    )
    parser.add_argument(
        "--export-only",
        type=Path,
        metavar="MODEL_DIR",
        help="Skip training, export ONNX from existing model directory",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)

    if args.validate_only:
        print(f"Validating data: {args.data_path}")
        examples = load_jsonl(args.data_path)
        print(f"  Total examples: {len(examples)}")

        # Count event kinds
        kind_counts: dict[str, int] = {}
        for ex in examples:
            for kind in ex["event_kinds"]:
                kind_counts[kind] = kind_counts.get(kind, 0) + 1
        print("  Event kind distribution:")
        for kind, count in sorted(kind_counts.items()):
            print(f"    {kind}: {count}")

        # Count entities
        total_entities = sum(len(ex["entities"]) for ex in examples)
        print(f"  Total entities: {total_entities}")

        # Try split
        train_ex, val_ex = split_data(examples, val_fraction=args.val_fraction, seed=args.seed)
        print(f"  Train/val split: {len(train_ex)}/{len(val_ex)}")
        print("Validation passed!")
        return

    if args.export_only:
        print(f"Exporting model from: {args.export_only}")
        onnx_path = export_onnx(args.export_only, args.output, args.task)
        validate_onnx(onnx_path, args.export_only, args.task)
        print(f"\nONNX exported to: {onnx_path}")
        return

    # Full training pipeline
    config = TrainConfig(
        task=args.task,
        model_name=args.model,
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr=args.lr,
        warmup_ratio=args.warmup_ratio,
        weight_decay=args.weight_decay,
        seed=args.seed,
        val_fraction=args.val_fraction,
        output_dir=str(args.output),
        fp16=args.fp16,
        use_cpu=args.cpu,
    )

    model_dir = train(config, args.data_path)

    # Export to ONNX
    print("\nExporting to ONNX...")
    onnx_path = export_onnx(model_dir, args.output, args.task)
    validate_onnx(onnx_path, model_dir, args.task)
    print(f"\nDone! ONNX model: {onnx_path}")


if __name__ == "__main__":
    main()
