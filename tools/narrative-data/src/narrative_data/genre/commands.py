"""Genre command orchestration: elicitation and structuring for B.1."""

from pathlib import Path
from typing import Any

from rich.console import Console

from narrative_data.config import (
    ELICITATION_MODEL,
    GENRE_CATEGORIES,
    STRUCTURING_MODEL,
)
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.pipeline.events import append_event
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_content_digest,
    compute_prompt_hash,
    is_stale,
    load_manifest,
    update_manifest_entry,
)
from narrative_data.prompts import PromptBuilder
from narrative_data.utils import now_iso, slug_to_name

console = Console()

GENRE_REGIONS: list[str] = [
    # Horror
    "folk-horror",
    "cosmic-horror",
    # Fantasy
    "high-epic-fantasy",
    "dark-fantasy",
    "cozy-fantasy",
    "fairy-tale-mythic",
    "urban-fantasy",
    "quiet-contemplative-fantasy",
    # Science fiction
    "hard-sci-fi",
    "space-opera",
    "cyberpunk",
    # Mystery / thriller
    "nordic-noir",
    "cozy-mystery",
    "psychological-thriller",
    "domestic-noir",
    # Romance
    "romantasy",
    "historical-romance",
    "contemporary-romance",
    # Gothic
    "southern-gothic",
    # Historical / period
    "westerns",
    # Adventure / action
    "swashbuckling-adventure",
    "survival-fiction",
    # Comedy-horror
    "horror-comedy",
    # Realism
    "working-class-realism",
    "pastoral-rural-fiction",
    # Tragedy
    "classical-tragedy",
    # Modifier regions (self-identify via enriched prompt)
    "solarpunk",
    "historical-fiction",
    "literary-fiction",
    "magical-realism",
]


def _order_categories(categories: list[str]) -> list[str]:
    """Return categories with 'region' first, preserving relative order of the rest."""
    ordered = []
    if "region" in categories:
        ordered.append("region")
    for cat in categories:
        if cat != "region":
            ordered.append(cat)
    return ordered


def elicit_genre(
    client: OllamaClient,
    output_base: Path,
    manifest_path: Path,
    regions: list[str] | None = None,
    categories: list[str] | None = None,
    model: str = ELICITATION_MODEL,
    force: bool = False,
) -> None:
    """Stage 1: Elicit raw markdown for genre region descriptions.

    Now restricted to 'region' category only. For primitive elaboration use
    elaborate_genre(); for genre-native types (tropes, narrative-shapes) use
    elicit_native().

    Skips up-to-date cells unless force=True.
    """
    non_region = [c for c in (categories or GENRE_CATEGORIES) if c != "region"]
    if non_region:
        raise ValueError(
            f"Categories {non_region} are no longer supported via 'genre elicit'. "
            "Use 'narrative-data discover' for primitive extraction, "
            "'narrative-data primitive' for standalone elicitation, "
            "or 'narrative-data genre elaborate' for genre × primitive elaboration."
        )

    if regions is None:
        regions = GENRE_REGIONS
    if categories is None:
        categories = GENRE_CATEGORIES

    ordered_categories = _order_categories(categories)
    manifest = load_manifest(manifest_path)
    builder = PromptBuilder()

    for region_slug in regions:
        region_dir = output_base / "genres" / region_slug
        region_name = slug_to_name(region_slug)

        for category in ordered_categories:
            manifest_key = f"{region_slug}/{category}"
            entry: dict[str, Any] | None = manifest["entries"].get(manifest_key)

            context: dict[str, str] = {}

            # Compute prompt hash to check staleness
            try:
                prompt = builder.build_stage1(
                    domain="genre",
                    category=category,
                    target_name=region_name,
                    context=context if context else None,
                )
                current_hash = compute_prompt_hash(prompt)
            except FileNotFoundError:
                # Prompt template not yet authored — skip silently
                console.print(
                    f"[dim]  Skipping {region_slug}/{category} — prompt template missing[/dim]"
                )
                continue

            if not force and not is_stale(entry, current_hash):
                console.print(f"[dim]  {region_slug}/{category} up to date, skipping[/dim]")
                continue

            console.print(f"[cyan]  Eliciting {region_slug}/{category}…[/cyan]")
            result = run_elicitation(
                client=client,
                builder=builder,
                domain="genre",
                category=category,
                target_name=region_name,
                target_slug=region_slug,
                output_dir=region_dir,
                model=model,
                context=context if context else None,
            )

            update_manifest_entry(
                manifest_path,
                manifest_key,
                {
                    "prompt_hash": result["prompt_hash"],
                    "content_digest": result["content_digest"],
                    "elicited_at": now_iso(),
                    "raw_path": result["raw_path"],
                },
            )
            manifest = load_manifest(manifest_path)  # refresh


def structure_genre(
    client: OllamaClient,
    output_base: Path,
    manifest_path: Path,
    regions: list[str] | None = None,
    categories: list[str] | None = None,
    model: str = STRUCTURING_MODEL,
    force: bool = False,
) -> None:
    """Stage 2 structuring — replaced by the new 'structure' CLI command (Task 10).

    This function is a no-op placeholder. The old schema types (GenreRegion, etc.)
    have been removed as part of the Stage 2 architecture migration.
    """
    console.print(
        "[yellow]  structure_genre() is deprecated"
        " — use 'narrative-data structure' instead[/yellow]"
    )


def elaborate_genre(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    genres: list[str],
    primitives: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 4a: Elaborate a primitive within each genre's context.

    For each (genre, primitive) pair, assembles context from the genre's
    region description and the primitive's standalone Layer 0 description,
    then generates a genre-specific elaboration.
    """
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()

    for genre_slug in genres:
        genre_dir = output_base / "genres" / genre_slug
        region_path = genre_dir / "region.md"
        if not region_path.exists():
            console.print(f"[dim]  Skipping {genre_slug} — no region.md[/dim]")
            continue

        genre_content = region_path.read_text()
        elab_dir = genre_dir / "elaborations" / primitive_type
        elab_dir.mkdir(parents=True, exist_ok=True)

        for prim_slug in primitives:
            output_path = elab_dir / f"{prim_slug}.md"
            prim_path = output_base / primitive_type / prim_slug / "raw.md"

            context: dict[str, str] = {"genre_description": genre_content}
            if prim_path.exists():
                context["primitive_description"] = prim_path.read_text()

            prim_name = slug_to_name(prim_slug)
            genre_name = slug_to_name(genre_slug)
            target_name = f"{prim_name} in {genre_name}"

            try:
                prompt = builder.build_stage1(
                    domain="genre",
                    category=f"elaborate-{primitive_type}",
                    target_name=target_name,
                    context=context,
                )
            except FileNotFoundError:
                console.print(
                    f"[dim]  Skipping elaborate-{primitive_type} — prompt template missing[/dim]"
                )
                return

            if not force and output_path.exists():
                console.print(
                    f"[dim]  {genre_slug}/{primitive_type}/{prim_slug} "
                    f"exists, skipping (use --force to re-elaborate)[/dim]"
                )
                continue

            append_event(
                log_path,
                event="elaborate_started",
                phase=4,
                type=primitive_type,
                genre=genre_slug,
                primitive=prim_slug,
            )
            console.print(
                f"[cyan]  Elaborating {primitive_type}/{prim_slug} for {genre_slug}…[/cyan]"
            )

            result_text = client.generate(model=model, prompt=prompt)
            archive_existing(output_path)
            output_path.write_text(result_text)

            digest = compute_content_digest(result_text)
            append_event(
                log_path,
                event="elaborate_completed",
                phase=4,
                type=primitive_type,
                genre=genre_slug,
                primitive=prim_slug,
                output=str(output_path.relative_to(output_base)),
                content_digest=digest,
            )


def elicit_native(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    native_type: str,
    genres: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 4b: Elicit genre-native content (tropes, narrative-shapes).

    These are inherently genre-specific — they don't have standalone Layer 0
    descriptions. Context is the genre's region description only.
    """
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()

    for genre_slug in genres:
        genre_dir = output_base / "genres" / genre_slug
        region_path = genre_dir / "region.md"
        if not region_path.exists():
            console.print(f"[dim]  Skipping {genre_slug} — no region.md[/dim]")
            continue

        genre_content = region_path.read_text()
        output_path = genre_dir / f"{native_type}.md"
        genre_name = slug_to_name(genre_slug)

        context: dict[str, str] = {"genre_description": genre_content}

        try:
            prompt = builder.build_stage1(
                domain="genre",
                category=native_type,
                target_name=genre_name,
                context=context,
            )
        except FileNotFoundError:
            console.print(f"[dim]  Skipping {native_type} — prompt template missing[/dim]")
            return

        if not force and output_path.exists():
            console.print(f"[dim]  {genre_slug}/{native_type} exists, skipping (use --force)[/dim]")
            continue

        append_event(
            log_path,
            event="elicit_native_started",
            phase=4,
            type=native_type,
            genre=genre_slug,
        )
        console.print(f"[cyan]  Eliciting {native_type} for {genre_slug}…[/cyan]")

        result_text = client.generate(model=model, prompt=prompt)
        archive_existing(output_path)
        output_path.write_text(result_text)

        digest = compute_content_digest(result_text)
        append_event(
            log_path,
            event="elicit_native_completed",
            phase=4,
            type=native_type,
            genre=genre_slug,
            output=str(output_path.relative_to(output_base)),
            content_digest=digest,
        )
