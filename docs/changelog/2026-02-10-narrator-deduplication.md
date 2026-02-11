# Narrator Output Deduplication — One-Shot Architecture

**Date**: 2026-02-10
**Phase**: Phase D.4 — Narrator Output Deduplication
**Branch**: `jcoletaylor/tas-235-d4-narrator-output-deduplication`
**Linear**: TAS-235

## What Happened

We investigated narrator output duplication — the tendency for the LLM narrator to re-render content the player has already read — and discovered the root cause was architectural, not prompt-level. The fix was small (removing conversation history accumulation) but the reasoning path was important: prompt engineering alone failed completely, and only a structural change to one-shot rendering solved the problem.

Along the way we switched the baseline model from Mistral 7B to Qwen 2.5 14B, added scripted evaluation support to the play-scene binary, and tightened the narrator's prompt engineering for word count compliance and scope control.

## The Problem

The narrator was re-rendering the entire scene from the opening on every turn. By turn 5, the output was 13,281 tokens — a verbatim copy of all prior turns stacked on top of any new content. Token usage escalated linearly with turn count, and the scene never advanced.

## Root Cause: Conversation History Feedback Loop

`NarratorAgent` maintained a `history: Vec<Message>` that accumulated all prior user messages and assistant responses. Each turn, the LLM received:

1. Its own previous outputs (in conversation history)
2. Compressed summaries of those outputs (in the journal section of the user message)

The model treated both sources as material to render, creating a feedback loop where every turn repeated everything before it.

## The Fix: One-Shot Rendering

**Design principle already documented**: "Narrator doesn't remember — system remembers and provides on demand."

The fix aligns the implementation with the principle. Each narrator turn is now a one-shot LLM call: system prompt + single user message. The three-tier context assembly provides all continuity through the journal. The narrator never sees its own previous raw output.

The `history` field was removed from `NarratorAgent`. Both `render()` and `render_opening()` changed from `&mut self` to `&self`.

## Prompt Engineering (Second Pass)

With the structural fix in place, prompt engineering became effective:

| Change | Purpose |
|--------|---------|
| "Already Presented" section in system prompt | Explicitly frames journal as continuity reference, not material to re-render |
| Journal header changed to "Already Presented to the Player" | Reinforces the framing in the user message |
| "Scope" section in system prompt | "Render ONLY this moment. Do not resolve the scene." |
| Anti-pattern: "Inventing goodbyes, departures, or scene resolutions" | Prevents the model from writing scene endings |
| Anti-pattern: "Telling the reader what characters think, feel, or realize" | Strengthened from "feel" to "think, feel, or realize" |
| Word limit tightened to 200 words | Down from 300, with `max_tokens` reduced from 800 to 400 |
| Turn message closing: "Render ONLY this moment. Do not resolve the scene." | Reinforces scope at the end of each turn |

## Model Comparison

Evaluated Qwen 2.5 14B and Mistral 7B across three axes:

### Before fix (conversation history, Qwen 2.5 14B)

| Turn | Tokens | Behavior |
|------|--------|----------|
| 1 | 2,215 | Full scene re-render |
| 5 | 13,281 | 5x verbatim copy of entire scene |

### After fix (one-shot + tightened prompts)

| Metric | Qwen 2.5 14B | Mistral 7B |
|--------|-------------|------------|
| Total time (5 turns) | 122s | 74s |
| Turn 5 tokens | 2,613 | 2,981 |
| POV consistency | Third person throughout | Third person (improved) |
| Word count compliance | Mostly under 200 | Drifts to ~200 |
| Scene-ending tendency | Eliminated | Reduced but present |
| Emotional telling | Mostly shows through gesture | Still tells ("eyes glistening with unshed tears") |
| Deduplication | Clean | Clean |

**Verdict**: Qwen 2.5 14B is the better narrator baseline — stronger instruction following, consistent POV, better emotional restraint. Mistral 7B is 40% faster but lower quality on the axes that matter.

## What Was Changed

### storyteller-engine

- **`agents/narrator.rs`** — Removed `history: Vec<Message>` field. Both `render()` and `render_opening()` are now one-shot (`&self`, not `&mut self`). Added "Already Presented" and "Scope" sections to system prompt. Tightened word limit to 200. Reduced `max_tokens` from 800 to 400.
- **`context/preamble.rs`** — Added anti-pattern for inventing scene resolutions. Strengthened emotional telling anti-pattern.
- **`systems/rendering.rs`** — Removed `mut` from agent lock (follows from `&self` change).
- **`inference/external.rs`** — Default model changed from `mistral` to `qwen2.5:14b`.

### storyteller-cli

- **`bin/play_scene_context.rs`** — Added `--inputs` flag for scripted evaluation sessions. Default model changed to `qwen2.5:14b`. Reads player inputs from a file (one per line) instead of stdin, echoing each input for visibility.

### Configuration

- **`config/storyteller/base/storyteller.toml`** — Default model updated to `qwen2.5:14b`.

## Key Learnings

1. **Prompt engineering alone failed for deduplication.** The "Already Presented" framing had zero observable effect when conversation history was present. The structural fix (one-shot) was necessary first; prompt engineering then became effective as a refinement.

2. **The one-shot architecture aligns with the design principle.** "Narrator doesn't remember" was already documented but not implemented. The three-tier context assembly exists precisely to be the narrator's externalized memory.

3. **Word count compliance requires both prompt and mechanical constraint.** "Under 200 words" in the prompt plus `max_tokens: 400` together produce compliant output. Neither alone was sufficient.

4. **Scripted evaluation is essential for iteration.** The `--inputs` flag paid for itself immediately — five evaluation runs in one session, each producing directly comparable output.

5. **Model selection matters less than architecture.** Both Qwen 2.5 14B and Mistral 7B produced clean, non-duplicating output with the one-shot fix. The structural change solved the problem; model choice is a quality refinement on top.

## Deferred

- **Falcon-H1R-7B**: Not available in Ollama registry (Mamba2 hybrid architecture needs custom llama.cpp fork). Intended for training data enrichment, not narrator use.
- **Semantic deduplication**: The one-shot fix eliminates verbatim re-rendering. Thematic repetition (recurring phrases across turns) remains but is a lower-priority model-level issue that will improve with better models.
- **Dynamic word budget**: Currently hardcoded at 200 words / 400 tokens. Production should scale with scene complexity and turn density.
