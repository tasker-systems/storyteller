# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Spatial command orchestration: elicitation and structuring for B.2."""

from pathlib import Path
from typing import Any

from rich.console import Console

from narrative_data.config import (
    ELICITATION_MODEL,
    SPATIAL_CATEGORIES,
    STRUCTURING_MODEL,
)
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.pipeline.invalidation import (
    compute_prompt_hash,
    is_stale,
    load_manifest,
    update_manifest_entry,
)
from narrative_data.prompts import PromptBuilder
from narrative_data.utils import now_iso, slug_to_name

console = Console()

SETTING_SLUGS: list[str] = [
    "family-home",
    "inn-tavern",
    "boarding-school",
    "city-streets",
    "market-bazaar",
    "government-building",
    "underground-subway",
    "gothic-mansion",
    "cathedral-temple",
    "castle-fortress",
    "university-library",
    "pastoral-village",
    "farmstead",
    "coastal-settlement",
    "dense-forest",
    "mountain-pass",
    "desert-wasteland",
    "river-lake-shore",
    "space-station",
    "sailing-vessel",
    "train-carriage",
    "ruins-archaeological-site",
]

# Dependency order is encoded here: setting-type → place-entities → topology → tonal-inheritance
_SPATIAL_ORDER = ["setting-type", "place-entities", "topology", "tonal-inheritance"]


def _order_spatial_categories(categories: list[str]) -> list[str]:
    """Return categories in dependency order (setting-type first, tonal-inheritance last)."""
    ordered = []
    for cat in _SPATIAL_ORDER:
        if cat in categories:
            ordered.append(cat)
    # Append any unknown categories at the end
    for cat in categories:
        if cat not in ordered:
            ordered.append(cat)
    return ordered


def _build_spatial_context(setting_dir: Path, category: str) -> dict[str, str]:
    """Inject prior-stage JSON as context for a given category.

    Dependency chain:
      setting-type      → no prior context
      place-entities    → setting-type.json
      topology          → setting-type.json + place-entities.json
      tonal-inheritance → setting-type.json + place-entities.json + topology.json
    """
    context: dict[str, str] = {}

    if category == "setting-type":
        return context

    setting_type_path = setting_dir / "setting-type.json"
    if setting_type_path.exists():
        context["setting-type"] = setting_type_path.read_text()

    if category in ("topology", "tonal-inheritance"):
        place_entities_path = setting_dir / "place-entities.json"
        if place_entities_path.exists():
            context["place-entities"] = place_entities_path.read_text()

    if category == "tonal-inheritance":
        topology_path = setting_dir / "topology.json"
        if topology_path.exists():
            context["topology"] = topology_path.read_text()

    return context


def elicit_spatial(
    client: OllamaClient,
    output_base: Path,
    manifest_path: Path,
    settings: list[str] | None = None,
    categories: list[str] | None = None,
    model: str = ELICITATION_MODEL,
    force: bool = False,
) -> None:
    """Stage 1: Elicit raw markdown for each setting × category.

    Skips up-to-date cells unless force=True.
    Categories are processed in dependency order.
    """
    if settings is None:
        settings = SETTING_SLUGS
    if categories is None:
        categories = SPATIAL_CATEGORIES

    ordered_categories = _order_spatial_categories(categories)
    manifest = load_manifest(manifest_path)
    builder = PromptBuilder()

    for setting_slug in settings:
        setting_dir = output_base / "spatial" / setting_slug
        setting_name = slug_to_name(setting_slug)

        for category in ordered_categories:
            manifest_key = f"{setting_slug}/{category}"
            entry: dict[str, Any] | None = manifest["entries"].get(manifest_key)

            context = _build_spatial_context(setting_dir, category)

            try:
                prompt = builder.build_stage1(
                    domain="spatial",
                    category=category,
                    target_name=setting_name,
                    context=context if context else None,
                )
                current_hash = compute_prompt_hash(prompt)
            except FileNotFoundError:
                console.print(
                    f"[dim]  Skipping {setting_slug}/{category} — prompt template missing[/dim]"
                )
                continue

            if not force and not is_stale(entry, current_hash):
                console.print(f"[dim]  {setting_slug}/{category} up to date, skipping[/dim]")
                continue

            console.print(f"[cyan]  Eliciting {setting_slug}/{category}…[/cyan]")
            result = run_elicitation(
                client=client,
                builder=builder,
                domain="spatial",
                category=category,
                target_name=setting_name,
                target_slug=setting_slug,
                output_dir=setting_dir,
                model=model,
                context=context if context else None,
            )

            update_manifest_entry(
                manifest_path,
                manifest_key,
                {
                    "prompt_hash": result["prompt_hash"],
                    "content_digest": result["content_digest"],
                    "elicited_at": now_iso(),
                    "raw_path": result["raw_path"],
                },
            )
            manifest = load_manifest(manifest_path)  # refresh


def structure_spatial(
    client: OllamaClient,
    output_base: Path,
    manifest_path: Path,
    settings: list[str] | None = None,
    categories: list[str] | None = None,
    model: str = STRUCTURING_MODEL,
    force: bool = False,
) -> None:
    """Stage 2 structuring — replaced by the new 'structure' CLI command (Task 10).

    This function is a no-op placeholder. The old schema types (SettingType, etc.)
    have been removed as part of the Stage 2 architecture migration.
    """
    console.print(
        "[yellow]  structure_spatial() is deprecated"
        " — use 'narrative-data structure' instead[/yellow]"
    )
