"""Stage 1: Elicitation via large model → raw.md."""

from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_content_digest,
    compute_prompt_hash,
)
from narrative_data.prompts import PromptBuilder


def run_elicitation(
    client: OllamaClient,
    builder: PromptBuilder,
    domain: str,
    category: str,
    target_name: str,
    target_slug: str,
    output_dir: Path,
    model: str = ELICITATION_MODEL,
    context: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Run stage 1 elicitation: build prompt, call large model, write raw.md.

    Returns a dict with prompt_hash, content_digest, and raw_path.
    """
    prompt = builder.build_stage1(
        domain=domain,
        category=category,
        target_name=target_name,
        context=context,
    )
    prompt_hash = compute_prompt_hash(prompt)
    raw_content = client.generate(model=model, prompt=prompt)
    output_dir.mkdir(parents=True, exist_ok=True)
    raw_path = output_dir / f"{category}.raw.md"
    archived = archive_existing(raw_path)
    raw_path.write_text(raw_content)
    content_digest = compute_content_digest(raw_content)
    return {
        "prompt_hash": prompt_hash,
        "content_digest": content_digest,
        "raw_path": str(raw_path),
        "archived_from": str(archived) if archived else None,
    }
