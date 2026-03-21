"""Discovery pipeline: Phase 1 extraction and Phase 2 cluster synthesis."""

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

# Primitive types that need additional context beyond the genre region description.
# Maps type -> list of (label, source_path_fn) pairs.
_ENRICHED_CONTEXT: dict[str, list[tuple[str, str]]] = {
    "archetype-dynamics": [
        ("archetypes", "discovery/archetypes/{genre_slug}.raw.md"),
        ("dynamics", "discovery/dynamics/{genre_slug}.raw.md"),
    ],
    "spatial-topology": [
        ("settings", "discovery/settings/{genre_slug}.raw.md"),
    ],
    "place-entities": [
        ("settings", "discovery/settings/{genre_slug}.raw.md"),
        ("spatial_topology", "discovery/spatial-topology/{genre_slug}.raw.md"),
    ],
}


def _assemble_genre_content(output_base: Path, genre_slug: str, primitive_type: str) -> str:
    """Assemble genre content for a discovery prompt, including additional sources if needed."""
    region_path = output_base / "genres" / genre_slug / "region.raw.md"
    content = region_path.read_text()

    enrichments = _ENRICHED_CONTEXT.get(primitive_type, [])
    for label, path_template in enrichments:
        source_path = output_base / path_template.format(genre_slug=genre_slug)
        if source_path.exists():
            content += f"\n\n---\n\n## {label.replace('_', ' ').title()} for {genre_slug}\n\n"
            content += source_path.read_text()
        else:
            console.print(
                f"[dim]  Note: {label} data not found for {genre_slug}, proceeding without[/dim]"
            )

    return content


def extract_primitives(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    genres: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 1: Extract primitive candidates from each genre's region description."""
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()
    disc_dir = output_base / "discovery" / primitive_type
    disc_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = disc_dir / "manifest.json"

    for genre_slug in genres:
        region_path = output_base / "genres" / genre_slug / "region.raw.md"
        if not region_path.exists():
            console.print(f"[dim]  Skipping {genre_slug} — no region.raw.md[/dim]")
            continue

        output_path = disc_dir / f"{genre_slug}.raw.md"
        manifest_key = f"{genre_slug}"

        # Assemble genre content — some types need additional context sources
        genre_content = _assemble_genre_content(output_base, genre_slug, primitive_type)
        try:
            prompt = builder.build_discovery(
                primitive_type, slug_to_name(genre_slug), genre_content
            )
        except FileNotFoundError:
            console.print(f"[dim]  Skipping {primitive_type} — prompt template missing[/dim]")
            return

        current_hash = compute_prompt_hash(prompt)
        if not force:
            entry = load_manifest(manifest_path).get("entries", {}).get(manifest_key)
            if entry and entry.get("prompt_hash") == current_hash and output_path.exists():
                console.print(f"[dim]  {genre_slug}/{primitive_type} up to date, skipping[/dim]")
                continue

        append_event(
            log_path, event="extract_started", phase=1, type=primitive_type, genre=genre_slug
        )
        console.print(f"[cyan]  Extracting {primitive_type} from {genre_slug}…[/cyan]")

        result_text = client.generate(model=model, prompt=prompt)
        archive_existing(output_path)
        output_path.write_text(result_text)

        digest = compute_content_digest(result_text)
        update_manifest_entry(
            manifest_path,
            manifest_key,
            {
                "prompt_hash": current_hash,
                "content_digest": digest,
                "elicited_at": now_iso(),
                "raw_path": str(output_path),
            },
        )

        append_event(
            log_path,
            event="extract_completed",
            phase=1,
            type=primitive_type,
            genre=genre_slug,
            output=str(output_path.relative_to(output_base)),
            content_digest=digest,
        )


def synthesize_cluster(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    cluster_name: str,
    genres: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 2: Synthesize per-genre extractions into a deduplicated cluster list."""
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()
    disc_dir = output_base / "discovery" / primitive_type
    output_path = disc_dir / f"cluster-{cluster_name}.raw.md"

    extractions: dict[str, str] = {}
    for genre_slug in genres:
        ext_path = disc_dir / f"{genre_slug}.raw.md"
        if ext_path.exists():
            extractions[genre_slug] = ext_path.read_text()
        else:
            console.print(f"[dim]  No extraction for {genre_slug}, skipping in synthesis[/dim]")

    if not extractions:
        console.print(
            f"[yellow]  No extractions found for cluster {cluster_name}, skipping[/yellow]"
        )
        return

    try:
        prompt = builder.build_synthesis(primitive_type, slug_to_name(cluster_name), extractions)
    except FileNotFoundError:
        console.print("[dim]  Skipping synthesis — prompt template missing[/dim]")
        return

    append_event(
        log_path, event="synthesize_started", phase=2, type=primitive_type, cluster=cluster_name
    )
    console.print(f"[cyan]  Synthesizing {primitive_type} for cluster {cluster_name}…[/cyan]")

    result_text = client.generate(model=model, prompt=prompt)
    archive_existing(output_path)
    output_path.write_text(result_text)

    digest = compute_content_digest(result_text)
    append_event(
        log_path,
        event="synthesize_completed",
        phase=2,
        type=primitive_type,
        cluster=cluster_name,
        output=str(output_path.relative_to(output_base)),
        content_digest=digest,
        primitives_found=result_text.count("\n##"),
    )
