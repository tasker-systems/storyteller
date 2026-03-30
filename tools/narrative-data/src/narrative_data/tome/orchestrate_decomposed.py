# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Pipeline orchestrator for the Tome decomposed elicitation pipeline.

Coordinates the full decomposed pipeline:
  compress preamble -> plan entities -> [fan-out -> cohere] per stage -> compose world.json

Stages run in order: places -> orgs -> substrate -> characters-mundane -> characters-significant.
Each stage builds FanOutSpec lists from the entity plan, dispatches fan-out calls,
aggregates drafts, runs coherence, and saves per-stage output. The final step
composes a unified world.json from all per-stage files.

Two modes are available:
  - **fanout** (original): fan-out/fan-in with model tiering (7b + 35b)
  - **compressed** (default): compressed preamble + single 35b call per stage,
    using the existing Phase 3b elicitation prompts.  After each stage completes
    the response is parsed into per-entity instance files under ``decomposed/fan-out/``.

Usage (called by CLI command):
    orchestrate_world(data_path, world_slug)
    orchestrate_world(data_path, world_slug, stage="orgs")
    orchestrate_world(data_path, world_slug, coherence_only=True)
    orchestrate_compressed(data_path, world_slug)
    orchestrate_compressed(data_path, world_slug, stage="places")
"""

from __future__ import annotations

import contextlib
import json
import logging
from itertools import combinations
from pathlib import Path
from typing import Any

from narrative_data.tome.cohere import (
    _STAGE_FILENAME,
    cohere,
    save_coherence_output,
)
from narrative_data.tome.compress_preamble import build_world_summary, subset_axes
from narrative_data.tome.fan_out import aggregate, fan_out, save_instances
from narrative_data.tome.models import FanOutSpec
from narrative_data.utils import now_iso

_log = logging.getLogger(__name__)

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"

_STAGES = [
    "places",
    "orgs",
    "substrate",
    "characters-mundane",
    "characters-significant",
]

_DOMAIN_SUBSETS: dict[str, list[str]] = {
    "places": ["Material Conditions", "Aesthetic and Cultural Forms"],
    "orgs": [
        "Economic Forms",
        "Political Structures",
        "Social Forms of Production and Reproduction",
    ],
    "substrate": [
        "Social Forms of Production and Reproduction",
        "Material Conditions",
    ],
    "characters-mundane": ["Social Forms of Production and Reproduction"],
    "characters-significant": [
        "Social Forms of Production and Reproduction",
        "Material Conditions",
    ],
}


# ---------------------------------------------------------------------------
# Directory setup
# ---------------------------------------------------------------------------


def _ensure_decomposed_dir(data_path: Path, world_slug: str) -> Path:
    """Create the decomposed/ directory with fan-out subdirectories for all 5 stages.

    Args:
        data_path: Root of the storyteller-data checkout.
        world_slug: World identifier.

    Returns:
        Path to the decomposed directory.
    """
    decomposed = data_path / "narrative-data" / "tome" / "worlds" / world_slug / "decomposed"
    decomposed.mkdir(parents=True, exist_ok=True)

    fan_out_dir = decomposed / "fan-out"
    fan_out_dir.mkdir(exist_ok=True)

    for stage in _STAGES:
        (fan_out_dir / stage).mkdir(exist_ok=True)

    return decomposed


# ---------------------------------------------------------------------------
# World summary
# ---------------------------------------------------------------------------


def _build_world_summary_from_path(data_path: Path, world_slug: str) -> dict[str, Any]:
    """Load world-position.json and build a compressed summary.

    Args:
        data_path: Root of the storyteller-data checkout.
        world_slug: World identifier.

    Returns:
        Dict with keys: genre_slug, setting_slug, compressed_preamble,
        axis_count, seed_count, genre_profile.
    """
    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    domains_dir = data_path / "narrative-data" / "domains"

    wp_path = world_dir / "world-position.json"
    world_pos = json.loads(wp_path.read_text())

    summary = build_world_summary(world_pos, domains_dir)
    summary["genre_profile"] = world_pos.get("genre_profile")

    return summary


# ---------------------------------------------------------------------------
# Spec builders
# ---------------------------------------------------------------------------


def _build_place_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
) -> list[FanOutSpec]:
    """Create FanOutSpec list for the places stage.

    Args:
        plan: Entity plan dict with a "places" key containing "distribution".
        compressed_preamble: Full compressed preamble string.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.

    Returns:
        List of FanOutSpec, one per place in the distribution.
    """
    axes_subset = subset_axes(compressed_preamble, _DOMAIN_SUBSETS["places"])
    distribution = plan["places"]["distribution"]

    specs: list[FanOutSpec] = []
    index = 0
    for place_type, count in distribution.items():
        for _ in range(int(count)):
            specs.append(
                FanOutSpec(
                    stage="places",
                    index=index,
                    template_name="place-fanout.md",
                    model_role="fan_out_structured",
                    context={
                        "genre_slug": genre_slug,
                        "setting_slug": setting_slug,
                        "place_type": place_type,
                        "axes_subset": axes_subset,
                        "spatial_function": "",
                    },
                )
            )
            index += 1

    return specs


def _build_org_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    places_summary: str,
) -> list[FanOutSpec]:
    """Create FanOutSpec list for the organizations stage.

    Args:
        plan: Entity plan dict with an "organizations" key.
        compressed_preamble: Full compressed preamble string.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.
        places_summary: Markdown summary of places for upstream context.

    Returns:
        List of FanOutSpec, one per organization.
    """
    axes_subset = subset_axes(compressed_preamble, _DOMAIN_SUBSETS["orgs"])
    orgs = plan["organizations"]
    count = orgs.get("count", 0) if isinstance(orgs, dict) else int(orgs)

    # If distribution exists, iterate it; otherwise generate count specs
    distribution = orgs.get("distribution", {}) if isinstance(orgs, dict) else {}

    specs: list[FanOutSpec] = []
    index = 0

    if distribution:
        for org_type, type_count in distribution.items():
            for _ in range(int(type_count)):
                specs.append(
                    FanOutSpec(
                        stage="orgs",
                        index=index,
                        template_name="org-fanout.md",
                        model_role="fan_out_structured",
                        context={
                            "genre_slug": genre_slug,
                            "setting_slug": setting_slug,
                            "org_type": org_type,
                            "axes_subset": axes_subset,
                            "places_summary": places_summary,
                        },
                    )
                )
                index += 1
    else:
        for i in range(count):
            specs.append(
                FanOutSpec(
                    stage="orgs",
                    index=i,
                    template_name="org-fanout.md",
                    model_role="fan_out_structured",
                    context={
                        "genre_slug": genre_slug,
                        "setting_slug": setting_slug,
                        "org_type": "",
                        "axes_subset": axes_subset,
                        "places_summary": places_summary,
                    },
                )
            )

    return specs


def _build_substrate_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    upstream_summary: str,
) -> list[FanOutSpec]:
    """Create FanOutSpec list for the social substrate stage.

    Args:
        plan: Entity plan dict with a "clusters" key.
        compressed_preamble: Full compressed preamble string.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.
        upstream_summary: Combined places + orgs markdown summary.

    Returns:
        List of FanOutSpec, one per cluster.
    """
    axes_subset = subset_axes(compressed_preamble, _DOMAIN_SUBSETS["substrate"])
    clusters = plan["clusters"]
    count = clusters.get("count", 0) if isinstance(clusters, dict) else int(clusters)
    cluster_basis = clusters.get("basis", "") if isinstance(clusters, dict) else ""

    specs: list[FanOutSpec] = []
    for i in range(count):
        specs.append(
            FanOutSpec(
                stage="substrate",
                index=i,
                template_name="substrate-fanout.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "axes_subset": axes_subset,
                    "upstream_summary": upstream_summary,
                    "cluster_basis": cluster_basis,
                },
            )
        )

    return specs


def _build_mundane_character_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    clusters: list[dict[str, Any]],
) -> list[FanOutSpec]:
    """Create FanOutSpec list for the mundane characters stage (Q1 then Q2).

    Characters are distributed across clusters by cycling (i % len(clusters)).

    Args:
        plan: Entity plan dict with a "characters_mundane" key.
        compressed_preamble: Full compressed preamble string.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.
        clusters: List of cluster dicts from the substrate stage.

    Returns:
        List of FanOutSpec, Q1 characters first then Q2.
    """
    axes_subset = subset_axes(compressed_preamble, _DOMAIN_SUBSETS["characters-mundane"])
    chars = plan["characters_mundane"]
    q1_count = chars.get("q1_count", 0) if isinstance(chars, dict) else 0
    q2_count = chars.get("q2_count", 0) if isinstance(chars, dict) else 0

    cluster_slugs = [c.get("slug", f"cluster-{i}") for i, c in enumerate(clusters)]
    if not cluster_slugs:
        cluster_slugs = ["default"]

    specs: list[FanOutSpec] = []
    index = 0

    # Q1 first
    for i in range(q1_count):
        cluster = cluster_slugs[i % len(cluster_slugs)]
        specs.append(
            FanOutSpec(
                stage="characters-mundane",
                index=index,
                template_name="character-mundane-fanout.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "axes_subset": axes_subset,
                    "centrality": "Q1",
                    "cluster_membership": cluster,
                },
            )
        )
        index += 1

    # Q2 second
    for i in range(q2_count):
        cluster = cluster_slugs[i % len(cluster_slugs)]
        specs.append(
            FanOutSpec(
                stage="characters-mundane",
                index=index,
                template_name="character-mundane-fanout.md",
                model_role="fan_out_structured",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "axes_subset": axes_subset,
                    "centrality": "Q2",
                    "cluster_membership": cluster,
                },
            )
        )
        index += 1

    return specs


def _build_significant_character_specs(
    plan: dict[str, Any],
    compressed_preamble: str,
    genre_slug: str,
    setting_slug: str,
    clusters: list[dict[str, Any]],
    archetypes_summary: str,
) -> list[FanOutSpec]:
    """Create FanOutSpec list for significant characters (Q3 then Q4).

    Uses fan_out_creative model (9b). Boundary positions are assigned by
    cycling through cluster pairs.

    Args:
        plan: Entity plan dict with a "characters_significant" key.
        compressed_preamble: Full compressed preamble string.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.
        clusters: List of cluster dicts from the substrate stage.
        archetypes_summary: Markdown summary of bedrock archetypes.

    Returns:
        List of FanOutSpec, Q3 characters first then Q4.
    """
    axes_subset = subset_axes(compressed_preamble, _DOMAIN_SUBSETS["characters-significant"])
    chars = plan["characters_significant"]
    q3_count = chars.get("q3_count", 0) if isinstance(chars, dict) else 0
    q4_count = chars.get("q4_count", 0) if isinstance(chars, dict) else 0

    # Build cluster pairs for boundary positions
    cluster_slugs = [c.get("slug", f"cluster-{i}") for i, c in enumerate(clusters)]
    if len(cluster_slugs) >= 2:
        pairs = list(combinations(cluster_slugs, 2))
    elif cluster_slugs:
        pairs = [(cluster_slugs[0], cluster_slugs[0])]
    else:
        pairs = [("default-a", "default-b")]

    specs: list[FanOutSpec] = []
    index = 0

    # Q3 first
    for i in range(q3_count):
        pair = pairs[i % len(pairs)]
        specs.append(
            FanOutSpec(
                stage="characters-significant",
                index=index,
                template_name="character-significant-fanout.md",
                model_role="fan_out_creative",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "axes_subset": axes_subset,
                    "centrality": "Q3",
                    "boundary_position": f"{pair[0]} / {pair[1]}",
                    "archetypes_summary": archetypes_summary,
                },
            )
        )
        index += 1

    # Q4 second
    for i in range(q4_count):
        pair = pairs[i % len(pairs)]
        specs.append(
            FanOutSpec(
                stage="characters-significant",
                index=index,
                template_name="character-significant-fanout.md",
                model_role="fan_out_creative",
                context={
                    "genre_slug": genre_slug,
                    "setting_slug": setting_slug,
                    "axes_subset": axes_subset,
                    "centrality": "Q4",
                    "boundary_position": f"{pair[0]} / {pair[1]}",
                    "archetypes_summary": archetypes_summary,
                },
            )
        )
        index += 1

    return specs


# ---------------------------------------------------------------------------
# Context summarizers
# ---------------------------------------------------------------------------


def _summarize_places(places: list[dict[str, Any]]) -> str:
    """Format places as brief markdown for downstream context.

    Args:
        places: List of place entity dicts.

    Returns:
        Markdown-formatted summary string.
    """
    if not places:
        return "No places generated yet."

    lines: list[str] = []
    for p in places:
        name = p.get("name", p.get("slug", "Unknown"))
        place_type = p.get("place_type", "?")
        desc = p.get("description", "")
        line = f"- **{name}** ({place_type})"
        if desc:
            line += f" — {desc[:150]}"
        lines.append(line)

    return "\n".join(lines)


def _summarize_orgs(orgs: list[dict[str, Any]]) -> str:
    """Format organizations as brief markdown for downstream context.

    Args:
        orgs: List of organization entity dicts.

    Returns:
        Markdown-formatted summary string.
    """
    if not orgs:
        return "No organizations generated yet."

    lines: list[str] = []
    for o in orgs:
        name = o.get("name", o.get("slug", "Unknown"))
        org_type = o.get("org_type", o.get("type", "?"))
        desc = o.get("description", "")
        line = f"- **{name}** ({org_type})"
        if desc:
            line += f" — {desc[:150]}"
        lines.append(line)

    return "\n".join(lines)


def _summarize_clusters(clusters: list[dict[str, Any]]) -> str:
    """Format social clusters as brief markdown for downstream context.

    Args:
        clusters: List of cluster entity dicts.

    Returns:
        Markdown-formatted summary string.
    """
    if not clusters:
        return "No social clusters generated yet."

    lines: list[str] = []
    for c in clusters:
        name = c.get("name", c.get("slug", "Unknown"))
        desc = c.get("description", "")
        line = f"- **{name}**"
        if desc:
            line += f" — {desc[:150]}"
        lines.append(line)

    return "\n".join(lines)


def _summarize_mundane_characters(chars: list[dict[str, Any]]) -> str:
    """Format mundane characters as brief markdown for downstream context.

    Args:
        chars: List of mundane character entity dicts.

    Returns:
        Markdown-formatted summary string.
    """
    if not chars:
        return "No mundane characters generated yet."

    lines: list[str] = []
    for c in chars:
        name = c.get("name", c.get("slug", "Unknown"))
        centrality = c.get("centrality", "?")
        role = c.get("role", "?")
        cluster = c.get("cluster_membership", "?")
        line = f"- [{centrality}] **{name}** — {role} ({cluster})"
        lines.append(line)

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Internal stage helpers
# ---------------------------------------------------------------------------


def _load_stage_output(decomposed_dir: Path, stage: str) -> list[dict[str, Any]]:
    """Load the coherence output for a completed stage.

    Args:
        decomposed_dir: Root directory for decomposed pipeline outputs.
        stage: Stage name.

    Returns:
        List of entity dicts from the stage output file.
    """
    filename = _STAGE_FILENAME[stage]
    path = decomposed_dir / filename

    if not path.exists():
        return []

    data = json.loads(path.read_text())

    # The entity key varies by stage
    for key in ("places", "organizations", "clusters", "characters"):
        if key in data and isinstance(data[key], list):
            return data[key]

    return []


def _load_draft(decomposed_dir: Path, stage: str) -> list[dict[str, Any]]:
    """Load the draft file for a stage (fan-out aggregate output).

    Args:
        decomposed_dir: Root directory for decomposed pipeline outputs.
        stage: Stage name.

    Returns:
        List of draft entity dicts.
    """
    draft_path = decomposed_dir / f"{stage}-draft.json"
    if not draft_path.exists():
        return []
    return json.loads(draft_path.read_text())


def _extract_clusters_from_substrate(
    entities: list[dict[str, Any]] | dict[str, Any],
) -> list[dict[str, Any]]:
    """Extract cluster list from substrate coherence output.

    The substrate coherence may return either a flat list or a dict
    with clusters/relationships — handle both.

    Args:
        entities: Raw substrate coherence output.

    Returns:
        List of cluster dicts.
    """
    if isinstance(entities, dict):
        # Dict form: may have "clusters" key
        return entities.get("clusters", [])
    if isinstance(entities, list):
        return entities
    return []


def _load_genre_profile_summary(data_path: Path, genre_slug: str) -> str:
    """Build a genre profile summary string for the entity plan prompt.

    Args:
        data_path: Root of the storyteller-data checkout.
        genre_slug: Genre identifier.

    Returns:
        Formatted genre profile summary string.
    """
    from narrative_data.tome.elicit_places import (
        _build_genre_profile_summary,
        _build_settings_context,
    )

    # Try to load region.json for genre profile
    region_path = data_path / "narrative-data" / "discovery" / "region" / f"{genre_slug}.json"
    genre_profile = None
    if region_path.exists():
        with contextlib.suppress(json.JSONDecodeError):
            genre_profile = json.loads(region_path.read_text())

    summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    if settings_context:
        summary += "\n\n" + settings_context

    return summary


def _load_archetypes_summary(data_path: Path, genre_slug: str) -> str:
    """Build archetypes + dynamics summary for significant character specs.

    Args:
        data_path: Root of the storyteller-data checkout.
        genre_slug: Genre identifier.

    Returns:
        Combined markdown summary of archetypes and dynamics.
    """
    from narrative_data.tome.elicit_characters_significant import (
        _build_archetypes_context,
        _build_dynamics_context,
        _load_archetype_dynamics,
        _load_archetypes,
    )

    archetypes = _load_archetypes(data_path, genre_slug)
    dynamics = _load_archetype_dynamics(data_path, genre_slug)

    parts: list[str] = []
    arch_ctx = _build_archetypes_context(archetypes)
    if arch_ctx:
        parts.append(arch_ctx)
    dyn_ctx = _build_dynamics_context(dynamics)
    if dyn_ctx:
        parts.append(dyn_ctx)

    return "\n\n".join(parts) if parts else ""


# ---------------------------------------------------------------------------
# Compose world.json
# ---------------------------------------------------------------------------


def _compose_world_json(
    decomposed_dir: Path,
    world_slug: str,
    genre_slug: str,
    setting_slug: str,
) -> Path:
    """Compose a unified world.json from all per-stage coherence outputs.

    Args:
        decomposed_dir: Root directory for decomposed pipeline outputs.
        world_slug: World identifier.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.

    Returns:
        Path to the written world.json file.
    """
    world: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "pipeline": "decomposed",
    }

    for stage in _STAGES:
        entities = _load_stage_output(decomposed_dir, stage)
        if stage == "places":
            world["places"] = entities
        elif stage == "orgs":
            world["organizations"] = entities
        elif stage == "substrate":
            world["clusters"] = entities
        elif stage == "characters-mundane":
            world["characters_mundane"] = entities
        elif stage == "characters-significant":
            world["characters_significant"] = entities

    output_path = decomposed_dir / "world.json"
    output_path.write_text(json.dumps(world, indent=2))

    return output_path


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def orchestrate_world(
    data_path: Path,
    world_slug: str,
    stage: str | None = None,
    coherence_only: bool = False,
) -> None:
    """Run the full decomposed pipeline or a single stage.

    Stages: places -> orgs -> substrate -> characters-mundane -> characters-significant.
    Each stage: plan specs -> fan_out -> save_instances -> aggregate -> cohere -> save.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier.
        stage: If set, run only this stage (load upstream from existing files).
        coherence_only: If True, skip fan-out and load from existing draft files.
    """
    from rich.console import Console

    from narrative_data.ollama import OllamaClient
    from narrative_data.tome.plan_entities import plan_entities

    console = Console()

    # ------------------------------------------------------------------
    # Setup
    # ------------------------------------------------------------------
    console.print(f"[bold]Orchestrating decomposed pipeline for[/bold] [cyan]{world_slug}[/cyan]")

    decomposed_dir = _ensure_decomposed_dir(data_path, world_slug)
    client = OllamaClient()

    # ------------------------------------------------------------------
    # World summary + plan
    # ------------------------------------------------------------------
    console.print("[bold]Building world summary...[/bold]")
    world_summary = _build_world_summary_from_path(data_path, world_slug)
    genre_slug = world_summary["genre_slug"]
    setting_slug = world_summary["setting_slug"]
    compressed_preamble = world_summary["compressed_preamble"]

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"axes=[cyan]{world_summary['axis_count']}[/cyan]"
    )

    # Genre profile summary for entity plan
    genre_profile_summary = _load_genre_profile_summary(data_path, genre_slug)

    # Entity plan
    plan_template = _PROMPTS_DIR / "tome" / "decomposed" / "entity-plan.md"
    console.print("[bold]Planning entity budget...[/bold]")
    plan = plan_entities(client, plan_template, world_summary, genre_profile_summary)

    plan_path = decomposed_dir / "entity-plan.json"
    plan_path.write_text(json.dumps(plan, indent=2))
    console.print(f"  [dim]Saved entity plan to {plan_path}[/dim]")

    # Template directory for fan-out and coherence prompts
    template_dir = _PROMPTS_DIR / "tome" / "decomposed"

    # Determine which stages to run
    stages_to_run = [stage] if stage else _STAGES

    # ------------------------------------------------------------------
    # Stage: places
    # ------------------------------------------------------------------
    if "places" in stages_to_run:
        console.print("\n[bold blue]--- Stage: places ---[/bold blue]")
        specs = _build_place_specs(plan, compressed_preamble, genre_slug, setting_slug)
        console.print(f"  Specs: [cyan]{len(specs)}[/cyan] place(s)")

        if coherence_only:
            draft = _load_draft(decomposed_dir, "places")
            console.print(f"  [dim]Loaded {len(draft)} draft(s) from disk[/dim]")
        else:
            results = fan_out(client, template_dir, specs)
            save_instances(decomposed_dir, "places", specs[: len(results)], results)
            aggregate(decomposed_dir, "places", results)
            draft = results
            console.print(f"  Fan-out: [green]{len(results)}[/green] succeeded")

        coherence_template = template_dir / "places-coherence.md"
        cohered = cohere(client, coherence_template, world_summary, draft, "")
        save_coherence_output(
            decomposed_dir, "places", cohered, world_slug, genre_slug, setting_slug
        )
        console.print(f"  Coherence: [green]{len(cohered)}[/green] place(s)")

    # Load places for downstream context
    places = _load_stage_output(decomposed_dir, "places")
    places_summary = _summarize_places(places)

    # ------------------------------------------------------------------
    # Stage: orgs
    # ------------------------------------------------------------------
    if "orgs" in stages_to_run:
        console.print("\n[bold blue]--- Stage: orgs ---[/bold blue]")
        specs = _build_org_specs(
            plan, compressed_preamble, genre_slug, setting_slug, places_summary
        )
        console.print(f"  Specs: [cyan]{len(specs)}[/cyan] org(s)")

        if coherence_only:
            draft = _load_draft(decomposed_dir, "orgs")
            console.print(f"  [dim]Loaded {len(draft)} draft(s) from disk[/dim]")
        else:
            results = fan_out(client, template_dir, specs)
            save_instances(decomposed_dir, "orgs", specs[: len(results)], results)
            aggregate(decomposed_dir, "orgs", results)
            draft = results
            console.print(f"  Fan-out: [green]{len(results)}[/green] succeeded")

        coherence_template = template_dir / "orgs-coherence.md"
        cohered = cohere(client, coherence_template, world_summary, draft, places_summary)
        save_coherence_output(decomposed_dir, "orgs", cohered, world_slug, genre_slug, setting_slug)
        console.print(f"  Coherence: [green]{len(cohered)}[/green] org(s)")

    # Load orgs for downstream context
    orgs = _load_stage_output(decomposed_dir, "orgs")
    orgs_summary = _summarize_orgs(orgs)
    upstream_summary = f"{places_summary}\n\n{orgs_summary}"

    # ------------------------------------------------------------------
    # Stage: substrate
    # ------------------------------------------------------------------
    if "substrate" in stages_to_run:
        console.print("\n[bold blue]--- Stage: substrate ---[/bold blue]")
        specs = _build_substrate_specs(
            plan, compressed_preamble, genre_slug, setting_slug, upstream_summary
        )
        console.print(f"  Specs: [cyan]{len(specs)}[/cyan] cluster(s)")

        if coherence_only:
            draft = _load_draft(decomposed_dir, "substrate")
            console.print(f"  [dim]Loaded {len(draft)} draft(s) from disk[/dim]")
        else:
            results = fan_out(client, template_dir, specs)
            save_instances(decomposed_dir, "substrate", specs[: len(results)], results)
            aggregate(decomposed_dir, "substrate", results)
            draft = results
            console.print(f"  Fan-out: [green]{len(results)}[/green] succeeded")

        coherence_template = template_dir / "substrate-coherence.md"
        cohered = cohere(client, coherence_template, world_summary, draft, upstream_summary)
        # Substrate coherence may return list or dict with clusters/relationships
        substrate_entities = _extract_clusters_from_substrate(cohered)
        save_coherence_output(
            decomposed_dir,
            "substrate",
            substrate_entities,
            world_slug,
            genre_slug,
            setting_slug,
        )
        console.print(f"  Coherence: [green]{len(substrate_entities)}[/green] cluster(s)")

    # Load clusters for downstream context
    clusters = _load_stage_output(decomposed_dir, "substrate")
    clusters_summary = _summarize_clusters(clusters)

    # ------------------------------------------------------------------
    # Stage: characters-mundane
    # ------------------------------------------------------------------
    if "characters-mundane" in stages_to_run:
        console.print("\n[bold blue]--- Stage: characters-mundane ---[/bold blue]")
        specs = _build_mundane_character_specs(
            plan, compressed_preamble, genre_slug, setting_slug, clusters
        )
        console.print(f"  Specs: [cyan]{len(specs)}[/cyan] character(s)")

        if coherence_only:
            draft = _load_draft(decomposed_dir, "characters-mundane")
            console.print(f"  [dim]Loaded {len(draft)} draft(s) from disk[/dim]")
        else:
            results = fan_out(client, template_dir, specs)
            save_instances(
                decomposed_dir,
                "characters-mundane",
                specs[: len(results)],
                results,
            )
            aggregate(decomposed_dir, "characters-mundane", results)
            draft = results
            console.print(f"  Fan-out: [green]{len(results)}[/green] succeeded")

        mundane_upstream = f"{upstream_summary}\n\n{clusters_summary}"
        coherence_template = template_dir / "characters-mundane-coherence.md"
        cohered = cohere(
            client,
            coherence_template,
            world_summary,
            draft,
            mundane_upstream,
        )
        save_coherence_output(
            decomposed_dir,
            "characters-mundane",
            cohered,
            world_slug,
            genre_slug,
            setting_slug,
        )
        console.print(f"  Coherence: [green]{len(cohered)}[/green] character(s)")

    # Load mundane characters for downstream context
    mundane_chars = _load_stage_output(decomposed_dir, "characters-mundane")
    mundane_summary = _summarize_mundane_characters(mundane_chars)

    # ------------------------------------------------------------------
    # Stage: characters-significant
    # ------------------------------------------------------------------
    if "characters-significant" in stages_to_run:
        console.print("\n[bold blue]--- Stage: characters-significant ---[/bold blue]")

        archetypes_summary = _load_archetypes_summary(data_path, genre_slug)
        specs = _build_significant_character_specs(
            plan,
            compressed_preamble,
            genre_slug,
            setting_slug,
            clusters,
            archetypes_summary,
        )
        console.print(f"  Specs: [cyan]{len(specs)}[/cyan] character(s)")

        if coherence_only:
            draft = _load_draft(decomposed_dir, "characters-significant")
            console.print(f"  [dim]Loaded {len(draft)} draft(s) from disk[/dim]")
        else:
            results = fan_out(client, template_dir, specs)
            save_instances(
                decomposed_dir,
                "characters-significant",
                specs[: len(results)],
                results,
            )
            aggregate(decomposed_dir, "characters-significant", results)
            draft = results
            console.print(f"  Fan-out: [green]{len(results)}[/green] succeeded")

        sig_upstream = f"{upstream_summary}\n\n{clusters_summary}\n\n{mundane_summary}"
        coherence_template = template_dir / "characters-significant-coherence.md"
        cohered = cohere(
            client,
            coherence_template,
            world_summary,
            draft,
            sig_upstream,
            extra_context={"archetypes_summary": archetypes_summary},
        )
        save_coherence_output(
            decomposed_dir,
            "characters-significant",
            cohered,
            world_slug,
            genre_slug,
            setting_slug,
        )
        console.print(f"  Coherence: [green]{len(cohered)}[/green] character(s)")

    # ------------------------------------------------------------------
    # Compose world.json
    # ------------------------------------------------------------------
    console.print("\n[bold]Composing world.json...[/bold]")
    world_path = _compose_world_json(decomposed_dir, world_slug, genre_slug, setting_slug)
    console.print(f"[bold green]Written:[/bold green] {world_path}")


# ---------------------------------------------------------------------------
# Compressed-mode helpers
# ---------------------------------------------------------------------------

_COMPRESSED_TIMEOUT = 600.0
_COMPRESSED_TEMPERATURE = 0.5

# Template filenames (under prompts/tome/) and the entity key in the output
_COMPRESSED_STAGE_META: dict[str, dict[str, str]] = {
    "places": {
        "template": "place-elicitation.md",
        "entity_key": "places",
        "filename": "places.json",
    },
    "orgs": {
        "template": "org-elicitation.md",
        "entity_key": "organizations",
        "filename": "organizations.json",
    },
    "substrate": {
        "template": "social-substrate-elicitation.md",
        "entity_key": "clusters",
        "filename": "social-substrate.json",
    },
    "characters-mundane": {
        "template": "character-mundane-elicitation.md",
        "entity_key": "characters",
        "filename": "characters-mundane.json",
    },
    "characters-significant": {
        "template": "character-significant-elicitation.md",
        "entity_key": "characters",
        "filename": "characters-significant.json",
    },
}


def _build_compressed_prompt_places(
    template: str,
    world_pos: dict[str, Any],
    compressed_preamble: str,
    data_path: Path,
) -> str:
    """Build the places elicitation prompt with compressed preamble."""
    from narrative_data.tome.elicit_places import (
        _build_genre_profile_summary,
        _build_settings_context,
    )

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    genre_summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", compressed_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
    )


def _build_compressed_prompt_orgs(
    template: str,
    world_pos: dict[str, Any],
    compressed_preamble: str,
    data_path: Path,
    world_dir: Path,
) -> str:
    """Build the orgs elicitation prompt with compressed preamble."""
    from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
    from narrative_data.tome.elicit_places import (
        _build_genre_profile_summary,
        _build_settings_context,
    )

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    genre_summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    places = _load_places(world_dir)
    places_context = _build_places_context(places)

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", compressed_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
    )


def _build_compressed_prompt_substrate(
    template: str,
    world_pos: dict[str, Any],
    compressed_preamble: str,
    data_path: Path,
    world_dir: Path,
) -> str:
    """Build the social-substrate elicitation prompt with compressed preamble."""
    from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
    from narrative_data.tome.elicit_places import (
        _build_genre_profile_summary,
        _build_settings_context,
    )
    from narrative_data.tome.elicit_social_substrate import (
        _build_orgs_context,
        _extract_axis_value,
        _load_orgs,
    )

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")
    positions = world_pos.get("positions", [])

    genre_summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    places = _load_places(world_dir)
    orgs = _load_orgs(world_dir)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)

    kinship_value = _extract_axis_value(positions, "kinship-system")
    stratification_value = _extract_axis_value(positions, "social-stratification")

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", compressed_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{kinship_system_value}", kinship_value)
        .replace("{stratification_value}", stratification_value)
    )


def _build_compressed_prompt_characters_mundane(
    template: str,
    world_pos: dict[str, Any],
    compressed_preamble: str,
    data_path: Path,
    world_dir: Path,
) -> str:
    """Build the mundane character elicitation prompt with compressed preamble."""
    from narrative_data.tome.elicit_characters_mundane import (
        _build_social_substrate_context,
        _load_social_substrate,
    )
    from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
    from narrative_data.tome.elicit_places import (
        _build_genre_profile_summary,
        _build_settings_context,
    )
    from narrative_data.tome.elicit_social_substrate import (
        _build_orgs_context,
        _load_orgs,
    )

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    genre_summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    places = _load_places(world_dir)
    orgs = _load_orgs(world_dir)
    substrate = _load_social_substrate(world_dir)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)
    substrate_context = _build_social_substrate_context(substrate)

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", compressed_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{social_substrate_context}", substrate_context)
    )


def _build_compressed_prompt_characters_significant(
    template: str,
    world_pos: dict[str, Any],
    compressed_preamble: str,
    data_path: Path,
    world_dir: Path,
) -> str:
    """Build the significant character elicitation prompt with compressed preamble."""
    from narrative_data.tome.elicit_characters_mundane import (
        _build_social_substrate_context,
        _load_social_substrate,
    )
    from narrative_data.tome.elicit_characters_significant import (
        _build_archetypes_context,
        _build_dynamics_context,
        _build_mundane_characters_context,
        _load_archetype_dynamics,
        _load_archetypes,
        _load_mundane_characters,
    )
    from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
    from narrative_data.tome.elicit_places import (
        _build_genre_profile_summary,
        _build_settings_context,
    )
    from narrative_data.tome.elicit_social_substrate import (
        _build_orgs_context,
        _load_orgs,
    )

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    genre_summary = _build_genre_profile_summary(genre_profile)
    settings_context = _build_settings_context(data_path, genre_slug)
    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    places = _load_places(world_dir)
    orgs = _load_orgs(world_dir)
    substrate = _load_social_substrate(world_dir)
    mundane_characters = _load_mundane_characters(world_dir)

    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)
    substrate_context = _build_social_substrate_context(substrate)
    mundane_context = _build_mundane_characters_context(mundane_characters)

    archetypes = _load_archetypes(data_path, genre_slug)
    archetype_dynamics = _load_archetype_dynamics(data_path, genre_slug)
    archetypes_context = _build_archetypes_context(archetypes)
    dynamics_context = _build_dynamics_context(archetype_dynamics)

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", compressed_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{social_substrate_context}", substrate_context)
        .replace("{mundane_characters_context}", mundane_context)
        .replace("{archetypes_context}", archetypes_context)
        .replace("{archetype_dynamics_context}", dynamics_context)
    )


def _build_compressed_prompt(
    stage: str,
    world_pos: dict[str, Any],
    compressed_preamble: str,
    data_path: Path,
    world_dir: Path,
) -> str:
    """Build a prompt for a compressed-mode stage.

    Loads the existing Phase 3b template and substitutes the compressed
    preamble (instead of the full ``_build_world_preamble`` output) while
    using the existing helper functions for all other placeholders.

    Args:
        stage: Pipeline stage name.
        world_pos: Parsed world-position.json dict.
        compressed_preamble: Compressed world preamble string.
        data_path: Root of the storyteller-data checkout.
        world_dir: Path to the world directory.

    Returns:
        Fully substituted prompt string.
    """
    meta = _COMPRESSED_STAGE_META[stage]
    template_path = _PROMPTS_DIR / "tome" / meta["template"]
    template = template_path.read_text()

    builders = {
        "places": _build_compressed_prompt_places,
        "orgs": _build_compressed_prompt_orgs,
        "substrate": _build_compressed_prompt_substrate,
        "characters-mundane": _build_compressed_prompt_characters_mundane,
        "characters-significant": _build_compressed_prompt_characters_significant,
    }

    builder = builders[stage]

    # places takes 4 args; the rest take 5 (world_dir)
    if stage == "places":
        return builder(template, world_pos, compressed_preamble, data_path)
    return builder(template, world_pos, compressed_preamble, data_path, world_dir)


def _parse_compressed_response(
    stage: str,
    response: str,
) -> list[dict[str, Any]] | dict[str, Any]:
    """Parse an LLM response for a compressed-mode stage.

    Args:
        stage: Pipeline stage name.
        response: Raw LLM response text.

    Returns:
        Parsed entities — a list for most stages, a dict for substrate.

    Raises:
        ValueError: If parsing fails.
    """
    if stage == "substrate":
        from narrative_data.tome.elicit_social_substrate import (
            _parse_substrate_response,
        )

        return _parse_substrate_response(response)

    # All other stages return JSON arrays
    from narrative_data.tome.elicit_places import _parse_places_response

    return _parse_places_response(response)


def _save_compressed_output(
    decomposed_dir: Path,
    stage: str,
    parsed: list[dict[str, Any]] | dict[str, Any],
    world_slug: str,
    genre_slug: str,
    setting_slug: str,
) -> Path:
    """Save compressed-mode output to decomposed/{filename} and per-entity instance files.

    Args:
        decomposed_dir: Root of decomposed pipeline outputs.
        stage: Pipeline stage name.
        parsed: Parsed response — list for most stages, dict for substrate.
        world_slug: World identifier.
        genre_slug: Genre identifier.
        setting_slug: Setting identifier.

    Returns:
        Path to the written stage output file.
    """
    meta = _COMPRESSED_STAGE_META[stage]
    entity_key = meta["entity_key"]
    filename = meta["filename"]

    # Determine the entity list for instance files
    if stage == "substrate" and isinstance(parsed, dict):
        entities = parsed.get("clusters", [])
        relationships = parsed.get("relationships", [])
        output: dict[str, Any] = {
            "world_slug": world_slug,
            "genre_slug": genre_slug,
            "setting_slug": setting_slug,
            "generated_at": now_iso(),
            "pipeline": "compressed",
            "cluster_count": len(entities),
            "relationship_count": len(relationships),
            "clusters": entities,
            "relationships": relationships,
        }
    else:
        entities = parsed if isinstance(parsed, list) else []
        output = {
            "world_slug": world_slug,
            "genre_slug": genre_slug,
            "setting_slug": setting_slug,
            "generated_at": now_iso(),
            "pipeline": "compressed",
            "count": len(entities),
            entity_key: entities,
        }

    # Write full stage output
    decomposed_dir.mkdir(parents=True, exist_ok=True)
    output_path = decomposed_dir / filename
    output_path.write_text(json.dumps(output, indent=2))

    # Write per-entity instance files
    instance_dir = decomposed_dir / "fan-out" / stage
    instance_dir.mkdir(parents=True, exist_ok=True)
    for i, entity in enumerate(entities):
        instance_path = instance_dir / f"instance-{i:03d}.json"
        instance_path.write_text(json.dumps(entity, indent=2))

    return output_path


# ---------------------------------------------------------------------------
# Public entry point: compressed mode
# ---------------------------------------------------------------------------


def orchestrate_compressed(
    data_path: Path,
    world_slug: str,
    stage: str | None = None,
) -> None:
    """Run the compressed-preamble pipeline: single 35b call per stage.

    Uses existing Phase 3b elicitation templates with the compressed preamble
    substituted for the full world preamble.  Saves output to the decomposed/
    directory and writes per-entity instance files under decomposed/fan-out/.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier.
        stage: If set, run only this stage.  Upstream data must already exist
            (e.g. places.json for running the orgs stage).
    """
    from rich.console import Console

    from narrative_data.config import ELICITATION_MODEL
    from narrative_data.ollama import OllamaClient

    console = Console()
    console.print(
        f"[bold]Compressed pipeline for[/bold] [cyan]{world_slug}[/cyan] "
        f"[dim](mode=compressed)[/dim]"
    )

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    domains_dir = data_path / "narrative-data" / "domains"
    decomposed_dir = _ensure_decomposed_dir(data_path, world_slug)

    # Load world position and build compressed preamble
    wp_path = world_dir / "world-position.json"
    if not wp_path.exists():
        console.print(f"[red]Error:[/red] world-position.json not found at {wp_path}")
        raise SystemExit(1)

    world_pos = json.loads(wp_path.read_text())
    summary = build_world_summary(world_pos, domains_dir)
    compressed_preamble = summary["compressed_preamble"]
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"preamble=[dim]{len(compressed_preamble)} chars[/dim]"
    )

    client = OllamaClient()
    stages_to_run = [stage] if stage else _STAGES

    for current_stage in _STAGES:
        if current_stage not in stages_to_run:
            continue

        console.print(f"\n[bold blue]--- Stage: {current_stage} ---[/bold blue]")

        # Build prompt
        console.print("[bold]Building prompt...[/bold]")
        prompt = _build_compressed_prompt(
            current_stage, world_pos, compressed_preamble, data_path, world_dir
        )
        console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

        # Call LLM
        console.print(
            f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
            f"[dim](timeout={_COMPRESSED_TIMEOUT}s, "
            f"temperature={_COMPRESSED_TEMPERATURE})[/dim]"
        )
        response = client.generate(
            model=ELICITATION_MODEL,
            prompt=prompt,
            timeout=_COMPRESSED_TIMEOUT,
            temperature=_COMPRESSED_TEMPERATURE,
        )
        console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

        # Parse
        console.print("[bold]Parsing response...[/bold]")
        try:
            parsed = _parse_compressed_response(current_stage, response)
        except ValueError as exc:
            console.print(f"[red]Parse error:[/red] {exc}")
            raise SystemExit(1) from exc

        # Save
        output_path = _save_compressed_output(
            decomposed_dir, current_stage, parsed, world_slug, genre_slug, setting_slug
        )
        if current_stage == "substrate" and isinstance(parsed, dict):
            entity_count = len(parsed.get("clusters", []))
        elif isinstance(parsed, list):
            entity_count = len(parsed)
        else:
            entity_count = 0
        console.print(
            f"  [green]{entity_count}[/green] entities written to [dim]{output_path}[/dim]"
        )

        # Copy to the standard location so downstream stages can load prerequisites.
        # Backs up existing files to {name}.baseline.json to preserve Phase 3b data.
        standard_path = world_dir / _COMPRESSED_STAGE_META[current_stage]["filename"]
        if standard_path.resolve() != output_path.resolve():
            import shutil

            if standard_path.exists():
                backup = standard_path.with_suffix(".baseline.json")
                if not backup.exists():
                    shutil.copy2(standard_path, backup)
                    console.print(f"  [dim]Backed up baseline to {backup.name}[/dim]")
            shutil.copy2(output_path, standard_path)
            console.print(f"  [dim]Copied to {standard_path}[/dim]")

    # Compose world.json
    console.print("\n[bold]Composing world.json...[/bold]")
    world_path = _compose_world_json(decomposed_dir, world_slug, genre_slug, setting_slug)
    console.print(f"[bold green]Written:[/bold green] {world_path}")
