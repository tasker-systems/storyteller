# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Manifest I/O, content digest, prompt hash, staleness detection, and run logging."""

import hashlib
import json
from pathlib import Path
from typing import Any


def compute_content_digest(content: str) -> str:
    h = hashlib.sha256(content.encode()).hexdigest()
    return f"sha256:{h}"


def compute_prompt_hash(prompt_text: str) -> str:
    h = hashlib.sha256(prompt_text.encode()).hexdigest()[:16]
    return h


def compute_intersection_hash(upstream_digests: list[str]) -> str:
    combined = "|".join(upstream_digests)
    h = hashlib.sha256(combined.encode()).hexdigest()
    return f"sha256:{h}"


def load_manifest(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"entries": {}}
    with open(path) as f:
        return json.load(f)


def save_manifest(path: Path, manifest: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(manifest, f, indent=2)


def update_manifest_entry(path: Path, key: str, entry: dict[str, Any]) -> None:
    manifest = load_manifest(path)
    manifest["entries"][key] = entry
    save_manifest(path, manifest)


def is_stale(
    entry: dict[str, Any] | None,
    current_prompt_hash: str,
    upstream_digest: str | None = None,
) -> bool:
    if entry is None:
        return True
    if entry.get("prompt_hash") != current_prompt_hash:
        return True
    return bool(upstream_digest and entry.get("content_digest") != upstream_digest)


def archive_existing(path: Path) -> Path | None:
    """Archive an existing file by renaming to .raw.v{N}.md before overwriting.

    Returns the archive path, or None if no file existed to archive.
    Finds the next available version number (v1, v2, ...).
    Archive files always use the .raw.v{N}.md convention regardless of
    whether the source file contained .raw. in its name.
    """
    if not path.exists():
        return None
    # Find next version number
    # Archive naming is always <stem>.raw.v{N}.md — version archives keep the
    # .raw. infix to distinguish them from the canonical .md artifacts.
    stem = path.stem  # e.g., "region" (after rename from region.raw.md)
    parent = path.parent
    version = 1
    while (parent / f"{stem}.raw.v{version}.md").exists():
        version += 1
    archive_path = parent / f"{stem}.raw.v{version}.md"
    path.rename(archive_path)
    return archive_path


def write_run_log(output_base: Path, run_data: dict[str, Any]) -> Path:
    """Write a generation run log to meta/runs/."""
    runs_dir = output_base / "meta" / "runs"
    runs_dir.mkdir(parents=True, exist_ok=True)
    from narrative_data.utils import now_iso

    timestamp = now_iso().replace(":", "-").replace("+", "-")
    log_path = runs_dir / f"{timestamp}.json"
    log_path.write_text(json.dumps(run_data, indent=2))
    return log_path
