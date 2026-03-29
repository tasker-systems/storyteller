# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Entity budget planning for the Tome decomposed elicitation pipeline.

Produces an entity-plan.json that drives the fan-out phase — determining how
many places, organizations, social clusters, and characters (mundane + significant)
to generate. The planning call uses the fan_out_structured model (7b-instruct).

The plan is derived from the world's material conditions encoded as compressed
axis positions, not from genre conventions.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING, Any

from narrative_data.tome.models import get_model

if TYPE_CHECKING:
    from narrative_data.ollama import OllamaClient

_PLAN_TIMEOUT = 120.0
_PLAN_TEMPERATURE = 0.3

_REQUIRED_KEYS = [
    "places",
    "organizations",
    "clusters",
    "characters_mundane",
    "characters_significant",
]


# ---------------------------------------------------------------------------
# _build_plan_prompt
# ---------------------------------------------------------------------------


def _build_plan_prompt(
    template_path: Path,
    world_summary: dict[str, Any],
    genre_profile_summary: str,
) -> str:
    """Substitute placeholders into the entity-plan prompt template.

    Args:
        template_path: Path to the entity-plan.md prompt template.
        world_summary: Dict produced by :func:`compress_preamble.build_world_summary`,
            containing keys ``genre_slug``, ``setting_slug``, ``compressed_preamble``.
        genre_profile_summary: Pre-formatted genre profile markdown string.

    Returns:
        Fully substituted prompt string ready for LLM submission.
    """
    template = template_path.read_text()

    genre_slug = world_summary.get("genre_slug", "unknown")
    setting_slug = world_summary.get("setting_slug", "unknown")
    compressed_preamble = world_summary.get("compressed_preamble", "")

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{compressed_preamble}", compressed_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
    )


# ---------------------------------------------------------------------------
# _parse_plan_response
# ---------------------------------------------------------------------------


def _parse_plan_response(response: str) -> dict[str, Any]:
    """Parse an LLM response as a JSON entity-plan dict.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown ```json ... ``` code fence.
    3. Find the outermost { ... } object boundaries and parse that.

    Args:
        response: Raw LLM response text.

    Returns:
        Parsed entity-plan dict.

    Raises:
        ValueError: If all strategies fail or the result is not a dict.
    """
    text = response.strip()

    # Strategy 1: direct parse
    try:
        result = json.loads(text)
        if isinstance(result, dict):
            return result
    except json.JSONDecodeError:
        pass

    # Strategy 2: extract from ```json ... ``` fence
    fence_match = re.search(r"```json\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    # Also try plain ``` fence
    plain_fence_match = re.search(r"```\s*(\{.*?)\s*```", text, re.DOTALL)
    if plain_fence_match:
        try:
            result = json.loads(plain_fence_match.group(1))
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 3: find outermost { ... } object
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        try:
            result = json.loads(text[start : end + 1])
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    raise ValueError(
        "Could not parse LLM response as a JSON entity-plan dict. "
        f"Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# _validate_plan
# ---------------------------------------------------------------------------


def _validate_plan(plan: dict[str, Any]) -> None:
    """Validate the structure of an entity plan dict.

    Checks:
    - All required top-level keys are present.
    - If ``places`` has both ``count`` and ``distribution``, the distribution
      values must sum to ``count``.

    Args:
        plan: Parsed entity plan dict.

    Raises:
        ValueError: If any validation check fails.
    """
    for key in _REQUIRED_KEYS:
        if key not in plan:
            raise ValueError(
                f"Entity plan missing required key '{key}'. Present keys: {list(plan.keys())}"
            )

    places = plan["places"]
    if isinstance(places, dict) and "distribution" in places:
        distribution = places["distribution"]
        if isinstance(distribution, dict):
            # Filter out zero-count types and use distribution sum as authoritative count
            places["distribution"] = {
                k: v for k, v in distribution.items() if isinstance(v, int | float) and v > 0
            }
            dist_sum = sum(places["distribution"].values())
            places["count"] = dist_sum


# ---------------------------------------------------------------------------
# plan_entities
# ---------------------------------------------------------------------------


def plan_entities(
    client: OllamaClient,
    template_path: Path,
    world_summary: dict[str, Any],
    genre_profile_summary: str,
) -> dict[str, Any]:
    """Plan the entity budget for a Tome world.

    Builds a prompt from the world summary and genre profile, calls the
    fan_out_structured model (7b-instruct), parses the response, and validates
    the resulting entity plan.

    Args:
        client: Configured :class:`OllamaClient` instance.
        template_path: Path to the entity-plan.md prompt template.
        world_summary: Dict produced by :func:`compress_preamble.build_world_summary`,
            containing ``genre_slug``, ``setting_slug``, ``compressed_preamble``.
        genre_profile_summary: Pre-formatted genre profile markdown string.

    Returns:
        Validated entity plan dict with keys: places, organizations, clusters,
        characters_mundane, characters_significant.

    Raises:
        ValueError: If the LLM response cannot be parsed or fails validation.
    """
    model = get_model("fan_out_structured")
    prompt = _build_plan_prompt(template_path, world_summary, genre_profile_summary)

    response = client.generate(
        model=model,
        prompt=prompt,
        timeout=_PLAN_TIMEOUT,
        temperature=_PLAN_TEMPERATURE,
    )

    plan = _parse_plan_response(response)
    _validate_plan(plan)

    return plan
