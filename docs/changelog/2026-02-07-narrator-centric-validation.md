# Narrator-Centric Architecture Validated

**Date**: 2026-02-07
**Phase**: Phase 2 — Context Assembly + Architectural Validation
**Branch**: `jcoletaylor/agent-prototyping`

## What Happened

We built the three-tier context assembly pipeline (Phase 2 of the narrator-centric pivot) and ran a controlled parity test comparing the legacy multi-agent path (3 LLM calls/turn) against the new single-Narrator path (1 LLM call/turn). Same scene, same model (Mistral 7B), same player inputs. Results validated the architectural bet. Legacy code was removed from the codebase.

We then tested a larger model (Qwen 2.5 32B) to evaluate whether bigger models improve prose quality under the same prompts.

## Parity Test: Legacy vs. Narrator-Centric

### Setup

- **Scene**: "The Flute Kept" (Bramblehoof + Pyotir, quiet two-character encounter)
- **Model**: Mistral 7B (identical for both paths)
- **Hardware**: Apple M4, 64GB RAM
- **Player inputs**: (1) "I approach the fence slowly, my hooves careful in the turned earth" (2) "I look at the boy more closely, searching for recognition"

### Results: Responsiveness

| Metric | Legacy (3 LLM calls) | Context (1 LLM call) | Improvement |
|--------|----------------------|----------------------|-------------|
| Opening | N/A | 12.3s | — |
| Turn 1 | ~35s | 13.5s | 2.6x faster |
| Turn 2 | ~49s | 20.8s | 2.4x faster |
| Total session | ~1m24s | ~52s | 38% less wall time |
| LLM calls/turn | 3 (2 character + narrator) | 1 (narrator only) | 67% reduction |
| CPU/thermal load | Fan sustained high | Brief per turn | Substantially lower |

### Results: Prose Quality

| Dimension | Legacy | Context | Notes |
|-----------|--------|---------|-------|
| Character voice differentiation | Good | Good | Both produce distinct voices from same tensor data |
| Information boundary discipline | Good | Better | Context path: Pyotir doesn't prematurely open up. Narrator respects information boundaries more consistently when they're explicitly structured in the preamble |
| Emotional subtext | Good | Good | Both convey defended grief, cautious approach. Context path has richer annotation |
| Repetition across turns | Both repeat | Both repeat | Mistral 7B limitation — re-describes setting, echoes prior phrases |
| Telling vs. showing | Both tell | Both tell | "A testament to..." type summarizing. Mistral 7B instruction-following ceiling |

### Verdict

The narrator-centric architecture is **faster, equally coherent, and more disciplined at information boundaries** than the legacy multi-agent approach. Quality parity was achieved with a single LLM call receiving structured context, vs. three LLM calls with separate agent contexts. The remaining quality issues (repetition, telling-not-showing) are model limitations, not architecture limitations.

## Model Comparison: Mistral 7B vs. Qwen 2.5 32B

After validating the architecture, we tested whether a larger model improves the experience.

### Results: Latency

| Phase | Mistral 7B | Qwen 32B | Ratio |
|-------|-----------|----------|-------|
| Opening | 12.3s | 52.4s | 4.3x slower |
| Turn 1 | 13.5s | 59.7s | 4.4x slower |
| Turn 2 | 20.8s | 68.9s | 3.3x slower |
| Total | ~52s | ~181s | 3.5x slower |
| Tokens generated | Moderate | Excessive (863, 2228, 3917) | 3-5x more |

### Results: Prose Quality

| Dimension | Mistral 7B | Qwen 32B |
|-----------|-----------|----------|
| Vocabulary | Adequate | Richer, more varied |
| Physical detail | Basic | More sensory, more atmospheric |
| Word count discipline | Reasonable | **Ignored** — blew past 200/300 word limits consistently |
| Repetition across turns | Moderate | **Worse** — same images repeated verbatim between turns (mended fence, roughened hands, herbs by the door) |
| Meta-fictional leakage | None | **Present** — "Pyotir's refusal to be reduced to a character arc", "being framed as just another data point" |
| Internal state exposition | Some | **More** — "Bramblehoof feels the pull of his own narrative" violates the "never state internal thoughts" constraint |
| Instruction following | Moderate | Poor — ignored multiple explicit system prompt constraints |

### Verdict

The Mistral 7B output was **more disciplined** than Qwen 32B. The larger model has a richer vocabulary but does not respect narrator constraints — word limits, internal thought prohibition, fourth-wall maintenance. This confirms that **the improvement path is better prompts and context assembly, not bigger models**. The architecture is doing its job. The bottleneck is prompt engineering and model instruction-following, not pipeline design.

For interactive play on M4 hardware, Mistral 7B at 12-20s/turn is far more usable than Qwen 32B at 50-70s/turn. A middle-ground model (14B, strong instruction-following) or a cloud API (Sonnet/Haiku) may hit the quality/speed sweet spot.

## Legacy Code Removed

With the parity test confirming the narrator-centric approach, we removed the legacy multi-agent code:

| Removed | Lines | What |
|---------|-------|------|
| `agents/character.rs` | ~560 | LLM-based CharacterAgent with tensor→NL pipeline |
| `agents/reconciler.rs` | ~148 | Deterministic intent sequencer |
| `agents/storykeeper.rs` | ~165 | Directive producer for character agents |
| `bin/play_scene.rs` | ~162 | Legacy multi-agent interactive binary |
| `StorykeeperDirective` | — | Message type for character agent directives |
| `CharacterIntent` | — | LLM-generated character behavior output |
| `ReconcilerOutput` | — | Reconciled multi-character sequencing |

**17 tests removed** with the deleted code (90 → 73 passing). All remaining tests pass. No dangling references.

The NarratorAgent was simplified: `from_context()` → `new()`, `render_from_context()` → `render()`, `render_opening_from_context()` → `render_opening()`. These are now the only constructor and rendering methods.

## What Was Built (Phase 2)

### storyteller-core additions

| File | What |
|------|------|
| `types/narrator_context.rs` | `NarratorContextInput`, `PersistentPreamble`, `CastDescription`, `SceneJournal`, `JournalEntry`, `CompressionLevel`, `RetrievedContext` |
| `types/prediction.rs` | `CharacterPrediction`, `ActionPrediction`, `SpeechPrediction`, `ThoughtPrediction`, `EmotionalDelta`, `ActionType`, `SpeechRegister`, `ClassifiedEvent`, `EventType`, `EmotionalRegister` |
| `types/resolver.rs` | `ResolverOutput`, `SequencedCharacterAction`, `ActionOutcome`, `SuccessDegree`, `ConflictResolution` |
| `traits/phase_observer.rs` | `PhaseObserver` trait, `PhaseEvent`, `PhaseEventDetail` enum, `CollectingObserver`, `NoopObserver` |

### storyteller-engine additions

| File | What |
|------|------|
| `context/mod.rs` | `assemble_narrator_context()` — orchestrates all three tiers |
| `context/preamble.rs` | `build_preamble()` from scene + cast data, `render_preamble()` to structured text |
| `context/journal.rs` | `add_turn()`, `compress_journal()` with progressive compression, `render_journal()` |
| `context/retrieval.rs` | `retrieve_context()` with information boundary enforcement, `render_retrieved()` |
| `context/tokens.rs` | `estimate_tokens()` — word-based approximation for budget management |

### Test counts

- **73 total** (29 storyteller-core + 44 storyteller-engine)
- Context assembly: preamble construction (3), journal compression (6), retrieval + boundaries (5), full assembly + budget (4), token estimation (2)
- Narrator: prompt construction (2), rendering (2), observer events (1)

## Key Insight

> The improvement path is better prompts and better context assembly — not bigger models.

The three-tier context architecture (preamble + journal + retrieval) provides the Narrator with structured narrative fact and emotional annotation. The Narrator's job is to transform this into prose. A well-prompted 7B model with rich structured context outperformed a poorly-disciplined 32B model on the metrics that matter: information boundaries, character discipline, instruction following.

This validates the architectural bet: the system does the remembering, the boundary enforcement, the character prediction, the conflict resolution. The LLM — freed from all those responsibilities — does what it does best: tell the story. Making the upstream pipeline richer and more precise is where the leverage is.

## What Comes Next

1. **Phase 0**: Validate the character prediction approach — generate training data via the combinatorial matrix, train a first model, evaluate whether ML-predicted character intents produce coherent behavior
2. **Phase 4**: Resolver rules engine — deterministic action resolution with narrative distance zones and graduated success
3. **Phase 5**: Event classifier — structured player input processing
4. **Cloud LLM provider**: Add Anthropic API support (Sonnet/Haiku) for quality comparison without hardware constraints
5. **Prompt engineering**: Iterate on narrator system prompt to address repetition and telling-vs-showing
