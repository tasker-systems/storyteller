# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit named places for a Tome world using the mutual production graph context.

Reads the world-position.json produced by compose-world, builds a structured
prompt from the axis positions and genre profile, calls qwen3.5:35b, parses the
JSON response, and writes places.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-places --world-slug <slug>
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_PLACE_TIMEOUT = 600.0
_PLACE_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# World position loading
# ---------------------------------------------------------------------------


def _load_world_position(world_dir: Path) -> dict[str, Any]:
    """Read world-position.json from the world directory.

    Args:
        world_dir: Path to the world directory containing world-position.json.

    Returns:
        Parsed world position dict.

    Raises:
        FileNotFoundError: If world-position.json does not exist.
        ValueError: If the file cannot be parsed as JSON.
    """
    wp_path = world_dir / "world-position.json"
    if not wp_path.exists():
        raise FileNotFoundError(
            f"world-position.json not found at {wp_path}. "
            "Run 'tome compose-world' first."
        )
    try:
        return json.loads(wp_path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse world-position.json: {exc}") from exc


# ---------------------------------------------------------------------------
# World preamble construction
# ---------------------------------------------------------------------------


def _build_world_preamble(world_pos: dict[str, Any]) -> str:
    """Build a markdown preamble from the world position dict.

    Groups seed positions first, then inferred positions with justification
    and confidence. Each position is formatted as a markdown list item.

    Args:
        world_pos: Parsed world-position.json dict.

    Returns:
        Markdown-formatted preamble string.
    """
    positions: list[dict[str, Any]] = world_pos.get("positions", [])
    seeds = [p for p in positions if p.get("source") == "seed"]
    inferred = [p for p in positions if p.get("source") != "seed"]

    lines: list[str] = []

    if seeds:
        lines.append("### Seed Positions (author-provided)")
        lines.append("")
        for p in seeds:
            lines.append(f"- **{p['axis_slug']}**: {p['value']}")
        lines.append("")

    if inferred:
        lines.append("### Inferred Positions (propagated from seeds)")
        lines.append("")
        for p in sorted(inferred, key=lambda x: -x.get("confidence", 0.0)):
            confidence = p.get("confidence", 0.0)
            justification = p.get("justification", "")
            line = f"- **{p['axis_slug']}**: {p['value']} (confidence: {confidence:.2f})"
            if justification:
                line += f" — {justification}"
            lines.append(line)
        lines.append("")

    if not seeds and not inferred:
        lines.append("No axis positions recorded.")

    return "\n".join(lines).rstrip()


# ---------------------------------------------------------------------------
# Genre profile summary
# ---------------------------------------------------------------------------


def _build_genre_profile_summary(genre_profile: dict[str, Any] | None) -> str:
    """Extract key genre signals from the genre_profile dict.

    Formats world_affordances, aesthetic register, agency, and other
    narratively-relevant dimensions as a readable summary.

    Args:
        genre_profile: Parsed region.json dict, or None if unavailable.

    Returns:
        Markdown-formatted genre profile summary string.
    """
    if genre_profile is None:
        return "Genre profile unavailable — no region.json found for this genre."

    lines: list[str] = []

    # World affordances
    affordances = genre_profile.get("world_affordances", {})
    if affordances:
        lines.append("**World Affordances:**")
        for key, val in affordances.items():
            lines.append(f"- {key}: {val}")
        lines.append("")

    # Aesthetic register
    aesthetic = genre_profile.get("aesthetic_register", {})
    if aesthetic:
        lines.append("**Aesthetic Register:**")
        for key, val in aesthetic.items():
            lines.append(f"- {key}: {val}")
        lines.append("")

    # Agency
    agency = genre_profile.get("agency", {})
    if agency:
        level = agency.get("level", "")
        agency_type = agency.get("type", "")
        if level or agency_type:
            parts = []
            if level:
                parts.append(f"level: {level}")
            if agency_type:
                parts.append(f"type: {agency_type}")
            lines.append(f"**Agency:** {', '.join(parts)}")
            lines.append("")

    # Spatial topology
    spatial = genre_profile.get("spatial_topology", {})
    if spatial:
        lines.append("**Spatial Topology:**")
        for key, val in spatial.items():
            lines.append(f"- {key}: {val}")
        lines.append("")

    # Temporal register
    temporal = genre_profile.get("temporal_register", {})
    if temporal:
        lines.append("**Temporal Register:**")
        for key, val in temporal.items():
            lines.append(f"- {key}: {val}")
        lines.append("")

    # Narrative contract
    contract = genre_profile.get("narrative_contract", {})
    if contract:
        lines.append("**Narrative Contract:**")
        for key, val in contract.items():
            lines.append(f"- {key}: {val}")
        lines.append("")

    # Any remaining top-level keys not already handled
    handled = {
        "world_affordances",
        "aesthetic_register",
        "agency",
        "spatial_topology",
        "temporal_register",
        "narrative_contract",
    }
    extras = {k: v for k, v in genre_profile.items() if k not in handled}
    if extras:
        lines.append("**Additional Genre Signals:**")
        for key, val in extras.items():
            if isinstance(val, (str, int, float, bool)):
                lines.append(f"- {key}: {val}")
            elif isinstance(val, dict):
                lines.append(f"- {key}:")
                for subk, subv in val.items():
                    lines.append(f"    - {subk}: {subv}")
            elif isinstance(val, list):
                lines.append(f"- {key}: {', '.join(str(v) for v in val)}")
        lines.append("")

    if not lines:
        return "Genre profile present but contains no extractable signals."

    return "\n".join(lines).rstrip()


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
) -> str:
    """Substitute all placeholders into the place-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    world_preamble = _build_world_preamble(world_pos)
    genre_profile_summary = _build_genre_profile_summary(genre_profile)

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", world_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_places_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of place objects.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown ```json ... ``` code fence.
    3. Find the outermost [ ... ] array boundaries and parse that.

    Args:
        response: Raw LLM response text.

    Returns:
        List of place dicts.

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
        "Could not parse LLM response as a JSON array of places. "
        f"Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_places(data_path: Path, world_slug: str) -> None:
    """Elicit named places for a Tome world.

    Reads world-position.json, builds a structured prompt from axis positions
    and genre profile, calls the elicitation model, parses the JSON response,
    and writes places.json to the world directory.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier — must match a directory under
            {data_path}/narrative-data/tome/worlds/.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "place-elicitation.md"

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
    # 2. Load prompt template
    # ------------------------------------------------------------------
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # ------------------------------------------------------------------
    # 3. Build prompt
    # ------------------------------------------------------------------
    console.print("[bold]Building prompt…[/bold]")
    prompt = _build_prompt(template, world_pos, genre_profile)
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # ------------------------------------------------------------------
    # 4. Call LLM
    # ------------------------------------------------------------------
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_PLACE_TIMEOUT}s, temperature={_PLACE_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_PLACE_TIMEOUT,
        temperature=_PLACE_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # ------------------------------------------------------------------
    # 5. Parse response
    # ------------------------------------------------------------------
    console.print("[bold]Parsing response…[/bold]")
    try:
        places = _parse_places_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    console.print(f"  Parsed [green]{len(places)}[/green] place(s)")

    # ------------------------------------------------------------------
    # 6. Write places.json
    # ------------------------------------------------------------------
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "place_count": len(places),
        "places": places,
    }

    output_path = world_dir / "places.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # ------------------------------------------------------------------
    # 7. Summary
    # ------------------------------------------------------------------
    console.print()
    console.print(f"[bold]Places generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for place in places:
        slug = place.get("slug", "?")
        name = place.get("name", "?")
        place_type = place.get("place_type", "?")
        spatial_role = place.get("spatial_role", "?")
        console.print(
            f"  [green]✓[/green] [bold]{name}[/bold] "
            f"[dim]({slug}, {place_type}, {spatial_role})[/dim]"
        )
