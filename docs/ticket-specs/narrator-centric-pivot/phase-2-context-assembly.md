# Phase 2: Narrator Context Assembly

**Status**: Draft
**Depends on**: Phase 1 (complete)
**Enables**: Phase 6 (integration)

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

### Build

1. **Preamble construction** (`PersistentPreamble` from `SceneData` + `CharacterSheet`s)
   - Extract narrator voice, anti-patterns, setting, cast, boundaries from scene data
   - Build `CastDescription` for each character with voice notes
   - Construct the complete Tier 1 preamble
   - Estimate token count for budget tracking

2. **Scene journal** (`SceneJournal` with progressive compression)
   - Create journal entries from turn data (player input + resolved intents + emotional markers)
   - Implement compression: Full → Summary → Skeleton based on recency
   - Implement compression resistance for high-weight entries (emotional significance, revelations)
   - Token budget tracking — trigger compression when journal exceeds budget
   - Start with simple recency-based compression; narrative-weight compression can iterate

3. **Retrieved context assembly** (basic Tier 3)
   - For the prototype, Tier 3 is assembled from `CharacterSheet` data (backstory, knows/does_not_know, relational context) rather than graph queries
   - Select retrieval based on entity references in current turn
   - Information boundary enforcement: filter by what characters know
   - Structured output as `RetrievedContext` entries
   - Full GraphRAG (PostgreSQL + AGE traversal) deferred to later — this phase uses in-memory data

4. **Context input assembly** (`NarratorContextInput` from tiers + resolver output)
   - Combine preamble + journal + retrieved context + turn data
   - Token budget estimation across all tiers
   - Truncation strategy when total exceeds budget

5. **Narrator refactoring** — new `render_from_context()` method
   - `NarratorAgent` gains a method accepting `NarratorContextInput`
   - Construct the LLM prompt from structured context (not raw prose)
   - Existing `render()` (taking `ReconcilerOutput`) remains for backward compatibility
   - System prompt construction refactored to use preamble data

6. **Manual integration test**
   - Construct `NarratorContextInput` by hand for the workshop scene
   - Feed through Narrator via Ollama
   - Compare output quality with the legacy `ReconcilerOutput` path
   - This is a qualitative evaluation, not automated

### Not in Scope

- GraphRAG / PostgreSQL / AGE queries (deferred — use in-memory data)
- ML-generated journal entries (journals are built from structured data)
- Compression via LLM summarization (compression is structural/deterministic)
- Bevy system integration (Phase 6)
- Event classifier (Phase 5) — classified events are constructed manually for testing

## Implementation Plan

### Step 1: Preamble Builder

New module: `storyteller-engine/src/context/preamble.rs`

```
fn build_preamble(scene: &SceneData, characters: &[&CharacterSheet]) -> PersistentPreamble
```

Extracts from scene data:
- Narrator voice from a voice configuration (for now, hardcoded literary fiction voice matching the existing narrator system prompt)
- Anti-patterns from scene evaluation criteria (inverted)
- Setting from `SceneSetting`
- Cast from `CastEntry` + `CharacterSheet.voice`
- Boundaries from `SceneConstraints.hard`

Tests: preamble construction from workshop data, token estimation.

### Step 2: Scene Journal

New module: `storyteller-engine/src/context/journal.rs`

```
impl SceneJournal {
    fn add_turn(&mut self, entry: JournalEntry)
    fn compress_if_needed(&mut self)
    fn to_context_string(&self) -> String
    fn estimated_tokens(&self) -> u32
}
```

Compression strategy (initial):
- Turns 0 to N-3: Skeleton
- Turns N-2 to N-1: Summary
- Turn N (current): Full

Entries with emotional markers matching scene stakes resist one compression level.

Tests: add turns, verify compression triggers, verify token budget respected, verify emotional resistance.

### Step 3: Retrieved Context Builder

New module: `storyteller-engine/src/context/retrieval.rs`

```
fn retrieve_context(
    referenced_entities: &[EntityId],
    characters: &[&CharacterSheet],
    scene: &SceneData,
    information_horizon: &InformationHorizon,
) -> Vec<RetrievedContext>
```

For the prototype, `InformationHorizon` is derived from character `knows`/`does_not_know` lists. Retrieval walks character sheets looking for relevant backstory, relational context, and emotional subtext for referenced entities.

Tests: retrieval with entity references, information boundary enforcement.

### Step 4: Context Assembler

New module: `storyteller-engine/src/context/mod.rs` (re-exports)

```
fn assemble_narrator_context(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    journal: &SceneJournal,
    resolver_output: &ResolverOutput,
    player_input: &str,
    referenced_entities: &[EntityId],
) -> NarratorContextInput
```

Combines all three tiers. Estimates token count. If over budget, trims Tier 3 first, then compresses Tier 2 more aggressively.

Tests: full assembly from workshop data, token budget within limits.

### Step 5: Narrator Refactoring

Modify: `storyteller-engine/src/agents/narrator.rs`

Add:
```
impl NarratorAgent {
    pub async fn render_from_context(
        &mut self,
        context: &NarratorContextInput,
    ) -> StorytellerResult<NarratorRendering>
}
```

The new method constructs the LLM prompt from structured context rather than from `ReconcilerOutput`. The prompt format:

```
[System: Tier 1 preamble — narrator identity, scene, cast, boundaries]

[Scene journal — Tier 2, rendered as structured narrative record]

[Retrieved context — Tier 3, rendered as structured facts with emotional annotation]

[Current turn — resolver output rendered as structured intent summaries]

Render this moment as narrative prose. [word limit, tonal guidance]
```

The key insight from the architecture doc: context should be structured, not pre-rendered. The Narrator receives facts with emotional annotation, not prose to parrot.

Tests: prompt construction from assembled context (unit test, no LLM). Integration test (manual, against Ollama).

### Step 6: Manual Quality Evaluation

Run the workshop scene through both paths:
1. Legacy: `StorykeeperDirective` → `CharacterAgent` (LLM) → `ReconcilerOutput` → `NarratorAgent::render()`
2. New: Manually constructed `NarratorContextInput` → `NarratorAgent::render_from_context()`

Compare on the scene's own evaluation criteria:
- Tone: quiet compression, not volume
- Information discipline: agents respect boundaries
- Subtext fidelity: more beneath dialogue than in it
- Character consistency: Bramblehoof reaches for metaphor, Pyotir is measured

This is qualitative and subjective. Pete evaluates.

## New Module Structure

```
storyteller-engine/src/
├── context/
│   ├── mod.rs          # Re-exports, assemble_narrator_context()
│   ├── preamble.rs     # build_preamble()
│   ├── journal.rs      # SceneJournal methods, compression
│   └── retrieval.rs    # retrieve_context(), InformationHorizon
```

This is a new top-level module in the engine crate alongside `agents/`, `inference/`, `systems/`, etc.

## Acceptance Criteria

- [ ] `build_preamble()` produces a `PersistentPreamble` from workshop scene data
- [ ] `SceneJournal` tracks turns with progressive compression
- [ ] Compression respects token budget (~800-1200 tokens)
- [ ] Emotionally significant entries resist compression
- [ ] `retrieve_context()` returns relevant backstory for referenced entities
- [ ] Information boundaries enforced (characters don't leak what they don't know)
- [ ] `assemble_narrator_context()` combines all three tiers within total token budget
- [ ] `NarratorAgent::render_from_context()` produces prose from structured context
- [ ] All existing tests pass (59+)
- [ ] New tests for each context module
- [ ] Manual evaluation: Narrator output quality comparable to legacy path

## Open Questions for This Phase

1. **Token estimation**: How do we estimate tokens without a tokenizer? Options: word count / 0.75 (rough), tiktoken-rs, or a simple character-count heuristic. For the prototype, word count / 0.75 is probably fine.

2. **Compression format**: When compressing Full → Summary, what does Summary look like? Proposal: "{Character} {action_verb} {object/target}. {one emotional marker}." e.g., "Bramblehoof approached the fence. Anticipation and dread."

3. **Narrator prompt format**: The existing narrator system prompt is well-tuned. How much of it should migrate into the preamble vs. remain in the system prompt? Proposal: voice/anti-patterns stay in system prompt (they're stable), scene-specific content moves to preamble.

4. **Mock resolver output**: Phase 2 doesn't have a real Resolver yet. We need mock `ResolverOutput` for testing. Proposal: hand-construct from the existing workshop scene's character intents, or build a trivial pass-through resolver that wraps predictions into outcomes without real resolution.

## Risk

Low. This phase is deterministic Rust code with no external dependencies beyond the existing Ollama integration. The workshop data provides concrete test material. The main risk is that the three-tier context doesn't improve Narrator output quality — but even if it doesn't, the structured context format is architecturally correct and will improve with better models.
