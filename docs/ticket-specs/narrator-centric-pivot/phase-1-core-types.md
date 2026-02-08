# Phase 1: Core Type Restructuring

**Status**: Complete
**Date**: February 7, 2026
**Tests**: 59 passing (27 new), all clippy/fmt clean

## Goal

Update `storyteller-core` types to support the narrator-centric architecture. Add prediction, resolver, world model, and context assembly types. Extend existing types where needed. All existing tests continue to pass.

## Scope

Phase 1 is pure type work in `storyteller-core` plus workshop data updates in `storyteller-engine`. No behavioral changes, no pipeline rewiring. The new types establish the vocabulary for Phases 2-6.

## What Was Done

### New Type Modules (storyteller-core/src/types/)

#### `prediction.rs` — Character Prediction Types

The heart of the pivot. These types replace the LLM-generated prose of `CharacterIntent` with structured ML predictions.

| Type | Purpose |
|------|---------|
| `ActionPrediction` | What a character intends to do, with confidence |
| `ActionType` | Perform, Speak, Move, Examine, Wait, Resist |
| `SpeechPrediction` | What and how a character would speak (direction, not exact words) |
| `SpeechRegister` | Whisper, Conversational, Declamatory, Internal |
| `ThoughtPrediction` | Internal state visible to the Narrator for subtext rendering |
| `EmotionalDelta` | Predicted shift in emotional primaries per turn |
| `ClassifiedEvent` | Structured player input from the event classifier |
| `EventType` | Speech, Action, Movement, Observation, Interaction, Emote, Inquiry |
| `EmotionalRegister` | Aggressive, Vulnerable, Playful, Guarded, Neutral, Tender, Inquisitive |
| `ActivatedTensorFrame` | ML model's selection of relevant tensor axes for current context |
| `CharacterPrediction` | Complete ML output for one character per turn (frame + actions + speech + thought + deltas) |

**Design note**: `SpeechPrediction.content_direction` captures topic and direction rather than dialogue. The Narrator writes the actual words. This is important — if the ML model produced exact dialogue, the Narrator would parrot rather than create.

#### `resolver.rs` — Resolver Types

Replaces `ReconcilerOutput` with a richer resolution model.

| Type | Purpose |
|------|---------|
| `SuccessDegree` | FullSuccess, PartialSuccess, FailureWithConsequence, FailureWithOpportunity |
| `ActionOutcome` | One action's resolution with consequences and state changes |
| `StateChange` | EmotionalShift, RelationalShift, or WorldState change |
| `ConflictResolution` | How conflicting actions between characters were resolved |
| `ResolvedCharacterAction` | One character's complete resolved turn |
| `ResolverOutput` | Complete turn resolution — sequenced actions, original predictions, dynamics, conflicts |

**Design note**: `StateChange` is an enum with three variants rather than a trait or generic. This is deliberate — the Resolver's output must be fully serializable and inspectable. State changes feed back into the next turn's prediction features.

#### `world_model.rs` — Spatial Zones and Capabilities

| Type | Purpose |
|------|---------|
| `NarrativeDistanceZone` | Intimate, Conversational, Awareness, Peripheral, Absent — with affordance methods |
| `DistanceEntry` | Distance between two entities in a scene |
| `EnvironmentalConstraint` | Named constraint affecting action resolution |
| `WorldModel` | Complete scene world model (genre physics, spatial zones, constraints) |
| `Attribute` | Broad capability dimension (e.g., Presence, Insight, Resilience) |
| `Skill` | Specific learned capability with primary/secondary attribute links |
| `CapabilityProfile` | Complete attribute + skill profile for a character |

**Design note**: `NarrativeDistanceZone` derives `Ord` — the zones have a natural ordering from Intimate (closest) to Absent (farthest). The affordance methods (`can_hear_speech()`, `can_touch()`, `is_perceptible()`) encode zone semantics for the Resolver.

#### `narrator_context.rs` — Three-Tier Context Assembly

| Type | Purpose |
|------|---------|
| `PersistentPreamble` | Tier 1: Narrator identity, anti-patterns, setting, cast, boundaries |
| `CastDescription` | One cast member's entry in the preamble |
| `CompressionLevel` | Full, Summary, Skeleton — progressive compression for journal entries |
| `JournalEntry` | One turn's record at a given compression level |
| `SceneJournal` | Tier 2: Rolling scene journal with token budget |
| `RetrievedContext` | Tier 3: One piece of on-demand backstory or fact |
| `NarratorContextInput` | Complete assembled context for one Narrator turn |

**Design note**: `CompressionLevel` derives `Ord` (Full < Summary < Skeleton) for compression-level comparisons. `JournalEntry` tracks `referenced_entities` and `emotional_markers` to support compression priority decisions — emotionally significant turns resist compression.

### New Trait

#### `traits/game_design.rs` — GameDesignSystem

Pluggable resolution mechanics trait with `resolve()` and `resolve_single()` methods. Takes predictions + world model, returns resolver output. The trait exists to establish the extension point; no implementations yet.

### Modified Existing Types

#### `message.rs` — Turn Phase and Legacy Annotations

- `TurnPhaseKind` updated: `CharactersDeliberating` → `CharacterPrediction`, `Reconciling` → `Resolving`, added `ContextAssembly`, `NarratorRendering` → `Rendering`
- `StorykeeperDirective`, `CharacterIntent`, `ReconcilerOutput` annotated as **Legacy** with doc comments pointing to their replacements
- Module doc updated to describe narrator-centric pipeline

#### `character.rs` — CharacterSheet Extended

- Added `capabilities: CapabilityProfile` field with `#[serde(default)]` for backward compatibility
- Import added for `world_model::CapabilityProfile`
- Doc comment updated to mention capabilities and narrator-centric architecture

### Workshop Data Updates

#### `the_flute_kept.rs` — Capabilities for Both Characters

**Bramblehoof capabilities**:
- Attributes: Presence (0.85), Insight (0.75), Resilience (0.60), Agility (0.70)
- Skills: Music (0.95, primary: Presence), Persuasion (0.75), Perception (0.70), Fey Attunement (0.65)

**Pyotir capabilities**:
- Attributes: Presence (0.40), Insight (0.60), Resilience (0.80), Agility (0.50)
- Skills: Farming (0.80, primary: Resilience), Herbalism (0.55), Perception (0.65), Music (0.60, dormant)

**Design note**: Bramblehoof's Presence is high (bard) while Pyotir's Resilience is high (endurance under hardship). Pyotir's Music skill (0.60) is lower than Bramblehoof's (0.95) but higher than you might expect — the capacity is bedrock, the expression is suppressed. This mirrors the tensor where `creative_capacity` is 0.60 but `creative_expression` is 0.10.

## Test Summary

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| storyteller-core | 7 (grammars) | 25 | +18 (prediction: 5, resolver: 4, world_model: 4, narrator_context: 4, grammars: unchanged) |
| storyteller-engine | 25 (agents, workshop, inference) | 34 | +9 (workshop capabilities: 2, existing workshop: +7 from unchanged) |
| **Total** | **32** | **59** | **+27** |

All existing tests pass unmodified. The `#[serde(default)]` on `CapabilityProfile` means the existing workshop test data construction compiles and serializes correctly — the new field defaults to empty.

## What This Enables

With Phase 1 complete, the type vocabulary exists for:

- **Phase 2**: Build `PersistentPreamble` and `SceneJournal` construction from `SceneData` and `CharacterSheet`. Refactor `NarratorAgent` to accept `NarratorContextInput`. Testable against Ollama immediately.
- **Phase 4**: Implement `GameDesignSystem` using `CapabilityProfile`, `WorldModel`, and `ResolverOutput`. The Resolver has types to consume and produce.
- **Phase 3**: ML pipeline targets `CharacterPrediction` as output format. Training data validation uses `ActionType`, `SpeechRegister`, `AwarenessLevel` for coherence checks.
- **Phase 5**: Event classifier produces `ClassifiedEvent` with `EventType` and `EmotionalRegister`.
