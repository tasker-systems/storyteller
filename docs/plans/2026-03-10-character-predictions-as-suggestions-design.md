# Character Predictions as Intent Statements + Session Reload

**Date:** 2026-03-10
**Branch:** `jcoletaylor/character-predictions-as-suggestions`
**Status:** Design

## Problem Statement

### NPC Agency

ML character predictions reach the narrator as templated markdown (e.g., "Action (0.85): Approaches — driven by shared history, with warmth"). The narrator treats these as optional background context. Non-player characters are rendered tonally appropriate but lack agency — they react with body language and incidental actions but rarely speak proactively, initiate meaningful action, or drive the scene. They function as "person-shaped scenery with emotions."

Root causes identified:

1. **Predictions are informational, not directive.** The narrator system prompt says "You receive structured facts about what characters did, said, and felt" — framing predictions as things that already happened rather than things that should happen.
2. **Enrichment templates produce weak language.** Templated descriptions like "Observes — driven by shared history" carry no narrative weight or urgency.
3. **Character tensor data doesn't reach the narrator.** The preamble includes only a `voice_note` string per character. The rich tensor data (personality axes, emotional state, relationship edges) feeds the ML model but the narrator never sees who the character *is* — only what the model predicts they'll do.
4. **Static preamble conflicts with dynamic state.** The narrator-centric architecture reconstructs context fresh each turn (no chat history), but the preamble's characterization is frozen at scene start. As characters evolve through the scene, static descriptions become stale or contradictory to ML predictions.

### Session Reload

The workshop UI supports loading from a saved session, but the persisted data in `.story/sessions/` isn't rich enough to restore game state. The `events.jsonl` file captures per-turn event classification data but is missing character predictions, context assembly state, and the opening narration. Additionally, `events.jsonl` has become overloaded — it started as event classification output and accumulated turn-level metadata that doesn't belong there.

## Design

### Workstream 1: Intent Synthesis Pipeline

Insert an **intent synthesizer** (3b-instruct model, same as event decomposition) between ML predictions and narrator rendering. It converts structured prediction data into natural language behavioral directives that tell the narrator what each character wants to do and why.

#### Pipeline (per turn)

```
Player Input
    |
[Parallel]
|-- Event Decomposition (3b-instruct, existing)
|-- ML Character Prediction (ONNX, existing, ~50ms)
|       |
|   Intent Synthesizer (3b-instruct, NEW, ~2-5s)
|       Input: predictions + character tensor summary + journal tail + scene context
|       Output: per-character intent statements (~150-300 tokens total)
'-- Action Arbitration (existing)
    |
Context Assembly (CHANGED)
    |-- Tier 1 Preamble: voice, style, boundaries only (SLIMMED)
    |-- Tier 2 Journal: rolling history (unchanged)
    |-- Tier 3 Retrieved: on-demand facts (unchanged)
    |-- Intent Statements: synthesized character directives (NEW, replaces "## Character Predictions")
    '-- This Turn: player input + scene dynamics
    |
Narrator LLM
```

#### Intent Synthesizer

**Model:** Same 3b-instruct (qwen2.5:3b-instruct via Ollama) used for event decomposition.

**Input context:**
- Each character's name, role, current emotional state (from tensor), key personality axes
- ML prediction output (action type, speech prediction, emotional deltas, confidence)
- Last 1-2 journal entries (what just happened in the scene)
- Player's current input
- Scene stakes and constraints

**Output format:** One paragraph per non-player character describing:
- What they want to do this turn and why
- Speech intent when predicted (direction, not dialogue)
- Emotional subtext the narrator can show through physicality
- Relationship dynamics relevant to this moment

**Example output:**

> **Margaret** has just asked Arthur a direct question. She's warm but worried — she can see he's retreating into himself. She waits for his answer, her hand still on his.
>
> **Arthur** is guarded but beginning to thaw. Margaret's familiarity is disarming him despite himself. He should respond to her question — reluctantly, deflecting with Shaw rather than answering directly. His body language softens even as his words stay careful.

**Key constraint:** The intent synthesizer is *directive but not prescriptive*. It says "Arthur should respond reluctantly, deflecting" not "Arthur says 'Well, Shaw would argue...'" — the narrator owns the actual prose and dialogue.

**Output format:** Freeform markdown, not structured JSON. The output is consumed as-is by the narrator — no parsing required. The intent synthesizer produces prose that reads naturally as a briefing. Each character section is headed by `**CharacterName**` in bold. The entire output string is stored in `ResolverOutput.intent_statements: Option<String>` (new field) and rendered directly into the narrator's user message under `## Character Intents`.

**System prompt (draft):**

```
You are the Intent Synthesizer — a dramaturgical assistant preparing a briefing for a narrator.

You receive:
- Character data: personality traits, emotional state, relationships
- ML predictions: what a behavior model predicts each character will do
- Recent scene history: what just happened
- Player input: what the player character just did or said

Your job: Write a brief directive for each non-player character describing what they WANT to do this turn and WHY.

Rules:
- Be directive: "Arthur should respond" not "Arthur might respond"
- Be specific about emotional subtext: "reluctantly, deflecting with humor" not "with some emotion"
- Include speech direction when a character should speak: "should say something about..." not prescribing exact words
- Ground in physical behavior: "his shoulders drop" not "he feels sad"
- One paragraph per character, 2-4 sentences each
- Do NOT write dialogue. The narrator writes all dialogue.
- Do NOT narrate the scene. You are briefing the narrator, not writing prose.

Format each character section as:
**CharacterName** directive paragraph here.
```

**Graceful degradation:** If the intent synthesis LLM call fails (timeout, connection error, malformed output), fall back to the existing `render_predictions()` markdown output. The narrator can still work with raw predictions — it's less directive but functional. Log a warning when fallback activates.

**Code location:** `crates/storyteller-engine/src/inference/intent_synthesis.rs` — parallel to `inference/event_decomposition.rs`. Uses the same `OllamaStructuredProvider` (or a simpler non-structured Ollama call since output is freeform).

**Latency:** Intent synthesis starts as soon as ML predictions return and runs in parallel with event decomposition. The 3b-instruct model is already loaded for decomposition. Expected added latency: minimal, overlapping with existing async work.

**Token budget impact:** Intent statements replace the `## Character Predictions` section (~150-300 tokens) with `## Character Intents` (~150-300 tokens). Net token change is approximately zero. The preamble is not slimmed in this iteration — voice notes are already compact. `DEFAULT_TOTAL_TOKEN_BUDGET` does not need adjustment.

#### Narrator Prompt Changes

**System prompt:**
- Remove: "You receive structured facts about what characters did, said, and felt."
- Add: Instruction that characters are agents with their own drives. The narrator receives intent statements describing what each character wants to do. Honor these intents — render them with the character's full agency. Characters act, speak, and drive the scene.
- Preamble cast section remains: name + role + voice note (stable reference for the narrator's rendering style).

**User message:**
- Remove: `## Character Predictions` markdown section with raw prediction rendering
- Add: `## Character Intents` section containing the synthesized intent statements (from `ResolverOutput.intent_statements`)
- All other sections unchanged (journal, retrieved context, this turn)

**Data path:** `ResolverOutput` gains a new field `intent_statements: Option<String>`. In `build_turn_message()` (narrator.rs), when `intent_statements` is `Some`, render it under `## Character Intents`. When `None` (fallback), render `original_predictions` via `render_predictions()` as today. This keeps backward compatibility and supports graceful degradation.

#### Preamble Slimming

The preamble no longer needs to carry characterization weight — the intent synthesizer handles that dynamically each turn. The preamble retains:
- Narrator voice and style directives
- Anti-patterns
- Setting description
- Cast list (name, role, voice note only — for consistent rendering style)
- Hard boundaries

This keeps the preamble truly stable across the scene and avoids the static-vs-dynamic conflict.

### Workstream 2: turns.jsonl + Session Reload

#### New Persistence Format

Replace `events.jsonl` with `turns.jsonl` — a complete turn-by-turn record indexed from turn 0 (opening narration).

```
.story/sessions/{uuidv7}/
|-- scene-selections.json    # Wizard choices (unchanged)
|-- scene.json               # Composed scene (unchanged)
|-- characters.json          # Character tensor data (unchanged)
'-- turns.jsonl              # Complete turn-by-turn record (NEW, replaces events.jsonl)
```

#### Turn Record Schema

**Turn 0 (opening narration):**
```json
{
  "turn": 0,
  "player_input": null,
  "narrator_output": "Opening narration prose...",
  "predictions": [],
  "intent_statements": null,
  "classifications": null,
  "decomposition": null,
  "arbitration": null,
  "context_assembly": {
    "preamble_tokens": 232,
    "journal_entries": [],
    "retrieved_context": [],
    "total_tokens": 232
  },
  "timing": {
    "prediction_ms": 0,
    "intent_ms": 0,
    "assembly_ms": 0,
    "narrator_ms": 5200
  },
  "timestamp": "2026-03-10T21:05:00.000Z"
}
```

**Turn N (player turn):**
```json
{
  "turn": 1,
  "player_input": "Margaret looks at the boy...",
  "narrator_output": "Margaret's gaze softens...",
  "predictions": [
    {
      "character_id": "...",
      "character_name": "Arthur",
      "frame": { "activated_axes": ["empathy", "guardedness"], "confidence": 0.80 },
      "actions": [{ "description": "...", "confidence": 0.85, "action_type": "Approach" }],
      "speech": { "content_direction": "...", "register": "Conversational", "confidence": 0.70 },
      "thought": { "emotional_subtext": "...", "awareness_level": "Recognizable" },
      "emotional_deltas": [{ "primary_id": "joy", "intensity_change": 0.2 }]
    }
  ],
  "intent_statements": "Arthur is guarded but beginning to thaw...",
  "classifications": ["SpeechAct: 0.62"],
  "decomposition": {
    "entities": [...],
    "events": [...]
  },
  "arbitration": { "conditions": [], "verdict": "Permitted" },
  "context_assembly": {
    "preamble_tokens": 232,
    "journal_entries": ["Turn 0 narrator output summary..."],
    "retrieved_context": ["Bramblehoof backstory: ..."],
    "total_tokens": 545
  },
  "timing": {
    "prediction_ms": 56,
    "intent_ms": 2300,
    "decomposition_ms": 3100,
    "assembly_ms": 5,
    "narrator_ms": 11832
  },
  "timestamp": "2026-03-10T21:07:15.641Z"
}
```

#### Turn 0 Write Location

The opening narration is generated in `setup_and_render_opening()` (commands.rs). This function is called by both `compose_scene` and `resume_session`. After generating the opening narration, `setup_and_render_opening()` writes turn 0 to `turns.jsonl` before returning `SceneInfo` to the frontend. This ensures every session has a turn 0 record.

#### Session Reload Process

1. **Read session files:** `scene.json` + `characters.json` + `scene-selections.json`
2. **Read turns.jsonl:** Collect all turn records
3. **Hydrate chat UI:** Send `(player_input, narrator_output)` pairs from all turns to the frontend
4. **Reconstruct journal:** Collect `narrator_output` from all turns in order, apply existing truncation/budget logic
5. **Set turn counter:** Last turn number + 1
6. **Ready for next input:** ML predictions, intent synthesis, and context assembly all run fresh on the next turn

#### `resume_session` Return Type

The current `resume_session` command returns `SceneInfo` and re-renders the opening from scratch. This changes:

- **New return type:** A struct containing `SceneInfo` + `Vec<TurnRecord>` (all prior turns). The frontend receives the full conversation history and populates the chat view without re-rendering.
- **No opening re-render on resume:** The opening narration is already in turn 0 of `turns.jsonl`. The frontend displays it from the persisted data.
- **Journal reconstruction:** The engine rebuilds the journal from persisted `narrator_output` fields, applying the existing truncation budget. The engine is then ready to accept new player input immediately.
- **Frontend contract:** The frontend receives an array of `{ turn, player_input, narrator_output }` objects and renders them as the conversation history. Debug tab data from prior turns is not sent — it remains available in `turns.jsonl` for future inspection features.

**What doesn't need restoring:**
- Prior turns' debug data — historical, available in turns.jsonl for inspection but not loaded into memory
- Frozen context snapshots — context is always assembled fresh
- ML model state — stateless inference

**Key property:** On reload, the next turn picks up any code changes (prompt updates, ML model changes, enrichment logic) automatically since context assembly is always fresh. Narrative state (what happened) is restored; implementation state (how it was processed) is regenerated.

#### Migration

Existing sessions with `events.jsonl` are not migrated. New sessions write `turns.jsonl` only. The `resume_session` command checks for `turns.jsonl` first. If only `events.jsonl` exists (legacy session), the session is listed but resume loads a minimal view: chat history is reconstructed from `player_input` + `narrator_output` fields in the old format, but predictions and intent statements will be absent. This provides read-only backward compatibility without requiring migration.

## Scope Boundaries

**In scope:**
- Intent synthesizer implementation (new structured LLM call)
- Narrator prompt rewrite (system prompt + user message construction)
- Preamble slimming (remove characterization weight)
- turns.jsonl persistence (new file, new schema)
- Session reload from turns.jsonl
- Turn 0 persistence (opening narration)
- Workshop integration (commands.rs wiring)
- Debug inspector updates (Intent tab or updated predictions display)

**Out of scope:**
- ML model changes (predictions pipeline unchanged)
- Character tensor evolution across turns (future work)
- Journal summarization/decay logic changes (existing truncation applies)
- Multi-session continuity (each session is independent)
- Graph-based retrieved context (Tier 3 still manual/placeholder)

## Testing Strategy

- Unit tests for intent synthesizer prompt construction and response parsing
- Unit tests for turns.jsonl serialization/deserialization roundtrip
- Unit tests for session reload (journal reconstruction from turn records)
- Integration test with mock LLM verifying intent statements appear in narrator context
- Manual testing via workshop: verify NPC agency improvement in rendered prose
- Manual testing via workshop: compose scene, play 2-3 turns, quit, reload, continue
