# Tier B Narrative Data — Session Handoff

**Date:** 2026-03-18
**Branch:** `jcoletaylor/narrative-data-exploratory-research`
**Status:** Active generation, tooling complete, genre regions done, derivatives and spatial pending

---

## What Was Built This Session

### `tools/narrative-data/` Python package
- 82 tests, full CLI (`narrative-data genre elicit/structure`, `spatial elicit/structure`, `cross-pollinate`, `status`, `list`)
- Two-stage pipeline: qwen3.5:35b (rich elicitation → raw.md) + qwen2.5:7b-instruct (structuring → .json, deferred)
- Pydantic schemas for all entity types (code reflects original spec; spec has been updated ahead of code — deliberate)
- File versioning: re-runs archive to `.v{N}.md`
- 12 prompt templates (8 genre + 3 spatial + 1 cross-pollination), all genre prompts enriched with Genre Integration section

### Genre region elicitation — 2 passes complete
- **Pass 1** (25 regions, original 5-dimensional prompt): ~290KB. Archived as `.v1.md`.
- **Pass 2** (30 regions, enriched prompt with temporal/agency/locus-of-power/epistemological + exclusions/state-variables/topology/boundary-conditions): ~438KB. Current files.
- **5 new regions from dimensional gap analysis**: quiet-contemplative-fantasy, domestic-noir, horror-comedy, working-class-realism, classical-tragedy
- **4 modifier regions**: solarpunk, historical-fiction, literary-fiction, magical-realism
- **Cross-genre analysis**: `storyteller-data/narrative-data/genres/ANALYSIS-pass-1.md`

---

## What Needs to Happen Next (Ordered)

### 1. Derivative genre categories — test run
Run archetypes and tropes for 2-3 test genres (suggest: folk-horror, cozy-fantasy, domestic-noir — diverse cluster).
```bash
cd tools/narrative-data
STORYTELLER_DATA_PATH=../storyteller-data narrative-data genre elicit --regions folk-horror,cozy-fantasy,domestic-noir --categories archetypes,tropes
```
Review quality. The prompts now include a Genre Integration section that asks the model to connect archetypes/tropes to state variables, locus of power, temporal orientation, and exclusions. This is the first test of whether the enriched region data improves derivative output.

### 2. Spatial setting elicitation
22 settings defined. Run `setting-type` category first (same pattern as genre regions — foundation before derivatives).
```bash
STORYTELLER_DATA_PATH=../storyteller-data narrative-data spatial elicit --categories setting-type
```
Consider batching (machine gets sluggish with 35B model running continuously). The `run-batches.sh` pattern could be adapted.

### 3. Pass-2 cross-genre analysis
The enriched data likely reveals patterns the pass-1 analysis didn't catch (state variable commonalities, topology relationships, boundary condition symmetries). Worth running the same analysis pattern against the pass-2 outputs.

### 4. Derivative genre categories — full run
Once test quality is validated, run remaining categories (narrative-shapes, dynamics, profiles, goals, settings) across all 30 genres. This is the combinatorial explosion — ~210 cells (30 × 7 categories). Will take significant compute time.

### 5. Schema stabilization
When the data shape stops producing new axis suggestions at a meaningful rate:
- Update Pydantic models in `schemas/genre.py` to match spec
- Add StateVariable, BoundaryCondition, GenreTopology, EpistemologicalStance types
- Attempt Stage 2 structuring with qwen2.5:7b-instruct
- Iterate on Stage 2 prompt to handle commentary/suggestions mapping and entity_id assignment

### 6. Cross-pollination (B.3)
Requires both genre and spatial data. The `cross-pollinate` command is stubbed. Design the synthesis prompt once we have spatial data to cross against.

---

## Technical Notes for Next Session

- **STORYTELLER_DATA_PATH** must be set (not in shell env by default; use `export` or prefix commands)
- **Descriptor context injection**: Only injected for derivative categories, NOT for region. See `genre/commands.py` line 136.
- **Subagents can't read `storyteller-data/`** — it's outside the working directory. Use the Explore agent type or read files directly in main session for analysis.
- **Machine performance**: The 35B model saturates the machine. Use batches of 5 with cooldown. `run-batches.sh` is a template.
- **Manifest staleness**: The CLI skips cells where prompt hash hasn't changed. Use `--force` to re-run anyway. Prompt changes automatically trigger re-generation.
- **The spec has evolved ahead of the code** — the GenreRegion schema in the spec includes fields (temporal, agency, etc.) that don't exist in Pydantic yet. This is deliberate per the schema evolution strategy.
