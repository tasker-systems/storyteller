# Primitive-First Narrative Data Architecture

**Date:** 2026-03-18
**Branch:** `jcoletaylor/narrative-data-exploratory-research`
**Status:** Design — pending implementation plan
**Supersedes:** The derivative-category approach in `2026-03-17-narrative-data-tooling-design.md` §"Two-Stage Pipeline" for categories other than genre regions. Genre region elicitation (30 regions, 2 passes) remains valid and is the foundation this design builds on.

---

## Motivation

The original narrative-data pipeline was genre-centric: for each genre region, elicit derivative categories (archetypes, tropes, dynamics, etc.) scoped to that genre. A test run revealed two problems:

1. **Prompt bloat.** Derivative prompts injected all existing descriptor JSON (~450KB) as context, causing 30+ minute generation times for a single cell.
2. **Wrong direction of discovery.** Archetypes, dynamics, and other primitives have genre-agnostic identities that should be discovered and described independently before being elaborated through a genre lens. The genre-centric approach skipped this foundational layer.

The new architecture inverts the pipeline: discover primitives from the genre corpus, describe them standalone, then produce genre-specific elaborations. Each prompt gets exactly the context it needs — no descriptor dumps, no 450KB payloads.

---

## Architecture Overview

### Two Layers

**Layer 0 — Standalone Primitives.** Genre-agnostic descriptions of narrative building blocks (~25-30 per type). Rich, dimensional, exploratory — the same quality as genre region descriptions. These are the narrator's vocabulary: what an archetype *is*, independent of any genre.

**Layer 1 — Genre × Primitive Elaborations.** What changes about a primitive when it operates inside a specific genre. Axis shifts, forbidden expressions, narrative function. Focused on the *delta*, not restating what's already known. These are the narrator's genre-specific guidance.

### Primitive Types

| Type | Layer 0 | Layer 1 | Discovery method |
|------|---------|---------|-----------------|
| Archetypes | ~25-30 standalone | genre × archetype (selective) | Extracted from genre corpus |
| Dynamics | ~25-30 standalone | genre × dynamic (selective) | Extracted from genre corpus |
| Goals | ~20-30 standalone | genre × goal (selective) | Extracted from genre corpus |
| Profiles | ~10-15 standalone | genre × profile (selective) | Extracted from genre corpus |
| Settings | ~20-25 standalone | genre × setting (selective) | Extracted from genre corpus |
| Tropes | — | genre-native (per genre) | No Layer 0; genre-specific only |
| Narrative-shapes | — | genre-native (per genre) | No Layer 0; genre-specific only |

Tropes and narrative-shapes are genre-native primitives — they only exist within the context of a genre (or genre cluster). They skip directly to Layer 1 with no standalone ancestor.

### Why No Layer 2

The original design considered Layer 2 combinations (genre × archetype × dynamic, genre × setting × profile). These were cut because they pre-compute what the narrator should synthesize in context. The narrator receives Layer 0 and Layer 1 data as reference material and performs the combinatorial synthesis during scene resolution, with full scene state available. Pre-computing triple intersections produces static analysis where dynamic judgment is needed.

---

## Discovery Pipeline

Primitives are not seeded from convention or existing descriptor files. They are discovered from the genre corpus through a four-phase pipeline.

### Phase 1: Per-Genre Extraction

For each of the 30 genre regions, ask qwen3.5:35b: "Given this genre's dimensional description, what 5-8 [archetypes] are essential or distinctive to this genre?"

The prompt asks for each candidate:
- **Name** — descriptive label
- **Why this genre** — what axes, affordances, or constraints give rise to it
- **Distinguishing tension** — what internal contradiction makes it interesting here
- **Overlap signal** — what other genres likely produce this archetype, and how the expression differs

**Input:** One genre region.raw.md (~15KB). **Output:** Per-genre extraction file. **Volume:** 30 calls per primitive type.

This grounds primitive discovery in the genre data itself. Archetypes emerge from what genres actually need, not from a conventional list.

### Phase 2: Cluster Synthesis

Group per-genre extractions by genre cluster and synthesize into ~8-12 distinct primitives per cluster. The synthesis prompt asks to:
- Merge variants of the same primitive across genres, preserving genre-specific expressions
- Keep genuinely distinct primitives separate, explaining the discriminating dimension
- Flag primitives unique to one genre vs. common across the cluster vs. likely universal

**Genre clusters:**

| Cluster | Genres |
|---------|--------|
| Horror | folk-horror, cosmic-horror, horror-comedy |
| Fantasy | high-epic-fantasy, dark-fantasy, cozy-fantasy, fairy-tale-mythic, urban-fantasy, quiet-contemplative-fantasy |
| Sci-fi | hard-sci-fi, space-opera, cyberpunk |
| Mystery/thriller | nordic-noir, cozy-mystery, psychological-thriller, domestic-noir |
| Romance | romantasy, historical-romance, contemporary-romance |
| Realism/gothic/other | southern-gothic, westerns, swashbuckling-adventure, survival-fiction, working-class-realism, pastoral-rural-fiction, classical-tragedy |

Modifier regions (solarpunk, historical-fiction, literary-fiction, magical-realism) are included in their natural cluster or handled as a cross-cutting concern in synthesis.

**Input:** 4-8 per-genre extraction files (~30-60KB). **Output:** Cluster synthesis file. **Volume:** 6 calls per primitive type.

**Human review gate:** After cluster synthesis, review the merged list. Decide what merges, what stays distinct, what's missing. This is the list that becomes Layer 0.

### Phase 3: Layer 0 Elicitation

From the synthesized list, elicit each primitive individually with a rich prompt: dimensional positioning, behavioral patterns, narrative functions, genre-agnostic invariants, what flexes depending on context. Same quality and format as genre region descriptions, including `_commentary` and `_suggestions` sections.

**Input:** Prompt template + primitive name and cluster-synthesis description (~2-3KB). **Output:** Standalone primitive description. **Volume:** ~25-30 per type.

### Phase 4: Layer 1 Elicitation

Two sub-types:

**Phase 4a — Genre × Primitive Elaborations.** For each relevant genre × primitive pair, analyze what is specific, unique, or transformed. Focus on axis shifts, affordance effects, exclusions, and narrative function. Input is the genre region description + the Layer 0 primitive description (~25-30KB total).

Crucially, **not every combination is elicited.** Phase 2 cluster synthesis identifies which primitives are relevant to which genres. An archetype flagged as "unique to horror" doesn't need elaboration for every romance genre.

**Phase 4b — Genre-Native Tropes and Narrative-Shapes.** Elicited per-genre using the existing prompt templates, refactored to use only the genre region description as context (no descriptor injection). Same quality target as other outputs.

---

## Data Layout

```
storyteller-data/narrative-data/
├── genres/                              # Layer 0 for genres (EXISTING, complete)
│   ├── manifest.json
│   ├── folk-horror/
│   │   ├── region.raw.md
│   │   ├── region.json
│   │   ├── tropes.raw.md               # Genre-native (Phase 4b)
│   │   ├── narrative-shapes.raw.md      # Genre-native (Phase 4b)
│   │   └── elaborations/               # Layer 1: genre × primitive
│   │       ├── archetypes/
│   │       │   ├── mentor.raw.md
│   │       │   └── trickster.raw.md
│   │       ├── dynamics/
│   │       ├── settings/
│   │       ├── profiles/
│   │       └── goals/
│   └── ...30 genres
│
├── archetypes/                          # Layer 0 standalone primitives
│   ├── manifest.json
│   ├── mentor/
│   │   ├── raw.md
│   │   └── raw.json                     # Optional structured extraction
│   ├── trickster/
│   └── ...~25-30 archetypes
│
├── dynamics/                            # Layer 0
│   ├── manifest.json
│   └── ...
│
├── settings/                            # Layer 0
├── profiles/                            # Layer 0
├── goals/                               # Layer 0
│
└── discovery/                           # Phase 1-2 working artifacts
    ├── archetypes/
    │   ├── folk-horror.raw.md           # Phase 1: per-genre extraction
    │   ├── cosmic-horror.raw.md
    │   ├── cluster-horror.raw.md        # Phase 2: cluster synthesis
    │   ├── cluster-fantasy.raw.md
    │   └── ...
    ├── dynamics/
    ├── settings/
    ├── profiles/
    └── goals/
```

**Key decisions:**
- Discovery artifacts are preserved — they're research data, not temp files
- Layer 1 lives under `genres/{slug}/elaborations/` — genre remains the organizing principle for genre-specific content
- Tropes and narrative-shapes stay flat under genre (not in elaborations/) since they have no standalone parent
- Each primitive type and discovery directory gets its own manifest
- The existing `genres/manifest.json` is unchanged

---

## Tooling Changes

### CLI Commands

New commands for the discovery and primitive pipeline:

```
# Phase 1: Per-genre extraction
narrative-data discover extract --type archetypes [--genres folk-horror,cosmic-horror]

# Phase 2: Cluster synthesis
narrative-data discover synthesize --type archetypes [--cluster horror]

# Phase 3: Layer 0 standalone elicitation
narrative-data primitive elicit --type archetypes [--primitives mentor,trickster]

# Phase 3: Layer 0 structured extraction (optional)
narrative-data primitive structure --type archetypes [--primitives mentor]

# Phase 4a: Layer 1 genre × primitive elaboration
narrative-data genre elaborate --type archetypes --genres folk-horror [--primitives mentor]

# Phase 4b: Genre-native tropes/shapes
narrative-data genre elicit-native --type tropes --genres folk-horror
```

The existing `narrative-data genre elicit --categories region` continues to work for genre regions.

### New Modules

- `src/narrative_data/discovery/commands.py` — Phase 1 extraction + Phase 2 synthesis
- `src/narrative_data/primitive/commands.py` — Phase 3 elicitation + structuring
- Extend `src/narrative_data/genre/commands.py` — add `elaborate` and `elicit_native`

### New Prompt Templates

- `prompts/discovery/extract-{type}.md` — Phase 1 (one per primitive type)
- `prompts/discovery/synthesize-{type}.md` — Phase 2 (one per primitive type)
- `prompts/primitive/{type}.md` — Phase 3 (one per primitive type)
- `prompts/genre/elaborate-{type}.md` — Phase 4a (one per primitive type)
- Existing `prompts/genre/tropes.md` and `prompts/genre/narrative-shapes.md` refactored for Phase 4b

### Removed

- `_load_descriptor_context()` — no prompt in the new pipeline needs old flat descriptor files
- Old derivative category flow in `genre/commands.py` — the path where `genre elicit --categories archetypes` injected descriptors and produced genre-scoped derivatives is replaced by the Phase 1-4 pipeline. The `genre elicit` command retains only `--categories region` for genre region elicitation. Running `genre elicit --categories archetypes` should produce a clear error directing to the new `discover` / `primitive` / `genre elaborate` commands.

### Unchanged

- `OllamaClient`, `PromptBuilder`, `pipeline/elicit.py`, `pipeline/invalidation.py`, `pipeline/structure.py` — generic infrastructure, works for all phases
- File versioning, manifest format, hash-based staleness detection
- Genre region data and genre region elicitation path
- 7b-instruct structuring pipeline

### Configuration

Genre cluster mapping added to config:

```python
GENRE_CLUSTERS: dict[str, list[str]] = {
    "horror": ["folk-horror", "cosmic-horror", "horror-comedy"],
    "fantasy": ["high-epic-fantasy", "dark-fantasy", "cozy-fantasy",
                "fairy-tale-mythic", "urban-fantasy", "quiet-contemplative-fantasy"],
    "sci-fi": ["hard-sci-fi", "space-opera", "cyberpunk"],
    "mystery-thriller": ["nordic-noir", "cozy-mystery", "psychological-thriller", "domestic-noir"],
    "romance": ["romantasy", "historical-romance", "contemporary-romance"],
    "realism-gothic-other": ["southern-gothic", "westerns", "swashbuckling-adventure",
                             "survival-fiction", "working-class-realism",
                             "pastoral-rural-fiction", "classical-tragedy"],
}
```

Primitive type registry (parallel to existing `GENRE_CATEGORIES`):

```python
PRIMITIVE_TYPES: list[str] = [
    "archetypes", "dynamics", "goals", "profiles", "settings",
]

GENRE_NATIVE_TYPES: list[str] = [
    "tropes", "narrative-shapes",
]
```

---

## Pipeline Control Plane

With ~580 LLM calls across multiple sessions and days, the pipeline must be self-describing — no reliance on handoff documents or human memory to track where we are. An append-only event log provides observability, resumability, and review gate enforcement.

### Event Log

```
storyteller-data/narrative-data/pipeline.jsonl
```

Each line is a JSON event recording a pipeline action:

```jsonl
{"event":"extract_started","phase":1,"type":"archetypes","genre":"folk-horror","timestamp":"2026-03-18T18:30:00Z"}
{"event":"extract_completed","phase":1,"type":"archetypes","genre":"folk-horror","timestamp":"2026-03-18T18:35:12Z","output":"discovery/archetypes/folk-horror.raw.md","content_digest":"abc123..."}
{"event":"synthesize_started","phase":2,"type":"archetypes","cluster":"horror","timestamp":"2026-03-18T19:00:00Z"}
{"event":"synthesize_completed","phase":2,"type":"archetypes","cluster":"horror","timestamp":"2026-03-18T19:08:45Z","output":"discovery/archetypes/cluster-horror.raw.md","primitives_found":9}
{"event":"review_gate","phase":2,"type":"archetypes","decision":"approved","primitives":["mentor","trickster","outsider","sacrifice","guardian","corrupted-authority"],"timestamp":"2026-03-18T20:00:00Z","note":"merged 3 duplicates, added 1 from gap analysis"}
{"event":"elicit_started","phase":3,"type":"archetypes","primitive":"mentor","timestamp":"2026-03-19T08:00:00Z"}
{"event":"elicit_completed","phase":3,"type":"archetypes","primitive":"mentor","timestamp":"2026-03-19T08:05:30Z","output":"archetypes/mentor/raw.md"}
{"event":"elaborate_started","phase":4,"type":"archetypes","genre":"folk-horror","primitive":"mentor","timestamp":"2026-03-19T09:00:00Z"}
{"event":"elaborate_completed","phase":4,"type":"archetypes","genre":"folk-horror","primitive":"mentor","timestamp":"2026-03-19T09:05:15Z","output":"genres/folk-horror/elaborations/archetypes/mentor.raw.md"}
```

**Design principles:**
- **Append-only** — never edit or delete events. Same philosophy as the event ledger in storyteller-core.
- **Events, not state** — current state is derived by replaying the log. What's completed is computed, not maintained.
- **Review gates are events** — when a human approves Phase 2 output and confirms the primitive list, that decision is recorded with the approved list and any notes.
- **Lightweight** — no separate database, just a JSONL file that grows.

### Event Schema

All events share common fields:

| Field | Type | Description |
|-------|------|-------------|
| `event` | string | Event type (see below) |
| `phase` | int | Pipeline phase (1-4) |
| `type` | string | Primitive type (archetypes, dynamics, etc.) |
| `timestamp` | string | ISO 8601 UTC |

Event-specific fields:

| Event | Additional fields |
|-------|------------------|
| `extract_started` | `genre` |
| `extract_completed` | `genre`, `output` (relative path), `content_digest` |
| `synthesize_started` | `cluster` |
| `synthesize_completed` | `cluster`, `output`, `primitives_found` (count) |
| `review_gate` | `decision` (approved/rejected), `primitives` (approved list), `note` (optional) |
| `elicit_started` | `primitive` |
| `elicit_completed` | `primitive`, `output`, `content_digest` |
| `elaborate_started` | `genre`, `primitive` |
| `elaborate_completed` | `genre`, `primitive`, `output`, `content_digest` |
| `elicit_native_started` | `genre`, `native_type` (tropes/narrative-shapes). Phase is always 4. |
| `elicit_native_completed` | `genre`, `native_type`, `output`, `content_digest`. Phase is always 4. |

### CLI Commands

```
# Show pipeline progress for a primitive type (or all types)
narrative-data pipeline status [--type archetypes]

# Resume pipeline execution from where it left off
narrative-data pipeline resume --type archetypes [--phase 1]

# Record a human review gate decision
narrative-data pipeline approve --type archetypes --phase 2 \
  --primitives mentor,trickster,outsider,sacrifice,guardian,corrupted-authority \
  [--note "merged 3 duplicates, added 1 from gap analysis"]
```

**`status`** reads the JSONL, computes current state per type and phase:

```
Pipeline Status: archetypes
  Phase 1 (extract):     23/30 genres complete, 7 remaining
  Phase 2 (synthesize):  3/6 clusters complete, 3 remaining
  Phase 3 (elicit):      blocked — awaiting Phase 2 completion + review gate
  Phase 4 (elaborate):   blocked — awaiting Phase 3 completion

  Next action: narrative-data discover extract --type archetypes
               (will process 7 remaining genres)
```

**`resume`** picks up where the last run left off — reads the log, determines incomplete work, and runs exactly those cells. Combines pipeline-level progress (what phases are done) with manifest-level staleness (is the file still valid).

**`approve`** records a review gate event. Phase transitions that require human review (Phase 2 → 3, Phase 3 → 4) are blocked until the corresponding gate event exists.

### Review Gate Enforcement

The pipeline enforces human checkpoints between phases:

- **Phase 2 → Phase 3**: Phase 3 elicitation for a type will not start until a `review_gate` event with `decision: "approved"` exists for that type at Phase 2. The approved `primitives` list in the gate event defines what gets elicited in Phase 3.
- **Phase 3 → Phase 4**: Phase 4 elaboration will not start until Phase 3 is complete for the type. No mandatory review gate between Phase 3 and Phase 4 — Phase 3 completion alone unblocks Phase 4. Phase 4 selectivity (which genre × primitive pairs to elaborate) is derived from Phase 2 cluster synthesis data (which genres flagged each primitive as relevant). A human can optionally narrow or expand the selectivity before starting Phase 4.

### Interaction with Per-Directory Manifests

The per-directory manifests (`genres/manifest.json`, `archetypes/manifest.json`, etc.) and the pipeline JSONL serve complementary purposes:

| Concern | Manifest | Pipeline JSONL |
|---------|----------|---------------|
| Scope | Per-file staleness | Per-phase orchestration |
| Granularity | Individual cell (prompt hash, content digest) | Phase × type × target |
| Question answered | "Is this specific file up to date?" | "Where are we in the overall process?" |
| Mutation model | Read-modify-write JSON | Append-only events |

A `resume` command checks both: the pipeline log says Phase 1 extraction for folk-horror is done, and the manifest confirms the file exists with the expected digest. If a file is manually deleted, manifest staleness catches it even though the pipeline log says "completed." If a prompt template changes, the manifest hash detects staleness and the cell is re-run.

### State Derivation

Current pipeline state is computed by replaying the JSONL:

1. Collect all `*_completed` events per (phase, type, target)
2. Collect all `review_gate` events per (phase, type)
3. For each type, determine the current phase boundary:
   - Phase 1 complete = all 30 genres have `extract_completed` events
   - Phase 2 complete = all 6 clusters have `synthesize_completed` events
   - Phase 2 gate = `review_gate` event exists with `decision: "approved"`
   - Phase 3 complete = all primitives from the gate's `primitives` list have `elicit_completed` events
   - Phase 4 progress = count of `elaborate_completed` events against the selectivity target
4. Cross-reference with manifests to detect file-level staleness (prompt changes, deleted files)

---

## Prompt Design

### Prompt Size Budget

| Phase | Context source | Est. size | Generation time |
|-------|---------------|-----------|----------------|
| Phase 1 | One genre region.raw.md | ~16-17KB | ~5 min |
| Phase 2 | 4-8 extraction files | ~35-65KB | ~8 min |
| Phase 3 | Primitive name + synthesis description | ~3-4KB | ~5 min |
| Phase 4a | Genre description + Layer 0 primitive | ~25-30KB | ~5 min |
| Phase 4b | Genre description only | ~16-17KB | ~5 min |

No prompt exceeds 65KB. Compare to the old derivative pipeline at ~450KB.

### Prompt Principles

1. **Each prompt gets exactly the context it needs.** No descriptor dumps.
2. **Rich markdown output with `_commentary` and `_suggestions`.** Don't compress signal early — structured JSON extraction is optional and downstream.
3. **Focus on delta at each layer.** Phase 4 says "what changes" not "describe from scratch."
4. **Discovery prompts ask for overlap signals.** This makes Phase 2 deduplication tractable.

### Phase 1 Template Structure (Per-Genre Extraction)

For each primitive type, the extraction prompt provides the genre region description and asks for 5-8 candidates with:
- Name and description
- Why this genre gives rise to this primitive (grounded in axes/affordances)
- Distinguishing tension or axis position
- Overlap signal with other genres

### Phase 2 Template Structure (Cluster Synthesis)

Concatenates all per-genre extractions for a cluster and asks for:
- Canonical names for ~8-12 distinct primitives
- Core identity (genre-agnostic)
- Genre variant notes (axis shifts per genre)
- Uniqueness assessment (one genre, cluster-wide, universal)

### Phase 3 Template Structure (Layer 0 Elicitation)

Type-specific dimensional analysis prompts. For archetypes: personality axes, behavioral patterns, narrative roles, relationship tendencies, narrative promise. For dynamics: role definitions, power asymmetry, evolution patterns, scene affordances. Each adapted to the primitive type's analytical needs.

Includes `_commentary` and `_suggestions` output sections.

### Phase 4a Template Structure (Genre × Primitive Elaboration)

Provides both the genre description and the Layer 0 primitive description. Asks specifically for:
- Axis shifts under this genre's constraints
- Affordance effects (locus of power, temporal orientation, state variables)
- Excluded expressions
- Genre-specific narrative function

### Phase 4b Template Structure (Genre-Native)

Existing tropes.md and narrative-shapes.md templates, refactored to remove descriptor injection. Context is the genre region description only.

---

## Execution Strategy

### Phase Ordering

```
Phase 1 (extract)  →  Phase 2 (synthesize)  →  Phase 3 (Layer 0)  →  Phase 4 (Layer 1)
   30 calls/type         6 calls/type            ~25-30/type           selective
   per-genre             per-cluster             per-primitive         per genre×primitive
```

Phases 1-3 are independent across primitive types. Phase 4 depends on Phase 3 for the same type.

### Primitive Type Order (Revised Mar 20 2026)

The execution order was revised based on two insights from the data generation work:

1. **Settings and ontological posture before dynamics**: spatial/dramatic ground and the genre's stance toward personhood should inform interpersonal dynamics, not the other way around.
2. **Tropes and narrative-shapes before connector profiles**: key-moment scene profiles (completed) revealed a gap — the scenes *between* scenes that provide narrative breathing room. These connector profiles need tropes (genre idioms) and narrative beats (pacing patterns) as context to be properly elicited.

**Completed:**
1. ~~**Archetypes**~~ — Phase 1+2 complete (197 per-genre, 60 cluster-level)
2. ~~**Settings**~~ — Phase 1+2 complete (211 per-genre, cluster-level)
3. ~~**Ontological posture**~~ — Phase 1+2 complete (30 per-genre, cluster-level). New primitive added during execution — genre's stance toward personhood, otherness, self/Other boundary.
4. ~~**Profiles (key moments)**~~ — Phase 1+2 complete. Genre-essential dramatic situations.

**Next (genre-native, no Layer 0):**
5. **Tropes** — genre-native. Genre idioms, narrative devices, audience expectations. Dependency for connector profiles.
6. **Narrative-shapes** — genre-native. Pacing patterns, tension arcs, the rhythm of rest between peaks. Dependency for connector profiles.

**Then (informed by tropes + shapes):**
7. **Profiles (connectors)** — second-pass extraction. Scenes between scenes: approach beats, recovery beats, transit beats, ambient beats. Informed by tropes (what the genre's idioms expect between peaks) and narrative-shapes (where the genre breathes).
8. **Dynamics** — interpersonal patterns. Enriched with ontological posture lens (dynamics between humans and nonhuman agents).
9. **Goals** — character motivations. Last, informed by the full landscape.

Phase 3 (standalone elicitation) was intentionally skipped for all types — Phase 1+2 are the primary value generators. The per-genre extractions are richer than distilled standalone descriptions; cluster syntheses provide organizational indexing.

### Batch Execution

Same thermal management as genre regions (qwen3.5:35b saturates machine resources):
- Batches of 5-6 with 30-second cooldown between batches
- `run-batches.sh` pattern adapted per phase

### Human Review Gates

1. **After Phase 1 for archetypes** — Are extractions grounded in genre axes, or just listing familiar names?
2. **After Phase 2 for archetypes** — Is the synthesized list coherent? ~25-30 distinct archetypes? Good dedup?
3. **After Phase 3 for first ~5 archetypes** — Rich and dimensional? Comparable quality to genre regions?
4. **After Phase 4 for a test set** — Does elaboration produce useful delta over standalone?

### Structured JSON Extraction Points

Not required at any phase to proceed. Candidate points:
- **After Phase 3** — compact Layer 0 descriptions for Phase 4 context (reduces ~10-15KB markdown to ~2-4KB JSON)
- **After Phase 4** — narrator reference library in structured form for runtime lookup

Decision made at each review gate based on observed output quality.

### Compute Budget

| Phase | Calls/type | Types | Total | Est. time/call | Est. total |
|-------|-----------|-------|-------|----------------|------------|
| Phase 1 | 30 | 5 | 150 | ~5 min | ~12.5 hrs |
| Phase 2 | 6 | 5 | 30 | ~8 min | ~4 hrs |
| Phase 3 | ~28 avg | 5 | ~140 | ~5 min | ~11.5 hrs |
| Phase 4 (selective) | ~200 est | — | ~200 | ~5 min | ~16.5 hrs |
| Tropes/shapes | 30 each | 2 | 60 | ~5 min | ~5 hrs |
| **Total** | | | **~580** | | **~49.5 hrs** |

Phase 4 estimate assumes ~40 relevant genre-primitive pairs per type based on cluster synthesis selectivity (roughly 8-10 genres per primitive out of 30, across ~25 primitives per type, with significant overlap).

Roughly a week of background compute spread across multiple sessions. Each phase produces reviewable artifacts before the next begins.

---

## Relationship to Existing Work

### What This Replaces

The derivative-category approach where `genre elicit --categories archetypes` dumped all descriptor JSON into the prompt and produced genre-scoped archetypes directly. That path produced excellent output quality (the folk-horror/mentor result is genuinely rich) but was architecturally wrong: it skipped the foundational layer and had unsustainable prompt sizes.

### What This Preserves

- All 30 genre region descriptions (Layer 0 for genres, complete)
- The `narrative-data` CLI structure and generic pipeline infrastructure
- The two-stage architecture (35B elicitation → 7B structuring)
- File versioning, manifests, hash-based staleness detection
- The enriched prompt design philosophy (Genre Integration section concepts feed into Phase 4 prompts)

### What This Defers

- Pydantic schema updates — Layer 0 and Layer 1 outputs are exploratory markdown first. Schemas evolve once data shape stabilizes.
- Spatial pipeline (B.2) — settings as a primitive type slots into this architecture. The 22 setting types from the design spec serve as a reference, not a constraint.
- Cross-pollination (B.3) — deferred until both genre and primitive data exist.

---

## Open Questions

1. **Modifier regions in cluster synthesis (decision required before Phase 2).** Solarpunk, historical-fiction, literary-fiction, and magical-realism self-identified as modifiers. Should they be included in their natural genre cluster for Phase 2, handled as a separate "modifiers" cluster, or excluded from primitive discovery (since they modify rather than define)? This must be resolved during implementation planning — it blocks Phase 2 cluster synthesis.

2. **Phase 4 selectivity criteria.** Phase 2 cluster synthesis identifies which primitives are relevant to which genres. What threshold determines "relevant enough to elaborate"? Options: (a) only elaborate where Phase 2 flagged a genre-specific variant, (b) elaborate for all genres in the primitive's home cluster + any cross-cluster mentions, (c) human decides per primitive.

3. **Discovery prompt tuning.** The Phase 1 prompt asks for 5-8 candidates per genre. Too few risks missing important archetypes; too many produces noise. Worth calibrating on the first few genres before committing to a full run.
