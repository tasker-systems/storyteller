# TAS-240: Phase E — Turn-Level Event Composition Detection

Detect compound events from sequential EventAtoms within committed turns. Composition detection runs after D.3 committed-turn classification inside `commit_previous_system`, producing `CompoundEvent`s stored alongside atoms on `CompletedTurn`.

## Context

Phase E is the next step after D.1-D.4 in the event system foundations. The committed-turn classification (D.3) already runs in `commit_previous_system`, producing a `ClassificationOutput` with string event kind labels and NER entity mentions. Phase E bridges that output into structured `EventAtom`s, then detects temporal and causal compositions.

**Key gap**: `ClassificationOutput.event_kinds` is `Vec<(String, f32)>` (labels + confidence), not `Vec<EventAtom>`. Phase E includes the bridge function to construct atoms from classification output, using existing `assign_participant_roles()` and `infer_implications_heuristic()` from `implication.rs`.

**Integration point**: Composition runs within `commit_previous_system` (not a new pipeline stage), after D.3 classification and before archiving the `CompletedTurn`. No changes to `TurnCycleStage` enum.

## Step 1: Add fields to CompletedTurn

**File**: `storyteller-engine/src/components/turn.rs`

Add two fields to `CompletedTurn` (after `committed_classification`):

```rust
/// Event atoms built from committed-turn classification (Phase E).
pub committed_atoms: Vec<storyteller_core::types::event_grammar::EventAtom>,
/// Compound events detected from committed atoms (Phase E).
pub committed_compounds: Vec<storyteller_core::types::event_grammar::CompoundEvent>,
```

Update all construction sites to include `committed_atoms: vec![], committed_compounds: vec![]`:
- `turn_cycle.rs:350` (main construction in `commit_previous_system`)
- `turn.rs:250` (test: `completed_turn_captures_fields`)

## Step 2: Create event_composition.rs — helpers and bridge

**New file**: `storyteller-engine/src/context/event_composition.rs`

### 2a: Event kind discriminant + label mapping

```rust
/// Returns a &str discriminant for EventKind (matches EVENT_KIND_LABELS).
fn event_kind_discriminant(kind: &EventKind) -> &'static str

/// Convert classifier label string → EventKind with default sub-type values.
/// Unknown labels → StateAssertion fallback.
fn label_to_event_kind(label: &str) -> EventKind
```

Label mapping (8 labels from `storyteller_ml::event_labels::EVENT_KIND_LABELS`):
- `"StateAssertion"` → `EventKind::StateAssertion { assertion: String::new() }`
- `"ActionOccurrence"` → `EventKind::ActionOccurrence { action_type: ActionType::Perform }`
- `"SpatialChange"` → `EventKind::SpatialChange { from: None, to: None }`
- `"EmotionalExpression"` → `EventKind::EmotionalExpression { emotion_hint: None, intensity: 0.5 }`
- `"InformationTransfer"` → `EventKind::InformationTransfer { content_summary: String::new() }`
- `"SpeechAct"` → `EventKind::SpeechAct { register: SpeechRegister::Conversational }`
- `"RelationalShift"` → `EventKind::RelationalShift { dimension: String::new(), delta: 0.0 }`
- `"EnvironmentalChange"` → `EventKind::EnvironmentalChange { description: String::new() }`

### 2b: Entity conversion

```rust
/// Convert ExtractedEntity → (EntityRef::Unresolved, EntityCategory).
fn extracted_to_participant_input(
    entity: &ExtractedEntity,
) -> (EntityRef, EntityCategory)
```

Maps `NerCategory` → `EntityCategory`: Character→Character, Object→Object, Location→Location, all others→Other. Creates `EntityRef::Unresolved` with the entity mention text and minimal `ReferentialContext`.

### 2c: build_event_atoms (the bridge)

```rust
/// Convert ClassificationOutput → Vec<EventAtom>.
///
/// For each (label, confidence) above threshold:
/// 1. Map label → EventKind via label_to_event_kind()
/// 2. Convert entity_mentions → (EntityRef, EntityCategory) tuples
/// 3. Call assign_participant_roles() from implication.rs
/// 4. Call infer_implications_heuristic() with the kind and participants
/// 5. Construct EventAtom with TurnExtraction source
pub fn build_event_atoms(
    classification: &ClassificationOutput,
    scene_id: SceneId,
    turn_id: TurnId,
) -> Vec<EventAtom>
```

**Reuses existing functions**:
- `storyteller_core::types::implication::assign_participant_roles`
- `storyteller_core::types::implication::infer_implications_heuristic`

## Step 3: Temporal composition detection (E.1)

### 3a: Participant overlap check

```rust
/// Check whether two atoms share a participant in Actor or Target role.
/// Uses entity_matches() from promotion/weight.rs for entity comparison.
fn has_participant_overlap(a: &EventAtom, b: &EventAtom) -> bool
```

Reuses `storyteller_core::promotion::weight::entity_matches` for `EntityRef` comparison (handles Resolved UUID equality, Unresolved normalized mention matching, Implicit).

### 3b: Temporal detection

```rust
/// Detect temporal compositions: atoms within the same set that share
/// at least one Actor/Target participant.
///
/// excluded_pairs: atom ID pairs already consumed by causal detection.
pub fn detect_temporal_compositions(
    atoms: &[EventAtom],
    excluded_pairs: &[(EventId, EventId)],
) -> Vec<CompoundEvent>
```

For each pair `(i, j)` where `i < j`: check not excluded, check `has_participant_overlap`, if both → create `CompoundEvent { composition_type: Temporal, ... }`.

## Step 4: Causal composition detection (E.2)

### 4a: Causal pattern library (data-driven)

```rust
#[derive(Debug, Clone)]
pub struct CausalPattern {
    pub cause: &'static str,      // EventKind discriminant
    pub effect: &'static str,     // EventKind discriminant
    pub mechanism: &'static str,  // Human-readable description
}

fn causal_pattern_library() -> &'static [CausalPattern]
```

Initial 5 patterns from spec (using discriminant strings since classifier doesn't distinguish sub-types):
1. `ActionOccurrence → EmotionalExpression` ("action triggered emotional response")
2. `SpeechAct → RelationalShift` ("speech shifted relationship")
3. `ActionOccurrence → SpatialChange` ("action caused movement")
4. `InformationTransfer → EmotionalExpression` ("revelation triggered emotional response")
5. `SpeechAct → ActionOccurrence` ("speech prompted action")

### 4b: Causal detection

```rust
/// Detect causal compositions: ordered pairs matching known causal patterns
/// AND sharing participant overlap.
///
/// Returns (compounds, consumed_pairs) — consumed pairs excluded from temporal.
pub fn detect_causal_compositions(
    atoms: &[EventAtom],
) -> (Vec<CompoundEvent>, Vec<(EventId, EventId)>)
```

For each ordered pair `(i, j)` where `i < j`:
1. Check `event_kind_discriminant(i.kind)` matches a pattern's `cause` AND `event_kind_discriminant(j.kind)` matches its `effect`
2. Check `has_participant_overlap(i, j)`
3. Both → create `CompoundEvent { composition_type: Causal { mechanism }, ... }`
4. Track consumed pairs

## Step 5: Emergent weight calculation (E.3)

```rust
/// Compute emergent relational implications for a compound event.
///
/// Merges constituent atoms' implications with a multiplier:
/// - Causal: 1.5x
/// - Temporal: 1.0x (no amplification)
/// Deduplicates by (source, target) keeping max weight. Clamps to [0.0, 1.0].
pub fn compute_emergent_implications(
    atoms: &[EventAtom],
    composition_type: &CompositionType,
) -> Vec<RelationalImplication>
```

## Step 6: Top-level pipeline orchestrator

```rust
/// Run full composition detection: causal first (higher priority),
/// then temporal (excluding causal-consumed pairs), with emergent
/// implications computed for all compounds.
pub fn detect_compositions(atoms: &[EventAtom]) -> Vec<CompoundEvent>
```

1. `detect_causal_compositions(atoms)` → causal compounds + consumed pairs
2. `detect_temporal_compositions(atoms, &consumed_pairs)` → temporal compounds
3. For each compound, `compute_emergent_implications(constituent_atoms, composition_type)`
4. Return all compounds with emergent implications populated

## Step 7: Register module

**File**: `storyteller-engine/src/context/mod.rs`

Add after `pub mod prediction;`:
```rust
pub mod event_composition;
```

## Step 8: Wire into commit_previous_system (E.4)

**File**: `storyteller-engine/src/systems/turn_cycle.rs`

After D.3 committed classification (line ~348), before `CompletedTurn` construction:

```rust
// Phase E: Build event atoms and detect compositions
let (committed_atoms, committed_compounds) = committed_classification
    .as_ref()
    .map(|classification| {
        let atoms = crate::context::event_composition::build_event_atoms(
            classification, scene_id, turn_id,
        );
        let compounds = crate::context::event_composition::detect_compositions(&atoms);
        tracing::debug!(
            turn_number, atom_count = atoms.len(), compound_count = compounds.len(),
            "commit_previous_system: Phase E composition detection"
        );
        (atoms, compounds)
    })
    .unwrap_or_default();
```

Note: `scene_id` and `turn_id` need placeholder values (`SceneId::new()`, `TurnId::new()`) since persistence layer isn't wired yet.

Include `committed_atoms` and `committed_compounds` in `CompletedTurn` construction.

## Step 9: Tests (~20 tests in event_composition.rs)

**Helper functions:**
- `make_test_atom(kind, participants) -> EventAtom` — minimal atom with defaults
- `make_actor(mention) -> Participant` — Unresolved EntityRef with Actor role
- `make_target(mention) -> Participant` — Unresolved EntityRef with Target role

**Bridge tests (5):**
1. `label_to_event_kind_known_labels` — all 8 labels produce correct variants
2. `label_to_event_kind_unknown_returns_state_assertion` — fallback
3. `build_event_atoms_from_single_kind` — one label → one atom with participants
4. `build_event_atoms_multiple_kinds` — two labels → two atoms
5. `build_event_atoms_empty_classification` — no kinds → empty vec

**Participant overlap tests (3):**
6. `participant_overlap_shared_actor` — same mention text in Actor → overlap
7. `participant_overlap_actor_target_cross` — Actor of A = Target of B → overlap
8. `participant_overlap_no_match` — disjoint participants → no overlap

**Temporal tests (3):**
9. `temporal_shared_participants_compose` — two atoms with shared Actor → compound
10. `temporal_no_overlap_skips` — disjoint → no compound
11. `temporal_respects_excluded_pairs` — excluded pairs skipped

**Causal tests (4):**
12. `causal_action_then_emotional` — ActionOccurrence + EmotionalExpression with overlap → causal
13. `causal_speech_then_relational_shift` — SpeechAct + RelationalShift → causal
14. `causal_requires_participant_overlap` — matching kinds but no overlap → skip
15. `causal_wrong_order_skips` — effect before cause → no match

**Emergent weight tests (2):**
16. `emergent_causal_multiplier_applied` — causal 1.5x on atom weights
17. `emergent_temporal_no_amplification` — temporal 1.0x (no change)

**Pipeline tests (3):**
18. `detect_compositions_prefers_causal_over_temporal` — same pair → causal only, not both
19. `detect_compositions_mixed_atoms` — 3+ atoms → some causal, some temporal
20. `detect_compositions_empty_atoms` — no atoms → no compounds

## Verification

1. `cargo check --all-features` — compilation
2. `cargo test --workspace` — all tests pass (existing + ~20 new)
3. `cargo lint` — no clippy warnings
4. `cargo fmt --check` — clean formatting
5. Verify test count: expect ~356 total (336 existing + ~20 new)

## Files Summary

| File | Action | Description |
|------|--------|-------------|
| `storyteller-engine/src/context/event_composition.rs` | CREATE | Core composition detection + bridge + tests |
| `storyteller-engine/src/context/mod.rs` | MODIFY | Add `pub mod event_composition;` |
| `storyteller-engine/src/components/turn.rs` | MODIFY | Add `committed_atoms`, `committed_compounds` to CompletedTurn |
| `storyteller-engine/src/systems/turn_cycle.rs` | MODIFY | Wire Phase E into commit_previous_system |

## Design Decisions

1. **No new TurnCycleStage** — composition runs within CommittingPrevious, not a separate stage. The atoms are derived from committed-turn classification, so they belong in the commitment flow.
2. **Causal patterns require participant overlap** — prevents false positives from unrelated events in the same turn.
3. **Causal preferred over temporal** — when atoms match both, causal wins (higher narrative significance).
4. **Default sub-type values in label_to_event_kind** — classifier outputs "ActionOccurrence" but can't distinguish Examine vs Perform. Intentionally naive prototype.
5. **Placeholder scene/turn IDs** — `SceneId::new()` and `TurnId::new()` until persistence layer provides real values.
6. **Composition depth = 1** — compounds contain only atoms, never other compounds.
