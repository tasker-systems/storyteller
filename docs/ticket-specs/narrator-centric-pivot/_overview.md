# Narrator-Centric Architecture Pivot

**Branch**: `jcoletaylor/agent-prototyping`
**Started**: February 7, 2026
**Design document**: [`docs/technical/narrator-architecture.md`](../../technical/narrator-architecture.md)

## Motivation

The first playable scene ("The Flute Kept," Feb 7 2026) proved the multi-agent pipeline shape correct but revealed that 3+ LLM calls per turn is computationally prohibitive — minutes between turns on M4/64GB hardware. The architectural pivot collapses all LLM calls to a single Narrator agent, replacing Character Agent LLMs with ML prediction models and the Reconciler/World Agent with a deterministic rules engine (Resolver).

The design preserves everything that worked: information boundaries, character tensors, the scene model, narrative gravity. What changes is who does what — ML models predict, rules engines resolve, graph queries retrieve, and the LLM tells the story.

## Phase Map

```
Phase 0 (validate prediction) ─── decision gate ───→ Phase 3 (ML pipeline)
                                                          │
Phase 1 (core types) ──→ Phase 2 (context assembly) ─────┤
                    │                                     │
                    └──→ Phase 4 (resolver) ──────────────┤
                                                          │
                         Phase 5 (classifier) ────────────┤
                                                          ▼
                                                    Phase 6 (integration)
```

| Phase | Name | Status | Spec |
|-------|------|--------|------|
| 1 | Core Type Restructuring | **Complete** | [phase-1-core-types.md](phase-1-core-types.md) |
| 2 | Narrator Context Assembly | **Draft** | [phase-2-context-assembly.md](phase-2-context-assembly.md) |
| 0 | Validate Prediction Approach | Planned | — |
| 3 | Character Prediction Pipeline | Planned (blocked on Phase 0 gate) | — |
| 4 | Resolver (Rules Engine) | Planned | — |
| 5 | Event Classification | Planned | — |
| 6 | Integration and Evaluation | Planned (blocked on all prior) | — |

## Parallelism

- Phases 0 and 1 can run in parallel (Phase 1 complete)
- Phases 2 and 4 can run in parallel after Phase 1
- Phase 3 depends on Phase 0's decision gate
- Phase 5 can run independently after Phase 1
- Phase 6 requires all prior phases

## What the Codebase Gives Us

The existing code provides a strong foundation. The architectural pivot is surgical — most of the core types, the Narrator agent, the Ollama integration, the workshop data, and all Bevy components are fully valid. The work is additive (new type modules, new pipeline stages) with targeted refactoring of the turn cycle and message types.

See the Phase 1 spec for the detailed audit of what stays, what changes, and what becomes obsolete.

## Investigation Areas

These questions need answers before or during implementation. They are not blockers for Phase 1 or 2, but affect Phases 3-6.

1. **Character prediction model architecture** — MLP vs. small transformer vs. gradient-boosted trees on structured features. Affects Phase 3.
2. **Training data scale** — Is 2000-7500 examples enough? Phase 0 validates this.
3. **Scene journal compression calibration** — How aggressively to compress. Affects Phase 2 (testable with workshop data).
4. **GraphRAG for Tier 3** — How much is graph traversal vs. semantic search. Affects Phase 2.
5. **Echo detection feasibility** — Structured matching vs. semantic understanding. Affects Phase 5.
6. **Resolver depth** — How much to build before stress-test scenes exist. Affects Phase 4.
7. **Narrator model selection** — What model best serves structured-context-to-literary-prose. Affects Phase 2 (testable immediately).

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| Feb 7, 2026 | Single Narrator, ML prediction for characters | Multi-LLM prohibitively slow; LLMs are mediocre at persona maintenance, excellent at prose |
| Feb 7, 2026 | Deterministic Resolver replaces Reconciler + World Agent | World constraints are deterministic, not generative |
| Feb 7, 2026 | Three-tier context (preamble/journal/retrieved) | Finite context window requires structured assembly with different update frequencies |
| Feb 7, 2026 | `GameDesignSystem` trait for pluggable resolution | Genre-specific mechanics should be parameterized, not hardcoded |
| Feb 7, 2026 | Legacy types retained alongside new types | Existing prototype code path still works; migration is gradual |
| Feb 7, 2026 | `CapabilityProfile` added to `CharacterSheet` | Resolver needs attributes/skills; `#[serde(default)]` preserves backward compat |
| Feb 7, 2026 | `TurnPhaseKind` updated to narrator-centric pipeline | New phases (CharacterPrediction, Resolving, ContextAssembly, Rendering) reflect actual pipeline |
