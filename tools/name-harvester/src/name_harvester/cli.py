"""CLI for name harvesting."""

import json
import os
import sys
from pathlib import Path

from .harvester import GENRE_MAPPINGS, harvest_names


def main() -> None:
    """Harvest names for all mapped genres and write to names.json."""
    output_path = None
    if len(sys.argv) > 1:
        output_path = Path(sys.argv[1])
    else:
        data_path = os.environ.get("STORYTELLER_DATA_PATH")
        if data_path:
            output_path = Path(data_path) / "training-data" / "descriptors" / "names.json"

    if output_path is None:
        print("Usage: harvest-names <output-path>", file=sys.stderr)
        print("Or set STORYTELLER_DATA_PATH environment variable", file=sys.stderr)
        sys.exit(1)

    # Load existing file if present (idempotent merge)
    existing: dict = {}
    if output_path.exists():
        with open(output_path) as f:
            existing = json.load(f)

    for mapping in GENRE_MAPPINGS:
        print(f"Harvesting names for {mapping.genre_id} from {mapping.url}...")
        try:
            names = harvest_names(mapping.url)
            # Merge: union of existing + new, deduped
            existing_names = set()
            if mapping.genre_id in existing:
                existing_names = set(existing[mapping.genre_id].get("names", []))
            merged = sorted(set(names) | existing_names)
            existing[mapping.genre_id] = {
                "names": merged,
                "source": mapping.source_description,
            }
            print(f"  Got {len(names)} names, {len(merged)} after merge")
        except Exception as e:
            print(f"  Error: {e}", file=sys.stderr)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(existing, f, indent=2, ensure_ascii=False)
    print(f"Written to {output_path}")
