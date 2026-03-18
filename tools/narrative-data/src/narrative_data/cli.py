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
