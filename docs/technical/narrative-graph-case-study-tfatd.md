# Narrative Graph Case Study: "The Fair and the Dead" Part I

## Purpose

This document maps Part I ("To the Witch") of "The Fair and the Dead" as a narrative graph, forcing the gravitational model from `narrative_graph.md` into concrete structures. Where the foundation document describes scenes as "gravitational bodies" with "narrative mass" and "attractor basins," this case study assigns specific properties to real scenes, discovers what's computable, and identifies where the metaphor needs engineering.

Part I contains 6 authored scenes across 2 sections ("In the Fever Bed" and "Sarah and the Wolf"), plus plot notes that imply additional scenes. The story is authored fiction, not an interactive narrative — so this case study also explores how an authored story would be *adapted* for the storyteller system, identifying where player agency would live and where narrative gravity constrains.

---

## Design Decisions Made During This Case Study

1. **Narrative mass is a composite score**, not a single authored number. It combines: `authored_base_mass` (set by story designer), `structural_modifiers` (connections, gate importance, convergence), and `dynamic_adjustment` (player state proximity to the scene's attractor basin). Formula proposed below.

2. **Scenes need a `scene_type` field**: `gravitational` (high mass, attractor), `connective` (low mass, texture/travel), `gate` (medium mass, information-critical), `threshold` (transition between narrative zones). These types have different default behaviors and different activation patterns.

3. **Approach vectors are not just paths — they are state requirements.** An approach vector is a predicate over the player's current state: `(emotional_state, information_state, relational_state, physical_state) → bool`. The scene activates differently depending on which predicates are satisfied.

4. **The "Before/Now" temporal structure** in the manuscript maps to a system concept: scenes can be tagged with **narrative time markers** that the Storykeeper uses to control sequencing. "Before" scenes establish backstory; "Now" scenes are the active quest. A player entering the story would begin in the "Now" timeline, with "Before" content surfaced through flashback, memory, or narration as approach vectors are satisfied.

5. **Connective space needs more design than the foundation document suggests.** In TFATD, the travel between scenes (walking with the Wolf, the mist, exhaustion) carries significant emotional and relational weight. Connective space is not "low mass" — it is differently massed, with mass distributed across texture rather than concentrated in events.

---

## The Scene Graph

### Scene Inventory

| ID | Title | Section | Type | Time |
|----|-------|---------|------|------|
| S1 | Tom Lies Still and Dying | Fever Bed | Gravitational | Before |
| S2 | Speaking with the Gate | Fever Bed | Gate/Threshold | Before |
| S3 | A Mother's Prayer | Sarah & Wolf | Gate/Gravitational | Before |
| S4 | A Haunt on the Rise | Sarah & Wolf | Connective/Gate | Now |
| S5 | Crossing a Stream | Sarah & Wolf | Connective | Now |
| S6 | The Other Bank | Sarah & Wolf | Gravitational | Now |
| S7 | The Abandoned Village | (implied by S4 ending) | Gate | Now |
| S8 | Meeting the Witch | (implied by plot notes, title "To the Witch") | Gravitational | Now |

S7 and S8 are not yet written but are structurally necessary.

---

## Scene Specifications

### S1: Tom Lies Still and Dying

```yaml
id: s1_tom_dying
type: gravitational
narrative_mass:
  authored_base: 0.8          # high — this is the inciting event
  structural_modifiers:
    - convergence_point: false
    - information_gates: 2     # (+0.1) Tommy is "lost"; preacher vs. Adam
    - required_for_story: true # (+0.1) everything flows from this
  computed_mass: 0.9

tonal_signature: elegiac, intimate, desperate
thematic_register: [loss, helplessness, the_limits_of_knowledge]

required_presences: [Sarah, Tom (unconscious)]
contingent_presences: [John, Kate (background), Preacher, Adam (overheard)]

information_gates:
  - gate: tommy_is_lost
    condition: Sarah overhears Adam
    reveals: Tommy's illness is spiritual, not physical
    structural: true          # always fires in this scene
  - gate: preacher_opposes_adam
    condition: always (structural)
    reveals: the mortal world has no solution; the spiritual path is contested

approach_vectors:
  - vector: story_opening
    # If this is the first scene (as in the authored version):
    player_state: {information: minimal, emotional: neutral→grief}
    experience: full emotional impact; discovery of stakes
  - vector: returning_from_quest
    # If the player revisits after entering the Wood (system adaptation):
    player_state: {information: expanded, emotional: weighted_with_journey}
    experience: seeing Tommy's body with new understanding; urgency intensified

departure_trajectories:
  - toward: s2_speaking_with_gate
    conditions: Sarah knows Adam mentioned something important
    momentum: curiosity + desperation
  - toward: connective_home
    conditions: always available
    momentum: inertia (staying home, waiting — a valid but low-energy path)
```

### S2: Speaking with the Gate

```yaml
id: s2_speaking_with_gate
type: gate_threshold
narrative_mass:
  authored_base: 0.7
  structural_modifiers:
    - information_gates: 4     # (+0.2) heavy information delivery
    - threshold_to_new_zone: true # (+0.1) this is the door to the quest
  computed_mass: 0.9

tonal_signature: uncanny, intimate, shifting
thematic_register: [the_supernatural_is_real, courage, threshold_crossing]

required_presences: [Sarah, Adam]
contingent_presences: [The Wolf (appears at end)]

information_gates:
  - gate: adam_is_a_gate
    condition: structural
    reveals: Adam communes with spirits; his form shifts constantly
  - gate: tommy_is_in_the_shadowed_wood
    condition: Sarah asks about "lost"
    reveals: Tommy's spirit is wandering in an overlaid spirit-realm
  - gate: the_witch_can_help
    condition: Adam directs Sarah there
    reveals: There is a person at the borders who might aid the quest
  - gate: the_wolf_appears
    condition: structural (end of scene)
    reveals: Adam can summon a powerful spirit-beast; the Wolf is terrifying and real

approach_vectors:
  - vector: from_sickbed_desperate
    player_state: {information: tommy_is_lost (raw), emotional: grief+determination}
    experience: Adam's strangeness is shocking but Sarah pushes through
  - vector: from_sickbed_curious
    player_state: {information: tommy_is_lost (raw), emotional: curiosity > grief}
    experience: Adam's shifting is fascinating; Sarah's questions are eager

departure_trajectories:
  - toward: s3_mothers_prayer
    conditions: Sarah decides to enter the Wood
    momentum: high (the Wolf has appeared; the quest is real)
  - toward: s1_sickbed (return)
    conditions: Sarah is too frightened
    momentum: low (this path contracts the story)
    note: "Adam expects her to take this path. He is wrong."
```

### S3: A Mother's Prayer

```yaml
id: s3_mothers_prayer
type: gate_gravitational
narrative_mass:
  authored_base: 0.85
  structural_modifiers:
    - information_gates: 3     # (+0.15) Kate's nature, the water blessing, family dynamics
    - emotional_hinge: true    # (+0.1) the mother-daughter farewell
  computed_mass: 0.95
  note: "This is arguably the highest-mass scene in Part I despite not being
         the climactic scene (S6). Its mass comes from emotional weight and
         the irreversibility of the departure."

tonal_signature: tender, uncanny, bittersweet, sacred
thematic_register: [mother_love, sacrifice, the_borderlands, growing_up]

required_presences: [Sarah, Kate]
contingent_presences: [John (mentioned, absent)]

information_gates:
  - gate: kate_has_power
    condition: structural
    reveals: Kate's prayers are not idle; water responds to her touch
  - gate: water_as_borderland
    condition: Kate's blessing
    reveals: streams are liminal spaces between worlds; they will guide Sarah
    note: "This gate is critical for S6 (The Other Bank). Without this
           information, Sarah's ability to see the hidden path is
           unexplained. The Storykeeper must ensure this gate has fired
           before S6 can fully activate."
  - gate: john_cannot_enter_wood
    condition: Kate explains
    reveals: mortal-only perception cannot navigate the Wood; Sarah must go alone

approach_vectors:
  - vector: determined_and_ready
    player_state: {emotional: resolved, information: wood+wolf known}
    experience: a farewell scene; poignant but forward-moving
  - vector: frightened_but_going
    player_state: {emotional: fear > determination, information: wood+wolf known}
    experience: Kate's warmth provides the courage Sarah needs

departure_trajectories:
  - toward: connective_entering_wood
    conditions: Sarah leaves home
    momentum: very high (irreversible feeling; crossing threshold)
    note: "No authored return path to home until Part III"
```

### S4: A Haunt on the Rise

```yaml
id: s4_haunt_on_rise
type: connective_gate
narrative_mass:
  authored_base: 0.5
  structural_modifiers:
    - information_gates: 2     # (+0.1)
    - contains_branch_point: true # (+0.1) the abandoned village door
  computed_mass: 0.65

tonal_signature: tense, atmospheric, watchful
thematic_register: [displacement, trespass, the_cost_of_passage]

required_presences: [Sarah, Wolf]
contingent_presences: [The haunt (observed, not interacted), the boy across the water]

information_gates:
  - gate: haunts_exist
    condition: structural
    reveals: there are beings in the Wood that are "neither living nor dead"
  - gate: village_invitation
    condition: reaching the moss-covered fence
    reveals: there are communities in the Wood; debts can be incurred

approach_vectors:
  - vector: trudging_deeper
    player_state: {emotional: exhausted+determined, physical: days_of_travel}
    experience: the mist and cold and the haunt following create unease

departure_trajectories:
  - toward: s7_abandoned_village
    conditions: Sarah chooses to enter the gate
    momentum: moderate (Sarah's initiative against Wolf's counsel)
    note: "This is a player choice point. The Wolf advises against it.
           Sarah's pragmatism (Track B: pragmatism_over_principle 0.6)
           drives her toward the door."
  - toward: s5_crossing_stream (continue on path)
    conditions: Sarah follows the Wolf past the village
    momentum: moderate (Wolf's preferred path)
    note: "The Wolf counsels this. The path 'calls him.' This is the
           low-agency continuation."
```

### S5: Crossing a Stream

```yaml
id: s5_crossing_stream
type: connective
narrative_mass:
  authored_base: 0.3
  structural_modifiers: none significant
  computed_mass: 0.3
  note: "Lowest mass in Part I, but essential connective tissue. Establishes
         exhaustion, the Wolf's alien nature, Sarah's doubts."

tonal_signature: weary, sparse, intimate
thematic_register: [endurance, doubt, the_alien_companion]

required_presences: [Sarah, Wolf]
contingent_presences: none

information_gates: none
  # This scene reveals nothing factual. Its function is relational and
  # emotional: deepening the reader's understanding of the Wolf's voice,
  # Sarah's exhaustion, the texture of the journey.

approach_vectors:
  - vector: standard_travel
    player_state: {emotional: tired+lonely, physical: days_walking}

departure_trajectories:
  - toward: s6_other_bank
    conditions: continuing along the stream
    momentum: natural (they arrive at the next crossing)
```

### S6: The Other Bank

```yaml
id: s6_other_bank
type: gravitational
narrative_mass:
  authored_base: 0.9
  structural_modifiers:
    - information_gates: 2     # (+0.1) Sarah's power, the hidden path
    - turning_point: true      # (+0.1) power dynamic between Sarah and Wolf shifts
  computed_mass: 1.0
  note: "Highest mass scene in Part I. This is the first major revelation
         and the scene where Sarah's nature changes the terms of the quest."

tonal_signature: wondrous, tense, mysterious, intimate
thematic_register: [hidden_perception, power_shift, trust, the_borderlands]

required_presences: [Sarah, Wolf]
contingent_presences: none

information_gates:
  - gate: sarah_sees_hidden_paths
    condition: structural (always fires)
    reveals: Sarah has an innate supernatural perception that exceeds the Wolf's
    dependencies: s3_mothers_prayer.gate.water_as_borderland
    note: "Without the water-blessing context from S3, this scene still
           works narratively but lacks the resonance of understanding
           *why* Sarah can see this. The system should track whether
           S3's gate has fired."
  - gate: the_darker_path_is_truer
    condition: Sarah chooses the dark bank
    reveals: the obvious/safe path is illusion; truth requires going into darkness
    note: "Thematic gate. This doesn't reveal a fact — it establishes
           a pattern that will recur."

approach_vectors:
  - vector: exhausted_but_persisting
    player_state: {emotional: tired+determined, relational: wolf_trust moderate}
    experience: the revelation is startling; energy surges
  - vector: growing_trust_with_wolf
    player_state: {relational: wolf_trust high, emotional: companionable}
    experience: the moment of leading the Wolf is tender; the power shift is gentle
  - vector: frightened_and_alone
    player_state: {emotional: fear dominant}
    experience: seeing the dark bank is terrifying; the Wolf's disappearance is devastating
    note: "All three vectors converge to the same structural event but
           with profoundly different emotional textures."

departure_trajectories:
  - toward: deeper_wood (the dark path)
    conditions: Sarah leads the Wolf to the other bank
    momentum: very high
  - toward: s5_retreat (back across the stream)
    conditions: Sarah cannot face the dark path alone
    momentum: low (narratively, she briefly does this before returning)
    note: "The scene contains its own approach-retreat-approach pattern.
           The player might retreat permanently — this would be a soft
           branch that the Storykeeper handles through narrative contraction."
```

---

## The Gravitational Landscape

### Mass Distribution

```
Scene Mass Map (Part I):

  S3 (0.95) ●━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━●  S6 (1.0)
  Mother's    ╲                              ╱  Other Bank
  Prayer       ╲                            ╱
                ╲                          ╱
  S1 (0.9) ●━━━━╲━━━━━━━━━━━━━━━━━━━━━━━╱━━━●  S2 (0.9)
  Sickbed        ╲                      ╱       Gate
                  ╲                    ╱
                   ╲                  ╱
  S4 (0.65) ●━━━━━━╲━━━━━━━━━━━━━━━╱━━━━━━━━●  S8 (est. 0.95)
  Haunt            ╲              ╱             Witch
                    ╲            ╱
  S5 (0.3) ●━━━━━━━━╲━━━━━━━━━╱━━━━━━━━━━━━━●  S7 (est. 0.5)
  Stream Rest        ╲        ╱                 Village
                      ╲      ╱
                       ╲    ╱
                        ●  ●
                      Connective
                        Space
```

### Attractor Basins

**S6 (The Other Bank)** has the largest attractor basin in Part I. Its pull begins as soon as Sarah enters the Wood (after S3). Every scene in the journey contributes to the approach vector — exhaustion builds, the Wolf relationship deepens, the water-blessing primes her perception. The basin is shaped primarily by:
- **Emotional accumulation**: determination + exhaustion + growing trust
- **Information state**: water-as-borderland knowledge from S3
- **Relational state**: Sarah-Wolf dynamic must have enough depth for the power-shift to land

A player who rushes through the connective space (S4, S5) reaches S6 with a thinner approach vector. The scene still works — but with less emotional mass. The system should not prevent this, but the Storykeeper might use narrative drift to slow the pace, offering small moments that build relational depth.

**S3 (A Mother's Prayer)** has a narrow but deep basin — it requires Sarah to have decided to enter the Wood (S2 departure), and it pulls strongly because Kate's scene combines emotional climax with critical information delivery. A player who tries to leave without speaking to Kate would miss the water-blessing gate, which weakens S6.

**S8 (Meeting the Witch)** — not yet written but implied as Part I's destination — is the convergence point toward which the entire "To the Witch" title points. Its basin extends across the full Part I journey.

### Connective Space Properties

The connective space between S3 and S6 is unusually rich for the storyteller system. In authored fiction, it is rendered through atmosphere, internal monologue, and the Wolf's taciturnity. In the system, this space would be where:

1. **Player-Wolf relationship develops** through small interactions (asking questions, resting together, observing the Wood)
2. **The haunt encounter (S4)** provides a localized event with a branch point
3. **Sarah's doubts surface** (S5: "Does our walking help us save him?")
4. **The Wood's nature is felt** (mist, cold, shifting paths, the sense of being watched)

Connective space needs a **distributed mass** model: instead of a single mass value, it carries a `mass_per_unit_time` that accumulates as the player spends time there. This rewards engagement without punishing speed.

---

## Player Agency Map

If TFATD Part I were adapted for the storyteller system, where would player agency live?

### High-Agency Points (Genuine Choice)

1. **S2 → Quest acceptance**: Does the player decide to enter the Wood? The authored story assumes yes, but the system must handle a player who says no. (Storykeeper response: narrative contraction — the story becomes about sitting with a dying brother, which is a valid but diminished experience.)

2. **S4 → The abandoned village door**: Enter or continue? This is the most explicit branch point in Part I. Each path leads to different information and different relationships (the refugees/haunts vs. the Wolf's preferred route).

3. **S6 → The dark bank**: Follow instinct or follow the Wolf? The scene contains its own approach-retreat, but a player might commit to the bright path. (Storykeeper response: the Wolf leads them further, but the darker truths remain hidden — the story continues but with less depth.)

### Low-Agency Points (Constrained by Narrative Gravity)

1. **S1 → S2**: Sarah must seek out Adam. The narrative mass of S2 is too high to bypass without active resistance. A player who never leaves the sickbed enters narrative stasis.

2. **S3**: Kate's blessing cannot be refused without fundamentally breaking the story's magical logic. The system would handle refusal not as a hard constraint but through narrative consequence — without the blessing, S6 doesn't fully activate.

3. **The journey itself**: The player must travel through the Wood. The route varies (village vs. direct) but the direction doesn't.

### The Adaptation Question

TFATD as written is a tightly authored story with a clear protagonist arc. Adapting it for the system would require:

- **Expanding connective space**: More small scenes, more opportunities for exploration, more Wolf interactions
- **Deepening branch consequences**: The village branch (S4/S7) needs a full subplot with its own scenes and characters
- **Adding contingent presences**: The boy across the water, the haunt — these should be interactable
- **Loosening the convergence points**: S6 should be reachable via multiple paths with genuinely different approach vectors, not just different emotional textures of the same path

---

## Formalization: Toward Computable Types

### Narrative Mass Formula

```
effective_mass(scene, player_state) =
  authored_base_mass
  + structural_modifier_sum
  + dynamic_adjustment(scene, player_state)

where:
  structural_modifier_sum = Σ(modifier_weights)
    # information_gates: +0.05 per gate
    # convergence_point: +0.1
    # threshold: +0.1
    # required_for_story: +0.1
    # emotional_hinge: +0.1
    # branch_point: +0.05

  dynamic_adjustment(scene, player_state) =
    approach_vector_satisfaction(scene, player_state) * 0.2
    # How well the player's current state matches the scene's
    # optimal approach vectors. Range: [0.0, 0.2]
    # A scene that the player is perfectly prepared for has higher
    # effective mass — it pulls harder because it would land better.
```

### Approach Vector Satisfaction

```
approach_vector_satisfaction(scene, player_state) =
  max(
    for each vector in scene.approach_vectors:
      match_score(vector.player_state_predicate, player_state)
  )

where:
  match_score(predicate, state) =
    Σ(dimension_match * dimension_weight) / Σ(dimension_weight)

  dimension_match =
    1.0 if predicate.dimension fully satisfied
    0.5 if predicate.dimension partially satisfied
    0.0 if predicate.dimension unsatisfied
```

### Distance Function (for Attractor Basins)

```
narrative_distance(player_position, scene) =
  w_info  * information_distance(player.info_state, scene.required_info)
  + w_emo  * emotional_distance(player.emotional_state, scene.optimal_emotional)
  + w_rel  * relational_distance(player.relationships, scene.required_relationships)
  + w_phys * physical_distance(player.location, scene.location)

where:
  weights w_* are scene-specific (set by designer or computed from scene properties)
  each distance function returns [0.0, 1.0]

gravitational_pull(player_position, scene) =
  scene.effective_mass / narrative_distance(player_position, scene)^2
  # Inverse-square law, as in physics. Pull increases rapidly as distance decreases.
  # Cutoff: if distance < threshold, the scene activates.
```

---

## Open Questions Surfaced by This Case Study

1. **Mass units**: What scale? The `[0.0, 1.0]` range for `authored_base_mass` works within a single story but doesn't compose across stories. Does a "massive" scene in a short story have the same mass as one in an epic? Probably not — mass may need to be relative to the story's total mass budget.

2. **Connective space mass distribution**: The `mass_per_unit_time` model for connective space needs specification. Does it accumulate linearly? Does it have diminishing returns (so spending 30 minutes in connective space gives more than 15 but less than double)?

3. **Gate dependencies across scenes**: S6 depends on S3's water-blessing gate. The Storykeeper must track these dependencies. Should dependencies be authored explicitly (story designer marks them) or inferred (the system detects that S6's activation conditions reference information from S3)?

4. **The "Before/Now" split**: Temporal structure in TFATD distinguishes exposition (Before) from active quest (Now). For the system, "Before" scenes might be surfaced as flashbacks, narrated backstory, or playable prologues. The choice affects pacing and information delivery. This needs design in the Storykeeper's temporal management.

5. **Branch consequences and content investment**: The village branch (S4→S7) requires authored content that many players will never see. How does the system handle the story designer's ROI on optional content? The quality problem (Open Question #3) is sharpened here.

6. **Gravitational pull and player resistance**: The inverse-square law means pull becomes very strong near a scene. What happens when a player actively resists? At what point does pull become railroading? The formula needs a `player_resistance` modifier that reduces pull based on active player choices against the direction.

---

## Appendix: Scene Connection Graph

```
S1 (Sickbed) ──[curiosity + desperation]──→ S2 (Gate)
     │                                        │
     │ [stay home: narrative stasis]           │
     ▼                                        ▼
  (stasis)                              S3 (Mother's Prayer)
                                              │
                                              │ [crossing threshold]
                                              ▼
                                     ╔══════════════════╗
                                     ║  Connective Space ║
                                     ║  (entering Wood)   ║
                                     ╚════════╤═════════╝
                                              │
                                              ▼
                                     S4 (Haunt on Rise)
                                        │          │
                          [enter gate]  │          │ [continue on path]
                                        ▼          ▼
                                  S7 (Village)    S5 (Stream Rest)
                                        │          │
                                        │          ▼
                                        │    S6 (Other Bank) ★
                                        │          │
                                        └──────────┤
                                                   ▼
                                          ╔══════════════════╗
                                          ║  Deeper Wood      ║
                                          ║  (toward S8/Witch) ║
                                          ╚══════════════════╝
```
