# Tome Phase 3b: Social Substrate and Character Elicitation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the world composition pipeline with social substrate elicitation and character generation across a narrative centrality gradient (Q1-Q4).

**Architecture:** Three new Python modules (`elicit_social_substrate.py`, `elicit_characters_mundane.py`, `elicit_characters_significant.py`) following the existing `elicit_places.py`/`elicit_orgs.py` pattern. Each reads predecessor files from the world directory, renders a prompt template, calls `qwen3.5:35b` via Ollama, parses JSON, and writes output. Three new prompt templates. Three new CLI commands in the `tome` group.

**Tech Stack:** Python 3.11+, Click (CLI), httpx (Ollama), Rich (console output), pytest (tests)

**Spec:** `docs/superpowers/specs/2026-03-28-tome-phase-3b-social-substrate-and-character-elicitation-design.md`

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `tools/narrative-data/prompts/tome/social-substrate-elicitation.md` | Prompt template for social substrate generation |
| `tools/narrative-data/prompts/tome/character-mundane-elicitation.md` | Prompt template for Q1-Q2 character generation |
| `tools/narrative-data/prompts/tome/character-significant-elicitation.md` | Prompt template for Q3-Q4 character generation |
| `tools/narrative-data/src/narrative_data/tome/elicit_social_substrate.py` | Social substrate elicitation module |
| `tools/narrative-data/src/narrative_data/tome/elicit_characters_mundane.py` | Q1-Q2 character elicitation module |
| `tools/narrative-data/src/narrative_data/tome/elicit_characters_significant.py` | Q3-Q4 character elicitation module |
| `tools/narrative-data/tests/tome/test_elicit_social_substrate.py` | Tests for social substrate module |
| `tools/narrative-data/tests/tome/test_elicit_characters_mundane.py` | Tests for mundane character module |
| `tools/narrative-data/tests/tome/test_elicit_characters_significant.py` | Tests for significant character module |

### Modified Files

| File | Change |
|------|--------|
| `tools/narrative-data/src/narrative_data/cli.py` | Add 3 new `tome` subcommands |

---

## Task 1: Social Substrate Prompt Template

**Files:**
- Create: `tools/narrative-data/prompts/tome/social-substrate-elicitation.md`

- [ ] **Step 1: Write the social substrate elicitation prompt**

```markdown
You are generating the social substrate for a narrative world. The world has been composed
from a mutual production graph, and places and organizations have already been generated.

Your task is to produce the named social clusters — lineages, factions, kinship groups —
that people in this world are born into, marry across, and escape from. These are not
organizations (which are formal power — what you join). These are identity and belonging —
what you are.

Every person in this world belongs to one of these clusters. Narrative tension lives at
the boundaries between them, not at their centers.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

The following axis positions define this world's narrative coordinates. Seeds are
author-provided; inferred positions were propagated from seeds via the Tome mutual
production graph, with justification and confidence shown.

**Social-forms axes** (primary drivers of social clustering) are marked with ★.
**Economic-forms axes** (secondary — material basis for group formation) are marked with ◆.

{world_preamble}

## Genre Profile

{genre_profile_summary}

## Places Context

{places_context}

## Organizations Context

{orgs_context}

## Anti-Template Instruction

Do NOT produce genre-typical factions. Let the axes determine what social groups this
world must produce. A clan-tribal kinship system with caste-hereditary stratification
produces different clusters than a chosen-elective system with meritocratic stratification.

If you find yourself generating a group because it sounds like it belongs in this genre
(a coven, a street gang, a noble house), stop and ask: do the actual axis positions of
THIS world produce that social form? What does kinship look like when the kinship-system
is {kinship_system_value} and social-stratification is {stratification_value}?

## Task

### Part 1: Social Clusters (3-5)

Generate a flat list of social clusters. These are NOT tiered — every cluster carries
narrative weight. The basis of each cluster is driven by the kinship-system axis:
- clan-tribal → blood-basis clusters (lineages, family names)
- chosen-elective → affiliation-basis clusters (crews, lodges, cohorts)
- institutional-assigned → occupation-basis clusters (work units, professional castes)
- communal-collective → geography-basis clusters (neighborhoods, commons groups)
- nuclear-conjugal → smaller family units with weaker cluster identity

Each cluster needs:
- A name grounded in the world (not generic)
- A basis (blood, occupation, belief, geography, affiliation)
- A hierarchy position (dominant, established, marginal, outsider, contested)
- Relationships to existing organizations
- One sentence of history

### Part 2: Pairwise Relationships

For each pair of clusters, generate one relationship entry with:
- A relationship type (alliance, rivalry, intermarriage, avoidance, dependency, contested-boundary)
- One sentence describing the boundary
- A boundary tension — the specific point of friction or exchange between these groups

The boundary tensions are where characters will be placed. Make them specific and material,
not abstract.

## Output Schema

Output valid JSON: an object with `clusters` array and `relationships` array.
No commentary outside the JSON.

**Cluster object:**

```json
{
  "slug": "kebab-case-identifier",
  "name": "Human-readable cluster name",
  "basis": "<one of: blood, occupation, belief, geography, affiliation>",
  "description": "2-3 sentences. Grounded in axes. What does membership mean, feel like, require?",
  "grounding": {
    "social_axes": ["axis-slug:value"],
    "economic_axes": ["axis-slug:value"],
    "active_edges": ["source →type→ target (weight)"]
  },
  "hierarchy_position": "<one of: dominant, established, marginal, outsider, contested>",
  "org_relationships": ["org-slug:relationship"],
  "history": "One sentence — how long established, what they survived, what they claim."
}
```

**Relationship object:**

```json
{
  "cluster_a": "cluster-slug",
  "cluster_b": "cluster-slug",
  "type": "<one of: alliance, rivalry, intermarriage, avoidance, dependency, contested-boundary>",
  "description": "One sentence describing the boundary.",
  "boundary_tension": "The specific point of friction or exchange. Material, not abstract."
}
```

Field notes:
- `org_relationships`: Directional references to organizations from organizations.json,
  e.g. "parish-council:founding-members", "keepers:excluded", "labor-guild:primary-workforce"
- `hierarchy_position`: Driven by social-stratification axis. The stated hierarchy may not
  match operative position — if the world has a stated/operative gap on social-stratification,
  note which position is stated and which is operative in the description.
- Generate relationships for ALL cluster pairs (for 3 clusters: 3 pairs; for 4: 6 pairs; for 5: 10 pairs).

Output the JSON object only. No preamble, no explanation, no markdown fences.
```

- [ ] **Step 2: Verify template placeholders match what the module will provide**

Placeholders used: `{genre_slug}`, `{setting_slug}`, `{world_preamble}`, `{genre_profile_summary}`, `{places_context}`, `{orgs_context}`, `{kinship_system_value}`, `{stratification_value}`.

- [ ] **Step 3: Commit**

```bash
git add tools/narrative-data/prompts/tome/social-substrate-elicitation.md
git commit -m "feat(tome): add social substrate elicitation prompt template"
```

---

## Task 2: Social Substrate Elicitation Module

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/elicit_social_substrate.py`
- Create: `tools/narrative-data/tests/tome/test_elicit_social_substrate.py`

- [ ] **Step 1: Write the test file**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for social substrate elicitation module."""

import json
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def world_dir(tmp_path: Path) -> Path:
    """Create a minimal world directory with world-position, places, and orgs."""
    world = tmp_path / "narrative-data" / "tome" / "worlds" / "test-world"
    world.mkdir(parents=True)

    world_pos = {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "seed_count": 2,
        "inferred_count": 3,
        "total_positions": 5,
        "positions": [
            {"axis_slug": "kinship-system", "value": "clan-tribal", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "social-stratification", "value": "caste-hereditary", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "community-cohesion", "value": "high", "confidence": 0.8, "source": "inferred", "justification": "kinship-system →produces→ community-cohesion (0.7)"},
            {"axis_slug": "outsider-integration-pattern", "value": "persecutory-expulsive", "confidence": 0.7, "source": "inferred", "justification": "community-cohesion →constrains→ outsider-integration-pattern (0.6)"},
            {"axis_slug": "labor-organization", "value": "household-subsistence", "confidence": 0.6, "source": "inferred", "justification": "kinship-system →produces→ labor-organization (0.5)"},
        ],
    }
    (world / "world-position.json").write_text(json.dumps(world_pos))

    places = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "places": [
            {"slug": "the-market", "name": "The Market", "tier": 2, "place_type": "gathering-place", "description": "A dusty market square."},
            {"slug": "the-hall", "name": "The Hall", "tier": 1, "place_type": "infrastructure", "description": "The council hall.", "spatial_role": "center"},
        ],
    }
    (world / "places.json").write_text(json.dumps(places))

    orgs = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "organizations": [
            {"slug": "parish-council", "name": "Parish Council", "tier": 2, "org_type": "governance", "description": "Local governance."},
            {"slug": "the-keepers", "name": "The Keepers", "tier": 1, "org_type": "religious", "description": "Knowledge suppressors."},
        ],
    }
    (world / "organizations.json").write_text(json.dumps(orgs))

    return world


# ---------------------------------------------------------------------------
# Context loading tests
# ---------------------------------------------------------------------------


class TestLoadOrgs:
    def test_loads_org_list(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import _load_orgs

        orgs = _load_orgs(world_dir)
        assert len(orgs) == 2
        assert orgs[0]["slug"] == "parish-council"

    def test_raises_when_missing(self, tmp_path: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import _load_orgs

        empty = tmp_path / "empty"
        empty.mkdir()
        with pytest.raises(FileNotFoundError, match="organizations.json"):
            _load_orgs(empty)


class TestBuildOrgsContext:
    def test_formats_orgs_as_markdown(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs

        orgs = _load_orgs(world_dir)
        ctx = _build_orgs_context(orgs)
        assert "Parish Council" in ctx
        assert "The Keepers" in ctx
        assert "parish-council" in ctx


class TestBuildSubstratePrompt:
    def test_substitutes_all_placeholders(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_social_substrate import (
            _build_orgs_context,
            _build_prompt,
            _load_orgs,
        )
        from narrative_data.tome.elicit_places import (
            _build_genre_profile_summary,
            _build_world_preamble,
            _load_world_position,
        )

        world_pos = _load_world_position(world_dir)
        orgs = _load_orgs(world_dir)

        # Minimal template with all placeholders
        template = (
            "{genre_slug} {setting_slug} {world_preamble} "
            "{genre_profile_summary} {places_context} {orgs_context} "
            "{kinship_system_value} {stratification_value}"
        )
        prompt = _build_prompt(
            template=template,
            world_pos=world_pos,
            genre_profile=None,
            places=[{"slug": "the-market", "name": "The Market", "spatial_role": "center", "description": "A market."}],
            orgs=orgs,
            settings_context="",
        )
        assert "folk-horror" in prompt
        assert "test-village" in prompt
        assert "clan-tribal" in prompt
        assert "caste-hereditary" in prompt
        assert "{" not in prompt  # No unsubstituted placeholders


class TestParseSubstrateResponse:
    def test_parses_valid_json_object(self) -> None:
        from narrative_data.tome.elicit_social_substrate import _parse_substrate_response

        response = json.dumps({
            "clusters": [{"slug": "the-morrows", "name": "The Morrows"}],
            "relationships": [{"cluster_a": "the-morrows", "cluster_b": "the-others", "type": "rivalry"}],
        })
        result = _parse_substrate_response(response)
        assert "clusters" in result
        assert len(result["clusters"]) == 1

    def test_parses_json_in_code_fence(self) -> None:
        from narrative_data.tome.elicit_social_substrate import _parse_substrate_response

        response = '```json\n{"clusters": [{"slug": "a"}], "relationships": []}\n```'
        result = _parse_substrate_response(response)
        assert len(result["clusters"]) == 1

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.elicit_social_substrate import _parse_substrate_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_substrate_response("This is not JSON at all")
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_elicit_social_substrate.py -v`
Expected: FAIL — module does not exist yet.

- [ ] **Step 3: Write the elicitation module**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit social substrate (lineages, factions, kinship groups) for a Tome world.

Reads the world-position.json, places.json, and organizations.json produced by
prior pipeline steps, builds a structured prompt from axis positions, genre
profile, and entity context, calls qwen3.5:35b, parses the JSON response, and
writes social-substrate.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-social-substrate --world-slug <slug>
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.tome.elicit_places import (
    _build_genre_profile_summary,
    _build_settings_context,
    _build_world_preamble,
    _load_world_position,
)
from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_SUBSTRATE_TIMEOUT = 600.0
_SUBSTRATE_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# Organizations loading
# ---------------------------------------------------------------------------


def _load_orgs(world_dir: Path) -> list[dict[str, Any]]:
    """Read organizations.json from the world directory and return the orgs list.

    Args:
        world_dir: Path to the world directory containing organizations.json.

    Returns:
        List of organization dicts.

    Raises:
        FileNotFoundError: If organizations.json does not exist.
        ValueError: If the file cannot be parsed or lacks an organizations array.
    """
    orgs_path = world_dir / "organizations.json"
    if not orgs_path.exists():
        raise FileNotFoundError(
            f"organizations.json not found at {orgs_path}. "
            "Run 'tome elicit-orgs' first."
        )
    try:
        data = json.loads(orgs_path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse organizations.json: {exc}") from exc

    orgs = data.get("organizations")
    if not isinstance(orgs, list):
        raise ValueError(
            f"organizations.json does not contain an 'organizations' array at {orgs_path}."
        )
    return orgs


# ---------------------------------------------------------------------------
# Context construction
# ---------------------------------------------------------------------------


def _build_orgs_context(orgs: list[dict[str, Any]]) -> str:
    """Summarize organizations as a markdown list for the prompt.

    Args:
        orgs: List of organization dicts from organizations.json.

    Returns:
        Markdown-formatted organizations context string.
    """
    if not orgs:
        return "No organizations generated for this world yet."

    lines: list[str] = []
    for org in orgs:
        name = org.get("name", org.get("slug", "Unknown"))
        slug = org.get("slug", "")
        org_type = org.get("org_type", "unknown")
        description = org.get("description", "")
        if len(description) > 200:
            description = description[:197] + "..."
        line = f"- **{name}** `{slug}` ({org_type}): {description}"

        # Include stated/operative gap if present
        gap = org.get("stated_vs_operative")
        if gap and isinstance(gap, dict):
            stated = gap.get("stated", "")
            operative = gap.get("operative", "")
            if stated and operative:
                line += f"\n  - Stated: {stated[:150]}"
                line += f"\n  - Operative: {operative[:150]}"

        lines.append(line)

    return "\n".join(lines)


def _extract_axis_value(
    positions: list[dict[str, Any]], axis_slug: str
) -> str:
    """Extract the value of a specific axis from world positions.

    Args:
        positions: List of position dicts from world-position.json.
        axis_slug: The axis slug to look up.

    Returns:
        The axis value string, or "unset" if not found.
    """
    for p in positions:
        if p.get("axis_slug") == axis_slug:
            return str(p.get("value", "unset"))
    return "unset"


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    places: list[dict[str, Any]],
    orgs: list[dict[str, Any]],
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the social-substrate-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts from places.json.
        orgs: List of organization dicts from organizations.json.
        settings_context: Formatted genre settings archetypes.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    positions = world_pos.get("positions", [])
    world_preamble = _build_world_preamble(world_pos)
    genre_summary = _build_genre_profile_summary(genre_profile)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)

    kinship_value = _extract_axis_value(positions, "kinship-system")
    stratification_value = _extract_axis_value(positions, "social-stratification")

    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", world_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{kinship_system_value}", kinship_value)
        .replace("{stratification_value}", stratification_value)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_substrate_response(response: str) -> dict[str, Any]:
    """Parse LLM response as a JSON object with clusters and relationships.

    Three strategies are attempted in order:
    1. Direct json.loads on the stripped text.
    2. Extract from a markdown ```json ... ``` code fence.
    3. Find the outermost { ... } object boundaries and parse that.

    Args:
        response: Raw LLM response text.

    Returns:
        Dict with 'clusters' and 'relationships' keys.

    Raises:
        ValueError: If all three strategies fail.
    """
    text = response.strip()

    def _try_parse(s: str) -> dict[str, Any] | None:
        try:
            result = json.loads(s)
            if isinstance(result, dict) and "clusters" in result:
                return result
        except json.JSONDecodeError:
            pass
        return None

    # Strategy 1: direct parse
    parsed = _try_parse(text)
    if parsed:
        return parsed

    # Strategy 2: extract from ```json ... ``` fence
    fence_match = re.search(r"```json\s*(.*?)\s*```", text, re.DOTALL)
    if fence_match:
        parsed = _try_parse(fence_match.group(1))
        if parsed:
            return parsed

    # Also try plain ``` fence
    plain_fence_match = re.search(r"```\s*(\{.*?\})\s*```", text, re.DOTALL)
    if plain_fence_match:
        parsed = _try_parse(plain_fence_match.group(1))
        if parsed:
            return parsed

    # Strategy 3: find outermost { ... } object
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        parsed = _try_parse(text[start : end + 1])
        if parsed:
            return parsed

    raise ValueError(
        "Could not parse LLM response as a JSON object with 'clusters'. "
        f"Response began with: {text[:200]!r}"
    )


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_social_substrate(data_path: Path, world_slug: str) -> None:
    """Elicit social substrate (lineages, factions, kinship groups) for a Tome world.

    Reads world-position.json, places.json, and organizations.json, builds a
    structured prompt, calls the elicitation model, parses the JSON response,
    and writes social-substrate.json to the world directory.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier — must match a directory under
            {data_path}/narrative-data/tome/worlds/.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "social-substrate-elicitation.md"

    # 1. Load world position
    console.print(f"[bold]Loading world position for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"positions=[cyan]{world_pos.get('total_positions', 0)}[/cyan]"
    )

    # 2. Load places and organizations
    console.print("[bold]Loading places and organizations…[/bold]")
    try:
        places = _load_places(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    try:
        orgs = _load_orgs(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    console.print(f"  Loaded [green]{len(places)}[/green] place(s), [green]{len(orgs)}[/green] org(s)")

    # 3. Load prompt template
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # 4. Build prompt
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    prompt = _build_prompt(template, world_pos, genre_profile, places, orgs, settings_context)
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # 5. Call LLM
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_SUBSTRATE_TIMEOUT}s, temperature={_SUBSTRATE_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_SUBSTRATE_TIMEOUT,
        temperature=_SUBSTRATE_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # 6. Parse response
    console.print("[bold]Parsing response…[/bold]")
    try:
        substrate = _parse_substrate_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    clusters = substrate.get("clusters", [])
    relationships = substrate.get("relationships", [])
    console.print(
        f"  Parsed [green]{len(clusters)}[/green] cluster(s), "
        f"[green]{len(relationships)}[/green] relationship(s)"
    )

    # 7. Write social-substrate.json
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "cluster_count": len(clusters),
        "relationship_count": len(relationships),
        "clusters": clusters,
        "relationships": relationships,
    }

    output_path = world_dir / "social-substrate.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # 8. Summary
    console.print()
    console.print(f"[bold]Social substrate generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for cluster in clusters:
        slug = cluster.get("slug", "?")
        name = cluster.get("name", "?")
        basis = cluster.get("basis", "?")
        position = cluster.get("hierarchy_position", "?")
        console.print(
            f"  [green]✓[/green] [bold]{name}[/bold] "
            f"[dim]({slug}, {basis}, {position})[/dim]"
        )
    if relationships:
        console.print()
        console.print("[bold]Relationships:[/bold]")
        for rel in relationships:
            a = rel.get("cluster_a", "?")
            b = rel.get("cluster_b", "?")
            rtype = rel.get("type", "?")
            console.print(f"  [dim]{a} ↔ {b}: {rtype}[/dim]")
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_elicit_social_substrate.py -v`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/tome/elicit_social_substrate.py tools/narrative-data/tests/tome/test_elicit_social_substrate.py
git commit -m "feat(tome): add social substrate elicitation module and tests"
```

---

## Task 3: Mundane Character Prompt Template

**Files:**
- Create: `tools/narrative-data/prompts/tome/character-mundane-elicitation.md`

- [ ] **Step 1: Write the mundane character elicitation prompt**

```markdown
You are generating the people who inhabit a narrative world. The world has been composed
from a mutual production graph, and places, organizations, and social substrate have
already been generated.

Your task is to produce background and community characters — the people who make this
world feel inhabited. These are not heroes or villains. They are the mail carrier, the
smallholder, the guard, the scribe. They belong to social clusters, they work in places,
they have neighbors. Their ordinariness is what makes the extraordinary legible.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{world_preamble}

## Genre Profile

{genre_profile_summary}

## Places Context

{places_context}

## Organizations Context

{orgs_context}

## Social Substrate

These are the social clusters — lineages, factions, kinship groups — that people in
this world belong to. Every character you generate must belong to one of these clusters.

{social_substrate_context}

## Anti-Template Instruction

Do not generate characters to represent archetypes. Generate people who live in this world,
do this work, and belong to these groups. A mail carrier in a clan-tribal village with
caste-hereditary stratification is a different person than a mail carrier in a chosen-elective
community with meritocratic credentialing.

If you find yourself generating a character because this genre typically has one (the wise
elder, the suspicious stranger, the troubled youth), stop and ask: does THIS world at THIS
set of coordinates, with THESE social clusters and THESE organizations, produce this person?
What work do they do? Who do they answer to? What cluster were they born into?

## Task

Generate characters in two blocks. Generate Q1 FIRST.

### Q1 — Background Characters (4-6)

People who exist because the world requires their labor and presence. Each must have:
- A name (grounded in their cluster — use naming patterns consistent with the world)
- A role (their actual job or function)
- One sentence describing who they are in this world
- One place association (where they work or live)
- One cluster membership (which social group they belong to)
- One relational seed (one directional relationship to another entity)

No archetype. No tension. No communicability profile. These are people doing jobs.

### Q2 — Community Characters (3-4)

People who are slightly more visible in the community — they occupy positions with some
social weight, have a few relationships, and carry one tension or desire that arises
from their position in the world.

Each must have:
- A name and role
- 2-3 sentences of description, specific to this world's material conditions
- Archetype resonance — name the genre archetype this character most resembles, but do
  not force the fit. This is a soft echo, not a mapping.
- 1-2 place associations
- One cluster membership
- 2-3 relational seeds (directional relationships to other entities — places, orgs, Q1 characters)
- One tension or desire that arises from their position in the world, NOT from genre convention

## Output Schema

Output valid JSON: a single array containing ALL characters (Q1 first, then Q2).
No commentary outside the JSON.

**Q1 character object:**

```json
{
  "centrality": "Q1",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their job or function",
  "description": "One sentence. Grounded in a place and a function.",
  "place_association": "place-slug",
  "cluster_membership": "cluster-slug",
  "relational_seed": "relation:target-slug"
}
```

**Q2 character object:**

```json
{
  "centrality": "Q2",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their job or function",
  "description": "2-3 sentences. Specific to this world's material conditions.",
  "archetype_resonance": "The Archetype Name",
  "place_associations": ["place-slug", "place-slug"],
  "cluster_membership": "cluster-slug",
  "relational_seeds": [
    "relation:target-slug",
    "relation:target-slug"
  ],
  "tension": "One sentence. Arises from world-position, not genre convention."
}
```

Field notes:
- `relational_seed` / `relational_seeds`: Directional relationships using entity slugs,
  e.g. "delivers-to:parish-council", "works-at:the-market", "neighbors-with:elda-the-carrier"
- `archetype_resonance`: Name the bedrock archetype this character most resembles. If no
  archetype fits naturally, write "none" — do not force it.
- `cluster_membership`: Must reference a slug from the social substrate.
- Q2 characters should reference at least one Q1 character in their relational_seeds
  when it makes sense (they share a workplace, are neighbors, have a transactional relationship).

Output the JSON array only. No preamble, no explanation, no markdown fences.
```

- [ ] **Step 2: Commit**

```bash
git add tools/narrative-data/prompts/tome/character-mundane-elicitation.md
git commit -m "feat(tome): add mundane character (Q1-Q2) elicitation prompt template"
```

---

## Task 4: Mundane Character Elicitation Module

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/elicit_characters_mundane.py`
- Create: `tools/narrative-data/tests/tome/test_elicit_characters_mundane.py`

- [ ] **Step 1: Write the test file**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for mundane character (Q1-Q2) elicitation module."""

import json
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def world_dir(tmp_path: Path) -> Path:
    """Create a minimal world directory with all prerequisite files."""
    world = tmp_path / "narrative-data" / "tome" / "worlds" / "test-world"
    world.mkdir(parents=True)

    world_pos = {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "seed_count": 2,
        "inferred_count": 1,
        "total_positions": 3,
        "positions": [
            {"axis_slug": "kinship-system", "value": "clan-tribal", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "social-stratification", "value": "caste-hereditary", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "community-cohesion", "value": "high", "confidence": 0.8, "source": "inferred"},
        ],
    }
    (world / "world-position.json").write_text(json.dumps(world_pos))

    places = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "places": [
            {"slug": "the-market", "name": "The Market", "tier": 2, "place_type": "gathering-place", "description": "A dusty market.", "spatial_role": "center"},
        ],
    }
    (world / "places.json").write_text(json.dumps(places))

    orgs = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "organizations": [
            {"slug": "parish-council", "name": "Parish Council", "tier": 2, "org_type": "governance", "description": "Local governance."},
        ],
    }
    (world / "organizations.json").write_text(json.dumps(orgs))

    substrate = {
        "world_slug": "test-world",
        "genre_slug": "folk-horror",
        "clusters": [
            {"slug": "the-morrows", "name": "The Morrows", "basis": "blood", "hierarchy_position": "dominant"},
            {"slug": "the-hallodays", "name": "The Hallodays", "basis": "blood", "hierarchy_position": "established"},
        ],
        "relationships": [
            {"cluster_a": "the-morrows", "cluster_b": "the-hallodays", "type": "intermarriage-with-tension", "boundary_tension": "Land flows through Morrow blood but Halloday labor works it."},
        ],
    }
    (world / "social-substrate.json").write_text(json.dumps(substrate))

    return world


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestLoadSocialSubstrate:
    def test_loads_substrate(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import _load_social_substrate

        substrate = _load_social_substrate(world_dir)
        assert "clusters" in substrate
        assert len(substrate["clusters"]) == 2

    def test_raises_when_missing(self, tmp_path: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import _load_social_substrate

        empty = tmp_path / "empty"
        empty.mkdir()
        with pytest.raises(FileNotFoundError, match="social-substrate.json"):
            _load_social_substrate(empty)


class TestBuildSubstrateContext:
    def test_formats_clusters_and_relationships(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import (
            _build_social_substrate_context,
            _load_social_substrate,
        )

        substrate = _load_social_substrate(world_dir)
        ctx = _build_social_substrate_context(substrate)
        assert "The Morrows" in ctx
        assert "The Hallodays" in ctx
        assert "intermarriage" in ctx
        assert "Land flows" in ctx


class TestBuildMundanePrompt:
    def test_substitutes_all_placeholders(self, world_dir: Path) -> None:
        from narrative_data.tome.elicit_characters_mundane import (
            _build_prompt,
            _build_social_substrate_context,
            _load_social_substrate,
        )
        from narrative_data.tome.elicit_places import (
            _build_genre_profile_summary,
            _build_world_preamble,
            _load_world_position,
        )
        from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
        from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs

        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)

        template = (
            "{genre_slug} {setting_slug} {world_preamble} "
            "{genre_profile_summary} {places_context} {orgs_context} "
            "{social_substrate_context}"
        )
        prompt = _build_prompt(
            template=template,
            world_pos=world_pos,
            genre_profile=None,
            places=places,
            orgs=orgs,
            substrate=substrate,
            settings_context="",
        )
        assert "folk-horror" in prompt
        assert "{" not in prompt


class TestParseMundaneResponse:
    def test_parses_valid_array(self) -> None:
        from narrative_data.tome.elicit_characters_mundane import _parse_characters_response

        response = json.dumps([
            {"centrality": "Q1", "slug": "elda", "name": "Elda"},
            {"centrality": "Q2", "slug": "gareth", "name": "Gareth"},
        ])
        result = _parse_characters_response(response)
        assert len(result) == 2

    def test_raises_on_garbage(self) -> None:
        from narrative_data.tome.elicit_characters_mundane import _parse_characters_response

        with pytest.raises(ValueError, match="Could not parse"):
            _parse_characters_response("not json")
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_elicit_characters_mundane.py -v`
Expected: FAIL — module does not exist yet.

- [ ] **Step 3: Write the elicitation module**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit mundane characters (Q1 background + Q2 community) for a Tome world.

Reads world-position.json, places.json, organizations.json, and social-substrate.json,
builds a structured prompt, calls qwen3.5:35b, parses the JSON response, and writes
characters-mundane.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-characters-mundane --world-slug <slug>
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.tome.elicit_places import (
    _build_genre_profile_summary,
    _build_settings_context,
    _build_world_preamble,
    _load_world_position,
    _parse_places_response,
)
from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_CHAR_TIMEOUT = 600.0
_CHAR_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# Social substrate loading
# ---------------------------------------------------------------------------


def _load_social_substrate(world_dir: Path) -> dict[str, Any]:
    """Read social-substrate.json from the world directory.

    Args:
        world_dir: Path to the world directory containing social-substrate.json.

    Returns:
        Parsed social substrate dict with 'clusters' and 'relationships'.

    Raises:
        FileNotFoundError: If social-substrate.json does not exist.
        ValueError: If the file cannot be parsed or lacks a clusters array.
    """
    path = world_dir / "social-substrate.json"
    if not path.exists():
        raise FileNotFoundError(
            f"social-substrate.json not found at {path}. "
            "Run 'tome elicit-social-substrate' first."
        )
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse social-substrate.json: {exc}") from exc

    if not isinstance(data.get("clusters"), list):
        raise ValueError(
            f"social-substrate.json does not contain a 'clusters' array at {path}."
        )
    return data


# ---------------------------------------------------------------------------
# Context construction
# ---------------------------------------------------------------------------


def _build_social_substrate_context(substrate: dict[str, Any]) -> str:
    """Format the social substrate as markdown for the character prompt.

    Args:
        substrate: Parsed social-substrate.json dict.

    Returns:
        Markdown-formatted social substrate context.
    """
    clusters = substrate.get("clusters", [])
    relationships = substrate.get("relationships", [])

    if not clusters:
        return "No social substrate generated for this world yet."

    lines: list[str] = []
    lines.append("### Clusters")
    lines.append("")
    for c in clusters:
        name = c.get("name", c.get("slug", "Unknown"))
        slug = c.get("slug", "")
        basis = c.get("basis", "?")
        position = c.get("hierarchy_position", "?")
        description = c.get("description", "")
        history = c.get("history", "")
        lines.append(f"- **{name}** `{slug}` (basis: {basis}, position: {position})")
        if description:
            lines.append(f"  {description[:250]}")
        if history:
            lines.append(f"  History: {history[:200]}")

        org_rels = c.get("org_relationships", [])
        if org_rels:
            lines.append(f"  Org ties: {', '.join(str(r) for r in org_rels)}")
        lines.append("")

    if relationships:
        lines.append("### Inter-Cluster Relationships")
        lines.append("")
        for r in relationships:
            a = r.get("cluster_a", "?")
            b = r.get("cluster_b", "?")
            rtype = r.get("type", "?")
            tension = r.get("boundary_tension", "")
            lines.append(f"- **{a} ↔ {b}** ({rtype})")
            if tension:
                lines.append(f"  Boundary tension: {tension}")
            lines.append("")

    return "\n".join(lines).rstrip()


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    places: list[dict[str, Any]],
    orgs: list[dict[str, Any]],
    substrate: dict[str, Any],
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the character-mundane-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts from places.json.
        orgs: List of organization dicts from organizations.json.
        substrate: Parsed social-substrate.json dict.
        settings_context: Formatted genre settings archetypes.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    world_preamble = _build_world_preamble(world_pos)
    genre_summary = _build_genre_profile_summary(genre_profile)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)
    substrate_context = _build_social_substrate_context(substrate)

    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", world_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{social_substrate_context}", substrate_context)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_characters_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of character objects.

    Uses the same three-strategy approach as place/org parsing.

    Args:
        response: Raw LLM response text.

    Returns:
        List of character dicts.

    Raises:
        ValueError: If all strategies fail.
    """
    # Reuse the proven array-parsing logic from elicit_places
    return _parse_places_response(response)


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_characters_mundane(data_path: Path, world_slug: str) -> None:
    """Elicit mundane characters (Q1 background + Q2 community) for a Tome world.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "character-mundane-elicitation.md"

    # 1. Load all prerequisite data
    console.print(f"[bold]Loading world data for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"places=[cyan]{len(places)}[/cyan]  "
        f"orgs=[cyan]{len(orgs)}[/cyan]  "
        f"clusters=[cyan]{len(substrate.get('clusters', []))}[/cyan]"
    )

    # 2. Load prompt template
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # 3. Build prompt
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    prompt = _build_prompt(template, world_pos, genre_profile, places, orgs, substrate, settings_context)
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # 4. Call LLM
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_CHAR_TIMEOUT}s, temperature={_CHAR_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_CHAR_TIMEOUT,
        temperature=_CHAR_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # 5. Parse response
    console.print("[bold]Parsing response…[/bold]")
    try:
        characters = _parse_characters_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    q1 = [c for c in characters if c.get("centrality") == "Q1"]
    q2 = [c for c in characters if c.get("centrality") == "Q2"]
    console.print(f"  Parsed [green]{len(q1)}[/green] Q1 + [green]{len(q2)}[/green] Q2 character(s)")

    # 6. Write characters-mundane.json
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "q1_count": len(q1),
        "q2_count": len(q2),
        "total_count": len(characters),
        "characters": characters,
    }

    output_path = world_dir / "characters-mundane.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # 7. Summary
    console.print()
    console.print(f"[bold]Mundane characters generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for char in characters:
        slug = char.get("slug", "?")
        name = char.get("name", "?")
        centrality = char.get("centrality", "?")
        role = char.get("role", "?")
        cluster = char.get("cluster_membership", "?")
        console.print(
            f"  [green]✓[/green] [{centrality}] [bold]{name}[/bold] "
            f"[dim]({slug}, {role}, {cluster})[/dim]"
        )
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_elicit_characters_mundane.py -v`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/tome/elicit_characters_mundane.py tools/narrative-data/tests/tome/test_elicit_characters_mundane.py
git commit -m "feat(tome): add mundane character (Q1-Q2) elicitation module and tests"
```

---

## Task 5: Significant Character Prompt Template

**Files:**
- Create: `tools/narrative-data/prompts/tome/character-significant-elicitation.md`

- [ ] **Step 1: Write the significant character elicitation prompt**

```markdown
You are generating the narratively significant characters for a narrative world. The world
has been fully composed: places, organizations, social substrate, and mundane characters
already exist. You are now generating the people who carry narrative tension and drive story.

These characters inhabit the world — they do not merely appear in it. Their agency is
defined by, enabled by, and constrained by their position in the social web. Ascending
narrative centrality means deeper characterization AND deeper entanglement. The more a
character can do, the more the world can do to them.

## World Identity

Genre: {genre_slug}
Setting: {setting_slug}

## World Position

{world_preamble}

## Genre Profile

{genre_profile_summary}

## Places Context

{places_context}

## Organizations Context

{orgs_context}

## Social Substrate

{social_substrate_context}

## Mundane Characters (Q1-Q2)

These are the people already inhabiting this world. Q3-Q4 characters do not float above
them — they are embedded among them. Each significant character's relational_seeds must
reference at least one mundane character by slug.

{mundane_characters_context}

## Bedrock Archetypes

These are the structural character patterns that this genre produces. They represent
recurring tensions and structural positions, not personality templates.

Use them as lenses, not as a menu. A character may resonate with one archetype's tension
while occupying another's structural position. Not every archetype needs to appear. The
world-position and social substrate determine which patterns are structurally necessary —
an archetype that doesn't fit the material conditions of THIS world should not be forced.

{archetypes_context}

## Archetype Dynamics

These are characteristic relationship patterns between archetypes in this genre.
Use them to inform how significant characters relate to each other.

{archetype_dynamics_context}

## Anti-Template Instruction

Each character must be situated at a specific social substrate boundary. Their archetype
is how they cope with that boundary position, not a personality assigned from a catalog.

Do not generate characters to fill archetype slots. Generate people whose position in
the social web produces the tensions that archetypes describe. The Earnest Warden is not
a personality type — it is what happens when someone with genuine care for the community
is structurally positioned to enforce its most harmful norms.

## Task

Generate characters in two blocks. Generate Q3 FIRST.

### Q3 — Tension-Bearing Characters (2-3)

Characters who inhabit the gap between stated and operative reality. They live at social
substrate boundaries — caught between clusters, between loyalty and conscience, between
what they're supposed to be and what the world requires them to do.

Each must have:
- Name, role, 3-4 sentence description
- Full archetype mapping: primary + shadow + genre_inflection (how the archetype expresses
  in THIS character at THIS world-position)
- Place associations (1-2)
- Structured cluster_membership: primary cluster, boundary_with another cluster,
  boundary_tension specific to this character
- stated_operative_gap: what they claim vs. what they actually do
- Relational seeds (4+) including at least one Q1-Q2 character slug
- Arc-scale goal

### Q4 — Scene-Driving Characters (1-2)

Characters who are the genre expressing itself through a specific person in specific
material conditions at a specific social position. Everything Q3 has, plus:
- personality_profile: 7-axis numeric (warmth, authority, openness, interiority, stability,
  agency, morality) each 0.0-1.0. Informed by the bedrock archetype profile but adjusted
  for this character's world-position and social entanglement.
- communicability: surface_area (0.0-1.0), translation_friction (0.0-1.0),
  timescale (momentary|biographical|generational|geological|primordial),
  atmospheric_palette (sensory string)
- Multi-scale goals: existential (what they'd die for), arc (what changes over the story),
  scene (what they want right now)
- Relational seeds (5+) with multiple references to Q1-Q2 and Q3 characters

## Output Schema

Output valid JSON: a single array containing ALL characters (Q3 first, then Q4).
No commentary outside the JSON.

**Q3 character object:**

```json
{
  "centrality": "Q3",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their position or function",
  "description": "3-4 sentences. Narrative-rich, specific to world position.",
  "archetype": {
    "primary": "The Archetype Name",
    "shadow": "The Shadow Archetype Name",
    "genre_inflection": "How this archetype expresses in THIS character at THIS position."
  },
  "place_associations": ["place-slug", "place-slug"],
  "cluster_membership": {
    "primary": "cluster-slug",
    "boundary_with": "other-cluster-slug",
    "boundary_tension": "What makes this boundary productive for this character."
  },
  "stated_operative_gap": {
    "stated": "What they claim to do or be.",
    "operative": "What they actually do. Who benefits. What it costs them."
  },
  "relational_seeds": [
    "relation:target-slug",
    "relation:target-slug",
    "relation:target-slug",
    "relation:target-slug"
  ],
  "goals": {
    "arc": "What changes over the story for this character."
  }
}
```

**Q4 character object:**

```json
{
  "centrality": "Q4",
  "slug": "kebab-case-identifier",
  "name": "Character Name",
  "role": "their position or function",
  "description": "4-6 sentences. The genre expressing itself through a specific person.",
  "archetype": {
    "primary": "The Archetype Name",
    "shadow": "The Shadow Archetype Name",
    "genre_inflection": "How this archetype expresses in THIS character at THIS position."
  },
  "personality_profile": {
    "warmth": 0.0,
    "authority": 0.0,
    "openness": 0.0,
    "interiority": 0.0,
    "stability": 0.0,
    "agency": 0.0,
    "morality": 0.0
  },
  "place_associations": ["place-slug:role", "place-slug:role"],
  "cluster_membership": {
    "primary": "cluster-slug",
    "boundary_with": "other-cluster-slug",
    "boundary_tension": "What makes this boundary load-bearing for this character."
  },
  "stated_operative_gap": {
    "stated": "What they claim to do or be.",
    "operative": "What they actually do. The network requires it."
  },
  "relational_seeds": [
    "relation:target-slug",
    "relation:target-slug",
    "relation:target-slug",
    "relation:target-slug",
    "relation:target-slug"
  ],
  "goals": {
    "existential": "What they would die for or cannot live without.",
    "arc": "What changes over the story.",
    "scene": "What they want right now, today, this moment."
  },
  "communicability": {
    "surface_area": 0.0,
    "translation_friction": 0.0,
    "timescale": "biographical",
    "atmospheric_palette": "Sensory string: textures, sounds, smells that follow this person."
  }
}
```

Field notes:
- `personality_profile`: All values 0.0-1.0. Use the bedrock archetype's profile as a
  starting point, then adjust for this character's specific world-position and entanglement.
  A Warden in a mining community may have lower warmth and higher authority than the
  archetype baseline.
- `communicability.surface_area`: How much of this character is available to narrative
  interaction (0.0 = guarded/opaque, 1.0 = fully expressive)
- `communicability.translation_friction`: How difficult for the Narrator to render this
  character's inner state (0.0 = immediately legible, 1.0 = deeply alien)
- `relational_seeds`: Must include at least one Q1-Q2 character slug and at least one
  organization or social cluster slug. Format: "relation:target-slug"
- `cluster_membership.boundary_with`: Must reference a different cluster from the social
  substrate. This is where the character's narrative tension lives.

Output the JSON array only. No preamble, no explanation, no markdown fences.
```

- [ ] **Step 2: Commit**

```bash
git add tools/narrative-data/prompts/tome/character-significant-elicitation.md
git commit -m "feat(tome): add significant character (Q3-Q4) elicitation prompt template"
```

---

## Task 6: Significant Character Elicitation Module

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/elicit_characters_significant.py`
- Create: `tools/narrative-data/tests/tome/test_elicit_characters_significant.py`

- [ ] **Step 1: Write the test file**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for significant character (Q3-Q4) elicitation module."""

import json
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def data_root(tmp_path: Path) -> Path:
    """Create a full data directory with discovery corpus and world data."""
    # World directory
    world = tmp_path / "narrative-data" / "tome" / "worlds" / "test-world"
    world.mkdir(parents=True)

    world_pos = {
        "genre_slug": "folk-horror",
        "setting_slug": "test-village",
        "seed_count": 2,
        "inferred_count": 1,
        "total_positions": 3,
        "positions": [
            {"axis_slug": "kinship-system", "value": "clan-tribal", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "social-stratification", "value": "caste-hereditary", "confidence": 1.0, "source": "seed"},
            {"axis_slug": "community-cohesion", "value": "high", "confidence": 0.8, "source": "inferred"},
        ],
    }
    (world / "world-position.json").write_text(json.dumps(world_pos))

    places = {"world_slug": "test-world", "genre_slug": "folk-horror", "places": [
        {"slug": "the-hall", "name": "The Hall", "tier": 1, "place_type": "infrastructure", "description": "Council hall.", "spatial_role": "center"},
    ]}
    (world / "places.json").write_text(json.dumps(places))

    orgs = {"world_slug": "test-world", "genre_slug": "folk-horror", "organizations": [
        {"slug": "parish-council", "name": "Parish Council", "tier": 2, "org_type": "governance", "description": "Local governance."},
    ]}
    (world / "organizations.json").write_text(json.dumps(orgs))

    substrate = {"world_slug": "test-world", "genre_slug": "folk-horror",
        "clusters": [
            {"slug": "the-morrows", "name": "The Morrows", "basis": "blood", "hierarchy_position": "dominant"},
            {"slug": "the-hallodays", "name": "The Hallodays", "basis": "blood", "hierarchy_position": "established"},
        ],
        "relationships": [
            {"cluster_a": "the-morrows", "cluster_b": "the-hallodays", "type": "intermarriage-with-tension", "boundary_tension": "Land vs labor."},
        ],
    }
    (world / "social-substrate.json").write_text(json.dumps(substrate))

    mundane = {"world_slug": "test-world", "genre_slug": "folk-horror", "characters": [
        {"centrality": "Q1", "slug": "elda-farrow", "name": "Elda Farrow", "role": "mail carrier"},
        {"centrality": "Q2", "slug": "gareth-morrow", "name": "Gareth Morrow", "role": "tithe collector"},
    ]}
    (world / "characters-mundane.json").write_text(json.dumps(mundane))

    # Bedrock archetype data
    arch_dir = tmp_path / "narrative-data" / "discovery" / "archetypes" / "folk-horror"
    arch_dir.mkdir(parents=True)
    archetype = {
        "canonical_name": "The Earnest Warden",
        "genre_slug": "folk-horror",
        "personality_profile": {"warmth": 0.8, "authority": 0.7, "openness": 0.3, "interiority": 0.6, "stability": 0.8, "agency": 0.7, "morality": 0.5},
        "distinguishing_tension": "Genuine care vs. structural complicity",
        "structural_necessity": "Community enforcement through warmth",
    }
    (arch_dir / "the-earnest-warden.json").write_text(json.dumps(archetype))

    # Archetype dynamics data
    dyn_dir = tmp_path / "narrative-data" / "discovery" / "archetype-dynamics" / "folk-horror"
    dyn_dir.mkdir(parents=True)
    dynamic = {
        "pairing_name": "The Warmth That Prepares the Sacrifice",
        "archetype_a": "The Unwilling Vessel",
        "archetype_b": "The Earnest Warden",
        "edge_properties": {"edge_type": "Trust-textured, binding"},
    }
    (dyn_dir / "vessel-warden.json").write_text(json.dumps(dynamic))

    return tmp_path


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestLoadMundaneCharacters:
    def test_loads_characters(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_mundane_characters

        world_dir = data_root / "narrative-data" / "tome" / "worlds" / "test-world"
        chars = _load_mundane_characters(world_dir)
        assert len(chars) == 2

    def test_raises_when_missing(self, tmp_path: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_mundane_characters

        empty = tmp_path / "empty"
        empty.mkdir()
        with pytest.raises(FileNotFoundError, match="characters-mundane.json"):
            _load_mundane_characters(empty)


class TestLoadArchetypes:
    def test_loads_genre_archetypes(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_archetypes

        archetypes = _load_archetypes(data_root, "folk-horror")
        assert len(archetypes) == 1
        assert archetypes[0]["canonical_name"] == "The Earnest Warden"

    def test_returns_empty_for_missing_genre(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_archetypes

        archetypes = _load_archetypes(data_root, "nonexistent-genre")
        assert archetypes == []


class TestLoadArchetypeDynamics:
    def test_loads_genre_dynamics(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import _load_archetype_dynamics

        dynamics = _load_archetype_dynamics(data_root, "folk-horror")
        assert len(dynamics) == 1
        assert dynamics[0]["pairing_name"] == "The Warmth That Prepares the Sacrifice"


class TestBuildArchetypesContext:
    def test_formats_archetypes(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import (
            _build_archetypes_context,
            _load_archetypes,
        )

        archetypes = _load_archetypes(data_root, "folk-horror")
        ctx = _build_archetypes_context(archetypes)
        assert "The Earnest Warden" in ctx
        assert "Genuine care" in ctx
        assert "warmth" in ctx


class TestBuildPrompt:
    def test_substitutes_all_placeholders(self, data_root: Path) -> None:
        from narrative_data.tome.elicit_characters_significant import (
            _build_archetypes_context,
            _build_dynamics_context,
            _build_mundane_characters_context,
            _build_prompt,
            _load_archetypes,
            _load_archetype_dynamics,
            _load_mundane_characters,
        )
        from narrative_data.tome.elicit_places import _load_world_position
        from narrative_data.tome.elicit_orgs import _load_places
        from narrative_data.tome.elicit_social_substrate import _load_orgs
        from narrative_data.tome.elicit_characters_mundane import _load_social_substrate

        world_dir = data_root / "narrative-data" / "tome" / "worlds" / "test-world"
        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)
        mundane = _load_mundane_characters(world_dir)
        archetypes = _load_archetypes(data_root, "folk-horror")
        dynamics = _load_archetype_dynamics(data_root, "folk-horror")

        template = (
            "{genre_slug} {setting_slug} {world_preamble} "
            "{genre_profile_summary} {places_context} {orgs_context} "
            "{social_substrate_context} {mundane_characters_context} "
            "{archetypes_context} {archetype_dynamics_context}"
        )
        prompt = _build_prompt(
            template=template,
            world_pos=world_pos,
            genre_profile=None,
            places=places,
            orgs=orgs,
            substrate=substrate,
            mundane_characters=mundane,
            archetypes=archetypes,
            archetype_dynamics=dynamics,
            settings_context="",
        )
        assert "folk-horror" in prompt
        assert "The Earnest Warden" in prompt
        assert "Elda Farrow" in prompt
        assert "{" not in prompt
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_elicit_characters_significant.py -v`
Expected: FAIL — module does not exist yet.

- [ ] **Step 3: Write the elicitation module**

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Elicit significant characters (Q3 tension-bearing + Q4 scene-driving) for a Tome world.

Reads all prior pipeline context plus bedrock archetype data, builds a structured
prompt, calls qwen3.5:35b, parses the JSON response, and writes
characters-significant.json to the world directory.

Usage (via CLI):
    uv run narrative-data tome elicit-characters-significant --world-slug <slug>
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.tome.elicit_places import (
    _build_genre_profile_summary,
    _build_settings_context,
    _build_world_preamble,
    _load_world_position,
    _parse_places_response,
)
from narrative_data.tome.elicit_orgs import _build_places_context, _load_places
from narrative_data.tome.elicit_social_substrate import _build_orgs_context, _load_orgs
from narrative_data.tome.elicit_characters_mundane import (
    _build_social_substrate_context,
    _load_social_substrate,
)
from narrative_data.utils import now_iso

_PROMPTS_DIR = Path(__file__).parent.parent.parent.parent / "prompts"
_CHAR_TIMEOUT = 600.0
_CHAR_TEMPERATURE = 0.5


# ---------------------------------------------------------------------------
# Mundane character loading
# ---------------------------------------------------------------------------


def _load_mundane_characters(world_dir: Path) -> list[dict[str, Any]]:
    """Read characters-mundane.json and return the characters list.

    Args:
        world_dir: Path to the world directory.

    Returns:
        List of character dicts.

    Raises:
        FileNotFoundError: If characters-mundane.json does not exist.
        ValueError: If the file cannot be parsed or lacks a characters array.
    """
    path = world_dir / "characters-mundane.json"
    if not path.exists():
        raise FileNotFoundError(
            f"characters-mundane.json not found at {path}. "
            "Run 'tome elicit-characters-mundane' first."
        )
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise ValueError(f"Could not parse characters-mundane.json: {exc}") from exc

    chars = data.get("characters")
    if not isinstance(chars, list):
        raise ValueError(
            f"characters-mundane.json does not contain a 'characters' array at {path}."
        )
    return chars


# ---------------------------------------------------------------------------
# Bedrock archetype loading
# ---------------------------------------------------------------------------


def _load_archetypes(data_path: Path, genre_slug: str) -> list[dict[str, Any]]:
    """Load all bedrock archetype JSON files for a genre.

    Args:
        data_path: Root of the storyteller-data checkout.
        genre_slug: Genre region slug (e.g. "folk-horror").

    Returns:
        List of archetype dicts. Empty list if directory doesn't exist.
    """
    arch_dir = data_path / "narrative-data" / "discovery" / "archetypes" / genre_slug
    if not arch_dir.exists():
        return []

    archetypes: list[dict[str, Any]] = []
    for f in sorted(arch_dir.glob("*.json")):
        try:
            archetypes.append(json.loads(f.read_text()))
        except json.JSONDecodeError:
            continue
    return archetypes


def _load_archetype_dynamics(data_path: Path, genre_slug: str) -> list[dict[str, Any]]:
    """Load all bedrock archetype-dynamics JSON files for a genre.

    Args:
        data_path: Root of the storyteller-data checkout.
        genre_slug: Genre region slug (e.g. "folk-horror").

    Returns:
        List of archetype-dynamics dicts. Empty list if directory doesn't exist.
    """
    dyn_dir = data_path / "narrative-data" / "discovery" / "archetype-dynamics" / genre_slug
    if not dyn_dir.exists():
        return []

    dynamics: list[dict[str, Any]] = []
    for f in sorted(dyn_dir.glob("*.json")):
        try:
            dynamics.append(json.loads(f.read_text()))
        except json.JSONDecodeError:
            continue
    return dynamics


# ---------------------------------------------------------------------------
# Context construction
# ---------------------------------------------------------------------------


def _build_mundane_characters_context(characters: list[dict[str, Any]]) -> str:
    """Format mundane characters as markdown for the significant character prompt.

    Args:
        characters: List of Q1-Q2 character dicts.

    Returns:
        Markdown-formatted mundane character context.
    """
    if not characters:
        return "No mundane characters generated for this world yet."

    lines: list[str] = []
    for c in characters:
        centrality = c.get("centrality", "?")
        name = c.get("name", c.get("slug", "Unknown"))
        slug = c.get("slug", "")
        role = c.get("role", "?")
        cluster = c.get("cluster_membership", "?")
        description = c.get("description", "")

        line = f"- [{centrality}] **{name}** `{slug}` — {role} ({cluster})"
        if description:
            line += f"\n  {description[:200]}"

        tension = c.get("tension")
        if tension:
            line += f"\n  Tension: {tension[:200]}"

        lines.append(line)

    return "\n".join(lines)


def _build_archetypes_context(archetypes: list[dict[str, Any]]) -> str:
    """Format bedrock archetypes as markdown for the prompt.

    Args:
        archetypes: List of archetype dicts from discovery corpus.

    Returns:
        Markdown-formatted archetypes context.
    """
    if not archetypes:
        return "No bedrock archetype data available for this genre."

    lines: list[str] = []
    for a in archetypes:
        name = a.get("canonical_name", a.get("variant_name", "Unknown"))
        tension = a.get("distinguishing_tension", "")
        necessity = a.get("structural_necessity", "")
        profile = a.get("personality_profile", {})

        lines.append(f"### {name}")
        if tension:
            lines.append(f"**Distinguishing tension:** {tension}")
        if necessity:
            lines.append(f"**Structural necessity:** {necessity}")
        if profile and isinstance(profile, dict):
            profile_parts = [f"{k}: {v}" for k, v in profile.items()]
            lines.append(f"**Personality profile:** {', '.join(profile_parts)}")
        lines.append("")

    return "\n".join(lines).rstrip()


def _build_dynamics_context(dynamics: list[dict[str, Any]]) -> str:
    """Format archetype dynamics as markdown for the prompt.

    Args:
        dynamics: List of archetype-dynamics dicts.

    Returns:
        Markdown-formatted dynamics context.
    """
    if not dynamics:
        return "No archetype dynamics data available for this genre."

    lines: list[str] = []
    for d in dynamics:
        pairing = d.get("pairing_name", "Unknown Pairing")
        a = d.get("archetype_a", "?")
        b = d.get("archetype_b", "?")
        edge = d.get("edge_properties", {})
        edge_type = edge.get("edge_type", "") if isinstance(edge, dict) else ""
        scene = d.get("characteristic_scene", {})
        scene_title = scene.get("title", "") if isinstance(scene, dict) else ""

        lines.append(f"- **{pairing}** ({a} × {b})")
        if edge_type:
            lines.append(f"  Edge: {edge_type}")
        if scene_title:
            lines.append(f"  Scene: {scene_title}")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------


def _build_prompt(
    template: str,
    world_pos: dict[str, Any],
    genre_profile: dict[str, Any] | None,
    places: list[dict[str, Any]],
    orgs: list[dict[str, Any]],
    substrate: dict[str, Any],
    mundane_characters: list[dict[str, Any]],
    archetypes: list[dict[str, Any]],
    archetype_dynamics: list[dict[str, Any]],
    settings_context: str = "",
) -> str:
    """Substitute all placeholders into the character-significant-elicitation template.

    Args:
        template: Raw template text with {placeholders}.
        world_pos: Parsed world-position.json dict.
        genre_profile: Parsed region.json dict, or None.
        places: List of place dicts.
        orgs: List of organization dicts.
        substrate: Parsed social-substrate.json dict.
        mundane_characters: List of Q1-Q2 character dicts.
        archetypes: List of bedrock archetype dicts for the genre.
        archetype_dynamics: List of archetype-dynamics dicts for the genre.
        settings_context: Formatted genre settings archetypes.

    Returns:
        Fully substituted prompt string.
    """
    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    world_preamble = _build_world_preamble(world_pos)
    genre_summary = _build_genre_profile_summary(genre_profile)
    places_context = _build_places_context(places)
    orgs_context = _build_orgs_context(orgs)
    substrate_context = _build_social_substrate_context(substrate)
    mundane_context = _build_mundane_characters_context(mundane_characters)
    archetypes_context = _build_archetypes_context(archetypes)
    dynamics_context = _build_dynamics_context(archetype_dynamics)

    genre_profile_summary = genre_summary
    if settings_context:
        genre_profile_summary += "\n\n" + settings_context

    return (
        template.replace("{genre_slug}", genre_slug)
        .replace("{setting_slug}", setting_slug)
        .replace("{world_preamble}", world_preamble)
        .replace("{genre_profile_summary}", genre_profile_summary)
        .replace("{places_context}", places_context)
        .replace("{orgs_context}", orgs_context)
        .replace("{social_substrate_context}", substrate_context)
        .replace("{mundane_characters_context}", mundane_context)
        .replace("{archetypes_context}", archetypes_context)
        .replace("{archetype_dynamics_context}", dynamics_context)
    )


# ---------------------------------------------------------------------------
# Response parsing
# ---------------------------------------------------------------------------


def _parse_characters_response(response: str) -> list[dict[str, Any]]:
    """Parse LLM response as a JSON array of character objects.

    Args:
        response: Raw LLM response text.

    Returns:
        List of character dicts.

    Raises:
        ValueError: If parsing fails.
    """
    return _parse_places_response(response)


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def elicit_characters_significant(data_path: Path, world_slug: str) -> None:
    """Elicit significant characters (Q3 + Q4) for a Tome world.

    Args:
        data_path: Root of the storyteller-data checkout (STORYTELLER_DATA_PATH).
        world_slug: World identifier.
    """
    from rich.console import Console

    console = Console()

    world_dir = data_path / "narrative-data" / "tome" / "worlds" / world_slug
    template_path = _PROMPTS_DIR / "tome" / "character-significant-elicitation.md"

    # 1. Load all prerequisite data
    console.print(f"[bold]Loading world data for[/bold] [cyan]{world_slug}[/cyan]")
    try:
        world_pos = _load_world_position(world_dir)
        places = _load_places(world_dir)
        orgs = _load_orgs(world_dir)
        substrate = _load_social_substrate(world_dir)
        mundane_characters = _load_mundane_characters(world_dir)
    except (FileNotFoundError, ValueError) as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise SystemExit(1) from exc

    genre_slug = world_pos.get("genre_slug", "unknown")
    setting_slug = world_pos.get("setting_slug", "unknown")
    genre_profile: dict[str, Any] | None = world_pos.get("genre_profile")

    console.print(
        f"  genre=[cyan]{genre_slug}[/cyan]  "
        f"setting=[cyan]{setting_slug}[/cyan]  "
        f"places=[cyan]{len(places)}[/cyan]  "
        f"orgs=[cyan]{len(orgs)}[/cyan]  "
        f"clusters=[cyan]{len(substrate.get('clusters', []))}[/cyan]  "
        f"mundane=[cyan]{len(mundane_characters)}[/cyan]"
    )

    # 2. Load bedrock archetype data
    console.print(f"[bold]Loading bedrock archetypes for[/bold] [cyan]{genre_slug}[/cyan]")
    archetypes = _load_archetypes(data_path, genre_slug)
    archetype_dynamics = _load_archetype_dynamics(data_path, genre_slug)
    console.print(
        f"  [green]{len(archetypes)}[/green] archetype(s), "
        f"[green]{len(archetype_dynamics)}[/green] dynamic(s)"
    )

    # 3. Load prompt template
    if not template_path.exists():
        console.print(f"[red]Prompt template not found:[/red] {template_path}")
        raise SystemExit(1)

    template = template_path.read_text()

    # 4. Build prompt
    console.print("[bold]Building prompt…[/bold]")
    settings_context = _build_settings_context(data_path, genre_slug)
    prompt = _build_prompt(
        template, world_pos, genre_profile, places, orgs,
        substrate, mundane_characters, archetypes, archetype_dynamics,
        settings_context,
    )
    console.print(f"  Prompt length: [dim]{len(prompt)} chars[/dim]")

    # 5. Call LLM
    console.print(
        f"[bold]Calling[/bold] [cyan]{ELICITATION_MODEL}[/cyan] "
        f"[dim](timeout={_CHAR_TIMEOUT}s, temperature={_CHAR_TEMPERATURE})[/dim]"
    )
    client = OllamaClient()
    response = client.generate(
        model=ELICITATION_MODEL,
        prompt=prompt,
        timeout=_CHAR_TIMEOUT,
        temperature=_CHAR_TEMPERATURE,
    )
    console.print(f"  Response length: [dim]{len(response)} chars[/dim]")

    # 6. Parse response
    console.print("[bold]Parsing response…[/bold]")
    try:
        characters = _parse_characters_response(response)
    except ValueError as exc:
        console.print(f"[red]Parse error:[/red] {exc}")
        raise SystemExit(1) from exc

    q3 = [c for c in characters if c.get("centrality") == "Q3"]
    q4 = [c for c in characters if c.get("centrality") == "Q4"]
    console.print(f"  Parsed [green]{len(q3)}[/green] Q3 + [green]{len(q4)}[/green] Q4 character(s)")

    # 7. Write characters-significant.json
    output: dict[str, Any] = {
        "world_slug": world_slug,
        "genre_slug": genre_slug,
        "setting_slug": setting_slug,
        "generated_at": now_iso(),
        "model": ELICITATION_MODEL,
        "q3_count": len(q3),
        "q4_count": len(q4),
        "total_count": len(characters),
        "archetypes_available": len(archetypes),
        "dynamics_available": len(archetype_dynamics),
        "characters": characters,
    }

    output_path = world_dir / "characters-significant.json"
    output_path.write_text(json.dumps(output, indent=2))
    console.print(f"[bold green]Written:[/bold green] {output_path}")

    # 8. Summary
    console.print()
    console.print(f"[bold]Significant characters generated for[/bold] [cyan]{world_slug}[/cyan]:")
    for char in characters:
        slug = char.get("slug", "?")
        name = char.get("name", "?")
        centrality = char.get("centrality", "?")
        role = char.get("role", "?")
        archetype = char.get("archetype", {})
        primary = archetype.get("primary", "?") if isinstance(archetype, dict) else "?"
        console.print(
            f"  [green]✓[/green] [{centrality}] [bold]{name}[/bold] "
            f"[dim]({slug}, {role}, {primary})[/dim]"
        )
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/tome/test_elicit_characters_significant.py -v`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/tome/elicit_characters_significant.py tools/narrative-data/tests/tome/test_elicit_characters_significant.py
git commit -m "feat(tome): add significant character (Q3-Q4) elicitation module and tests"
```

---

## Task 7: CLI Commands

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/cli.py:380-389` (after existing `elicit-orgs` command)

- [ ] **Step 1: Add the three new CLI commands after the elicit-orgs command**

Insert after line 388 (`elicit_orgs(data_path, world_slug)`):

```python
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
```

- [ ] **Step 2: Verify CLI registration**

Run: `cd tools/narrative-data && uv run narrative-data tome --help`
Expected: All 3 new commands appear in the help output alongside existing compose-world, elicit-places, elicit-orgs.

- [ ] **Step 3: Commit**

```bash
git add tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat(tome): register social substrate and character elicitation CLI commands"
```

---

## Task 8: Run Full Test Suite

**Files:** None (validation only)

- [ ] **Step 1: Run all tome tests**

Run: `cd tools/narrative-data && uv run pytest tests/tome/ -v`
Expected: All tests pass.

- [ ] **Step 2: Run full narrative-data test suite**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests pass. No regressions from new modules.

- [ ] **Step 3: Run linting**

Run: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check .`
Expected: No lint errors, no format issues.

- [ ] **Step 4: Fix any issues and commit**

If issues found, fix and commit:
```bash
git add -A && git commit -m "fix: resolve lint/test issues in Phase 3b modules"
```

---

## Task 9: Validate Against Test Worlds

**Files:** None (manual validation)

This task runs the new pipeline against the 4 existing test worlds and evaluates output quality. It requires a running Ollama instance with `qwen3.5:35b`.

- [ ] **Step 1: Run social substrate elicitation on all 4 worlds**

```bash
cd tools/narrative-data
uv run narrative-data tome elicit-social-substrate --world-slug mccallisters-barn
uv run narrative-data tome elicit-social-substrate --world-slug the-windswept-crags
uv run narrative-data tome elicit-social-substrate --world-slug neon-depths
uv run narrative-data tome elicit-social-substrate --world-slug data-ghost
```

Review: Do substrates differ between intra-genre variants? Are cluster bases driven by the kinship-system axis? Are boundary tensions material and specific?

- [ ] **Step 2: Run mundane character elicitation on all 4 worlds**

```bash
uv run narrative-data tome elicit-characters-mundane --world-slug mccallisters-barn
uv run narrative-data tome elicit-characters-mundane --world-slug the-windswept-crags
uv run narrative-data tome elicit-characters-mundane --world-slug neon-depths
uv run narrative-data tome elicit-characters-mundane --world-slug data-ghost
```

Review: Do Q1 characters feel like inhabitants (not archetypes)? Do Q2 tensions arise from world-position? Do cluster memberships align with the social substrate?

- [ ] **Step 3: Run significant character elicitation on all 4 worlds**

```bash
uv run narrative-data tome elicit-characters-significant --world-slug mccallisters-barn
uv run narrative-data tome elicit-characters-significant --world-slug the-windswept-crags
uv run narrative-data tome elicit-characters-significant --world-slug neon-depths
uv run narrative-data tome elicit-characters-significant --world-slug data-ghost
```

Review: Do Q3-Q4 characters inhabit cluster boundaries? Do relational seeds reference Q1-Q2 characters? Does the same archetype produce different characters in different worlds? Does the centrality gradient feel right?

- [ ] **Step 4: Document findings**

Record observations, prompt adjustments needed, and whether Q3+Q4 should be split into separate prompts. Use `temper research save` if findings are substantial.

- [ ] **Step 5: Commit generated world data**

```bash
cd /path/to/storyteller-data
git add narrative-data/tome/worlds/*/social-substrate.json narrative-data/tome/worlds/*/characters-mundane.json narrative-data/tome/worlds/*/characters-significant.json
git commit -m "feat(tome): Phase 3b world data — social substrate + characters for 4 test worlds"
```
