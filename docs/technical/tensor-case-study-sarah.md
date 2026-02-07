# Tensor Case Study: Sarah

## Purpose

This document constructs Sarah's full tensor representation from "The Fair and the Dead," forcing every design decision about the character modeling system to be concrete. Where the foundation document (`character_modeling.md`) describes dimensions in the abstract, this case study assigns specific values, discovers what representation formats work, and identifies where the model needs refinement.

Sarah is chosen as the first case study because she is the protagonist — the most fully realized character in the source material — and because she is a child, which stress-tests whether the tensor model handles characters whose inner life is rich but whose experience is limited.

---

## Design Decisions Made During This Case Study

These decisions emerged from the process of constructing Sarah's tensor. They are recorded here as proposals for the technical specification.

1. **Representation format**: Values on personality axes use a `[central_tendency, variance, range_low, range_high]` tuple rather than a single number. This captures the "not uniformly brave" requirement from the foundation document.

2. **Scale**: All scalar values use a `[-1.0, 1.0]` float range where applicable, with 0.0 as neutral/unspecified. For non-bipolar dimensions (e.g., intensity, strength), `[0.0, 1.0]` is used.

3. **Contextual triggers**: Stored as tagged conditions rather than free text. Format: `(trigger_context, axis_shift_direction, magnitude)`. These are what context-dependent activation uses to select relevant dimensions.

4. **Motivational layers**: Surface/deep/shadow are distinct fields, not a hierarchy. A character can have contradictory motivations across layers — this is the point.

5. **Temporal layer assignment**: Each tensor element is tagged with its geological layer (`topsoil`, `sediment`, `bedrock`). This determines decay rate and resistance to change. Elements can exist in multiple layers (a bedrock trait that also has topsoil activation).

6. **Token budget estimate**: This full tensor representation is approximately 2,500-3,000 tokens in structured format. Context-dependent activation must reduce this to ~800-1,200 tokens for a Character Agent prompt. This is feasible — most scenes activate only 30-40% of the tensor.

---

## 1. Personality Axes

### 1.1 Temperament

```
axis: optimism_pessimism
  central_tendency: 0.3  # slightly optimistic — pragmatic but not dark
  variance: 0.4          # shifts significantly under stress
  range: [-0.3, 0.8]     # can become grim but not nihilistic; can become fiercely hopeful
  layer: sediment         # built over years of rural self-reliance
  contextual_triggers:
    - (brother_mentioned, shift_negative, 0.3)    # Tommy's illness darkens her outlook
    - (competence_demonstrated, shift_positive, 0.2) # succeeding at tasks lifts her
    - (alone_in_danger, shift_negative, 0.4)       # solitude in the Wood shakes her
```

```
axis: excitability_steadiness
  central_tendency: -0.2  # slightly steady — "levelheaded" per character sheet
  variance: 0.3
  range: [-0.6, 0.5]      # can become very calm under pressure; can become excited
  layer: sediment
  contextual_triggers:
    - (confronting_supernatural, shift_toward_steady, 0.3)  # she braces, goes still
    - (receiving_bad_news_about_tommy, shift_toward_excitable, 0.4)
```

```
axis: warmth_reserve
  central_tendency: -0.1  # slight reserve — the face "adults call sullen"
  variance: 0.5           # high variance — deeply warm with family, guarded with strangers
  range: [-0.7, 0.8]
  layer: sediment/topsoil  # reserve is sedimentary; warmth surges are topsoil
  contextual_triggers:
    - (with_family, shift_toward_warm, 0.6)
    - (with_strangers, shift_toward_reserve, 0.3)
    - (vulnerability_shown_by_other, shift_toward_warm, 0.4)
```

### 1.2 Moral Orientation

```
dimension: core_values
  - loyalty_to_family: 0.95        # near-absolute; the engine of the entire quest
  - willingness_to_sacrifice: 0.7  # high — she is walking into the Wood
  - honesty: 0.8                   # "I will go, whatever you say"
  - respect_for_authority: 0.3     # low — she acts on her own judgment
  - pragmatism_over_principle: 0.6 # she'll risk debts in the village of haunts
  layer: bedrock/sediment
```

```
dimension: moral_lines
  - will_not: abandon Tommy while any path remains
  - will_not: deceive her mother (though she would have left by night)
  - blurs_at: risk to self vs. risk to mission — she undervalues her own safety
  - blurs_at: trusting the Wolf's judgment vs. her own instincts (the other bank)
  layer: bedrock
```

### 1.3 Cognitive Style

```
axis: intuitive_analytical
  central_tendency: 0.4   # leans intuitive — "her feet want to walk"
  variance: 0.3
  range: [0.0, 0.8]       # never purely analytical; can be strongly intuitive
  layer: bedrock           # this is who she is, not learned behavior
  contextual_triggers:
    - (crossing_water, shift_toward_intuitive, 0.3)  # streams activate her gift
    - (conversation_with_adam, shift_toward_analytical, 0.2) # she reasons through his riddles
```

```
axis: cautious_impulsive
  central_tendency: 0.1   # slightly impulsive — brave, acts
  variance: 0.4
  range: [-0.5, 0.7]
  layer: sediment
  contextual_triggers:
    - (wolf_counsel_caution, shift_toward_cautious, 0.2) # she considers his warnings
    - (tommy_at_stake, shift_toward_impulsive, 0.4)      # urgency overrides caution
    - (alone_and_afraid, shift_toward_cautious, 0.3)     # fear slows her
```

### 1.4 Social Posture

```
axis: dominant_deferential
  central_tendency: -0.1  # slightly deferential — she's twelve
  variance: 0.5
  range: [-0.5, 0.6]
  layer: topsoil/sediment  # shifting as she grows into the quest
  contextual_triggers:
    - (adult_authority_figure, shift_toward_deferential, 0.2)
    - (competence_demonstrated, shift_toward_dominant, 0.3)
    - (the_other_bank_scene, shift_toward_dominant, 0.5)  # she leads the Wolf
```

```
axis: gregarious_solitary
  central_tendency: -0.2  # slightly solitary — comfortable alone in woods
  variance: 0.3
  range: [-0.6, 0.3]
  layer: sediment
```

```
axis: performative_private
  central_tendency: -0.5  # strongly private — won't smile deferentially
  variance: 0.2
  range: [-0.7, -0.1]
  layer: bedrock
```

---

## 2. Motivational Structure

### 2.1 Surface Wants (What Sarah Would Say If Asked)

```
- find_tommy: 1.0          # "Finding her brother Tommy and bringing him home"
  urgency: critical
  layer: topsoil           # activated by the crisis, though rooted deeper
```

```
- return_home_safely: 0.6  # she wants to survive this
  urgency: background      # subordinate to finding Tommy
  layer: topsoil
```

```
- understand_what_happened: 0.5  # why is Tommy "lost"? what did Adam mean?
  urgency: moderate
  layer: topsoil
```

### 2.2 Deep Wants (What Drives the Surface Wants)

```
- restore_connection_with_tommy: 0.9
  # The quest is not just about saving Tommy's life — it is about closing
  # the distance that grew between them. His withdrawal preceded his illness.
  # She wants her brother back in both senses.
  layer: sediment

- prove_herself_capable: 0.7
  # She needs to know she can do this — not for ego, but because the
  # alternative (being too young, too small, too unknowing) is terrifying.
  layer: sediment

- belong_to_her_family: 0.8
  # Family is the bedrock of her identity. The quest is an act of belonging.
  layer: bedrock
```

### 2.3 Shadow Wants (What Sarah Would Deny or Doesn't Recognize)

```
- anger_at_tommy_for_leaving: 0.4
  # He pulled away. He had secrets. He chose to go. She is furious at him
  # for this, underneath the grief and the love. She doesn't fully feel this
  # yet but it may surface when she finds him changed.
  layer: sediment (suppressed)

- desire_to_be_seen_as_more_than_a_child: 0.5
  # She wants to be recognized — by the Wolf, by Adam, by the Witch —
  # as someone whose judgment matters. This drives the other-bank moment.
  layer: sediment

- fear_that_she_is_not_enough: 0.6
  # The mirror of prove_herself_capable. She is afraid the boundary of
  # her own thinking will be the thing that fails Tommy.
  layer: bedrock
```

---

## 3. Values and Beliefs

```
belief: people_should_do_what_needs_doing
  strength: 0.9
  layer: bedrock
  # This is her core operating principle. She does not argue; she acts.
  # It filters her perception: she judges others by whether they act.

belief: family_bonds_are_the_strongest_thing
  strength: 0.95
  layer: bedrock
  # Uncritically held. Has not yet been tested by discovering Tommy's
  # secret life (Beth, the child). When she learns this, this belief
  # will be shaken — not destroyed, but complicated.

belief: the_world_has_hidden_depths
  strength: 0.6
  layer: sediment (growing)
  # She is discovering this through the quest. She grew up knowing her
  # mother was slightly otherworldly but didn't name it. Now it's becoming
  # explicit. This belief is actively forming.

belief: adults_often_cannot_see_what_matters
  strength: 0.5
  layer: sediment
  # Her father can't enter the Wood. The preacher dismissed Adam.
  # The doctor found no cause. She is learning that being grown
  # does not mean being capable.

belief: fear_is_real_but_not_a_reason_to_stop
  strength: 0.7
  layer: sediment
  # "Courageous, and more afraid than she allows herself to recognize."
  # She doesn't deny fear; she refuses to let it be the final word.
```

---

## 4. Capacities and Limitations

### Physical
```
- endurance: 0.5          # farm child, tough feet, but twelve years old
- agility: 0.6            # wiry, quick
- strength: 0.3           # small, thin
- sensory_acuity: 0.7     # observant — notices the cup, the face, the path
- comfort_with_physicality: 0.6  # barefoot, outdoors, at home in her body
- supernatural_perception: 0.8   # can see the other bank; feels the uncanny
  note: "This capacity is innate, not learned. Even the Wolf cannot match it."
```

### Intellectual
```
- problem_solving: 0.6    # practical, direct approaches
- linguistic_facility: 0.5 # speaks plainly but with directness
- memory: 0.6
- attention_to_detail: 0.7 # notices Adam's shifting faces, the water rising to Kate's hand
- metacognition: 0.7       # "Sometimes the smartest thing you can do is know where you cannot see further"
  note: "Unusually high for her age. This is a central trait."
```

### Social
```
- empathy: 0.6            # feels others' grief but processes it pragmatically
- persuasion: 0.3         # doesn't persuade; she states and acts
- deception: 0.1          # nearly incapable of it; transparent
- humor: 0.3              # almost laughs at Adam-as-hawk; catches herself
- ability_to_read_a_room: 0.5  # listens at doors, reads tones, but misses subtext
- comfort_giving: 0.4     # hugs her mother but doesn't know what to say
```

---

## 5. Temporal Dynamics

### Emotional Grammar and Plutchik Primary Mapping

*Added to reflect the emotional model (`docs/foundation/emotional-model.md`). Sarah uses the Plutchik-derived Western grammar.*

Sarah's topsoil emotional states map to Plutchik primaries as follows. Note that named emotions like "grief" and "determination" are not stored — the system stores primary intensities, and the named states are computed as mood-vectors during frame synthesis.

```
grammar: plutchik_western

Primary intensities (topsoil, entering the quest):
  joy:          0.2  -- low; her world is fear and duty, but there are flickers (wonder)
  sadness:      0.8  -- dominant primary; maps to "grief" at this intensity
  trust:        0.6  -- she trusts the Wolf's competence, her mother's love, her own feet
  disgust:      0.1  -- minimal; she is not revolted by the world, just frightened
  fear:         0.6  -- apprehension → fear; the Wood, the unknown, the boundary
  anger:        0.2  -- low topsoil, but there is sedimentary anger at Tommy (Defended)
  surprise:     0.3  -- the Wood keeps surprising her; she remains open to it
  anticipation: 0.7  -- high; she is moving forward, expecting to find Tommy

Awareness annotations:
  sadness (0.8):      Articulate — she knows she is grieving
  fear (0.6):         Recognizable — she would admit to being afraid if asked
  anger (0.2):        Defended — the anger at Tommy is suppressed; she changes the subject
  anticipation (0.7): Articulate — she can state her purpose plainly
  trust (0.6):        Preconscious — she relies on people without examining why

Active mood-vectors (computed, not stored):
  sadness + anticipation → anxious determination (the quest's emotional signature)
  fear + trust           → submission/dependence on the Wolf (shifting at the Other Bank)
  sadness + anger        → envy? no — more like bewildered hurt (the Defended anger
                           colors the grief without being felt as anger)
  joy + surprise         → moments of wonder in the Wood
```

### Topsoil (Current Emotional State — Scene-Level Half-Life)

The named states below are frame-synthesis vocabulary — how the mood-vectors and primary configurations are rendered in natural language for the Character Agent.

```
- grief: 0.8              # sadness primary at high intensity; Articulate awareness
  decay_rate: slow         # sustained by Tommy's ongoing illness
  trigger: seeing Tommy's sickbed, hearing his fever-mutters
  accumulated_deposit: 0.3 # she has been grieving for weeks; approaching sediment

- determination: 0.9      # mood-vector: sadness + anticipation + trust (in self)
  decay_rate: variable     # surges when challenged, fades when exhausted
  trigger: any reminder of Tommy's need

- loneliness: 0.5         # mood-vector: sadness + low trust (when isolated)
  decay_rate: fast         # relieved by the Wolf's presence
  trigger: being separated from the Wolf, silence, memory of Tommy's withdrawal

- wonder: 0.4             # mood-vector: joy + surprise
  decay_rate: fast
  trigger: supernatural phenomena, the uncanny moments with Kate

- exhaustion: 0.6         # not a Plutchik primary — a physical/cognitive state
  decay_rate: slow         # accumulating
  trigger: physical hardship, cold, hunger
```

### Sediment (Patterns Built Over Months/Years)

```
- self_reliance: 0.8
  # Years of doing what needs doing. This is the pattern that makes the
  # quest conceivable — not bravery in the moment but the accumulated
  # habit of acting when action is needed.

- growing_awareness_of_loss: 0.6
  # Tommy's withdrawal preceded the illness. She has been losing him
  # for months — the distance, the secrets, the sad eyes. This sedimentary
  # grief is deeper than the topsoil grief about the fever.

- trust_in_her_own_perceptions: 0.7
  # She notices things. She knows what she sees. This becomes critical
  # at the other bank — she trusts her feet over the Wolf's eyes.

- pragmatic_kindness: 0.7
  # Not demonstrative warmth but practical care. She sits with Tommy.
  # She listens at doors. She goes into the Wood.
```

### Bedrock (Deepest Patterns — Extraordinarily Resistant to Change)

```
- identity_as_someone_who_acts: 0.9
  # The absolute foundation. She is not a passive character. When
  # something must be done, she does it. This was formed before language.
  # It is the rut through which all her experience flows.

- attachment_to_family: 0.95
  # Predating conscious memory. The family is the ground she stands on.
  # Threatening it activates everything.

- relationship_to_the_land: 0.6
  # Rural, grounded, connected to the physical world. "Confident in the
  # woods." Her feet know the paths. This bedrock quality is what makes
  # her supernatural perception feel organic rather than imposed.

- fear_of_the_boundary: 0.5
  # "She can feel faintly at the edges of her thoughts the boundary
  # beyond which she cannot see." This is a bedrock awareness of her
  # own limitations — not a fear of the dark but a fear of the unknown
  # that lies past what she can think through.
```

### Echo Potential

```
echo: tommy_withdrawing
  historical_pattern: months of Tommy growing distant, secrets, sad eyes
  trigger_conditions: encountering someone who conceals truth, feels loss
  activated_state: sharp mix of loneliness and anger (shadow want surfaces)
  resonance_dimensions: [relational, emotional, thematic]
  note: "If Sarah finds Tommy and he cannot or will not return, this echo
         will fire with full force — the withdrawal she experienced is
         revealed as something much larger than adolescent distance."

echo: the_uncanny_at_the_stream
  historical_pattern: years of watching Kate at the water, sensing something
  trigger_conditions: crossing water, encountering borderlands, liminal spaces
  activated_state: heightened perception, feeling of almost-understanding
  resonance_dimensions: [sensory, thematic]
  note: "Kate's water-blessing may have formalized something Sarah always
         carried. The echoes of those thousand stream-side moments activate
         her supernatural perception."
```

### Self-Referential Edge

*Added to reflect the emotional model (`docs/foundation/emotional-model.md`). The self-edge uses the same relational substrate schema as inter-entity edges.*

```
edge: Sarah → Sarah
  trust:
    competence: 0.7        # she knows she can act; years of self-reliance confirm this
    intentions: 0.8        # she trusts her own motives — they are straightforward
    reliability: 0.5       # she fears the boundary of her own thinking
  affection: 0.6           # she is not harsh with herself, but not gentle either
  debt: 0.0                # no self-obligation (yet — this may change if she fails Tommy)
  history:
    pattern: "years of doing what needs doing while others didn't"
    weight: 0.8
  projection:
    content: "someone who can handle this"
    accuracy: 0.6          # she wants this to be true more than she knows it is
  information_state:
    knows: her own competence, her love for Tommy, her determination
    does_not_know: the depth of her anger at Tommy (Defended),
                   the source of her supernatural gift (Structural),
                   that her fear of the boundary IS the boundary
```

**What the self-edge produces in frame computation**: Sarah's moderate self-trust and uncertain projection ("someone who can handle this" at 0.6 accuracy) create an undercurrent of self-doubt that is not Articulate — she doesn't say "I doubt myself." Instead, it emerges in moments of hesitation, in the way she braces before acting, in the iron quality of her determination (which is partly compensation for the doubt). The Defended anger at Tommy and the Structural ignorance about her gift are information asymmetries on the self-edge: they create subtext that the Character Agent can perform without being told about directly.

**Self-edge shift at the Other Bank**: When the echo fires and she sees the hidden path, the self-edge shifts dramatically:
- `trust.competence` surges from 0.7 to 0.9 (she can do something the Wolf cannot)
- `projection.accuracy` jumps from 0.6 to 0.8 (she IS someone who can handle this)
- `trust.reliability` rises from 0.5 to 0.7 (her own perceptions are trustworthy)

This self-edge shift is what makes "you have never been more yourself" land — it is not just an emotional change but a reconfiguration of the character's relationship to her own capability.

---

## 6. The Player-Character Question

Sarah is *not* a player-character in the traditional sense — she is a protagonist in authored fiction. However, if she were adapted to the storyteller system, the player would inhabit her perspective. This creates a tension: the player makes decisions, but Sarah has strong existing motivations that constrain the decision space.

For the system, this suggests that **authored protagonist characters** need a distinction from blank-slate player-characters:

```
player_character_mode: authored_protagonist
  # The player is Sarah, but Sarah is not a blank vessel.
  # Decisions that violate her bedrock traits should meet soft resistance
  # (not refusal, but narrative friction — the Narrator communicates
  # that this feels wrong to Sarah, that her hands clench, that something
  # in her resists). The player can still choose, but the choice has weight.
  #
  # Decisions aligned with her motivations feel natural — the narrative
  # flows smoothly, Sarah's inner voice reinforces the choice.
  #
  # This is the "character capacity in context" soft constraint applied
  # to the player-character.
```

---

## 7. Context-Dependent Activation Examples

### Scene: Sitting at Tommy's Sickbed

**Activated subset** (~900 tokens):
- Temperament: warmth/reserve (shifted warm), optimism/pessimism (shifted pessimistic)
- Motivations: find_tommy (1.0), restore_connection (0.9), fear_not_enough (0.6)
- Values: family_bonds (0.95), people_should_do_what_needs_doing (0.9)
- Capacities: empathy, metacognition
- Topsoil: grief (0.8), exhaustion (0.6)
- Sediment: growing_awareness_of_loss (0.6)
- Bedrock: attachment_to_family (0.95), fear_of_the_boundary (0.5)
- Echoes: tommy_withdrawing (low activation — not triggered yet, but primed)

**Deactivated**: supernatural_perception, social posture axes, humor, physical capacities

### Scene: The Other Bank (Crossing the Stream)

**Activated subset** (~1,100 tokens):
- Temperament: excitability/steadiness (shifted steady), warmth/reserve (neutral)
- Cognitive: intuitive/analytical (strongly intuitive), cautious/impulsive (moderate)
- Social: dominant/deferential (shifted dominant — she leads the Wolf)
- Motivations: find_tommy (1.0), prove_herself_capable (0.7)
- Values: fear_is_real_but_not_a_reason_to_stop, the_world_has_hidden_depths
- Capacities: supernatural_perception (0.8), sensory_acuity (0.7), trust_in_own_perceptions (0.7)
- Topsoil: determination (0.9), wonder (0.4), loneliness (surges when Wolf disappears)
- Echoes: the_uncanny_at_the_stream (high activation — water + borderland + perception)
- Relational: Wolf (trust-in-competence: high, trust-in-intentions: moderate, power dynamic: shifting)

**Deactivated**: grief (backgrounded), exhaustion (backgrounded), most social capacities

---

## 8. Architectural Note: Storage, Activation, and Frame

*Added to reflect the power framework (`docs/foundation/power.md`) and the hexagonal architecture insight about separating what we store from what we present.*

### The Three Layers

The tensor as specified above is the **persistence layer** — the full internal domain model of who Sarah is. It is rich, geologically stratified, and designed to accumulate experience over time. But different consumers of this data need different views:

**Persistence layer** (full tensor, ~2,500-3,000 tokens): What the story designer authored plus what has accumulated through play. Changes slowly, governed by the geological temporal model. Read/written by the Storykeeper for state reasoning and updates. This is the hexagonal architecture's domain core.

**Activation layer** (scene-relevant subset, ~800-1,200 tokens): What the Storykeeper selects as relevant for the current scene — the context-dependent activation described in Section 7 above. This is an intermediate representation: a projection of the full tensor through scene context. The ML inference layer reads this alongside the relational web configuration and network topology.

**Frame layer** (psychological frame, ~200-400 tokens): What the ML inference model computes from the activated tensor + relational web configuration + network position + scene context. This is what the Character Agent actually receives. It is not a data structure — it is a compressed rendering of the psychological landscape the character is navigating, expressed in natural language that an LLM can inhabit.

### Psychological Frames for the Example Scenes

The context-dependent activation examples in Section 7 describe the activation layer. Here is what the frame layer would look like for each — the actual prompt fragment a Character Agent would receive:

**Scene: Sitting at Tommy's Sickbed**

> You are sitting with your dying brother and the grief is a weight in your chest, but underneath it there is iron — the part of you that does what needs to be done. You have cried all the tears you have. What remains is determination. You love him with everything you are and you are terrified that your love won't be enough, that there is a boundary past which your thinking cannot reach. Your mother's prayers drift from the stream but the warmth doesn't touch the cold place where Tommy is going. You don't know why he pulled away from you before the illness — that confusion lives underneath the grief, a splinter you can feel but can't reach. If someone suggests action, you lean into it. If someone counsels patience, something in you resists.

**Scene: The Other Bank (Crossing the Stream)**

> You can see a path the Wolf cannot. This is terrifying and clarifying in equal measure. Something in you — the part that has always noticed things, the part that watched your mother at the stream a thousand times — is awake and sure in a way you've never felt before. Your feet know this path. The Wolf is powerful, ancient, capable of rending the world — but right now he is uncertain and you are not. You don't fully understand what this means, but you know that this is yours: this seeing, this sureness. You are leading a creature of starlight and violence by the shoulder. You are twelve years old and afraid and you have never been more yourself. The part of you that needs to be seen as more than a child is not performing — it is simply being what it is.

### What the Frame Compresses

Note what happened between the activation layer and the frame layer:

- The activation layer for "The Other Bank" lists ~12 tensor dimensions with values. The frame compresses these into a psychological landscape the agent can inhabit.
- The frame incorporates relational configuration (the shifting dynamic with the Wolf) without naming it as data — it renders the relationship as felt experience.
- The frame activates the shadow want ("desire to be seen as more than a child") not as a labeled dimension but as an emotional undertone.
- The frame doesn't tell the agent what to do. It tells the agent who they are *right now*, and trusts the agent's generative capacity to produce behavior that emerges from that landscape.

This is the separation between computational-predictive and agentic-generative: the ML inference layer does the math (which dimensions matter, how the relational configuration shapes the scene, what power dynamics are operative). The Character Agent does the art (what Sarah says, how she moves, what she notices, how she surprises us).

### Echoes as Power Events

The echo mechanism (Section 5.4) needs an additional connection: echoes are not only emotional events but **power configuration events**. When Sarah's "uncanny at the stream" echo fires at the Other Bank, it activates her supernatural perception — which is what lets her see the hidden path — which inverts the power configuration with the Wolf. The echo doesn't just change how Sarah feels; it changes who holds capability in the relationship.

This means the ML inference layer must track echo potential as part of its computation. When an echo fires, the frame it produces should reflect the shifted configuration, not just the shifted emotion. The frame for "The Other Bank" implicitly does this ("the Wolf is uncertain and you are not") — the insight is that this shift is *produced by* the echo, and the inference layer should recognize the causal chain: echo activation → capability shift → configuration change → frame update.

---

## 9. Open Questions Surfaced by This Case Study

1. **Axis count and token budget**: This tensor has ~25 distinct axes/dimensions before relationships. That's manageable for a protagonist but may be excessive for secondary characters. Do we need a "character complexity tier" system (full tensor for protagonists, reduced for supporting, minimal for minor)?

2. **Trigger specificity**: Contextual triggers like `(crossing_water, shift_toward_intuitive, 0.3)` require scene tagging. Who tags the scenes — the story designer, the Storykeeper at runtime, or both? This needs resolution in the agent communication protocol.

3. **Shadow want activation**: When does `anger_at_tommy_for_leaving` surface? The echo mechanism handles historical resonance, but shadow wants are active suppressions. They need a distinct activation pathway — perhaps through accumulated stress or specific revelation events.

4. **Supernatural capacity modeling**: Sarah's ability to see the other bank is not a learned skill or a magical spell. It is an innate capacity that she doesn't understand. How does the constraint framework handle capacities that the character herself cannot explain? The World Agent must know it's possible (hard constraint: this world allows such perception) without the Character Agent knowing *why* it's possible.

5. **Authored protagonist tension**: The `authored_protagonist` mode is novel. It needs more design — specifically, how the Narrator communicates soft resistance when the player's choices diverge from the character's bedrock. This connects to Open Question #5 (Action Granularity).

6. **Relational asymmetry representation**: *Resolved*. The relational web case study (`relational-web-tfatd.md`) uses structs with named fields per directed edge, including substrate dimensions (trust[3], affection, debt, history, projection, information_state) and a configuration annotation. Power is emergent from the configuration, not stored. Each edge is ~250-400 tokens; the psychological frame compresses scene-relevant relationships to ~200-400 tokens total. See `docs/foundation/power.md` for the full argument.

---

## Appendix: Source Material Cross-References

| Tensor Element | Source |
|---|---|
| Personality: reserve, pragmatism | sarah.md: "Independent... not needlessly argumentative" |
| Personality: metacognition | before-tom-lies-still-and-dying.md: "the boundary beyond which she cannot see" |
| Motivation: restore connection | sarah.md: "Tommy seems these days to find ways to not really share" |
| Shadow: anger at Tommy | sarah.md: "wants to be mad at him" |
| Capacity: supernatural perception | now-the-other-bank.md: "How can you see what I cannot?" |
| Value: people should act | sarah.md: "willingness to do what needs to be done" |
| Echo: uncanny at stream | before-a-mothers-prayer.md: "the streams will guide your feet" |
| Temporal: exhaustion accumulating | now-crossing-a-stream.md: "exhausted from days of travel" |
| Bedrock: identity as actor | All scenes — she never waits; she goes |
