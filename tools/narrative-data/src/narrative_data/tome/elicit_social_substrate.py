# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit social substrate (lineages, factions, kinship groups) for a Tome world.

Reads the world-position.json, places.json, and organizations.json produced by
prior pipeline steps, builds a structured prompt from axis positions, genre
profile, and entity context, calls qwen3.5:35b, parses the JSON response, and
writes social-substrate.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-social-substrate --world-slug <slug>
"""

from __future__ import annotations

import json
import re
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
)
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_SUBSTRATE_TIMEOUT = 600.0
_SUBSTRATE_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# Organizations loading
# ---------------------------------------------------------------------------


def _load_orgs(world_dir: Path) -> list[dict[str, Any]]:
    """Read organizations.json from the world directory and return the orgs list.

    Args:
        world_dir: Path to the world directory containing organizations.json.

    Returns:
        List of organization dicts.

    Raises:
        FileNotFoundError: If organizations.json does not exist.
        ValueError: If the file cannot be parsed or lacks an organizations array.
    """
    orgs_path = world_dir / "organizations.json"
    if not orgs_path.exists():
        raise FileNotFoundError(
            f"organizations.json not found at {orgs_path}. Run 'tome elicit-orgs' first."
        )
    try:
        data = json.loads(orgs_path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse organizations.json: {exc}") from exc

    orgs = data.get("organizations")
    if not isinstance(orgs, list):
        raise ValueError(
            f"organizations.json does not contain an 'organizations' array at {orgs_path}."
        )
    return orgs


# ---------------------------------------------------------------------------
# Context construction
# ---------------------------------------------------------------------------


def _build_orgs_context(orgs: list[dict[str, Any]]) -> str:
    """Summarize organizations as a markdown list for the prompt.

    Args:
        orgs: List of organization dicts from organizations.json.

    Returns:
        Markdown-formatted organizations context string.
    """
    if not orgs:
        return "No organizations generated for this world yet."

    lines: list[str] = []
    for org in orgs:
        name = org.get("name", org.get("slug", "Unknown"))
        slug = org.get("slug", "")
        org_type = org.get("org_type", "unknown")
        description = org.get("description", "")
        if len(description) > 200:
            description = description[:197] + "..."
        line = f"- **{name}** `{slug}` ({org_type}): {description}"

        # Include stated/operative gap if present
        gap = org.get("stated_vs_operative")
        if gap and isinstance(gap, dict):
            stated = gap.get("stated", "")
            operative = gap.get("operative", "")
            if stated and operative:
                line += f"\n  - Stated: {stated[:150]}"
                line += f"\n  - Operative: {operative[:150]}"

        lines.append(line)

    return "\n".join(lines)


def _extract_axis_value(positions: list[dict[str, Any]], axis_slug: str) -> str:
    """Extract the value of a specific axis from world positions.

    Args:
        positions: List of position dicts from world-position.json.
        axis_slug: The axis slug to look up.

    Returns:
        The axis value string, or "unset" if not found.
    """
    for p in positions:
        if p.get("axis_slug") == axis_slug:
            return str(p.get("value", "unset"))
    return "unset"


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    places: list[dict[str, Any]],
    orgs: list[dict[str, Any]],
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the social-substrate-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts from places.json.
        orgs: List of organization dicts from organizations.json.
        settings_context: Formatted genre settings archetypes.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    positions = world_pos.get("positions", [])
    world_preamble = _build_world_preamble(world_pos)
    genre_summary = _build_genre_profile_summary(genre_profile)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)

    kinship_value = _extract_axis_value(positions, "kinship-system")
    stratification_value = _extract_axis_value(positions, "social-stratification")

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
        .replace("{kinship_system_value}", kinship_value)
        .replace("{stratification_value}", stratification_value)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_substrate_response(response: str) -> dict[str, Any]:
    """Parse LLM response as a JSON object with clusters and relationships.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown ```json ... ``` code fence.
    3. Find the outermost { ... } object boundaries and parse that.

    Args:
        response: Raw LLM response text.

    Returns:
        Dict with 'clusters' and 'relationships' keys.

    Raises:
        ValueError: If all three strategies fail.
    """
    text = response.strip()

    def _try_parse(s: str) -> dict[str, Any] | None:
        try:
            result = json.loads(s)
            if isinstance(result, dict) and "clusters" in result:
                return result
        except json.JSONDecodeError:
            pass
        return None

    # Strategy 1: direct parse
    parsed = _try_parse(text)
    if parsed:
        return parsed

    # Strategy 2: extract from ```json ... ``` fence
    fence_match = re.search(r"```json\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        parsed = _try_parse(fence_match.group(1))
        if parsed:
            return parsed

    # Also try plain ``` fence
    plain_fence_match = re.search(r"```\s*(\{.*?\})\s*```", text, re.DOTALL)
    if plain_fence_match:
        parsed = _try_parse(plain_fence_match.group(1))
        if parsed:
            return parsed

    # Strategy 3: find outermost { ... } object
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        parsed = _try_parse(text[start : end + 1])
        if parsed:
            return parsed

    raise ValueError(
        "Could not parse LLM response as a JSON object with 'clusters'. "
        f"Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_social_substrate(data_path: Path, world_slug: str) -> None:
    """Elicit social substrate (lineages, factions, kinship groups) for a Tome world.

    Reads world-position.json, places.json, and organizations.json, builds a
    structured prompt, calls the elicitation model, parses the JSON response,
    and writes social-substrate.json to the world directory.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier — must match a directory under
            {data_path}/narrative-data/tome/worlds/.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "social-substrate-elicitation.md"

    # 1. Load world position
    console.print(f"[bold]Loading world position for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"positions=[cyan]{world_pos.get('total_positions', 0)}[/cyan]"
    )

    # 2. Load places and organizations
    console.print("[bold]Loading places and organizations…[/bold]")
    try:
        places = _load_places(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    try:
        orgs = _load_orgs(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    console.print(
        f"  Loaded [green]{len(places)}[/green] place(s), [green]{len(orgs)}[/green] org(s)"
    )

    # 3. Load prompt template
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # 4. Build prompt
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    prompt = _build_prompt(template, world_pos, genre_profile, places, orgs, settings_context)
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # 5. Call LLM
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_SUBSTRATE_TIMEOUT}s, temperature={_SUBSTRATE_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_SUBSTRATE_TIMEOUT,
        temperature=_SUBSTRATE_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # 6. Parse response
    console.print("[bold]Parsing response…[/bold]")
    try:
        substrate = _parse_substrate_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    clusters = substrate.get("clusters", [])
    relationships = substrate.get("relationships", [])
    console.print(
        f"  Parsed [green]{len(clusters)}[/green] cluster(s), "
        f"[green]{len(relationships)}[/green] relationship(s)"
    )

    # 7. Write social-substrate.json
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "cluster_count": len(clusters),
        "relationship_count": len(relationships),
        "clusters": clusters,
        "relationships": relationships,
    }

    output_path = world_dir / "social-substrate.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # 8. Summary
    console.print()
    console.print(f"[bold]Social substrate generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for cluster in clusters:
        slug = cluster.get("slug", "?")
        name = cluster.get("name", "?")
        basis = cluster.get("basis", "?")
        position = cluster.get("hierarchy_position", "?")
        console.print(
            f"  [green]✓[/green] [bold]{name}[/bold] [dim]({slug}, {basis}, {position})[/dim]"
        )
    if relationships:
        console.print()
        console.print("[bold]Relationships:[/bold]")
        for rel in relationships:
            a = rel.get("cluster_a", "?")
            b = rel.get("cluster_b", "?")
            rtype = rel.get("type", "?")
            console.print(f"  [dim]{a} ↔ {b}: {rtype}[/dim]")
