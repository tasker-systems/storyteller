# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit organizations and institutions for a Tome world using the mutual production graph context.

Reads the world-position.json and places.json produced by compose-world and elicit-places,
builds a structured prompt from the axis positions, genre profile, and place summaries,
calls qwen3.5:35b, parses the JSON response, and writes organizations.json to the world
directory.

Usage (via CLI):
    uv run narrative-data tome elicit-orgs --world-slug <slug>
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.tome.elicit_places import (
    _build_genre_profile_summary,
    _build_world_preamble,
    _load_world_position,
)
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_ORG_TIMEOUT = 600.0
_ORG_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# Places loading
# ---------------------------------------------------------------------------


def _load_places(world_dir: Path) -> list[dict[str, Any]]:
    """Read places.json from the world directory and return the places list.

    Args:
        world_dir: Path to the world directory containing places.json.

    Returns:
        List of place dicts from places.json.

    Raises:
        FileNotFoundError: If places.json does not exist (run elicit-places first).
        ValueError: If the file cannot be parsed as JSON or lacks a places array.
    """
    places_path = world_dir / "places.json"
    if not places_path.exists():
        raise FileNotFoundError(
            f"places.json not found at {places_path}. "
            "Run 'tome elicit-places' first."
        )
    try:
        data = json.loads(places_path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse places.json: {exc}") from exc

    places = data.get("places")
    if not isinstance(places, list):
        raise ValueError(
            f"places.json does not contain a 'places' array at {places_path}."
        )
    return places


# ---------------------------------------------------------------------------
# Places context construction
# ---------------------------------------------------------------------------


def _build_places_context(places: list[dict[str, Any]]) -> str:
    """Summarize places as a markdown list for the org-elicitation prompt.

    Each place is summarized as:
    - **Name** (spatial_role): description (truncated to 200 chars)

    Args:
        places: List of place dicts from places.json.

    Returns:
        Markdown-formatted places context string.
    """
    if not places:
        return "No places generated for this world yet."

    lines: list[str] = []
    for place in places:
        name = place.get("name", place.get("slug", "Unknown"))
        slug = place.get("slug", "")
        spatial_role = place.get("spatial_role", "unknown")
        description = place.get("description", "")
        if len(description) > 200:
            description = description[:197] + "..."
        line = f"- **{name}** `{slug}` ({spatial_role}): {description}"
        lines.append(line)

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    places: list[dict[str, Any]],
) -> str:
    """Substitute all placeholders into the org-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts from places.json.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    world_preamble = _build_world_preamble(world_pos)
    genre_profile_summary = _build_genre_profile_summary(genre_profile)
    places_context = _build_places_context(places)

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", world_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_orgs_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of organization objects.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown ```json ... ``` code fence.
    3. Find the outermost [ ... ] array boundaries and parse that.

    Args:
        response: Raw LLM response text.

    Returns:
        List of organization dicts.

    Raises:
        ValueError: If all three strategies fail.
    """
    text = response.strip()

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if isinstance(result, list):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from ```json ... ``` fence
    fence_match = re.search(r"```json\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    # Also try plain ``` fence
    plain_fence_match = re.search(r"```\s*([\[{].*?)\s*```", text, re.DOTALL)
    if plain_fence_match:
        try:
            result = json.loads(plain_fence_match.group(1))
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 3: find outermost [ ... ] array
    start = text.find("[")
    end = text.rfind("]")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, list):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(
        "Could not parse LLM response as a JSON array of organizations. "
        f"Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_orgs(data_path: Path, world_slug: str) -> None:
    """Elicit organizations and institutions for a Tome world.

    Reads world-position.json and places.json, builds a structured prompt from
    axis positions, genre profile, and place summaries, calls the elicitation
    model, parses the JSON response, and writes organizations.json to the world
    directory.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier — must match a directory under
            {data_path}/narrative-data/tome/worlds/.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "org-elicitation.md"

    # ------------------------------------------------------------------
    # 1. Load world position
    # ------------------------------------------------------------------
    console.print(f"[bold]Loading world position for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
    except FileNotFoundError as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc
    except ValueError as exc:
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

    # ------------------------------------------------------------------
    # 2. Load places
    # ------------------------------------------------------------------
    console.print("[bold]Loading places…[/bold]")
    try:
        places = _load_places(world_dir)
    except FileNotFoundError as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc
    except ValueError as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    console.print(f"  Loaded [green]{len(places)}[/green] place(s)")

    # ------------------------------------------------------------------
    # 3. Load prompt template
    # ------------------------------------------------------------------
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # ------------------------------------------------------------------
    # 4. Build prompt
    # ------------------------------------------------------------------
    console.print("[bold]Building prompt…[/bold]")
    prompt = _build_prompt(template, world_pos, genre_profile, places)
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # ------------------------------------------------------------------
    # 5. Call LLM
    # ------------------------------------------------------------------
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_ORG_TIMEOUT}s, temperature={_ORG_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_ORG_TIMEOUT,
        temperature=_ORG_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # ------------------------------------------------------------------
    # 6. Parse response
    # ------------------------------------------------------------------
    console.print("[bold]Parsing response…[/bold]")
    try:
        organizations = _parse_orgs_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    console.print(f"  Parsed [green]{len(organizations)}[/green] organization(s)")

    # ------------------------------------------------------------------
    # 7. Write organizations.json
    # ------------------------------------------------------------------
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "org_count": len(organizations),
        "organizations": organizations,
    }

    output_path = world_dir / "organizations.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # ------------------------------------------------------------------
    # 8. Summary
    # ------------------------------------------------------------------
    console.print()
    console.print(f"[bold]Organizations generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for org in organizations:
        slug = org.get("slug", "?")
        name = org.get("name", "?")
        org_type = org.get("org_type", "?")
        authority_basis = org.get("authority_basis", "")
        console.print(
            f"  [green]✓[/green] [bold]{name}[/bold] "
            f"[dim]({slug}, {org_type})[/dim]"
        )
        if authority_basis:
            console.print(f"    [dim]{authority_basis}[/dim]")
