# Phase B: Event-Entity Bridge

**Status**: Complete (Feb 8, 2026)
**Branch**: `jcoletaylor/event-system-foundations`
**Commit**: follows Phase A (`505560f`)

## Context

Phase A established the event grammar vocabulary:

- `event_grammar.rs` — EventAtom, EventKind, Participant, ParticipantRole, RelationalImplication, ImplicationType, EventSource, EventConfidence, CompoundEvent, Turn, TurnState, CommittedTurn, PredictionMetadata, EventPayload
- `entity.rs` — EntityRef (Resolved/Unresolved/Implicit), ReferentialContext, PromotionTier, RelationalWeight, UnresolvedMention, EntityBudget
- `event.rs` — TurnId, NarrativeEvent with typed EventPayload

Phase B implements the **behavioral logic** that connects events to entity promotion. The organizing principle: **events create relationships, and relationships create entities worth tracking.** An entity earns its EntityId by participating in events that create or modify relationships.

## Design Decisions

1. **Pure functions with config struct** (not trait): The promotion logic is deterministic and has no need for runtime polymorphism. A `PromotionConfig` struct carries threshold values. Functions take references to types from Phase A. No trait abstraction — YAGNI.

2. **New `promotion/` module at crate root**: Behavioral logic doesn't belong in `types/` (which is pure data). A new `storyteller-core/src/promotion/` module sits alongside `types/`, `traits/`, `grammars/`. This follows the existing pattern where `types/` holds data and other modules hold logic.

3. **MentionIndex as in-memory BTreeMap**: Option 2 from `entity-reference-model.md`. Separate mention index rather than inline ledger mutation or resolution overlay events. BTreeMap<String, Vec<UnresolvedMention>> keyed by normalized mention text. Simpler than ledger mutation, more efficient than overlay events.

4. **ResolutionRecord for retroactive promotion**: When an entity is promoted, a `ResolutionRecord` is created (not mutating the original UnresolvedMention). This preserves ledger immutability — the original mention stays as-is, the resolution is recorded separately.

5. **Placeholder thresholds**: All threshold values (weight for Tracked, weight for Persistent, demotion scene count, etc.) are initial guesses that need calibration from play data. `PromotionConfig::default()` provides the guesses.

6. **No ClassifiedEvent→EventAtom bridge**: That's Phase C work. Phase B operates on EventAtom and RelationalImplication types directly.

## What Was Built

### Files Created/Modified

| File | Change |
|------|--------|
| `storyteller-core/src/promotion/mod.rs` | **NEW** — PromotionConfig with Default, module declarations |
| `storyteller-core/src/promotion/weight.rs` | **NEW** — compute_relational_weight(), normalize_mention(), entity_matches() |
| `storyteller-core/src/promotion/tier.rs` | **NEW** — determine_promotion_tier(), evaluate_demotion() |
| `storyteller-core/src/promotion/resolution.rs` | **NEW** — resolve_entity_ref(), TrackedEntity, SceneResolutionContext, 4 resolution strategies |
| `storyteller-core/src/promotion/mention_index.rs` | **NEW** — MentionIndex, ResolutionRecord, retroactively_promote() |
| `storyteller-core/src/lib.rs` | **EXTEND** — added `pub mod promotion;` and module doc entry |

No changes to Phase A types. No changes to engine crate.

### Public API Surface

**`promotion/mod.rs`** — Configuration
- `PromotionConfig` — 6 calibratable threshold fields with `Default` impl

**`promotion/weight.rs`** — Relational weight computation
- `compute_relational_weight(entity, events, player_entity_id, config) → RelationalWeight` — sums implication weights across events, tracks distinct events/partners, applies player multiplier
- `normalize_mention(mention) → String` — lowercase, trim, strip leading articles ("the", "a", "an")
- `entity_matches(needle, haystack) → bool` — cross-variant EntityRef comparison (Resolved by ID, Unresolved by normalized text, Implicit by normalized implied_entity, mixed variants never match)

**`promotion/tier.rs`** — Tier lifecycle
- `determine_promotion_tier(weight, current_tier, authored_floor, config) → PromotionTier` — monotonic promotion (never returns below current_tier or authored_floor)
- `evaluate_demotion(current_tier, authored_floor, scenes_without, turns_without, config) → Option<PromotionTier>` — inactivity-based demotion (never below Referenced, never below authored_floor)

**`promotion/resolution.rs`** — Entity reference resolution
- `resolve_entity_ref(entity_ref, context) → Option<EntityId>` — tries 4 strategies in order: possessive, spatial, anaphoric, descriptive. Conservative — ambiguous results return None
- `TrackedEntity` — entity with enough context for resolution (id, canonical_name, descriptors, current_scene, possessor)
- `SceneResolutionContext` — scene cast + scene ID for resolution scope

**`promotion/mention_index.rs`** — Retroactive promotion
- `MentionIndex` — BTreeMap<normalized_text, Vec<UnresolvedMention>> with insert/lookup/remove/len/is_empty
- `ResolutionRecord` — immutable record linking an unresolved mention to its resolved EntityId (preserves ledger immutability)
- `retroactively_promote(entity_id, mention_text, index) → Vec<ResolutionRecord>` — resolves all prior mentions, removes from index

### Test Coverage

48 new tests across 4 submodules (114 total in core, 202 workspace):

| Module | Tests | Coverage |
|--------|-------|----------|
| `mod.rs` | 2 | PromotionConfig default + custom values |
| `weight.rs` | 13 | normalize_mention (2), entity_matches (4), compute_relational_weight (7) |
| `tier.rs` | 13 | determine_promotion_tier (8), evaluate_demotion (5) |
| `resolution.rs` | 10 | Resolved passthrough, Implicit None, possessive, spatial, descriptive, ambiguity, empty cast, strategy ordering, name matching, no-match |
| `mention_index.rs` | 10 | insert/lookup, multiple same text, remove, normalize key, len/is_empty, default, retroactive promotion (4) |

## Implementation Notes

### Weight computation design

Player interaction weight is additive, not replacing. When a player is involved in an event, the implication weights for that event contribute both to `total_weight` (base) and to `player_interaction_weight` (multiplied by `player_interaction_multiplier`). The final `total_weight` in `RelationalWeight` is `base + player_interaction_weight`. This means player attention roughly triples the weight of an interaction with the default 2.0 multiplier.

Event deduplication uses `BTreeSet<EventId>` — if the same `EventAtom` appears twice in the input slice, it's counted once. Relationship partner counting uses a `BTreeSet<String>` of entity keys (UUID strings for resolved, normalized mention for unresolved).

### Resolution strategy: anaphoric is deferred

The anaphoric resolution strategy (resolving "it" to the most recently mentioned compatible entity) requires ledger access to look up what entity was mentioned in a prior event. The current implementation returns `None` — this is explicitly Phase D work when the Bevy ECS provides event ledger access as a system parameter. The function signature and strategy ordering are in place; only the body needs implementation.

### Resolution strategy: spatial is simplified

Spatial resolution currently uses the mention text to find a canonical name match in the scene cast, rather than doing full spatial reasoning. The `spatial_context` field from `ReferentialContext` serves as a gate (strategy only fires if spatial context exists) but the actual resolution is name-based. Full spatial reasoning (entity positions within scene geography) is future work that depends on the rendered space model.

### Descriptive resolution: two-phase matching

Descriptive resolution first tries canonical name matching (normalized). If exactly one entity matches by name, it wins immediately. Only if name matching fails or is ambiguous does it fall through to descriptor overlap scoring. This means `resolve_entity_ref` with mention "Sarah" and no descriptors will find Sarah by name — you don't need descriptors for entities with distinctive names.

### ResolutionRecord field naming

The plan specified `resolved_in_turn: TurnId` but the implementation uses `mention_turn: TurnId` — this better reflects that it records the turn where the *mention* occurred (provenance of the original unresolved reference), not the turn where resolution happened. Resolution is a system operation that can happen at any time.

## Verification Results

```
cargo check --all-features         ✓ Compiles cleanly
cargo test --workspace             ✓ 202 tests pass (114 core + 55 engine + 33 ml)
cargo clippy --all-targets --all-features  ✓ No warnings
cargo fmt --check                  ✓ Formatted
```

## What Phase B Does NOT Include

- **Bevy system integration** — Phase D (turn lifecycle management, entity_lifecycle.rs system)
- **ClassifiedEvent → EventAtom bridge** — Phase C (ML classification pipeline)
- **Implication inference from EventKind** — Phase C.4 (heuristic mapping)
- **Persistence layer** — no database writes. All Phase B logic operates on in-memory data structures
- **Ledger mutation** — MentionIndex + ResolutionRecord is the chosen approach (option 2 from entity-reference-model.md)
- **Anaphoric resolution body** — returns None; needs ledger access (Phase D)
- **Full spatial reasoning** — simplified to name matching with spatial gate; needs rendered space model

## Open Questions for Future Phases

1. **Threshold calibration**: All `PromotionConfig` defaults are guesses. Need play data to calibrate. Consider logging weight distributions during pilot sessions.
2. **Anaphoric resolution scope**: Should anaphoric resolution look at the whole scene or just recent N turns? Recency bias vs. completeness.
3. **Demotion and authored floor interaction**: When `evaluate_demotion` returns `Some(tier)` where tier equals the authored floor, should the caller treat this as "no effective demotion" or as "demotion that was clamped"? Currently returns the clamped tier; caller can compare to current.
4. **MentionIndex persistence**: Currently in-memory only. When sessions span multiple server restarts, the index needs to be reconstructable from the event ledger + resolution records.
5. **Multi-mention entities**: An entity might be mentioned as "the cup", "Tanya's cup", and "that old thing". Currently these are three separate normalized keys. Entity aliasing (multiple mention strings → same entity) needs a resolution record or alias table.
