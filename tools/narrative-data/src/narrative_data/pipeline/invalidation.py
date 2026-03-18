"""Manifest I/O, content digest, prompt hash, and staleness detection."""

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
