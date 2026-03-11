# Narrator Anti-Closure & Emergent Agency Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix narrator scene-closing language with positive mid-moment ending instruction, and document the emergent NPC agency discovery from gameplay.

**Architecture:** Two independent changes: (1) targeted prompt edits to narrator anti-patterns and scope section, (2) a new discovery report document in `docs/emergent/`. No structural code changes.

**Tech Stack:** Rust (storyteller-engine crate), Markdown documentation.

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/storyteller-engine/src/context/preamble.rs` | Modify | Add anti-closure anti-pattern |
| `crates/storyteller-engine/src/agents/narrator.rs` | Modify | Rewrite Scope section with mid-moment ending instruction |
| `docs/emergent/agency-from-data.md` | Create | Discovery report on emergent NPC resistance |

---

## Chunk 1: Narrator Anti-Closure Prompt Changes

### Task 1: Add anti-closure anti-pattern to preamble

**Files:**
- Modify: `crates/storyteller-engine/src/context/preamble.rs:43-51`

- [ ] **Step 1: Write the failing test**

Add to the test module in `preamble.rs`:

```rust
#[test]
fn anti_patterns_include_closure_guidance() {
    let scene = crate::workshop::the_flute_kept::scene();
    let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
    let pyotir = crate::workshop::the_flute_kept::pyotir();
    let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

    let observer = storyteller_core::traits::NoopObserver;
    let preamble = build_preamble(&scene, &characters, &observer, None);

    let has_closure_pattern = preamble
        .anti_patterns
        .iter()
        .any(|ap| ap.contains("resolution") && ap.contains("mid-scene"));
    assert!(
        has_closure_pattern,
        "Anti-patterns should include closure guidance: {:?}",
        preamble.anti_patterns
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine anti_patterns_include_closure --all-features`
Expected: FAIL — no anti-pattern contains both "resolution" and "mid-scene"

- [ ] **Step 3: Add the anti-pattern**

In `preamble.rs` line 50, after `"Breaking the fourth wall".to_string(),` add:

```rust
        "Ending on a note of resolution, summary, or poetic reflection — each passage is a mid-scene cut, not a conclusion".to_string(),
```

The full `anti_patterns` vec becomes 8 items.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p storyteller-engine anti_patterns_include_closure --all-features`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-engine/src/context/preamble.rs
git commit -m "feat(engine): add anti-closure anti-pattern to narrator preamble"
```

### Task 2: Rewrite narrator Scope section with mid-moment ending instruction

**Files:**
- Modify: `crates/storyteller-engine/src/agents/narrator.rs:228-233`

- [ ] **Step 1: Write the failing test**

Add to the test module in `narrator.rs`:

```rust
#[test]
fn system_prompt_has_mid_moment_ending_instruction() {
    let context = mock_context();
    let prompt = build_system_prompt(&context);
    assert!(
        prompt.contains("End mid-moment"),
        "Should instruct mid-moment endings: {prompt}"
    );
    assert!(
        prompt.contains("camera holds"),
        "Should use camera metaphor: {prompt}"
    );
    assert!(
        prompt.contains("gesture half-completed"),
        "Should give positive examples of how to end: {prompt}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p storyteller-engine system_prompt_has_mid_moment --all-features`
Expected: FAIL — none of these phrases exist

- [ ] **Step 3: Replace the Scope section**

In `narrator.rs`, in `build_system_prompt()`, replace lines 228-233:

```rust
## Scope
Render ONLY the actions and events described in "This Turn." Do not
invent departures, goodbyes, or scene resolutions. Do not write beyond
the moment. The scene continues after your passage ends.

Write in present tense, third person. HARD LIMIT: under 200 words."#
```

With:

```rust
## Scope
Render ONLY the actions and events described in "This Turn." Do not
invent departures, goodbyes, or scene resolutions. Do not write beyond
the moment.

End mid-moment. Your last sentence should feel like the next thing is
already happening — a gesture half-completed, a word hanging in the air,
a gaze that hasn't yet been returned. The camera holds; it does not fade.
The scene continues after your passage ends.

Write in present tense, third person. HARD LIMIT: under 200 words."#
```

- [ ] **Step 4: Update existing test to assert on new scope language**

In `narrator.rs`, update `system_prompt_has_preamble` to also assert on the new mid-moment language:

```rust
#[test]
fn system_prompt_has_preamble() {
    let context = mock_context();
    let prompt = build_system_prompt(&context);
    assert!(prompt.contains("Your Voice"));
    assert!(prompt.contains("Never Do"));
    assert!(prompt.contains("The Scene"));
    assert!(prompt.contains("Bramblehoof"));
    assert!(prompt.contains("Pyotir"));
    assert!(prompt.contains("present tense"));
    assert!(prompt.contains("intent statements"));
    assert!(prompt.contains("End mid-moment"), "Should have mid-moment ending instruction");
}
```

- [ ] **Step 5: Run all narrator tests to verify nothing breaks**

Run: `cargo test -p storyteller-engine narrator --all-features`
Expected: All tests PASS. Verify specifically:
- `system_prompt_has_preamble` passes (updated with "End mid-moment" assertion)
- `system_prompt_includes_tension_rendering_instruction` still passes
- `system_prompt_has_mid_moment_ending_instruction` passes (new test)

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/agents/narrator.rs
git commit -m "feat(engine): rewrite narrator scope with mid-moment ending instruction"
```

### Task 3: Run full integration check

**Files:** None (verification only)

- [ ] **Step 1: Run workspace tests**

Run: `cargo test --workspace --all-features 2>&1 | tail -20`
Expected: All tests PASS (except pre-existing failures requiring STORYTELLER_DATA_PATH or Ollama)

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -10`
Expected: Clean

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --check`
Expected: Clean

---

## Chunk 2: Emergent Agency Discovery Report

### Task 4: Write the discovery report

**Files:**
- Create: `docs/emergent/agency-from-data.md`

- [ ] **Step 1: Create the directory and file**

First create the new directory:
```bash
mkdir -p docs/emergent
```

Create `docs/emergent/agency-from-data.md` with the following content. The transcript excerpts come from `crates/storyteller-workshop/.story/sessions/019cdcac-f841-78a1-abf0-3057754871dc/turns.jsonl` (turns 1-3).

```markdown
# Emergent Agency from Character Data

**Status:** Discovery report — observed in gameplay, not yet formalized
**Date:** 2026-03-11
**Session:** `019cdcac-f841-78a1-abf0-3057754871dc` (Margaret & Arthur, The Old Rectory)

## Summary

NPC characters resist or facilitate player-directed actions based on their
tensor state, without any explicit resistance mechanic in the codebase. This
behavior emerged from the intent synthesis pipeline during the first live
gameplay session after the dramaturgy-of-tension implementation. No single
component "decides" to resist — the resistance is an emergent property of
rich character data flowing through a pipeline that was designed for something
else (behavioral directives for the narrator).

## The Observation

In a scene at The Old Rectory, the player controls Margaret — a warm,
empathetic character who is comforting Arthur, a guarded young man grieving
his father. Over three turns, a pattern emerged:

**Turn 1:** Margaret places a hand on Arthur's shoulder and speaks gently.
The intent synthesizer directs Arthur to "sit quietly... resists the urge
to respond with emotion, instead choosing to remain distant and guarded."
The narrator renders this as physical restraint — Arthur's back straightens,
his eyes flicker up then down, his chest rises and falls more rapidly.

**Turn 2:** Margaret says "Come here, Arthur" and pulls back a curtain.
The intent synthesizer directs Arthur to resist: "His shoulders drop, a
subtle gesture that speaks volumes about his internal struggle." The
narrator renders hesitation, measured steps, gaze fixed on the pane rather
than Margaret's face.

**Turn 3:** Margaret says "Tell me what you see." The intent synthesizer
produces a `[PLAYER CHARACTER]` tension note about "Arthur's nature resists
Margaret's request, but his longing for connection is strong enough to keep
him from completely shutting her out." The narrator renders Arthur's response
as a slow, reluctant opening — whispered words, fingers curling around the
window frame for support.

In each turn, Arthur's response was not scripted, not rule-based, and not
explicitly requested by the player. It emerged from his character data
meeting the player's directed actions.

## Why It Happens

The chain that produces emergent resistance:

1. **Character tensor** — Arthur's axes include `distance_management`,
   `expression_silence`, `attachment_security`, and `trust_emotional`, all
   with values reflecting a guarded, defended personality. His awareness
   level is `Defended` ("Arthur deflects from fear").

2. **ML prediction** — Given the classified player input (tender speech),
   the model predicts Arthur will `Examine` (observe with restraint) at
   ~0.70 confidence. Not speak. Not act. Observe.

3. **Intent synthesizer** — The 3b-instruct model receives Arthur's tensor
   data (formatted as readable traits: "guarded 0.82"), the ML prediction,
   the player's direct action ("Come here, Arthur"), and recent scene
   history. It synthesizes a directive that grounds Arthur's response in
   his character state — physical resistance expressed through body language.

4. **Narrator** — The 14b narrator model receives the directive and renders
   it as literary prose. Hesitation, measured steps, averted gaze. The
   resistance becomes physical, observable, and meaningful.

No component in this chain was designed to produce resistance. The ML model
predicts behavior from tensor data. The intent synthesizer translates
predictions into directives. The narrator renders directives as prose. But
when a player's directive action pushes against a character's tensor-encoded
nature, resistance emerges from the gap between "what was asked" and "who
this person is."

## The Asymmetry

The dramaturgy-of-tension design established an intentional asymmetry:

- **NPC resistance** is rendered neutrally. Arthur doesn't resist because a
  game mechanic told him to — he resists because that's who he is. The
  narrator presents this as observable behavior without editorial comment.

- **Player-character tension** is highlighted. When the player acts against
  their own character's nature, the intent synthesizer produces a tension
  note and the narrator renders it through physical tells — the character's
  body betraying the gap between action and identity.

This means NPCs have agency (they respond authentically) and the player has
awareness (they see when their character strains against its own nature).
Neither is a hard constraint — the player can always act, and NPCs will
always respond — but the rendering carries the weight of who these people
are.

## Open Questions

1. **Is this load-bearing?** If we changed the intent synthesis prompt or
   swapped the 3b model, would NPC resistance disappear? If so, we may want
   to formalize it rather than relying on emergent behavior.

2. **Should we formalize it?** One approach: tensor-derived "compliance
   thresholds" that explicitly modulate how readily a character follows
   player directives. Risk: making it mechanical rather than emergent.

3. **Does facilitation emerge too?** When a player's action aligns with an
   NPC's nature, does the rendering show eagerness, relief, or natural
   ease? We haven't observed this yet — worth testing with a scene where
   the player's action matches the NPC's dominant traits.

4. **Scaling to more characters:** The Margaret/Arthur scene had two
   characters. In a scene with 4-5 cast members, does the resistance
   pattern hold, or does it get diluted by the narrator juggling too many
   directives?

5. **Reinforcement without formalization:** Instead of building a resistance
   mechanic, we could reinforce this behavior through prompt engineering —
   e.g., adding "ground each character's response in their tensor-encoded
   personality" to the intent synthesis prompt. This preserves emergence
   while making it more reliable.
```

- [ ] **Step 2: Commit**

```bash
git add docs/emergent/agency-from-data.md
git commit -m "docs: emergent agency discovery report from first gameplay session"
```
