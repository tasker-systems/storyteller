---
type: session
date: 2026-03-29
project: storyteller
ticket: 2026-03-29-tome-phase-3c-elicitation-prompt-decomposition-for-scale
---

# Session: Tome Phase 3c — Elicitation Prompt Decomposition for Scale

## Goal

Design and implement a decomposed elicitation pipeline that reduces world generation time from ~40 minutes to ~20 minutes while maintaining the axis-grounded output quality established in Phase 3b.

## What happened

### Discovery: The 36.5 KB elephant

Context audit revealed that the world preamble dominates every prompt at 36.5 KB — 60-75% of each call. The "justifications" are raw edge-traversal traces from the mutual production graph (`historical-memory-depth →enables→ trauma-transmission-mode (w=0.7); ...`), averaging 754 chars each across 44 inferred positions. The LLM uses axis values to ground output but doesn't need graph provenance.

### Design: Fan-out / Fan-in with model tiering

Brainstormed and designed a map-reduce architecture:
- **Fan-out**: Small model (7b/9b) generates individual entities in parallel with focused axis subsets
- **Fan-in**: 35b coherence pass per stage binds entities relationally
- **Compressed preamble**: Domain-grouped axes, values only, no edge traces (~4 KB)

Key design decisions:
- Per-stage coherence (not one big aggregation) — smaller calls, natural feed-forward
- One entity per fan-out call — maximizes granularity of reuse
- Agent-backed planning call determines entity counts (not deterministic Python)
- Q3-Q4 character coherence gets deep treatment with explicit design principle framing
- Per-file outputs serve downstream narrative-landscape generation

### Implementation: 11 tasks, 169 tests

Built the full infrastructure in 10 tasks:
1. Model registry + FanOutSpec dataclass
2. Preamble compression (36.5 KB → ~4 KB)
3. Entity planning call (7b)
4. Fan-out dispatch engine (ThreadPoolExecutor)
5. Coherence engine (35b per-stage)
6. Fan-out prompt templates (5)
7. Coherence prompt templates (5, including deep Q3-Q4 pass)
8. Pipeline orchestrator
9. CLI registration (`tome elicit-decomposed`)
10. Full test validation (169 tests passing)

### Ablation: Fan-out quality gap

Ran the fan-out pipeline on McCallister's Barn. The 7b-instruct produced:
- Valid JSON but **generic, repetitive, ungrounded** output
- "Weathered stone" and "dilapidated" everywhere
- Made-up grounding values ("weathering:high") instead of actual axis references
- No material differentiation between places

**Key finding**: Entity generation isn't a "simple structured output" task — it's a creative grounding task that needs the larger model's reasoning capacity. The decomposition assumed generation was simpler than relational binding, but producing a *good* entity grounded in specific world conditions is itself hard.

### Pivot: Compressed preamble + 35b unified generation

Pivoted to a simpler architecture that preserves the valuable infrastructure:
- **Compressed preamble** feeds into the existing Phase 3b prompt templates
- **Single 35b call per stage** (not fan-out/coherence split)
- **Post-generation parse** into per-entity instance files for future reuse
- CLI: `--mode compressed` (default) vs `--mode fanout` (original)

Validated on McCallister's Barn places stage:
- Prompt: **16 KB** (down from ~45 KB, 65% reduction)
- Output: 13 places, axis-grounded, materially distinct
- Quality: On par with Phase 3b baseline
- Baseline files auto-backed up to `*.baseline.json`

### Broader strategic discussion

The generation problem has a trajectory:
1. **Now**: Generate ~10 high-quality reference worlds with compressed preamble + 35b
2. **Near-term**: Extract structural patterns — lexicon sets per axis range, character permutation bounds, genre-specific distribution shapes
3. **Medium-term**: Those patterns become training signal — deterministic set-intersection logic or synthetic training data for a smaller model

The LLM-heavy generation is *prospecting* for structural regularities, not the permanent pipeline. Once we have 10 worlds with full axis-grounded treatment, we can extract the grammar: "when kinship-system is clan-tribal and resource-scarcity is high, character archetypes cluster around these shapes with these personality bounds."

## Decisions

1. **Fan-out with small models doesn't work for creative grounding.** The 7b-instruct can produce valid JSON but not world-specific content. Entity generation and axis reasoning are not separable at this model scale.

2. **Compressed preamble is the real win.** Dropping edge traces and grouping by domain gives 65% prompt reduction with no quality loss. This is pure Python with no model dependency.

3. **Per-file segmented output preserved regardless of generation mode.** Individual entity files (`instance-NNN.json`) are written after every stage for future reuse, selective regeneration, and downstream consumption.

4. **Baseline backup on overwrite.** Compressed pipeline backs up existing files to `*.baseline.json` before writing, preserving Phase 3b data for comparison.

5. **Fan-out infrastructure retained.** The `--mode fanout` path stays in the codebase. If better local models emerge (or cloud APIs become cost-effective), the architecture is ready.

## What connected

- The model tiering insight from our narrative-data genre segmentation work (generate-and-slice with smaller models, aggregate with larger) was the right *pattern* but applied at the wrong *task boundary*. Entity generation needs the full reasoning chain; entity *segmentation* after generation is where the small-model/Python split works.
- The preamble compression validates the Phase 2 graph design — the edge traces serve the propagation algorithm, not the downstream consumers. Clean separation of concerns.
- Per-file outputs align with the future narrative-landscape architecture — each downstream consumer pulls just the files it needs.

## To pick up

### Complete McCallister's Barn compressed run
- Run remaining stages: orgs → substrate → mundane chars → significant chars
- Compare timing against Phase 3b (~40 min target: under 20 min)
- Compare output quality across all stages

### Run ~10 reference worlds
- Use compressed pipeline across genre × setting combinations
- Build comparison corpus for pattern extraction

### Spatial-topology integration (deferred seam)
- Genre settings data has rich per-setting narrative functions
- The decomposed architecture makes feeding setting archetypes to place generation straightforward
- Separate ticket, not this one

### Pattern extraction from reference corpus
- Once 10 worlds exist: extract lexicon sets, personality bounds, distribution shapes
- Determine whether set-intersection logic or tensor modeling is the right formalization
- This is the path from "LLM prospecting" to "deterministic/trainable generation"

## Artifacts

- **Design spec**: `docs/superpowers/specs/2026-03-29-tome-phase-3c-elicitation-prompt-decomposition-for-scale-design.md`
- **Implementation plan**: `docs/superpowers/plans/2026-03-29-tome-phase-3c-elicitation-prompt-decomposition-for-scale.md`
- **New modules**: `compress_preamble.py`, `plan_entities.py`, `fan_out.py`, `cohere.py`, `orchestrate_decomposed.py`, `models.py`
- **New tests**: 6 test files, 169 total tests passing
- **New templates**: 11 prompt templates in `prompts/tome/decomposed/`
- **CLI**: `tome elicit-decomposed --mode compressed|fanout --stage <name> --coherence-only`
