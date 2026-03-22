"""Stage 2 structuring orchestration: connects run_structuring() to the data corpus.

Knows which schema to use for which type, where files live, and how to iterate
over genres and clusters. The bridge between the CLI and run_structuring().
"""

from dataclasses import dataclass, field
from pathlib import Path

from pydantic import BaseModel
from rich.console import Console

from narrative_data.config import GENRE_CLUSTERS, MODIFIER_REGIONS
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.invalidation import load_manifest, update_manifest_entry
from narrative_data.pipeline.structure import run_structuring
from narrative_data.schemas.archetype_dynamics import ArchetypeDynamic, ClusterArchetypeDynamic
from narrative_data.schemas.archetypes import Archetype, ClusterArchetype
from narrative_data.schemas.dynamics import ClusterDynamic, Dynamic
from narrative_data.schemas.genre_dimensions import GenreDimensions
from narrative_data.schemas.goals import ClusterGoal, Goal
from narrative_data.schemas.narrative_shapes import NarrativeShape
from narrative_data.schemas.ontological_posture import ClusterOntologicalPosture, OntologicalPosture
from narrative_data.schemas.place_entities import ClusterPlaceEntity, PlaceEntity
from narrative_data.schemas.scene_profiles import ClusterSceneProfile, SceneProfile
from narrative_data.schemas.settings import ClusterSettings, Settings
from narrative_data.schemas.spatial_topology import ClusterSpatialTopology, SpatialTopologyEdge
from narrative_data.schemas.tropes import Trope

console = Console()

# ---------------------------------------------------------------------------
# All 30 genres: 26 from GENRE_CLUSTERS values + 4 MODIFIER_REGIONS
# ---------------------------------------------------------------------------

ALL_GENRES: list[str] = [
    genre for genres in GENRE_CLUSTERS.values() for genre in genres
] + MODIFIER_REGIONS


# ---------------------------------------------------------------------------
# TypeConfig
# ---------------------------------------------------------------------------


@dataclass
class TypeConfig:
    """Everything needed to run structuring for one type."""

    per_genre: type[BaseModel]
    cluster: type[BaseModel] | None
    data_dir: str  # relative to output_base (e.g., "discovery/archetypes" or "genres")
    file_pattern: str | None = None  # filename override when data_dir == "genres"
    is_collection: bool = True  # False for GenreDimensions (single object)
    prompt_slug: str = field(default="")


# ---------------------------------------------------------------------------
# TYPE_REGISTRY
# ---------------------------------------------------------------------------

TYPE_REGISTRY: dict[str, TypeConfig] = {
    "genre-dimensions": TypeConfig(
        per_genre=GenreDimensions,
        cluster=None,
        data_dir="genres",
        file_pattern="region",
        is_collection=False,
        prompt_slug="genre-dimensions",
    ),
    "archetypes": TypeConfig(
        per_genre=Archetype,
        cluster=ClusterArchetype,
        data_dir="discovery/archetypes",
        prompt_slug="archetypes",
    ),
    "dynamics": TypeConfig(
        per_genre=Dynamic,
        cluster=ClusterDynamic,
        data_dir="discovery/dynamics",
        prompt_slug="dynamics",
    ),
    "goals": TypeConfig(
        per_genre=Goal,
        cluster=ClusterGoal,
        data_dir="discovery/goals",
        prompt_slug="goals",
    ),
    "profiles": TypeConfig(
        per_genre=SceneProfile,
        cluster=ClusterSceneProfile,
        data_dir="discovery/profiles",
        prompt_slug="profiles",
    ),
    "settings": TypeConfig(
        per_genre=Settings,
        cluster=ClusterSettings,
        data_dir="discovery/settings",
        prompt_slug="settings",
    ),
    "ontological-posture": TypeConfig(
        per_genre=OntologicalPosture,
        cluster=ClusterOntologicalPosture,
        data_dir="discovery/ontological-posture",
        prompt_slug="ontological-posture",
    ),
    "archetype-dynamics": TypeConfig(
        per_genre=ArchetypeDynamic,
        cluster=ClusterArchetypeDynamic,
        data_dir="discovery/archetype-dynamics",
        prompt_slug="archetype-dynamics",
    ),
    "spatial-topology": TypeConfig(
        per_genre=SpatialTopologyEdge,
        cluster=ClusterSpatialTopology,
        data_dir="discovery/spatial-topology",
        prompt_slug="spatial-topology",
    ),
    "place-entities": TypeConfig(
        per_genre=PlaceEntity,
        cluster=ClusterPlaceEntity,
        data_dir="discovery/place-entities",
        prompt_slug="place-entities",
    ),
    "tropes": TypeConfig(
        per_genre=Trope,
        cluster=None,
        data_dir="genres",
        file_pattern="tropes",
        prompt_slug="tropes",
    ),
    "narrative-shapes": TypeConfig(
        per_genre=NarrativeShape,
        cluster=None,
        data_dir="genres",
        file_pattern="narrative-shapes",
        prompt_slug="narrative-shapes",
    ),
}

# Manifest file path (relative to output_base)
_MANIFEST_FILENAME = "structure_manifest.json"


# ---------------------------------------------------------------------------
# Path resolution helpers
# ---------------------------------------------------------------------------


def _resolve_per_genre_paths(
    output_base: Path, config: TypeConfig, genre_slug: str
) -> tuple[Path, Path]:
    """Return (source_md_path, output_json_path) for a given genre and type config."""
    if config.data_dir == "genres":
        # genre-native or genre-dimensions: genres/{genre}/{file_pattern}.md
        assert config.file_pattern is not None
        source = output_base / "genres" / genre_slug / f"{config.file_pattern}.md"
        output = output_base / "genres" / genre_slug / f"{config.file_pattern}.json"
    else:
        # discovery types: {data_dir}/{genre}.md → {data_dir}/{genre}.json
        source = output_base / config.data_dir / f"{genre_slug}.md"
        output = output_base / config.data_dir / f"{genre_slug}.json"
    return source, output


def _resolve_cluster_paths(
    output_base: Path, config: TypeConfig, cluster_name: str
) -> tuple[Path, Path]:
    """Return (source_md_path, output_json_path) for a cluster file."""
    source = output_base / config.data_dir / f"cluster-{cluster_name}.md"
    output = output_base / config.data_dir / f"cluster-{cluster_name}.json"
    return source, output


# ---------------------------------------------------------------------------
# structure_type()
# ---------------------------------------------------------------------------


def structure_type(
    client: OllamaClient,
    output_base: Path,
    type_slug: str,
    genres: list[str] | None = None,
    force: bool = False,
    plan_only: bool = False,
    model: str | None = None,
) -> dict:
    """Structure per-genre files for a given type.

    Returns a summary dict with 'succeeded', 'failed', 'skipped' counts.
    """
    if type_slug not in TYPE_REGISTRY:
        raise ValueError(f"Unknown type slug '{type_slug}'. Valid slugs: {sorted(TYPE_REGISTRY)}")
    config = TYPE_REGISTRY[type_slug]

    target_genres = genres if genres is not None else ALL_GENRES

    manifest_path = output_base / _MANIFEST_FILENAME
    manifest = load_manifest(manifest_path)

    succeeded = 0
    failed = 0
    skipped = 0

    for genre_slug in target_genres:
        source_path, output_path = _resolve_per_genre_paths(output_base, config, genre_slug)

        if not source_path.exists():
            console.print(f"[dim]  ⊘ {genre_slug} — no source .md, skipping[/dim]")
            skipped += 1
            continue

        manifest_key = f"{type_slug}/{genre_slug}"
        cached_entry = manifest.get("entries", {}).get(manifest_key)

        if not force and cached_entry and cached_entry.get("success"):
            console.print(f"[dim]⊘ {genre_slug} (cached)[/dim]")
            skipped += 1
            continue

        if plan_only:
            console.print(
                f"[cyan]  Would structure {type_slug} for {genre_slug}[/cyan] "
                f"→ {output_path.relative_to(output_base)}"
            )
            skipped += 1
            continue

        model_label = f" ({model})" if model else ""
        console.print(f"[cyan]Structuring {type_slug} for {genre_slug}{model_label}...[/cyan]")
        kwargs: dict = {
            "client": client,
            "raw_path": source_path,
            "output_path": output_path,
            "schema_type": config.per_genre,
            "structure_type": config.prompt_slug,
            "is_collection": config.is_collection,
        }
        if model:
            kwargs["model"] = model
        result = run_structuring(**kwargs)

        if result["success"]:
            console.print(f"[green]✓ {genre_slug}[/green]")
            update_manifest_entry(
                manifest_path,
                manifest_key,
                {"success": True, "output_path": result.get("output_path")},
            )
            manifest = load_manifest(manifest_path)
            succeeded += 1
        else:
            console.print(f"[red]✗ {genre_slug} (failed)[/red]")
            failed += 1

    _print_summary(type_slug, succeeded, failed, skipped)
    return {"succeeded": succeeded, "failed": failed, "skipped": skipped}


# ---------------------------------------------------------------------------
# structure_clusters()
# ---------------------------------------------------------------------------


def structure_clusters(
    client: OllamaClient,
    output_base: Path,
    type_slug: str,
    force: bool = False,
    plan_only: bool = False,
    model: str | None = None,
) -> dict:
    """Structure cluster synthesis files for a given type.

    Returns a summary dict with 'succeeded', 'failed', 'skipped' counts.
    Raises ValueError if the type has no cluster schema (tropes, narrative-shapes,
    genre-dimensions).
    """
    if type_slug not in TYPE_REGISTRY:
        raise ValueError(f"Unknown type slug '{type_slug}'. Valid slugs: {sorted(TYPE_REGISTRY)}")
    config = TYPE_REGISTRY[type_slug]

    if config.cluster is None:
        raise ValueError(
            f"Type '{type_slug}' has no cluster schema. "
            "Cluster structuring is not supported for tropes, narrative-shapes, "
            "or genre-dimensions."
        )

    manifest_path = output_base / _MANIFEST_FILENAME
    manifest = load_manifest(manifest_path)

    succeeded = 0
    failed = 0
    skipped = 0

    for cluster_name in GENRE_CLUSTERS:
        source_path, output_path = _resolve_cluster_paths(output_base, config, cluster_name)

        if not source_path.exists():
            console.print(f"[dim]  ⊘ cluster-{cluster_name} — no source .md, skipping[/dim]")
            skipped += 1
            continue

        manifest_key = f"{type_slug}/cluster-{cluster_name}"
        cached_entry = manifest.get("entries", {}).get(manifest_key)

        if not force and cached_entry and cached_entry.get("success"):
            console.print(f"[dim]⊘ cluster-{cluster_name} (cached)[/dim]")
            skipped += 1
            continue

        cluster_prompt_slug = f"{config.prompt_slug}-cluster"

        if plan_only:
            console.print(
                f"[cyan]  Would structure {type_slug} cluster '{cluster_name}'[/cyan] "
                f"→ {output_path.relative_to(output_base)}"
            )
            skipped += 1
            continue

        model_label = f" ({model})" if model else ""
        console.print(
            f"[cyan]Structuring {type_slug} for cluster"
            f" '{cluster_name}'{model_label}...[/cyan]"
        )
        kwargs: dict = {
            "client": client,
            "raw_path": source_path,
            "output_path": output_path,
            "schema_type": config.cluster,
            "structure_type": cluster_prompt_slug,
            "is_collection": True,
        }
        if model:
            kwargs["model"] = model
        result = run_structuring(**kwargs)

        if result["success"]:
            console.print(f"[green]✓ cluster-{cluster_name}[/green]")
            update_manifest_entry(
                manifest_path,
                manifest_key,
                {"success": True, "output_path": result.get("output_path")},
            )
            manifest = load_manifest(manifest_path)
            succeeded += 1
        else:
            console.print(f"[red]✗ cluster-{cluster_name} (failed)[/red]")
            failed += 1

    _print_summary(f"{type_slug} clusters", succeeded, failed, skipped)
    return {"succeeded": succeeded, "failed": failed, "skipped": skipped}


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _print_summary(label: str, succeeded: int, failed: int, skipped: int) -> None:
    console.print(
        f"\n[bold]Summary ({label}):[/bold] "
        f"[green]{succeeded} succeeded[/green], "
        f"[red]{failed} failed[/red], "
        f"[dim]{skipped} skipped[/dim]"
    )
