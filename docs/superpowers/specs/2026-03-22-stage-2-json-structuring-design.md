# Stage 2 JSON Structuring Design

**Date:** 2026-03-22
**Branch:** `jcoletaylor/narrative-data-exploratory-research`
**Predecessor:** `docs/superpowers/specs/2026-03-18-primitive-first-narrative-data-design.md`
**Analysis source:** `storyteller-data/narrative-data/analysis/2026-03-21-comprehensive-terrain-analysis.md`

---

## 1. Purpose

Extract structured JSON from the 7.1MB raw markdown corpus using the `qwen2.5:7b-instruct` model, guided by schemas derived from the comprehensive terrain analysis. This produces the queryable structured data that populates the narrative engine's persistence layer.

The raw markdown files are permanent artifacts — rich analytical documents that remain part of the curation loop. The structured JSON is a sibling representation: the markdown is the narrative voice, the JSON is the queryable structure.

---

## 2. Design Principles

### 2.1 Normalized 0.0–1.0 Data Contract

All continuous numeric values across all schemas use **normalized 0.0–1.0 floats**. This applies to:

- `ContinuousAxis.value`
- `WeightedTags` values
- `StateVariableTemplate.initial_value` and `.threshold`
- Personality profile axes (warmth, authority, openness, interiority, stability, agency, morality)
- Communicability channel intensities
- Beat positions in narrative shapes
- Any future continuous numeric field

**Rationale:** Consistent mental model across all types. No downstream normalization needed for analytical, embedding, or ML work. Enforced by Pydantic field validators.

### 2.2 Respect the Model's Capacity

Type-specific focused extraction prompts for the 7b model. Each prompt provides only the context and instructions relevant to that extraction type — no ancillary context, no multi-type instructions. The model is capable but performs best with focused, well-scoped tasks.

### 2.3 Both Levels Serve Different Consumers

**Per-genre structured data** is the granular runtime representation — the richest content for the game engine, scene selection, and context assembly.

**Cluster synthesis structured data** describes regions-of-regions — navigational structures for authorial tooling and generative play. These guide authors to genre regions via free-text-to-vector similarity matching (pg_vector embeddings for aesthetically rich descriptions → k-means or similar clustering to suggest genre regions or help craft new ones). The cluster level is the translation layer between human intent and the dimensional genre space.

---

## 3. Schema Architecture

### 3.1 Shared Primitives (`schemas/shared.py`)

Reusable building blocks referenced across all type schemas:

| Type | Fields | Purpose |
|------|--------|---------|
| `ContinuousAxis` | value (0.0–1.0), low_label, high_label, can_be_state_variable, flavor_text | Dimensional positioning with labels and prose |
| `WeightedTags` | dict[str, float] (values 0.0–1.0) | Thematic treatment maps, e.g. `{"Stewardship": 0.7, "Corruption": 0.3}` |
| `StateVariableTemplate` | canonical_id, genre_label, behavior (depleting/accumulating/fluctuating/progression/countdown), initial_value (0.0–1.0), threshold (0.0–1.0), threshold_effect, activation_condition | Dynamic variable configuration per genre |
| `StateVariableInteraction` | variable_id, operation (consumes/accumulates/depletes/transforms/gates), description | How entities affect state variables |
| `OverlapSignal` | adjacent_genre, similar_entity, differentiator | Cross-genre boundary marker |
| `GenreBoundary` | trigger, drift_target, description | Genre transition detection |

**No `NarrativeEntity` base class.** Each type schema stands on its own. The data varies too much across types for a useful inheritance hierarchy. Common patterns (genre_slug, canonical_name, flavor_text) appear where needed without being forced into shared inheritance.

### 3.2 The 12 Type Schemas

Each type that has cluster synthesis data gets two schema variants: a per-genre schema (full detail for runtime) and a cluster schema (canonical names + genre variant lists for authorial/navigational use).

**Note on naming:** The existing pipeline uses `profiles` as the type slug for scene profiles (in `config.py`, discovery prompts, and file paths). This spec uses `scene-profiles` for clarity in prose, but the implementation slug remains `profiles` to maintain consistency with existing data paths and manifests. The Pydantic schema module is `scene_profiles.py` for readability; the CLI and file paths use `profiles`.

**Note on key fields:** The Key Fields column highlights the most distinctive fields per type. The full Pydantic models will match the complete analysis schemas (Section 7 of the comprehensive terrain analysis), including fields not listed here (e.g., `currencies`, `network_position`, `constraints` on Dynamic; `agency`, `state_shift` on SpatialTopology; `overlap_signal` on NarrativeShape).

| Type | Per-Genre | Cluster | Key Fields (highlights — see analysis for complete set) |
|------|-----------|---------|------------|
| GenreDimensions | ✓ | — | 34 dimensions grouped by category, weighted tags, state variables, narrative contracts, boundaries, constraint_layer_type, modifies |
| Archetype | ✓ | ✓ | personality_profile (7 axes), extended_axes, distinguishing_tension, structural_necessity, overlap_signals, universality |
| Dynamic | ✓ | ✓ | scale, edge_type, directionality, currencies, network_position, role_slots, valence, evolution_pattern, scale_manifestations, state_variable_interactions |
| Goal | ✓ | ✓ | scale (existential/arc/scene/cross_scale), cross_scale_tension, state_variable_interactions, archetype_refs |
| ArchetypeDynamic | ✓ | ✓ | archetype_a/b, edge_properties, characteristic_scene, shadow_pairing, scale_properties |
| SceneProfile | ✓ | ✓ | dimensional_properties (tension, pacing, cast, resolution, info flow), uniqueness |
| OntologicalPosture | ✓ | ✓ | modes_of_being, self_other_boundary, ethical_orientation |
| Settings | ✓ | ✓ | atmospheric_palette, sensory_vocabulary, communicability dimensions |
| SpatialTopology | ✓ | ✓ | source_setting, target_setting, friction (type + level), directionality, agency, tonal_inheritance, traversal_cost, state_shift |
| PlaceEntity | ✓ | ✓ | communicability (4 channels: atmospheric, sensory, spatial, temporal), entity_properties, state_variable_expression |
| Trope | ✓ | — | narrative_function, variants (straight/inverted/deconstructed/violation), state_variable_interactions, overlap_signal |
| NarrativeShape | ✓ | — | tension_profile (family + description), beats (position/flexibility/tension_effect/state_thresholds), rest_beats, composability, overlap_signal |

Genre-native types (Trope, NarrativeShape) have per-genre schemas only — no cluster synthesis exists for these.

### 3.3 Schema Authoring Workflow

1. **Extract** JSON schemas from the comprehensive analysis document into discrete `json-schemas/*.schema.json` files (temporary scaffolding)
2. **Author** Pydantic models in `schemas/*.py` informed by those schemas
3. **Compare** `model_json_schema()` output against the extracted schemas — investigate any drift (intentional or not)
4. **Remove** the `json-schemas/` directory once Pydantic models are validated

The JSON schema files are scaffolding only — they make the Pydantic authoring and sub-agent review process tractable by providing discrete comparison targets rather than requiring parsing of the 1294-line analysis document.

---

## 4. Pipeline Architecture

### 4.1 Data Flow

```
Source .md file
    ↓
PromptBuilder.build_structure(type, raw_content, schema)
    → loads prompts/structure/{type}.md (type-specific template)
    → injects raw content + JSON schema
    → replaces the generic build_stage2() static method
    ↓
OllamaClient.generate_structured(model="qwen2.5:7b-instruct")
    → constrained JSON output
    ↓
Pydantic model_validate()
    → on failure: replace error section in prompt, retry (max 3)
    → on exhaust: write .errors.json
    ↓
Write sibling .json file + update manifest
```

**Note on `build_structure` vs `build_stage2`:** The existing `build_stage2()` is a static method with a hardcoded generic prompt. `build_structure()` is an instance method that loads type-specific templates from `prompts/structure/{type}.md` via `self.prompts_dir`, following the same pattern as `build_discovery()` and `build_synthesis()`. It replaces `build_stage2()` — the generic approach is insufficient for the quality extraction needed here. `build_stage2()` can be removed once `build_structure()` is in place.

**Note on retry strategy:** On validation failure, the error context *replaces* the previous error section rather than appending. This prevents error accumulation from pushing useful content out of the 7b model's context window.

### 4.2 Execution Phases

Phases respect data dependencies — foundation before dependents.

**Phase 0 — Preparation:**
- Rename `*.raw.md` → `*.md` across the corpus (~453 files)
- Remove stale `region.json` files (old schema, no useful signal)
- Remove existing Pydantic schemas (`schemas/shared.py`, `genre.py`, `spatial.py`, `intersections.py`) — these predate the inflection session and are incompatible with the new architecture
- Extract JSON schemas from analysis document into `json-schemas/`

**Phase 1 — Genre Dimensions (foundation):**
- 30 genres × GenreDimensions schema = 30 files
- Validates the most complex schema before dependents
- Catches issues that would cascade to all other types

**Phase 2 — Independent types:**
- Archetypes: 30 per-genre + 6 cluster = 36 files
- Settings: 30 + 6 = 36 files
- Ontological Posture: 30 + 6 = 36 files
- Scene Profiles: 30 + 6 = 36 files
- Tropes: 30 per-genre = 30 files
- Narrative Shapes: 30 per-genre = 30 files

**Phase 3 — Types that reference archetypes:**
- Dynamics: 30 + 6 = 36 files
- Goals: 30 + 6 = 36 files
- Archetype-Dynamics: 30 + 6 = 36 files

**Phase 4 — Spatial types:**
- Spatial Topology: 30 + 6 = 36 files
- Place Entities: 30 + 6 = 36 files

**Total: 414 structuring calls across all phases** (30 + 204 + 108 + 72).

### 4.3 Prompt Architecture

Each type gets a focused extraction prompt template:

- **Per-genre:** `prompts/structure/{type}.md` — extraction role, type-specific field guidance, numeric extraction rules, enum value mapping hints, flavor_text preservation directive
- **Cluster synthesis:** `prompts/structure/{type}-cluster.md` — same extraction role, plus canonical name extraction rules, genre variant list construction, navigational description guidance, uniqueness classification

Runtime injects the raw markdown content and target JSON schema into the template. The prompt is focused and scoped — only the context the 7b model needs for this specific extraction.

### 4.4 Infrastructure Reuse

| Component | Status | Notes |
|-----------|--------|-------|
| `pipeline/structure.py::run_structuring()` | **Keep** | Core loop: LLM → validate → retry → write |
| `OllamaClient.generate_structured()` | **Keep** | Constrained JSON generation |
| `pipeline/invalidation.py` | **Keep** | Manifest tracking, hash-based staleness |
| `pipeline/events.py` | **Keep** | JSONL event logging for observability |
| `PromptBuilder` | **Extend** | Add `build_structure()` instance method, replacing `build_stage2()` |
| `cli.py` | **Extend** | Add `structure` top-level subcommand |
| `schemas/*.py` | **Replace** | New schemas matching analysis proposals |

---

## 5. CLI Interface

The `structure` command is a new top-level subcommand, not nested under `genre` or `spatial`. This reflects its cross-cutting nature — it operates on all 12 types regardless of which pipeline phase produced them. The existing `genre structure` and `spatial structure` commands from the old pipeline are deprecated and should be removed; they used the old Pydantic schemas and are incompatible with the new architecture.

```bash
# Structure a single genre's dimensions
narrative-data structure genre-dimensions --genre folk-horror

# Structure all genres for a type
narrative-data structure genre-dimensions --all

# Structure per-genre discovery files for a type
narrative-data structure archetypes --all

# Structure cluster synthesis files for a type
narrative-data structure archetypes --clusters

# Structure everything for a type (per-genre + clusters)
narrative-data structure archetypes --all --clusters

# Force re-extraction (ignore manifest cache)
narrative-data structure genre-dimensions --all --force

# Dry run — show what would be structured for a specific type
narrative-data structure archetypes --plan
```

---

## 6. Batch Scripts

Shell scripts for running complete phases, matching the existing pattern (`run-discovery-extract.sh`, `run-discovery-synthesize.sh`):

```bash
# scripts/run-structure-p1.sh — Genre dimensions (foundation)
narrative-data structure genre-dimensions --all

# scripts/run-structure-p2.sh — Independent types
narrative-data structure archetypes --all --clusters
narrative-data structure settings --all --clusters
narrative-data structure ontological-posture --all --clusters
narrative-data structure scene-profiles --all --clusters
narrative-data structure tropes --all
narrative-data structure narrative-shapes --all

# scripts/run-structure-p3.sh — Types referencing archetypes
narrative-data structure dynamics --all --clusters
narrative-data structure goals --all --clusters
narrative-data structure archetype-dynamics --all --clusters

# scripts/run-structure-p4.sh — Spatial types
narrative-data structure spatial-topology --all --clusters
narrative-data structure place-entities --all --clusters
```

---

## 7. File Layout

### 7.1 New and Changed Files in `tools/narrative-data/`

**Schemas (replace all existing):**
```
schemas/
  shared.py                    — ContinuousAxis, WeightedTags, StateVariableTemplate, etc.
  genre_dimensions.py          — GenreDimensions
  archetypes.py                — Archetype + ClusterArchetype
  dynamics.py                  — Dynamic + ClusterDynamic
  goals.py                     — Goal + ClusterGoal
  archetype_dynamics.py        — ArchetypeDynamic + ClusterArchetypeDynamic
  scene_profiles.py            — SceneProfile + ClusterSceneProfile
  ontological_posture.py       — OntologicalPosture + ClusterOntologicalPosture
  settings.py                  — Settings + ClusterSettings
  spatial_topology.py          — SpatialTopologyEdge + ClusterSpatialTopology
  place_entities.py            — PlaceEntity + ClusterPlaceEntity
  tropes.py                    — Trope (per-genre only)
  narrative_shapes.py          — NarrativeShape (per-genre only)
```

**Prompts:**
```
prompts/structure/
  genre-dimensions.md
  archetypes.md / archetypes-cluster.md
  dynamics.md / dynamics-cluster.md
  goals.md / goals-cluster.md
  archetype-dynamics.md / archetype-dynamics-cluster.md
  scene-profiles.md / scene-profiles-cluster.md
  ontological-posture.md / ontological-posture-cluster.md
  settings.md / settings-cluster.md
  spatial-topology.md / spatial-topology-cluster.md
  place-entities.md / place-entities-cluster.md
  tropes.md
  narrative-shapes.md
```

**Source:**
```
src/narrative_data/
  prompts.py                   — extend: add build_structure() instance method, remove build_stage2()
  cli.py                       — extend: add structure top-level subcommand, deprecate genre/spatial structure
  pipeline/structure.py        — keep: run_structuring() core loop (called by commands)
  pipeline/structure_commands.py — new: structure_genre, structure_discovery, structure_type orchestration
```

**Note on module organization:** The orchestration commands live in `pipeline/structure_commands.py` (not a separate `structure/` package) to avoid a namespace collision with the existing `pipeline/structure.py`. The commands module imports `run_structuring()` from `pipeline/structure.py` and adds the type registry, path resolution, and batch iteration logic.

**Temporary scaffolding:**
```
json-schemas/                  — extracted from analysis, removed after Pydantic validation
  genre-dimensions.schema.json
  archetype.schema.json
  ... (11 total)
```

### 7.2 Data Output in `storyteller-data/`

After rename and structuring:

```
narrative-data/
  genres/folk-horror/
    region.md                  ← was region.raw.md
    region.json                ← new, GenreDimensions schema
    tropes.md                  ← was tropes.raw.md
    tropes.json                ← new, Trope schema (array)
    narrative-shapes.md        ← was narrative-shapes.raw.md
    narrative-shapes.json      ← new, NarrativeShape schema (array)

  discovery/archetypes/
    folk-horror.md             ← was folk-horror.raw.md
    folk-horror.json           ← new, Archetype schema (array)
    cluster-horror.md          ← was cluster-horror.raw.md
    cluster-horror.json        ← new, ClusterArchetype schema (array)
    ...
```

---

## 8. Error Handling and Observability

**On extraction failure:**
- 3 retries with Pydantic validation errors appended to prompt
- `.errors.json` written on retry exhaustion (raw output + errors + schema name)
- Pipeline continues to next file — failures don't block the batch
- Summary at end: N succeeded, M failed, K skipped (cached)
- Re-run with `--force` on failures only

**Observability:**
- JSONL pipeline events: `structure_started` / `structure_completed` / `structure_failed`
- Manifest updated per file (prompt_hash + content_digest)
- Console: rich progress output (type, genre, success/skip/fail)
- `narrative-data status` shows structuring progress alongside existing pipeline status

---

## 9. Testing Strategy

- **Schema tests:** Validate that each Pydantic model can round-trip (construct → dump → validate). Test field validators (0.0–1.0 range enforcement, enum membership).
- **Prompt builder tests:** Verify `build_structure()` loads correct template, injects content and schema, handles missing templates.
- **Structuring tests:** Mock OllamaClient, verify `run_structuring` retry behavior, `.errors.json` writing, manifest updates.
- **Integration test:** Structure a single known raw file (e.g., folk-horror archetypes) and validate the output against the schema. This catches prompt/schema mismatches that unit tests miss. Requires a running Ollama instance — feature-gate behind `test-llm` following the existing test tier pattern.

---

## 10. Key Files

| File | Purpose |
|------|---------|
| `storyteller-data/narrative-data/analysis/2026-03-21-comprehensive-terrain-analysis.md` | Source of JSON schema proposals (Section 7) |
| `storyteller-data/narrative-data/analysis/2026-03-21-unified-narrative-data-analysis.md` | Dimensional inventory and type design analysis |
| `docs/superpowers/specs/2026-03-18-primitive-first-narrative-data-design.md` | Pipeline architecture (predecessor) |
| `docs/technical/storykeeper-context-assembly.md` | Consumer of structured data |
| `tools/narrative-data/src/narrative_data/pipeline/structure.py` | Existing structuring infrastructure |
| `tools/narrative-data/src/narrative_data/config.py` | Model and path configuration |
