# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Compose a fully-propagated world position for a genre + setting combination.

Validates seeds against known axes, propagates inferred positions via the Tome
mutual-production graph, and writes the result to the tome/worlds output directory.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from rich.console import Console

from narrative_data.tome.propagation import propagate
from narrative_data.tome.world_position import WorldPosition, load_all_axes


def compose_world(
    data_path: Path,
    genre_slug: str,
    setting_slug: str,
    seeds: dict[str, str],
    world_slug: str,
    enriched: bool = False,
) -> WorldPosition:
    """Compose a fully-propagated WorldPosition for a genre + setting combination.

    Args:
        data_path: Root of the storyteller-data checkout (value of
            ``STORYTELLER_DATA_PATH``).
        genre_slug: Identifier for the genre region (e.g. ``"folk-horror"``).
        setting_slug: Identifier for the setting pattern (stored as metadata;
            does not need to match a bedrock slug).
        seeds: Mapping of axis slug → value provided by the caller.
        world_slug: Identifier used as the output directory name under
            ``{data_path}/narrative-data/tome/worlds/``.
        enriched: When ``True``, use LLM-enriched value selection during
            graph propagation (calls qwen2.5:7b-instruct via Ollama).

    Returns:
        The fully-propagated :class:`~narrative_data.tome.world_position.WorldPosition`.
    """
    console = Console()

    # ------------------------------------------------------------------
    # 1. Load all axes and validate seeds
    # ------------------------------------------------------------------
    console.print(f"[bold]Loading axes from[/bold] {data_path / 'narrative-data' / 'tome' / 'domains'}")
    all_axes = load_all_axes(data_path)
    known_slugs = set(all_axes.keys())

    invalid_seeds = [slug for slug in seeds if slug not in known_slugs]
    if invalid_seeds:
        raise ValueError(
            f"Unknown axis slug(s) in seeds: {', '.join(sorted(invalid_seeds))}. "
            f"Known axes: {len(known_slugs)} total."
        )

    console.print(
        f"[green]Validated[/green] {len(seeds)} seed(s) against {len(known_slugs)} known axes."
    )

    # ------------------------------------------------------------------
    # 2. Build WorldPosition and set seeds
    # ------------------------------------------------------------------
    wp = WorldPosition(genre_slug=genre_slug, setting_slug=setting_slug)
    for axis_slug, value in seeds.items():
        wp.set_seed(axis_slug, value)

    console.print(
        f"[bold]World:[/bold] {world_slug}  "
        f"[dim]genre={genre_slug}  setting={setting_slug}  seeds={len(seeds)}[/dim]"
    )

    # ------------------------------------------------------------------
    # 3. Load genre region profile (region.json)
    # ------------------------------------------------------------------
    region_path = (
        data_path / "narrative-data" / "genres" / genre_slug / "region.json"
    )
    genre_profile: dict[str, Any] | None = None
    if region_path.exists():
        try:
            genre_profile = json.loads(region_path.read_text())
            console.print(f"[green]Loaded[/green] genre profile from {region_path}")
        except Exception as exc:  # noqa: BLE001
            console.print(
                f"[yellow]Warning:[/yellow] Could not parse genre profile at {region_path}: {exc}"
            )
    else:
        console.print(
            f"[yellow]Warning:[/yellow] genre region.json not found at {region_path} — "
            "storing genre_profile as null"
        )

    # ------------------------------------------------------------------
    # 4. Propagate
    # ------------------------------------------------------------------
    if enriched:
        from narrative_data.ollama import OllamaClient

        ollama_client: OllamaClient | None = OllamaClient()
        console.print(
            "[bold]Running propagation[/bold] [yellow](LLM-enriched value selection active)[/yellow]…"
        )
    else:
        ollama_client = None
        console.print("[bold]Running propagation…[/bold]")

    propagate(wp, data_path, enriched=enriched, client=ollama_client)
    console.print(
        f"[green]Propagation complete.[/green] "
        f"seeds={wp.seed_count}  inferred={wp.inferred_count}  "
        f"total={wp.seed_count + wp.inferred_count}"
    )

    # ------------------------------------------------------------------
    # 5. Write output
    # ------------------------------------------------------------------
    output_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "world-position.json"

    output_data: dict[str, Any] = wp.to_dict()
    output_data["genre_profile"] = genre_profile

    output_path.write_text(json.dumps(output_data, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    return wp
