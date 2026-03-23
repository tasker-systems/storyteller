# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""CLI for goal lexicon enrichment."""

import argparse
from pathlib import Path

from .enrich import enrich_all_goals


def main():
    parser = argparse.ArgumentParser(description="Enrich goal vocabulary with behavioral lexicons")
    parser.add_argument(
        "descriptor_dir",
        type=Path,
        help="Path to training-data/descriptors directory",
    )
    parser.add_argument(
        "--model",
        default="qwen2.5:32b-instruct",
        help="Ollama model for enrichment (default: qwen2.5:32b-instruct)",
    )
    parser.add_argument(
        "--base-url",
        default="http://localhost:11434",
        help="Ollama base URL",
    )
    args = parser.parse_args()

    enrich_all_goals(args.descriptor_dir, args.model, args.base_url)


if __name__ == "__main__":
    main()
