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
