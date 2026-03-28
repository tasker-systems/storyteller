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


def _format_dimension_value(val: Any) -> str:
    """Format a genre dimension value for prompt display."""
    if isinstance(val, dict):
        # Structured dimension with value + labels (e.g., aesthetic.sensory_density)
        v = val.get("value")
        low = val.get("low_label", "")
        high = val.get("high_label", "")
        flavor = val.get("flavor_text", "")
        parts: list[str] = []
        if v is not None:
            parts.append(str(v))
        label = low or high
        if label:
            parts.append(f"({label})")
        if flavor:
            parts.append(f"— {flavor[:120]}")
        return " ".join(parts) if parts else str(val)
    return str(val)


def _build_genre_profile_summary(genre_profile: dict[str, Any] | None) -> str:
    """Extract key genre signals from the genre_profile dict (region.json).

    The region.json structure uses top-level keys: aesthetic, tonal,
    temporal, thematic, agency, world_affordances, epistemological,
    locus_of_power, narrative_structure, etc.

    Args:
        genre_profile: Parsed region.json dict, or None if unavailable.

    Returns:
        Markdown-formatted genre profile summary string.
    """
    if genre_profile is None:
        return "Genre profile unavailable — no region.json found for this genre."

    lines: list[str] = []

    # Key genre signal groups in priority order
    _SIGNAL_GROUPS = [
        ("world_affordances", "World Affordances"),
        ("aesthetic", "Aesthetic Register"),
        ("tonal", "Tonal Register"),
        ("agency", "Agency"),
        ("temporal", "Temporal Register"),
        ("thematic", "Thematic Treatment"),
        ("epistemological", "Epistemological Frame"),
    ]

    for key, label in _SIGNAL_GROUPS:
        group = genre_profile.get(key)
        if not group or not isinstance(group, dict):
            continue
        lines.append(f"**{label}:**")
        for dim_name, dim_val in group.items():
            formatted = _format_dimension_value(dim_val)
            lines.append(f"- {dim_name}: {formatted}")
        lines.append("")

    # List-valued signals (locus_of_power, narrative_structure, etc.)
    _LIST_SIGNALS = [
        ("locus_of_power", "Locus of Power"),
        ("narrative_structure", "Narrative Structure"),
        ("narrative_contracts", "Narrative Contracts"),
        ("active_state_variables", "Active State Variables"),
        ("boundaries", "Genre Boundaries"),
    ]

    for key, label in _LIST_SIGNALS:
        val = genre_profile.get(key)
        if val and isinstance(val, list):
            lines.append(f"**{label}:** {', '.join(str(v) for v in val)}")
            lines.append("")

    if not lines:
        return "Genre profile present but contains no extractable signals."

    return "\n".join(lines).rstrip()


def _build_settings_context(data_path: Path, genre_slug: str) -> str:
    """Load genre settings from the discovery corpus and format for the prompt.

    Settings data lives at {data_path}/narrative-data/discovery/settings/{genre_slug}.json
    and contains structured setting archetypes with communicability dimensions,
    atmospheric palettes, and narrative functions.

    Args:
        data_path: Root of storyteller-data.
        genre_slug: Genre region slug.

    Returns:
        Markdown-formatted settings context, or empty string if unavailable.
    """
    settings_path = (
        data_path / "narrative-data" / "discovery" / "settings" / f"{genre_slug}.json"
    )
    if not settings_path.exists():
        return ""

    try:
        settings = json.loads(settings_path.read_text())
    except (json.JSONDecodeError, OSError):
        return ""

    if not isinstance(settings, list) or not settings:
        return ""

    lines = ["**Genre Setting Archetypes** (from bedrock — use as spatial inspiration):", ""]
    for s in settings:
        name = s.get("canonical_name", "?")
        func = s.get("narrative_function", "")
        comm = s.get("communicability", {})
        palette = s.get("atmospheric_palette", [])

        lines.append(f"- **{name}**")
        if func:
            lines.append(f"  Function: {func}")
        if comm:
            comm_parts = [f"{k}: {v}" for k, v in comm.items()]
            lines.append(f"  Communicability: {', '.join(comm_parts)}")
        if palette:
            lines.append(f"  Atmosphere: {', '.join(str(p) for p in palette[:3])}")
        lines.append("")

    return "\n".join(lines).rstrip()


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the place-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        settings_context: Formatted genre settings archetypes.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    world_preamble = _build_world_preamble(world_pos)
    genre_summary = _build_genre_profile_summary(genre_profile)

    # Combine genre profile and settings context
    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

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
    # 3. Build prompt (with genre settings context)
    # ------------------------------------------------------------------
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    if settings_context:
        console.print(f"  [dim]Loaded genre settings archetypes for {genre_slug}[/dim]")
    prompt = _build_prompt(template, world_pos, genre_profile, settings_context)
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
