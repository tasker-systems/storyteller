# Narrative Data Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `narrative-data` CLI tool for Tier B exploratory research — a two-stage LLM pipeline that generates genre-contextualized narrative data and spatial place-entity topologies.

**Architecture:** Python CLI with Click subcommands, two-stage pipeline (qwen3.5:35b elicitation → qwen2.5:3b-instruct structuring), Pydantic schemas for validation and JSON Schema export, httpx for Ollama communication. Output persists to `storyteller-data/narrative-data/` with UUIDv7 entity tracking and content-hash invalidation.

**Tech Stack:** Python >=3.11, Click, Pydantic, httpx, uuid-utils, rich, ruff, pytest, uv, hatchling

**Spec:** `docs/superpowers/specs/2026-03-17-narrative-data-tooling-design.md`

---

## File Map

```
tools/narrative-data/
├── pyproject.toml                              # Package config, CLI entry point, deps
├── prompts/
│   ├── genre/
│   │   └── region.md                           # Core genre region elicitation prompt
│   ├── spatial/
│   │   └── setting-type.md                     # Core setting type elicitation prompt
│   └── _commentary.md                          # Shared commentary directive suffix
├── src/narrative_data/
│   ├── __init__.py                             # Package docstring
│   ├── cli.py                                  # Click CLI: genre, spatial, cross-pollinate, status, list
│   ├── config.py                               # Data path resolution, model config, constants
│   ├── utils.py                                # Shared helpers (slug_to_name, now_iso)
│   ├── ollama.py                               # Thin httpx Ollama client (generate + generate_structured)
│   ├── prompts.py                              # Prompt loader + compositional builder
│   ├── schemas/
│   │   ├── __init__.py                         # Re-exports all schema types
│   │   ├── shared.py                           # NarrativeEntity, GenerationProvenance, ProvenanceEdge, DimensionalPosition
│   │   ├── genre.py                            # GenreRegion, Trope, NarrativeShape, GenreArchetype, etc.
│   │   ├── spatial.py                          # SettingType, PlaceEntity, TopologyEdge, etc.
│   │   └── intersections.py                    # IntersectionSynthesis, UpstreamRef, Enrichment
│   ├── pipeline/
│   │   ├── __init__.py                         # Re-exports
│   │   ├── elicit.py                           # Stage 1: prompt → qwen3.5:35b → raw.md
│   │   ├── structure.py                        # Stage 2: raw.md → qwen2.5:3b-instruct → validated .json
│   │   └── invalidation.py                     # Manifest I/O, content digest, prompt hash, staleness check
│   ├── genre/
│   │   ├── __init__.py
│   │   └── commands.py                         # Genre elicit/structure orchestration, dependency ordering
│   ├── spatial/
│   │   ├── __init__.py
│   │   └── commands.py                         # Spatial elicit/structure orchestration, dependency ordering
│   └── cross_pollination/
│       ├── __init__.py
│       └── commands.py                         # B.3 cross-domain synthesis orchestration
└── tests/
    ├── __init__.py
    ├── conftest.py                             # Shared fixtures (tmp output dirs, sample data)
    ├── test_schemas.py                         # Schema validation, JSON Schema export, round-trip
    ├── test_config.py                          # Path resolution, env var handling
    ├── test_prompts.py                         # Prompt loading, composition, dependency injection
    ├── test_invalidation.py                    # Manifest I/O, content digest, staleness detection
    ├── test_ollama.py                          # Ollama client (mocked httpx)
    ├── test_pipeline.py                        # Stage 1 + Stage 2 integration (mocked Ollama)
    └── test_cli.py                             # CLI invocation via Click test runner
```

---

### Task 1: Package Scaffold

**Files:**
- Create: `tools/narrative-data/pyproject.toml`
- Create: `tools/narrative-data/src/narrative_data/__init__.py`
- Create: `tools/narrative-data/tests/__init__.py`
- Create: `tools/narrative-data/tests/conftest.py`

- [ ] **Step 1: Create pyproject.toml**

```toml
[project]
name = "narrative-data"
version = "0.1.0"
description = "Narrative data elicitation tooling for storyteller genre, spatial, and cross-domain research"
requires-python = ">=3.11"
dependencies = [
    "click>=8.1.0",
    "httpx>=0.27.0",
    "pydantic>=2.6.0",
    "uuid-utils>=0.9.0",
    "rich>=13.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0.0",
    "ruff>=0.4.0",
]

[dependency-groups]
dev = ["pytest>=8.0.0", "ruff>=0.4.0"]

[project.scripts]
narrative-data = "narrative_data.cli:cli"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/narrative_data"]

[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "W", "I", "UP", "B", "C4", "SIM"]
```

- [ ] **Step 2: Create __init__.py files**

`src/narrative_data/__init__.py`:
```python
"""Narrative data elicitation tooling for storyteller research."""
```

`tests/__init__.py`: empty file.

`tests/conftest.py`:
```python
"""Shared test fixtures for narrative-data tests."""

from pathlib import Path

import pytest


@pytest.fixture
def tmp_output_dir(tmp_path: Path) -> Path:
    """Create a temporary output directory mimicking storyteller-data/narrative-data/."""
    for subdir in ["genres", "spatial", "intersections", "meta/schemas", "meta/runs"]:
        (tmp_path / subdir).mkdir(parents=True)
    return tmp_path


@pytest.fixture
def tmp_descriptor_dir(tmp_path: Path) -> Path:
    """Create a temporary descriptor directory with minimal test data."""
    desc_dir = tmp_path / "descriptors"
    desc_dir.mkdir()
    return desc_dir
```

- [ ] **Step 3: Install package in dev mode**

Run: `cd tools/narrative-data && uv sync --dev`
Expected: Package installs with all dependencies.

- [ ] **Step 4: Verify ruff and pytest run clean**

Run: `cd tools/narrative-data && uv run ruff check . && uv run pytest -v`
Expected: No lint errors, 0 tests collected (no test files with tests yet), exit 0.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/
git commit -m "feat: scaffold narrative-data Python package"
```

---

### Task 2: Shared Schemas

**Files:**
- Create: `tools/narrative-data/src/narrative_data/schemas/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/shared.py`
- Create: `tools/narrative-data/tests/test_schemas.py`

- [ ] **Step 1: Write failing tests for shared schema types**

`tests/test_schemas.py`:
```python
"""Tests for Pydantic schema validation and JSON Schema export."""

import json

from narrative_data.schemas.shared import (
    DimensionalPosition,
    GenerationProvenance,
    NarrativeEntity,
    ProvenanceEdge,
)


class TestGenerationProvenance:
    def test_valid_provenance(self):
        p = GenerationProvenance(
            prompt_hash="abc123",
            model="qwen3.5:35b",
            generated_at="2026-03-17T20:00:00Z",
        )
        assert p.model == "qwen3.5:35b"
        assert p.source_content_digest is None

    def test_provenance_with_digest(self):
        p = GenerationProvenance(
            prompt_hash="abc123",
            model="qwen2.5:3b-instruct",
            generated_at="2026-03-17T20:00:00Z",
            source_content_digest="sha256:deadbeef",
        )
        assert p.source_content_digest == "sha256:deadbeef"


class TestProvenanceEdge:
    def test_llm_elicited_edge(self):
        edge = ProvenanceEdge(
            source_id="ollama-qwen3.5:35b-run-1",
            source_type="llm_elicited",
            contribution_type="originated",
            weight=1.0,
        )
        assert edge.extractable is True
        assert edge.license is None

    def test_future_cc_by_sa_edge(self):
        edge = ProvenanceEdge(
            source_id="cthulhu-reborn-module-42",
            source_type="cc_by_sa",
            contribution_type="reinforced",
            weight=0.4,
            license="CC-BY-SA-4.0",
            extractable=True,
        )
        assert edge.license == "CC-BY-SA-4.0"


class TestDimensionalPosition:
    def test_bipolar_dimension(self):
        d = DimensionalPosition(dimension="dread_wonder", value=-0.7, note="high dread")
        assert d.value == -0.7

    def test_unipolar_dimension(self):
        d = DimensionalPosition(dimension="intimacy", value=0.3)
        assert d.note is None


class TestNarrativeEntity:
    def test_minimal_entity(self):
        e = NarrativeEntity(
            entity_id="019d0000-0000-7000-8000-000000000001",
            name="Test Entity",
            description="A test entity",
            provenance=GenerationProvenance(
                prompt_hash="abc", model="test", generated_at="2026-03-17T00:00:00Z"
            ),
        )
        assert e.commentary is None
        assert e.suggestions == []
        assert e.provenance_edges == []

    def test_entity_with_commentary(self):
        e = NarrativeEntity(
            entity_id="019d0000-0000-7000-8000-000000000002",
            name="Rich Entity",
            description="Has commentary",
            commentary="This entity could also express isolation themes",
            suggestions=["Consider adding a spatial dimension", "Links to gothic tradition"],
            provenance=GenerationProvenance(
                prompt_hash="abc", model="test", generated_at="2026-03-17T00:00:00Z"
            ),
            provenance_edges=[
                ProvenanceEdge(
                    source_id="run-1",
                    source_type="llm_elicited",
                    contribution_type="originated",
                    weight=1.0,
                )
            ],
        )
        assert len(e.suggestions) == 2
        assert len(e.provenance_edges) == 1

    def test_json_schema_export(self):
        schema = NarrativeEntity.model_json_schema()
        assert "properties" in schema
        assert "entity_id" in schema["properties"]
        assert "provenance" in schema["properties"]

    def test_round_trip_json(self):
        e = NarrativeEntity(
            entity_id="019d0000-0000-7000-8000-000000000003",
            name="Round Trip",
            description="Test serialization",
            provenance=GenerationProvenance(
                prompt_hash="abc", model="test", generated_at="2026-03-17T00:00:00Z"
            ),
        )
        json_str = e.model_dump_json()
        restored = NarrativeEntity.model_validate_json(json_str)
        assert restored.entity_id == e.entity_id
        assert restored.name == e.name
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_schemas.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'narrative_data.schemas'`

- [ ] **Step 3: Implement shared schemas**

`src/narrative_data/schemas/__init__.py`:
```python
"""Pydantic schemas for narrative data entities."""

from narrative_data.schemas.shared import (
    DimensionalPosition,
    GenerationProvenance,
    NarrativeEntity,
    ProvenanceEdge,
)

__all__ = [
    "DimensionalPosition",
    "GenerationProvenance",
    "NarrativeEntity",
    "ProvenanceEdge",
]
```

`src/narrative_data/schemas/shared.py`:
```python
"""Shared base types for all narrative data schemas."""

from pydantic import BaseModel


class GenerationProvenance(BaseModel):
    """Tracks how this entity was generated."""

    prompt_hash: str
    model: str
    generated_at: str
    source_content_digest: str | None = None


class ProvenanceEdge(BaseModel):
    """Attribution edge from source to knowledge node.

    Currently populated only for LLM elicitation. Schema designed
    to support future strategies: public domain analysis, CC-BY-SA
    RPG module extraction, and cross-source synthesis.
    """

    source_id: str
    source_type: str
    contribution_type: str
    weight: float
    license: str | None = None
    extractable: bool = True
    notes: str | None = None


class DimensionalPosition(BaseModel):
    """Weighted position along a named dimension."""

    dimension: str
    value: float
    note: str | None = None


class NarrativeEntity(BaseModel):
    """Base for all generated entities."""

    entity_id: str
    name: str
    description: str
    commentary: str | None = None
    suggestions: list[str] = []
    provenance: GenerationProvenance
    provenance_edges: list[ProvenanceEdge] = []
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_schemas.py -v`
Expected: All tests PASS.

- [ ] **Step 5: Run ruff**

Run: `cd tools/narrative-data && uv run ruff check .`
Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/schemas/ tools/narrative-data/tests/test_schemas.py
git commit -m "feat: add shared Pydantic schemas (NarrativeEntity, ProvenanceEdge)"
```

---

### Task 3: Genre and Spatial Schemas

**Files:**
- Create: `tools/narrative-data/src/narrative_data/schemas/genre.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/spatial.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/intersections.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/__init__.py`
- Modify: `tools/narrative-data/tests/test_schemas.py`

- [ ] **Step 1: Add failing tests for genre, spatial, and intersection schemas**

Append to `tests/test_schemas.py`:
```python
from narrative_data.schemas.genre import (
    GenreArchetype,
    GenreDynamic,
    GenreGoal,
    GenreProfile,
    GenreRegion,
    GenreSetting,
    NarrativeBeat,
    NarrativeShape,
    SubversionPattern,
    Trope,
    WorldAffordances,
)
from narrative_data.schemas.spatial import (
    CommunicabilityProfile,
    PlaceEntity,
    SensoryDetail,
    SettingType,
    TonalInheritanceRule,
    TopologyEdge,
)
from narrative_data.schemas.intersections import (
    Enrichment,
    IntersectionSynthesis,
    UpstreamRef,
)


def _provenance():
    return GenerationProvenance(
        prompt_hash="test", model="test", generated_at="2026-03-17T00:00:00Z"
    )


class TestGenreRegion:
    def test_full_genre_region(self):
        r = GenreRegion(
            entity_id="019d0000-0000-7000-8000-000000000010",
            name="Folk Horror",
            description="Rural dread, community as threat",
            provenance=_provenance(),
            aesthetic=[DimensionalPosition(dimension="spare_ornate", value=-0.3)],
            tonal=[DimensionalPosition(dimension="dread_wonder", value=-0.8)],
            thematic=[DimensionalPosition(dimension="belonging", value=0.7)],
            structural=[DimensionalPosition(dimension="mystery", value=0.6)],
            world_affordances=WorldAffordances(
                magic="subtle",
                technology="historical",
                violence="consequence-laden",
                death="permanent",
                supernatural="ambiguous",
            ),
        )
        assert r.name == "Folk Horror"
        assert len(r.aesthetic) == 1
        assert r.trope_refs == []

    def test_genre_region_json_schema(self):
        schema = GenreRegion.model_json_schema()
        assert "world_affordances" in schema["properties"]


class TestTrope:
    def test_trope_with_subversion(self):
        t = Trope(
            entity_id="019d0000-0000-7000-8000-000000000011",
            name="The Wicker Man",
            description="Community sacrifices outsider for renewal",
            provenance=_provenance(),
            genre_associations=["019d0000-0000-7000-8000-000000000010"],
            narrative_function="Reveals that community belonging has a price",
            subversion_patterns=[
                SubversionPattern(
                    name="Willing sacrifice",
                    description="The outsider chooses to participate",
                    effect="Transforms horror into tragedy",
                )
            ],
        )
        assert len(t.subversion_patterns) == 1


class TestGenreArchetype:
    def test_genre_archetype(self):
        a = GenreArchetype(
            entity_id="019d0000-0000-7000-8000-000000000012",
            name="The Cunning Elder",
            description="Authority figure hiding community secrets",
            provenance=_provenance(),
            base_archetype_ref="019c0000-0000-7000-8000-000000000001",
            genre_ref="019d0000-0000-7000-8000-000000000010",
            personality_axes=[DimensionalPosition(dimension="trust", value=0.2)],
            typical_roles=["antagonist", "gatekeeper"],
            genre_specific_notes="In folk horror, authority conceals ritual purpose",
        )
        assert a.base_archetype_ref is not None


class TestGenreDynamic:
    def test_genre_dynamic_has_domain_fields(self):
        d = GenreDynamic(
            entity_id="019d0000-0000-7000-8000-000000000013",
            name="Outsider and Community",
            description="Tension between newcomer and established group",
            provenance=_provenance(),
            genre_ref="019d0000-0000-7000-8000-000000000010",
            role_a_expression="Naive investigator drawn by curiosity",
            role_b_expression="Collective voice masking shared purpose",
            relational_texture="Surface warmth concealing assessment",
            typical_escalation="Hospitality → subtle tests → reveal of true nature",
            genre_specific_notes="Folk horror inverts the welcome",
        )
        assert d.relational_texture == "Surface warmth concealing assessment"


class TestSettingType:
    def test_setting_type(self):
        s = SettingType(
            entity_id="019d0000-0000-7000-8000-000000000020",
            name="Gothic Mansion",
            description="Decay, rooms with purpose, verticality",
            provenance=_provenance(),
            genre_associations=["019d0000-0000-7000-8000-000000000010"],
            atmospheric_signature="Oppressive grandeur in decline",
            sensory_palette=["dust", "cold stone", "ticking clock"],
            temporal_character="Time feels slower, layered with history",
        )
        assert len(s.sensory_palette) == 3


class TestPlaceEntity:
    def test_place_entity(self):
        p = PlaceEntity(
            entity_id="019d0000-0000-7000-8000-000000000021",
            name="Entry Hall",
            description="Imposing first impression, threshold to the interior",
            provenance=_provenance(),
            setting_type_ref="019d0000-0000-7000-8000-000000000020",
            narrative_function="threshold",
            communicability=CommunicabilityProfile(
                atmospheric="imposing, watchful",
                sensory="dust, old wood, ticking clock",
                spatial="high ceiling, branching paths",
                temporal="the house remembers who enters",
            ),
            sensory_details=[
                SensoryDetail(sense="sight", detail="faded portraits line the walls"),
                SensoryDetail(
                    sense="sound",
                    detail="floorboards creak under weight",
                    emotional_valence="unease",
                ),
            ],
        )
        assert p.narrative_function == "threshold"


class TestTopologyEdge:
    def test_topology_edge(self):
        e = TopologyEdge(
            edge_id="019d0000-0000-7000-8000-000000000030",
            from_place="019d0000-0000-7000-8000-000000000021",
            to_place="019d0000-0000-7000-8000-000000000022",
            adjacency_type="doorway",
            friction="low",
            permeability=["sound", "light"],
        )
        assert e.tonal_shift_note is None


class TestIntersectionSynthesis:
    def test_intersection_synthesis(self):
        s = IntersectionSynthesis(
            entity_id="019d0000-0000-7000-8000-000000000040",
            name="Folk Horror × Gothic Mansion",
            description="How folk horror transforms the gothic mansion",
            provenance=_provenance(),
            upstream_refs=[
                UpstreamRef(
                    entity_id="019d0000-0000-7000-8000-000000000010",
                    content_digest="sha256:aaa",
                    domain="genre",
                ),
                UpstreamRef(
                    entity_id="019d0000-0000-7000-8000-000000000020",
                    content_digest="sha256:bbb",
                    domain="spatial",
                ),
            ],
            content_hash="sha256:combined",
            enrichments=[
                Enrichment(
                    target_entity_id="019d0000-0000-7000-8000-000000000021",
                    enrichment_type="tonal_refinement",
                    content="Entry hall gains ritual significance in folk horror",
                )
            ],
            gaps_identified=["No place entity for ritual site"],
            new_entries=[],
        )
        assert len(s.upstream_refs) == 2
        assert len(s.enrichments) == 1
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_schemas.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement genre schemas**

`src/narrative_data/schemas/genre.py`:
```python
"""Genre domain schemas: regions, tropes, narrative shapes, and genre-contextualized descriptors."""

from pydantic import BaseModel

from narrative_data.schemas.shared import DimensionalPosition, NarrativeEntity


class WorldAffordances(BaseModel):
    """What is possible in this genre's physics."""

    magic: str
    technology: str
    violence: str
    death: str
    supernatural: str
    commentary: str | None = None


class GenreRegion(NarrativeEntity):
    """A named cluster in multidimensional narrative space."""

    aesthetic: list[DimensionalPosition]
    tonal: list[DimensionalPosition]
    thematic: list[DimensionalPosition]
    structural: list[DimensionalPosition]
    world_affordances: WorldAffordances
    trope_refs: list[str] = []


class SubversionPattern(BaseModel):
    """A known inversion of a trope and its narrative effect."""

    name: str
    description: str
    effect: str


class Trope(NarrativeEntity):
    """A shared narrative convention with reinforcement and subversion patterns."""

    genre_associations: list[str]
    narrative_function: str
    subversion_patterns: list[SubversionPattern] = []
    reinforcement_patterns: list[str] = []


class NarrativeBeat(BaseModel):
    """A single beat in a narrative shape's progression."""

    name: str
    description: str
    position: str
    flexibility: str


class NarrativeShape(NarrativeEntity):
    """An expected arc structure for a genre region."""

    beats: list[NarrativeBeat]
    genre_associations: list[str]
    tension_profile: str


class GenreArchetype(NarrativeEntity):
    """Genre-contextualized expression of a character archetype."""

    base_archetype_ref: str | None = None
    genre_ref: str
    personality_axes: list[DimensionalPosition]
    typical_roles: list[str]
    genre_specific_notes: str


class GenreDynamic(NarrativeEntity):
    """Genre-contextualized expression of a relational dynamic."""

    base_dynamic_ref: str | None = None
    genre_ref: str
    role_a_expression: str
    role_b_expression: str
    relational_texture: str
    typical_escalation: str
    genre_specific_notes: str


class GenreProfile(NarrativeEntity):
    """Genre-contextualized expression of a scene profile."""

    base_profile_ref: str | None = None
    genre_ref: str
    scene_shape: str
    tension_signature: str
    characteristic_moments: list[str]
    genre_specific_notes: str


class GenreGoal(NarrativeEntity):
    """Genre-contextualized expression of a narrative goal."""

    base_goal_ref: str | None = None
    genre_ref: str
    pursuit_expression: str
    success_shape: str
    failure_shape: str
    genre_specific_notes: str


class GenreSetting(NarrativeEntity):
    """Genre-contextualized setting vocabulary."""

    genre_ref: str
    typical_locations: list[str]
    atmospheric_vocabulary: list[str]
    sensory_vocabulary: list[str]
    genre_specific_notes: str
```

- [ ] **Step 4: Implement spatial schemas**

`src/narrative_data/schemas/spatial.py`:
```python
"""Spatial domain schemas: setting types, place entities, topology, tonal inheritance."""

from pydantic import BaseModel

from narrative_data.schemas.shared import NarrativeEntity


class SettingType(NarrativeEntity):
    """A category of physical setting with atmospheric and sensory character."""

    genre_associations: list[str]
    atmospheric_signature: str
    sensory_palette: list[str]
    temporal_character: str


class SensoryDetail(BaseModel):
    """A single sensory observation about a place."""

    sense: str
    detail: str
    emotional_valence: str | None = None


class CommunicabilityProfile(BaseModel):
    """Four dimensions of how a place communicates narratively."""

    atmospheric: str
    sensory: str
    spatial: str
    temporal: str


class PlaceEntity(NarrativeEntity):
    """A narratively significant space within a setting type."""

    setting_type_ref: str
    narrative_function: str
    communicability: CommunicabilityProfile
    sensory_details: list[SensoryDetail]


class TopologyEdge(BaseModel):
    """A spatial relationship between two place entities."""

    edge_id: str
    from_place: str
    to_place: str
    adjacency_type: str
    friction: str
    permeability: list[str]
    tonal_shift_note: str | None = None


class TonalInheritanceRule(NarrativeEntity):
    """A rule governing how tone propagates across spatial boundaries."""

    setting_type_ref: str
    rule: str
    applies_across: str
    friction_level: str
    examples: list[str] = []
```

- [ ] **Step 5: Implement intersection schemas**

`src/narrative_data/schemas/intersections.py`:
```python
"""Intersection schemas: cross-domain synthesis between genre and spatial data."""

from pydantic import BaseModel

from narrative_data.schemas.shared import NarrativeEntity


class UpstreamRef(BaseModel):
    """Reference to an upstream entity used in synthesis."""

    entity_id: str
    content_digest: str
    domain: str


class Enrichment(BaseModel):
    """A specific enrichment produced by synthesis."""

    target_entity_id: str
    enrichment_type: str
    content: str


class IntersectionSynthesis(NarrativeEntity):
    """Cross-domain synthesis between genre and spatial data."""

    upstream_refs: list[UpstreamRef]
    content_hash: str
    enrichments: list[Enrichment]
    gaps_identified: list[str]
    new_entries: list[str] = []
```

- [ ] **Step 6: Update schemas/__init__.py with re-exports**

Add all new types to the `__init__.py` imports and `__all__` list.

- [ ] **Step 7: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_schemas.py -v`
Expected: All tests PASS.

- [ ] **Step 8: Run ruff**

Run: `cd tools/narrative-data && uv run ruff check .`
Expected: No errors.

- [ ] **Step 9: Commit**

```bash
git add tools/narrative-data/src/narrative_data/schemas/ tools/narrative-data/tests/test_schemas.py
git commit -m "feat: add genre, spatial, and intersection Pydantic schemas"
```

---

### Task 4: Config and Data Path Resolution

**Files:**
- Create: `tools/narrative-data/src/narrative_data/config.py`
- Create: `tools/narrative-data/tests/test_config.py`

- [ ] **Step 1: Write failing tests**

`tests/test_config.py`:
```python
"""Tests for configuration and data path resolution."""

import os
from pathlib import Path

import pytest

from narrative_data.config import (
    GENRE_CATEGORIES,
    SPATIAL_CATEGORIES,
    resolve_data_path,
    resolve_output_path,
)


class TestPathResolution:
    def test_resolve_data_path_from_env(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
        desc_dir = tmp_path / "training-data" / "descriptors"
        desc_dir.mkdir(parents=True)
        monkeypatch.setenv("STORYTELLER_DATA_PATH", str(tmp_path))
        assert resolve_data_path() == tmp_path

    def test_resolve_data_path_missing_env(self, monkeypatch: pytest.MonkeyPatch):
        monkeypatch.delenv("STORYTELLER_DATA_PATH", raising=False)
        with pytest.raises(RuntimeError, match="STORYTELLER_DATA_PATH"):
            resolve_data_path()

    def test_resolve_output_path(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
        monkeypatch.setenv("STORYTELLER_DATA_PATH", str(tmp_path))
        output = resolve_output_path()
        assert output == tmp_path / "narrative-data"

    def test_descriptor_dir(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
        desc_dir = tmp_path / "training-data" / "descriptors"
        desc_dir.mkdir(parents=True)
        monkeypatch.setenv("STORYTELLER_DATA_PATH", str(tmp_path))
        from narrative_data.config import resolve_descriptor_dir
        assert resolve_descriptor_dir() == desc_dir


class TestConstants:
    def test_genre_categories(self):
        assert "region" in GENRE_CATEGORIES
        assert "archetypes" in GENRE_CATEGORIES
        assert "tropes" in GENRE_CATEGORIES

    def test_spatial_categories(self):
        assert "setting-type" in SPATIAL_CATEGORIES
        assert "place-entities" in SPATIAL_CATEGORIES
        assert "topology" in SPATIAL_CATEGORIES
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_config.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement config module**

`src/narrative_data/config.py`:
```python
"""Configuration and data path resolution."""

import os
from pathlib import Path

# Genre descriptor categories in dependency order.
# "region" must be elicited first; all others depend on it.
GENRE_CATEGORIES: list[str] = [
    "region",
    "archetypes",
    "tropes",
    "narrative-shapes",
    "dynamics",
    "profiles",
    "goals",
    "settings",
]

# Spatial descriptor categories in dependency order.
# Each depends on the one before it.
SPATIAL_CATEGORIES: list[str] = [
    "setting-type",
    "place-entities",
    "topology",
    "tonal-inheritance",
]

# Default model configuration
ELICITATION_MODEL = "qwen3.5:35b"
STRUCTURING_MODEL = "qwen2.5:3b-instruct"
OLLAMA_BASE_URL = "http://localhost:11434"
ELICITATION_TIMEOUT = 600.0  # 10 minutes for 35B model
STRUCTURING_TIMEOUT = 120.0  # 2 minutes for 3B model


def resolve_data_path() -> Path:
    """Resolve the storyteller-data root path from STORYTELLER_DATA_PATH env var."""
    path = os.environ.get("STORYTELLER_DATA_PATH")
    if not path:
        raise RuntimeError(
            "STORYTELLER_DATA_PATH environment variable is not set. "
            "Set it to the path of the storyteller-data repository."
        )
    return Path(path)


def resolve_output_path() -> Path:
    """Resolve the narrative-data output directory."""
    return resolve_data_path() / "narrative-data"


def resolve_descriptor_dir() -> Path:
    """Resolve the existing flat descriptor directory."""
    return resolve_data_path() / "training-data" / "descriptors"
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_config.py -v`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/config.py tools/narrative-data/tests/test_config.py
git commit -m "feat: add config module with path resolution and constants"
```

---

### Task 5: Ollama Client

**Files:**
- Create: `tools/narrative-data/src/narrative_data/ollama.py`
- Create: `tools/narrative-data/tests/test_ollama.py`

- [ ] **Step 1: Write failing tests (mocked httpx)**

`tests/test_ollama.py`:
```python
"""Tests for Ollama client (httpx mocked)."""

from unittest.mock import AsyncMock, patch

import pytest

from narrative_data.ollama import OllamaClient


class TestOllamaClient:
    def test_default_config(self):
        client = OllamaClient()
        assert client.base_url == "http://localhost:11434"

    def test_custom_config(self):
        client = OllamaClient(base_url="http://gpu-box:11434")
        assert client.base_url == "http://gpu-box:11434"

    def test_generate(self):
        client = OllamaClient()
        mock_response = {
            "response": "# Folk Horror\n\nA genre rooted in rural dread..."
        }
        with patch("httpx.post") as mock_post:
            mock_post.return_value.json.return_value = mock_response
            mock_post.return_value.raise_for_status = lambda: None
            result = client.generate(
                model="qwen3.5:35b",
                prompt="Describe folk horror as a genre region",
            )
        assert "Folk Horror" in result
        call_json = mock_post.call_args[1]["json"]
        assert call_json["model"] == "qwen3.5:35b"
        assert call_json["stream"] is False

    def test_generate_structured(self):
        client = OllamaClient()
        mock_response = {
            "response": '{"name": "Folk Horror", "description": "Rural dread"}'
        }
        with patch("httpx.post") as mock_post:
            mock_post.return_value.json.return_value = mock_response
            mock_post.return_value.raise_for_status = lambda: None
            result = client.generate_structured(
                model="qwen2.5:3b-instruct",
                prompt="Structure this content",
                schema={"type": "object", "properties": {"name": {"type": "string"}}},
            )
        assert result["name"] == "Folk Horror"
        call_json = mock_post.call_args[1]["json"]
        assert "format" in call_json

    def test_generate_timeout_retry(self):
        client = OllamaClient()
        import httpx as httpx_mod
        with patch("httpx.post") as mock_post:
            mock_post.side_effect = [
                httpx_mod.ReadTimeout("timeout"),
                type("Response", (), {
                    "json": lambda self: {"response": "ok"},
                    "raise_for_status": lambda self: None,
                })(),
            ]
            result = client.generate(model="test", prompt="test")
        assert result == "ok"
        assert mock_post.call_count == 2
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_ollama.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement Ollama client**

`src/narrative_data/ollama.py`:
```python
"""Thin httpx client for Ollama API."""

import json
from typing import Any

import httpx

from narrative_data.config import (
    ELICITATION_TIMEOUT,
    OLLAMA_BASE_URL,
    STRUCTURING_TIMEOUT,
)


class OllamaClient:
    """Client for Ollama's /api/generate endpoint."""

    def __init__(self, base_url: str = OLLAMA_BASE_URL):
        self.base_url = base_url

    def generate(
        self,
        model: str,
        prompt: str,
        timeout: float = ELICITATION_TIMEOUT,
        temperature: float = 0.8,
        max_retries: int = 3,
    ) -> str:
        """Stage 1: Generate raw text from the model. Returns the response string."""
        assert max_retries >= 1, "max_retries must be >= 1"
        for attempt in range(max_retries):
            try:
                response = httpx.post(
                    f"{self.base_url}/api/generate",
                    json={
                        "model": model,
                        "prompt": prompt,
                        "stream": False,
                        "options": {"temperature": temperature},
                    },
                    timeout=timeout,
                )
                response.raise_for_status()
                return response.json()["response"]
            except httpx.ReadTimeout:
                if attempt < max_retries - 1:
                    continue
                raise
        raise RuntimeError("unreachable")

    def generate_structured(
        self,
        model: str,
        prompt: str,
        schema: dict[str, Any],
        timeout: float = STRUCTURING_TIMEOUT,
        temperature: float = 0.1,
        max_retries: int = 3,
    ) -> dict[str, Any]:
        """Stage 2: Generate structured JSON from the model. Returns parsed dict."""
        for attempt in range(max_retries):
            try:
                response = httpx.post(
                    f"{self.base_url}/api/generate",
                    json={
                        "model": model,
                        "prompt": prompt,
                        "stream": False,
                        "format": schema,
                        "options": {"temperature": temperature},
                    },
                    timeout=timeout,
                )
                response.raise_for_status()
                text = response.json()["response"]
                return json.loads(text)
            except httpx.ReadTimeout:
                if attempt < max_retries - 1:
                    continue
                raise
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_ollama.py -v`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/ollama.py tools/narrative-data/tests/test_ollama.py
git commit -m "feat: add Ollama httpx client with retry logic"
```

---

### Task 6: Prompt Loader and Compositional Builder

**Files:**
- Create: `tools/narrative-data/src/narrative_data/prompts.py`
- Create: `tools/narrative-data/prompts/_commentary.md`
- Create: `tools/narrative-data/prompts/genre/region.md`
- Create: `tools/narrative-data/prompts/spatial/setting-type.md`
- Create: `tools/narrative-data/tests/test_prompts.py`

- [ ] **Step 1: Write failing tests**

`tests/test_prompts.py`:
```python
"""Tests for prompt loading and compositional building."""

from pathlib import Path

import pytest

from narrative_data.prompts import PromptBuilder


@pytest.fixture
def prompt_dir(tmp_path: Path) -> Path:
    """Create a minimal prompt directory."""
    genre_dir = tmp_path / "genre"
    genre_dir.mkdir()
    (genre_dir / "region.md").write_text("Describe genre region: {target_name}")
    spatial_dir = tmp_path / "spatial"
    spatial_dir.mkdir()
    (spatial_dir / "setting-type.md").write_text("Describe setting type: {target_name}")
    (tmp_path / "_commentary.md").write_text(
        "\n---\nInclude _commentary and _suggestions sections."
    )
    return tmp_path


class TestPromptBuilder:
    def test_load_core_prompt(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.load_core_prompt("genre", "region")
        assert "Describe genre region" in prompt

    def test_load_missing_prompt_raises(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        with pytest.raises(FileNotFoundError):
            builder.load_core_prompt("genre", "nonexistent")

    def test_build_stage1_prompt(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.build_stage1(
            domain="genre",
            category="region",
            target_name="Folk Horror",
        )
        assert "Folk Horror" in prompt
        assert "_commentary" in prompt
        assert "_suggestions" in prompt

    def test_build_stage1_with_context(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        prompt = builder.build_stage1(
            domain="genre",
            category="region",
            target_name="Folk Horror",
            context={"prior_region": '{"name": "Cosmic Horror"}'},
        )
        assert "Cosmic Horror" in prompt

    def test_build_stage2_prompt(self, prompt_dir: Path):
        builder = PromptBuilder(prompt_dir)
        raw_content = "# Folk Horror\n\nA genre of rural dread."
        schema = {"type": "object", "properties": {"name": {"type": "string"}}}
        prompt = builder.build_stage2(raw_content, schema)
        assert "Folk Horror" in prompt
        assert '"type": "object"' in prompt
        assert "Produce JSON matching this schema" in prompt
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_prompts.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Create prompt template files**

`prompts/_commentary.md`:
```markdown

---

## Commentary and Suggestions

After your main response, include two additional sections:

### _commentary
Your evaluative notes on this content. What patterns did you notice? What tensions exist between dimensions? What felt uncertain or forced? What surprised you?

### _suggestions
A list of things you wanted to express but couldn't within the requested structure. Connections to other genres or settings. Dimensions that feel missing. Alternative framings that might work better.
```

`prompts/genre/region.md`:
```markdown
You are helping build a multidimensional genre model for a narrative engine. A genre region is a named cluster in narrative space, defined by its position along aesthetic, tonal, thematic, structural, and world-affordance dimensions.

## Dimensional Framework

**Aesthetic dimensions** — visual and sensory vocabulary, prose register, descriptive density:
- Spare/minimalist ←→ Lush/ornate
- Grounded/mundane ←→ Heightened/mythic
- Gritty realism ←→ Lyrical beauty

**Tonal dimensions** — emotional contract with the reader/player:
- Dread ←→ Wonder
- Cynicism ←→ Earnestness
- Irony ←→ Sincerity
- Intimacy ←→ Epic distance

**Thematic dimensions** — what the story is about beneath its surface:
- Power and its corruption
- Identity and belonging
- Knowledge and its cost
- Connection and isolation

**Structural dimensions** — how the story moves, what shapes it takes:
- Mystery (concealment → revelation)
- Romance (separation → union)
- Tragedy (ascent → fall)
- Comedy (disorder → restored order)
- Horror (safety → violation of safety)
- Quest (lack → fulfillment or failure)

**World affordances** — what is possible in this story's physics:
- Magic: absent / subtle / rule-bound / wild-mythic
- Technology: historical / contemporary / speculative / post-human
- Violence: consequence-laden / stylized / absent
- Death: permanent / negotiable / metaphorical
- The supernatural: nonexistent / ambiguous / matter-of-fact

## Your Task

Describe the genre region **{target_name}** in rich detail. For each dimensional category, provide specific positions with explanatory notes. Explain what makes this genre distinctive — not just what it contains, but what it excludes, what it promises the reader, and where its boundaries blur with neighboring genres.

Be expansive. This is exploratory research, not a database entry.
```

`prompts/spatial/setting-type.md`:
```markdown
You are helping build a spatial topology model for a narrative engine. A setting type is a category of physical space that carries atmospheric, sensory, and narrative character.

## Setting Dimensions

**Atmospheric** — mood, feeling, emotional valence. How does this space make people feel?

**Sensory palette** — dominant sensory vocabulary. What do you see, hear, smell, touch, taste here?

**Temporal character** — how does time feel in this space? Does it move slowly, press urgently, layer with history?

**Genre associations** — which genres naturally use this setting? How does the setting transform across genres?

## Your Task

Describe the setting type **{target_name}** in rich detail. Cover its atmospheric signature, sensory palette, temporal character, and how it functions across different genres. What makes this space narratively significant — not just as a backdrop, but as a participant in the story?

Be expansive. This is exploratory research, not a database entry.
```

- [ ] **Step 4: Implement prompt builder**

`src/narrative_data/prompts.py`:
```python
"""Prompt loader and compositional builder for the two-stage pipeline."""

import json
from pathlib import Path


_PACKAGE_DIR = Path(__file__).parent.parent.parent
_DEFAULT_PROMPTS_DIR = _PACKAGE_DIR / "prompts"


class PromptBuilder:
    """Loads markdown prompt templates and composes them with dynamic context."""

    def __init__(self, prompts_dir: Path = _DEFAULT_PROMPTS_DIR):
        self.prompts_dir = prompts_dir

    def load_core_prompt(self, domain: str, category: str) -> str:
        """Load a core prompt template from prompts/{domain}/{category}.md."""
        path = self.prompts_dir / domain / f"{category}.md"
        if not path.exists():
            raise FileNotFoundError(f"Prompt template not found: {path}")
        return path.read_text()

    def _load_commentary_directive(self) -> str:
        """Load the shared commentary directive suffix."""
        path = self.prompts_dir / "_commentary.md"
        if path.exists():
            return path.read_text()
        return ""

    def build_stage1(
        self,
        domain: str,
        category: str,
        target_name: str,
        context: dict[str, str] | None = None,
    ) -> str:
        """Build a complete Stage 1 prompt from template + context + commentary."""
        core = self.load_core_prompt(domain, category)
        prompt = core.replace("{target_name}", target_name)

        if context:
            prompt += "\n\n---\n\n## Additional Context\n\n"
            for label, content in context.items():
                prompt += f"### {label}\n\n{content}\n\n"

        prompt += self._load_commentary_directive()
        return prompt

    @staticmethod
    def build_stage2(raw_content: str, schema: dict) -> str:
        """Build a Stage 2 structuring prompt from raw content + JSON Schema."""
        schema_str = json.dumps(schema, indent=2)
        return f"""Given the following content:
---
{raw_content}
---

Produce JSON matching this schema:
{schema_str}

Rules:
- Preserve all substantive information from the source
- Map evaluative notes to the commentary and suggestions fields
- Do not invent information not present in the source
- If a field cannot be populated from the source, use null"""
```

- [ ] **Step 5: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_prompts.py -v`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/prompts.py tools/narrative-data/prompts/ tools/narrative-data/tests/test_prompts.py
git commit -m "feat: add prompt loader and compositional builder with templates"
```

---

### Task 7: Invalidation and Manifest Management

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/pipeline/invalidation.py`
- Create: `tools/narrative-data/tests/test_invalidation.py`

- [ ] **Step 1: Write failing tests**

`tests/test_invalidation.py`:
```python
"""Tests for manifest I/O, content digest, and staleness detection."""

import json
from pathlib import Path

from narrative_data.pipeline.invalidation import (
    compute_content_digest,
    compute_intersection_hash,
    compute_prompt_hash,
    is_stale,
    load_manifest,
    save_manifest,
    update_manifest_entry,
)


class TestContentDigest:
    def test_digest_deterministic(self):
        d1 = compute_content_digest("hello world")
        d2 = compute_content_digest("hello world")
        assert d1 == d2

    def test_digest_changes_with_content(self):
        d1 = compute_content_digest("hello")
        d2 = compute_content_digest("world")
        assert d1 != d2

    def test_digest_prefix(self):
        d = compute_content_digest("test")
        assert d.startswith("sha256:")


class TestPromptHash:
    def test_prompt_hash_deterministic(self):
        h1 = compute_prompt_hash("prompt text")
        h2 = compute_prompt_hash("prompt text")
        assert h1 == h2

    def test_prompt_hash_changes(self):
        h1 = compute_prompt_hash("v1 prompt")
        h2 = compute_prompt_hash("v2 prompt")
        assert h1 != h2


class TestIntersectionHash:
    def test_intersection_hash_from_digests(self):
        h = compute_intersection_hash(["sha256:aaa", "sha256:bbb"])
        assert h.startswith("sha256:")

    def test_intersection_hash_order_sensitive(self):
        h1 = compute_intersection_hash(["sha256:aaa", "sha256:bbb"])
        h2 = compute_intersection_hash(["sha256:bbb", "sha256:aaa"])
        assert h1 != h2


class TestManifest:
    def test_load_empty_manifest(self, tmp_path: Path):
        manifest = load_manifest(tmp_path / "manifest.json")
        assert manifest == {"entries": {}}

    def test_save_and_load_manifest(self, tmp_path: Path):
        path = tmp_path / "manifest.json"
        manifest = {"entries": {"folk-horror": {"entity_id": "abc", "stage": "elicited"}}}
        save_manifest(path, manifest)
        loaded = load_manifest(path)
        assert loaded["entries"]["folk-horror"]["entity_id"] == "abc"

    def test_update_manifest_entry(self, tmp_path: Path):
        path = tmp_path / "manifest.json"
        save_manifest(path, {"entries": {}})
        update_manifest_entry(path, "folk-horror", {
            "entity_id": "abc",
            "prompt_hash": "h1",
            "content_digest": "d1",
            "stage": "elicited",
            "generated_at": "2026-03-17T00:00:00Z",
        })
        manifest = load_manifest(path)
        assert "folk-horror" in manifest["entries"]


class TestStaleness:
    def test_not_stale_when_hashes_match(self):
        entry = {"prompt_hash": "h1", "content_digest": "d1"}
        assert not is_stale(entry, current_prompt_hash="h1", upstream_digest="d1")

    def test_stale_when_prompt_changed(self):
        entry = {"prompt_hash": "h1", "content_digest": "d1"}
        assert is_stale(entry, current_prompt_hash="h2", upstream_digest="d1")

    def test_stale_when_upstream_changed(self):
        entry = {"prompt_hash": "h1", "content_digest": "d1"}
        assert is_stale(entry, current_prompt_hash="h1", upstream_digest="d2")

    def test_stale_when_no_entry(self):
        assert is_stale(None, current_prompt_hash="h1", upstream_digest="d1")
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_invalidation.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement invalidation module**

`src/narrative_data/pipeline/__init__.py`:
```python
"""Two-stage pipeline: elicitation and structuring."""
```

`src/narrative_data/pipeline/invalidation.py`:
```python
"""Manifest I/O, content digest, prompt hash, and staleness detection."""

import hashlib
import json
from pathlib import Path
from typing import Any


def compute_content_digest(content: str) -> str:
    """Compute a SHA-256 content digest with prefix."""
    h = hashlib.sha256(content.encode()).hexdigest()
    return f"sha256:{h}"


def compute_prompt_hash(prompt_text: str) -> str:
    """Compute a hash of the prompt text for invalidation tracking."""
    h = hashlib.sha256(prompt_text.encode()).hexdigest()[:16]
    return h


def compute_intersection_hash(upstream_digests: list[str]) -> str:
    """Compute a composite hash from ordered upstream content digests."""
    combined = "|".join(upstream_digests)
    h = hashlib.sha256(combined.encode()).hexdigest()
    return f"sha256:{h}"


def load_manifest(path: Path) -> dict[str, Any]:
    """Load a manifest file, returning empty structure if not found."""
    if not path.exists():
        return {"entries": {}}
    with open(path) as f:
        return json.load(f)


def save_manifest(path: Path, manifest: dict[str, Any]) -> None:
    """Save a manifest file with pretty formatting."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(manifest, f, indent=2)


def update_manifest_entry(path: Path, key: str, entry: dict[str, Any]) -> None:
    """Update a single entry in a manifest file."""
    manifest = load_manifest(path)
    manifest["entries"][key] = entry
    save_manifest(path, manifest)


def is_stale(
    entry: dict[str, Any] | None,
    current_prompt_hash: str,
    upstream_digest: str | None = None,
) -> bool:
    """Check if a manifest entry is stale relative to current prompt and upstream content."""
    if entry is None:
        return True
    if entry.get("prompt_hash") != current_prompt_hash:
        return True
    if upstream_digest and entry.get("content_digest") != upstream_digest:
        return True
    return False
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_invalidation.py -v`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/ tools/narrative-data/tests/test_invalidation.py
git commit -m "feat: add invalidation module with manifest I/O and staleness detection"
```

---

### Task 8: Pipeline — Stage 1 Elicitation and Stage 2 Structuring

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/elicit.py`
- Create: `tools/narrative-data/src/narrative_data/pipeline/structure.py`
- Create: `tools/narrative-data/tests/test_pipeline.py`

- [ ] **Step 1: Write failing tests**

`tests/test_pipeline.py`:
```python
"""Tests for the two-stage pipeline (Ollama calls mocked)."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.elicit import run_elicitation
from narrative_data.pipeline.structure import run_structuring, validate_and_save
from narrative_data.prompts import PromptBuilder
from narrative_data.schemas.genre import GenreRegion


@pytest.fixture
def mock_ollama() -> OllamaClient:
    client = MagicMock(spec=OllamaClient)
    return client


@pytest.fixture
def prompt_builder(tmp_path: Path) -> PromptBuilder:
    genre_dir = tmp_path / "genre"
    genre_dir.mkdir()
    (genre_dir / "region.md").write_text("Describe genre: {target_name}")
    (tmp_path / "_commentary.md").write_text("\n---\nCommentary directive.")
    return PromptBuilder(tmp_path)


class TestRunElicitation:
    def test_writes_raw_md(
        self, mock_ollama: OllamaClient, prompt_builder: PromptBuilder, tmp_path: Path
    ):
        mock_ollama.generate.return_value = "# Folk Horror\n\nRich content here."
        output_dir = tmp_path / "output" / "folk-horror"
        output_dir.mkdir(parents=True)

        result = run_elicitation(
            client=mock_ollama,
            builder=prompt_builder,
            domain="genre",
            category="region",
            target_name="Folk Horror",
            target_slug="folk-horror",
            output_dir=output_dir,
            model="qwen3.5:35b",
        )

        raw_path = output_dir / "region.raw.md"
        assert raw_path.exists()
        assert "Folk Horror" in raw_path.read_text()
        assert result["prompt_hash"] is not None
        assert result["content_digest"].startswith("sha256:")


class TestRunStructuring:
    def test_structures_and_validates(
        self, mock_ollama: OllamaClient, tmp_path: Path
    ):
        raw_path = tmp_path / "region.raw.md"
        raw_path.write_text("# Folk Horror\n\nRural dread content.")

        structured_output = {
            "entity_id": "019d0000-0000-7000-8000-000000000010",
            "name": "Folk Horror",
            "description": "Rural dread, community as threat",
            "provenance": {
                "prompt_hash": "abc",
                "model": "test",
                "generated_at": "2026-03-17T00:00:00Z",
            },
            "aesthetic": [{"dimension": "spare_ornate", "value": -0.3}],
            "tonal": [{"dimension": "dread_wonder", "value": -0.8}],
            "thematic": [],
            "structural": [],
            "world_affordances": {
                "magic": "subtle",
                "technology": "historical",
                "violence": "consequence-laden",
                "death": "permanent",
                "supernatural": "ambiguous",
            },
        }
        mock_ollama.generate_structured.return_value = structured_output

        result = run_structuring(
            client=mock_ollama,
            raw_path=raw_path,
            output_path=tmp_path / "region.json",
            schema_type=GenreRegion,
            model="qwen2.5:3b-instruct",
            is_collection=False,
        )

        assert result["success"] is True
        json_path = tmp_path / "region.json"
        assert json_path.exists()
        data = json.loads(json_path.read_text())
        assert data["name"] == "Folk Horror"

    def test_validation_failure_writes_errors(
        self, mock_ollama: OllamaClient, tmp_path: Path
    ):
        raw_path = tmp_path / "test.raw.md"
        raw_path.write_text("Some content")

        # Return invalid data (missing required fields)
        mock_ollama.generate_structured.return_value = {"invalid": True}

        result = run_structuring(
            client=mock_ollama,
            raw_path=raw_path,
            output_path=tmp_path / "test.json",
            schema_type=GenreRegion,
            model="qwen2.5:3b-instruct",
            is_collection=False,
            max_retries=1,
        )

        assert result["success"] is False
        errors_path = tmp_path / "test.errors.json"
        assert errors_path.exists()
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_pipeline.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement Stage 1 elicitation**

`src/narrative_data/pipeline/elicit.py`:
```python
"""Stage 1: Elicitation via qwen3.5:35b → raw.md."""

from pathlib import Path
from typing import Any

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.invalidation import compute_content_digest, compute_prompt_hash
from narrative_data.prompts import PromptBuilder


def run_elicitation(
    client: OllamaClient,
    builder: PromptBuilder,
    domain: str,
    category: str,
    target_name: str,
    target_slug: str,
    output_dir: Path,
    model: str = ELICITATION_MODEL,
    context: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Run Stage 1 elicitation for a single cell in the matrix.

    Returns a dict with prompt_hash and content_digest for manifest tracking.
    """
    prompt = builder.build_stage1(
        domain=domain,
        category=category,
        target_name=target_name,
        context=context,
    )
    prompt_hash = compute_prompt_hash(prompt)

    raw_content = client.generate(model=model, prompt=prompt)

    output_dir.mkdir(parents=True, exist_ok=True)
    raw_path = output_dir / f"{category}.raw.md"
    raw_path.write_text(raw_content)

    content_digest = compute_content_digest(raw_content)

    return {
        "prompt_hash": prompt_hash,
        "content_digest": content_digest,
        "raw_path": str(raw_path),
    }
```

- [ ] **Step 4: Implement Stage 2 structuring**

`src/narrative_data/pipeline/structure.py`:
```python
"""Stage 2: Structuring via qwen2.5:3b-instruct → validated .json."""

import json
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ValidationError

from narrative_data.config import STRUCTURING_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.prompts import PromptBuilder


def _build_stage2_prompt(raw_content: str, schema: dict) -> str:
    """Build the Stage 2 structuring prompt."""
    return PromptBuilder.build_stage2(raw_content, schema)


def run_structuring(
    client: OllamaClient,
    raw_path: Path,
    output_path: Path,
    schema_type: type[BaseModel],
    model: str = STRUCTURING_MODEL,
    is_collection: bool = True,
    max_retries: int = 3,
) -> dict[str, Any]:
    """Run Stage 2 structuring for a single raw.md → validated .json.

    Returns a dict with success status and any errors.
    """
    raw_content = raw_path.read_text()

    if is_collection:
        target_schema = {
            "type": "array",
            "items": schema_type.model_json_schema(),
        }
    else:
        target_schema = schema_type.model_json_schema()

    prompt = _build_stage2_prompt(raw_content, target_schema)
    errors: list[str] = []

    for attempt in range(max_retries):
        raw_output = client.generate_structured(
            model=model, prompt=prompt, schema=target_schema
        )

        try:
            validated = validate_and_save(raw_output, schema_type, output_path, is_collection)
            return {"success": True, "output_path": str(output_path), "validated": validated}
        except ValidationError as e:
            error_msg = str(e)
            errors.append(error_msg)
            prompt += f"\n\nThe previous output had validation errors:\n{error_msg}\nPlease fix."

    # All retries exhausted — write errors file
    errors_path = output_path.with_suffix(".errors.json")
    errors_path.write_text(json.dumps({
        "errors": errors,
        "raw_output": raw_output,
        "schema": schema_type.__name__,
    }, indent=2))

    return {"success": False, "errors_path": str(errors_path), "errors": errors}


def validate_and_save(
    data: Any,
    schema_type: type[BaseModel],
    output_path: Path,
    is_collection: bool,
) -> Any:
    """Validate data against schema and save to JSON file. Raises ValidationError on failure."""
    if is_collection:
        validated = [schema_type.model_validate(item) for item in data]
        output_data = [item.model_dump() for item in validated]
    else:
        validated = schema_type.model_validate(data)
        output_data = validated.model_dump()

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output_data, indent=2))
    return validated
```

- [ ] **Step 5: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_pipeline.py -v`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tools/narrative-data/src/narrative_data/pipeline/ tools/narrative-data/tests/test_pipeline.py
git commit -m "feat: add two-stage pipeline (elicitation + structuring with validation)"
```

---

### Task 9: Genre and Spatial Command Orchestration

**Files:**
- Create: `tools/narrative-data/src/narrative_data/genre/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/genre/commands.py`
- Create: `tools/narrative-data/src/narrative_data/spatial/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/spatial/commands.py`
- Create: `tools/narrative-data/src/narrative_data/cross_pollination/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/cross_pollination/commands.py`

- [ ] **Step 1: Create shared utils module**

`src/narrative_data/utils.py`:
```python
"""Shared helpers for narrative-data commands."""

from datetime import datetime, timezone


def slug_to_name(slug: str) -> str:
    """Convert a slug to a display name: 'folk-horror' → 'Folk Horror'."""
    return slug.replace("-", " ").title()


def now_iso() -> str:
    """Return current UTC time as ISO 8601."""
    return datetime.now(timezone.utc).isoformat()
```

- [ ] **Step 2: Create genre __init__.py and commands.py**

`src/narrative_data/genre/__init__.py`: empty file.

`src/narrative_data/genre/commands.py`:
```python
"""Genre-specific elicitation and structuring orchestration."""

import json
from pathlib import Path

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

# Map category names to their schema types and collection status
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


def elicit_genre(
    output_base: Path,
    prompts_dir: Path,
    regions: list[str] | None = None,
    categories: list[str] | None = None,
    force: bool = False,
    client: OllamaClient | None = None,
) -> None:
    """Run Stage 1 elicitation for genre regions."""
    client = client or OllamaClient()
    builder = PromptBuilder(prompts_dir)
    cats = categories or GENRE_CATEGORIES
    manifest_path = output_base / "genres" / "manifest.json"

    # Load existing flat descriptors for context injection
    descriptor_context = _load_descriptor_context()

    for region_slug in (regions or _default_regions()):
        region_name = slug_to_name(region_slug)
        region_dir = output_base / "genres" / region_slug

        # Ensure "region" runs first if any categories depend on it
        ordered_cats = _order_categories(cats)

        for cat in ordered_cats:
            manifest = load_manifest(manifest_path)
            entry_key = f"{region_slug}/{cat}"
            entry = manifest["entries"].get(entry_key)

            prompt = builder.build_stage1(
                domain="genre", category=cat, target_name=region_name
            )
            from narrative_data.pipeline.invalidation import compute_prompt_hash

            current_hash = compute_prompt_hash(prompt)

            if not force and entry and not is_stale(entry, current_hash):
                console.print(f"  [dim]Skipping {entry_key} (up to date)[/dim]")
                continue

            console.print(f"  [bold]Eliciting {entry_key}...[/bold]")

            # Build context: existing flat descriptors + prior region data
            context = dict(descriptor_context)  # Copy base context
            if cat != "region":
                region_json = region_dir / "region.json"
                if region_json.exists():
                    context["genre_region"] = region_json.read_text()

            result = run_elicitation(
                client=client,
                builder=builder,
                domain="genre",
                category=cat,
                target_name=region_name,
                target_slug=region_slug,
                output_dir=region_dir,
                model=ELICITATION_MODEL,
                context=context,
            )

            update_manifest_entry(manifest_path, entry_key, {
                "entity_id": None,  # Assigned during structuring
                "prompt_hash": result["prompt_hash"],
                "content_digest": result["content_digest"],
                "stage": "elicited",
                "generated_at": now_iso(),
            })
            console.print(f"  [green]✓ {entry_key}[/green]")


def structure_genre(
    output_base: Path,
    regions: list[str] | None = None,
    categories: list[str] | None = None,
    force: bool = False,
    client: OllamaClient | None = None,
) -> None:
    """Run Stage 2 structuring for genre regions."""
    client = client or OllamaClient()
    cats = categories or GENRE_CATEGORIES
    manifest_path = output_base / "genres" / "manifest.json"

    for region_slug in (regions or _default_regions()):
        region_dir = output_base / "genres" / region_slug

        for cat in cats:
            schema_type, is_collection = CATEGORY_SCHEMAS[cat]
            raw_path = region_dir / f"{cat}.raw.md"

            if not raw_path.exists():
                console.print(f"  [yellow]Skipping {region_slug}/{cat} (no raw.md)[/yellow]")
                continue

            output_path = region_dir / f"{cat}.json"
            entry_key = f"{region_slug}/{cat}"

            # Skip if already structured and not forced
            if not force and output_path.exists():
                manifest = load_manifest(manifest_path)
                entry = manifest["entries"].get(entry_key)
                if entry and entry.get("stage") == "structured":
                    console.print(f"  [dim]Skipping {entry_key} (already structured)[/dim]")
                    continue

            console.print(f"  [bold]Structuring {entry_key}...[/bold]")

            result = run_structuring(
                client=client,
                raw_path=raw_path,
                output_path=output_path,
                schema_type=schema_type,
                model=STRUCTURING_MODEL,
                is_collection=is_collection,
            )

            if result["success"]:
                from uuid_utils import uuid7

                update_manifest_entry(manifest_path, entry_key, {
                    **load_manifest(manifest_path)["entries"].get(entry_key, {}),
                    "entity_id": str(uuid7()),
                    "stage": "structured",
                })
                console.print(f"  [green]✓ {entry_key}[/green]")
            else:
                console.print(f"  [red]✗ {entry_key} (see .errors.json)[/red]")


def _load_descriptor_context() -> dict[str, str]:
    """Load existing flat descriptors for injection into elicitation prompts."""
    context: dict[str, str] = {}
    try:
        desc_dir = resolve_descriptor_dir()
        for name in ["archetypes", "dynamics", "profiles", "goals", "genres"]:
            path = desc_dir / f"{name}.json"
            if path.exists():
                context[f"existing_{name}"] = path.read_text()
    except RuntimeError:
        pass  # STORYTELLER_DATA_PATH not set; proceed without descriptors
    return context


def _order_categories(cats: list[str]) -> list[str]:
    """Ensure 'region' comes first if present."""
    if "region" in cats:
        return ["region"] + [c for c in cats if c != "region"]
    return cats


def _default_regions() -> list[str]:
    """Return default genre region slugs."""
    return [
        "folk-horror", "cosmic-horror",
        "high-epic-fantasy", "dark-fantasy", "cozy-fantasy",
        "fairy-tale-mythic", "urban-fantasy",
        "hard-sci-fi", "space-opera", "cyberpunk", "solarpunk",
        "nordic-noir", "cozy-mystery", "psychological-thriller",
        "romantasy", "historical-romance", "contemporary-romance",
        "literary-fiction", "magical-realism", "southern-gothic",
        "historical-fiction", "westerns",
        "swashbuckling-adventure", "survival-fiction",
        "pastoral-rural-fiction",
    ]
```

- [ ] **Step 2: Create spatial __init__.py and commands.py**

`src/narrative_data/spatial/__init__.py`: empty file.

`src/narrative_data/spatial/commands.py`:
```python
"""Spatial-specific elicitation and structuring orchestration."""

import json
from pathlib import Path

from rich.console import Console

from narrative_data.config import (
    ELICITATION_MODEL,
    SPATIAL_CATEGORIES,
    STRUCTURING_MODEL,
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
from narrative_data.schemas.spatial import (
    PlaceEntity,
    SettingType,
    TonalInheritanceRule,
    TopologyEdge,
)
from narrative_data.utils import now_iso, slug_to_name

console = Console()

# Map category names to schema types and collection status.
# TopologyEdge is not a NarrativeEntity, handled as a collection of edges.
CATEGORY_SCHEMAS: dict[str, tuple[type, bool]] = {
    "setting-type": (SettingType, False),
    "place-entities": (PlaceEntity, True),
    "topology": (TopologyEdge, True),
    "tonal-inheritance": (TonalInheritanceRule, True),
}


def elicit_spatial(
    output_base: Path,
    prompts_dir: Path,
    settings: list[str] | None = None,
    force: bool = False,
    client: OllamaClient | None = None,
) -> None:
    """Run Stage 1 elicitation for setting types."""
    client = client or OllamaClient()
    builder = PromptBuilder(prompts_dir)
    manifest_path = output_base / "spatial" / "manifest.json"

    for setting_slug in (settings or _default_settings()):
        setting_name = slug_to_name(setting_slug)
        setting_dir = output_base / "spatial" / setting_slug

        for cat in SPATIAL_CATEGORIES:
            manifest = load_manifest(manifest_path)
            entry_key = f"{setting_slug}/{cat}"
            entry = manifest["entries"].get(entry_key)

            prompt = builder.build_stage1(
                domain="spatial", category=cat, target_name=setting_name
            )
            current_hash = compute_prompt_hash(prompt)

            if not force and entry and not is_stale(entry, current_hash):
                console.print(f"  [dim]Skipping {entry_key} (up to date)[/dim]")
                continue

            console.print(f"  [bold]Eliciting {entry_key}...[/bold]")

            # Build context from prior stages
            context = _build_spatial_context(setting_dir, cat)

            result = run_elicitation(
                client=client,
                builder=builder,
                domain="spatial",
                category=cat,
                target_name=setting_name,
                target_slug=setting_slug,
                output_dir=setting_dir,
                model=ELICITATION_MODEL,
                context=context,
            )

            update_manifest_entry(manifest_path, entry_key, {
                "entity_id": None,
                "prompt_hash": result["prompt_hash"],
                "content_digest": result["content_digest"],
                "stage": "elicited",
                "generated_at": now_iso(),
            })
            console.print(f"  [green]✓ {entry_key}[/green]")


def structure_spatial(
    output_base: Path,
    settings: list[str] | None = None,
    force: bool = False,
    client: OllamaClient | None = None,
) -> None:
    """Run Stage 2 structuring for setting types."""
    client = client or OllamaClient()
    manifest_path = output_base / "spatial" / "manifest.json"

    for setting_slug in (settings or _default_settings()):
        setting_dir = output_base / "spatial" / setting_slug

        for cat in SPATIAL_CATEGORIES:
            schema_type, is_collection = CATEGORY_SCHEMAS[cat]
            raw_path = setting_dir / f"{cat}.raw.md"

            if not raw_path.exists():
                console.print(f"  [yellow]Skipping {setting_slug}/{cat} (no raw.md)[/yellow]")
                continue

            output_path = setting_dir / f"{cat}.json"
            entry_key = f"{setting_slug}/{cat}"

            # Skip if already structured and not forced
            if not force and output_path.exists():
                manifest = load_manifest(manifest_path)
                entry = manifest["entries"].get(entry_key)
                if entry and entry.get("stage") == "structured":
                    console.print(f"  [dim]Skipping {entry_key} (already structured)[/dim]")
                    continue

            console.print(f"  [bold]Structuring {entry_key}...[/bold]")

            result = run_structuring(
                client=client,
                raw_path=raw_path,
                output_path=output_path,
                schema_type=schema_type,
                model=STRUCTURING_MODEL,
                is_collection=is_collection,
            )

            if result["success"]:
                from uuid_utils import uuid7

                update_manifest_entry(manifest_path, entry_key, {
                    **load_manifest(manifest_path)["entries"].get(entry_key, {}),
                    "entity_id": str(uuid7()),
                    "stage": "structured",
                })
                console.print(f"  [green]✓ {entry_key}[/green]")
            else:
                console.print(f"  [red]✗ {entry_key} (see .errors.json)[/red]")


def _build_spatial_context(setting_dir: Path, category: str) -> dict[str, str] | None:
    """Build context from prior spatial stages for dependency injection."""
    context: dict[str, str] = {}
    if category != "setting-type":
        st_json = setting_dir / "setting-type.json"
        if st_json.exists():
            context["setting_type"] = st_json.read_text()
    if category in ("topology", "tonal-inheritance"):
        pe_json = setting_dir / "place-entities.json"
        if pe_json.exists():
            context["place_entities"] = pe_json.read_text()
    if category == "tonal-inheritance":
        topo_json = setting_dir / "topology.json"
        if topo_json.exists():
            context["topology"] = topo_json.read_text()
    return context if context else None


def _default_settings() -> list[str]:
    return [
        "family-home", "inn-tavern", "boarding-school",
        "city-streets", "market-bazaar", "government-building", "underground-subway",
        "gothic-mansion", "cathedral-temple", "castle-fortress", "university-library",
        "pastoral-village", "farmstead", "coastal-settlement",
        "dense-forest", "mountain-pass", "desert-wasteland", "river-lake-shore",
        "space-station", "sailing-vessel", "train-carriage", "ruins-archaeological-site",
    ]
```

- [ ] **Step 3: Create cross-pollination stub**

`src/narrative_data/cross_pollination/__init__.py`: empty file.

`src/narrative_data/cross_pollination/commands.py`:
```python
"""B.3 Cross-domain synthesis orchestration.

Depends on B.1 (genre) and B.2 (spatial) both reaching initial completion.
Implementation deferred until B.1 and B.2 produce initial data.
"""

from pathlib import Path

from rich.console import Console

console = Console()


def run_cross_pollination(
    output_base: Path,
    force: bool = False,
) -> None:
    """Run B.3 cross-pollination synthesis."""
    console.print(
        "[yellow]Cross-pollination requires B.1 and B.2 initial completion. "
        "Use 'narrative-data status' to check readiness.[/yellow]"
    )
```

- [ ] **Step 4: Run ruff**

Run: `cd tools/narrative-data && uv run ruff check .`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/src/narrative_data/genre/ tools/narrative-data/src/narrative_data/spatial/ tools/narrative-data/src/narrative_data/cross_pollination/
git commit -m "feat: add genre, spatial, and cross-pollination command orchestration"
```

---

### Task 10: CLI with Click

**Files:**
- Create: `tools/narrative-data/src/narrative_data/cli.py`
- Create: `tools/narrative-data/tests/test_cli.py`

- [ ] **Step 1: Write failing tests**

`tests/test_cli.py`:
```python
"""Tests for CLI invocation via Click test runner."""

from click.testing import CliRunner

from narrative_data.cli import cli


class TestCLI:
    def test_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["--help"])
        assert result.exit_code == 0
        assert "narrative-data" in result.output.lower() or "genre" in result.output

    def test_genre_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["genre", "--help"])
        assert result.exit_code == 0
        assert "elicit" in result.output
        assert "structure" in result.output

    def test_spatial_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["spatial", "--help"])
        assert result.exit_code == 0
        assert "elicit" in result.output

    def test_status_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["status", "--help"])
        assert result.exit_code == 0

    def test_list_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["list", "--help"])
        assert result.exit_code == 0
        assert "genres" in result.output or "spatial" in result.output
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_cli.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement CLI**

`src/narrative_data/cli.py`:
```python
"""CLI entry point for narrative-data tooling."""

import json

import click
from rich.console import Console
from rich.table import Table

from narrative_data.config import resolve_output_path
from narrative_data.pipeline.invalidation import load_manifest

console = Console()

# Resolve prompts directory relative to package
from pathlib import Path

_PROMPTS_DIR = Path(__file__).parent.parent.parent / "prompts"


@click.group()
def cli():
    """Narrative data elicitation tooling for storyteller research."""


# --- Genre commands ---


@cli.group()
def genre():
    """Genre region elicitation and structuring."""


@genre.command("elicit")
@click.option("--regions", default=None, help="Comma-separated region slugs")
@click.option("--categories", default=None, help="Comma-separated categories")
@click.option("--force", is_flag=True, help="Bypass staleness checks")
def genre_elicit(regions, categories, force):
    """Run Stage 1 elicitation for genre regions."""
    from narrative_data.genre.commands import elicit_genre

    output = resolve_output_path()
    elicit_genre(
        output_base=output,
        prompts_dir=_PROMPTS_DIR,
        regions=_parse_list(regions),
        categories=_parse_list(categories),
        force=force,
    )


@genre.command("structure")
@click.option("--regions", default=None, help="Comma-separated region slugs")
@click.option("--categories", default=None, help="Comma-separated categories")
@click.option("--force", is_flag=True, help="Force re-structuring")
def genre_structure(regions, categories, force):
    """Run Stage 2 structuring for genre regions."""
    from narrative_data.genre.commands import structure_genre

    output = resolve_output_path()
    structure_genre(
        output_base=output,
        regions=_parse_list(regions),
        categories=_parse_list(categories),
        force=force,
    )


# --- Spatial commands ---


@cli.group()
def spatial():
    """Setting type elicitation and structuring."""


@spatial.command("elicit")
@click.option("--settings", default=None, help="Comma-separated setting slugs")
@click.option("--force", is_flag=True, help="Bypass staleness checks")
def spatial_elicit(settings, force):
    """Run Stage 1 elicitation for setting types."""
    from narrative_data.spatial.commands import elicit_spatial

    output = resolve_output_path()
    elicit_spatial(
        output_base=output,
        prompts_dir=_PROMPTS_DIR,
        settings=_parse_list(settings),
        force=force,
    )


@spatial.command("structure")
@click.option("--settings", default=None, help="Comma-separated setting slugs")
@click.option("--force", is_flag=True, help="Force re-structuring")
def spatial_structure(settings, force):
    """Run Stage 2 structuring for setting types."""
    from narrative_data.spatial.commands import structure_spatial

    output = resolve_output_path()
    structure_spatial(
        output_base=output,
        settings=_parse_list(settings),
        force=force,
    )


# --- Cross-pollination ---


@cli.command("cross-pollinate")
@click.option("--force", is_flag=True, help="Force reprocessing")
def cross_pollinate(force):
    """Run B.3 cross-domain synthesis."""
    from narrative_data.cross_pollination.commands import run_cross_pollination

    output = resolve_output_path()
    run_cross_pollination(output_base=output, force=force)


# --- Status ---


@cli.command()
def status():
    """Show pipeline health: what needs work, what's stale."""
    output = resolve_output_path()
    _show_domain_status(output, "genres")
    _show_domain_status(output, "spatial")
    _show_domain_status(output, "intersections")


def _show_domain_status(output: Path, domain: str) -> None:
    manifest_path = output / domain / "manifest.json"
    manifest = load_manifest(manifest_path)
    entries = manifest.get("entries", {})

    if not entries:
        console.print(f"\n[yellow]{domain}: no entries yet[/yellow]")
        return

    table = Table(title=f"{domain.title()} Status")
    table.add_column("Entry", style="cyan")
    table.add_column("Stage", style="green")
    table.add_column("Generated", style="dim")

    for key, entry in sorted(entries.items()):
        table.add_row(key, entry.get("stage", "?"), entry.get("generated_at", "?"))

    console.print(table)


# --- List ---


@cli.group("list")
def list_cmd():
    """Query and inspect generated data."""


@list_cmd.command("genres")
@click.option("--region", default=None, help="Specific region slug")
@click.option("--category", default=None, help="Specific category")
@click.option("--format", "fmt", default="json", type=click.Choice(["json", "table"]))
def list_genres(region, category, fmt):
    """List genre data."""
    output = resolve_output_path()
    _list_domain_data(output / "genres", region, category, fmt)


@list_cmd.command("spatial")
@click.option("--setting", default=None, help="Specific setting slug")
@click.option("--category", default=None, help="Specific category")
@click.option("--format", "fmt", default="json", type=click.Choice(["json", "table"]))
def list_spatial(setting, category, fmt):
    """List spatial data."""
    output = resolve_output_path()
    _list_domain_data(output / "spatial", setting, category, fmt)


@list_cmd.command("intersections")
@click.option("--stale", is_flag=True, help="Show only stale intersections")
@click.option("--format", "fmt", default="json", type=click.Choice(["json", "table"]))
def list_intersections(stale, fmt):
    """List intersection data."""
    output = resolve_output_path()
    manifest = load_manifest(output / "intersections" / "manifest.json")
    entries = manifest.get("entries", {})
    if stale:
        entries = {k: v for k, v in entries.items() if v.get("stage") != "structured"}
    click.echo(json.dumps(entries, indent=2))


def _list_domain_data(
    domain_dir: Path, target: str | None, category: str | None, fmt: str
) -> None:
    """List data for a domain, optionally filtered."""
    if target and category:
        json_path = domain_dir / target / f"{category}.json"
        if json_path.exists():
            click.echo(json_path.read_text())
        else:
            click.echo(f"Not found: {json_path}")
    elif target:
        target_dir = domain_dir / target
        if target_dir.exists():
            files = sorted(target_dir.glob("*.json"))
            result = {f.stem: json.loads(f.read_text()) for f in files}
            click.echo(json.dumps(result, indent=2))
        else:
            click.echo(f"Not found: {target_dir}")
    else:
        manifest = load_manifest(domain_dir / "manifest.json")
        if fmt == "table":
            table = Table()
            table.add_column("Entry")
            table.add_column("Stage")
            for key, entry in sorted(manifest.get("entries", {}).items()):
                table.add_row(key, entry.get("stage", "?"))
            console.print(table)
        else:
            click.echo(json.dumps(manifest, indent=2))


def _parse_list(value: str | None) -> list[str] | None:
    """Parse a comma-separated string into a list, or return None."""
    if value is None:
        return None
    return [v.strip() for v in value.split(",") if v.strip()]
```

- [ ] **Step 4: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_cli.py -v`
Expected: All tests PASS.

- [ ] **Step 5: Verify CLI entry point**

Run: `cd tools/narrative-data && uv run narrative-data --help`
Expected: Shows help with genre, spatial, cross-pollinate, status, list subcommands.

- [ ] **Step 6: Run full test suite and ruff**

Run: `cd tools/narrative-data && uv run ruff check . && uv run pytest -v`
Expected: All lint clean, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add tools/narrative-data/src/narrative_data/cli.py tools/narrative-data/tests/test_cli.py
git commit -m "feat: add Click CLI with genre, spatial, status, and list subcommands"
```

---

### Task 11: Remaining Prompt Templates

**Files:**
- Create: `tools/narrative-data/prompts/genre/archetypes.md`
- Create: `tools/narrative-data/prompts/genre/tropes.md`
- Create: `tools/narrative-data/prompts/genre/narrative-shapes.md`
- Create: `tools/narrative-data/prompts/genre/dynamics.md`
- Create: `tools/narrative-data/prompts/genre/profiles.md`
- Create: `tools/narrative-data/prompts/genre/goals.md`
- Create: `tools/narrative-data/prompts/genre/settings.md`
- Create: `tools/narrative-data/prompts/spatial/place-entities.md`
- Create: `tools/narrative-data/prompts/spatial/topology.md`
- Create: `tools/narrative-data/prompts/spatial/tonal-inheritance.md`
- Create: `tools/narrative-data/prompts/cross-pollination/synthesis.md`

- [ ] **Step 1: Author genre category prompt templates**

Each prompt follows the same pattern as `region.md`: domain context, dimensional framework relevant to this category, `{target_name}` placeholder, and instruction to be expansive. See the spec for the schema fields each category should produce — the prompts should naturally elicit content that maps to those fields.

Key prompts to author:
- `archetypes.md` — "Express how standard character archetypes manifest differently within {target_name}. A mentor in this genre is not the same character as a mentor in other genres..."
- `tropes.md` — "Identify the trope inventory for {target_name}. For each trope, describe its narrative function, how it's typically reinforced, and known subversion patterns..."
- `narrative-shapes.md` — "What arc structures does {target_name} use? Describe the beats, their flexibility, and the tension profile..."
- `dynamics.md` — "How do relational dynamics between characters manifest in {target_name}? Describe how each role expresses, the relational texture, and typical escalation patterns..."
- `profiles.md` — "What scene types are characteristic of {target_name}? Describe how each scene type unfolds, its tension signature, and characteristic moments..."
- `goals.md` — "What narrative goals drive characters in {target_name}? How are they pursued, what does success/failure look like..."
- `settings.md` — "What settings and locations are characteristic of {target_name}? Describe the atmospheric and sensory vocabulary..."

- [ ] **Step 2: Author spatial category prompt templates**

- `place-entities.md` — "Generate the plausible place-entity inventory for {target_name}. Each place has a narrative function, communicability profile..."
- `topology.md` — "Describe the adjacency relationships between places in {target_name}. Include friction levels, permeability, tonal shift notes..."
- `tonal-inheritance.md` — "How does tone propagate across spatial boundaries in {target_name}? Describe the inheritance rules, friction levels, examples..."

- [ ] **Step 3: Author cross-pollination prompt template**

- `synthesis.md` — "Given the genre region data and spatial setting data below, identify enrichments, gaps, and new connections between the two domains..."

- [ ] **Step 4: Verify prompts load correctly**

Run: `cd tools/narrative-data && uv run python -c "from narrative_data.prompts import PromptBuilder; b = PromptBuilder(); [b.load_core_prompt('genre', c) for c in ['region','archetypes','tropes','narrative-shapes','dynamics','profiles','goals','settings']]; print('All genre prompts load OK')""`
Expected: "All genre prompts load OK"

- [ ] **Step 5: Commit**

```bash
git add tools/narrative-data/prompts/
git commit -m "feat: add all prompt templates for genre, spatial, and cross-pollination"
```

---

### Task 12: End-to-End Smoke Test and Schema Export

**Files:**
- Modify: `tools/narrative-data/tests/test_pipeline.py` (or create a new integration test file)

- [ ] **Step 1: Run the full test suite**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests PASS.

- [ ] **Step 2: Run ruff check**

Run: `cd tools/narrative-data && uv run ruff check .`
Expected: No errors.

- [ ] **Step 3: Verify CLI entry point works end-to-end**

Run: `cd tools/narrative-data && uv run narrative-data --help && uv run narrative-data genre --help && uv run narrative-data list --help && uv run narrative-data status`
Expected: All help texts display correctly. Status shows empty state.

- [ ] **Step 4: Export JSON Schemas to `meta/schemas/`**

Write a small script or add a CLI command to export schemas. For now, run directly:
```bash
cd tools/narrative-data && uv run python -c "
import json
from pathlib import Path
from narrative_data.config import resolve_output_path
from narrative_data.schemas.genre import GenreRegion, Trope, NarrativeShape, GenreArchetype, GenreDynamic, GenreProfile, GenreGoal, GenreSetting
from narrative_data.schemas.spatial import SettingType, PlaceEntity, TopologyEdge, TonalInheritanceRule
from narrative_data.schemas.intersections import IntersectionSynthesis

schemas_dir = resolve_output_path() / 'meta' / 'schemas'
schemas_dir.mkdir(parents=True, exist_ok=True)

for cls in [GenreRegion, Trope, NarrativeShape, GenreArchetype, GenreDynamic, GenreProfile, GenreGoal, GenreSetting, SettingType, PlaceEntity, TopologyEdge, TonalInheritanceRule, IntersectionSynthesis]:
    name = cls.__name__
    # Convert CamelCase to kebab-case
    kebab = ''.join(['-' + c.lower() if c.isupper() else c for c in name]).lstrip('-')
    path = schemas_dir / f'{kebab}.schema.json'
    path.write_text(json.dumps(cls.model_json_schema(), indent=2))
    print(f'Exported: {path.name}')
print('All schemas exported OK')
"
```
Expected: Schema files written to `storyteller-data/narrative-data/meta/schemas/`, confirmation printed.

- [ ] **Step 5: Write generation run log to `meta/runs/`**

Add a `write_run_log` function to `pipeline/invalidation.py` (or a new `pipeline/logging.py`):

```python
def write_run_log(output_base: Path, run_data: dict) -> Path:
    """Write a generation run log to meta/runs/."""
    runs_dir = output_base / "meta" / "runs"
    runs_dir.mkdir(parents=True, exist_ok=True)
    from narrative_data.utils import now_iso
    timestamp = now_iso().replace(":", "-")
    path = runs_dir / f"{timestamp}.json"
    path.write_text(json.dumps(run_data, indent=2))
    return path
```

Wire this into the `elicit_genre`, `elicit_spatial`, `structure_genre`, and `structure_spatial` functions — each writes a run log at the end with: timestamp, model versions, prompt versions used, cells processed, validation results.

- [ ] **Step 6: Final commit**

```bash
git add -A tools/narrative-data/
git commit -m "chore: finalize narrative-data package with schema export and run logging"
```

---

## Notes for Implementer

1. **Ollama must be running** for any actual elicitation (`narrative-data genre elicit`). Tests mock the Ollama client. The `status` and `list` commands work without Ollama.

2. **Prompt templates are creative artifacts.** Task 11 provides structural guidance but the actual prompt content should be reviewed with the project owner. The prompts should naturally elicit content that maps to the Pydantic schema fields.

3. **`STORYTELLER_DATA_PATH` must be set** to point at the `storyteller-data` repository. The `narrative-data/` subdirectory is created automatically.

4. **Cross-pollination (B.3)** is stubbed. Full implementation depends on B.1 and B.2 producing initial data to synthesize against.

5. **The `_PROMPTS_DIR` resolution** in `cli.py` uses a path relative to the source file. This works for `uv sync --dev` (editable install) and `uv run`. A non-editable install would need prompts included as package data in `pyproject.toml` — this is a known limitation acceptable for dev tooling that's only run from the source tree.

6. **Genre and spatial category prompt templates** (Task 11) are the highest-value creative work in the plan. The structural code (Tasks 1-10) is mechanical; the prompts determine elicitation quality.

7. **Task 9 command orchestration** should be tested with mocked Ollama. The review identified this gap — add tests for dependency ordering, context building, staleness checks, and manifest updates. Use the same mock patterns from `test_pipeline.py`.
