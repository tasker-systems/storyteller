# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Annotate pending edge markdown files using qwen3.5:35b.

Reads each pending chunk from the manifest, extracts axis context and the pair
table from the generated markdown, sends them to the LLM via the edge-annotation
prompt template, and writes the annotated result back.  The old file is archived
before each write.

Usage (via CLI):
    uv run narrative-data tome annotate-edges [--chunks KEY,...] [--force]
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_prompt_hash,
    load_manifest,
    save_manifest,
)
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_ANNOTATION_TIMEOUT = 600.0
_ANNOTATION_TEMPERATURE = 0.3


# ---------------------------------------------------------------------------
# Markdown section extraction
# ---------------------------------------------------------------------------


def _extract_axis_context(content: str) -> str:
    """Extract everything before '## Edge Assessment' as axis context."""
    marker = "## Edge Assessment"
    idx = content.find(marker)
    if idx == -1:
        return content
    return content[:idx].rstrip()


def _extract_pair_table(content: str) -> str:
    """Extract the pair table from '## Edge Assessment' to '## Compound Edges'.

    Returns the section including the '## Edge Assessment' heading.
    """
    start_marker = "## Edge Assessment"
    end_marker = "## Compound Edges"
    start = content.find(start_marker)
    if start == -1:
        return ""
    end = content.find(end_marker, start)
    if end == -1:
        return content[start:].rstrip()
    return content[start:end].rstrip()


def _extract_tail_sections(content: str) -> str:
    """Extract everything from '## Compound Edges' onward.

    Returns the compound edges and cluster annotation sections so they can be
    preserved (or re-populated by the model) after the annotated table is
    inserted.
    """
    marker = "## Compound Edges"
    idx = content.find(marker)
    if idx == -1:
        return ""
    return content[idx:]


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(template: str, axis_context: str, pair_table: str) -> str:
    """Substitute axis_context and pair_table into the annotation template."""
    return template.replace("{axis_context}", axis_context).replace(
        "{pair_table}", pair_table
    )


# ---------------------------------------------------------------------------
# File reconstruction
# ---------------------------------------------------------------------------


def _reconstruct_file(
    original_content: str,
    annotated_response: str,
) -> str:
    """Reconstruct the full markdown file after annotation.

    Structure:
        <header + axis context + edge type definitions>  (unchanged)
        <annotated response from the model>              (replaces Edge Assessment table)
        <Compound Edges + Cluster Annotation>            (from model response if present,
                                                          otherwise original tail)

    The model is asked to fill in the pair table — its response should start with
    '## Edge Assessment' and may include '## Compound Edges' and '## Cluster Annotation'.
    If the model doesn't include the tail sections we restore the originals.
    """
    axis_context = _extract_axis_context(original_content)
    original_tail = _extract_tail_sections(original_content)

    # Normalise the model response
    response = annotated_response.strip()

    # Ensure the response begins with the Edge Assessment heading
    if not response.startswith("## Edge Assessment"):
        response = "## Edge Assessment\n\n" + response

    # If the model included compound/cluster sections, use them; otherwise
    # append the originals so the file stays structurally complete.
    has_compound = "## Compound Edges" in response
    has_cluster = "## Cluster Annotation" in response

    if not has_compound and not has_cluster and original_tail:
        response = response + "\n\n" + original_tail.strip()
    elif not has_compound and original_tail:
        # Only append compound + cluster if the model left them out entirely
        compound_idx = original_tail.find("## Compound Edges")
        if compound_idx != -1:
            response = response + "\n\n" + original_tail[compound_idx:].strip()

    return axis_context + "\n\n" + response + "\n"


# ---------------------------------------------------------------------------
# Core annotation function
# ---------------------------------------------------------------------------


def _annotate_chunk(
    chunk_key: str,
    entry: dict[str, Any],
    template: str,
    client: OllamaClient,
    console: Any,
) -> str:
    """Annotate a single chunk.  Returns the annotated file content."""
    filepath = Path(entry["filepath"])
    if not filepath.exists():
        raise FileNotFoundError(f"Edge file not found: {filepath}")

    original_content = filepath.read_text()

    axis_context = _extract_axis_context(original_content)
    pair_table = _extract_pair_table(original_content)

    if not pair_table:
        raise ValueError(f"Could not find pair table in {filepath}")

    prompt = _build_prompt(template, axis_context, pair_table)
    console.print(
        f"  [cyan]Annotating[/cyan] {chunk_key} "
        f"({entry.get('pair_count', '?')} pairs) …"
    )

    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_ANNOTATION_TIMEOUT,
        temperature=_ANNOTATION_TEMPERATURE,
    )

    return _reconstruct_file(original_content, response), prompt


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def annotate_all(
    data_path: Path,
    chunks: list[str] | None = None,
    force: bool = False,
) -> None:
    """Annotate all pending edge files.

    Args:
        data_path: Root of the storyteller-data repository.
        chunks: Optional list of manifest keys to annotate (e.g.
                ["material-conditions/economic-forms"]).  Annotates all
                pending chunks when None.
        force: Re-annotate even if the chunk is already annotated.
    """
    from rich.console import Console

    console = Console()

    edges_dir = data_path / "narrative-data" / "tome" / "edges"
    manifest_path = edges_dir / "manifest.json"
    template_path = _PROMPTS_DIR / "tome" / "edge-annotation.md"

    if not template_path.exists():
        console.print(f"[red]Prompt template not found: {template_path}[/red]")
        raise SystemExit(1)

    template = template_path.read_text()
    manifest = load_manifest(manifest_path)
    entries = manifest.get("entries", {})

    client = OllamaClient()

    # Determine which chunks to process
    if chunks is not None:
        targets = [(k, entries[k]) for k in chunks if k in entries]
        missing = [k for k in chunks if k not in entries]
        for k in missing:
            console.print(f"[yellow]Warning: chunk '{k}' not found in manifest[/yellow]")
    else:
        targets = list(entries.items())

    pending = []
    for key, entry in targets:
        status = entry.get("status", "pending")
        if status == "annotated" and not force:
            console.print(f"  [dim]Skipping {key} (already annotated)[/dim]")
            continue
        if status not in ("pending", "annotated"):
            console.print(f"  [yellow]Skipping {key} (status: {status})[/yellow]")
            continue
        pending.append((key, entry))

    if not pending:
        console.print("[green]Nothing to annotate.[/green]")
        return

    console.print(
        f"[cyan]Annotating {len(pending)} chunk(s) with {ELICITATION_MODEL}[/cyan]"
    )

    annotated_count = 0
    for key, entry in pending:
        filepath = Path(entry["filepath"])
        try:
            annotated_content, prompt = _annotate_chunk(
                key, entry, template, client, console
            )
        except Exception as exc:
            console.print(f"  [red]Error annotating {key}: {exc}[/red]")
            continue

        # Archive the existing file before writing
        archive_path = archive_existing(filepath)
        if archive_path:
            console.print(f"    [dim]Archived → {archive_path.name}[/dim]")

        filepath.write_text(annotated_content)

        # Update manifest entry
        prompt_hash = compute_prompt_hash(prompt)
        entry.update(
            {
                "status": "annotated",
                "annotated_at": now_iso(),
                "model": ELICITATION_MODEL,
                "prompt_hash": prompt_hash,
            }
        )
        manifest["entries"][key] = entry
        save_manifest(manifest_path, manifest)

        console.print(f"  [green]✓[/green] {key}")
        annotated_count += 1

    console.print()
    console.print(f"[bold green]Annotated {annotated_count} chunk(s)[/bold green]")
