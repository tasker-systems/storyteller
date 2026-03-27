# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Generate edge chains from the mutual production graph to validate coherence.

Starts from seed positions on material-conditions axes, follows edges
forward through domains, and generates world-position sketches at
varying chain depths.  Includes discriminative power analysis to test
whether different seed configurations produce distinguishable worlds.

Usage (via CLI):
    uv run narrative-data tome stress-test [--count N] [--depth N] [--seed-domain DOMAIN]
"""

from __future__ import annotations

import json
import random
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------


def load_graph(data_path: Path) -> dict[str, Any]:
    """Load the canonical edges.json."""
    path = data_path / "narrative-data" / "tome" / "edges.json"
    with open(path) as f:
        return json.load(f)


def load_axes(data_path: Path) -> dict[str, dict[str, Any]]:
    """Load all axes keyed by slug."""
    axes: dict[str, dict[str, Any]] = {}
    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    for f in domains_dir.glob("*.json"):
        with open(f) as fh:
            domain = json.load(fh)
            for axis in domain["axes"]:
                axes[axis["slug"]] = axis
    return axes


# ---------------------------------------------------------------------------
# Adjacency index (built once, queried many times)
# ---------------------------------------------------------------------------


def build_adjacency(graph: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    """Build from_axis → [edge, ...] index for fast lookup."""
    adj: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for e in graph["edges"]:
        adj[e["from_axis"]].append(e)
    return dict(adj)


# ---------------------------------------------------------------------------
# Position generation
# ---------------------------------------------------------------------------


def random_position(axis: dict[str, Any]) -> str:
    """Generate a random position for an axis based on its type."""
    axis_type = axis["axis_type"]
    values = axis.get("values", [])

    if axis_type in ("categorical", "set", "ordinal") and isinstance(values, list):
        return random.choice(values)
    elif axis_type == "bipolar" and isinstance(values, dict):
        # Pick a named pole or a midpoint
        pole = random.choice(["low", "high", "mid"])
        if pole == "low":
            return values.get("low_label", "low").split(" — ")[0].strip()
        elif pole == "high":
            return values.get("high_label", "high").split(" — ")[0].strip()
        return "moderate"
    elif axis_type == "profile" and isinstance(values, dict):
        levels = values.get("levels", ["scarce", "moderate", "abundant"])
        subs = values.get("sub_dimensions", [])
        return ", ".join(f"{s}:{random.choice(levels)}" for s in subs)
    return "unknown"


# ---------------------------------------------------------------------------
# Chain generation
# ---------------------------------------------------------------------------


def generate_chain(
    adj: dict[str, list[dict[str, Any]]],
    axes: dict[str, dict[str, Any]],
    seed_axis: str,
    seed_position: str,
    max_depth: int = 6,
    weight_noise: float = 0.15,
) -> list[dict[str, Any]]:
    """Follow edges from a seed axis with weighted-stochastic traversal.

    Instead of always picking the highest-weight edge (greedy), we add
    gaussian noise to weights before ranking.  This produces diverse chains
    from the same seed while still preferring strong edges.
    """
    chain: list[dict[str, Any]] = [
        {"axis": seed_axis, "position": seed_position, "via_edge": None}
    ]
    visited = {seed_axis}
    current = seed_axis

    for _ in range(max_depth):
        outgoing = adj.get(current, [])
        candidates = [e for e in outgoing if e["to_axis"] not in visited]
        if not candidates:
            break

        # Weighted-stochastic selection: add noise, pick top
        scored = [
            (e, e.get("weight", 0.5) + random.gauss(0, weight_noise))
            for e in candidates
        ]
        scored.sort(key=lambda x: x[1], reverse=True)
        edge = scored[0][0]
        target = edge["to_axis"]

        if target not in axes:
            break

        position = random_position(axes[target])
        chain.append(
            {
                "axis": target,
                "position": position,
                "domain": axes[target].get("domain", "?"),
                "via_edge": {
                    "type": edge["edge_type"],
                    "weight": edge.get("weight", 0.5),
                    "description": edge.get("description", ""),
                },
            }
        )

        visited.add(target)
        current = target

    return chain


# ---------------------------------------------------------------------------
# World sketch generation
# ---------------------------------------------------------------------------


def generate_world_sketches(
    data_path: Path,
    n_sketches: int = 20,
    max_depth: int = 6,
    seed_domain: str = "material-conditions",
) -> list[dict[str, Any]]:
    """Generate n world sketches by stochastic chain traversal."""
    graph = load_graph(data_path)
    axes = load_axes(data_path)
    adj = build_adjacency(graph)

    seed_axes = [
        slug for slug, axis in axes.items() if axis.get("domain") == seed_domain
    ]
    if not seed_axes:
        raise ValueError(f"No axes found in domain '{seed_domain}'")

    sketches = []
    for i in range(n_sketches):
        seed = random.choice(seed_axes)
        position = random_position(axes[seed])
        chain = generate_chain(adj, axes, seed, position, max_depth)

        domains_touched = list(
            {
                step.get("domain", axes.get(step["axis"], {}).get("domain", "?"))
                for step in chain
                if step["axis"] in axes
            }
        )

        sketches.append(
            {
                "sketch_id": i + 1,
                "seed_axis": seed,
                "seed_position": position,
                "chain_depth": len(chain),
                "domains_touched": sorted(domains_touched),
                "chain": chain,
            }
        )

    return sketches


# ---------------------------------------------------------------------------
# Discriminative power analysis
# ---------------------------------------------------------------------------


def analyze_discriminative_power(
    sketches: list[dict[str, Any]],
) -> dict[str, Any]:
    """Measure whether different seeds produce distinguishable worlds.

    Returns metrics on:
    - axis_convergence: how often different seeds reach the same axes
    - domain_reach: distribution of domains touched
    - path_diversity: how many unique next-hops appear from the same axis
    - seed_separation: whether the same seed produces varied chains
    """
    # Axis convergence: which axes appear most across all sketches?
    axis_freq: Counter[str] = Counter()
    for s in sketches:
        for step in s["chain"]:
            axis_freq[step["axis"]] += 1

    # Domain reach per sketch
    domain_reach = Counter(len(s["domains_touched"]) for s in sketches)

    # Group by seed axis to check separation
    by_seed: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for s in sketches:
        by_seed[s["seed_axis"]].append(s)

    seed_diversity: dict[str, dict[str, Any]] = {}
    for seed, group in by_seed.items():
        if len(group) < 2:
            continue
        # How many unique second-step axes appear?
        second_steps = {
            s["chain"][1]["axis"] for s in group if len(s["chain"]) > 1
        }
        # How many unique terminal axes?
        terminals = {s["chain"][-1]["axis"] for s in group}
        # Jaccard similarity of axis sets between pairs
        axis_sets = [
            {step["axis"] for step in s["chain"]} for s in group
        ]
        jaccard_pairs = []
        for i in range(len(axis_sets)):
            for j in range(i + 1, len(axis_sets)):
                inter = len(axis_sets[i] & axis_sets[j])
                union = len(axis_sets[i] | axis_sets[j])
                jaccard_pairs.append(inter / union if union else 1.0)

        seed_diversity[seed] = {
            "n_sketches": len(group),
            "unique_second_steps": len(second_steps),
            "unique_terminals": len(terminals),
            "mean_jaccard": (
                sum(jaccard_pairs) / len(jaccard_pairs) if jaccard_pairs else 1.0
            ),
        }

    # Edge type distribution across all chains
    edge_types: Counter[str] = Counter()
    for s in sketches:
        for step in s["chain"]:
            if step["via_edge"]:
                edge_types[step["via_edge"]["type"]] += 1

    # Economic → political convergence check
    econ_to_political: list[str] = []
    for s in sketches:
        for step in s["chain"]:
            if step.get("domain") == "political-structures" and step["via_edge"]:
                econ_to_political.append(step["axis"])
    political_convergence = Counter(econ_to_political)

    return {
        "axis_frequency": axis_freq.most_common(15),
        "domain_reach_distribution": dict(sorted(domain_reach.items())),
        "seed_diversity": seed_diversity,
        "edge_type_distribution": dict(edge_types),
        "political_convergence": dict(political_convergence.most_common(10)),
    }


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def run_stress_test(
    data_path: Path,
    n_sketches: int = 20,
    max_depth: int = 6,
    seed_domain: str = "material-conditions",
) -> None:
    """Run the chain generation stress test and print results."""
    from rich.console import Console

    console = Console()

    sketches = generate_world_sketches(
        data_path,
        n_sketches=n_sketches,
        max_depth=max_depth,
        seed_domain=seed_domain,
    )

    console.print(
        f"[bold cyan]Generated {len(sketches)} world sketches "
        f"(depth={max_depth}, seed={seed_domain})[/bold cyan]\n"
    )

    for sketch in sketches:
        console.print(
            f"[bold]--- Sketch {sketch['sketch_id']} ---[/bold]"
        )
        console.print(
            f"  Seed: [cyan]{sketch['seed_axis']}[/cyan] = "
            f"[green]{sketch['seed_position']}[/green]"
        )
        console.print(
            f"  Depth: {sketch['chain_depth']}, "
            f"Domains: {', '.join(sketch['domains_touched'])}"
        )
        for step in sketch["chain"]:
            if step["via_edge"]:
                edge = step["via_edge"]
                console.print(
                    f"    → [cyan]{step['axis']}[/cyan] = "
                    f"[green]{step['position']}[/green]"
                )
                console.print(
                    f"      via {edge['type']} (w={edge['weight']}): "
                    f"{edge['description'][:80]}"
                )
            else:
                console.print(
                    f"    * [cyan]{step['axis']}[/cyan] = "
                    f"[green]{step['position']}[/green] (seed)"
                )
        console.print()

    # --- Summary statistics ---
    depths = [s["chain_depth"] for s in sketches]
    domain_counts = [len(s["domains_touched"]) for s in sketches]
    console.print("[bold]Summary[/bold]")
    console.print(
        f"  Chain depth: min={min(depths)}, max={max(depths)}, "
        f"avg={sum(depths)/len(depths):.1f}"
    )
    console.print(
        f"  Domains touched: min={min(domain_counts)}, "
        f"max={max(domain_counts)}, "
        f"avg={sum(domain_counts)/len(domain_counts):.1f}"
    )

    # Short chain warnings
    short = [s for s in sketches if s["chain_depth"] <= 2]
    if short:
        console.print(
            f"\n  [yellow]Warning: {len(short)} sketches had depth ≤ 2 "
            f"(possible disconnected regions)[/yellow]"
        )
        for s in short:
            console.print(f"    Sketch {s['sketch_id']}: seed={s['seed_axis']}")

    # --- Discriminative power analysis ---
    console.print(f"\n[bold]Discriminative Power Analysis[/bold]")
    analysis = analyze_discriminative_power(sketches)

    console.print("\n  [cyan]Most-visited axes[/cyan] (convergence indicator):")
    for axis, count in analysis["axis_frequency"][:10]:
        pct = count / len(sketches) * 100
        console.print(f"    {axis:<40} {count:>3} ({pct:.0f}%)")

    console.print(f"\n  [cyan]Edge types traversed[/cyan]:")
    for etype, count in sorted(
        analysis["edge_type_distribution"].items(), key=lambda x: -x[1]
    ):
        console.print(f"    {etype:<14} {count}")

    console.print(f"\n  [cyan]Domain reach distribution[/cyan]:")
    for n_domains, count in sorted(analysis["domain_reach_distribution"].items()):
        console.print(f"    {n_domains} domains: {count} sketches")

    if analysis["seed_diversity"]:
        console.print(f"\n  [cyan]Seed diversity[/cyan] (same seed, different chains):")
        for seed, info in sorted(
            analysis["seed_diversity"].items(),
            key=lambda x: x[1]["mean_jaccard"],
        ):
            console.print(
                f"    {seed:<35} "
                f"n={info['n_sketches']}  "
                f"2nd-steps={info['unique_second_steps']}  "
                f"terminals={info['unique_terminals']}  "
                f"jaccard={info['mean_jaccard']:.2f}"
            )
    else:
        console.print(
            "\n  [dim]No seed appeared more than once — "
            "increase --count for diversity analysis[/dim]"
        )

    if analysis["political_convergence"]:
        console.print(
            f"\n  [cyan]Political convergence check[/cyan] "
            f"(which political axes get reached?):"
        )
        for axis, count in analysis["political_convergence"].items():
            console.print(f"    {axis:<40} {count}")

    # --- Write results to JSON ---
    output_path = (
        data_path / "narrative-data" / "tome" / "edges" / "stress-test-results.json"
    )
    results = {
        "config": {
            "n_sketches": n_sketches,
            "max_depth": max_depth,
            "seed_domain": seed_domain,
        },
        "summary": {
            "chain_depth": {
                "min": min(depths),
                "max": max(depths),
                "avg": round(sum(depths) / len(depths), 1),
            },
            "domains_touched": {
                "min": min(domain_counts),
                "max": max(domain_counts),
                "avg": round(sum(domain_counts) / len(domain_counts), 1),
            },
            "short_chains": len(short),
        },
        "analysis": {
            "axis_frequency": analysis["axis_frequency"],
            "edge_type_distribution": analysis["edge_type_distribution"],
            "domain_reach_distribution": analysis["domain_reach_distribution"],
            "seed_diversity": analysis["seed_diversity"],
            "political_convergence": analysis["political_convergence"],
        },
        "sketches": sketches,
    }
    with output_path.open("w") as f:
        json.dump(results, f, indent=2)
        f.write("\n")

    console.print(f"\n[cyan]→[/cyan] {output_path}")
