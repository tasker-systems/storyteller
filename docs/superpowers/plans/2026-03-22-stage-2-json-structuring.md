# Stage 2 JSON Structuring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract structured JSON from the 7.1MB narrative data corpus, producing sibling `.json` files validated by new Pydantic schemas for all 12 primitive types.

**Architecture:** Type-specific extraction prompts guide `qwen2.5:7b-instruct` to produce constrained JSON from raw markdown. Pydantic models enforce the 0.0–1.0 data contract and structural validity. Orchestration commands batch the work across genres and clusters with manifest-based caching.

**Tech Stack:** Python 3.11+, Pydantic v2, Click CLI, Ollama (qwen2.5:7b-instruct), ruff, pytest

**Spec:** `docs/superpowers/specs/2026-03-22-stage-2-json-structuring-design.md`

**Analysis source (JSON schemas):** `storyteller-data/narrative-data/analysis/2026-03-21-comprehensive-terrain-analysis.md` Section 7

---

## File Structure

### Files to Remove
```
tools/narrative-data/src/narrative_data/schemas/shared.py      — old NarrativeEntity base
tools/narrative-data/src/narrative_data/schemas/genre.py        — old genre schemas
tools/narrative-data/src/narrative_data/schemas/spatial.py      — old spatial schemas
tools/narrative-data/src/narrative_data/schemas/intersections.py — old intersection schemas
tools/narrative-data/src/narrative_data/schemas/__init__.py     — old re-exports
tools/narrative-data/tests/test_schemas.py                      — old schema tests (302 lines)
```

### Files to Create
```
# Schemas (13 files)
tools/narrative-data/src/narrative_data/schemas/__init__.py       — new re-exports
tools/narrative-data/src/narrative_data/schemas/shared.py         — ContinuousAxis, WeightedTags, StateVariableTemplate, etc.
tools/narrative-data/src/narrative_data/schemas/genre_dimensions.py
tools/narrative-data/src/narrative_data/schemas/archetypes.py
tools/narrative-data/src/narrative_data/schemas/dynamics.py
tools/narrative-data/src/narrative_data/schemas/goals.py
tools/narrative-data/src/narrative_data/schemas/archetype_dynamics.py
tools/narrative-data/src/narrative_data/schemas/scene_profiles.py
tools/narrative-data/src/narrative_data/schemas/ontological_posture.py
tools/narrative-data/src/narrative_data/schemas/settings.py
tools/narrative-data/src/narrative_data/schemas/spatial_topology.py
tools/narrative-data/src/narrative_data/schemas/place_entities.py
tools/narrative-data/src/narrative_data/schemas/tropes.py
tools/narrative-data/src/narrative_data/schemas/narrative_shapes.py

# Orchestration (1 file)
tools/narrative-data/src/narrative_data/pipeline/structure_commands.py

# Prompts — genre dimensions first, others in later tasks
tools/narrative-data/prompts/structure/genre-dimensions.md

# Tests
tools/narrative-data/tests/test_schemas_shared.py
tools/narrative-data/tests/test_schemas_genre_dimensions.py
tools/narrative-data/tests/test_schemas_types.py
tools/narrative-data/tests/test_structure_commands.py
tools/narrative-data/tests/test_prompts_structure.py

# Batch scripts
tools/narrative-data/scripts/run-structure-p1.sh

# Temporary scaffolding
tools/narrative-data/json-schemas/genre-dimensions.schema.json
```

### Files to Modify
```
tools/narrative-data/src/narrative_data/prompts.py          — add build_structure(), remove build_stage2()
tools/narrative-data/src/narrative_data/pipeline/structure.py — update run_structuring() to use build_structure()
tools/narrative-data/src/narrative_data/cli.py              — add structure subcommand, deprecate old structure commands
tools/narrative-data/src/narrative_data/genre/commands.py    — update schema imports (if referenced)
tools/narrative-data/src/narrative_data/spatial/commands.py  — update schema imports (if referenced)
tools/narrative-data/tests/test_pipeline.py                 — update GenreRegion import
tools/narrative-data/tests/test_prompts.py                  — add build_structure tests
```

---

### Task 1: Phase 0a — Rename `.raw.md` to `.md` across the corpus

**Files:**
- Modify: files in `storyteller-data/narrative-data/` (rename only, no content changes)
- Modify: `tools/narrative-data/src/narrative_data/genre/commands.py` (update `.raw.md` references)
- Modify: `tools/narrative-data/src/narrative_data/discovery/commands.py` (update `.raw.md` references)
- Modify: `tools/narrative-data/src/narrative_data/spatial/commands.py` (update `.raw.md` references)
- Modify: `tools/narrative-data/src/narrative_data/pipeline/elicit.py` (update `.raw.md` references)

- [ ] **Step 1: Check how many `.raw.md` files exist and audit references**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
find narrative-data -name "*.raw.md" | wc -l
```

Then in the storyteller repo:

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
grep -r "\.raw\.md" tools/narrative-data/src/ --include="*.py" -l
```

Expected: File list showing all Python files that reference `.raw.md`.

- [ ] **Step 2: Rename all `.raw.md` files to `.md` in storyteller-data**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
find narrative-data -name "*.raw.md" -exec sh -c 'mv "$1" "${1%.raw.md}.md"' _ {} \;
```

Verify: `find narrative-data -name "*.raw.md" | wc -l` should return 0.

- [ ] **Step 3: Update all `.raw.md` references in Python source to `.md`**

In each file found in Step 1, replace `.raw.md` with `.md`. Key patterns to find:
- `region.raw.md` → `region.md`
- `{genre_slug}.raw.md` → `{genre_slug}.md`
- `f"{slug}.raw.md"` → `f"{slug}.md"`
- `"tropes.raw.md"` → `"tropes.md"`
- `"narrative-shapes.raw.md"` → `"narrative-shapes.md"`

Also check and update any test files that reference `.raw.md`.

- [ ] **Step 4: Remove stale `region.json` files**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
find narrative-data/genres -name "region.json" -exec rm {} \;
```

- [ ] **Step 5: Run existing tests to verify nothing broke**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run pytest -x -q
```

Expected: All existing tests pass (tests don't read from storyteller-data at runtime — they use tmp fixtures).

- [ ] **Step 6: Commit in both repos**

```bash
# In storyteller-data
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add -A && git commit -m "refactor: rename .raw.md to .md — markdown artifacts are permanent"

# In storyteller
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add -A && git commit -m "refactor: update .raw.md references to .md after corpus rename"
```

---

### Task 2: Phase 0b — Remove old schemas and extract JSON schema scaffolding

**Files:**
- Remove: `tools/narrative-data/src/narrative_data/schemas/shared.py`
- Remove: `tools/narrative-data/src/narrative_data/schemas/genre.py`
- Remove: `tools/narrative-data/src/narrative_data/schemas/spatial.py`
- Remove: `tools/narrative-data/src/narrative_data/schemas/intersections.py`
- Remove: `tools/narrative-data/tests/test_schemas.py`
- Modify: `tools/narrative-data/src/narrative_data/schemas/__init__.py` — empty for now
- Modify: `tools/narrative-data/src/narrative_data/genre/commands.py` — remove old schema imports
- Modify: `tools/narrative-data/src/narrative_data/spatial/commands.py` — remove old schema imports
- Modify: `tools/narrative-data/tests/test_pipeline.py` — remove GenreRegion import
- Create: `tools/narrative-data/json-schemas/genre-dimensions.schema.json`

- [ ] **Step 1: Remove old schema files**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
rm src/narrative_data/schemas/shared.py
rm src/narrative_data/schemas/genre.py
rm src/narrative_data/schemas/spatial.py
rm src/narrative_data/schemas/intersections.py
rm tests/test_schemas.py
```

- [ ] **Step 2: Clear the schemas `__init__.py`**

Write an empty `__init__.py`:

```python
"""Narrative data schemas for Stage 2 JSON structuring."""
```

- [ ] **Step 3: Fix broken imports in source files**

In `genre/commands.py`, find and comment out or remove the import block referencing old schemas (lines ~26+). The `structure_genre` function in that file uses the old schemas — it will be replaced by the new `structure` CLI command. For now, make the import conditional or remove the function body.

In `spatial/commands.py`, same treatment for old schema imports.

In `tests/test_pipeline.py` (line 13), remove the `from narrative_data.schemas.genre import GenreRegion` import and any test that depends on it.

- [ ] **Step 4: Run tests to verify clean state**

```bash
uv run pytest -x -q
```

Expected: All remaining tests pass. Some tests removed with `test_schemas.py`; count will drop from ~120 to ~90.

- [ ] **Step 5: Extract JSON schema scaffolding from analysis**

Read Section 7 of `storyteller-data/narrative-data/analysis/2026-03-21-comprehensive-terrain-analysis.md` and extract the GenreDimensions JSON schema (7.1) to:

```
tools/narrative-data/json-schemas/genre-dimensions.schema.json
```

This is the primary scaffolding target — GenreDimensions is the most complex schema and the foundation type. For the remaining 11 types, the implementer should work directly from the analysis document's Section 7.2–7.11. The scaffolding is deliberately limited to GenreDimensions to avoid busywork — the analysis sections are structured clearly enough to serve as direct references for simpler types.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "refactor: remove old schemas, extract genre-dimensions JSON schema scaffold"
```

---

### Task 3: Shared schema primitives (`schemas/shared.py`)

**Files:**
- Create: `tools/narrative-data/src/narrative_data/schemas/shared.py`
- Create: `tools/narrative-data/tests/test_schemas_shared.py`

- [ ] **Step 1: Write failing tests for shared primitives**

```python
# tests/test_schemas_shared.py
"""Tests for shared schema primitives."""

import pytest
from pydantic import ValidationError


class TestContinuousAxis:
    def test_valid_axis(self):
        from narrative_data.schemas.shared import ContinuousAxis
        axis = ContinuousAxis(value=0.7, low_label="Spare", high_label="Lush")
        assert axis.value == 0.7
        assert axis.flavor_text is None

    def test_value_below_zero_rejected(self):
        from narrative_data.schemas.shared import ContinuousAxis
        with pytest.raises(ValidationError):
            ContinuousAxis(value=-0.1)

    def test_value_above_one_rejected(self):
        from narrative_data.schemas.shared import ContinuousAxis
        with pytest.raises(ValidationError):
            ContinuousAxis(value=1.1)

    def test_minimal_axis(self):
        from narrative_data.schemas.shared import ContinuousAxis
        axis = ContinuousAxis(value=0.5)
        assert axis.can_be_state_variable is False

    def test_round_trip(self):
        from narrative_data.schemas.shared import ContinuousAxis
        axis = ContinuousAxis(value=0.3, low_label="Low", high_label="High", can_be_state_variable=True, flavor_text="test")
        data = axis.model_dump()
        restored = ContinuousAxis.model_validate(data)
        assert restored == axis


class TestWeightedTags:
    def test_valid_tags(self):
        from narrative_data.schemas.shared import WeightedTags
        tags = WeightedTags(root={"Stewardship": 0.7, "Corruption": 0.3})
        assert tags.root["Stewardship"] == 0.7

    def test_value_out_of_range_rejected(self):
        from narrative_data.schemas.shared import WeightedTags
        with pytest.raises(ValidationError):
            WeightedTags(root={"Bad": 1.5})


class TestStateVariableTemplate:
    def test_valid_template(self):
        from narrative_data.schemas.shared import StateVariableTemplate
        sv = StateVariableTemplate(
            canonical_id="V1",
            genre_label="Sanity",
            behavior="depleting",
            initial_value=0.8,
            threshold=0.2,
            threshold_effect="Genre shifts to horror mode",
        )
        assert sv.behavior == "depleting"
        assert sv.initial_value == 0.8

    def test_threshold_out_of_range_rejected(self):
        from narrative_data.schemas.shared import StateVariableTemplate
        with pytest.raises(ValidationError):
            StateVariableTemplate(
                canonical_id="V1", genre_label="X", behavior="depleting", threshold=2.0
            )

    def test_minimal_template(self):
        from narrative_data.schemas.shared import StateVariableTemplate
        sv = StateVariableTemplate(canonical_id="V1", genre_label="Hope", behavior="fluctuating")
        assert sv.initial_value is None
        assert sv.threshold is None


class TestStateVariableInteraction:
    def test_valid_interaction(self):
        from narrative_data.schemas.shared import StateVariableInteraction
        svi = StateVariableInteraction(variable_id="V1", operation="consumes", description="Depletes sanity")
        assert svi.operation == "consumes"


class TestOverlapSignal:
    def test_valid_signal(self):
        from narrative_data.schemas.shared import OverlapSignal
        sig = OverlapSignal(adjacent_genre="cosmic-horror", similar_entity="The Witness", differentiator="Cosmic horror variant lacks warmth")
        assert sig.adjacent_genre == "cosmic-horror"


class TestGenreBoundary:
    def test_valid_boundary(self):
        from narrative_data.schemas.shared import GenreBoundary
        gb = GenreBoundary(trigger="Hope below 0.3", drift_target="horror", description="Genre shifts to horror mode")
        assert gb.drift_target == "horror"
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_schemas_shared.py -v
```

Expected: FAIL — `ModuleNotFoundError: No module named 'narrative_data.schemas.shared'`

- [ ] **Step 3: Implement shared primitives**

Create `src/narrative_data/schemas/shared.py`:

```python
"""Shared primitives for all narrative data schemas.

Design contract: all continuous numeric values are normalized 0.0-1.0 floats.
This is enforced by field validators and applies across all schema types.
"""

from typing import Literal

from pydantic import BaseModel, RootModel, field_validator


class ContinuousAxis(BaseModel):
    """Dimensional positioning with labels and prose."""

    value: float
    low_label: str | None = None
    high_label: str | None = None
    can_be_state_variable: bool = False
    flavor_text: str | None = None

    @field_validator("value")
    @classmethod
    def value_in_unit_interval(cls, v: float) -> float:
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"value must be 0.0-1.0, got {v}")
        return v


class WeightedTags(RootModel[dict[str, float]]):
    """Weighted tag set, e.g. {"Stewardship": 0.7, "Corruption": 0.3}."""

    @field_validator("root")
    @classmethod
    def values_in_unit_interval(cls, v: dict[str, float]) -> dict[str, float]:
        for tag, weight in v.items():
            if not 0.0 <= weight <= 1.0:
                raise ValueError(f"weight for '{tag}' must be 0.0-1.0, got {weight}")
        return v


class StateVariableTemplate(BaseModel):
    """Dynamic variable configuration per genre."""

    canonical_id: str
    genre_label: str
    behavior: Literal["depleting", "accumulating", "fluctuating", "progression", "countdown"]
    initial_value: float | None = None
    threshold: float | None = None
    threshold_effect: str | None = None
    activation_condition: str | None = None

    @field_validator("initial_value", "threshold")
    @classmethod
    def optional_unit_interval(cls, v: float | None) -> float | None:
        if v is not None and not 0.0 <= v <= 1.0:
            raise ValueError(f"value must be 0.0-1.0, got {v}")
        return v


class StateVariableInteraction(BaseModel):
    """How an entity affects a state variable."""

    variable_id: str
    operation: Literal["consumes", "accumulates", "depletes", "transforms", "gates"]
    description: str | None = None


class OverlapSignal(BaseModel):
    """Cross-genre boundary marker."""

    adjacent_genre: str
    similar_entity: str
    differentiator: str


class GenreBoundary(BaseModel):
    """Genre transition detection trigger."""

    trigger: str
    drift_target: str
    description: str
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
uv run pytest tests/test_schemas_shared.py -v
```

Expected: All tests PASS.

- [ ] **Step 5: Run ruff**

```bash
uv run ruff check src/narrative_data/schemas/shared.py tests/test_schemas_shared.py
uv run ruff format --check src/narrative_data/schemas/shared.py tests/test_schemas_shared.py
```

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add shared schema primitives with 0.0-1.0 data contract"
```

---

### Task 4: GenreDimensions schema

**Files:**
- Create: `tools/narrative-data/src/narrative_data/schemas/genre_dimensions.py`
- Create: `tools/narrative-data/tests/test_schemas_genre_dimensions.py`

This is the most complex schema — 34 dimensions grouped by category, weighted tags, state variables, narrative contracts, and boundaries.

- [ ] **Step 1: Write failing tests**

Test file: `tests/test_schemas_genre_dimensions.py`. Tests should cover:
- Constructing a minimal valid `GenreDimensions` with all required field groups
- Round-trip (construct → dump → validate)
- Continuous axes within dimension groups enforce 0.0–1.0
- `WeightedTags` in thematic section
- `StateVariableTemplate` list
- `GenreBoundary` list
- Narrative contracts (list of objects with `invariant` and `enforced` fields)
- Classification enum (`standalone_region`, `constraint_layer`, `hybrid_modifier`)
- `constraint_layer_type` and `modifies` fields for constraint layers
- `locus_of_power` as ranked list (max 3 items)

Reference `json-schemas/genre-dimensions.schema.json` for the complete field set.

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_schemas_genre_dimensions.py -v
```

- [ ] **Step 3: Implement GenreDimensions schema**

Create `src/narrative_data/schemas/genre_dimensions.py`. Structure the model with nested sub-models for each dimension group:
- `AestheticDimensions` (sensory_density, groundedness, aesthetic_register as ContinuousAxis; prose_register as list[str])
- `TonalDimensions` (emotional_contract, cynicism_earnestness, surface_irony, structural_irony, intimacy_distance)
- `TemporalDimensions` (time_structure, pacing, temporal_grounding, narrative_span)
- `ThematicDimensions` (power_treatment, identity_treatment, knowledge_treatment, connection_treatment as WeightedTags)
- `AgencyDimensions` (agency_level as ContinuousAxis, agency_type as Literal enum, triumph_mode, competence_relevance)
- `WorldAffordances` (magic as list[str], technology, violence, death, supernatural as str)
- `EpistemologicalDimensions` (knowability, knowledge_reward, narration_reliability)
- `NarrativeContract` (invariant: str, enforced: bool)
- `GenreDimensions` — top-level model composing all sub-models

All ContinuousAxis fields inherit the 0.0–1.0 validation from shared.py.

- [ ] **Step 4: Run tests to verify they pass**

```bash
uv run pytest tests/test_schemas_genre_dimensions.py -v
```

- [ ] **Step 5: Compare model_json_schema() against scaffolding**

```python
import json
from narrative_data.schemas.genre_dimensions import GenreDimensions
print(json.dumps(GenreDimensions.model_json_schema(), indent=2))
```

Compare output against `json-schemas/genre-dimensions.schema.json`. Document any intentional drift.

- [ ] **Step 6: Run ruff and commit**

```bash
uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add GenreDimensions schema — 34 dimensions with grouped sub-models"
```

---

### Task 5: Discovery type schemas (archetypes, dynamics, goals, settings, scene profiles, ontological posture)

**Files:**
- Create: `tools/narrative-data/src/narrative_data/schemas/archetypes.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/dynamics.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/goals.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/settings.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/scene_profiles.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/ontological_posture.py`
- Create: `tools/narrative-data/tests/test_schemas_types.py`

Each schema file contains both a per-genre model and a cluster model. Reference analysis Section 7 for complete field sets.

- [ ] **Step 1: Write failing tests for all 6 schema modules**

Test `tests/test_schemas_discovery.py` with one class per schema module. Each class tests:
- Minimal valid construction
- Round-trip
- Required field enforcement
- 0.0–1.0 validation where applicable
- Cluster variant construction (canonical_name + genre_variants list)

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_schemas_discovery.py -v
```

- [ ] **Step 3: Implement all 6 schema modules**

For each module, follow the analysis schemas. Key patterns:
- **Archetype:** `PersonalityProfile` nested model (7 axes as floats with 0.0–1.0 validators), `extended_axes: dict[str, float]`, `ClusterArchetype` with `genre_variants` list
- **Dynamic:** Scale enum, directionality enum, `RoleSlot` nested model, valence enum, `ClusterDynamic`
- **Goal:** Scale enum, `CrossScaleTension` nested model, `ClusterGoal`
- **Settings:** Atmospheric palette, sensory vocabulary, communicability dimensions
- **SceneProfile:** `DimensionalProperties` nested model (all enums), uniqueness enum
- **OntologicalPosture:** `ModeOfBeing`, `SelfOtherBoundary` nested models, ethical orientation

- [ ] **Step 4: Run tests to verify they pass**

```bash
uv run pytest tests/test_schemas_discovery.py -v
```

- [ ] **Step 5: Run ruff and commit**

```bash
uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add 6 discovery type schemas with per-genre and cluster variants"
```

---

### Task 6: Remaining type schemas (archetype-dynamics, spatial-topology, place-entities, tropes, narrative-shapes)

**Files:**
- Create: `tools/narrative-data/src/narrative_data/schemas/archetype_dynamics.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/spatial_topology.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/place_entities.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/tropes.py`
- Create: `tools/narrative-data/src/narrative_data/schemas/narrative_shapes.py`
- Append: `tools/narrative-data/tests/test_schemas_types.py` (add test classes for these types)

- [ ] **Step 1: Write failing tests**

Add test classes to `test_schemas_discovery.py` for the 5 remaining types. Key patterns:
- **ArchetypeDynamic:** archetype_a/b refs, `EdgeProperties` nested model, `CharacteristicScene`, `ShadowPairing`
- **SpatialTopology:** `Friction` (type + level), `Directionality`, `TonalInheritance` nested models, `TraversalCost`
- **PlaceEntity:** `Communicability` with 4 channel sub-models (atmospheric, sensory, spatial, temporal), `EntityProperties`
- **Trope:** `narrative_function` as list of Literal enums, `TropeVariants` (straight/inverted/deconstructed/violation)
- **NarrativeShape:** `TensionProfile`, `Beat` (position 0.0–1.0, flexibility enum, tension_effect enum, state_thresholds dict), `RestBeat`, `Composability`

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_schemas_discovery.py -v -k "ArchetypeDynamic or SpatialTopology or PlaceEntity or Trope or NarrativeShape"
```

- [ ] **Step 3: Implement all 5 schema modules**

Follow analysis Section 7.5–7.11 for complete field sets.

- [ ] **Step 4: Run tests to verify they pass**

```bash
uv run pytest tests/test_schemas_discovery.py -v
```

- [ ] **Step 5: Update schemas `__init__.py` with all re-exports**

```python
"""Narrative data schemas for Stage 2 JSON structuring."""

from narrative_data.schemas.shared import (
    ContinuousAxis,
    GenreBoundary,
    OverlapSignal,
    StateVariableInteraction,
    StateVariableTemplate,
    WeightedTags,
)
# ... re-export all schema types
```

- [ ] **Step 6: Remove JSON schema scaffolding**

```bash
rm -rf tools/narrative-data/json-schemas/
```

The scaffolding has served its purpose — Pydantic models are validated.

- [ ] **Step 7: Run full test suite, ruff, and commit**

```bash
uv run pytest -x -q && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: complete all 12 type schemas, remove JSON schema scaffolding"
```

---

### Task 7: PromptBuilder.build_structure() and genre-dimensions extraction prompt

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/prompts.py`
- Create: `tools/narrative-data/prompts/structure/genre-dimensions.md`
- Create: `tools/narrative-data/tests/test_prompts_structure.py`

- [ ] **Step 1: Write failing tests for build_structure**

```python
# tests/test_prompts_structure.py
"""Tests for PromptBuilder.build_structure()."""

from pathlib import Path

import pytest

from narrative_data.prompts import PromptBuilder


@pytest.fixture
def structure_prompts_dir(tmp_path: Path) -> Path:
    """Create a minimal prompts directory with structure templates."""
    struct_dir = tmp_path / "structure"
    struct_dir.mkdir()
    (struct_dir / "genre-dimensions.md").write_text(
        "Extract genre dimensions from:\n\n{raw_content}\n\nTarget schema:\n{schema}"
    )
    return tmp_path


class TestBuildStructure:
    def test_loads_template_and_injects(self, structure_prompts_dir: Path):
        builder = PromptBuilder(prompts_dir=structure_prompts_dir)
        result = builder.build_structure(
            structure_type="genre-dimensions",
            raw_content="Some raw markdown",
            schema={"type": "object"},
        )
        assert "Some raw markdown" in result
        assert '"type": "object"' in result

    def test_missing_template_raises(self, structure_prompts_dir: Path):
        builder = PromptBuilder(prompts_dir=structure_prompts_dir)
        with pytest.raises(FileNotFoundError):
            builder.build_structure(
                structure_type="nonexistent",
                raw_content="content",
                schema={},
            )

    def test_cluster_template(self, structure_prompts_dir: Path):
        struct_dir = structure_prompts_dir / "structure"
        (struct_dir / "archetypes-cluster.md").write_text(
            "Extract cluster archetypes from:\n\n{raw_content}\n\nSchema:\n{schema}"
        )
        builder = PromptBuilder(prompts_dir=structure_prompts_dir)
        result = builder.build_structure(
            structure_type="archetypes-cluster",
            raw_content="Cluster content",
            schema={"type": "array"},
        )
        assert "Cluster content" in result
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_prompts_structure.py -v
```

- [ ] **Step 3: Implement build_structure() on PromptBuilder**

Add to `prompts.py`:

```python
def build_structure(
    self,
    structure_type: str,
    raw_content: str,
    schema: dict,
) -> str:
    """Build a type-specific structuring prompt for the 7b model."""
    template_path = self.prompts_dir / "structure" / f"{structure_type}.md"
    if not template_path.exists():
        raise FileNotFoundError(f"Structure prompt template not found: {template_path}")
    prompt = template_path.read_text()
    prompt = prompt.replace("{raw_content}", raw_content)
    prompt = prompt.replace("{schema}", json.dumps(schema, indent=2))
    return prompt
```

Remove the `build_stage2` static method.

- [ ] **Step 4: Write the genre-dimensions extraction prompt**

Create `prompts/structure/genre-dimensions.md`. This is a focused prompt for the 7b model:
- Role: structured data extractor for genre dimensional analysis
- Specific guidance for mapping prose descriptions to continuous axis values (0.0–1.0)
- Guidance for weighted tag extraction (identify 1–3 primary treatments, assign weights)
- Guidance for state variable template extraction (identify behavior type, initial values, thresholds)
- Enum mapping hints for classification, agency_type, world affordances
- Preserve analytical prose in flavor_text fields
- Output format: single JSON object matching the provided schema

- [ ] **Step 5: Run tests to verify they pass**

```bash
uv run pytest tests/test_prompts_structure.py -v
```

- [ ] **Step 6: Run ruff and commit**

```bash
uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add build_structure() to PromptBuilder with genre-dimensions prompt"
```

---

### Task 8: Update run_structuring() to use build_structure()

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/structure.py`
- Modify: `tools/narrative-data/tests/test_pipeline.py`

- [ ] **Step 1: Write a test for replace-on-retry behavior**

Before changing the implementation, add a test that verifies error context is *replaced* not *accumulated*:

```python
def test_retry_replaces_error_section_not_appends(mock_client, tmp_path, ...):
    """Errors are replaced on retry to protect the 7b model's context window."""
    # Set up mock to fail twice with different errors, then succeed
    # After retries, verify the prompt passed to the final call contains
    # only the LAST error message, not all accumulated errors
```

- [ ] **Step 2: Update run_structuring() signature and implementation**

The function needs to accept a `PromptBuilder` and `structure_type` instead of using the static `build_stage2`. Update the retry logic to replace error sections rather than append.

```python
def run_structuring(
    client: OllamaClient,
    raw_path: Path,
    output_path: Path,
    schema_type: type[BaseModel],
    prompt_builder: PromptBuilder,
    structure_type: str,
    model: str = STRUCTURING_MODEL,
    is_collection: bool = True,
    max_retries: int = 3,
) -> dict[str, Any]:
```

The retry loop should build a fresh prompt each time, replacing the error section:

```python
ERROR_MARKER = "\n\n--- VALIDATION ERRORS ---\n"
for attempt in range(max_retries):
    # Build base prompt (without errors) each time
    base_prompt = prompt_builder.build_structure(structure_type, raw_content, target_schema)
    if errors:
        # Replace, don't accumulate — only the most recent error
        current_prompt = base_prompt + ERROR_MARKER + errors[-1] + "\nPlease fix."
    else:
        current_prompt = base_prompt
    ...
```

**Note on `is_collection`:** GenreDimensions uses `is_collection=False` (single object per genre file). All discovery types use `is_collection=True` (array of entities per file). Genre-native types (tropes, narrative-shapes) also use `is_collection=True`. The TYPE_REGISTRY in Task 9 will specify this per type.

- [ ] **Step 3: Verify no other code references build_stage2**

```bash
grep -r "build_stage2" tools/narrative-data/src/ tools/narrative-data/tests/ --include="*.py"
```

Remove any remaining references.

- [ ] **Step 4: Update test_pipeline.py**

Remove old `GenreRegion` import. Update any tests that call `run_structuring` to pass the new `prompt_builder` and `structure_type` parameters. Use a mock prompt builder or create minimal structure prompt templates in the test fixture.

- [ ] **Step 3: Run tests**

```bash
uv run pytest tests/test_pipeline.py -v
```

- [ ] **Step 4: Run full suite and commit**

```bash
uv run pytest -x -q && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "refactor: update run_structuring to use build_structure with replace-on-retry"
```

---

### Task 9: Structure orchestration commands

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/structure_commands.py`
- Create: `tools/narrative-data/tests/test_structure_commands.py`

- [ ] **Step 1: Write failing tests**

Test the orchestration logic with mocked OllamaClient:
- `structure_type()` — given a type slug, structures all per-genre files for that type
- Skips files already in manifest (cache hit)
- Handles missing `.md` files gracefully
- `structure_clusters()` — given a type slug, structures all cluster synthesis files
- Event logging (structure_started / structure_completed events)

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_structure_commands.py -v
```

- [ ] **Step 3: Implement structure_commands.py**

Key components:
- `TYPE_REGISTRY`: maps CLI type slugs to `(per_genre_schema, cluster_schema, data_dir, is_collection, prompt_slug)` tuples. This is the central lookup table:

```python
TYPE_REGISTRY = {
    "genre-dimensions": TypeConfig(
        per_genre=GenreDimensions, cluster=None,
        data_dir="genres", file_pattern="region",
        is_collection=False, prompt_slug="genre-dimensions",
    ),
    "archetypes": TypeConfig(
        per_genre=Archetype, cluster=ClusterArchetype,
        data_dir="discovery/archetypes",
        is_collection=True, prompt_slug="archetypes",
    ),
    # ... etc for all 12 types
    # NOTE: "profiles" is the CLI/file slug, maps to SceneProfile schema
    "profiles": TypeConfig(
        per_genre=SceneProfile, cluster=ClusterSceneProfile,
        data_dir="discovery/profiles",
        is_collection=True, prompt_slug="profiles",
    ),
}
```

- `structure_type()`: iterates genres, resolves `.md` → `.json` paths, calls `run_structuring()`, logs events
- `structure_clusters()`: iterates clusters, resolves `cluster-{name}.md` → `cluster-{name}.json`, calls `run_structuring()`
- Both functions support `--force` and `--plan` modes
- Console output with rich (cyan for processing, green for success, dim for skipped, red for failed)
- Summary at end: succeeded/failed/skipped counts

**Naming note:** The CLI uses `profiles` as the type slug (matching existing data paths and config). The Pydantic schema module is `scene_profiles.py`. The prompt template is `prompts/structure/profiles.md`. This is consistent with the spec's Section 3.2 naming note.

- [ ] **Step 4: Run tests to verify they pass**

```bash
uv run pytest tests/test_structure_commands.py -v
```

- [ ] **Step 5: Run ruff and commit**

```bash
uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add structure orchestration commands with type registry"
```

---

### Task 10: CLI `structure` subcommand

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/cli.py`
- Create: `tools/narrative-data/tests/test_cli_structure.py`

- [ ] **Step 1: Write failing tests**

Test the Click CLI integration using `CliRunner`:
- `narrative-data structure genre-dimensions --all` invokes the right command
- `--genre` flag filters to a single genre
- `--clusters` flag triggers cluster structuring
- `--force` flag is passed through
- `--plan` flag shows plan without executing

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_cli_structure.py -v
```

- [ ] **Step 3: Implement the structure subcommand**

Add a new `@cli.group()` for `structure`, with a command per type (or a single command that takes type as argument). Follow the existing CLI patterns in `cli.py`.

```python
@cli.group()
def structure() -> None:
    """Stage 2: Structure raw markdown into validated JSON."""


@structure.command("genre-dimensions")
@click.option("--genre", default=None, help="Single genre slug.")
@click.option("--all", "all_genres", is_flag=True, help="Structure all genres.")
@click.option("--force", is_flag=True, help="Re-structure even if cached.")
@click.option("--plan", "plan_only", is_flag=True, help="Show plan without executing.")
def structure_genre_dimensions(genre, all_genres, force, plan_only):
    ...
```

Also deprecate the old `genre structure` and `spatial structure` commands by adding deprecation warnings to their docstrings or removing them.

- [ ] **Step 4: Run tests to verify they pass**

```bash
uv run pytest tests/test_cli_structure.py -v
```

- [ ] **Step 5: Run full test suite, ruff, and commit**

```bash
uv run pytest -x -q && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add structure CLI subcommand for Stage 2 extraction"
```

---

### Task 11: Batch scripts and P1 smoke test

**Files:**
- Create: `tools/narrative-data/scripts/run-structure-p1.sh`

- [ ] **Step 1: Write the P1 batch script**

```bash
#!/usr/bin/env bash
# Stage 2 Phase 1: Structure genre dimensions (foundation)
set -euo pipefail

echo "=== Stage 2 Phase 1: Genre Dimensions ==="
narrative-data structure genre-dimensions --all
echo "=== Phase 1 Complete ==="
```

```bash
chmod +x tools/narrative-data/scripts/run-structure-p1.sh
```

- [ ] **Step 2: Smoke test with a single genre (requires Ollama)**

This step requires a running Ollama instance with `qwen2.5:7b-instruct`. If available:

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
narrative-data structure genre-dimensions --genre folk-horror
```

Verify:
- `storyteller-data/narrative-data/genres/folk-horror/region.json` is created
- The JSON validates against the GenreDimensions schema
- Continuous axis values are 0.0–1.0
- Weighted tags are present and valid
- State variables have correct behavior types

If Ollama is not running, skip this step — the integration test in the test suite (feature-gated behind `test-llm`) will cover this.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add P1 batch script for genre dimensions structuring"
```

---

### Task 12: Remaining extraction prompts (Phase 2–4 types)

**Files:**
- Create: `tools/narrative-data/prompts/structure/archetypes.md`
- Create: `tools/narrative-data/prompts/structure/archetypes-cluster.md`
- Create: `tools/narrative-data/prompts/structure/dynamics.md`
- Create: `tools/narrative-data/prompts/structure/dynamics-cluster.md`
- Create: `tools/narrative-data/prompts/structure/goals.md`
- Create: `tools/narrative-data/prompts/structure/goals-cluster.md`
- Create: `tools/narrative-data/prompts/structure/settings.md`
- Create: `tools/narrative-data/prompts/structure/settings-cluster.md`
- Create: `tools/narrative-data/prompts/structure/profiles.md`
- Create: `tools/narrative-data/prompts/structure/profiles-cluster.md`
- Create: `tools/narrative-data/prompts/structure/ontological-posture.md`
- Create: `tools/narrative-data/prompts/structure/ontological-posture-cluster.md`
- Create: `tools/narrative-data/prompts/structure/archetype-dynamics.md`
- Create: `tools/narrative-data/prompts/structure/archetype-dynamics-cluster.md`
- Create: `tools/narrative-data/prompts/structure/spatial-topology.md`
- Create: `tools/narrative-data/prompts/structure/spatial-topology-cluster.md`
- Create: `tools/narrative-data/prompts/structure/place-entities.md`
- Create: `tools/narrative-data/prompts/structure/place-entities-cluster.md`
- Create: `tools/narrative-data/prompts/structure/tropes.md`
- Create: `tools/narrative-data/prompts/structure/narrative-shapes.md`

Each prompt follows the same focused pattern as the genre-dimensions prompt but with type-specific field guidance. Key differences per type:

- **Archetypes:** Guide extraction of personality_profile (7 axes as 0.0–1.0 floats), distinguishing_tension as prose, overlap_signals as structured objects
- **Dynamics:** Multi-scale extraction (orbital/arc/scene), edge type and directionality enums, role_slots with want/withhold
- **Goals:** Cross-scale tension extraction, state variable interaction mapping
- **Settings:** Atmospheric palette and sensory vocabulary as lists, communicability dimensions
- **Profiles:** Dimensional properties as enum selections (tension_signature, emotional_register, pacing, etc.)
- **Ontological posture:** Modes of being list, self-other boundary stability enum, ethical permitted/forbidden lists
- **Archetype-dynamics:** Characteristic scene extraction, shadow pairing with drift_target_genre
- **Spatial topology:** Friction type + level enums, tonal inheritance direction, traversal cost as state variable deltas
- **Place entities:** 4-channel communicability extraction, entity properties booleans
- **Tropes:** Narrative function enum list, 4 variant descriptions (straight/inverted/deconstructed/violation)
- **Narrative shapes:** Beat extraction with position (0.0–1.0), flexibility + tension_effect enums, state_thresholds dict

Cluster variants (`*-cluster.md`) add: canonical name extraction, genre variant list construction, uniqueness classification, navigational description guidance.

- [ ] **Step 1: Write all per-genre extraction prompts (10 files)**

Each ~40-80 lines of focused extraction instructions.

- [ ] **Step 2: Write all cluster extraction prompts (8 files)**

- [ ] **Step 3: Verify all prompts load via PromptBuilder**

```python
from narrative_data.prompts import PromptBuilder
pb = PromptBuilder()
for t in ["archetypes", "dynamics", "goals", "settings", "profiles", "ontological-posture",
          "archetype-dynamics", "spatial-topology", "place-entities", "tropes", "narrative-shapes"]:
    pb.build_structure(t, "test", {})
    if t not in ["tropes", "narrative-shapes"]:
        pb.build_structure(f"{t}-cluster", "test", {})
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add all Stage 2 extraction prompts for 12 types"
```

---

### Task 13: Remaining batch scripts (P2–P4)

**Files:**
- Create: `tools/narrative-data/scripts/run-structure-p2.sh`
- Create: `tools/narrative-data/scripts/run-structure-p3.sh`
- Create: `tools/narrative-data/scripts/run-structure-p4.sh`

- [ ] **Step 1: Write P2, P3, P4 batch scripts**

Follow the same pattern as P1. See design spec Section 6 for the exact commands per phase.

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: add P2-P4 batch scripts for full Stage 2 extraction"
```

---

### Task 14: Final verification and cleanup

- [ ] **Step 1: Run full test suite**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run pytest -v
```

Expected: All tests pass. Count should be higher than the starting ~120 (old tests removed, new tests added).

- [ ] **Step 2: Run ruff across everything**

```bash
uv run ruff check . && uv run ruff format --check .
```

- [ ] **Step 3: Verify no old schema references remain**

```bash
grep -r "NarrativeEntity\|DimensionalPosition\|GenerationProvenance\|ProvenanceEdge" src/ tests/ --include="*.py"
```

Expected: No matches (all old types removed).

- [ ] **Step 4: Verify prompt templates all exist**

```bash
ls -la prompts/structure/
```

Expected: 21 files (1 genre-dimensions + 10 per-genre + 8 cluster + 2 genre-native).

- [ ] **Step 5: Update `narrative-data status` to show structuring progress**

The spec (Section 8) requires `narrative-data status` to show structuring progress. Update the `status` command in `cli.py` to count `.json` sibling files alongside `.md` files per type, showing how many have been structured. This is a small addition to the existing `status` command, not a new command.

- [ ] **Step 6: Update memory with project status**

Update `memory/project_tier_b_narrative_data.md` with Stage 2 structuring status.

- [ ] **Step 7: Final commit**

```bash
git add -A && git commit -m "chore: Stage 2 JSON structuring pipeline complete — ready for extraction runs"
```

---

## Future: Rust Type Generation via typify

Once Stage 2 is complete and all JSON is validated, the Pydantic schemas can produce JSON Schema via `model_json_schema()`, which the [`typify`](https://crates.io/crates/typify) crate can consume to generate Rust types. This creates a `Pydantic model → JSON Schema → typify → Rust types` chain ensuring cohesion across the Python data pipeline and the Rust engine. This is out of scope for this plan but should be the next integration step after structuring is validated.
