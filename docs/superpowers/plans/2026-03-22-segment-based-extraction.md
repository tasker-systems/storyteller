# Segment-Based Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic segmentation before LLM extraction so each model call receives focused context (~1-3K tokens) and a sub-schema, replacing the monolithic whole-file approach that exceeded the 7b model's semantic inference capacity.

**Architecture:** A Python slicer splits markdown files into segments by heading/structure boundaries. Each segment is extracted independently with a sub-schema-specific prompt. An aggregator merges segment JSONs into the type-level object with Pydantic validation. The CLI interface is unchanged — segmentation is an internal pipeline concern.

**Tech Stack:** Python 3.11+, Pydantic v2, Click CLI, Ollama (qwen2.5:7b-instruct), ruff, pytest

**Spec:** `docs/superpowers/specs/2026-03-22-segment-based-extraction-design.md`
**Parent spec:** `docs/superpowers/specs/2026-03-22-stage-2-json-structuring-design.md`

---

## File Structure

### Files to Create
```
# Slicer
tools/narrative-data/src/narrative_data/pipeline/slicer.py

# Aggregator
tools/narrative-data/src/narrative_data/pipeline/aggregator.py

# Segment extraction prompts (17 files)
tools/narrative-data/prompts/structure/segments/genre-region-meta.md
tools/narrative-data/prompts/structure/segments/genre-region-aesthetic.md
tools/narrative-data/prompts/structure/segments/genre-region-tonal.md
tools/narrative-data/prompts/structure/segments/genre-region-temporal.md
tools/narrative-data/prompts/structure/segments/genre-region-thematic.md
tools/narrative-data/prompts/structure/segments/genre-region-agency.md
tools/narrative-data/prompts/structure/segments/genre-region-epistemological.md
tools/narrative-data/prompts/structure/segments/genre-region-world-affordances.md
tools/narrative-data/prompts/structure/segments/genre-region-locus-of-power.md
tools/narrative-data/prompts/structure/segments/genre-region-narrative-structure.md
tools/narrative-data/prompts/structure/segments/genre-region-narrative-contracts.md
tools/narrative-data/prompts/structure/segments/genre-region-state-variables.md
tools/narrative-data/prompts/structure/segments/genre-region-boundaries.md
tools/narrative-data/prompts/structure/segments/discovery-entity.md
tools/narrative-data/prompts/structure/segments/discovery-entity-cluster.md
tools/narrative-data/prompts/structure/segments/trope-entity.md
tools/narrative-data/prompts/structure/segments/narrative-shape-entity.md

# Tests
tools/narrative-data/tests/test_slicer.py
tools/narrative-data/tests/test_aggregator.py
tools/narrative-data/tests/fixtures/sample-region.md
tools/narrative-data/tests/fixtures/sample-archetypes.md
tools/narrative-data/tests/fixtures/sample-cluster-archetypes.md
tools/narrative-data/tests/fixtures/sample-narrative-shapes.md
```

### Files to Modify
```
tools/narrative-data/src/narrative_data/pipeline/structure.py          — add run_segment_extraction() for segment-level calls
tools/narrative-data/src/narrative_data/pipeline/structure_commands.py  — rewire to slice → extract → aggregate
tools/narrative-data/src/narrative_data/prompts.py                     — add build_segment_structure()
tools/narrative-data/tests/test_structure_commands.py                  — update for segment-based flow
tools/narrative-data/tests/test_pipeline.py                            — add run_segment_extraction tests
```

---

### Task 1: Slicer — genre region segmentation

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/slicer.py`
- Create: `tools/narrative-data/tests/test_slicer.py`
- Create: `tools/narrative-data/tests/fixtures/sample-region.md`

This is the core segmentation logic. Start with genre regions since they're the most complex segmentation pattern (splitting by dimension group within sections). Other document types are simpler and added in Task 2.

- [ ] **Step 1: Create a sample genre region fixture**

Create `tests/fixtures/sample-region.md` — a minimal but structurally complete genre region document. Include:
- H1 title (`# Genre Region: Test Horror`)
- `## 1. Dimensional Positions` with **Aesthetic dimensions**, **Tonal dimensions**, **Temporal dimensions**, **Thematic dimensions**, **Agency dimensions** blocks
- `**Locus of power**` section with Primary/Secondary/Tertiary
- `**Narrative structure**` section
- Epistemological section
- World affordances section
- Genre contracts / narrative contracts
- `## 3. State Variables` section
- `## 4. Genre Topology` / boundaries section
- `### _commentary` and `### _suggestions` at the end (should be dropped)

Use the structure from the real `folk-horror/region.md` but with abbreviated content (~80-100 lines total). The fixture must have all the structural markers the slicer needs to detect.

- [ ] **Step 2: Write failing tests for genre region slicing**

Create `tests/test_slicer.py`:

```python
"""Tests for deterministic markdown segmentation."""

import pytest
from pathlib import Path

from narrative_data.pipeline.slicer import SegmentInfo, slice_genre_region


@pytest.fixture
def sample_region(tmp_path: Path) -> Path:
    src = Path(__file__).parent / "fixtures" / "sample-region.md"
    dest = tmp_path / "region.md"
    dest.write_text(src.read_text())
    return dest


class TestSliceGenreRegion:
    def test_produces_expected_segments(self, sample_region: Path, tmp_path: Path):
        output_dir = tmp_path / "region"
        segments = slice_genre_region(sample_region, output_dir)
        names = [s.name for s in segments]
        assert "meta" in names
        assert "aesthetic" in names
        assert "tonal" in names
        assert "temporal" in names
        assert "thematic" in names
        assert "agency" in names
        assert "epistemological" in names
        assert "world-affordances" in names
        assert "locus-of-power" in names
        assert "narrative-structure" in names
        assert "narrative-contracts" in names
        assert "state-variables" in names
        assert "boundaries" in names

    def test_segment_files_exist(self, sample_region: Path, tmp_path: Path):
        output_dir = tmp_path / "region"
        segments = slice_genre_region(sample_region, output_dir)
        for seg in segments:
            assert seg.path.exists()
            content = seg.path.read_text()
            assert len(content) > 0

    def test_frontmatter_present(self, sample_region: Path, tmp_path: Path):
        output_dir = tmp_path / "region"
        segments = slice_genre_region(sample_region, output_dir)
        for seg in segments:
            content = seg.path.read_text()
            assert content.startswith("---\n")
            assert "source:" in content
            assert "segment:" in content
            assert "lines:" in content

    def test_commentary_dropped(self, sample_region: Path, tmp_path: Path):
        output_dir = tmp_path / "region"
        segments = slice_genre_region(sample_region, output_dir)
        names = [s.name for s in segments]
        assert "commentary" not in names
        assert "suggestions" not in names
        # Also verify no segment contains commentary content
        for seg in segments:
            content = seg.path.read_text()
            assert "_commentary" not in content
            assert "_suggestions" not in content

    def test_segment_count(self, sample_region: Path, tmp_path: Path):
        output_dir = tmp_path / "region"
        segments = slice_genre_region(sample_region, output_dir)
        assert len(segments) == 13  # meta + 12 dimension/structure segments

    def test_idempotent(self, sample_region: Path, tmp_path: Path):
        output_dir = tmp_path / "region"
        segments1 = slice_genre_region(sample_region, output_dir)
        segments2 = slice_genre_region(sample_region, output_dir, force=True)
        assert len(segments1) == len(segments2)
        for s1, s2 in zip(segments1, segments2):
            assert s1.path.read_text() == s2.path.read_text()
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
uv run pytest tests/test_slicer.py -v
```

- [ ] **Step 4: Implement SegmentInfo and slice_genre_region**

Create `src/narrative_data/pipeline/slicer.py`:

```python
"""Deterministic markdown segmentation — no LLM involved.

Splits structured markdown files into focused segments for extraction.
Each segment maps to one Pydantic sub-model or one entity.
"""

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SegmentInfo:
    """Metadata about a produced segment."""
    name: str
    path: Path
    source_path: Path
    line_start: int
    line_end: int
```

Implement `slice_genre_region()`:
- Read the source file into lines
- Two-pass parsing:
  - Pass 1: Identify H2 section boundaries (`## N. ...`)
  - Pass 2: Within the "Dimensional Positions" section, identify bold dimension group headers (`**Aesthetic dimensions**`, `**Tonal dimensions**`, etc.) and other structural markers (`**Locus of power**`, `**Narrative structure**`, epistemological, world affordances, genre contracts)
- Extract state variables from `## 3. State Variables` (or similar numbered section)
- Extract boundaries from `## 4. Genre Topology` (or similar)
- Build meta segment from H1 title and any classification/modifier info before the first H2
- Drop everything after `### _commentary` or `### _suggestions`
- Write each segment with YAML frontmatter
- Return list of SegmentInfo

Key parsing patterns:
- `r'^## \d+\.'` — H2 numbered section boundary
- `r'^\*\*(\w[\w\s]+) dimensions\*\*'` — dimension group header
- `r'^\*\*Locus of power\*\*'` — locus section
- `r'^### _commentary'` — commentary start (drop point)

- [ ] **Step 5: Run tests to verify they pass**

```bash
uv run pytest tests/test_slicer.py -v
```

- [ ] **Step 6: Write segments-manifest.json on slice**

After writing segments, compute content hash of the source file and write `segments-manifest.json`:

```python
def _write_manifest(output_dir: Path, source_path: Path, segments: list[SegmentInfo]) -> None:
    source_hash = hashlib.sha256(source_path.read_bytes()).hexdigest()[:16]
    manifest = {
        "source_hash": source_hash,
        "source_path": str(source_path),
        "segments": {s.name: str(s.path.name) for s in segments},
    }
    (output_dir / "segments-manifest.json").write_text(json.dumps(manifest, indent=2))
```

Add skip-if-fresh logic: if manifest exists and source_hash matches, return early (unless `force=True`).

- [ ] **Step 7: Run ruff and commit**

```bash
uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add slicer with genre region segmentation"
```

---

### Task 2: Slicer — discovery, cluster, and genre-native segmentation

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/slicer.py`
- Modify: `tools/narrative-data/tests/test_slicer.py`
- Create: `tools/narrative-data/tests/fixtures/sample-archetypes.md`
- Create: `tools/narrative-data/tests/fixtures/sample-cluster-archetypes.md`
- Create: `tools/narrative-data/tests/fixtures/sample-narrative-shapes.md`

- [ ] **Step 1: Create fixtures**

Three new fixture files:
- `sample-archetypes.md` — 3 archetypes with H4 headings, bold labels, plus `### _commentary` at end. ~60 lines. **Also serves as the tropes fixture** since tropes share the identical H4 heading structure.
- `sample-cluster-archetypes.md` — 3 cluster archetypes with H3 headings. ~50 lines.
- `sample-narrative-shapes.md` — 2 narrative shapes with H2 headings and H3 subsections (including beats table). ~60 lines.

- [ ] **Step 2: Write failing tests**

Add to `test_slicer.py`:

```python
from narrative_data.pipeline.slicer import slice_discovery, slice_cluster, slice_genre_native


class TestSliceDiscovery:
    def test_splits_on_h4_headers(self, tmp_path):
        ...  # 3 segments from 3 archetypes
    def test_slug_generation(self, tmp_path):
        ...  # "The Unwilling Vessel" → "the-unwilling-vessel"
    def test_drops_commentary(self, tmp_path):
        ...

class TestSliceCluster:
    def test_splits_on_h3_headers(self, tmp_path):
        ...  # 3 segments from 3 cluster archetypes

class TestSliceGenreNative:
    def test_narrative_shapes_split_on_h2(self, tmp_path):
        ...  # 2 segments, each including H3 subsections
    def test_tropes_split_on_h4(self, tmp_path):
        ...  # same as discovery
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
uv run pytest tests/test_slicer.py -v -k "Discovery or Cluster or GenreNative"
```

- [ ] **Step 4: Implement slice_discovery, slice_cluster, slice_genre_native**

These are simpler than genre regions — single heading-level splits:

- `slice_discovery(source, output_dir)` — split on `r'^#### \d+\.\s+(.+)'`, slug from capture group
- `slice_cluster(source, output_dir)` — split on `r'^### \d+\.\s+(.+)'`, slug from capture group
- `slice_genre_native(source, output_dir, heading_level)` — generic: `heading_level=2` for narrative shapes, `heading_level=4` for tropes

All share the same frontmatter writing and manifest logic from Task 1.

- [ ] **Step 5: Add a top-level dispatch function**

```python
def slice_file(
    source_path: Path,
    output_dir: Path,
    doc_type: str,
    force: bool = False,
) -> list[SegmentInfo]:
    """Dispatch to the right slicer based on document type."""
    if doc_type == "genre-region":
        return slice_genre_region(source_path, output_dir, force=force)
    elif doc_type == "discovery":
        return slice_discovery(source_path, output_dir, force=force)
    elif doc_type == "cluster":
        return slice_cluster(source_path, output_dir, force=force)
    elif doc_type == "narrative-shapes":
        return slice_genre_native(source_path, output_dir, heading_level=2, force=force)
    elif doc_type == "tropes":
        return slice_genre_native(source_path, output_dir, heading_level=4, force=force)
    else:
        raise ValueError(f"Unknown doc_type: {doc_type}")
```

- [ ] **Step 6: Add parameterized test for slice_file dispatch**

```python
import pytest
from narrative_data.pipeline.slicer import slice_file

@pytest.mark.parametrize("doc_type", ["genre-region", "discovery", "cluster", "narrative-shapes", "tropes"])
def test_slice_file_dispatch(doc_type, tmp_path):
    # Each doc_type needs an appropriate fixture — use the right sample file
    ...

def test_slice_file_unknown_type_raises(tmp_path):
    with pytest.raises(ValueError, match="Unknown doc_type"):
        slice_file(tmp_path / "dummy.md", tmp_path / "out", "nonexistent")
```

- [ ] **Step 7: Run full slicer test suite, ruff, commit**

```bash
uv run pytest tests/test_slicer.py -v && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add discovery, cluster, and genre-native segmentation"
```

---

### Task 3: Segment extraction prompts

**Files:**
- Create: 17 files under `tools/narrative-data/prompts/structure/segments/`

Each segment prompt is short and focused (~15-30 lines). They follow the same template pattern:

```markdown
You are a structured data extractor. Read the text below and produce {output description}.

## Rules
{2-5 type-specific rules}

## General Rules
- All numeric values must be normalized floats between 0.0 and 1.0
- Preserve analytical prose in flavor_text fields
- If a field cannot be determined, use null for optional fields
- Do not invent information not present in the source

## Source Content
{raw_content}

## Target Schema
{schema}

Output only valid JSON, no markdown formatting.
```

- [ ] **Step 1: Write genre-region segment prompts (13 files)**

Key type-specific rules per prompt:
- **aesthetic/tonal/temporal/epistemological**: Extract ContinuousAxis objects. Map "X ←→ Y" to low_label/high_label. Map position words to 0.0-1.0. Copy *Note:* text to flavor_text.
- **thematic**: Extract WeightedTags. Identify 1-3 treatments with weights.
- **agency**: Mix of ContinuousAxis and Literal enum (agency_type).
- **world-affordances**: String fields. Magic is list of strings.
- **locus-of-power**: Extract ranked list. "Primary: Place, Secondary: System, Tertiary: Cosmos" → ["place", "system", "cosmos"]. Use lowercase.
- **narrative-structure**: Same ranked list pattern. Values from: quest, mystery, tragedy, comedy, romance, horror.
- **narrative-contracts**: Array of {invariant, enforced} objects.
- **state-variables**: Array of StateVariableTemplate. Identify behavior type, initial_value, threshold.
- **boundaries**: Array of {trigger, drift_target, description}. drift_target should be kebab-case genre slug.
- **meta**: genre_slug (kebab-case from title), genre_name, classification (default "constraint_layer"), constraint_layer_type, modifies, flavor_text.

- [ ] **Step 2: Write generic entity prompts (4 files)**

- **discovery-entity.md**: "Extract a single entity from this text matching the schema. Map personality/dimensional descriptions to 0.0-1.0 floats. Preserve analytical prose."
- **discovery-entity-cluster.md**: "Extract a canonical entity with genre_variants list. Each variant has genre_slug, variant_name, key_differences."
- **trope-entity.md**: "Extract one trope. narrative_function is a list of enum values. Extract variants (straight/inverted/deconstructed/violation)."
- **narrative-shape-entity.md**: "Extract one narrative shape. Beats have position (0.0-1.0), flexibility (load_bearing/ornamental), tension_effect enum. Parse markdown tables if present."

- [ ] **Step 3: Verify all prompts load**

```python
from narrative_data.prompts import PromptBuilder
pb = PromptBuilder()
# Test loading via the segments/ subdirectory
for name in ["genre-region-aesthetic", "genre-region-meta", "discovery-entity", ...]:
    pb.build_segment_structure(name, "test", {})
```

- [ ] **Step 4: Commit**

```bash
git add prompts/structure/segments/
git commit -m "feat: add 17 segment-level extraction prompts"
```

---

### Task 4: PromptBuilder.build_segment_structure()

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/prompts.py`
- Modify: `tools/narrative-data/tests/test_prompts_structure.py`

- [ ] **Step 1: Write failing tests**

Add to `test_prompts_structure.py`:

```python
class TestBuildSegmentStructure:
    def test_loads_from_segments_subdirectory(self, tmp_path):
        seg_dir = tmp_path / "structure" / "segments"
        seg_dir.mkdir(parents=True)
        (seg_dir / "genre-region-aesthetic.md").write_text(
            "Extract:\n{raw_content}\nSchema:\n{schema}"
        )
        builder = PromptBuilder(prompts_dir=tmp_path)
        result = builder.build_segment_structure(
            "genre-region-aesthetic", "content here", {"type": "object"}
        )
        assert "content here" in result

    def test_missing_template_raises(self, tmp_path):
        builder = PromptBuilder(prompts_dir=tmp_path)
        with pytest.raises(FileNotFoundError):
            builder.build_segment_structure("nonexistent", "content", {})
```

- [ ] **Step 2: Implement build_segment_structure()**

Add to `prompts.py`:

```python
def build_segment_structure(
    self,
    segment_type: str,
    raw_content: str,
    schema: dict,
) -> str:
    """Build a segment-level structuring prompt."""
    template_path = self.prompts_dir / "structure" / "segments" / f"{segment_type}.md"
    if not template_path.exists():
        raise FileNotFoundError(f"Segment prompt template not found: {template_path}")
    prompt = template_path.read_text()
    prompt = prompt.replace("{raw_content}", raw_content)
    prompt = prompt.replace("{schema}", json.dumps(schema, indent=2))
    return prompt
```

- [ ] **Step 3: Run tests, ruff, commit**

```bash
uv run pytest tests/test_prompts_structure.py -v && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add build_segment_structure() to PromptBuilder"
```

---

### Task 5: Aggregator

**Files:**
- Create: `tools/narrative-data/src/narrative_data/pipeline/aggregator.py`
- Create: `tools/narrative-data/tests/test_aggregator.py`

- [ ] **Step 1: Write failing tests**

```python
"""Tests for segment JSON aggregation."""

import json
from pathlib import Path

import pytest
from pydantic import ValidationError

from narrative_data.pipeline.aggregator import (
    aggregate_genre_dimensions,
    aggregate_discovery,
    load_segment_json,
)
from narrative_data.schemas.genre_dimensions import GenreDimensions
from narrative_data.schemas.archetypes import Archetype


class TestLoadSegmentJson:
    def test_loads_dict(self, tmp_path):
        (tmp_path / "test.json").write_text('{"key": "value"}')
        result = load_segment_json(tmp_path / "test.json")
        assert result == {"key": "value"}

    def test_loads_list(self, tmp_path):
        (tmp_path / "test.json").write_text('[1, 2, 3]')
        result = load_segment_json(tmp_path / "test.json")
        assert result == [1, 2, 3]

    def test_missing_file_raises(self, tmp_path):
        with pytest.raises(FileNotFoundError):
            load_segment_json(tmp_path / "missing.json")


class TestAggregateGenreDimensions:
    def test_assembles_from_segments(self, tmp_path):
        # Write minimal valid segment JSONs for all 13 segments
        # (meta, aesthetic, tonal, temporal, thematic, agency,
        #  epistemological, world-affordances, locus-of-power,
        #  narrative-structure, narrative-contracts, state-variables, boundaries)
        ...  # Write each as JSON file matching the sub-schema
        result = aggregate_genre_dimensions(tmp_path)
        assert isinstance(result, GenreDimensions)
        assert result.genre_slug == "test-horror"

    def test_missing_segment_raises(self, tmp_path):
        # Write only meta segment, missing others
        ...
        with pytest.raises(FileNotFoundError):
            aggregate_genre_dimensions(tmp_path)

    def test_invalid_segment_data_raises(self, tmp_path):
        # Write a segment with invalid data (e.g., axis value > 1.0)
        ...
        with pytest.raises(ValidationError):
            aggregate_genre_dimensions(tmp_path)


class TestAggregateDiscovery:
    def test_collects_entity_segments(self, tmp_path):
        # Write 3 segment JSONs as valid Archetype objects
        ...
        result = aggregate_discovery(tmp_path, Archetype)
        assert len(result) == 3

    def test_validates_each_entity(self, tmp_path):
        # Write one valid + one invalid segment
        ...
        with pytest.raises(ValidationError):
            aggregate_discovery(tmp_path, Archetype)
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
uv run pytest tests/test_aggregator.py -v
```

- [ ] **Step 3: Implement aggregator**

Create `src/narrative_data/pipeline/aggregator.py`:

```python
"""Aggregate segment JSONs into type-level validated objects."""

import json
from pathlib import Path

from pydantic import BaseModel

from narrative_data.schemas.genre_dimensions import GenreDimensions


def load_segment_json(path: Path):
    """Load a segment JSON file. Returns dict or list."""
    if not path.exists():
        raise FileNotFoundError(f"Segment JSON not found: {path}")
    return json.loads(path.read_text())


def aggregate_genre_dimensions(segment_dir: Path) -> GenreDimensions:
    """Assemble GenreDimensions from segment JSONs."""
    meta = load_segment_json(segment_dir / "segment-meta.json")
    return GenreDimensions(
        genre_slug=meta["genre_slug"],
        genre_name=meta["genre_name"],
        classification=meta["classification"],
        constraint_layer_type=meta.get("constraint_layer_type"),
        aesthetic=load_segment_json(segment_dir / "segment-aesthetic.json"),
        tonal=load_segment_json(segment_dir / "segment-tonal.json"),
        temporal=load_segment_json(segment_dir / "segment-temporal.json"),
        thematic=load_segment_json(segment_dir / "segment-thematic.json"),
        agency=load_segment_json(segment_dir / "segment-agency.json"),
        epistemological=load_segment_json(segment_dir / "segment-epistemological.json"),
        world_affordances=load_segment_json(segment_dir / "segment-world-affordances.json"),
        locus_of_power=load_segment_json(segment_dir / "segment-locus-of-power.json"),
        narrative_structure=load_segment_json(segment_dir / "segment-narrative-structure.json"),
        narrative_contracts=load_segment_json(segment_dir / "segment-narrative-contracts.json"),
        active_state_variables=load_segment_json(segment_dir / "segment-state-variables.json"),
        boundaries=load_segment_json(segment_dir / "segment-boundaries.json"),
        modifies=meta.get("modifies", []),
        flavor_text=meta.get("flavor_text"),
    )


def aggregate_discovery(segment_dir: Path, schema: type[BaseModel]) -> list:
    """Assemble entity array from segment JSONs."""
    entities = []
    for seg_json in sorted(segment_dir.glob("segment-*.json")):
        # Note: segments-manifest.json is named "segments-" not "segment-"
        # so the glob naturally excludes it
        data = json.loads(seg_json.read_text())
        entities.append(schema.model_validate(data))
    return entities
```

- [ ] **Step 4: Run tests, ruff, commit**

```bash
uv run pytest tests/test_aggregator.py -v && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add segment aggregator with genre dimensions and discovery assembly"
```

---

### Task 6: Add run_segment_extraction() to structure.py

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/structure.py`
- Modify: `tools/narrative-data/tests/test_pipeline.py`

The existing `run_structuring()` reads a full source file and builds prompts via `build_structure()`. Segments need a different path: the content is already sliced, the schema is a sub-schema, and prompts come from `build_segment_structure()`.

- [ ] **Step 1: Write failing tests for run_segment_extraction**

```python
class TestRunSegmentExtraction:
    def test_extracts_segment_to_json(self, mock_client, tmp_path):
        """Given a segment .md, produces a sibling .json."""
        ...

    def test_uses_build_segment_structure(self, mock_client, tmp_path):
        """Uses segment prompt path, not monolithic prompt."""
        ...

    def test_replace_on_retry(self, mock_client, tmp_path):
        """Same replace-not-append behavior as run_structuring."""
        ...
```

- [ ] **Step 2: Implement run_segment_extraction()**

```python
def run_segment_extraction(
    client: OllamaClient,
    segment_path: Path,
    output_path: Path,
    schema: dict,
    segment_prompt_slug: str,
    model: str = STRUCTURING_MODEL,
    max_retries: int = 3,
) -> dict[str, Any]:
    """Extract structured JSON from a single segment.

    Unlike run_structuring(), this:
    - Takes a pre-computed schema dict (not a Pydantic type)
    - Uses build_segment_structure() for prompts
    - Does NOT validate against Pydantic (aggregator does that)
    - Reads segment content, stripping YAML frontmatter
    """
    raw_content = _strip_frontmatter(segment_path.read_text())
    base_prompt = PromptBuilder().build_segment_structure(
        segment_prompt_slug, raw_content, schema
    )
    # ... same retry loop as run_structuring but writes raw JSON
```

Key difference: segment extraction writes the raw JSON output without Pydantic validation — validation happens at aggregation time when all segments are assembled. This avoids needing to import every sub-model into structure.py.

- [ ] **Step 3: Run tests, ruff, commit**

```bash
uv run pytest tests/test_pipeline.py -v && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: add run_segment_extraction for segment-level LLM calls"
```

---

### Task 7: Wire orchestration to use segments

**Files:**
- Modify: `tools/narrative-data/src/narrative_data/pipeline/structure_commands.py`
- Modify: `tools/narrative-data/tests/test_structure_commands.py`

This is the integration task — connecting slicer → extraction → aggregation in the existing orchestration commands.

- [ ] **Step 1: Add segment configuration to TypeConfig and SEGMENT_MAP**

Extend `TypeConfig`:

```python
@dataclass
class TypeConfig:
    per_genre: type[BaseModel]
    cluster: type[BaseModel] | None
    data_dir: str
    file_pattern: str | None = None
    is_collection: bool = True
    prompt_slug: str = field(default="")
    doc_type: str = "discovery"  # slicer dispatch key
```

All 12 types use the segment pipeline. The `doc_type` field determines which slicer is used:
- `genre-dimensions`: `doc_type="genre-region"`
- `tropes`: `doc_type="tropes"`
- `narrative-shapes`: `doc_type="narrative-shapes"`
- All 9 discovery types: `doc_type="discovery"` (default)

For genre-region, define a separate `GENRE_REGION_SEGMENT_MAP` that maps segment names to sub-schemas and prompt slugs:

```python
from narrative_data.schemas.genre_dimensions import (
    AestheticDimensions, TonalDimensions, TemporalDimensions,
    ThematicDimensions, AgencyDimensions, WorldAffordances,
    EpistemologicalDimensions, NarrativeContract,
)

@dataclass
class SegmentConfig:
    schema_source: dict  # pre-computed JSON schema dict
    prompt_slug: str     # under prompts/structure/segments/

GENRE_REGION_SEGMENT_MAP: dict[str, SegmentConfig] = {
    "meta": SegmentConfig(
        schema_source={"type": "object", "properties": {
            "genre_slug": {"type": "string"},
            "genre_name": {"type": "string"},
            "classification": {"type": "string", "enum": ["standalone_region", "constraint_layer", "hybrid_modifier"]},
            "constraint_layer_type": {"type": "string"},
            "modifies": {"type": "array", "items": {"type": "string"}},
            "flavor_text": {"type": "string"},
        }, "required": ["genre_slug", "genre_name", "classification"]},
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
        schema_source={"type": "array", "items": {"type": "string", "enum": ["place", "person", "system", "relationship", "cosmos"]}, "maxItems": 3},
        prompt_slug="genre-region-locus-of-power",
    ),
    "narrative-structure": SegmentConfig(
        schema_source={"type": "array", "items": {"type": "string", "enum": ["quest", "mystery", "tragedy", "comedy", "romance", "horror"]}, "maxItems": 3},
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
```

Update the `genre-dimensions` entry in `TYPE_REGISTRY` to use `doc_type="genre-region"`. Update `tropes` to `doc_type="tropes"` and `narrative-shapes` to `doc_type="narrative-shapes"`.

- [ ] **Step 2: Update structure_type() to use segment pipeline**

All types now go through the segment pipeline. The flow for each genre:

1. Resolve source path (unchanged)
2. Resolve segment directory: `source_path.parent / source_path.stem` (e.g., `genres/folk-horror/region/`)
3. Slice: `slice_file(source_path, segment_dir, config.doc_type, force=force)`
4. For each segment produced:
   - Determine schema + prompt slug:
     - **genre-region**: look up in `GENRE_REGION_SEGMENT_MAP[segment.name]`
     - **discovery/cluster/tropes/shapes**: use `config.per_genre.model_json_schema()` + generic prompt slug (`"discovery-entity"`, `"trope-entity"`, etc.)
   - Call `run_segment_extraction(client, segment.path, segment_json_path, schema, prompt_slug, model=model)`
   - Print per-segment progress
5. Aggregate:
   - **genre-region**: `aggregate_genre_dimensions(segment_dir)`
   - **all others**: `aggregate_discovery(segment_dir, config.per_genre)`
6. Write final `.json` from aggregated result
7. Print summary

Rich console output:
```
[cyan]Structuring genre-dimensions for folk-horror...[/cyan]
  [dim]Slicing region.md → 13 segments[/dim]
  [cyan]  segment-meta[/cyan] [green]✓[/green]
  [cyan]  segment-aesthetic[/cyan] [green]✓[/green]
  ...
  [cyan]  Aggregating...[/cyan] [green]✓[/green]
[green]✓ folk-horror[/green]
```

**`force` propagation:** `force=True` means re-slice AND re-extract AND re-aggregate. Without force, each stage checks its own staleness (slicer via manifest hash, extraction via segment JSON existence).

- [ ] **Step 3: Update structure_clusters() similarly**

Same pattern but uses `doc_type="cluster"`, `slice_cluster()`, and `aggregate_discovery()` with the cluster schema. Prompt slug: `"discovery-entity-cluster"`.

- [ ] **Step 4: Update tests**

Update `test_structure_commands.py` to account for the segment-based flow:
- Mock `slice_file` to produce predictable segment SegmentInfo lists
- Mock `run_segment_extraction` to write segment JSONs
- Verify aggregation produces the final `.json`
- Test force propagation
- Test per-segment progress output

- [ ] **Step 5: Run full test suite, ruff, commit**

```bash
uv run pytest -x -q && uv run ruff check . && uv run ruff format --check .
git add -A && git commit -m "feat: wire segment pipeline into structure orchestration"
```

---

### Task 8: Smoke test — folk-horror genre dimensions

**Files:** No new files — this is a validation run.

- [ ] **Step 1: Slice folk-horror region**

```bash
STORYTELLER_DATA_PATH=/path/to/storyteller-data \
  uv run narrative-data structure run genre-dimensions --genre folk-horror --force
```

Observe the per-segment progress output.

- [ ] **Step 2: Inspect segment files**

```bash
ls storyteller-data/narrative-data/genres/folk-horror/region/
cat storyteller-data/narrative-data/genres/folk-horror/region/segment-locus-of-power.md
cat storyteller-data/narrative-data/genres/folk-horror/region/segment-locus-of-power.json
```

Verify:
- All 13 segment `.md` files exist with frontmatter
- Segment JSONs match expected sub-schemas
- `segment-locus-of-power.json` contains `["place", "system", "cosmos"]` (the field that failed in monolithic extraction)
- Labels populated in continuous axis segments
- Flavor text contains analytical prose, not one-word summaries

- [ ] **Step 3: Inspect aggregated output**

```bash
cat storyteller-data/narrative-data/genres/folk-horror/region.json | python3 -m json.tool | head -60
```

Verify the final `GenreDimensions` object has all fields populated.

- [ ] **Step 4: Compare against the monolithic 7b extraction**

Key fields to check:
- `locus_of_power`: should now be `["place", "system", "cosmos"]` (was `[]`)
- `classification`: should be `"constraint_layer"` (was `"hybrid_modifier"`)
- `genre_slug`: should be `"folk-horror"` (was `"folk_horror"`)
- Continuous axes: should have `low_label`/`high_label` populated (were all null)
- `flavor_text` fields: should contain analytical prose (were one-word)

- [ ] **Step 5: Document findings and commit any prompt adjustments**

If prompts need tuning based on the smoke test, fix them and re-run. Commit when the output quality is satisfactory.

```bash
git add -A && git commit -m "test: validate segment-based extraction on folk-horror genre dimensions"
```

---

### Task 9: Final verification

- [ ] **Step 1: Run full test suite**

```bash
uv run pytest -v
```

- [ ] **Step 2: Run ruff**

```bash
uv run ruff check . && uv run ruff format --check .
```

- [ ] **Step 3: Verify file counts**

```bash
# Segment prompts
ls prompts/structure/segments/ | wc -l
# Expected: 17

# Slicer + aggregator modules
ls src/narrative_data/pipeline/slicer.py src/narrative_data/pipeline/aggregator.py
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "chore: segment-based extraction pipeline complete — ready for P1 runs"
```
