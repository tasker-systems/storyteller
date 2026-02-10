"""CLI entry point for evaluate-classifier.

Evaluates deployed ONNX models against labeled JSONL data, printing
per-task metrics. Runs manually (not in CI) to measure quality when
models or training data change.

Usage:
    evaluate-classifier $STORYTELLER_DATA_PATH/models/event_classifier/ \\
        $STORYTELLER_DATA_PATH/training-data/event_classification.jsonl

    # Evaluate on all data (no split — for a separate test set)
    evaluate-classifier --no-split model_dir/ test_data.jsonl

    # Save JSON report
    evaluate-classifier -o report.json model_dir/ data.jsonl
"""

import argparse
import json
from pathlib import Path

from event_classifier.evaluation import evaluate, format_report, result_to_json


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="evaluate-classifier",
        description="Evaluate event classification and NER ONNX models against labeled data.",
    )
    parser.add_argument(
        "model_dir",
        type=Path,
        help="Directory containing ONNX models and tokenizer.json",
    )
    parser.add_argument(
        "data_path",
        type=Path,
        help="Path to labeled JSONL data",
    )
    parser.add_argument(
        "--no-split",
        action="store_true",
        help="Evaluate on all data (don't hold out a val split). "
        "Use when data_path is already a separate test set.",
    )
    parser.add_argument(
        "--val-fraction",
        type=float,
        default=0.15,
        help="Fraction to hold out for evaluation (default: 0.15, ignored with --no-split)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed for data splitting (default: 42)",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.5,
        help="Sigmoid threshold for event classification (default: 0.5)",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Write JSON report to file",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)

    if not args.model_dir.is_dir():
        raise SystemExit(f"Model directory not found: {args.model_dir}")
    if not args.data_path.is_file():
        raise SystemExit(f"Data file not found: {args.data_path}")

    result = evaluate(
        model_dir=args.model_dir,
        data_path=args.data_path,
        val_fraction=args.val_fraction,
        seed=args.seed,
        threshold=args.threshold,
        use_split=not args.no_split,
    )

    print(format_report(result))

    if args.output:
        report = result_to_json(result)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, "w") as f:
            json.dump(report, f, indent=2)
        print(f"\nJSON report written to: {args.output}")


if __name__ == "__main__":
    main()
