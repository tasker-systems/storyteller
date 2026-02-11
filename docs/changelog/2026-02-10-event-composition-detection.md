# Turn-Level Event Composition Detection — Phase E

**Date**: 2026-02-10
**Phase**: Phase E — Turn-Level Event Composition Detection
**Branch**: `jcoletaylor/tas-240-phase-e-turn-level-event-composition-detection`
**Linear**: TAS-240

## What Happened

We implemented the bridge from flat classifier output to the typed event grammar, then built composition detection on top. Committed turns now produce structured `EventAtom`s (from D.3's `ClassificationOutput`) and `CompoundEvent`s (from sequential atom analysis). This completes the event extraction side of the turn commitment pipeline.

## The Bridge Problem

Phase D.3 committed-turn classification produces `ClassificationOutput` — a `Vec<(String, f32)>` of event kind labels with confidence scores, plus `Vec<ExtractedEntity>` NER mentions. The event grammar types (`EventAtom`, `CompoundEvent`) are structurally richer: they carry typed `EventKind` variants, `Participant`s with roles, `RelationalImplication`s, confidence provenance, and scene/turn context.

Phase E bridges this gap. The bridge:
1. Maps classifier label strings → `EventKind` variants with default sub-type values (intentionally naive — classifier can't distinguish e.g. `ActionType::Examine` vs `Perform`)
2. Converts NER `ExtractedEntity` → `(EntityRef::Unresolved, EntityCategory)` for role assignment
3. Calls existing `assign_participant_roles()` and `infer_implications_heuristic()` from `implication.rs`
4. Constructs `EventAtom`s with `TurnExtraction` source provenance

## Composition Detection

Two composition types, run in priority order:

**Causal** (detected first): Ordered atom pairs matching a pattern library AND sharing at least one Actor/Target participant. Five initial patterns:
- ActionOccurrence → EmotionalExpression ("action triggered emotional response")
- SpeechAct → RelationalShift ("speech shifted relationship")
- ActionOccurrence → SpatialChange ("action caused movement")
- InformationTransfer → EmotionalExpression ("revelation triggered emotional response")
- SpeechAct → ActionOccurrence ("speech prompted action")

**Temporal** (detected second, excluding causal-consumed pairs): Atoms sharing Actor/Target participants within the same turn. No causal ordering required.

Emergent relational implications are computed for all compounds: causal compounds get a 1.5x weight multiplier on constituent implications; temporal compounds get 1.0x (no amplification). Deduplication by (source, target) keeps max weight, clamped to [0.0, 1.0].

## Integration Point

Composition runs within `commit_previous_system` (the CommittingPrevious stage), after D.3 classification and before archiving the `CompletedTurn`. No new `TurnCycleStage` variant — this is part of the commitment flow, not a separate pipeline stage.

Placeholder `SceneId::new()` and `TurnId::new()` are used since the persistence layer isn't wired yet. These will be replaced when real IDs come from the session/scene lifecycle.

## What's Not Yet Realized

The atoms and compounds are captured on `CompletedTurn` but **nothing reads them downstream yet**. The value chain requires:

1. **Event Ledger** (Milestone 3) — durable storage for atoms
2. **AGE Graph** (Milestone 4) — where `RelationalImplication`s would update substrate dimensions
3. **Entity Promotion** — `RelationalWeight` computation exists but isn't called on committed atoms
4. **Graph Queries & Retrieval** (Milestone 7) — Tier 3 context enriched by accumulated relational data

This is the same "capture now, consume later" pattern as `predictions` and `committed_classification` on `CompletedTurn`. The typed pipeline is tested and correct; when persistence and the graph arrive, integration connects existing outputs to new inputs.

## What Was Changed

### storyteller-engine

- **`context/event_composition.rs`** (NEW) — Bridge functions (`label_to_event_kind`, `extracted_to_participant_input`, `build_event_atoms`), composition detection (`detect_causal_compositions`, `detect_temporal_compositions`, `compute_emergent_implications`, `detect_compositions`), causal pattern library, 23 tests.
- **`context/mod.rs`** — Added `pub mod event_composition;`.
- **`components/turn.rs`** — Added `committed_atoms: Vec<EventAtom>` and `committed_compounds: Vec<CompoundEvent>` to `CompletedTurn`.
- **`systems/turn_cycle.rs`** — Wired Phase E into `commit_previous_system` after D.3 classification. Updated `CompletedTurn` construction and debug logging.

## Design Decisions

1. **No new TurnCycleStage** — composition is part of commitment, not a separate stage.
2. **Causal patterns require participant overlap** — prevents false positives from unrelated events in the same turn.
3. **Causal preferred over temporal** — when atoms match both, causal wins (higher narrative significance).
4. **Default sub-type values** — classifier outputs "ActionOccurrence" but can't distinguish sub-types. Intentionally naive.
5. **Composition depth = 1** — compounds contain only atoms, never other compounds. Recursive composition is deferred.
6. **Reuses existing functions** — `assign_participant_roles`, `infer_implications_heuristic`, and `entity_matches` are all called from their existing locations in core, not reimplemented.

## Test Counts

165 core + 155 engine + 74 ML = **394 total Rust tests**, all passing. Phase E added 23 new tests in `event_composition.rs`.
