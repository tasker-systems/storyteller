# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Fan-out dispatch engine for the Tome decomposed elicitation pipeline.

Each FanOutSpec produces one entity via one LLM call. This module handles
parallel dispatch of all specs concurrently, with per-instance retry on
parse failure. Failed instances are logged and skipped; the caller receives
only successfully parsed entity dicts, sorted by spec index.

Usage (called by the pipeline orchestrator):
    results = fan_out(client, template_dir, specs)
    save_instances(decomposed_dir, stage, specs, results)
    aggregate(decomposed_dir, stage, results)
"""

from __future__ import annotations

import json
import logging
import re
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import TYPE_CHECKING, Any

from narrative_data.config import STRUCTURING_TIMEOUT
from narrative_data.tome.models import FanOutSpec, get_model

if TYPE_CHECKING:
    from narrative_data.ollama import OllamaClient

_log = logging.getLogger(__name__)

_FAN_OUT_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# _build_fan_out_prompt
# ---------------------------------------------------------------------------


def _build_fan_out_prompt(template_dir: Path, spec: FanOutSpec) -> str:
    """Load a template and substitute all context keys.

    Args:
        template_dir: Directory containing prompt template files.
        spec: FanOutSpec with template_name and context dict.

    Returns:
        Fully substituted prompt string.
    """
    template_path = template_dir / spec.template_name
    text = template_path.read_text()

    for key, value in spec.context.items():
        text = text.replace(f"{{{key}}}", str(value))

    return text


# ---------------------------------------------------------------------------
# _parse_fan_out_response
# ---------------------------------------------------------------------------


def _parse_fan_out_response(response: str) -> dict[str, Any]:
    """Parse an LLM response as a single JSON object.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown ```json ... ``` or ``` ... ``` code fence.
    3. Find the outermost { ... } object boundaries and parse that.

    Args:
        response: Raw LLM response text.

    Returns:
        Parsed entity dict.

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

    # Strategy 2a: extract from ```json ... ``` fence
    fence_match = re.search(r"```json\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        try:
            result = json.loads(fence_match.group(1))
            if isinstance(result, dict):
                return result
        except json.JSONDecodeError:
            pass

    # Strategy 2b: extract from plain ``` ... ``` fence
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
        f"Could not parse LLM response as a JSON entity dict. Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# _generate_one
# ---------------------------------------------------------------------------


def _generate_one(
    client: OllamaClient,
    template_dir: Path,
    spec: FanOutSpec,
) -> dict[str, Any]:
    """Generate one entity with one retry on parse failure.

    First attempt uses the base prompt. Second attempt appends a JSON-only
    hint to encourage the model to produce clean output.

    Args:
        client: Configured OllamaClient instance.
        template_dir: Directory containing prompt template files.
        spec: FanOutSpec describing the entity to generate.

    Returns:
        Parsed entity dict.

    Raises:
        ValueError: If both attempts fail to produce parseable JSON.
    """
    model = get_model(spec.model_role)
    base_prompt = _build_fan_out_prompt(template_dir, spec)

    # First attempt
    response = client.generate(
        model=model,
        prompt=base_prompt,
        timeout=STRUCTURING_TIMEOUT,
        temperature=_FAN_OUT_TEMPERATURE,
    )
    try:
        return _parse_fan_out_response(response)
    except ValueError:
        pass

    # Second attempt with JSON hint
    retry_prompt = base_prompt + "\n\nOutput valid JSON only."
    response = client.generate(
        model=model,
        prompt=retry_prompt,
        timeout=STRUCTURING_TIMEOUT,
        temperature=_FAN_OUT_TEMPERATURE,
    )
    return _parse_fan_out_response(response)


# ---------------------------------------------------------------------------
# fan_out
# ---------------------------------------------------------------------------


def fan_out(
    client: OllamaClient,
    template_dir: Path,
    specs: list[FanOutSpec],
) -> list[dict[str, Any]]:
    """Dispatch all specs concurrently and return successfully parsed entity dicts.

    Uses a ThreadPoolExecutor with max_workers=4 for parallel LLM calls.
    Failed instances are logged and skipped. Results are sorted by spec.index
    for deterministic ordering.

    Args:
        client: Configured OllamaClient instance.
        template_dir: Directory containing prompt template files.
        specs: List of FanOutSpec instances to dispatch.

    Returns:
        List of successfully parsed entity dicts, sorted by spec.index.
    """
    if not specs:
        return []

    indexed_results: dict[int, dict[str, Any]] = {}

    with ThreadPoolExecutor(max_workers=4) as executor:
        future_to_spec = {
            executor.submit(_generate_one, client, template_dir, spec): spec for spec in specs
        }

        for future in as_completed(future_to_spec):
            spec = future_to_spec[future]
            try:
                result = future.result()
                indexed_results[spec.index] = result
            except Exception as exc:
                _log.warning(
                    "Fan-out instance failed: stage=%s index=%d error=%s",
                    spec.stage,
                    spec.index,
                    exc,
                )

    return [indexed_results[i] for i in sorted(indexed_results)]


# ---------------------------------------------------------------------------
# save_instances
# ---------------------------------------------------------------------------


def save_instances(
    decomposed_dir: Path,
    stage: str,
    specs: list[FanOutSpec],
    results: list[dict[str, Any]],
) -> None:
    """Save individual fan-out results to per-instance files.

    Files are written to ``decomposed_dir / "fan-out" / stage / spec.output_filename``.
    Directories are created as needed.

    Args:
        decomposed_dir: Root directory for decomposed pipeline outputs.
        stage: Stage name (e.g. "places", "organizations").
        specs: List of FanOutSpecs in the same order as results.
        results: List of parsed entity dicts, one per spec.
    """
    output_dir = decomposed_dir / "fan-out" / stage
    output_dir.mkdir(parents=True, exist_ok=True)

    for spec, result in zip(specs, results, strict=True):
        path = output_dir / spec.output_filename
        path.write_text(json.dumps(result, indent=2))


# ---------------------------------------------------------------------------
# aggregate
# ---------------------------------------------------------------------------


def aggregate(
    decomposed_dir: Path,
    stage: str,
    results: list[dict[str, Any]],
) -> None:
    """Write aggregated draft file for a stage.

    The draft is written to ``decomposed_dir / f"{stage}-draft.json"`` as a
    JSON array of all successfully generated entity dicts.

    Args:
        decomposed_dir: Root directory for decomposed pipeline outputs.
        stage: Stage name (e.g. "places", "organizations").
        results: List of parsed entity dicts to aggregate.
    """
    decomposed_dir.mkdir(parents=True, exist_ok=True)
    draft_path = decomposed_dir / f"{stage}-draft.json"
    draft_path.write_text(json.dumps(results, indent=2))
