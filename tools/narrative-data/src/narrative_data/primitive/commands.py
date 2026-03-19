"""Primitive elicitation: Phase 3 Layer 0 standalone descriptions."""

from pathlib import Path

from rich.console import Console

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.events import append_event
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_content_digest,
    compute_prompt_hash,
    load_manifest,
    update_manifest_entry,
)
from narrative_data.prompts import PromptBuilder
from narrative_data.utils import now_iso, slug_to_name

console = Console()


def elicit_primitives(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    primitives: list[str],
    descriptions: dict[str, str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 3: Elicit standalone Layer 0 descriptions for each primitive."""
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()
    type_dir = output_base / primitive_type
    manifest_path = type_dir / "manifest.json"

    for prim_slug in primitives:
        prim_dir = type_dir / prim_slug
        prim_dir.mkdir(parents=True, exist_ok=True)
        output_path = prim_dir / "raw.md"

        description = descriptions.get(prim_slug, "")
        prim_name = slug_to_name(prim_slug)

        try:
            prompt = builder.build_stage1(
                domain="primitive",
                category=primitive_type,
                target_name=prim_name,
                context={"synthesis_description": description} if description else None,
            )
        except FileNotFoundError:
            console.print(f"[dim]  Skipping {primitive_type} — prompt template missing[/dim]")
            return

        current_hash = compute_prompt_hash(prompt)
        if not force:
            entry = load_manifest(manifest_path).get("entries", {}).get(prim_slug)
            if entry and entry.get("prompt_hash") == current_hash and output_path.exists():
                console.print(f"[dim]  {prim_slug} up to date, skipping[/dim]")
                continue

        append_event(
            log_path, event="elicit_started", phase=3, type=primitive_type, primitive=prim_slug
        )
        console.print(f"[cyan]  Eliciting {primitive_type}/{prim_slug}…[/cyan]")

        result_text = client.generate(model=model, prompt=prompt)
        archive_existing(output_path)
        output_path.write_text(result_text)

        digest = compute_content_digest(result_text)
        update_manifest_entry(
            manifest_path,
            prim_slug,
            {
                "prompt_hash": current_hash,
                "content_digest": digest,
                "elicited_at": now_iso(),
                "raw_path": str(output_path),
            },
        )

        append_event(
            log_path,
            event="elicit_completed",
            phase=3,
            type=primitive_type,
            primitive=prim_slug,
            output=str(output_path.relative_to(output_base)),
            content_digest=digest,
        )
