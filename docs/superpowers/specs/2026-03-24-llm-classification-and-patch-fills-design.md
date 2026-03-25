# LLM Classification and Patch Fills — Design Specification

**Date**: 2026-03-24
**Ticket**: `2026-03-24-llm-classification-and-patch-fills`
**Branch**: `jcoletaylor/llm-classification-and-patch-fills`
**Scope**: Feature

## Summary

Four workstreams that enrich the ground-state narrative corpus using existing infrastructure with minimal new tooling. The unifying constraint is **delta in conceptual set** — each workstream reuses the Ollama client, `fill_all_llm_patch` orchestrator, shell script batch pattern, and loader upsert pipeline. New code is limited to extraction functions and normalization logic.

## Workstreams

### A — Dynamics LLM Patch Fill Execution

**Goal**: Fill `valence`, `currencies`, and `scale_manifestations` across all 30 genres (~873 Ollama calls).

**Existing infrastructure** (no code changes unless prompt quality requires iteration):
- `pipeline/llm_patch.py`: `extract_valence()`, `extract_currencies()`, `extract_scale_manifestations()`, `fill_all_llm_patch()`
- `ollama.py`: `OllamaClient.generate()` and `generate_structured()`
- Config: `STRUCTURING_MODEL = "qwen2.5:7b-instruct"`, `STRUCTURING_TIMEOUT = 120.0`
- CLI: `narrative-data fill --tier llm-patch --type dynamics`
- Tests: `test_llm_patch.py` (442 lines, 38 test methods)

**Execution workflow**:
1. Smoke test: `narrative-data fill --tier llm-patch --type dynamics --genre folk-horror`
2. Review diff in `storyteller-data/narrative-data/genres/folk-horror/dynamics.json`
3. Iterate on prompts if quality is off
4. Shell script: 6 batches of 5 genres, `sleep 30` between batches, `set -euo pipefail`
5. Background execution while working on other workstreams
6. Reload: `narrative-data load-ground-state`
7. Verify: `narrative-data audit --type dynamics` to confirm null rate reduction

**Shell script pattern** (matches existing `run-discovery-extract.sh`):
```bash
#!/usr/bin/env bash
set -euo pipefail

BATCH1="folk-horror,cosmic-horror,horror-comedy,high-epic-fantasy,dark-fantasy"
BATCH2="cozy-fantasy,fairy-tale-mythic,urban-fantasy,quiet-contemplative-fantasy,hard-sci-fi"
BATCH3="space-opera,cyberpunk,solarpunk,dystopian,post-apocalyptic"
BATCH4="cozy-mystery,noir,psychological-thriller,spy-thriller,detective-procedural"
BATCH5="contemporary-romance,historical-romance,paranormal-romance,romantasy,literary-fiction"
BATCH6="historical-fiction,southern-gothic,magical-realism,afrofuturism,classical-tragedy,pastoral"

for i in 1 2 3 4 5 6; do
    BATCH_VAR="BATCH$i"
    GENRES="${!BATCH_VAR}"
    echo "=== Batch $i: $GENRES ==="
    uv run narrative-data fill --tier llm-patch --type dynamics --genre "$GENRES"
    if [ "$i" -lt 6 ]; then
        sleep 30
    fi
done
```

**Skip guards**: Each extraction function checks if the field is already populated before calling Ollama. Re-running the script after partial completion resumes where it left off.

### B — Trope Family Consolidation

**Goal**: Replace the colon-splitting heuristic in `normalize_family_name()` with dimension keyword extraction. Collapse 115 families → ~11 canonical dimension-based families.

**Problem analysis** (from corpus investigation):
- 248 raw `genre_derivation` values across 30 genres
- 108 derivations > 100 chars → "unclassified" by current heuristic
- 95% of all derivations contain a dimension keyword (aesthetic, tonal, temporal, thematic, agency, epistemological, structural, world-affordance, locus-of-power, ontological, state-variable)
- 12 derivations match no dimension keyword; all map to `thematic-dimension`, `structural-dimension`, `world-affordance`, or `locus-of-power` via secondary keywords

**Canonical families** (11):
| Slug | Keyword Match |
|------|---------------|
| `aesthetic-dimension` | "aesthetic" |
| `tonal-dimension` | "tonal" |
| `temporal-dimension` | "temporal" |
| `thematic-dimension` | "thematic" |
| `agency-dimension` | "agency" |
| `epistemological-stance` | "epistemological" |
| `structural-dimension` | "structural" |
| `world-affordance` | "world affordance", "world-affordance" |
| `locus-of-power` | "locus of power", "locus-of-power" |
| `ontological-posture` | "ontological" |
| `state-variable` | "state variable", "state-variable" |

Plus a `genre-specific` fallback (expected to be empty after secondary keywords, kept as safety net).

**Secondary keyword table** (for the 12 unmatched derivations):
```python
_SECONDARY_KEYWORDS = {
    "identity": "thematic-dimension",
    "belonging": "thematic-dimension",
    "social capital": "thematic-dimension",
    "collective": "thematic-dimension",
    "solidarity": "thematic-dimension",
    "materiality": "thematic-dimension",
    "memory": "thematic-dimension",
    "mystery": "structural-dimension",
    "tragedy": "structural-dimension",
    "magic": "world-affordance",
    "medical": "world-affordance",
    "violence": "world-affordance",
    "antagonistic": "locus-of-power",
    "power": "locus-of-power",
}
```

**Implementation** (in `trope_families.py`):
1. Replace `normalize_family_name()` body:
   - Remove colon-split, plural-collapse logic
   - Add ordered `_DIMENSION_KEYWORDS` dict scan (most specific first to avoid false matches)
   - Fall through to `_SECONDARY_KEYWORDS` scan
   - Default to `"genre-specific"` if no match
   - Keep `_MAX_FAMILY_LENGTH` guard (still useful for garbage input)
2. Update `_slug_to_display_name()` if needed (current implementation works for new slugs)
3. Update tests in `test_loader.py` (`TestTropeFamilyLoading`) to assert new family slugs
4. Re-run loader to update `trope_families` table and `tropes.trope_family_id` FKs
5. Verify via `genre_context()` that family_slug/family_name are correct in query output

**No parenthetical capture**: The raw `genre_derivation` field on each trope entity preserves the full prose context. Family slug is the queryable dimension reference.

### C — State Variable Normalization (Research + Deterministic Triage)

**Goal**: Triage 319 state variable reference mismatches. Fix mechanical issues, surface semantic gaps.

**Problem analysis**:
- 164 canonical state variables in `state_variables` table (extracted from `region.json` `active_state_variables`)
- 483 unique variable references in entity payloads (`state_variable_interactions[].variable_id` etc.)
- 319 references fail lookup in `_load_state_variable_interactions()` → silently skipped with warning log
- Mismatch types: title case ("Community Trust"), underscores ("moral_stance"), parenthetical suffixes ("Knowledge (Secrets Known)"), truncation ("sanctuary" vs "sanctuary-integrity"), semantic drift ("Moral Standing" vs "moral-stance")

**Three-phase approach**:

**Phase 1 — Deterministic normalizer** (`sv_normalization.py`):
```python
def normalize_sv_slug(raw: str) -> str:
    text = raw.strip().lower()
    text = re.sub(r"\s*\(.*?\)\s*", "", text)  # strip parentheticals
    text = text.replace("_", "-").replace(" ", "-")
    text = re.sub(r"-+", "-", text).strip("-")
    return text
```

**Phase 2 — Fuzzy matching** against canonical slugs:
- After normalization, exact match first
- If no exact match, prefix match (e.g., "sanctuary" → "sanctuary-integrity")
- If no prefix match, log as unresolved

**Phase 3 — Assessment report** via `narrative-data sv-audit`:
- Count: resolved via exact match after normalization
- Count: resolved via prefix/fuzzy match
- Count: unresolved (with full list of raw values and their normalized forms)
- Decision gate: if unresolved < ~30, hand-build mapping table; if larger, document for next session

**Integration**: Wire `normalize_sv_slug()` into `_load_state_variable_interactions()` as a pre-lookup step:
```python
variable_slug = normalize_sv_slug(ix["variable_id"])
sv_id = sv_map.get(variable_slug)
```

**New file**: `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py`
**New CLI command**: `narrative-data sv-audit` (prints triage report)
**No schema changes**: Resolution happens at the Python layer between payload data and canonical table.

### D — Extending LLM Fills to Spatial-Topology and Ontological-Posture

**Goal**: Fill high-value extended fields that are 100% null but present in source markdown.

**spatial-topology** (3 fields):
| Field | Type | Source Presence | Extraction Strategy |
|-------|------|-----------------|---------------------|
| `state_shift` | `str \| None` | Described in ontological dimension commentary | Focused prompt targeting transition effects |
| `directionality.description` | `str \| None` | Explicitly described per transition | Section-based extraction |
| `friction.description` | `str \| None` | Explicitly described per transition | Section-based extraction |

**ontological-posture** (2 fields):
| Field | Type | Source Presence | Extraction Strategy |
|-------|------|-----------------|---------------------|
| `crossing_rules` | `str \| None` | Explicit section in source markdown | Section heading detection + extraction |
| `obligations_across` | `str \| None` | Explicit section "Obligations Across the Boundary" | Section heading detection + extraction |

**Implementation** (extends `llm_patch.py`):
1. Add `"spatial-topology"` and `"ontological-posture"` to `supported_patch_types`
2. Implement extraction functions per field:
   - `extract_state_shift(entity, md_content, client) -> str | None`
   - `extract_directionality_description(entity, md_content, client) -> str | None`
   - `extract_friction_description(entity, md_content, client) -> str | None`
   - `extract_crossing_rules(entity, md_content, client) -> str | None`
   - `extract_obligations_across(entity, md_content, client) -> str | None`
3. Each function follows the dynamics pattern: skip if already populated, extract relevant markdown section, focused prompt, validate output
4. Wire into `fill_all_llm_patch()` entity processing loop with type dispatch
5. Add unit tests following `test_llm_patch.py` patterns (mock Ollama, test skip logic, test prompt content)

**Execution** (same workflow as A):
1. Smoke test one genre per type
2. Review output quality
3. Shell scripts for full corpus (separate scripts per type for independent re-runs)
4. Background execution
5. Final reload

**Companion `.md` file discovery**: spatial-topology source files are at `genres/{genre}/spatial-topology.md`; ontological-posture at `genres/{genre}/ontological-posture.md`. The existing `fill_all_llm_patch()` already handles this path pattern for genre-native types.

**Decision gate**: If smoke test reveals the source markdown doesn't contain extractable content for a field (information genuinely absent, not just unextracted), we skip that field and document the finding.

## Execution Order

```
B (trope families)  →  A (dynamics fills)  →  C (SV audit)  →  D (spatial/ontological fills)  →  final reload
         │                    │                     │
     pure Python         background run        overlaps with A
     fast, no LLM        while C runs          read-only analysis
```

**Dependencies**:
- B before any reload (fixes family table)
- A can run in background while C executes
- D depends on A's pattern validation (prompt quality confirmed)
- D depends on C's triage (SV normalization wired in before new types loaded)
- Single final reload after D, or intermediate reload after A to validate

## Files Changed

**Modified**:
- `tools/narrative-data/src/narrative_data/persistence/trope_families.py` — rewrite normalization logic (B)
- `tools/narrative-data/src/narrative_data/pipeline/llm_patch.py` — add spatial-topology + ontological-posture support (D)
- `tools/narrative-data/src/narrative_data/persistence/loader.py` — wire SV normalization into interaction loading (C)
- `tools/narrative-data/src/narrative_data/cli.py` — add `sv-audit` command (C)
- `tools/narrative-data/tests/test_loader.py` — update trope family assertions (B)
- `tools/narrative-data/tests/test_llm_patch.py` — add spatial-topology + ontological-posture tests (D)

**New**:
- `tools/narrative-data/src/narrative_data/persistence/sv_normalization.py` — normalizer + fuzzy matcher + audit report (C)
- `tools/narrative-data/tests/test_sv_normalization.py` — unit tests for normalizer (C)
- `tools/narrative-data/scripts/run-dynamics-patch-fill.sh` — batch script for A
- `tools/narrative-data/scripts/run-spatial-topology-patch-fill.sh` — batch script for D
- `tools/narrative-data/scripts/run-ontological-posture-patch-fill.sh` — batch script for D

**No changes**:
- SQL migrations (all columns/tables exist)
- Pydantic schemas (all fields defined)
- `ollama.py` (client unchanged)
- `reference_data.py` (canonical extraction unchanged)

## Verification

- `uv run pytest tools/narrative-data/tests/` — all tests pass after each workstream
- `uv run ruff check tools/narrative-data/` — no lint violations
- `narrative-data audit --type dynamics` — null rate reduction on valence/currencies/scale_manifestations
- `narrative-data sv-audit` — triage report showing resolution rates
- `narrative-data load-ground-state` — clean load with updated enrichments
- `genre_context('folk-horror')` — spot-check that trope families, SV interactions, and new fields are present
