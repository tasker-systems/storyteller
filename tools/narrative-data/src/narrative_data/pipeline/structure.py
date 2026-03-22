# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Stage 2: Structuring via small instruct model → validated .json."""

import json
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ValidationError

from narrative_data.config import STRUCTURING_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.prompts import PromptBuilder

ERROR_MARKER = "\n\n--- VALIDATION ERRORS ---\n"


def run_structuring(
    client: OllamaClient,
    raw_path: Path,
    output_path: Path,
    schema_type: type[BaseModel],
    structure_type: str,
    model: str = STRUCTURING_MODEL,
    is_collection: bool = True,
    max_retries: int = 3,
) -> dict[str, Any]:
    """Run stage 2 structuring: read raw.md, call small model, validate and write .json.

    Retries on validation failure, replacing (not appending) the error section each time
    to protect the 7b model's context window.
    On exhausting retries, writes a .errors.json file and returns success=False.
    """
    raw_content = raw_path.read_text()

    if is_collection:
        target_schema = {"type": "array", "items": schema_type.model_json_schema()}
    else:
        target_schema = schema_type.model_json_schema()

    base_prompt = PromptBuilder().build_structure(structure_type, raw_content, target_schema)
    errors: list[str] = []
    raw_output: Any = None
    last_error: str | None = None

    for _attempt in range(max_retries):
        if last_error:
            fix_suffix = "\nPlease fix and output valid JSON."
            current_prompt = base_prompt + ERROR_MARKER + last_error + fix_suffix
        else:
            current_prompt = base_prompt

        raw_output = client.generate_structured(
            model=model, prompt=current_prompt, schema=target_schema
        )

        try:
            validated = _validate_and_save(raw_output, schema_type, output_path, is_collection)
            return {"success": True, "output_path": str(output_path), "validated": validated}
        except ValidationError as e:
            last_error = str(e)
            errors.append(last_error)

    errors_path = output_path.with_suffix(".errors.json")
    errors_path.write_text(
        json.dumps(
            {"errors": errors, "raw_output": raw_output, "schema": schema_type.__name__},
            indent=2,
        )
    )
    return {"success": False, "errors_path": str(errors_path), "errors": errors}


def run_segment_extraction(
    client: OllamaClient,
    segment_path: Path,
    output_path: Path,
    schema: dict,
    segment_prompt_slug: str,
    model: str = STRUCTURING_MODEL,
    max_retries: int = 3,
) -> dict[str, Any]:
    """Extract structured JSON from a single segment.

    Unlike run_structuring(), this:
    - Takes a pre-computed schema dict (not a Pydantic type)
    - Uses build_segment_structure() for prompts (not build_structure())
    - Does NOT validate against Pydantic (aggregator does that later)
    - Strips YAML frontmatter from segment content before sending to LLM
    """
    raw_content = _strip_frontmatter(segment_path.read_text())

    base_prompt = PromptBuilder().build_segment_structure(segment_prompt_slug, raw_content, schema)
    errors: list[str] = []
    raw_output: Any = None
    last_error: str | None = None

    for _attempt in range(max_retries):
        if last_error:
            fix_suffix = "\nPlease fix and output valid JSON."
            current_prompt = base_prompt + ERROR_MARKER + last_error + fix_suffix
        else:
            current_prompt = base_prompt

        raw_output = client.generate_structured(model=model, prompt=current_prompt, schema=schema)

        # Basic JSON validity check (not Pydantic validation — aggregator does that)
        if isinstance(raw_output, (dict, list)):
            output_path.parent.mkdir(parents=True, exist_ok=True)
            output_path.write_text(json.dumps(raw_output, indent=2))
            return {"success": True, "output_path": str(output_path)}

        last_error = f"Expected dict or list, got {type(raw_output).__name__}"
        errors.append(last_error)

    errors_path = output_path.with_suffix(".errors.json")
    errors_path.write_text(
        json.dumps(
            {"errors": errors, "raw_output": raw_output, "prompt_slug": segment_prompt_slug},
            indent=2,
        )
    )
    return {"success": False, "errors_path": str(errors_path), "errors": errors}


def _strip_frontmatter(content: str) -> str:
    """Strip YAML frontmatter (---...---) from segment content."""
    if not content.startswith("---\n"):
        return content
    end = content.find("\n---\n", 4)
    if end == -1:
        return content
    return content[end + 5 :]  # skip past the closing ---\n


def _validate_and_save(
    data: Any,
    schema_type: type[BaseModel],
    output_path: Path,
    is_collection: bool,
) -> Any:
    if is_collection:
        validated = [schema_type.model_validate(item) for item in data]
        output_data = [item.model_dump() for item in validated]
    else:
        validated = schema_type.model_validate(data)
        output_data = validated.model_dump()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output_data, indent=2))
    return validated
