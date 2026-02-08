# Phase 2: Narrator Context Assembly

**Status**: Complete
**Depends on**: Phase 1 (complete)
**Enables**: Phase 6 (integration)
**Completed**: 2026-02-07
**Tests**: 90 total (29 core + 61 engine), up from 59 at Phase 1 completion

## Goal

Build the Storykeeper's context assembly pipeline and prove the three-tier system works. By the end of Phase 2, the Narrator receives a `NarratorContextInput` instead of a `ReconcilerOutput`, and produces comparable or better prose against Ollama.

This is the first phase where the narrator-centric architecture produces visible output. The Narrator still works — it just receives richer, more structured context.

## Why Phase 2 Next

Phase 2 is the highest-value next step because:

1. **Immediately testable**: The Narrator and Ollama integration already work. We can construct context bundles by hand, feed them through the Narrator, and evaluate quality.
2. **No ML dependency**: Context assembly is deterministic Rust code. No training data, no model files, no ONNX. Pure data transformation.
3. **Proves the architecture**: If the three-tier context produces good Narrator output, the whole pivot is validated. If it doesn't, we learn that before investing in ML pipelines.
4. **Unblocks Phase 6**: Context assembly is on the critical path to end-to-end integration.

## Scope

### Built

1. **PhaseObserver trait** (`storyteller-core/src/traits/phase_observer.rs`) — Layer 2 session observability
   - `PhaseEvent` struct with timestamp, turn number, phase kind, and typed detail
   - `PhaseEventDetail` enum with 11 variants covering context assembly, narrator rendering, and generic lifecycle events
   - `PhaseObserver` trait (`emit(&self, event)`) — decouples emission from consumption
   - `NoopObserver` — discards events (production paths that don't need observability)
   - `CollectingObserver` — captures events in `Vec` (test assertions)
   - All pipeline functions accept `&dyn PhaseObserver` and emit structured domain events
   - 4 tests

2. **Preamble construction** (`storyteller-engine/src/context/preamble.rs`)
   - `build_preamble(scene, characters, observer)` → `PersistentPreamble`
   - Extracts narrator voice (hardcoded literary fiction for prototype), anti-patterns, setting, cast descriptions with voice notes, boundaries from scene constraints
   - `estimate_preamble_tokens()` for budget tracking
   - `render_preamble()` produces the full Tier 1 text block
   - Emits `PreambleBuilt` event with cast count, boundary count, estimated tokens
   - 3 tests

3. **Scene journal** (`storyteller-engine/src/context/journal.rs`)
   - Free functions (not methods on `SceneJournal`) accepting `&dyn PhaseObserver`:
     - `add_turn(journal, turn_number, content, referenced_entities, emotional_markers, observer)`
     - `compress_if_needed(journal, observer)` — triggers when token budget exceeded
     - `render_journal(journal)` → rendered text for Narrator
     - `estimate_journal_tokens(journal)` → token count
   - Compression strategy: current turn = Full, N-1/N-2 = Summary, older = Skeleton
   - Emotional resistance: entries with emotional markers resist one compression level (`Skeleton` → `Summary`, `Summary` stays `Full`)
   - Compression transforms: Summary = first sentence; Skeleton = first clause
   - Emits `JournalEntryAdded`, `JournalCompressed`, `JournalEntryCompressed` events
   - 7 tests

4. **Retrieved context assembly** (`storyteller-engine/src/context/retrieval.rs`)
   - `retrieve_context(referenced_entities, characters, scene, observer)` → `Vec<RetrievedContext>`
   - Retrieves from character sheets: backstory (first paragraph), knowledge items, performance notes, self-edge history pattern, scene-specific stakes
   - Information boundary enforcement via keyword matching against `does_not_know`
   - Revealed items bypass boundaries
   - Emits `ContextRetrieved` and `InformationBoundaryApplied` events
   - 5 tests

5. **Context input assembly** (`storyteller-engine/src/context/mod.rs`)
   - `assemble_narrator_context(scene, characters, journal, resolver_output, player_input, referenced_entities, total_budget, observer)` → `NarratorContextInput`
   - Combines all three tiers with token budget tracking
   - Budget trimming: if total exceeds budget, trim Tier 3 first (drop lowest-priority items)
   - `DEFAULT_TOTAL_TOKEN_BUDGET = 2500`
   - Emits `ContextAssembled` event with per-tier token counts and trimmed flag
   - 2 tests

6. **Token estimation** (`storyteller-engine/src/context/tokens.rs`)
   - `estimate_tokens(text)` → `u32` using `(word_count * 4).div_ceil(3)` heuristic (~words / 0.75)
   - Intentionally naive for prototype; production would use a real tokenizer
   - 4 tests

7. **Narrator refactoring** (`storyteller-engine/src/agents/narrator.rs`)
   - New constructor: `from_context(context, llm)` — builds narrator from `NarratorContextInput`
   - New methods:
     - `render_from_context(context, observer)` → `NarratorRendering`
     - `render_opening_from_context(observer)` → `NarratorRendering`
   - Prompt construction helpers:
     - `build_system_prompt_from_context(context)` — Tier 1 preamble as system prompt
     - `build_turn_message_from_context(context)` — Tier 2 journal + Tier 3 retrieved + resolver output + player input as user message
   - Legacy `new()`, `render()`, `render_opening()` preserved for backward compatibility
   - Emits `NarratorPromptBuilt` and `NarratorRenderingComplete` events
   - 5 new tests (using MockLlm provider)

### Not in Scope (unchanged)

- GraphRAG / PostgreSQL / AGE queries (deferred — use in-memory data)
- ML-generated journal entries (journals are built from structured data)
- Compression via LLM summarization (compression is structural/deterministic)
- Bevy system integration (Phase 6)
- Event classifier (Phase 5) — classified events are constructed manually for testing

### Deferred

- **Manual quality evaluation against Ollama** (Step 6 from original spec) — not yet performed. Requires constructing a full `NarratorContextInput` for the workshop scene and running both legacy and new paths side-by-side. Can be done anytime; the code is ready.

## Module Structure (as built)

```
storyteller-core/src/
├── traits/
│   ├── mod.rs              # Re-exports PhaseObserver, NoopObserver, CollectingObserver
│   └── phase_observer.rs   # PhaseObserver trait, PhaseEvent, PhaseEventDetail, observers

storyteller-engine/src/
├── context/
│   ├── mod.rs              # assemble_narrator_context(), DEFAULT_TOTAL_TOKEN_BUDGET
│   ├── preamble.rs         # build_preamble(), estimate_preamble_tokens(), render_preamble()
│   ├── journal.rs          # add_turn(), compress_if_needed(), render_journal()
│   ├── retrieval.rs        # retrieve_context(), information boundary enforcement
│   └── tokens.rs           # estimate_tokens() — word-count heuristic
├── agents/
│   └── narrator.rs         # +from_context(), +render_from_context(), +render_opening_from_context()
```

## Acceptance Criteria

- [x] `build_preamble()` produces a `PersistentPreamble` from workshop scene data
- [x] Scene journal tracks turns with progressive compression
- [x] Compression respects token budget (~800-1200 tokens)
- [x] Emotionally significant entries resist compression
- [x] `retrieve_context()` returns relevant backstory for referenced entities
- [x] Information boundaries enforced (characters don't leak what they don't know)
- [x] `assemble_narrator_context()` combines all three tiers within total token budget
- [x] `NarratorAgent::render_from_context()` produces prose from structured context
- [x] All existing tests pass (59+ → 90)
- [x] New tests for each context module (31 new tests)
- [x] Session observability: all pipeline phases emit `PhaseEvent`s through `PhaseObserver` trait
- [ ] Manual evaluation: Narrator output quality comparable to legacy path (deferred)

## Resolved Design Questions

1. **Token estimation**: Word count / 0.75 via `(word_count * 4).div_ceil(3)`. Simple, fast, good enough for budget tracking. Production can swap in tiktoken-rs or model-specific tokenizer.

2. **Compression format**: Summary = first sentence of full content. Skeleton = first clause (up to first comma, semicolon, or em-dash, minimum 20 chars). Emotional markers resist one compression level via `resist_one_level()`.

3. **Narrator prompt format**: Tier 1 preamble is the system prompt. Tier 2 journal + Tier 3 retrieved context + resolver output + player input compose the user message. The Narrator receives structured facts with emotional annotation, not prose to parrot.

4. **Mock resolver output**: Hand-constructed `ResolverOutput` with empty `sequenced_actions` and a `scene_dynamics` string. Sufficient for context assembly testing; real resolver output arrives in Phase 4.

5. **Observability model**: `PhaseObserver` trait (named for conciseness over `SessionObservabilityEmitter`). Synchronous `emit()` method. Production implementation will back this with a bounded MPSC channel dispatching to a dedicated observer system, following tasker-core's actor-command-service pattern.

6. **Journal as free functions vs methods**: Implemented as free functions (`add_turn(journal, ...)`) rather than methods on `SceneJournal`. This keeps `SceneJournal` as a pure data type in `storyteller-core` and puts behavior in `storyteller-engine` where it belongs. The observer parameter flows naturally through function arguments.

## Risk Assessment (post-implementation)

Low risk realized. The three-tier context assembly is clean, well-tested, and architecturally sound. The `PhaseObserver` trait provides the observability hooks needed for Layer 2 session debugging without coupling to any specific transport. The main remaining risk is qualitative — whether the structured context actually improves Narrator output — which requires the deferred manual evaluation step.
