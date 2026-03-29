# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Coherence engine for the Tome decomposed elicitation pipeline.

Takes draft entities from the fan-out phase and runs a single 35b coherence
call to bind them relationally. This is the fan-in half of the fan-out/fan-in
architecture — each entity stage gets one coherence call that produces the
final, relationally-bound entity array.

Usage (called by the pipeline orchestrator):
    entities = cohere(client, template_path, world_summary, draft_entities, upstream_context)
    save_coherence_output(decomposed_dir, stage, entities, world_slug, genre_slug, setting_slug)
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING, Any

from narrative_data.config import ELICITATION_TIMEOUT
from narrative_data.tome.models import get_model
from narrative_data.utils import now_iso

if TYPE_CHECKING:
    from narrative_data.ollama import OllamaClient

_COHERENCE_TEMPERATURE = 0.5

_STAGE_ENTITY_KEY: dict[str, str] = {
    "places": "places",
    "orgs": "organizations",
    "substrate": "clusters",
    "characters-mundane": "characters",
    "characters-significant": "characters",
}

_STAGE_FILENAME: dict[str, str] = {
    "places": "places.json",
    "orgs": "organizations.json",
    "substrate": "social-substrate.json",
    "characters-mundane": "characters-mundane.json",
    "characters-significant": "characters-significant.json",
}


# ---------------------------------------------------------------------------
# _build_coherence_prompt
# ---------------------------------------------------------------------------


def _build_coherence_prompt(
    template_path: Path,
    world_summary: dict[str, Any],
    draft_entities: list[dict[str, Any]],
    upstream_context: str,
    extra_context: dict[str, str] | None = None,
) -> str:
    """Load a coherence template and substitute all placeholders.

    Args:
        template_path: Path to the coherence prompt template file.
        world_summary: Dict with keys ``genre_slug``, ``setting_slug``,
            ``compressed_preamble``.
        draft_entities: List of entity dicts from the fan-out phase.
        upstream_context: Pre-formatted string of upstream entity context
            (e.g. places summary for the orgs coherence call).
        extra_context: Optional additional key/value substitutions.

    Returns:
        Fully substituted prompt string.
    """
    text = template_path.read_text()

    genre_slug = world_summary.get("genre_slug", "unknown")
    setting_slug = world_summary.get("setting_slug", "unknown")
    compressed_preamble = world_summary.get("compressed_preamble", "")

    text = (
        text.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{compressed_preamble}", compressed_preamble)
        .replace("{draft_entities}", json.dumps(draft_entities, indent=2))
        .replace("{upstream_context}", upstream_context)
    )

    if extra_context:
        for key, value in extra_context.items():
            text = text.replace(f"{{{key}}}", str(value))

    return text


# ---------------------------------------------------------------------------
# _parse_coherence_response
# ---------------------------------------------------------------------------


def _parse_coherence_response(response: str) -> list[dict[str, Any]] | dict[str, Any]:
    """Parse an LLM response as a JSON array or object.

    Most coherence calls return a JSON array of entities. The substrate
    coherence call returns a dict with "clusters" and "relationships" keys.
    Both forms are accepted.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown code fence.
    3. Find the outermost JSON boundaries and parse.

    Args:
        response: Raw LLM response text.

    Returns:
        Parsed list of entity dicts, or dict (for substrate coherence).

    Raises:
        ValueError: If all strategies fail.
    """
    text = response.strip()

    def _acceptable(val: object) -> bool:
        return isinstance(val, list | dict)

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if _acceptable(result):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from code fence
    fence_match = re.search(r"```(?:json)?\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if _acceptable(result):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 3: find outermost JSON structure
    # Try array first, then object
    for open_ch, close_ch in [("[", "]"), ("{", "}")]:
        start = text.find(open_ch)
        end = text.rfind(close_ch)
        if start != -1 and end != -1 and end > start:
            try:
                result = json.loads(text[start : end + 1])
                if _acceptable(result):
                    return result
            except json.JSONDecodeError:
                pass

    raise ValueError(
        f"Could not parse coherence response as JSON. Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# cohere
# ---------------------------------------------------------------------------


def cohere(
    client: OllamaClient,
    template_path: Path,
    world_summary: dict[str, Any],
    draft_entities: list[dict[str, Any]],
    upstream_context: str,
    extra_context: dict[str, str] | None = None,
) -> list[dict[str, Any]] | dict[str, Any]:
    """Run a 35b coherence call to bind draft entities relationally.

    Builds a prompt from the template and world context, calls the coherence
    model (qwen3.5:35b), and parses the JSON array response.

    Args:
        client: Configured OllamaClient instance.
        template_path: Path to the coherence prompt template file.
        world_summary: Dict with keys ``genre_slug``, ``setting_slug``,
            ``compressed_preamble``.
        draft_entities: List of entity dicts from the fan-out phase.
        upstream_context: Pre-formatted string of upstream entity context.
        extra_context: Optional additional key/value substitutions for the template.

    Returns:
        List of relationally-bound entity dicts.

    Raises:
        ValueError: If the LLM response cannot be parsed as a JSON array.
    """
    model = get_model("coherence")
    prompt = _build_coherence_prompt(
        template_path,
        world_summary,
        draft_entities,
        upstream_context,
        extra_context,
    )

    response = client.generate(
        model=model,
        prompt=prompt,
        timeout=ELICITATION_TIMEOUT,
        temperature=_COHERENCE_TEMPERATURE,
    )

    return _parse_coherence_response(response)


# ---------------------------------------------------------------------------
# save_coherence_output
# ---------------------------------------------------------------------------


def save_coherence_output(
    decomposed_dir: Path,
    stage: str,
    entities: list[dict[str, Any]],
    world_slug: str,
    genre_slug: str,
    setting_slug: str,
) -> Path:
    """Write the final per-stage coherence output as a JSON file with metadata.

    Args:
        decomposed_dir: Root directory for decomposed pipeline outputs.
        stage: Stage name — one of "places", "orgs", "substrate",
            "characters-mundane", "characters-significant".
        entities: List of relationally-bound entity dicts from :func:`cohere`.
        world_slug: World identifier.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.

    Returns:
        Path to the written JSON file.

    Raises:
        KeyError: If ``stage`` is not a recognised stage name.
    """
    entity_key = _STAGE_ENTITY_KEY[stage]
    filename = _STAGE_FILENAME[stage]

    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "pipeline": "decomposed",
        "count": len(entities),
        entity_key: entities,
    }

    decomposed_dir.mkdir(parents=True, exist_ok=True)
    output_path = decomposed_dir / filename
    output_path.write_text(json.dumps(output, indent=2))

    return output_path
