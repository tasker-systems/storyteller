# Segment-Based Extraction Design

**Date:** 2026-03-22
**Addendum to:** `docs/superpowers/specs/2026-03-22-stage-2-json-structuring-design.md`
**Branch:** `jcoletaylor/narrative-data-exploratory-research`

---

## 1. Problem

The Stage 2 structuring pipeline sends entire markdown documents (~16KB) plus the full JSON schema (~8KB) to `qwen2.5:7b-instruct` in a single call. At ~7800 tokens input plus complex structured output, the model:

- Misses fields requiring semantic inference (empty `locus_of_power` despite explicit "Primary: Place, Secondary: System, Tertiary: Cosmos" in the source)
- Produces shallow `flavor_text` (one-word summaries instead of analytical prose)
- Confuses similar fields across dimension groups (reuses labels from one group in another)
- Misclassifies genre types

Larger models (qwen2.5:14b, qwen3.5:4b) produce better results but are too slow — 2+ minutes for a trivial extraction, 5+ minutes for a full genre region, making the 414-file corpus impractical.

The 7b model handles small, focused extraction tasks well. The issue is context overload, not model capability.

## 2. Solution

Insert a deterministic segmentation stage before extraction. Split each markdown file into focused segments (~30-80 lines each), extract each segment independently with a focused sub-schema, then aggregate segment JSONs into the type-level object with Pydantic validation.

```
Stage 1 (existing): Elicitation → raw .md files
Stage 1.5 (new):    Segmentation → segment .md files
Stage 2 (updated):  Extraction → segment .json files
Stage 2.5 (new):    Aggregation → type-level .json files
```

## 3. Design Principles

### 3.1 Bound Context to Retain Focus

The same principle that governs the engine's imperfect information architecture applies to model interaction. A 7b model given 500 tokens of focused content and a 200-token sub-schema outperforms the same model given 7800 tokens of mixed content and a 4000-token schema.

### 3.2 Deterministic Segmentation

The slicer is pure Python — no LLM involved. The qwen3.5:35b model produced structurally consistent documents with predictable markdown patterns (heading levels, bold labels, numbered sections, consistent separators). These patterns are reliable parsing targets.

### 3.3 Segment-Level Inspectability

Every segment and its extracted JSON are durable files on disk. If `segment-locus-of-power.json` is wrong, you inspect the segment, fix the prompt, and re-run that one extraction. No need to re-process the entire file or corpus.

### 3.4 Provenance by Path

Each segment's file path encodes its source and identity. The frontmatter records exact source file and line range. This is free provenance that supports downstream vectorization and tooling-findability.

## 4. Segmentation Rules

### 4.1 Genre Regions

Split `region.md` by dimension group. The source has bold dimension group headers (`**Aesthetic dimensions**`, `**Tonal dimensions**`, etc.) within numbered H2 sections.

| Segment | Source Pattern | Maps To |
|---|---|---|
| `segment-meta.md` | H1 title + classification info | genre_slug, genre_name, classification, constraint_layer_type, modifies |
| `segment-aesthetic.md` | `**Aesthetic dimensions**` block | `AestheticDimensions` |
| `segment-tonal.md` | `**Tonal dimensions**` block | `TonalDimensions` |
| `segment-temporal.md` | `**Temporal dimensions**` block | `TemporalDimensions` |
| `segment-thematic.md` | `**Thematic dimensions**` block | `ThematicDimensions` |
| `segment-agency.md` | `**Agency dimensions**` block | `AgencyDimensions` |
| `segment-epistemological.md` | Epistemological section | `EpistemologicalDimensions` |
| `segment-world-affordances.md` | World affordances / magic / technology section | `WorldAffordances` |
| `segment-locus-of-power.md` | `**Locus of power**` section | `list[str]` (ranked, max 3) |
| `segment-narrative-structure.md` | Narrative structure section | `list[str]` (ranked, max 3) |
| `segment-state-variables.md` | `## 3. State Variables` H2 section | `list[StateVariableTemplate]` |
| `segment-boundaries.md` | `## 4. Genre Topology` or boundaries section | `list[GenreBoundary]` |

**Commentary/suggestions sections** (`### _commentary`, `### _suggestions`) are dropped — they were generation-phase thinking, not entity data. The source `.md` file is unchanged and available for future review.

### 4.2 Discovery Per-Genre

Split on H4 numbered entity headers (`#### 1. The Unwilling Vessel`). Each entity from its heading to the next heading (or `### _commentary` / end of file) becomes one segment.

| Example Source | Segment | Maps To |
|---|---|---|
| `#### 1. The Unwilling Vessel` ... | `segment-the-unwilling-vessel.md` | `Archetype` |
| `#### 2. The Earnest Warden` ... | `segment-the-earnest-warden.md` | `Archetype` |

Commentary and suggestions sections are dropped.

### 4.3 Cluster Synthesis

Split on H3 numbered entity headers (`### 1. The Epistemic Seeker`). Same pattern as discovery but one heading level up.

| Example Source | Segment | Maps To |
|---|---|---|
| `### 1. The Epistemic Seeker` ... | `segment-the-epistemic-seeker.md` | `ClusterArchetype` |

### 4.4 Genre-Native Tropes

Split on H4 numbered trope headers. Same pattern as discovery per-genre.

### 4.5 Genre-Native Narrative Shapes

Split on H2 numbered shape headers (`## 1. The Spiral of Diminishing Certainty`). Each shape includes its H3 subsections (beats table, tension profile, rest rhythm, etc.) as a single segment.

### 4.6 Slug Generation

Entity names are kebab-cased for segment filenames:
- `The Unwilling Vessel` → `the-unwilling-vessel`
- `The Spiral of Diminishing Certainty` → `the-spiral-of-diminishing-certainty`
- Strip leading numbers and punctuation: `#### 1. The Unwilling Vessel` → `the-unwilling-vessel`

## 5. Segment File Format

Each segment file has a YAML frontmatter header followed by the raw content:

```markdown
---
source: genres/folk-horror/region.md
segment: aesthetic
lines: 5-10
---

**Aesthetic dimensions**
*   **Visual/Sensory:** **Grounded/mundane ←→ Heightened/mythic.**
    *   *Note:* The genre thrives on the friction...
```

The frontmatter is lightweight metadata. Extraction prompts can ignore it (it's above the content). Downstream tools can use it for provenance.

## 6. File Layout

```
genres/folk-horror/
  region.md                          ← source (unchanged)
  region/                            ← segment directory
    segment-meta.md
    segment-meta.json
    segment-aesthetic.md
    segment-aesthetic.json
    segment-tonal.md
    segment-tonal.json
    ...
    segments-manifest.json           ← content hashes for staleness detection
  region.json                        ← final aggregated output

discovery/archetypes/
  folk-horror.md                     ← source (unchanged)
  folk-horror/                       ← segment directory
    segment-the-unwilling-vessel.md
    segment-the-unwilling-vessel.json
    segment-the-earnest-warden.md
    segment-the-earnest-warden.json
    ...
    segments-manifest.json
  folk-horror.json                   ← final aggregated output (array)
```

## 7. Focused Extraction

Each segment gets its own extraction call with a **sub-schema** matching its target Pydantic model.

### 7.1 Genre Region Segments

| Segment | Sub-Schema | Output |
|---|---|---|
| `segment-aesthetic.md` | `AestheticDimensions.model_json_schema()` | single object |
| `segment-tonal.md` | `TonalDimensions.model_json_schema()` | single object |
| `segment-temporal.md` | `TemporalDimensions.model_json_schema()` | single object |
| `segment-thematic.md` | `ThematicDimensions.model_json_schema()` | single object |
| `segment-agency.md` | `AgencyDimensions.model_json_schema()` | single object |
| `segment-epistemological.md` | `EpistemologicalDimensions.model_json_schema()` | single object |
| `segment-world-affordances.md` | `WorldAffordances.model_json_schema()` | single object |
| `segment-locus-of-power.md` | `{"type": "array", "items": {"type": "string"}}` | array |
| `segment-narrative-structure.md` | same pattern | array |
| `segment-state-variables.md` | `{"type": "array", "items": StateVariableTemplate.model_json_schema()}` | array |
| `segment-boundaries.md` | `{"type": "array", "items": GenreBoundary.model_json_schema()}` | array |
| `segment-meta.md` | inline schema for genre_slug, genre_name, classification, etc. | single object |

### 7.2 Discovery / Cluster / Genre-Native Segments

Each entity segment gets the per-entity schema. `segment-the-unwilling-vessel.md` gets `Archetype.model_json_schema()` and produces one JSON object.

### 7.3 Prompt Templates

Segment-level prompts replace the monolithic type prompts. They are shorter and more focused:

```
prompts/structure/segments/
  genre-region-aesthetic.md
  genre-region-tonal.md
  genre-region-temporal.md
  genre-region-thematic.md
  genre-region-agency.md
  genre-region-epistemological.md
  genre-region-world-affordances.md
  genre-region-locus-of-power.md
  genre-region-narrative-structure.md
  genre-region-state-variables.md
  genre-region-boundaries.md
  genre-region-meta.md
  discovery-entity.md              ← generic for any discovery entity type
  discovery-entity-cluster.md      ← generic for any cluster entity
  trope-entity.md
  narrative-shape-entity.md
```

Genre-region segments need type-specific prompts because each dimension group has different extraction rules (continuous axes vs weighted tags vs ranked lists vs state variable templates). Discovery types can share a generic entity prompt since the schema itself guides the extraction — the model just needs to know "extract one entity from this text into this schema."

### 7.4 Token Budget

With segments of 30-80 lines (~500-2000 chars) plus a focused sub-schema (~500-1000 chars), each extraction call is ~1000-3000 tokens input. The 7b model handles this comfortably.

## 8. Aggregation

### 8.1 Genre Regions

Read all segment JSONs from the segment directory, assemble into a `GenreDimensions` object:

```python
def aggregate_genre_dimensions(segment_dir: Path) -> GenreDimensions:
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
        active_state_variables=load_segment_json(segment_dir / "segment-state-variables.json"),
        boundaries=load_segment_json(segment_dir / "segment-boundaries.json"),
        modifies=meta.get("modifies", []),
        flavor_text=meta.get("flavor_text"),
    )
```

Pydantic validates the assembled object. If it fails, the error identifies which field (and therefore which segment) caused the problem.

### 8.2 Discovery / Cluster / Genre-Native Types

Collect all `segment-*.json` files, validate each against the entity schema, assemble into an array:

```python
def aggregate_discovery(segment_dir: Path, schema: type[BaseModel]) -> list:
    entities = []
    for seg_json in sorted(segment_dir.glob("segment-*.json")):
        entities.append(schema.model_validate(json.loads(seg_json.read_text())))
    return entities
```

### 8.3 Output

The aggregated, validated JSON writes to the sibling `.json` file (`region.json`, `folk-horror.json`). Segment directories and segment JSONs persist for inspectability and provenance.

## 9. Pipeline Integration

### 9.1 New Modules

| Module | Purpose |
|---|---|
| `pipeline/slicer.py` | Deterministic markdown → segment files |
| `pipeline/aggregator.py` | Segment JSONs → validated type-level JSON |

### 9.2 Modified Modules

| Module | Change |
|---|---|
| `pipeline/structure.py` | Add `run_segment_structuring()` — takes one segment, one sub-schema |
| `pipeline/structure_commands.py` | Orchestrate slice → extract segments → aggregate |

### 9.3 CLI

The `narrative-data structure run` command internals change but the interface stays the same:

```bash
# Full pipeline: slice → extract → aggregate
narrative-data structure run genre-dimensions --genre folk-horror

# Force re-segmentation + re-extraction
narrative-data structure run genre-dimensions --genre folk-horror --force
```

A `narrative-data segment` subcommand for standalone slicing is optional — useful for debugging but not required for the pipeline.

### 9.4 Staleness Detection

Each segment directory contains a `segments-manifest.json` with content hashes of the source file and each segment. If the source `.md` changes, re-slice. If a segment `.md` changes (shouldn't happen, but), re-extract that segment. If all segment JSONs are present and valid, skip to aggregation.

### 9.5 What Stays the Same

- All 12 Pydantic schemas (the sub-models are exactly what segments produce)
- `OllamaClient` (including thinking model support)
- `--model` override
- Batch scripts (P1-P4)
- The 0.0-1.0 data contract
- The CLI interface from the user's perspective

## 10. Estimated Performance

| Stage | Per File | Per Genre (30 files) | Notes |
|---|---|---|---|
| Segmentation | <1s | <30s | Deterministic Python, no LLM |
| Extraction (genre region) | 12 segments × ~30s = ~6min | ~3 hours | 7b model, focused context |
| Extraction (discovery type) | 5-8 segments × ~20s = ~2min | ~1 hour | Simpler schemas |
| Aggregation | <1s | <30s | Deterministic Python |

Total P1 (genre dimensions): ~3 hours. Total P2-P4: ~8-10 hours across all types.

These times assume `qwen2.5:7b-instruct`. Faster models or parallelization would reduce them, but the segment-based approach means each call is individually fast and failures are cheap to retry.

## 11. Testing Strategy

- **Slicer tests:** Verify segment count, filenames, frontmatter, and content boundaries for each document type. Use a fixture markdown file per type.
- **Aggregator tests:** Verify assembly from segment JSONs into validated Pydantic objects. Test with known-good segment data.
- **Integration test:** Slice a real file, extract segments (feature-gated behind `test-llm`), aggregate, validate.
- **Existing schema tests:** Unchanged — the schemas are already validated.
