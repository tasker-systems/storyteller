# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

#!/usr/bin/env python3
"""Migrate descriptor JSON files to include UUIDv7 entity_id fields.

Two-pass approach:
  Pass 1 — build a slug→UUID mapping across all descriptor files
  Pass 2 — write entity_id into each object, leaving cross-reference arrays as slugs

Files migrated (array-of-objects with id fields):
  genres.json       key: "genres"
  archetypes.json   key: "archetypes"
  profiles.json     key: "profiles"
  dynamics.json     key: "dynamics"
  goals.json        key: "goals"

Files NOT modified (keyed by genre slug, not arrays of id objects):
  settings.json, names.json, axis-vocabulary.json, cross-dimensions.json

Writes slug_to_uuid.json alongside the descriptor files as a reference.
"""

import json
import sys
from pathlib import Path

try:
    import uuid_utils

    def make_uuid7() -> str:
        return str(uuid_utils.uuid7())
except ImportError:
    try:
        import uuid6

        def make_uuid7() -> str:
            return str(uuid6.uuid7())
    except ImportError:
        raise ImportError(
            "Neither uuid_utils nor uuid6 is available. "
            "Install with: pip install uuid-utils"
        )


# Descriptor files that contain arrays of objects with "id" slugs
ARRAY_FILES: list[tuple[str, str]] = [
    ("genres.json", "genres"),
    ("archetypes.json", "archetypes"),
    ("profiles.json", "profiles"),
    ("dynamics.json", "dynamics"),
    ("goals.json", "goals"),
]

# Cross-reference field names that must stay as slug strings (no UUID replacement)
CROSS_REF_FIELDS = {
    "valid_archetypes",
    "valid_dynamics",
    "valid_profiles",
    "pursuable_goals",
    "enabled_goals",
    "blocked_goals",
    "scene_goals",
}


def load_json(path: Path) -> dict:
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def write_json(path: Path, data: dict) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")


def build_slug_to_uuid(descriptors_dir: Path) -> dict[str, str]:
    """Pass 1: scan all array files and assign a UUIDv7 to each slug."""
    mapping: dict[str, str] = {}

    for filename, array_key in ARRAY_FILES:
        path = descriptors_dir / filename
        if not path.exists():
            print(f"  WARNING: {filename} not found, skipping", file=sys.stderr)
            continue

        data = load_json(path)
        items = data.get(array_key, [])
        for item in items:
            slug = item.get("id")
            if slug is None:
                print(
                    f"  WARNING: item in {filename} has no 'id' field: {item!r}",
                    file=sys.stderr,
                )
                continue
            if slug not in mapping:
                mapping[slug] = make_uuid7()

    return mapping


def already_migrated(item: dict) -> bool:
    """Return True if the item already has a non-empty entity_id."""
    eid = item.get("entity_id", "")
    return bool(eid)


def inject_entity_ids(
    data: dict,
    array_key: str,
    slug_to_uuid: dict[str, str],
) -> tuple[dict, int, int]:
    """Inject entity_id into each object in the array.

    Returns (modified_data, added_count, skipped_count).
    entity_id is inserted immediately after the "id" field for readability.
    Cross-reference arrays are left untouched.
    """
    items = data.get(array_key, [])
    added = 0
    skipped = 0

    new_items = []
    for item in items:
        slug = item.get("id")
        if already_migrated(item):
            skipped += 1
            new_items.append(item)
            continue

        if slug is None or slug not in slug_to_uuid:
            print(
                f"  WARNING: no UUID mapping for slug {slug!r}, skipping item",
                file=sys.stderr,
            )
            new_items.append(item)
            continue

        # Rebuild the dict with entity_id inserted after "id"
        new_item: dict = {}
        for k, v in item.items():
            new_item[k] = v
            if k == "id":
                new_item["entity_id"] = slug_to_uuid[slug]
        added += 1
        new_items.append(new_item)

    new_data = {**data, array_key: new_items}
    return new_data, added, skipped


def migrate(descriptors_dir: Path, dry_run: bool = False) -> None:
    print(f"Descriptor directory: {descriptors_dir}")

    # Pass 1: build slug→UUID map
    print("\nPass 1: building slug→UUID mapping...")
    slug_to_uuid = build_slug_to_uuid(descriptors_dir)
    print(f"  {len(slug_to_uuid)} unique slugs mapped")

    # Write reference file
    ref_path = descriptors_dir / "slug_to_uuid.json"
    if not dry_run:
        write_json(ref_path, {"slug_to_uuid": slug_to_uuid})
        print(f"  Wrote {ref_path}")
    else:
        print(f"  DRY RUN: would write {ref_path}")

    # Pass 2: inject entity_id into each file
    print("\nPass 2: injecting entity_id fields...")
    for filename, array_key in ARRAY_FILES:
        path = descriptors_dir / filename
        if not path.exists():
            print(f"  SKIP {filename} (not found)")
            continue

        data = load_json(path)
        new_data, added, skipped = inject_entity_ids(data, array_key, slug_to_uuid)

        if not dry_run:
            write_json(path, new_data)

        status = "DRY RUN" if dry_run else "wrote"
        print(
            f"  {filename}: {added} entity_ids added, {skipped} already present → {status}"
        )

    print("\nMigration complete.")


def spot_check(descriptors_dir: Path) -> None:
    """Verify genres.json has entity_id on each genre, slugs preserved in cross-refs."""
    print("\nSpot check: genres.json")
    path = descriptors_dir / "genres.json"
    data = load_json(path)
    genres = data.get("genres", [])
    for genre in genres:
        slug = genre.get("id")
        eid = genre.get("entity_id", "")
        if not eid:
            print(f"  FAIL: genre '{slug}' missing entity_id")
            continue
        # Verify cross-refs are still slugs (not UUIDs)
        for ref_field in ("valid_archetypes", "valid_dynamics", "valid_profiles"):
            refs = genre.get(ref_field, [])
            for ref in refs:
                if "-" in ref and len(ref) == 36:
                    print(
                        f"  WARN: genre '{slug}'.{ref_field} contains UUID-like value '{ref}'"
                    )
        print(f"  OK: '{slug}' → entity_id={eid[:8]}...")
    print(f"  {len(genres)} genres checked")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Add UUIDv7 entity_id fields to storyteller descriptor JSON files."
    )
    parser.add_argument(
        "descriptors_dir",
        nargs="?",
        default=None,
        help="Path to the descriptors directory. "
        "Defaults to $STORYTELLER_DATA_PATH/training-data/descriptors",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would be done without modifying files.",
    )
    parser.add_argument(
        "--spot-check",
        action="store_true",
        help="After migration, run spot check on genres.json.",
    )
    args = parser.parse_args()

    if args.descriptors_dir:
        desc_dir = Path(args.descriptors_dir)
    else:
        import os

        data_path = os.environ.get("STORYTELLER_DATA_PATH")
        if not data_path:
            print(
                "ERROR: provide descriptors_dir argument or set STORYTELLER_DATA_PATH",
                file=sys.stderr,
            )
            sys.exit(1)
        desc_dir = Path(data_path) / "training-data" / "descriptors"

    if not desc_dir.is_dir():
        print(f"ERROR: {desc_dir} is not a directory", file=sys.stderr)
        sys.exit(1)

    migrate(desc_dir, dry_run=args.dry_run)

    if args.spot_check and not args.dry_run:
        spot_check(desc_dir)
