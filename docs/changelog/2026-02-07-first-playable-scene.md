# First Playable Scene

**Date**: 2026-02-07
**Phase**: 2 — Agent Architecture + Playable Scene
**Branch**: `jcoletaylor/types-and-first-steps`

## What Happened

We ran an interactive two-character scene ("The Flute Kept") against local Ollama with mistral 7B. A human scene director typed narrative inputs, two LLM character agents deliberated in parallel, and a narrator agent rendered the result as literary prose. Five turns, clean exit.

This is the first time the storyteller system produced live narrative from its data structures.

## What We Proved

1. **Character tensors translate to LLM behavior.** The tensor-to-natural-language pipeline converts axis values, temporal layers, awareness levels, and contextual triggers into prose that shapes how the model performs. Bramblehoof reaches for metaphor. Pyotir is measured. The voice differentiation is real.

2. **Information boundaries hold.** Pyotir's agent never referenced ley line corruption, Whisperthorn, or the systematic pattern — information restricted to Bramblehoof's sheet. Bramblehoof's agent didn't know Pyotir's family details. The Storykeeper function (deterministic Rust, not LLM) filtered correctly.

3. **The emotional model produces subtext.** The awareness-level framing (Articulate → "you know you feel...", Defended → "there is something you push away...") gives each character a different relationship to their own inner state. Pyotir's defended anger and structural joy created a character who acts from feelings he can't name.

4. **Parallel agent deliberation works.** Both character agents run concurrently via `tokio::join!`, each with their own system prompt, conversation history, and LLM call. No shared state, no interference.

5. **The turn cycle pipeline is correct.** Player input → Storykeeper filtering → parallel character deliberation → Reconciler sequencing → Narrator rendering. All message types flow through the pipeline as designed.

## What We Observed (Honestly)

1. **The narrator re-renders prior scene content each turn.** Rather than advancing the narrative beat by beat, mistral re-describes the setting, echoes earlier dialogue, and re-summarizes emotional states. This is a prompt engineering problem — the system prompt needs stronger "don't repeat" constraints and the narrator needs to be told each passage is the NEXT beat, not a re-rendering of the whole scene.

2. **The narrator tells more than it shows.** "A testament to his unwavering commitment to survival" is exactly the kind of summarizing the anti-patterns list was supposed to prevent. Mistral 7B hits the ceiling of instruction-following with a ~3,000 token system prompt. A larger model would do better.

3. **The reconciler dynamics notes are basic.** The heuristic approach (keyword scanning for dialogue/movement) produces generic notes. This is fine for the prototype — the reconciler is the right shape, and making it smarter is independent of the architecture.

4. **Character agents sometimes over-share emotional state.** The BENEATH and THINKING sections are well-structured, but the ACTION section occasionally includes emotional language that should stay internal. Better prompt constraints ("ACTION is only what a camera would see") would help.

5. **Context accumulation has no bounds.** By turn 5, the conversation history for each agent is substantial. The natural scene boundary (context window fills up) works but isn't graceful.

## Architecture

```
Scene Director (human)
    │
    ▼
Storykeeper (Rust function)
    │  produces per-character directives
    ├──────────────────┐
    ▼                  ▼
CharacterAgent     CharacterAgent
(Bramblehoof)      (Pyotir)
    │                  │
    │  tokio::join!    │
    ├──────────────────┘
    ▼
Reconciler (Rust function)
    │  sequences intents, notes dynamics
    ▼
NarratorAgent (LLM)
    │  renders literary prose
    ▼
Scene Director reads output
```

Each agent struct holds: system prompt, conversation history, Arc\<dyn LlmProvider\>.

## What Was Built

### storyteller-engine/src/agents/

| File | Lines | What |
|------|-------|------|
| `character.rs` | ~560 | CharacterAgent struct, tensor→NL, emotional state→NL, self-edge→NL, triggers→NL, system prompt builder, ACTION/BENEATH/THINKING response parser, 8 unit tests |
| `narrator.rs` | ~245 | NarratorAgent struct, literary voice system prompt, scene opening + turn rendering, 1 unit test |
| `storykeeper.rs` | ~160 | Deterministic `produce_directives()` — per-character filtering with scene-phase guidance from emotional arc, 4 unit tests |
| `reconciler.rs` | ~135 | Deterministic `reconcile()` — intent sequencing, dynamics detection (dialogue/silence/movement/contrasting subtext), 4 unit tests |
| `mod.rs` | ~17 | Re-exports CharacterAgent, NarratorAgent |

### storyteller-cli/src/bin/

| File | Lines | What |
|------|-------|------|
| `play_scene.rs` | ~110 | Interactive scene player binary. CLI args (--model, --temperature, --ollama-url). Loads workshop data, creates agents, runs turn loop with parallel deliberation. |

### Other

| File | What |
|------|------|
| `storyteller-cli/Cargo.toml` | Added `[[bin]]` for play-scene, added anyhow dependency |
| `Brewfile` | Development dependencies starting with ollama |

## Test Results

32 tests passing (9 new), 0 failures, 1 ignored (Ollama integration).

- Character agent: response parsing (well-formatted, partial, unformatted), tensor→NL, emotional state→NL, self-edge→NL, triggers→NL, system prompt construction
- Narrator: system prompt construction
- Storykeeper: directive count, character name filtering, narration context, guidance progression
- Reconciler: empty, single, two-character sequencing, contrasting subtext detection

## Key Design Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| Agent structure | Structs with async methods, not trait hierarchy | Need conversation history across turns. Natural migration path to Bevy resources. |
| Storykeeper | Rust function, not LLM | Information boundaries are compile-time known for hardcoded scenes. Deterministic. |
| Reconciler | Rust function, not LLM | Two-character sequencing is trivial. Just ordering + dynamics note. |
| Bevy | Skipped for now | Plain tokio async binary. Migrate to ECS after validating agent behavior. |
| Response format | ACTION/BENEATH/THINKING | Three-layer structure lets narrator see subtext without stating it. Parser falls back gracefully. |

## What Comes Next

1. **Prompt iteration** — fix narrator repetition, tighten character ACTION constraints, test with larger models
2. **Evaluation harness** — automated scene runs with scoring against the evaluation criteria
3. **Bevy migration** — move agents to ECS resources, turn cycle to Bevy systems
4. **Context management** — sliding window or summarization for long scenes
5. **Richer reconciler** — LLM-based for larger casts, or at least better heuristics
