# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for preamble compression module."""

import json
from pathlib import Path

from narrative_data.tome.compress_preamble import (
    build_domain_index,
    build_world_summary,
    compress_preamble,
    subset_axes,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_domain_file(
    domains_dir: Path,
    domain_slug: str,
    domain_name: str,
    axes: list[str],
) -> None:
    """Write a minimal domain JSON file."""
    domains_dir.mkdir(parents=True, exist_ok=True)
    data = {
        "domain": {"slug": domain_slug, "name": domain_name},
        "axes": [{"slug": slug} for slug in axes],
    }
    (domains_dir / f"{domain_slug}.json").write_text(json.dumps(data))


def _make_world_pos(
    genre_slug: str = "folk-horror",
    setting_slug: str = "mccallisters-barn",
    positions: list[dict] | None = None,
) -> dict:
    """Build a minimal world-position dict."""
    if positions is None:
        positions = []
    return {
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "positions": positions,
    }


def _seed(axis_slug: str, value: str) -> dict:
    return {
        "axis_slug": axis_slug,
        "value": value,
        "source": "seed",
        "confidence": 1.0,
        "justification": None,
    }


_DEFAULT_JUSTIFICATION = "edge1 →produces→ edge2 (w=0.8)"


def _inferred(axis_slug: str, value: str, justification: str = _DEFAULT_JUSTIFICATION) -> dict:
    return {
        "axis_slug": axis_slug,
        "value": value,
        "source": "inferred",
        "confidence": 0.8,
        "justification": justification,
    }


# ---------------------------------------------------------------------------
# Tests: build_domain_index
# ---------------------------------------------------------------------------


def test_build_domain_index_maps_axes_to_domain_name(tmp_path: Path) -> None:
    """Axes are mapped to their domain display name."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir, "material-conditions", "Material Conditions", ["geography-climate", "ecology"]
    )
    _make_domain_file(
        domains_dir, "social-forms", "Social Forms", ["power-distance", "collectivism"]
    )

    index = build_domain_index(domains_dir)

    assert index["geography-climate"] == "Material Conditions"
    assert index["ecology"] == "Material Conditions"
    assert index["power-distance"] == "Social Forms"
    assert index["collectivism"] == "Social Forms"


def test_build_domain_index_multiple_axes_same_domain(tmp_path: Path) -> None:
    """Multiple axes from one domain all map to the same name."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir,
        "economic-forms",
        "Economic Forms",
        ["labor-relations", "trade-systems", "surplus-distribution"],
    )

    index = build_domain_index(domains_dir)

    assert index["labor-relations"] == "Economic Forms"
    assert index["trade-systems"] == "Economic Forms"
    assert index["surplus-distribution"] == "Economic Forms"


def test_build_domain_index_returns_empty_for_missing_dir(tmp_path: Path) -> None:
    """Returns empty dict when the domains directory does not exist."""
    missing_dir = tmp_path / "nonexistent" / "domains"

    index = build_domain_index(missing_dir)

    assert index == {}


def test_build_domain_index_returns_empty_for_empty_dir(tmp_path: Path) -> None:
    """Returns empty dict when domains directory exists but has no JSON files."""
    domains_dir = tmp_path / "domains"
    domains_dir.mkdir(parents=True)

    index = build_domain_index(domains_dir)

    assert index == {}


# ---------------------------------------------------------------------------
# Tests: compress_preamble
# ---------------------------------------------------------------------------


def test_compress_preamble_groups_by_domain(tmp_path: Path) -> None:
    """Positions are grouped under their domain name headers."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir, "material-conditions", "Material Conditions", ["geography-climate"]
    )
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        positions=[
            _seed("geography-climate", "temperate-wet"),
            _seed("power-distance", "high"),
        ]
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "### Material Conditions" in result
    assert "### Social Forms" in result


def test_compress_preamble_drops_justifications(tmp_path: Path) -> None:
    """No edge traces, arrows, or weight notation appear in output."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        positions=[
            _inferred("power-distance", "high", "seed-axis →produces→ power-distance (w=0.9)"),
        ]
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "→produces→" not in result
    assert "→enables→" not in result
    assert "(w=" not in result


def test_compress_preamble_drops_confidence_values(tmp_path: Path) -> None:
    """Confidence numeric values do not appear in output."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        positions=[_inferred("power-distance", "high")],
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "0.8" not in result
    assert "confidence" not in result


def test_compress_preamble_labels_seeds(tmp_path: Path) -> None:
    """Seed positions are labeled with [seed], inferred are not."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir,
        "social-forms",
        "Social Forms",
        ["power-distance", "collectivism"],
    )

    world_pos = _make_world_pos(
        positions=[
            _seed("power-distance", "high"),
            _inferred("collectivism", "collective"),
        ]
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "power-distance: high [seed]" in result
    assert "collectivism: collective" in result
    # inferred entry must NOT have [seed]
    lines = [line for line in result.splitlines() if "collectivism" in line]
    assert len(lines) == 1
    assert "[seed]" not in lines[0]


def test_compress_preamble_preserves_all_axis_values(tmp_path: Path) -> None:
    """All axis slugs and values appear in the output."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir,
        "material-conditions",
        "Material Conditions",
        ["geography-climate", "ecology", "resource-abundance"],
    )

    world_pos = _make_world_pos(
        positions=[
            _seed("geography-climate", "temperate-wet"),
            _inferred("ecology", "dense-forest"),
            _seed("resource-abundance", "moderate"),
        ]
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "geography-climate: temperate-wet" in result
    assert "ecology: dense-forest" in result
    assert "resource-abundance: moderate" in result


def test_compress_preamble_ungrouped_axes_go_to_other(tmp_path: Path) -> None:
    """Axes not in any domain file appear under an 'Other' section."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        positions=[
            _seed("power-distance", "high"),
            _seed("mystery-axis", "unknown"),
        ]
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "### Other" in result
    assert "mystery-axis: unknown [seed]" in result


def test_compress_preamble_known_domains_sorted_alphabetically(tmp_path: Path) -> None:
    """Known domain sections appear in alphabetical order before Other."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])
    _make_domain_file(
        domains_dir, "material-conditions", "Material Conditions", ["geography-climate"]
    )
    _make_domain_file(domains_dir, "economic-forms", "Economic Forms", ["labor-relations"])

    world_pos = _make_world_pos(
        positions=[
            _seed("power-distance", "high"),
            _seed("geography-climate", "temperate"),
            _seed("labor-relations", "feudal"),
            _seed("orphan-axis", "value"),
        ]
    )

    result = compress_preamble(world_pos, domains_dir)

    economic_pos = result.index("### Economic Forms")
    material_pos = result.index("### Material Conditions")
    social_pos = result.index("### Social Forms")
    other_pos = result.index("### Other")

    assert economic_pos < material_pos < social_pos < other_pos


def test_compress_preamble_no_other_section_when_all_grouped(tmp_path: Path) -> None:
    """No Other section is emitted when all axes belong to a domain."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        positions=[_seed("power-distance", "high")],
    )

    result = compress_preamble(world_pos, domains_dir)

    assert "### Other" not in result


def test_compress_preamble_empty_positions(tmp_path: Path) -> None:
    """Empty positions list produces empty or minimal output without errors."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(positions=[])

    result = compress_preamble(world_pos, domains_dir)

    # Should not crash; domain headers should not appear for empty positions
    assert isinstance(result, str)
    assert "### Social Forms" not in result


# ---------------------------------------------------------------------------
# Tests: subset_axes
# ---------------------------------------------------------------------------


def test_subset_axes_returns_matching_domain(tmp_path: Path) -> None:
    """A single domain section is returned correctly."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir,
        "material-conditions",
        "Material Conditions",
        ["geography-climate"],
    )
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        positions=[
            _seed("geography-climate", "temperate"),
            _seed("power-distance", "high"),
        ]
    )

    preamble = compress_preamble(world_pos, domains_dir)
    result = subset_axes(preamble, ["Material Conditions"])

    assert "### Material Conditions" in result
    assert "geography-climate: temperate [seed]" in result
    assert "### Social Forms" not in result
    assert "power-distance" not in result


def test_subset_axes_returns_multiple_domains(tmp_path: Path) -> None:
    """Multiple domain sections can be extracted at once."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir,
        "material-conditions",
        "Material Conditions",
        ["geography-climate"],
    )
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])
    _make_domain_file(domains_dir, "economic-forms", "Economic Forms", ["labor-relations"])

    world_pos = _make_world_pos(
        positions=[
            _seed("geography-climate", "temperate"),
            _seed("power-distance", "high"),
            _seed("labor-relations", "feudal"),
        ]
    )

    preamble = compress_preamble(world_pos, domains_dir)
    result = subset_axes(preamble, ["Material Conditions", "Economic Forms"])

    assert "### Material Conditions" in result
    assert "### Economic Forms" in result
    assert "### Social Forms" not in result


def test_subset_axes_returns_empty_for_unknown_domain(tmp_path: Path) -> None:
    """Returns empty string when none of the requested domains exist."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(positions=[_seed("power-distance", "high")])

    preamble = compress_preamble(world_pos, domains_dir)
    result = subset_axes(preamble, ["Nonexistent Domain"])

    assert result == ""


def test_subset_axes_on_empty_preamble() -> None:
    """Returns empty string for empty preamble input."""
    result = subset_axes("", ["Material Conditions"])
    assert result == ""


# ---------------------------------------------------------------------------
# Tests: build_world_summary
# ---------------------------------------------------------------------------


def test_build_world_summary_complete_structure(tmp_path: Path) -> None:
    """build_world_summary returns dict with all required keys."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(
        genre_slug="folk-horror",
        setting_slug="mccallisters-barn",
        positions=[_seed("power-distance", "high")],
    )

    summary = build_world_summary(world_pos, domains_dir)

    assert summary["genre_slug"] == "folk-horror"
    assert summary["setting_slug"] == "mccallisters-barn"
    assert "compressed_preamble" in summary
    assert isinstance(summary["compressed_preamble"], str)
    assert "axis_count" in summary
    assert "seed_count" in summary


def test_build_world_summary_counts_seeds_correctly(tmp_path: Path) -> None:
    """seed_count and axis_count reflect positions in the world-position dict."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(
        domains_dir,
        "social-forms",
        "Social Forms",
        ["power-distance", "collectivism", "time-orientation"],
    )

    world_pos = _make_world_pos(
        positions=[
            _seed("power-distance", "high"),
            _seed("collectivism", "collective"),
            _inferred("time-orientation", "present"),
        ]
    )

    summary = build_world_summary(world_pos, domains_dir)

    assert summary["axis_count"] == 3
    assert summary["seed_count"] == 2


def test_build_world_summary_compressed_preamble_is_non_empty(tmp_path: Path) -> None:
    """Compressed preamble string is populated when positions exist."""
    domains_dir = tmp_path / "domains"
    _make_domain_file(domains_dir, "social-forms", "Social Forms", ["power-distance"])

    world_pos = _make_world_pos(positions=[_seed("power-distance", "high")])
    summary = build_world_summary(world_pos, domains_dir)

    assert len(summary["compressed_preamble"]) > 0
