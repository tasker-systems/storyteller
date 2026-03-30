# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Click CLI for narrative-data: genre, spatial, cross-pollination, status, and list commands."""

from pathlib import Path

import click

_PROMPTS_DIR = Path(__file__).parent.parent.parent / "prompts"


def _parse_list(value: str | None) -> list[str] | None:
    """Split a comma-separated string into a list of stripped strings, or return None."""
    if value is None:
        return None
    parts = [p.strip() for p in value.split(",") if p.strip()]
    return parts if parts else None


@click.group()
def cli() -> None:
    """Narrative data elicitation and structuring tooling for storyteller B.1/B.2/B.3."""


# ---------------------------------------------------------------------------
# genre subgroup
# ---------------------------------------------------------------------------


@cli.group()
def genre() -> None:
    """Genre elicitation and structuring commands (B.1)."""


@genre.command("elicit")
@click.option("--regions", default=None, help="Comma-separated list of region slugs.")
@click.option(
    "--categories",
    default=None,
    help="Comma-separated list of categories (e.g. region,archetypes).",
)
@click.option("--force", is_flag=True, default=False, help="Re-elicit even if up to date.")
def genre_elicit(regions: str | None, categories: str | None, force: bool) -> None:
    """Elicit raw markdown for each genre region × category (stage 1)."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.genre.commands import elicit_genre
    from narrative_data.ollama import OllamaClient

    output_base = resolve_output_path()
    manifest_path = output_base / "genres" / "manifest.json"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    elicit_genre(
        client=client,
        output_base=output_base,
        manifest_path=manifest_path,
        regions=_parse_list(regions),
        categories=_parse_list(categories),
        force=force,
    )


@genre.command("elaborate")
@click.option("--type", "primitive_type", required=True, help="Primitive type to elaborate.")
@click.option("--genres", default=None, help="Comma-separated genre slugs (default: all).")
@click.option("--primitives", default=None, help="Comma-separated primitive slugs.")
@click.option("--force", is_flag=True, default=False, help="Re-elaborate even if exists.")
def genre_elaborate(
    primitive_type: str, genres: str | None, primitives: str | None, force: bool
) -> None:
    """Phase 4a: Elaborate genre x primitive pairs."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.genre.commands import GENRE_REGIONS, elaborate_genre
    from narrative_data.ollama import OllamaClient

    output_base = resolve_output_path()
    log_path = output_base / "pipeline.jsonl"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    genre_list = _parse_list(genres) or GENRE_REGIONS
    prim_list = _parse_list(primitives) or []
    elaborate_genre(
        client=client,
        output_base=output_base,
        log_path=log_path,
        primitive_type=primitive_type,
        genres=genre_list,
        primitives=prim_list,
        force=force,
    )


@genre.command("elicit-native")
@click.option(
    "--type",
    "native_type",
    required=True,
    help="Genre-native type (tropes, narrative-shapes).",
)
@click.option("--genres", default=None, help="Comma-separated genre slugs (default: all).")
@click.option("--force", is_flag=True, default=False, help="Re-elicit even if exists.")
def genre_elicit_native(native_type: str, genres: str | None, force: bool) -> None:
    """Phase 4b: Elicit genre-native tropes or narrative-shapes."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.genre.commands import GENRE_REGIONS, elicit_native
    from narrative_data.ollama import OllamaClient

    output_base = resolve_output_path()
    log_path = output_base / "pipeline.jsonl"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    genre_list = _parse_list(genres) or GENRE_REGIONS
    elicit_native(
        client=client,
        output_base=output_base,
        log_path=log_path,
        native_type=native_type,
        genres=genre_list,
        force=force,
    )


@genre.command("structure")
@click.option("--regions", default=None, help="Comma-separated list of region slugs.")
@click.option(
    "--categories",
    default=None,
    help="Comma-separated list of categories.",
)
@click.option("--force", is_flag=True, default=False, help="Re-structure even if already done.")
def genre_structure(regions: str | None, categories: str | None, force: bool) -> None:
    """Structure raw markdown into validated JSON for each genre cell (stage 2)."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.genre.commands import structure_genre
    from narrative_data.ollama import OllamaClient

    output_base = resolve_output_path()
    manifest_path = output_base / "genres" / "manifest.json"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    structure_genre(
        client=client,
        output_base=output_base,
        manifest_path=manifest_path,
        regions=_parse_list(regions),
        categories=_parse_list(categories),
        force=force,
    )


# ---------------------------------------------------------------------------
# spatial subgroup
# ---------------------------------------------------------------------------


@cli.group()
def spatial() -> None:
    """Spatial/setting elicitation and structuring commands (B.2)."""


@spatial.command("elicit")
@click.option("--settings", default=None, help="Comma-separated list of setting slugs.")
@click.option("--force", is_flag=True, default=False, help="Re-elicit even if up to date.")
def spatial_elicit(settings: str | None, force: bool) -> None:
    """Elicit raw markdown for each setting × category (stage 1)."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.ollama import OllamaClient
    from narrative_data.spatial.commands import elicit_spatial

    output_base = resolve_output_path()
    manifest_path = output_base / "spatial" / "manifest.json"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    elicit_spatial(
        client=client,
        output_base=output_base,
        manifest_path=manifest_path,
        settings=_parse_list(settings),
        force=force,
    )


@spatial.command("structure")
@click.option("--settings", default=None, help="Comma-separated list of setting slugs.")
@click.option("--force", is_flag=True, default=False, help="Re-structure even if already done.")
def spatial_structure(settings: str | None, force: bool) -> None:
    """Structure raw markdown into validated JSON for each spatial cell (stage 2)."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.ollama import OllamaClient
    from narrative_data.spatial.commands import structure_spatial

    output_base = resolve_output_path()
    manifest_path = output_base / "spatial" / "manifest.json"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    structure_spatial(
        client=client,
        output_base=output_base,
        manifest_path=manifest_path,
        settings=_parse_list(settings),
        force=force,
    )


# ---------------------------------------------------------------------------
# cross-pollinate command
# ---------------------------------------------------------------------------


@cli.command("cross-pollinate")
@click.option("--force", is_flag=True, default=False, help="Re-run even if already complete.")
def cross_pollinate(force: bool) -> None:
    """Run cross-domain synthesis (B.3). Requires B.1 and B.2 completion."""
    from narrative_data.config import resolve_output_path
    from narrative_data.cross_pollination.commands import run_cross_pollination

    output_base = resolve_output_path()
    run_cross_pollination(output_base=output_base, force=force)


# ---------------------------------------------------------------------------
# status command
# ---------------------------------------------------------------------------


@cli.command()
def status() -> None:
    """Show pipeline status for genre, spatial, and cross-pollination domains."""
    from rich.console import Console
    from rich.table import Table

    from narrative_data.config import resolve_output_path
    from narrative_data.pipeline.invalidation import load_manifest

    console = Console()

    try:
        output_base = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    domains = [
        ("genre", output_base / "genres" / "manifest.json"),
        ("spatial", output_base / "spatial" / "manifest.json"),
        ("cross-pollination", output_base / "cross-pollination" / "manifest.json"),
    ]

    table = Table(title="Narrative Data Pipeline Status")
    table.add_column("Domain", style="cyan")
    table.add_column("Entries", justify="right")
    table.add_column("Elicited", justify="right")
    table.add_column("Structured", justify="right")
    table.add_column("Manifest")

    for domain_name, manifest_path in domains:
        manifest = load_manifest(manifest_path)
        entries = manifest.get("entries", {})
        total = len(entries)
        elicited = sum(1 for e in entries.values() if e.get("elicited_at"))
        structured = sum(1 for e in entries.values() if e.get("structured_at"))
        if manifest_path.exists():
            manifest_status = "[green]exists[/green]"
        else:
            manifest_status = "[dim]missing[/dim]"
        table.add_row(domain_name, str(total), str(elicited), str(structured), manifest_status)

    console.print(table)


# ---------------------------------------------------------------------------
# tome subgroup
# ---------------------------------------------------------------------------


@cli.group()
def tome() -> None:
    """Tome world-building axis and edge operations."""


@tome.command("generate-edges")
def tome_generate_edges() -> None:
    """Generate exhaustive edge combinatorics for mutual production graph."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.generate_edges import generate_all

    data_path = resolve_data_path()
    generate_all(data_path)


@tome.command("annotate-edges")
@click.option("--chunks", default=None, help="Comma-separated chunk keys to annotate")
@click.option("--force", is_flag=True, default=False, help="Re-annotate even if already done")
def tome_annotate_edges(chunks: str | None, force: bool) -> None:
    """Annotate edge files using qwen3.5:35b."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.annotate_edges import annotate_all

    data_path = resolve_data_path()
    chunk_list = _parse_list(chunks) if chunks else None
    annotate_all(data_path, chunks=chunk_list, force=force)


@tome.command("export-edges")
def tome_export_edges() -> None:
    """Export curated edge files to canonical edges.json."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.export_edges import export_all

    data_path = resolve_data_path()
    export_all(data_path)


@tome.command("stress-test")
@click.option("--count", default=20, help="Number of world sketches to generate")
@click.option("--depth", default=6, help="Maximum chain depth per sketch")
@click.option(
    "--seed-domain",
    default="material-conditions",
    help="Domain to seed chains from",
)
def tome_stress_test(count: int, depth: int, seed_domain: str) -> None:
    """Run chain generation stress test on the mutual production graph."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.chain_generator import run_stress_test

    data_path = resolve_data_path()
    run_stress_test(data_path, n_sketches=count, max_depth=depth, seed_domain=seed_domain)


@tome.command("compose-world")
@click.option("--genre", required=True, help="Genre region slug")
@click.option("--setting", required=True, help="Setting pattern slug")
@click.option(
    "--seed",
    multiple=True,
    help="Seed axis as axis-slug=value (repeatable)",
)
@click.option("--world-slug", required=True, help="World identifier")
@click.option("--enriched", is_flag=True, default=False, help="Use LLM-enriched value selection")
def tome_compose_world(
    genre: str, setting: str, seed: tuple[str, ...], world_slug: str, enriched: bool
) -> None:
    """Compose a fully-propagated world position for a genre + setting combination."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.compose_world import compose_world

    # Parse seed options: each is "axis-slug=value"
    seeds: dict[str, str] = {}
    for raw in seed:
        if "=" not in raw:
            raise click.BadParameter(
                f"Seed must be in axis-slug=value format, got: {raw!r}",
                param_hint="--seed",
            )
        axis_slug, _, value = raw.partition("=")
        seeds[axis_slug.strip()] = value.strip()

    data_path = resolve_data_path()
    try:
        compose_world(
            data_path=data_path,
            genre_slug=genre,
            setting_slug=setting,
            seeds=seeds,
            world_slug=world_slug,
            enriched=enriched,
        )
    except ValueError as exc:
        raise click.ClickException(str(exc)) from exc


@tome.command("elicit-places")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_places(world_slug: str) -> None:
    """Elicit named places for a composed world using qwen3.5:35b."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_places import elicit_places

    data_path = resolve_data_path()
    elicit_places(data_path, world_slug)


@tome.command("elicit-orgs")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_orgs(world_slug: str) -> None:
    """Elicit organizations and institutions for a composed world using qwen3.5:35b."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_orgs import elicit_orgs

    data_path = resolve_data_path()
    elicit_orgs(data_path, world_slug)


@tome.command("elicit-social-substrate")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_social_substrate(world_slug: str) -> None:
    """Elicit social substrate (lineages, factions, kinship groups) for a composed world."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_social_substrate import elicit_social_substrate

    data_path = resolve_data_path()
    elicit_social_substrate(data_path, world_slug)


@tome.command("elicit-characters-mundane")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_characters_mundane(world_slug: str) -> None:
    """Elicit mundane characters (Q1 background + Q2 community) for a composed world."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_characters_mundane import elicit_characters_mundane

    data_path = resolve_data_path()
    elicit_characters_mundane(data_path, world_slug)


@tome.command("elicit-characters-significant")
@click.option("--world-slug", required=True, help="World identifier")
def tome_elicit_characters_significant(world_slug: str) -> None:
    """Elicit significant characters (Q3 tension-bearing + Q4 scene-driving) for a composed world."""
    from narrative_data.config import resolve_data_path
    from narrative_data.tome.elicit_characters_significant import elicit_characters_significant

    data_path = resolve_data_path()
    elicit_characters_significant(data_path, world_slug)


@tome.command("elicit-decomposed")
@click.option("--world-slug", required=True, help="World slug under tome/worlds/.")
@click.option(
    "--stage",
    default=None,
    help="Run only this stage (places, orgs, substrate, characters-mundane, "
    "characters-significant).",
)
@click.option(
    "--coherence-only",
    is_flag=True,
    default=False,
    help="Skip fan-out, use existing drafts (fanout mode only).",
)
@click.option(
    "--mode",
    type=click.Choice(["compressed", "fanout"], case_sensitive=False),
    default="compressed",
    help="Pipeline mode: compressed (single 35b/stage, default) or fanout (fan-out/fan-in).",
)
def tome_elicit_decomposed(
    world_slug: str, stage: str | None, coherence_only: bool, mode: str
) -> None:
    """Run the decomposed elicitation pipeline for a world."""
    from narrative_data.config import resolve_data_path

    data_path = resolve_data_path()

    if mode == "compressed":
        from narrative_data.tome.orchestrate_decomposed import orchestrate_compressed

        orchestrate_compressed(data_path, world_slug, stage=stage)
    else:
        from narrative_data.tome.orchestrate_decomposed import orchestrate_world

        orchestrate_world(data_path, world_slug, stage=stage, coherence_only=coherence_only)


# ---------------------------------------------------------------------------
# list subgroup
# ---------------------------------------------------------------------------


@cli.group("list")
def list_cmd() -> None:
    """List and query elicited/structured data."""


def _list_domain_data(
    domain_dir: Path,
    target: str | None,
    category: str | None,
    fmt: str,
) -> None:
    """Print structured data files from a domain directory as JSON or a table."""
    import json

    from rich.console import Console
    from rich.table import Table

    console = Console()

    if not domain_dir.exists():
        if fmt == "json":
            click.echo("[]")
        else:
            console.print("[dim]No data found.[/dim]")
        return

    results: list[dict] = []

    # Collect all matching .json files (not manifests, not raw.md, not .errors.json)
    for json_path in sorted(domain_dir.rglob("*.json")):
        if json_path.name in ("manifest.json",):
            continue
        if json_path.name.endswith(".errors.json"):
            continue

        # Filter by target (region/setting slug — parent directory name)
        slug = json_path.parent.name
        if target and slug != target:
            continue

        # Filter by category (stem of filename, e.g. "archetypes")
        cat = json_path.stem
        if category and cat != category:
            continue

        try:
            data = json.loads(json_path.read_text())
        except (json.JSONDecodeError, OSError):
            continue

        results.append({"slug": slug, "category": cat, "path": str(json_path), "data": data})

    if fmt == "json":
        click.echo(json.dumps(results, indent=2))
    else:
        table = Table()
        table.add_column("Slug", style="cyan")
        table.add_column("Category")
        table.add_column("Path", style="dim")
        for item in results:
            table.add_row(item["slug"], item["category"], item["path"])
        console.print(table)


@list_cmd.command("genres")
@click.option("--region", default=None, help="Filter by region slug.")
@click.option("--category", default=None, help="Filter by category name.")
@click.option(
    "--format",
    "fmt",
    default="json",
    type=click.Choice(["json", "table"]),
    help="Output format (default: json).",
)
def list_genres(region: str | None, category: str | None, fmt: str) -> None:
    """List structured genre data files."""
    from narrative_data.config import resolve_output_path

    output_base = resolve_output_path()
    _list_domain_data(output_base / "genres", region, category, fmt)


@list_cmd.command("spatial")
@click.option("--setting", default=None, help="Filter by setting slug.")
@click.option("--category", default=None, help="Filter by category name.")
@click.option(
    "--format",
    "fmt",
    default="json",
    type=click.Choice(["json", "table"]),
    help="Output format (default: json).",
)
def list_spatial(setting: str | None, category: str | None, fmt: str) -> None:
    """List structured spatial data files."""
    from narrative_data.config import resolve_output_path

    output_base = resolve_output_path()
    _list_domain_data(output_base / "spatial", setting, category, fmt)


@list_cmd.command("intersections")
@click.option("--stale", is_flag=True, default=False, help="Show only stale intersections.")
@click.option(
    "--format",
    "fmt",
    default="json",
    type=click.Choice(["json", "table"]),
    help="Output format (default: json).",
)
def list_intersections(stale: bool, fmt: str) -> None:
    """List cross-pollination intersection data."""
    import json

    from rich.console import Console
    from rich.table import Table

    from narrative_data.config import resolve_output_path
    from narrative_data.pipeline.invalidation import load_manifest

    console = Console()

    try:
        output_base = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    manifest_path = output_base / "cross-pollination" / "manifest.json"
    manifest = load_manifest(manifest_path)
    entries = manifest.get("entries", {})

    results = []
    for key, entry in entries.items():
        if stale and entry.get("structured_at"):
            continue
        results.append({"key": key, **entry})

    if fmt == "json":
        click.echo(json.dumps(results, indent=2))
    else:
        table = Table()
        table.add_column("Key", style="cyan")
        table.add_column("Elicited")
        table.add_column("Structured")
        for item in results:
            table.add_row(
                item["key"],
                item.get("elicited_at", "—"),
                item.get("structured_at", "—"),
            )
        console.print(table)


# ---------------------------------------------------------------------------
# discover subgroup
# ---------------------------------------------------------------------------


@cli.group()
def discover() -> None:
    """Primitive discovery: extract from genres and synthesize across clusters."""


@discover.command("extract")
@click.option(
    "--type",
    "primitive_type",
    required=True,
    help="Primitive type (archetypes, dynamics, goals, profiles, settings).",
)
@click.option("--genres", default=None, help="Comma-separated genre slugs (default: all).")
@click.option("--force", is_flag=True, default=False, help="Re-extract even if up to date.")
def discover_extract(primitive_type: str, genres: str | None, force: bool) -> None:
    """Phase 1: Extract primitive candidates from each genre's region description."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.discovery.commands import extract_primitives
    from narrative_data.genre.commands import GENRE_REGIONS
    from narrative_data.ollama import OllamaClient

    output_base = resolve_output_path()
    log_path = output_base / "pipeline.jsonl"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    genre_list = _parse_list(genres) if genres is not None else GENRE_REGIONS
    extract_primitives(
        client=client,
        output_base=output_base,
        log_path=log_path,
        primitive_type=primitive_type,
        genres=genre_list,
        force=force,
    )


@discover.command("synthesize")
@click.option("--type", "primitive_type", required=True, help="Primitive type.")
@click.option(
    "--cluster",
    "cluster_name",
    required=True,
    help="Genre cluster name (horror, fantasy, sci-fi, mystery-thriller, "
    "romance, realism-gothic-other).",
)
@click.option("--force", is_flag=True, default=False, help="Re-synthesize even if exists.")
def discover_synthesize(primitive_type: str, cluster_name: str, force: bool) -> None:
    """Phase 2: Synthesize per-genre extractions into deduplicated cluster list."""
    from narrative_data.config import GENRE_CLUSTERS, OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.discovery.commands import synthesize_cluster
    from narrative_data.ollama import OllamaClient

    output_base = resolve_output_path()
    log_path = output_base / "pipeline.jsonl"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    genres = GENRE_CLUSTERS.get(cluster_name, [])
    if not genres:
        click.echo(f"Unknown cluster: {cluster_name}", err=True)
        raise SystemExit(1)
    synthesize_cluster(
        client=client,
        output_base=output_base,
        log_path=log_path,
        primitive_type=primitive_type,
        cluster_name=cluster_name,
        genres=genres,
        force=force,
    )


# ---------------------------------------------------------------------------
# primitive subgroup
# ---------------------------------------------------------------------------


@cli.group()
def primitive() -> None:
    """Standalone primitive elicitation (Layer 0)."""


@primitive.command("elicit")
@click.option("--type", "primitive_type", required=True, help="Primitive type.")
@click.option("--primitives", default=None, help="Comma-separated primitive slugs.")
@click.option("--force", is_flag=True, default=False, help="Re-elicit even if up to date.")
def primitive_elicit(primitive_type: str, primitives: str | None, force: bool) -> None:
    """Phase 3: Elicit standalone Layer 0 descriptions for primitives."""
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.ollama import OllamaClient
    from narrative_data.primitive.commands import elicit_primitives

    output_base = resolve_output_path()
    log_path = output_base / "pipeline.jsonl"
    client = OllamaClient(base_url=OLLAMA_BASE_URL)
    prim_list = _parse_list(primitives) or []
    elicit_primitives(
        client=client,
        output_base=output_base,
        log_path=log_path,
        primitive_type=primitive_type,
        primitives=prim_list,
        descriptions={},
        force=force,
    )


# ---------------------------------------------------------------------------
# pipeline subgroup
# ---------------------------------------------------------------------------


@cli.group()
def pipeline() -> None:
    """Pipeline control plane: status, resume, and review gates."""


@pipeline.command("status")
@click.option(
    "--type",
    "primitive_type",
    default=None,
    help="Show status for specific primitive type.",
)
def pipeline_status(primitive_type: str | None) -> None:
    """Show pipeline progress for primitive discovery and elicitation."""
    from rich.console import Console

    from narrative_data.config import GENRE_CLUSTERS, PRIMITIVE_TYPES, resolve_output_path
    from narrative_data.genre.commands import GENRE_REGIONS
    from narrative_data.pipeline.events import derive_state

    console = Console()
    try:
        output_base = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    log_path = output_base / "pipeline.jsonl"
    types = [primitive_type] if primitive_type else PRIMITIVE_TYPES

    for pt in types:
        state = derive_state(log_path, pt)
        n_genres = len(GENRE_REGIONS)
        n_clusters = len(GENRE_CLUSTERS)
        p1 = len(state["phase1_completed"])
        p2 = len(state["phase2_completed"])
        p3 = len(state["phase3_completed"])
        p4 = len(state["phase4_completed"])
        gate = state["phase2_gate"]

        console.print(f"\n[bold]Pipeline Status: {pt}[/bold]")
        console.print(f"  Phase 1 (extract):     {p1}/{n_genres} genres complete")
        console.print(f"  Phase 2 (synthesize):  {p2}/{n_clusters} clusters complete")
        if p2 == n_clusters and gate is None:
            console.print(
                "  Phase 3 (elicit):      [yellow]blocked — awaiting review gate[/yellow]"
            )
        elif gate:
            n_prims = len(gate["primitives"])
            console.print(f"  Phase 3 (elicit):      {p3}/{n_prims} primitives complete")
        else:
            console.print("  Phase 3 (elicit):      [dim]blocked — awaiting Phase 2[/dim]")
        console.print(f"  Phase 4 (elaborate):   {p4} pairs complete")


# ---------------------------------------------------------------------------
# load-bedrock command
# ---------------------------------------------------------------------------


@cli.command("load-bedrock")
@click.option(
    "--dry-run",
    is_flag=True,
    default=False,
    help="Validate and report without writing to the database.",
)
@click.option(
    "--type",
    "type_filter",
    default=None,
    help="Comma-separated list of primitive types to load (default: all).",
)
@click.option(
    "--genre",
    "genre_filter",
    default=None,
    help="Comma-separated list of genre slugs to load (default: all).",
)
@click.option(
    "--skip-prune",
    is_flag=True,
    default=True,
    help="Skip pruning of rows absent from current corpus (default: True).",
)
@click.option(
    "--refs-only",
    is_flag=True,
    default=False,
    help="Load only reference entities (genres, clusters, state variables, dimensions).",
)
def load_bedrock_cmd(
    dry_run: bool,
    type_filter: str | None,
    genre_filter: str | None,
    skip_prune: bool,
    refs_only: bool,
) -> None:
    """Load narrative corpus into bedrock PostgreSQL tables.

    Phase 1 loads reference entities (genres, clusters, state variables, dimensions).
    Phase 2 loads primitive types from the corpus into type-specific tables.

    Requires DATABASE_URL and STORYTELLER_DATA_PATH environment variables.
    """
    import psycopg
    from rich.console import Console

    from narrative_data.config import resolve_output_path
    from narrative_data.persistence.connection import get_connection_string
    from narrative_data.persistence.loader import load_bedrock

    console = Console()

    try:
        corpus_dir = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    try:
        dsn = get_connection_string()
    except ValueError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    types = _parse_list(type_filter)
    genres = _parse_list(genre_filter)

    if dry_run:
        console.print("[yellow]Dry-run mode — no database writes will occur.[/yellow]")

    try:
        with psycopg.connect(dsn) as conn:
            report = load_bedrock(
                conn=conn,
                corpus_dir=corpus_dir,
                types=types,
                genre_filter=genres,
                refs_only=refs_only,
                skip_prune=skip_prune,
                dry_run=dry_run,
            )
    except Exception as exc:
        console.print(f"[red]Load failed: {exc}[/red]")
        raise SystemExit(1) from exc

    console.print("[green]Load complete.[/green]")
    console.print(f"  inserted : {report.inserted}")
    console.print(f"  updated  : {report.updated}")
    console.print(f"  pruned   : {report.pruned}")
    console.print(f"  skipped  : {report.skipped}")
    console.print(f"  errors   : {report.errors}")
    if report.error_details:
        console.print("[red]Errors:[/red]")
        for detail in report.error_details[:20]:
            console.print(f"  - {detail}")
        if len(report.error_details) > 20:
            console.print(f"  ... and {len(report.error_details) - 20} more")


@pipeline.command("approve")
@click.option("--type", "primitive_type", required=True, help="Primitive type being approved.")
@click.option("--phase", required=True, type=int, help="Phase number (2).")
@click.option(
    "--primitives", required=True, help="Comma-separated list of approved primitive slugs."
)
@click.option("--note", default=None, help="Optional note about the review decision.")
def pipeline_approve(primitive_type: str, phase: int, primitives: str, note: str | None) -> None:
    """Record a human review gate decision."""
    from narrative_data.config import resolve_output_path
    from narrative_data.pipeline.events import append_event

    output_base = resolve_output_path()
    log_path = output_base / "pipeline.jsonl"
    prim_list = _parse_list(primitives) or []
    kwargs: dict = {"decision": "approved", "primitives": prim_list}
    if note:
        kwargs["note"] = note
    append_event(log_path, event="review_gate", phase=phase, type=primitive_type, **kwargs)
    click.echo(
        f"Recorded review gate: {primitive_type} phase {phase} "
        f"approved ({len(prim_list)} primitives)"
    )


# ---------------------------------------------------------------------------
# structure subgroup (Stage 2 structuring)
# ---------------------------------------------------------------------------


@cli.group()
def structure() -> None:
    """Stage 2: Structure raw markdown into validated JSON."""


# ---------------------------------------------------------------------------
# audit command
# ---------------------------------------------------------------------------


@cli.command("audit")
@click.option(
    "--type",
    "types",
    multiple=True,
    help="Type slug(s) to audit (repeatable). Defaults to all known types.",
)
@click.option(
    "--genre",
    "genres",
    multiple=True,
    help="Genre slug(s) to include (repeatable). Defaults to all found.",
)
@click.option(
    "--output",
    "output_path",
    default=None,
    type=click.Path(),
    help="Save JSON report to this path.",
)
@click.option(
    "--threshold-warn",
    default=0.5,
    show_default=True,
    type=float,
    help="Null rate >= this value is shown in yellow.",
)
@click.option(
    "--threshold-error",
    default=0.8,
    show_default=True,
    type=float,
    help="Null rate >= this value is shown in red.",
)
def audit(
    types: tuple[str, ...],
    genres: tuple[str, ...],
    output_path: str | None,
    threshold_warn: float,
    threshold_error: float,
) -> None:
    """Scan the corpus for null/empty fields and report coverage rates.

    Outputs a Rich table per type with color-coded field rates:
    green = well-filled, yellow = sparse, red = mostly empty.
    """
    import json as _json

    from rich.console import Console
    from rich.table import Table

    from narrative_data.config import resolve_output_path
    from narrative_data.pipeline.postprocess import AuditResult, audit_corpus

    console = Console()

    try:
        corpus_dir = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    type_list = list(types) if types else None
    genre_list = list(genres) if genres else None

    console.print(f"\n[bold]Auditing corpus at:[/bold] {corpus_dir}")
    if type_list:
        console.print(f"  Types:  {', '.join(type_list)}")
    if genre_list:
        console.print(f"  Genres: {', '.join(genre_list)}")
    console.print()

    results: dict[str, AuditResult] = audit_corpus(
        corpus_dir=corpus_dir,
        types=type_list,
        genres=genre_list,
    )

    for type_slug, result in sorted(results.items()):
        title = (
            f"{type_slug}  [dim]({result.total_entities} entities, {result.file_count} files)[/dim]"
        )
        table = Table(title=title, show_header=True, header_style="bold cyan")
        table.add_column("Field", style="cyan", no_wrap=True)
        table.add_column("Null Rate", justify="right")
        table.add_column("Status", justify="center")

        if result.errors and not result.field_rates:
            for err in result.errors:
                table.add_row("[dim]—[/dim]", "[dim]—[/dim]", f"[red]{err}[/red]")
        else:
            for field_name, rate in sorted(result.field_rates.items()):
                pct = f"{rate * 100:.1f}%"
                if rate >= threshold_error:
                    status = "[red]sparse[/red]"
                    rate_str = f"[red]{pct}[/red]"
                elif rate >= threshold_warn:
                    status = "[yellow]partial[/yellow]"
                    rate_str = f"[yellow]{pct}[/yellow]"
                else:
                    status = "[green]ok[/green]"
                    rate_str = f"[green]{pct}[/green]"
                table.add_row(field_name, rate_str, status)

            if result.errors:
                console.print(f"  [yellow]Warnings:[/yellow] {'; '.join(result.errors)}")

        console.print(table)
        console.print()

    if output_path:
        report = {
            type_slug: {
                "type_name": r.type_name,
                "genre": r.genre,
                "total_entities": r.total_entities,
                "file_count": r.file_count,
                "field_rates": r.field_rates,
                "errors": r.errors,
            }
            for type_slug, r in results.items()
        }
        Path(output_path).write_text(_json.dumps(report, indent=2))
        console.print(f"[dim]JSON report saved to {output_path}[/dim]")


# ---------------------------------------------------------------------------
# sv-audit command
# ---------------------------------------------------------------------------


@cli.command("sv-audit")
def sv_audit() -> None:
    """Audit state variable references against canonical set and report resolution rates."""
    from rich.console import Console
    from rich.table import Table

    from narrative_data.config import resolve_output_path
    from narrative_data.persistence.reference_data import extract_state_variables
    from narrative_data.persistence.sv_normalization import audit_sv_resolution

    console = Console()
    try:
        corpus_dir = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    # Build canonical set from region.json files
    sv_list = extract_state_variables(corpus_dir)
    canonical = {sv["slug"] for sv in sv_list}
    console.print(f"Canonical state variables: {len(canonical)}")

    results = audit_sv_resolution(corpus_dir, canonical)

    exact_n = len(results["exact"])
    prefix_n = len(results["prefix"])
    unresolved_n = len(results["unresolved"])
    total = exact_n + prefix_n + unresolved_n

    if total:
        console.print(f"\nTotal references scanned: {total}")
        console.print(f"  [green]Exact match:[/green]  {exact_n} ({100 * exact_n / total:.1f}%)")
        console.print(
            f"  [yellow]Prefix match:[/yellow] {prefix_n} ({100 * prefix_n / total:.1f}%)"
        )
        console.print(
            f"  [red]Unresolved:[/red]   {unresolved_n} ({100 * unresolved_n / total:.1f}%)"
        )
    else:
        console.print("\nNo state variable references found.")

    if results["unresolved"]:
        console.print(f"\n[bold]Unresolved references ({unresolved_n}):[/bold]")
        table = Table(show_header=True, header_style="bold")
        table.add_column("Raw", style="red")
        table.add_column("Normalized")
        table.add_column("Type")
        table.add_column("Genre")
        table.add_column("Entity")
        seen: set[tuple[str, str, str]] = set()
        for ref in sorted(results["unresolved"], key=lambda r: r["normalized"]):
            key = (ref["normalized"], ref["type"], ref["genre"])
            if key in seen:
                continue
            seen.add(key)
            table.add_row(ref["raw"], ref["normalized"], ref["type"], ref["genre"], ref["entity"])
        console.print(table)

    if results["prefix"]:
        console.print(f"\n[bold]Prefix matches ({prefix_n}):[/bold]")
        table = Table(show_header=True, header_style="bold")
        table.add_column("Raw", style="yellow")
        table.add_column("Resolved")
        seen_prefix: set[str] = set()
        for ref in sorted(results["prefix"], key=lambda r: r["normalized"]):
            key = ref["normalized"]
            if key in seen_prefix:
                continue
            seen_prefix.add(key)
            table.add_row(ref["raw"], ref["resolved"])
        console.print(table)


@structure.command("run")
@click.argument("type_slug")
@click.option("--genre", default=None, help="Single genre slug to structure.")
@click.option("--all", "all_genres", is_flag=True, default=False, help="Structure all genres.")
@click.option("--clusters", is_flag=True, default=False, help="Structure cluster synthesis files.")
@click.option("--force", is_flag=True, default=False, help="Re-structure even if cached.")
@click.option(
    "--plan", "plan_only", is_flag=True, default=False, help="Show plan without executing."
)
@click.option(
    "--model", default=None, help="Override structuring model (default: qwen2.5:7b-instruct)."
)
def structure_run(
    type_slug: str,
    genre: str | None,
    all_genres: bool,
    clusters: bool,
    force: bool,
    plan_only: bool,
    model: str | None,
) -> None:
    """Structure raw markdown into validated JSON for a given type.

    TYPE_SLUG is one of: genre-dimensions, archetypes, dynamics, goals, profiles,
    settings, ontological-posture, archetype-dynamics, spatial-topology, place-entities,
    tropes, narrative-shapes
    """
    from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
    from narrative_data.ollama import OllamaClient
    from narrative_data.pipeline.structure_commands import (
        TYPE_REGISTRY,
    )
    from narrative_data.pipeline.structure_commands import (
        structure_clusters as do_structure_clusters,
    )
    from narrative_data.pipeline.structure_commands import (
        structure_type as do_structure_type,
    )

    if type_slug not in TYPE_REGISTRY:
        click.echo(
            f"Unknown type: {type_slug}. Valid types: {', '.join(TYPE_REGISTRY.keys())}",
            err=True,
        )
        raise SystemExit(1)

    output_base = resolve_output_path()
    client = OllamaClient(base_url=OLLAMA_BASE_URL)

    if all_genres or genre:
        genres = [genre] if genre else None  # None = all
        do_structure_type(
            client=client,
            output_base=output_base,
            type_slug=type_slug,
            genres=genres,
            force=force,
            plan_only=plan_only,
            model=model,
        )

    if clusters:
        do_structure_clusters(
            client=client,
            output_base=output_base,
            type_slug=type_slug,
            force=force,
            plan_only=plan_only,
            model=model,
        )

    if not all_genres and not genre and not clusters:
        click.echo("Specify --all, --genre <slug>, or --clusters", err=True)
        raise SystemExit(1)


# ---------------------------------------------------------------------------
# fill command
# ---------------------------------------------------------------------------


# ---------------------------------------------------------------------------
# migrate command
# ---------------------------------------------------------------------------


@cli.command("migrate")
@click.option("--dry-run", is_flag=True, default=False, help="Show SQL without executing.")
def migrate_cmd(dry_run: bool) -> None:
    """Apply ground-state migrations via direct SQL execution.

    Reads migration files from crates/storyteller-storykeeper/migrations/ that
    begin with '20260323' (the ground-state schema date prefix) and applies
    them in filename order using a direct psycopg connection.

    Note: These migrations are co-located with sqlx migrations in
    crates/storyteller-storykeeper/migrations/ but can be applied
    independently via this command for Python-only workflows. In production,
    'sqlx migrate run' handles everything.

    Requires DATABASE_URL to be set, e.g.:
      postgres://storyteller:storyteller@localhost:5435/storyteller_development
    """
    import psycopg
    from rich.console import Console

    from narrative_data.persistence.connection import get_connection_string

    console = Console()

    # Locate the migrations directory relative to this file's repo root.
    # tools/narrative-data/src/narrative_data/cli.py → up 5 levels → repo root
    repo_root = Path(__file__).parent.parent.parent.parent.parent.parent
    migrations_dir = repo_root / "crates" / "storyteller-storykeeper" / "migrations"

    if not migrations_dir.exists():
        console.print(f"[red]Migrations directory not found: {migrations_dir}[/red]")
        raise SystemExit(1)

    # Collect ground-state migration files (date prefix 20260323)
    migration_files = sorted(f for f in migrations_dir.glob("20260323*.sql") if f.is_file())

    if not migration_files:
        console.print(f"[yellow]No ground-state migration files found in {migrations_dir}[/yellow]")
        return

    console.print(f"\n[bold]Ground-state migrations:[/bold] {migrations_dir}")
    console.print(f"  Found {len(migration_files)} file(s):\n")
    for f in migration_files:
        console.print(f"    [cyan]{f.name}[/cyan]")
    console.print()

    if dry_run:
        console.print("[yellow]Dry run — SQL will be printed but not executed.[/yellow]\n")
        for migration_path in migration_files:
            sql = migration_path.read_text()
            console.print(f"[bold]-- {migration_path.name}[/bold]")
            console.print(sql)
            console.print()
        return

    try:
        conn_str = get_connection_string()
    except ValueError as exc:
        console.print(f"[red]{exc}[/red]")
        raise SystemExit(1) from exc

    try:
        with psycopg.connect(conn_str) as conn:
            # Check whether bedrock schema already exists
            with conn.cursor() as cur:
                cur.execute(
                    "SELECT schema_name FROM information_schema.schemata "
                    "WHERE schema_name = 'bedrock'"
                )
                schema_exists = cur.fetchone() is not None

            if schema_exists:
                console.print(
                    "[yellow]Schema 'bedrock' already exists — "
                    "migrations are idempotent via CREATE IF NOT EXISTS, "
                    "but duplicate table creation will raise errors.[/yellow]"
                )
                console.print(
                    "  To re-run: drop the schema first with "
                    "[dim]DROP SCHEMA bedrock CASCADE;[/dim]\n"
                )

            for migration_path in migration_files:
                sql = migration_path.read_text()
                console.print(f"  Applying [cyan]{migration_path.name}[/cyan] ...", end=" ")
                with conn.cursor() as cur:
                    cur.execute(sql)
                conn.commit()
                console.print("[green]done[/green]")

    except psycopg.Error as exc:
        console.print(f"\n[red]Database error: {exc}[/red]")
        raise SystemExit(1) from exc

    console.print("\n[green]All ground-state migrations applied.[/green]")


@cli.command("fill")
@click.option(
    "--tier",
    type=click.Choice(["deterministic", "llm-patch"]),
    required=True,
    help=(
        "Fill tier: 'deterministic' uses regex/lookup rules; "
        "'llm-patch' uses an LLM for targeted field extraction."
    ),
)
@click.option(
    "--type",
    "types",
    multiple=True,
    help="Type slug(s) to fill (repeatable). Defaults to all supported fill types.",
)
@click.option(
    "--genre",
    "genres",
    multiple=True,
    help="Genre slug(s) to include (repeatable). Defaults to all found.",
)
@click.option(
    "--dry-run",
    is_flag=True,
    default=False,
    help="Report changes without writing any files.",
)
def fill(
    tier: str,
    types: tuple[str, ...],
    genres: tuple[str, ...],
    dry_run: bool,
) -> None:
    """Fill null/empty fields in structured JSON files.

    For --tier deterministic: applies rule-based fills (regex patterns,
    lookup tables) that require no LLM.  Currently supports:

    \b
      dynamics        → fills spans_scales from source markdown Scale: lines
      spatial-topology → fills agency from friction.type + directionality.type
    """
    if tier == "llm-patch":
        from rich.console import Console
        from rich.table import Table

        from narrative_data.config import OLLAMA_BASE_URL, resolve_output_path
        from narrative_data.ollama import OllamaClient
        from narrative_data.pipeline.llm_patch import fill_all_llm_patch

        llm_console = Console()

        try:
            llm_corpus_dir = resolve_output_path()
        except RuntimeError as exc:
            llm_console.print(f"[red]Error: {exc}[/red]")
            raise SystemExit(1) from exc

        llm_type_list = list(types) if types else None
        llm_genre_list = list(genres) if genres else None
        llm_client = OllamaClient(base_url=OLLAMA_BASE_URL)

        if dry_run:
            llm_console.print("[yellow]Dry run — no files will be written.[/yellow]")
        llm_console.print(f"\n[bold]Running LLM patch fills on:[/bold] {llm_corpus_dir}")
        if llm_type_list:
            llm_console.print(f"  Types:  {', '.join(llm_type_list)}")
        if llm_genre_list:
            llm_console.print(f"  Genres: {', '.join(llm_genre_list)}")
        llm_console.print()

        llm_summary = fill_all_llm_patch(
            corpus_dir=llm_corpus_dir,
            client=llm_client,
            types=llm_type_list,
            genres=llm_genre_list,
            dry_run=dry_run,
        )

        llm_table = Table(
            title="LLM Patch Fill Summary", show_header=True, header_style="bold cyan"
        )
        llm_table.add_column("Type", style="cyan")
        llm_table.add_column("Files", justify="right")
        llm_table.add_column("Updated", justify="right")
        llm_table.add_column("Skipped", justify="right")

        for type_slug, result in sorted(llm_summary.items()):
            updated = result["entities_updated"]
            updated_str = f"[green]{updated}[/green]" if updated > 0 else str(updated)
            llm_table.add_row(
                type_slug,
                str(result["files_processed"]),
                updated_str,
                str(result["entities_skipped"]),
            )

        llm_console.print(llm_table)
        if dry_run:
            llm_console.print("\n[yellow]Dry run complete — no files written.[/yellow]")
        return

    from rich.console import Console
    from rich.table import Table

    from narrative_data.config import resolve_output_path
    from narrative_data.pipeline.postprocess import fill_all_deterministic

    console = Console()

    try:
        corpus_dir = resolve_output_path()
    except RuntimeError as exc:
        console.print(f"[red]Error: {exc}[/red]")
        raise SystemExit(1) from exc

    type_list = list(types) if types else None
    genre_list = list(genres) if genres else None

    if dry_run:
        console.print("[yellow]Dry run — no files will be written.[/yellow]")
    console.print(f"\n[bold]Running deterministic fills on:[/bold] {corpus_dir}")
    if type_list:
        console.print(f"  Types:  {', '.join(type_list)}")
    if genre_list:
        console.print(f"  Genres: {', '.join(genre_list)}")
    console.print()

    summary = fill_all_deterministic(
        corpus_dir=corpus_dir,
        types=type_list,
        genres=genre_list,
        dry_run=dry_run,
    )

    table = Table(title="Fill Summary", show_header=True, header_style="bold cyan")
    table.add_column("Type", style="cyan")
    table.add_column("Files", justify="right")
    table.add_column("Updated", justify="right")
    table.add_column("Skipped", justify="right")

    for type_slug, result in sorted(summary.items()):
        updated = result["entities_updated"]
        updated_str = f"[green]{updated}[/green]" if updated > 0 else str(updated)
        table.add_row(
            type_slug,
            str(result["files_processed"]),
            updated_str,
            str(result["entities_skipped"]),
        )

    console.print(table)
    if dry_run:
        console.print("\n[yellow]Dry run complete — no files written.[/yellow]")
