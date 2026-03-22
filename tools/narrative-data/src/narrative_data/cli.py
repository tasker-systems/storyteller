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


@structure.command("run")
@click.argument("type_slug")
@click.option("--genre", default=None, help="Single genre slug to structure.")
@click.option("--all", "all_genres", is_flag=True, default=False, help="Structure all genres.")
@click.option("--clusters", is_flag=True, default=False, help="Structure cluster synthesis files.")
@click.option("--force", is_flag=True, default=False, help="Re-structure even if cached.")
@click.option(
    "--plan", "plan_only", is_flag=True, default=False, help="Show plan without executing."
)
def structure_run(
    type_slug: str,
    genre: str | None,
    all_genres: bool,
    clusters: bool,
    force: bool,
    plan_only: bool,
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
        )

    if clusters:
        do_structure_clusters(
            client=client,
            output_base=output_base,
            type_slug=type_slug,
            force=force,
            plan_only=plan_only,
        )

    if not all_genres and not genre and not clusters:
        click.echo("Specify --all, --genre <slug>, or --clusters", err=True)
        raise SystemExit(1)
