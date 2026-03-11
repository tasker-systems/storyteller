# Dramaturgy of Tension

**Date:** 2026-03-10
**Branch:** `jcoletaylor/character-predictions-as-suggestions`
**Status:** Design
**Depends on:** Character Predictions as Intent Statements (completed on this branch)

## Problem Statement

The intent synthesis pipeline (just completed) converts ML character predictions into natural language behavioral directives for the narrator. This solves NPC passivity — NPCs now act with agency because the narrator receives explicit directives about what each character wants to do and why.

However, the pipeline has a blind spot: the player character.

### Current Behavior

1. The ML model predicts behavior for **all** characters, including the player's.
2. `build_summaries()` **filters out the player character** — the intent synthesizer never sees them.
3. The narrator receives `"Player: {input}"` under `## This Turn` but has no information about who this character *is* — their personality, emotional state, or how the directed action relates to their identity.
4. The preamble lists cast members with name, role, and voice note, but **does not identify which character the player controls**.

### Consequences

- The narrator renders player actions faithfully but generically — it has no tensor data to inflect the rendering with character identity.
- When a player directs an action that contradicts their character's nature (e.g., a cautious character charging recklessly), the narrator has no way to surface this tension. The action is rendered as-is.
- There is no system-level mechanism to nudge players toward authentic characterization without blocking their agency.
- The ML prediction for the player character is computed but discarded — wasted signal.

### Design Goal

**The player's action always proceeds, but the character's identity inflects the rendering.** The narrator shows the body betraying who the character *is* even as they do what the player *said*. When the player's action aligns with the character's nature, the narrator renders it with the specific quality that character would bring. When there is tension, the narrator lets the character's hesitation, instinct, or resistance show through the action physically — without blocking or subverting the player's intent, and without explaining the tension to the reader.

This is the "Option A" outcome: inflected execution. The system is designed to be extensible toward partial success/failure outcomes (Option B) when the resolver agent and world agent layers mature, but those are out of scope here.

## Design

### Approach: Unified Intent Synthesis

Extend the existing intent synthesizer to handle both NPC intents and player character tension in a single call. The instruct model receives *all* characters' data. NPCs get the current treatment (behavioral directives). The player character gets a tension/alignment note grounded in their tensor data and the ML prediction. Output is a unified block of `**CharacterName**` paragraphs. The narrator prompt gains instruction on how to use player character paragraphs.

This was chosen over a separate tension synthesis call (adds latency and pipeline complexity) and a narrator-only approach (puts too much analytical burden on the narrator alongside its creative work).

### 1. Intent Synthesis Changes

#### System Prompt

The intent synthesizer system prompt gains a new rule block for the player character. The existing NPC rules are unchanged.

Current job description:
> Your job: Write a brief directive for each non-player character describing what they WANT to do this turn and WHY.

Updated to:
> Your job: Write a brief directive for each character. For non-player characters, describe what they WANT to do this turn and WHY. For the player character, describe how the character's nature relates to the directed action.

New rules appended:

```
For the player character (marked [PLAYER CHARACTER]):
- The player has directed this character's action. Do NOT override it.
- Describe how this character's personality and emotional state relate to
  the directed action — whether their nature resists it, inflects it, or
  suits it. Ground in physical behavior the narrator can render.
- If the directed action is in tension with the character's nature,
  call this out explicitly — this is how the system nudges players
  toward authentic characterization without forcing their hand.
- This is advisory — the narrator decides how to weigh it.
```

#### `build_summaries()` Changes

Stop filtering the player character. Instead, produce a different summary format:

**NPC summary** (unchanged):
```
Arthur | grieving_youth | Dominant traits (0-1 scale): guarded 0.82, empathetic 0.65 | Voice: Measured, clipped
```

**Player character summary** (new):
```
[PLAYER CHARACTER — directed action: "I charge at the intruder"]
Arthur | grieving_youth | Dominant traits (0-1 scale): guarded 0.82, conflict-averse 0.71, empathetic 0.65
ML prediction: would most likely observe (85% confidence), unlikely to charge (15%)
```

Key formatting decisions for the 3b-instruct model:
- Use plain English trait names, not axis IDs (`conflict-averse` not `conflict_aversion`)
- Label the scale explicitly: `(0-1 scale)` appears once per summary block
- ML predictions in natural language: `would most likely observe (85% confidence)` not `Action=Observe(0.85)`
- Include top 3-5 tensor axes by magnitude (central tendency value)
- The `[PLAYER CHARACTER]` marker and directed action quote let the instruct model distinguish roles

**Signature change:**
```rust
pub fn build_summaries(
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    player_entity_id: Option<EntityId>,
    player_input: Option<&str>,  // NEW — needed for player character summary
) -> (String, String)
```

#### Output Format

The instruct model produces `**CharacterName**` paragraphs as today. The player character paragraph appears naturally among NPC paragraphs. Examples:

**When tension exists:**
> **Arthur** has been directed to charge at the intruder, but his deep guardedness resists this — he is someone who watches, assesses, finds exits before acting. Render the charge through his body's reluctance: his grip wrong on whatever he grabs, his stride breaking before it commits, the mechanical quality of someone forcing himself past instinct.

**When aligned:**
> **Arthur** examines the room with the deliberate caution that defines him. His guardedness serves him here — he checks exits, notes distances, catalogues what could be a weapon. Render his attention as precise, methodical, unhurried.

### 2. Narrator Prompt Changes

#### Preamble Cast Section

`CastDescription` gains an `is_player: bool` field. `render_preamble()` uses this to mark the player character:

```
### Arthur — protagonist (player)
Voice: Measured, clipped sentences...

### Margaret — deuteragonist
Voice: Warm, flowing...
```

This gives the narrator a stable reference for who the player controls across all turns.

#### System Prompt Addition

One paragraph added after the existing intent statement instruction:

```
When the player character's paragraph appears in the intent statements,
it describes how the character's nature relates to the player's directed
action. The player's action is what happens — but render it through who
the character is. If friction is noted, let the character's body,
hesitation, or instinct show through the action. Do not block or
subvert the player's intent. Do not explain the tension to the reader.
Show it physically.
```

#### User Message

No structural change. The `## Character Intents` section already includes all paragraphs from the intent synthesizer. The `Player: {input}` line under `## This Turn` remains as the authoritative statement of what happens.

**Key principle:** The narrator has two sources of truth for the player character — the player's input (what happens) and the intent paragraph (how it's rendered). The player input is never overridden.

### 3. Pipeline Data Flow

```
Player Input
    |
[Parallel]
|-- Event Decomposition (3b-instruct, existing)
|-- ML Character Prediction (ONNX, existing, ~50ms)
|       |  (predicts ALL characters including player)
|   Intent Synthesizer (3b-instruct, CHANGED)
|       Input: NPC data + predictions (as today)
|              + player character data + prediction + tensor axes + player input (NEW)
|       Output: per-character paragraphs — NPC intents + player tension/alignment note
'-- Action Arbitration (existing, unchanged)
    |
Context Assembly (unchanged structure)
    |-- Tier 1 Preamble: voice, style, boundaries, cast with (player) marker (CHANGED)
    |-- Tier 2 Journal: rolling history (unchanged)
    |-- Tier 3 Retrieved: on-demand facts (unchanged)
    |-- Character Intents: unified block including player paragraph (CHANGED content, same section)
    '-- This Turn: player input + scene dynamics (unchanged)
    |
Narrator LLM (system prompt CHANGED, user message structure unchanged)
```

Changes to `synthesize_intents()` signature:
```rust
pub async fn synthesize_intents(
    llm: &dyn LlmProvider,
    characters: &[&CharacterSheet],
    predictions: &[CharacterPrediction],
    journal_tail: &str,
    player_input: &str,
    scene: &SceneData,
    player_entity_id: Option<EntityId>,
) -> Option<String>
```

No signature change needed — `player_input` is already a parameter. It just needs to be passed through to `build_summaries()`.

### 4. `summarize_character()` Formatting

The existing `summarize_character()` function produces a summary line per character. It needs a new code path for readable trait formatting:

**New helper:** `format_dominant_axes(sheet: &CharacterSheet, count: usize) -> String`
- Iterates tensor axes, sorts by `central_tendency` descending
- Takes top `count` (default 3-5)
- Formats as `"guarded 0.82, conflict-averse 0.71, empathetic 0.65"`
- Uses a simple mapping from axis ID to readable English name (e.g., `guardedness` → `guarded`, `conflict_aversion` → `conflict-averse`)

**New helper:** `format_prediction_readable(prediction: &CharacterPrediction) -> String`
- Converts structured prediction to natural language
- e.g., `"would most likely observe (85% confidence), unlikely to charge (15%)"`
- Includes speech prediction if present: `"likely to speak (conversational register, 70% confidence)"`

These helpers are used only for the player character's summary in `build_summaries()`.

## Scope Boundaries

**In scope:**
- Intent synthesis system prompt update (player character rules)
- `build_summaries()` includes player character with readable trait/prediction summaries
- `summarize_character()` gains readable trait formatting for dominant axes
- New helpers: `format_dominant_axes()`, `format_prediction_readable()`
- `CastDescription` gains `is_player: bool`
- `render_preamble()` appends `(player)` marker
- Narrator system prompt gains tension-rendering paragraph
- Workshop `commands.rs` wiring to pass `player_input` through to intent synthesis
- Tests for new summary format, player character inclusion, preamble player marker

**Out of scope:**
- Resolver agent / world agent integration (future — extends toward Option B outcomes)
- Event ledger edge-weight changes from tension (character evolution through graph cascades is an end-of-scene concern)
- Progress bar / pipeline stage animation in workshop UI (noted for future design)
- Changes to ML prediction pipeline (already computes all characters)
- Changes to arbitration (remains a physics/genre gate, not a dramaturgy gate)
- Changes to event decomposition or classification
- Token budget adjustments (player character paragraph is ~50-100 tokens, within existing headroom)

**Graceful degradation:** If intent synthesis fails (timeout, malformed output), the existing fallback to `render_predictions()` activates. The player character's prediction would be absent from the fallback rendering (filtered as today). This is acceptable — tension surfacing is an enhancement, not a gate.

## Testing Strategy

- Unit tests for `format_dominant_axes()` (sorting, readable names, count limiting)
- Unit tests for `format_prediction_readable()` (action types, speech, confidence formatting)
- Unit test for `build_summaries()` with player character included (verify marker, traits, prediction in output)
- Unit test for `build_summaries()` with `player_entity_id = None` (backward compat — all characters treated as NPCs)
- Unit test for `render_preamble()` with `is_player` flag (verify `(player)` marker in output)
- Unit test for updated system prompt (verify player character rules present)
- Integration test with mock LLM verifying player character paragraph appears in intent output
- Manual testing via workshop: play a scene, direct actions that align with and diverge from the player character's tensor, verify narrator rendering reflects character identity

## Future Extensions

This design is the foundation for richer dramaturgy as the system matures:

- **Resolver agent** can use tension signals to produce partial success/failure outcomes (Option B), where character identity meaningfully constrains action success
- **World agent** can integrate tension with spatial/physics rules for compound constraints (the character's nature *and* the environment resist the action)
- **Event ledger aggregates** can track tension frequency per character — repeated actions against type could trigger tensor drift at scene boundaries through graph edge-weight cascades
- **Workshop progress bar** can surface pipeline stages with dramaturgical flavor text during the turn cycle
