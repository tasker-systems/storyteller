# Narrative Data Tooling — Design Spec

**Date:** 2026-03-17
**Branch:** `jcoletaylor/narrative-data-exploratory-research`
**Status:** Design
**Meta-plan:** `docs/superpowers/specs/2026-03-16-scenes-chapters-stories-meta-plan.md`
**Roadmap reference:** Parts III, IV, V (data needs) — Tier B Exploratory Research

## Purpose

This spec defines the `narrative-data` CLI tool — a unified Python package for Tier B exploratory research. It generates genre/trope/narrative structure data (B.1), spatial/setting/place-entity data (B.2), and cross-pollination synthesis (B.3) through structured LLM elicitation against local Ollama models.

The generated data graduates beyond "training data" into core enrichment and world-building assets for the storyteller ecosystem. It lives in `storyteller-data/narrative-data/`, a new top-level namespace separate from the existing `training-data/descriptors/` pipeline.

## Context

### What exists today

The current descriptor system in `storyteller-data/training-data/descriptors/` provides flat, genre-unaware data: a single set of archetypes, profiles, dynamics, goals, settings, and cross-dimensions. Scene composition selects from these based on genre validity gates (which archetypes are valid for which genre), but the descriptors themselves don't vary by genre — a "mentor" archetype is the same character regardless of whether the story is folk horror or cozy fantasy.

### What this tool produces

Genre-contextualized expressions of all descriptor categories across ~20 genre regions, place-entity topologies with communicability profiles across ~5-6 setting types, and cross-domain synthesis that identifies gaps and enrichments at genre×setting intersections. Each generated entity carries provenance tracking and collaborative commentary fields where the elicitation model flags things it couldn't express within the schema.

### Relationship to existing tooling

The project has three existing Python packages under `tools/`:
- `tools/doc-tools/` — document extraction (Scrivener, DOCX)
- `tools/training/` — character prediction training + goal lexicon enrichment
- `tools/training/event_classifier/` — event classification training

All follow the same conventions: `hatchling` build backend, `ruff` linting (line-length 100), `uv` dependency management, `pytest` testing. The new `tools/narrative-data/` package follows these conventions.

The `goal_lexicon` sub-package in `tools/training/` validates the core pattern: `httpx` talking to Ollama for structured LLM elicitation. The narrative-data tool extends this into a two-stage pipeline.

---

## Architecture

### Two-Stage Pipeline

The tool uses two models in sequence, playing to each model's strengths:

**Stage 1 — Elicitation** (`qwen3.5:35b`): Rich, expansive generation. The large model receives composed prompts (core markdown template + combinatorial context + commentary directive) and produces organized, expressive markdown. Output is `raw.md` — the model's unconstrained thinking about the domain, with evaluative commentary and suggestions for things it couldn't express in the requested structure.

**Stage 2 — Structuring** (`qwen2.5:3b-instruct`): Mechanical transformation. The small instruct model receives the `raw.md` content plus the target Pydantic schema (exported as JSON Schema) and produces validated, schema-compliant JSON. This mirrors the proven intent-synthesis pattern already used in the storyteller engine — large model for richness, small model for structural compliance.

Both stages are independently re-runnable. Refining a prompt and re-running Stage 1 for a single genre region doesn't require re-running Stage 2 for other regions. Stage 2 can be re-run independently when schemas evolve.

**Validation and retry:**
1. Stage 2 output is validated against the Pydantic model
2. First retry: append validation errors to the prompt, ask the model to fix
3. Second retry: same, with accumulated errors
4. Third failure: write `{category}.errors.json` with validation failures and raw model output, log a warning, continue to next cell. Human review needed.

### Pipeline Dependency Ordering

**Within a genre region:**
1. `region` first — establishes dimensional position, tonal vocabulary, world affordances
2. All descriptor categories (`archetypes`, `tropes`, `narrative-shapes`, `dynamics`, `profiles`, `goals`, `settings`) — each receives the region as context, independent of each other

**Within a setting type:**
1. `setting-type` first — establishes atmospheric signature, genre associations
2. `place-entities` — informed by the setting type
3. `topology` — informed by the place entities
4. `tonal-inheritance` — informed by topology + place entities

**Cross-pollination (B.3):**
- Depends on B.1 and B.2 both reaching initial completion
- Receives both genre and spatial structured outputs as context

### Invalidation and Caching

Each structured `.json` records:
- Its own UUIDv7
- The prompt hash that produced its `raw.md`
- A content digest of its `raw.md`

Re-running without `--force` skips cells where prompt hash and upstream content are unchanged.

For intersections, `hash.json` stores a composite hash computed from upstream UUIDv7s + content digests. A quick calculation determines if anything upstream has changed in a way that requires reprocessing. The `status` command reports what's stale.

---

## Package Structure

```
tools/narrative-data/
├── pyproject.toml
├── prompts/
│   ├── genre/
│   │   ├── region.md
│   │   ├── archetypes.md
│   │   ├── tropes.md
│   │   ├── narrative-shapes.md
│   │   ├── dynamics.md
│   │   ├── profiles.md
│   │   ├── goals.md
│   │   └── settings.md
│   ├── spatial/
│   │   ├── setting-type.md
│   │   ├── place-entities.md
│   │   ├── topology.md
│   │   └── tonal-inheritance.md
│   └── cross-pollination/
│       └── synthesis.md
├── src/narrative_data/
│   ├── __init__.py
│   ├── cli.py                  # Subcommand dispatch
│   ├── ollama.py               # Thin httpx client for Ollama
│   ├── prompts.py              # Prompt loader + compositional builder
│   ├── schemas/
│   │   ├── __init__.py
│   │   ├── genre.py            # GenreRegion, Trope, NarrativeShape, etc.
│   │   ├── spatial.py          # PlaceEntity, TopologyEdge, CommunicabilityProfile, etc.
│   │   ├── shared.py           # NarrativeEntity base, GenerationProvenance, DimensionalPosition
│   │   └── intersections.py    # IntersectionSynthesis, hash computation
│   ├── pipeline/
│   │   ├── __init__.py
│   │   ├── elicit.py           # Stage 1: qwen3.5:35b → raw.md
│   │   ├── structure.py        # Stage 2: qwen2.5:3b-instruct → validated .json
│   │   └── invalidation.py     # UUIDv7 tracking, intersection-hash, skip logic
│   ├── genre/
│   │   ├── __init__.py
│   │   └── commands.py         # Genre-specific elicitation orchestration
│   ├── spatial/
│   │   ├── __init__.py
│   │   └── commands.py         # Spatial-specific elicitation orchestration
│   └── cross_pollination/
│       ├── __init__.py
│       └── commands.py         # B.3 cross-domain synthesis
└── tests/
    └── ...
```

---

## CLI Interface

```
narrative-data genre elicit [--regions folk-horror,nordic-noir] [--categories archetypes,tropes]
narrative-data genre structure [--regions folk-horror] [--force]
narrative-data spatial elicit [--settings gothic-mansion,pastoral-village]
narrative-data spatial structure [--settings gothic-mansion] [--force]
narrative-data cross-pollinate [--force]
narrative-data status                     # Pipeline health: what needs work, what's stale
narrative-data list genres                # All genre regions (JSON, pipeable to jq)
narrative-data list genres --region folk-horror
narrative-data list genres --category archetypes
narrative-data list spatial
narrative-data list spatial --setting gothic-mansion
narrative-data list intersections
narrative-data list intersections --stale
```

`status` answers "what needs work" — stale entries, missing stages, validation failures.

`list` answers "show me what we have" — query and inspect generated data. Output is JSON by default for piping to `jq`, with `--format table` for human-readable terminal output.

Filter flags (`--regions`, `--settings`, `--categories`) accept comma-separated values for targeting specific cells in the matrix. Omitting filters runs the full matrix.

---

## Output Structure

Generated data lives in `storyteller-data/narrative-data/`, organized hierarchically by domain then category:

```
storyteller-data/narrative-data/
├── genres/
│   ├── folk-horror/
│   │   ├── region.raw.md              # Stage 1 elicitation
│   │   ├── region.json                # Stage 2 structured output
│   │   ├── archetypes.raw.md
│   │   ├── archetypes.json
│   │   ├── tropes.raw.md
│   │   ├── tropes.json
│   │   ├── narrative-shapes.raw.md
│   │   ├── narrative-shapes.json
│   │   ├── dynamics.raw.md
│   │   ├── dynamics.json
│   │   ├── profiles.raw.md
│   │   ├── profiles.json
│   │   ├── goals.raw.md
│   │   ├── goals.json
│   │   ├── settings.raw.md
│   │   └── settings.json
│   ├── nordic-noir/
│   │   └── ...same structure
│   └── manifest.json                  # Registry: UUIDv7s, generation timestamps, prompt hashes
│
├── spatial/
│   ├── gothic-mansion/
│   │   ├── setting-type.raw.md
│   │   ├── setting-type.json
│   │   ├── place-entities.raw.md
│   │   ├── place-entities.json
│   │   ├── topology.raw.md
│   │   ├── topology.json
│   │   ├── tonal-inheritance.raw.md
│   │   └── tonal-inheritance.json
│   ├── pastoral-village/
│   │   └── ...
│   └── manifest.json
│
├── intersections/
│   ├── folk-horror×gothic-mansion/
│   │   ├── synthesis.raw.md
│   │   ├── synthesis.json
│   │   └── hash.json                  # Intersection hash + upstream UUIDv7 refs
│   └── manifest.json
│
└── meta/
    ├── schemas/                        # Exported JSON Schema from Pydantic models
    │   ├── genre-region.schema.json
    │   ├── place-entity.schema.json
    │   └── ...
    └── runs/                           # Generation run audit trail
        └── 2026-03-17T20-30-00.json
```

### Manifest files

Each domain-level `manifest.json` is the registry of what exists. It tracks:
- Entity UUIDv7s and names
- Generation timestamps
- Prompt hashes that produced each entry
- Content digests for invalidation
- Stage completion status (elicited / structured / both)

### Run logs

`meta/runs/` records each generation run: timestamp, model versions, prompt versions used, cells processed, validation results. Provides an audit trail for understanding drift and reproducing past runs.

---

## Pydantic Schema Design

Schemas serve triple duty: validate LLM output, export JSON Schema for Stage 2 prompts, and act as the proposed schemas the meta-plan requires.

### Shared base types (`schemas/shared.py`)

```python
class GenerationProvenance(BaseModel):
    """Tracks how this entity was generated."""
    prompt_hash: str
    model: str                           # e.g., "qwen3.5:35b"
    generated_at: str                    # ISO 8601
    source_content_digest: str | None = None

class NarrativeEntity(BaseModel):
    """Base for all generated entities."""
    entity_id: str                       # UUIDv7, assigned by pipeline
    name: str
    description: str
    commentary: str | None = None        # Model's evaluative notes
    suggestions: list[str] = []          # Things it couldn't express in the schema
    provenance: GenerationProvenance
    provenance_edges: list[ProvenanceEdge] = []  # Attribution model (see Provenance strawman)

class DimensionalPosition(BaseModel):
    """Weighted position along a named dimension."""
    dimension: str
    value: float                         # -1.0 to 1.0 (bipolar) or 0.0 to 1.0 (unipolar)
    note: str | None = None
```

### Genre domain (`schemas/genre.py`)

```python
class WorldAffordances(BaseModel):
    magic: str
    technology: str
    violence: str
    death: str
    supernatural: str
    commentary: str | None = None

class GenreRegion(NarrativeEntity):
    aesthetic: list[DimensionalPosition]
    tonal: list[DimensionalPosition]
    thematic: list[DimensionalPosition]
    structural: list[DimensionalPosition]
    world_affordances: WorldAffordances
    trope_refs: list[str] = []

class Trope(NarrativeEntity):
    genre_associations: list[str]
    narrative_function: str
    subversion_patterns: list[SubversionPattern] = []
    reinforcement_patterns: list[str] = []

class SubversionPattern(BaseModel):
    name: str
    description: str
    effect: str                          # What the subversion achieves narratively

class NarrativeBeat(BaseModel):
    name: str
    description: str
    position: str                        # Where in the arc this beat sits
    flexibility: str                     # How much this beat can move or be skipped

class NarrativeShape(NarrativeEntity):
    beats: list[NarrativeBeat]
    genre_associations: list[str]
    tension_profile: str

class GenreArchetype(NarrativeEntity):
    """Genre-contextualized expression of an archetype."""
    base_archetype_ref: str | None = None
    genre_ref: str
    personality_axes: list[DimensionalPosition]
    typical_roles: list[str]
    genre_specific_notes: str

class GenreDynamic(NarrativeEntity):
    """Genre-contextualized expression of a relational dynamic."""
    base_dynamic_ref: str | None = None
    genre_ref: str
    role_a_expression: str               # How role_a manifests in this genre
    role_b_expression: str               # How role_b manifests in this genre
    relational_texture: str              # How the dynamic's emotional/trust axes feel in this genre
    typical_escalation: str              # How tension in this dynamic typically builds genre-specifically
    genre_specific_notes: str

class GenreProfile(NarrativeEntity):
    """Genre-contextualized expression of a scene profile."""
    base_profile_ref: str | None = None
    genre_ref: str
    scene_shape: str                     # How this scene type unfolds in this genre
    tension_signature: str               # Genre-specific tension characteristics
    characteristic_moments: list[str]    # Key beats that define this profile in this genre
    genre_specific_notes: str

class GenreGoal(NarrativeEntity):
    """Genre-contextualized expression of a narrative goal."""
    base_goal_ref: str | None = None
    genre_ref: str
    pursuit_expression: str              # How characters pursue this goal in this genre
    success_shape: str                   # What success looks like genre-specifically
    failure_shape: str                   # What failure looks like genre-specifically
    genre_specific_notes: str

class GenreSetting(NarrativeEntity):
    """Genre-contextualized setting vocabulary."""
    genre_ref: str
    typical_locations: list[str]
    atmospheric_vocabulary: list[str]
    sensory_vocabulary: list[str]
    genre_specific_notes: str
```

### Spatial domain (`schemas/spatial.py`)

```python
class SettingType(NarrativeEntity):
    genre_associations: list[str]
    atmospheric_signature: str
    sensory_palette: list[str]
    temporal_character: str

class SensoryDetail(BaseModel):
    sense: str                           # "sight", "sound", "smell", "touch", "taste"
    detail: str
    emotional_valence: str | None = None

class CommunicabilityProfile(BaseModel):
    atmospheric: str
    sensory: str
    spatial: str
    temporal: str

class PlaceEntity(NarrativeEntity):
    setting_type_ref: str
    narrative_function: str
    communicability: CommunicabilityProfile
    sensory_details: list[SensoryDetail]

class TopologyEdge(BaseModel):
    edge_id: str                         # UUIDv7
    from_place: str
    to_place: str
    adjacency_type: str
    friction: str
    permeability: list[str]
    tonal_shift_note: str | None = None

class TonalInheritanceRule(NarrativeEntity):
    setting_type_ref: str
    rule: str                            # Natural language description of the rule
    applies_across: str                  # What kind of boundary this rule governs
    friction_level: str
    examples: list[str] = []
```

### Intersections (`schemas/intersections.py`)

```python
class UpstreamRef(BaseModel):
    entity_id: str
    content_digest: str
    domain: str                          # "genre" | "spatial"

class Enrichment(BaseModel):
    target_entity_id: str                # What entity was enriched
    enrichment_type: str                 # "new_detail", "tonal_refinement", "gap_fill", etc.
    content: str

class IntersectionSynthesis(NarrativeEntity):
    upstream_refs: list[UpstreamRef]
    content_hash: str
    enrichments: list[Enrichment]
    gaps_identified: list[str]
    new_entries: list[str]               # UUIDv7s of any new entities created
```

### Collection convention

Each category `.json` file contains a JSON array of the relevant entity type. For example, `archetypes.json` contains `[GenreArchetype, ...]`, `tropes.json` contains `[Trope, ...]`. Stage 2 validation wraps the target Pydantic model in `list[TargetType]` and validates the entire array.

The `region.json` and `setting-type.json` files are exceptions — they contain a single object (the genre region or setting type itself), not an array.

### Provenance strawman (`schemas/shared.py`)

The meta-plan requires a provenance graph strawman to validate the multi-source attribution model from Part IV of the roadmap. For this cycle, only LLM elicitation (strategy 1) populates provenance. The schema is designed to accommodate future multi-source integration:

```python
class ProvenanceEdge(BaseModel):
    """Attribution edge from source to knowledge node.

    Currently populated only for LLM elicitation. Schema designed
    to support future strategies: public domain analysis, CC-BY-SA
    RPG module extraction, and cross-source synthesis.
    """
    source_id: str                       # Identifier for the source
    source_type: str                     # "llm_elicited" | "public_domain" | "cc_by_sa" | "hand_authored"
    contribution_type: str               # "originated" | "reinforced" | "refined"
    weight: float                        # 0.0 to 1.0
    license: str | None = None           # License identifier for non-LLM sources
    extractable: bool = True             # Can this source's contribution be isolated for removal?
    notes: str | None = None
```

For this cycle, all generated entities carry `GenerationProvenance` (tracks the mechanical generation process) and optionally `provenance_edges: list[ProvenanceEdge]` on `NarrativeEntity` (tracks the attribution model). LLM-elicited entities get a single `ProvenanceEdge` with `source_type="llm_elicited"` and `weight=1.0`. Multi-source convergence weighting is deferred to a future cycle when strategies 2-3 are implemented.

### Schema design principles

- **Strings over enums** for exploratory fields. This is research tooling generating data whose shape is still being discovered. Premature enumeration constrains the LLM's expressiveness. Enums can be tightened as patterns stabilize across runs.
- **`commentary` and `suggestions` on `NarrativeEntity`** make elicitation collaborative. Every generated entity can carry the model's notes about what it couldn't express within the schema, patterns it noticed, or connections it wants to flag. These fields are populated in Stage 1 and preserved through Stage 2.
- **Cross-references use UUIDv7 strings**, not embedded objects. Keeps files flat, independently loadable, and avoids deep nesting that would complicate both LLM output and human review.
- **`base_*_ref` fields** on genre-contextualized types link back to existing flat descriptors by `entity_id` (UUIDv7), matching the identifiers added by the `descriptor-migration` tool. Where a genre-contextualized entity doesn't correspond to an existing flat descriptor, this field is `None`.
- **Domain-specific structure on genre-contextualized types** rather than flattening to a single `genre_specific_notes` string. Each type carries fields that reflect its source descriptor's domain (e.g., GenreDynamic has relational texture and escalation patterns, GenreProfile has scene shape and characteristic moments). This gives Stage 2 concrete targets and makes the data more useful for Tier C consumption.

---

## Prompt Architecture

### Three layers

**Layer 1 — Core prompts** (markdown files in `prompts/`)

Domain knowledge and elicitation framing. These establish what we're asking about: the dimensional framework for genre regions, the communicability model for place-entities, the trope vocabulary. Each file is a self-contained creative brief for the elicitation model.

These are the primary creative artifact of the tooling — iterated on as outputs are reviewed, refined to improve elicitation quality.

**Layer 2 — Compositional builder** (`prompts.py`)

Python code that assembles the final Stage 1 prompt from:

1. The core prompt (loaded from markdown file)
2. Combinatorial context — injected dynamically:
   - For genre archetypes: the already-generated `region.json` so the model knows the genre's dimensional position
   - For spatial topology: the already-generated `place-entities.json`
   - For cross-pollination: both genre and spatial structured outputs
   - The existing flat descriptors from `storyteller-data/training-data/descriptors/` so the model knows what vocabulary already exists
3. Commentary directive — standard suffix asking for `_commentary` and `_suggestions` sections
4. The specific target — "Now do this for: folk horror" or "Now do this for: folk horror × gothic mansion"

The builder handles mechanical composition; creative content lives in the markdown files.

**Layer 3 — Structuring prompt** (Stage 2, code-generated)

Consistent across all domains:

```
Given the following content:
---
{raw_md_content}
---

Produce JSON matching this schema:
{json_schema_from_pydantic}

Rules:
- Preserve all substantive information from the source
- Map evaluative notes to the commentary and suggestions fields
- Do not invent information not present in the source
- If a field cannot be populated from the source, use null
```

The schema varies by target type; the instruction pattern doesn't.

### Dependency awareness

The compositional builder enforces dependency ordering:
- Genre `region` must be elicited before genre descriptor categories (the region provides context)
- Setting `setting-type` before `place-entities` before `topology` before `tonal-inheritance`
- Cross-pollination requires both genre and spatial structured outputs

When a dependency hasn't been generated yet, the builder either runs it first (if within the same `elicit` invocation) or errors with a clear message about what's missing.

---

## Infrastructure

### Ollama client (`ollama.py`)

Thin `httpx` wrapper with two methods:

- `generate(model, prompt, **kwargs)` — Stage 1 calls, returns raw text
- `generate_structured(model, prompt, schema, **kwargs)` — Stage 2 calls, passes JSON Schema as format constraint if supported, returns parsed JSON

Timeout configuration appropriate for `qwen3.5:35b` (which may take significant time for rich elicitation). Connection to `localhost:11434` (Ollama default).

### Data path resolution

Reads `STORYTELLER_DATA_PATH` environment variable (same pattern as `storyteller-ml`). The output path is `{STORYTELLER_DATA_PATH}/narrative-data/`. The existing descriptors are read from `{STORYTELLER_DATA_PATH}/training-data/descriptors/`.

### Python package conventions

Following established project patterns:
- `hatchling` build backend
- `ruff` linting with `select = ["E", "F", "W", "I", "UP", "B", "C4", "SIM"]`, line-length 100
- `uv` for dependency management
- `pytest` for testing
- Python >=3.11

### Dependencies

- `httpx` — Ollama communication
- `pydantic` — schema validation and JSON Schema export
- `uuid-utils` or `uuid6` — UUIDv7 generation (same as `descriptor-migration` tool)
- `click` — CLI framework (richer subcommand support than argparse)
- `rich` — terminal output formatting for `--format table` and progress display

---

## Target Genre Regions (~25)

The list is designed for broad dimensional coverage: genre clusters that test differentiation within proximity (how does the system distinguish related genres?) as well as distance (how does it handle genres with almost no overlap?). These are starting points, not a closed set. The tool supports adding new regions at any time.

**Horror:**
1. Folk horror — rural dread, community as threat, pagan undercurrents
2. Cosmic horror — insignificance, incomprehensible scale, epistemological dread

**Fantasy:**
3. High / epic fantasy — quest structures, moral clarity, world-at-stake
4. Dark fantasy — moral ambiguity, cost of power, grimdark adjacent
5. Cozy fantasy — low stakes, community warmth, gentle conflicts
6. Fairy tale / mythic — archetypal, transformative, rule-of-three logic
7. Urban fantasy — magic in modern mundane, hidden worlds, identity straddling

**Science fiction:**
8. Hard sci-fi — extrapolation, epistemic rigor, ideas-as-character
9. Space opera — vast scale, political intrigue, operatic emotion
10. Cyberpunk — corporate dystopia, body modification, street-level resistance
11. Solarpunk — optimistic futures, ecological harmony, community solutions

**Mystery / thriller:**
12. Nordic noir — institutional rot, bleak landscapes, procedural
13. Cozy mystery — amateur sleuth, tight community, no graphic violence
14. Psychological thriller — unreliable perception, paranoia, interior tension

**Romance & intersections:**
15. Romantasy — fantasy + romance dual engines, relationship as plot
16. Historical romance — period constraints, social navigation, emotional core
17. Contemporary romance — modern relationships, identity, emotional realism

**Literary / realistic:**
18. Literary fiction — interiority, ambiguity, language-as-subject
19. Magical realism — the uncanny within the ordinary, culturally grounded
20. Southern gothic — decay, grotesque, place-as-character, social critique

**Historical / period:**
21. Historical fiction — research-grounded, period immersion, real constraints
22. Westerns — frontier, moral codes, landscape, expansion/displacement tension

**Adventure / action:**
23. Swashbuckling adventure — wit, daring, set pieces, moral lightness
24. Survival fiction — resource scarcity, environmental hostility, human limits

**Speculative / liminal:**
25. Pastoral / rural fiction — seasonal rhythms, land-as-relationship, quiet tension

## Target Setting Types (~22)

Settings cover genre-agnostic spaces (a kitchen is a kitchen in any genre), genre-inflected spaces (a village square reads differently in cozy mystery vs folk horror), and genre-specific spaces (a space station only makes sense in sci-fi). Spatial diversity spans indoor/outdoor, urban/rural/wilderness, intimate/vast, natural/built. These are starting points, not a closed set.

**Domestic / intimate:**
1. Family home — kitchen, bedrooms, attic, basement, garden
2. Inn / tavern — social hub, travelers, rumors, warmth against the outside
3. Boarding school — hierarchy, secrets, enclosed community, coming-of-age

**Urban:**
4. City streets — alleys, storefronts, crowds, anonymity
5. Market / bazaar — commerce, noise, sensory overload, chance encounters
6. Government building — bureaucracy, power, corridors, restricted areas
7. Underground / subway — transit, liminal, strangers in proximity, unease

**Grand / institutional:**
8. Gothic mansion — decay, rooms with purpose, verticality, secrets in architecture
9. Cathedral / temple — sacred space, acoustics, light, moral weight
10. Castle / fortress — defense, hierarchy, hidden passages, siege
11. University / library — knowledge, stacks, quiet intensity, discovery

**Rural / pastoral:**
12. Pastoral village — square, church, pub, surrounding fields, community rhythms
13. Farmstead — isolation, seasonal labor, land-as-relationship, self-sufficiency
14. Coastal settlement — harbor, cliffs, weather, the sea as presence

**Wilderness / natural:**
15. Dense forest — disorientation, canopy, trails that mislead, things watching
16. Mountain pass — exposure, altitude, narrow paths, weather as adversary
17. Desert / wasteland — emptiness, heat/cold extremes, mirages, endurance
18. River / lake shore — boundary between worlds, reflection, crossing

**Speculative / constructed:**
19. Space station — enclosed, artificial atmosphere, viewports, systems dependency
20. Sailing vessel — confined, hierarchy, weather, isolation from land
21. Train / carriage — motion, compartments, forced proximity, destination-driven
22. Ruins / archaeological site — layered time, what was here before, discovery and danger

---

## Deliberately Excluded Descriptor Categories

The existing flat descriptors include `cross-dimensions.json` (age, gender, species demographic axes) and `axis-vocabulary.json` (tensor axis definitions). These are **not genre-contextualized** in this cycle:

- **Cross-dimensions** are demographic axes that modify character tensors additively. They are genre-independent by design — age affects personality axes the same way regardless of whether the story is folk horror or cozy fantasy. If genre-specific demographic expressions emerge as a need during elicitation (e.g., "youth means something different in fairy tale than in noir"), the schema can be extended, but this is not assumed upfront.
- **Axis vocabulary** defines the tensor dimensions themselves. These are structural/mathematical definitions, not narrative content that varies by genre. The genre-contextualized archetypes already carry `personality_axes: list[DimensionalPosition]` that reference these axes in context.

## Cross-Genre Analysis

The meta-plan lists "cross-genre analysis: dimensional overlap, natural clusters, discriminating dimensions, false boundaries" as a B.1 scope item, with an analysis doc as output. This is **human-guided analytical work**, not automated tooling. The `list` command provides the data access needed to support this analysis (e.g., `narrative-data list genres | jq` to compare dimensional positions across regions), but the analysis itself — identifying patterns, surprises, and recommendations for Tier C — is a human activity that produces a standalone document, not a tool feature. The B.3 cross-pollination synthesis partially overlaps with this concern but focuses on genre×setting intersections rather than genre×genre patterns.

## What This Spec Does Not Contain

- **The prompts themselves** — those are authored during implementation and iterated based on output quality
- **Detailed testing strategy** — the implementation plan will define what's testable (schema validation, prompt composition, invalidation logic) vs. what requires human review (elicitation quality)
- **Tier C integration** — how the generated data feeds into the dramaturge, world agent, and scene resolution is Tier C's design concern. This tool produces the data; Tier C consumes it.
- **Timeline estimates** — same operational constraint as all project work (single developer, evening/weekend hours)
