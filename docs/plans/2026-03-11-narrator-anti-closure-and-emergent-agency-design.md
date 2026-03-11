# Narrator Anti-Closure & Emergent Agency Discovery

## Status: Design

## Context

After implementing the dramaturgy-of-tension feature (intent synthesis with player character tension notes, narrator prompt changes, `is_player` cascade through preamble), a live gameplay session with a wizard-composed scene (Margaret and Arthur at The Old Rectory) revealed two findings:

1. The 14b narrator model produces **tonal closure** at the end of each passage — poetic summing-up language that makes each turn read like a scene ending, undermining the mid-scene continuity the system depends on.
2. NPC characters **resist player-directed actions** based on their tensor state, without any explicit resistance mechanic. This is emergent behavior worth documenting and potentially reinforcing.

## Part 1: Narrator Anti-Closure Prompt Changes

### Problem

The narrator's passages end with resolution language:
- "leaving behind only the memory of its presence"
- "a mix of resignation and gratitude that hangs unspoken between them"

These are not invented events (anti-pattern #6 catches that). They are **tonal closure** — the model wrapping each passage with poetic finality. This creates two problems:
- Each passage reads as a scene ending, breaking the sense of continuous play
- Combined with the ~200 word limit, passages sometimes truncate mid-sentence because the model spent tokens on summative language instead of advancing action

### Changes

**1. New anti-pattern** in `build_preamble()` (preamble.rs, anti_patterns vec):

```
"Ending on a note of resolution, summary, or poetic reflection — each passage is a mid-scene cut, not a conclusion"
```

**2. Rewritten `## Scope` section** in `build_system_prompt()` (narrator.rs):

Replace the current Scope section with:

```
## Scope
Render ONLY the actions and events described in "This Turn." Do not
invent departures, goodbyes, or scene resolutions. Do not write beyond
the moment.

End mid-moment. Your last sentence should feel like the next thing is
already happening — a gesture half-completed, a word hanging in the air,
a gaze that hasn't yet been returned. The camera holds; it does not fade.
The scene continues after your passage ends.

Write in present tense, third person. HARD LIMIT: under 200 words.
```

The key addition is positive instruction: not just "don't close the scene" but "here's how to end — mid-moment, with continuation implied."

### Files

- Modify: `crates/storyteller-engine/src/context/preamble.rs` (anti_patterns vec)
- Modify: `crates/storyteller-engine/src/agents/narrator.rs` (build_system_prompt Scope section)

### Testing

- Update existing narrator tests to assert on new scope language
- Add test: `system_prompt_has_anti_closure_instruction` verifying "mid-moment" and "camera holds" appear
- Add test: anti-pattern list includes "resolution" or "poetic reflection"

## Part 2: Emergent Agency Discovery Report

### What

Write a discovery report to `docs/emergent/agency-from-data.md` documenting the finding that NPC characters exhibit resistance or facilitation behaviors based on their tensor state, without any explicit resistance mechanic.

### Why

This behavior emerged from the intent synthesis pipeline during the first live gameplay session after the dramaturgy-of-tension implementation. If it proves to be load-bearing for character agency, it should be formalized. If it's fragile, we need to know that too. The document captures the evidence so the decision can be made later.

### Document Structure

1. **Summary** — NPC characters resist or facilitate player-directed actions based on their tensor state. This emerges from rich data flowing through the intent synthesis pipeline, not from any explicit resistance mechanic.

2. **The Observation** — Gameplay transcript excerpts from the Margaret/Arthur session. Margaret (player) directs Arthur to "come here" and "tell me what you see." Arthur's tensor (distance_management 0.82, expression_silence, defended awareness) creates physical resistance in the rendering — hesitation, measured steps, gaze avoidance.

3. **Why It Happens** — The chain:
   - Character tensor axes with high values on guarded/distant traits
   - ML prediction: "observe with restraint" (Examine action type)
   - Intent synthesizer reads prediction + traits + player action
   - Produces directive grounding Arthur's response in his character state
   - Narrator renders the directive physically
   - No single component "decides" to resist — resistance is an emergent property

4. **The Asymmetry** — NPC resistance is rendered neutrally (it's just who they are). Player-character tension with their own nature is highlighted through the dramaturgy-of-tension narrator paragraph. This asymmetry is intentional: NPCs have agency, the player has awareness.

5. **Open Questions** — Is this load-bearing? Should we formalize NPC resistance (e.g., tensor-derived compliance thresholds)? Does formalizing risk making it mechanical? Does facilitation emerge as visibly as resistance when player actions align with NPC nature?

### Tone

Discovery report, not specification. Evidence-based. The transcript excerpts serve as primary evidence. Written so a future reader can assess whether to reinforce or leave as emergent.

### Source Data

Gameplay session: `crates/storyteller-workshop/.story/sessions/019cdcac-f841-78a1-abf0-3057754871dc/turns.jsonl` (turns 1-3, Margaret/Arthur at The Old Rectory)

## Scope Boundaries

- No changes to intent synthesis (the emergent behavior works; don't fix what isn't broken)
- No changes to ML prediction pipeline (static predictions are a separate concern — Spec 2)
- No changes to the 3b-instruct model selection or prompting (parroting is a known issue, not addressed here)
- The discovery report is documentation only — no code changes to "support" emergent agency
