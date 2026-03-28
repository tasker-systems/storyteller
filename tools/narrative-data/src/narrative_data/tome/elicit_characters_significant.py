# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit significant characters (Q3 tension-bearing + Q4 scene-driving) for a Tome world.

Reads all prior pipeline context plus bedrock archetype data, builds a structured
prompt, calls qwen3.5:35b, parses the JSON response, and writes
characters-significant.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-characters-significant --world-slug <slug>
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.tome.elicit_characters_mundane import (
    _build_social_substrate_context,
    _load_social_substrate,
)
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
# Mundane character loading
# ---------------------------------------------------------------------------


def _load_mundane_characters(world_dir: Path) -> list[dict[str, Any]]:
    """Read characters-mundane.json and return the characters list.

    Args:
        world_dir: Path to the world directory.

    Returns:
        List of character dicts.

    Raises:
        FileNotFoundError: If characters-mundane.json does not exist.
        ValueError: If the file cannot be parsed or lacks a characters array.
    """
    path = world_dir / "characters-mundane.json"
    if not path.exists():
        raise FileNotFoundError(
            f"characters-mundane.json not found at {path}. "
            "Run 'tome elicit-characters-mundane' first."
        )
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse characters-mundane.json: {exc}") from exc

    chars = data.get("characters")
    if not isinstance(chars, list):
        raise ValueError(
            f"characters-mundane.json does not contain a 'characters' array at {path}."
        )
    return chars


# ---------------------------------------------------------------------------
# Bedrock archetype loading
# ---------------------------------------------------------------------------


def _load_archetypes(data_path: Path, genre_slug: str) -> list[dict[str, Any]]:
    """Load all bedrock archetype JSON files for a genre.

    Args:
        data_path: Root of the storyteller-data checkout.
        genre_slug: Genre region slug (e.g. "folk-horror").

    Returns:
        List of archetype dicts. Empty list if directory doesn't exist.
    """
    arch_dir = data_path / "narrative-data" / "discovery" / "archetypes" / genre_slug
    if not arch_dir.exists():
        return []

    archetypes: list[dict[str, Any]] = []
    for f in sorted(arch_dir.glob("*.json")):
        try:
            archetypes.append(json.loads(f.read_text()))
        except json.JSONDecodeError:
            continue
    return archetypes


def _load_archetype_dynamics(data_path: Path, genre_slug: str) -> list[dict[str, Any]]:
    """Load all bedrock archetype-dynamics JSON files for a genre.

    Args:
        data_path: Root of the storyteller-data checkout.
        genre_slug: Genre region slug (e.g. "folk-horror").

    Returns:
        List of archetype-dynamics dicts. Empty list if directory doesn't exist.
    """
    dyn_dir = data_path / "narrative-data" / "discovery" / "archetype-dynamics" / genre_slug
    if not dyn_dir.exists():
        return []

    dynamics: list[dict[str, Any]] = []
    for f in sorted(dyn_dir.glob("*.json")):
        try:
            dynamics.append(json.loads(f.read_text()))
        except json.JSONDecodeError:
            continue
    return dynamics


# ---------------------------------------------------------------------------
# Context construction
# ---------------------------------------------------------------------------


def _build_mundane_characters_context(characters: list[dict[str, Any]]) -> str:
    """Format mundane characters as markdown for the significant character prompt.

    Args:
        characters: List of Q1-Q2 character dicts.

    Returns:
        Markdown-formatted mundane character context.
    """
    if not characters:
        return "No mundane characters generated for this world yet."

    lines: list[str] = []
    for c in characters:
        centrality = c.get("centrality", "?")
        name = c.get("name", c.get("slug", "Unknown"))
        slug = c.get("slug", "")
        role = c.get("role", "?")
        cluster = c.get("cluster_membership", "?")
        description = c.get("description", "")

        line = f"- [{centrality}] **{name}** `{slug}` — {role} ({cluster})"
        if description:
            line += f"\n  {description[:200]}"

        tension = c.get("tension")
        if tension:
            line += f"\n  Tension: {tension[:200]}"

        lines.append(line)

    return "\n".join(lines)


def _build_archetypes_context(archetypes: list[dict[str, Any]]) -> str:
    """Format bedrock archetypes as markdown for the prompt.

    Args:
        archetypes: List of archetype dicts from discovery corpus.

    Returns:
        Markdown-formatted archetypes context.
    """
    if not archetypes:
        return "No bedrock archetype data available for this genre."

    lines: list[str] = []
    for a in archetypes:
        name = a.get("canonical_name", a.get("variant_name", "Unknown"))
        tension = a.get("distinguishing_tension", "")
        necessity = a.get("structural_necessity", "")
        profile = a.get("personality_profile", {})

        lines.append(f"### {name}")
        if tension:
            lines.append(f"**Distinguishing tension:** {tension}")
        if necessity:
            lines.append(f"**Structural necessity:** {necessity}")
        if profile and isinstance(profile, dict):
            profile_parts = [f"{k}: {v}" for k, v in profile.items()]
            lines.append(f"**Personality profile:** {', '.join(profile_parts)}")
        lines.append("")

    return "\n".join(lines).rstrip()


def _build_dynamics_context(dynamics: list[dict[str, Any]]) -> str:
    """Format archetype dynamics as markdown for the prompt.

    Args:
        dynamics: List of archetype-dynamics dicts.

    Returns:
        Markdown-formatted dynamics context.
    """
    if not dynamics:
        return "No archetype dynamics data available for this genre."

    lines: list[str] = []
    for d in dynamics:
        pairing = d.get("pairing_name", "Unknown Pairing")
        a = d.get("archetype_a", "?")
        b = d.get("archetype_b", "?")
        edge = d.get("edge_properties", {})
        edge_type = edge.get("edge_type", "") if isinstance(edge, dict) else ""
        scene = d.get("characteristic_scene", {})
        scene_title = scene.get("title", "") if isinstance(scene, dict) else ""

        lines.append(f"- **{pairing}** ({a} × {b})")
        if edge_type:
            lines.append(f"  Edge: {edge_type}")
        if scene_title:
            lines.append(f"  Scene: {scene_title}")

    return "\n".join(lines)


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
    mundane_characters: list[dict[str, Any]],
    archetypes: list[dict[str, Any]],
    archetype_dynamics: list[dict[str, Any]],
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the character-significant-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts.
        orgs: List of organization dicts.
        substrate: Parsed social-substrate.json dict.
        mundane_characters: List of Q1-Q2 character dicts.
        archetypes: List of bedrock archetype dicts for the genre.
        archetype_dynamics: List of archetype-dynamics dicts for the genre.
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
    mundane_context = _build_mundane_characters_context(mundane_characters)
    archetypes_context = _build_archetypes_context(archetypes)
    dynamics_context = _build_dynamics_context(archetype_dynamics)

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
        .replace("{mundane_characters_context}", mundane_context)
        .replace("{archetypes_context}", archetypes_context)
        .replace("{archetype_dynamics_context}", dynamics_context)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_characters_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of character objects.

    Args:
        response: Raw LLM response text.

    Returns:
        List of character dicts.

    Raises:
        ValueError: If parsing fails.
    """
    return _parse_places_response(response)


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_characters_significant(data_path: Path, world_slug: str) -> None:
    """Elicit significant characters (Q3 + Q4) for a Tome world.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "character-significant-elicitation.md"

    # 1. Load all prerequisite data
    console.print(f"[bold]Loading world data for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)
        mundane_characters = _load_mundane_characters(world_dir)
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
        f"clusters=[cyan]{len(substrate.get('clusters', []))}[/cyan]  "
        f"mundane=[cyan]{len(mundane_characters)}[/cyan]"
    )

    # 2. Load bedrock archetype data
    console.print(f"[bold]Loading bedrock archetypes for[/bold] [cyan]{genre_slug}[/cyan]")
    archetypes = _load_archetypes(data_path, genre_slug)
    archetype_dynamics = _load_archetype_dynamics(data_path, genre_slug)
    console.print(
        f"  [green]{len(archetypes)}[/green] archetype(s), "
        f"[green]{len(archetype_dynamics)}[/green] dynamic(s)"
    )

    # 3. Load prompt template
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # 4. Build prompt
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    prompt = _build_prompt(
        template, world_pos, genre_profile, places, orgs,
        substrate, mundane_characters, archetypes, archetype_dynamics,
        settings_context,
    )
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # 5. Call LLM
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

    # 6. Parse response
    console.print("[bold]Parsing response…[/bold]")
    try:
        characters = _parse_characters_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    q3 = [c for c in characters if c.get("centrality") == "Q3"]
    q4 = [c for c in characters if c.get("centrality") == "Q4"]
    console.print(
        f"  Parsed [green]{len(q3)}[/green] Q3 + [green]{len(q4)}[/green] Q4 character(s)"
    )

    # 7. Write characters-significant.json
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "q3_count": len(q3),
        "q4_count": len(q4),
        "total_count": len(characters),
        "archetypes_available": len(archetypes),
        "dynamics_available": len(archetype_dynamics),
        "characters": characters,
    }

    output_path = world_dir / "characters-significant.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # 8. Summary
    console.print()
    console.print(f"[bold]Significant characters generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for char in characters:
        slug = char.get("slug", "?")
        name = char.get("name", "?")
        centrality = char.get("centrality", "?")
        role = char.get("role", "?")
        archetype = char.get("archetype", {})
        primary = archetype.get("primary", "?") if isinstance(archetype, dict) else "?"
        console.print(
            f"  [green]✓[/green] [{centrality}] [bold]{name}[/bold] "
            f"[dim]({slug}, {role}, {primary})[/dim]"
        )
