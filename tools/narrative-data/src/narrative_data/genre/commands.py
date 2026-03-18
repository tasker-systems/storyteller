"""Genre command orchestration: elicitation and structuring for B.1."""

from pathlib import Path
from typing import Any

from rich.console import Console

from narrative_data.config import (
    ELICITATION_MODEL,
    GENRE_CATEGORIES,
    STRUCTURING_MODEL,
    resolve_descriptor_dir,
)
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.pipeline.invalidation import (
    compute_prompt_hash,
    is_stale,
    load_manifest,
    update_manifest_entry,
)
from narrative_data.pipeline.structure import run_structuring
from narrative_data.prompts import PromptBuilder
from narrative_data.schemas.genre import (
    GenreArchetype,
    GenreDynamic,
    GenreGoal,
    GenreProfile,
    GenreRegion,
    GenreSetting,
    NarrativeShape,
    Trope,
)
from narrative_data.utils import now_iso, slug_to_name

console = Console()

GENRE_REGIONS: list[str] = [
    "folk-horror",
    "cosmic-horror",
    "high-epic-fantasy",
    "dark-fantasy",
    "cozy-fantasy",
    "fairy-tale-mythic",
    "urban-fantasy",
    "hard-sci-fi",
    "space-opera",
    "cyberpunk",
    "solarpunk",
    "nordic-noir",
    "cozy-mystery",
    "psychological-thriller",
    "romantasy",
    "historical-romance",
    "contemporary-romance",
    "literary-fiction",
    "magical-realism",
    "southern-gothic",
    "historical-fiction",
    "westerns",
    "swashbuckling-adventure",
    "survival-fiction",
    "pastoral-rural-fiction",
]

CATEGORY_SCHEMAS: dict[str, tuple[type, bool]] = {
    "region": (GenreRegion, False),
    "archetypes": (GenreArchetype, True),
    "tropes": (Trope, True),
    "narrative-shapes": (NarrativeShape, True),
    "dynamics": (GenreDynamic, True),
    "profiles": (GenreProfile, True),
    "goals": (GenreGoal, True),
    "settings": (GenreSetting, True),
}


def _order_categories(categories: list[str]) -> list[str]:
    """Return categories with 'region' first, preserving relative order of the rest."""
    ordered = []
    if "region" in categories:
        ordered.append("region")
    for cat in categories:
        if cat != "region":
            ordered.append(cat)
    return ordered


def _load_descriptor_context() -> dict[str, str]:
    """Load existing flat descriptors from training-data/descriptors/ for context."""
    context: dict[str, str] = {}
    try:
        desc_dir = resolve_descriptor_dir()
        for name in ["archetypes", "dynamics", "profiles", "goals", "genres"]:
            path = desc_dir / f"{name}.json"
            if path.exists():
                context[f"existing_{name}"] = path.read_text()
    except RuntimeError:
        pass  # STORYTELLER_DATA_PATH not set
    return context


def elicit_genre(
    client: OllamaClient,
    output_base: Path,
    manifest_path: Path,
    regions: list[str] | None = None,
    categories: list[str] | None = None,
    model: str = ELICITATION_MODEL,
    force: bool = False,
) -> None:
    """Stage 1: Elicit raw markdown for each genre region × category.

    Skips up-to-date cells unless force=True.
    'region' category is always processed before dependent categories.
    """
    if regions is None:
        regions = GENRE_REGIONS
    if categories is None:
        categories = GENRE_CATEGORIES

    ordered_categories = _order_categories(categories)
    manifest = load_manifest(manifest_path)
    builder = PromptBuilder()
    descriptor_context = _load_descriptor_context()

    for region_slug in regions:
        region_dir = output_base / "genres" / region_slug
        region_name = slug_to_name(region_slug)

        for category in ordered_categories:
            manifest_key = f"{region_slug}/{category}"
            entry: dict[str, Any] | None = manifest["entries"].get(manifest_key)

            # Build context for this cell.
            # "region" needs no descriptor context — it's about dimensional positioning.
            # Derivative categories (archetypes, tropes, etc.) get existing descriptors
            # so the model knows what vocabulary exists and can extend/contextualize it.
            context: dict[str, str] = {}
            if category != "region":
                context.update(descriptor_context)
                region_json_path = region_dir / "region.json"
                if region_json_path.exists():
                    context["region"] = region_json_path.read_text()

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
                console.print(
                    f"[dim]  {region_slug}/{category} up to date, skipping[/dim]"
                )
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
    """Stage 2: Structure raw markdown into validated JSON for each genre cell.

    Skips cells where structured .json already exists unless force=True.
    Assigns UUIDv7 via uuid_utils.
    """
    if regions is None:
        regions = GENRE_REGIONS
    if categories is None:
        categories = GENRE_CATEGORIES

    ordered_categories = _order_categories(categories)

    for region_slug in regions:
        region_dir = output_base / "genres" / region_slug
        region_name = slug_to_name(region_slug)

        for category in ordered_categories:
            manifest_key = f"{region_slug}/{category}"
            raw_path = region_dir / f"{category}.raw.md"
            output_path = region_dir / f"{category}.json"

            if not raw_path.exists():
                console.print(
                    f"[dim]  {region_slug}/{category} raw.md missing, skipping[/dim]"
                )
                continue

            if not force and output_path.exists():
                entry = load_manifest(manifest_path)["entries"].get(manifest_key, {})
                if entry.get("structured_at"):
                    console.print(
                        f"[dim]  {region_slug}/{category} already structured, skipping[/dim]"
                    )
                    continue

            schema_type, is_collection = CATEGORY_SCHEMAS.get(
                category, (GenreRegion, False)
            )
            console.print(f"[cyan]  Structuring {region_slug}/{category}…[/cyan]")
            result = run_structuring(
                client=client,
                raw_path=raw_path,
                output_path=output_path,
                schema_type=schema_type,
                model=model,
                is_collection=is_collection,
            )

            if result["success"]:
                console.print(
                    f"[green]  {region_slug}/{category} structured OK[/green]"
                )
                update_manifest_entry(
                    manifest_path,
                    manifest_key,
                    {
                        **load_manifest(manifest_path)["entries"].get(manifest_key, {}),
                        "structured_at": now_iso(),
                        "output_path": result["output_path"],
                        "region_name": region_name,
                    },
                )
            else:
                console.print(
                    f"[red]  {region_slug}/{category} structuring failed — "
                    f"errors at {result.get('errors_path')}[/red]"
                )
