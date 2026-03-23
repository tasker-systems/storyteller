# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Stage 2 structuring orchestration: connects the segment pipeline to the data corpus.

Knows which schema to use for which type, where files live, and how to iterate
over genres and clusters. The bridge between the CLI and the segment pipeline
(slice → extract → aggregate → write).
"""

import json
from dataclasses import dataclass, field
from pathlib import Path

from pydantic import BaseModel
from rich.console import Console

from narrative_data.config import GENRE_CLUSTERS, MODIFIER_REGIONS
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.aggregator import aggregate_discovery, aggregate_genre_dimensions
from narrative_data.pipeline.slicer import slice_file
from narrative_data.pipeline.structure import run_segment_extraction
from narrative_data.schemas.archetype_dynamics import ArchetypeDynamic, ClusterArchetypeDynamic
from narrative_data.schemas.archetypes import Archetype, ClusterArchetype
from narrative_data.schemas.dynamics import ClusterDynamic, Dynamic
from narrative_data.schemas.genre_dimensions import (
    AestheticDimensions,
    AgencyDimensions,
    EpistemologicalDimensions,
    GenreDimensions,
    NarrativeContract,
    TemporalDimensions,
    ThematicDimensions,
    TonalDimensions,
    WorldAffordances,
)
from narrative_data.schemas.goals import ClusterGoal, Goal
from narrative_data.schemas.narrative_shapes import NarrativeShape
from narrative_data.schemas.ontological_posture import ClusterOntologicalPosture, OntologicalPosture
from narrative_data.schemas.place_entities import ClusterPlaceEntity, PlaceEntity
from narrative_data.schemas.scene_profiles import ClusterSceneProfile, SceneProfile
from narrative_data.schemas.settings import ClusterSettings, Settings
from narrative_data.schemas.shared import GenreBoundary, StateVariableTemplate
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
    doc_type: str = "discovery"  # slicer dispatch key


# ---------------------------------------------------------------------------
# Segment config for genre-region extraction
# ---------------------------------------------------------------------------


@dataclass
class SegmentConfig:
    """Schema source and prompt slug for a single genre-region segment."""

    schema_source: dict
    prompt_slug: str


GENRE_REGION_SEGMENT_MAP: dict[str, SegmentConfig] = {
    "meta": SegmentConfig(
        schema_source={
            "type": "object",
            "properties": {
                "genre_slug": {"type": "string"},
                "genre_name": {"type": "string"},
                "classification": {
                    "type": "string",
                    "enum": ["standalone_region", "constraint_layer", "hybrid_modifier"],
                },
                "constraint_layer_type": {
                    "type": ["string", "null"],
                    "enum": [
                        "world_affordance",
                        "tonal_emotional",
                        "agency_outcome",
                        "setting_locus",
                        None,
                    ],
                },
                "modifies": {"type": "array", "items": {"type": "string"}},
                "flavor_text": {"type": ["string", "null"]},
            },
            "required": ["genre_slug", "genre_name", "classification"],
        },
        prompt_slug="genre-region-meta",
    ),
    "aesthetic": SegmentConfig(
        schema_source=AestheticDimensions.model_json_schema(),
        prompt_slug="genre-region-aesthetic",
    ),
    "tonal": SegmentConfig(
        schema_source=TonalDimensions.model_json_schema(),
        prompt_slug="genre-region-tonal",
    ),
    "temporal": SegmentConfig(
        schema_source=TemporalDimensions.model_json_schema(),
        prompt_slug="genre-region-temporal",
    ),
    "thematic": SegmentConfig(
        schema_source=ThematicDimensions.model_json_schema(),
        prompt_slug="genre-region-thematic",
    ),
    "agency": SegmentConfig(
        schema_source=AgencyDimensions.model_json_schema(),
        prompt_slug="genre-region-agency",
    ),
    "epistemological": SegmentConfig(
        schema_source=EpistemologicalDimensions.model_json_schema(),
        prompt_slug="genre-region-epistemological",
    ),
    "world-affordances": SegmentConfig(
        schema_source=WorldAffordances.model_json_schema(),
        prompt_slug="genre-region-world-affordances",
    ),
    "locus-of-power": SegmentConfig(
        schema_source={
            "type": "array",
            "items": {
                "type": "string",
                "enum": ["place", "person", "system", "relationship", "cosmos"],
            },
            "maxItems": 3,
        },
        prompt_slug="genre-region-locus-of-power",
    ),
    "narrative-structure": SegmentConfig(
        schema_source={
            "type": "array",
            "items": {
                "type": "string",
                "enum": ["quest", "mystery", "tragedy", "comedy", "romance", "horror"],
            },
            "maxItems": 3,
        },
        prompt_slug="genre-region-narrative-structure",
    ),
    "narrative-contracts": SegmentConfig(
        schema_source={"type": "array", "items": NarrativeContract.model_json_schema()},
        prompt_slug="genre-region-narrative-contracts",
    ),
    "state-variables": SegmentConfig(
        schema_source={"type": "array", "items": StateVariableTemplate.model_json_schema()},
        prompt_slug="genre-region-state-variables",
    ),
    "boundaries": SegmentConfig(
        schema_source={"type": "array", "items": GenreBoundary.model_json_schema()},
        prompt_slug="genre-region-boundaries",
    ),
}


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
        doc_type="genre-region",
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
        doc_type="tropes",
    ),
    "narrative-shapes": TypeConfig(
        per_genre=NarrativeShape,
        cluster=None,
        data_dir="genres",
        file_pattern="narrative-shapes",
        prompt_slug="narrative-shapes",
        doc_type="narrative-shapes",
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


def _resolve_segment_config(config: TypeConfig, segment_name: str) -> tuple[dict, str]:
    """Return (schema_dict, prompt_slug) for a given segment."""
    if config.doc_type == "genre-region":
        seg_config = GENRE_REGION_SEGMENT_MAP.get(segment_name)
        if seg_config is None:
            raise ValueError(f"Unknown genre-region segment: {segment_name}")
        return seg_config.schema_source, seg_config.prompt_slug
    elif config.doc_type == "cluster":
        assert config.cluster is not None
        return config.cluster.model_json_schema(), "discovery-entity-cluster"
    elif config.doc_type == "tropes":
        return config.per_genre.model_json_schema(), "trope-entity"
    elif config.doc_type == "narrative-shapes":
        return config.per_genre.model_json_schema(), "narrative-shape-entity"
    else:  # discovery
        return config.per_genre.model_json_schema(), "discovery-entity"


# ---------------------------------------------------------------------------
# Genre context injection (slug + state variables from region.json)
# ---------------------------------------------------------------------------


def _build_genre_context(output_base: Path, genre_slug: str) -> dict[str, str]:
    """Build extra template context for a genre's segment extraction.

    Reads the genre's region.json (if it exists) to extract canonical state
    variable IDs, and provides the genre slug for direct injection into prompts.
    """
    context: dict[str, str] = {"genre_slug": genre_slug}

    region_json = output_base / "genres" / genre_slug / "region.json"
    if region_json.exists():
        data = json.loads(region_json.read_text())
        sv_ids = [sv["canonical_id"] for sv in data.get("active_state_variables", [])]
        if sv_ids:
            context["state_variables"] = ", ".join(sv_ids)
        else:
            context["state_variables"] = "(none defined for this genre)"
    else:
        context["state_variables"] = "(no region data available)"

    return context


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
    """Structure per-genre files for a given type using the segment pipeline.

    Flow per genre: slice → extract each segment → aggregate → write final JSON.
    Returns a summary dict with 'succeeded', 'failed', 'skipped' counts.
    """
    if type_slug not in TYPE_REGISTRY:
        raise ValueError(f"Unknown type slug '{type_slug}'. Valid slugs: {sorted(TYPE_REGISTRY)}")
    config = TYPE_REGISTRY[type_slug]

    target_genres = genres if genres is not None else ALL_GENRES

    succeeded = 0
    failed = 0
    skipped = 0

    for genre_slug in target_genres:
        source_path, final_output_path = _resolve_per_genre_paths(output_base, config, genre_slug)

        if not source_path.exists():
            console.print(f"[dim]  ⊘ {genre_slug} — no source .md, skipping[/dim]")
            skipped += 1
            continue

        if plan_only:
            console.print(f"[cyan]  Would structure {type_slug} for {genre_slug}[/cyan]")
            skipped += 1
            continue

        console.print(f"[cyan]Structuring {type_slug} for {genre_slug}...[/cyan]")

        # Build extra context for prompt injection (genre slug + state variables)
        extra_context = _build_genre_context(output_base, genre_slug)

        # Segment directory is sibling to source, named after the stem
        segment_dir = source_path.parent / source_path.stem

        try:
            # Step 1: Slice
            segments = slice_file(source_path, segment_dir, config.doc_type, force=force)
            console.print(f"  [dim]Sliced → {len(segments)} segments[/dim]")

            # Step 2: Extract each segment
            all_succeeded = True
            for seg in segments:
                seg_json_path = seg.path.with_suffix(".json")

                # Skip if cached (json exists and not forced)
                if not force and seg_json_path.exists():
                    console.print(f"  [dim]⊘ {seg.name} (cached)[/dim]")
                    continue

                # Determine schema + prompt slug for this segment
                schema, prompt_slug = _resolve_segment_config(config, seg.name)

                model_label = f" ({model})" if model else ""
                console.print(f"  [cyan]{seg.name}{model_label}[/cyan]", end=" ")

                kwargs: dict = {
                    "client": client,
                    "segment_path": seg.path,
                    "output_path": seg_json_path,
                    "schema": schema,
                    "segment_prompt_slug": prompt_slug,
                    "extra_context": extra_context,
                }
                if model:
                    kwargs["model"] = model

                result = run_segment_extraction(**kwargs)
                if result["success"]:
                    console.print("[green]✓[/green]")
                else:
                    console.print("[red]✗[/red]")
                    all_succeeded = False

            if not all_succeeded:
                console.print(f"[red]✗ {genre_slug} (segment extraction failed)[/red]")
                failed += 1
                continue

            # Step 3: Aggregate
            console.print("  [cyan]Aggregating...[/cyan]", end=" ")
            if config.doc_type == "genre-region":
                result_obj = aggregate_genre_dimensions(segment_dir)
                output_data = result_obj.model_dump()
            else:
                result_list = aggregate_discovery(segment_dir, config.per_genre)
                output_data = [e.model_dump() for e in result_list]

            # Step 4: Write final JSON
            final_output_path.parent.mkdir(parents=True, exist_ok=True)
            final_output_path.write_text(json.dumps(output_data, indent=2))
            console.print("[green]✓[/green]")
            console.print(f"[green]✓ {genre_slug}[/green]")
            succeeded += 1

        except Exception as e:
            console.print(f"\n[red]✗ {genre_slug} ({e})[/red]")
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
    """Structure cluster synthesis files for a given type using the segment pipeline.

    Flow per cluster: slice → extract each segment → aggregate → write final JSON.
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

    succeeded = 0
    failed = 0
    skipped = 0

    for cluster_name in GENRE_CLUSTERS:
        source_path, final_output_path = _resolve_cluster_paths(output_base, config, cluster_name)

        if not source_path.exists():
            console.print(f"[dim]  ⊘ cluster-{cluster_name} — no source .md, skipping[/dim]")
            skipped += 1
            continue

        if plan_only:
            console.print(
                f"[cyan]  Would structure {type_slug} cluster '{cluster_name}'[/cyan] "
                f"→ {final_output_path.relative_to(output_base)}"
            )
            skipped += 1
            continue

        model_label = f" ({model})" if model else ""
        console.print(
            f"[cyan]Structuring {type_slug} for cluster '{cluster_name}'{model_label}...[/cyan]"
        )

        # Segment directory is sibling to source, named after the stem
        segment_dir = source_path.parent / source_path.stem

        try:
            # Step 1: Slice
            segments = slice_file(source_path, segment_dir, "cluster", force=force)
            console.print(f"  [dim]Sliced → {len(segments)} segments[/dim]")

            # Step 2: Extract each segment
            all_succeeded = True
            for seg in segments:
                seg_json_path = seg.path.with_suffix(".json")

                if not force and seg_json_path.exists():
                    console.print(f"  [dim]⊘ {seg.name} (cached)[/dim]")
                    continue

                schema = config.cluster.model_json_schema()
                prompt_slug = "discovery-entity-cluster"

                console.print(f"  [cyan]{seg.name}{model_label}[/cyan]", end=" ")

                kwargs: dict = {
                    "client": client,
                    "segment_path": seg.path,
                    "output_path": seg_json_path,
                    "schema": schema,
                    "segment_prompt_slug": prompt_slug,
                }
                if model:
                    kwargs["model"] = model

                result = run_segment_extraction(**kwargs)
                if result["success"]:
                    console.print("[green]✓[/green]")
                else:
                    console.print("[red]✗[/red]")
                    all_succeeded = False

            if not all_succeeded:
                console.print(f"[red]✗ cluster-{cluster_name} (segment extraction failed)[/red]")
                failed += 1
                continue

            # Step 3: Aggregate
            console.print("  [cyan]Aggregating...[/cyan]", end=" ")
            result_list = aggregate_discovery(segment_dir, config.cluster)
            output_data = [e.model_dump() for e in result_list]

            # Step 4: Write final JSON
            final_output_path.parent.mkdir(parents=True, exist_ok=True)
            final_output_path.write_text(json.dumps(output_data, indent=2))
            console.print("[green]✓[/green]")
            console.print(f"[green]✓ cluster-{cluster_name}[/green]")
            succeeded += 1

        except Exception as e:
            console.print(f"\n[red]✗ cluster-{cluster_name} ({e})[/red]")
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
