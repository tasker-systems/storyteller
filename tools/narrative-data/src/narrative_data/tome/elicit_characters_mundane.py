# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit mundane characters (Q1 background + Q2 community) for a Tome world.

Reads world-position.json, places.json, organizations.json, and social-substrate.json,
builds a structured prompt, calls qwen3.5:35b, parses the JSON response, and writes
characters-mundane.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-characters-mundane --world-slug <slug>
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
from narrative_data.tome.elicit_places import (
    _build_genre_profile_summary,
    _build_settings_context,
    _build_world_preamble,
    _load_world_position,
    _parse_places_response,
)
from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_CHAR_TIMEOUT = 600.0
_CHAR_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# Social substrate loading
# ---------------------------------------------------------------------------


def _load_social_substrate(world_dir: Path) -> dict[str, Any]:
    """Read social-substrate.json from the world directory.

    Args:
        world_dir: Path to the world directory containing social-substrate.json.

    Returns:
        Parsed social substrate dict with 'clusters' and 'relationships'.

    Raises:
        FileNotFoundError: If social-substrate.json does not exist.
        ValueError: If the file cannot be parsed or lacks a clusters array.
    """
    path = world_dir / "social-substrate.json"
    if not path.exists():
        raise FileNotFoundError(
            f"social-substrate.json not found at {path}. Run 'tome elicit-social-substrate' first."
        )
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse social-substrate.json: {exc}") from exc

    if not isinstance(data.get("clusters"), list):
        raise ValueError(f"social-substrate.json does not contain a 'clusters' array at {path}.")
    return data


# ---------------------------------------------------------------------------
# Context construction
# ---------------------------------------------------------------------------


def _build_social_substrate_context(substrate: dict[str, Any]) -> str:
    """Format the social substrate as markdown for the character prompt.

    Args:
        substrate: Parsed social-substrate.json dict.

    Returns:
        Markdown-formatted social substrate context.
    """
    clusters = substrate.get("clusters", [])
    relationships = substrate.get("relationships", [])

    if not clusters:
        return "No social substrate generated for this world yet."

    lines: list[str] = []
    lines.append("### Clusters")
    lines.append("")
    for c in clusters:
        name = c.get("name", c.get("slug", "Unknown"))
        slug = c.get("slug", "")
        basis = c.get("basis", "?")
        position = c.get("hierarchy_position", "?")
        description = c.get("description", "")
        history = c.get("history", "")
        lines.append(f"- **{name}** `{slug}` (basis: {basis}, position: {position})")
        if description:
            lines.append(f"  {description[:250]}")
        if history:
            lines.append(f"  History: {history[:200]}")

        org_rels = c.get("org_relationships", [])
        if org_rels:
            lines.append(f"  Org ties: {', '.join(str(r) for r in org_rels)}")
        lines.append("")

    if relationships:
        lines.append("### Inter-Cluster Relationships")
        lines.append("")
        for r in relationships:
            a = r.get("cluster_a", "?")
            b = r.get("cluster_b", "?")
            rtype = r.get("type", "?")
            tension = r.get("boundary_tension", "")
            lines.append(f"- **{a} ↔ {b}** ({rtype})")
            if tension:
                lines.append(f"  Boundary tension: {tension}")
            lines.append("")

    return "\n".join(lines).rstrip()


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    places: list[dict[str, Any]],
    orgs: list[dict[str, Any]],
    substrate: dict[str, Any],
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the character-mundane-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts from places.json.
        orgs: List of organization dicts from organizations.json.
        substrate: Parsed social-substrate.json dict.
        settings_context: Formatted genre settings archetypes.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    world_preamble = _build_world_preamble(world_pos)
    genre_summary = _build_genre_profile_summary(genre_profile)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)
    substrate_context = _build_social_substrate_context(substrate)

    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", world_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{social_substrate_context}", substrate_context)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_characters_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of character objects.

    Uses the same three-strategy approach as place/org parsing.

    Args:
        response: Raw LLM response text.

    Returns:
        List of character dicts.

    Raises:
        ValueError: If all strategies fail.
    """
    # Reuse the proven array-parsing logic from elicit_places
    return _parse_places_response(response)


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_characters_mundane(data_path: Path, world_slug: str) -> None:
    """Elicit mundane characters (Q1 background + Q2 community) for a Tome world.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "character-mundane-elicitation.md"

    # 1. Load all prerequisite data
    console.print(f"[bold]Loading world data for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"places=[cyan]{len(places)}[/cyan]  "
        f"orgs=[cyan]{len(orgs)}[/cyan]  "
        f"clusters=[cyan]{len(substrate.get('clusters', []))}[/cyan]"
    )

    # 2. Load prompt template
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # 3. Build prompt
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    prompt = _build_prompt(
        template, world_pos, genre_profile, places, orgs, substrate, settings_context
    )
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # 4. Call LLM
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_CHAR_TIMEOUT}s, temperature={_CHAR_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_CHAR_TIMEOUT,
        temperature=_CHAR_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # 5. Parse response
    console.print("[bold]Parsing response…[/bold]")
    try:
        characters = _parse_characters_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    q1 = [c for c in characters if c.get("centrality") == "Q1"]
    q2 = [c for c in characters if c.get("centrality") == "Q2"]
    console.print(
        f"  Parsed [green]{len(q1)}[/green] Q1 + [green]{len(q2)}[/green] Q2 character(s)"
    )

    # 6. Write characters-mundane.json
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "q1_count": len(q1),
        "q2_count": len(q2),
        "total_count": len(characters),
        "characters": characters,
    }

    output_path = world_dir / "characters-mundane.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # 7. Summary
    console.print()
    console.print(f"[bold]Mundane characters generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for char in characters:
        slug = char.get("slug", "?")
        name = char.get("name", "?")
        centrality = char.get("centrality", "?")
        role = char.get("role", "?")
        cluster = char.get("cluster_membership", "?")
        console.print(
            f"  [green]✓[/green] [{centrality}] [bold]{name}[/bold] "
            f"[dim]({slug}, {role}, {cluster})[/dim]"
        )
