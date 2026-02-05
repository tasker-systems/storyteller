# Tensor Case Study: The Wolf

## Purpose

The Wolf is a stress test for the character tensor model. Sarah is human, young, emotionally legible — the model was designed for characters like her. The Wolf is none of these things. He is "a dream of wolves and cold hungry winters and starlight that has woven itself out of memories of the forgotten dead, taking form and prowling." Can the same tensor structure represent this?

If the tensor model only works for human characters, it is insufficient for the storyteller system, which must handle entities across a wide spectrum of mindedness — from fully human to deeply alien. This case study identifies where the model stretches, where it breaks, and what adaptations are needed.

---

## Design Decisions Made During This Case Study

1. **Non-human axes require reinterpretation, not replacement.** The personality axis structure works for the Wolf, but the semantics shift. "Warmth/reserve" for a human is a social behavior; for the Wolf, it is a question of whether the entity permits connection at all. The axis labels should be treated as templates, not prescriptions.

2. **Motivational layering works differently for bound entities.** The Wolf serves Adam, who serves the Ghostlight Queen. His surface wants are not his own — they are imposed obligations. The shadow layer contains his *actual* desires, which he may not even recognize as distinct from his duties. This inverts the human pattern.

3. **Temporal dynamics require a different geology.** The Wolf is ancient. His "bedrock" was not formed in childhood — it was woven from "memories of the forgotten dead." His topsoil (current emotional state) is thin and volatile. His sediment barely exists in the human sense — he does not accumulate relational patterns the way humans do, because he rarely has sustained relationships. This suggests the temporal model needs a **primordial** layer beneath bedrock for entities older than individual life.

4. **Capacities need a non-human category.** Physical capacities for the Wolf are fundamentally different: he doesn't have "strength 0.8" — he has world-rending power that operates by different rules than human physicality. The system needs a `capacity_domain` field that distinguishes natural, supernatural, and conceptual capacities.

5. **Token budget for alien characters is *lower*, not higher.** The Wolf's tensor is simpler than Sarah's because much of the human complexity (social performance, self-deception, developmental psychology) doesn't apply. An alien character's complexity lives in the strangeness of what remains, not in additional dimensions. Estimated: ~1,500-2,000 tokens for full tensor.

---

## 1. Personality Axes

### 1.1 Temperament

```
axis: presence_withdrawal
  # Replaces optimism/pessimism. The Wolf is not optimistic or pessimistic
  # about the future — he exists in the present with varying degrees of
  # engagement with the mortal world.
  central_tendency: -0.3  # withdrawn by default — he observes, patrols
  variance: 0.5
  range: [-0.9, 0.4]     # can become nearly absent; can become intensely present
  layer: primordial
  contextual_triggers:
    - (sarah_demonstrates_power, shift_toward_present, 0.5)  # the other bank
    - (routine_travel, shift_toward_withdrawn, 0.2)
    - (danger_to_sarah, shift_toward_present, 0.4)
```

```
axis: patience_urgency
  # Replaces excitability/steadiness. The Wolf operates on a geological
  # timescale. His urgency is rare and significant.
  central_tendency: -0.6  # deeply patient — "come" is all he says
  variance: 0.3
  range: [-0.9, 0.3]
  layer: primordial
  contextual_triggers:
    - (threat_approaches, shift_toward_urgency, 0.5)  # bounds ahead in the mist
    - (sarah_delays, minimal_shift, 0.1)  # he waits; mortals tire
```

```
axis: connection_isolation
  # Replaces warmth/reserve. The Wolf does not have warmth in the human
  # sense. He has moments where the boundary between himself and another
  # entity becomes permeable. These are surprising to him.
  central_tendency: -0.7  # strongly isolated — a lone predator dream
  variance: 0.4
  range: [-0.9, 0.1]     # can be moved toward connection; never warm
  layer: primordial/topsoil  # primordial tendency, topsoil shifts
  contextual_triggers:
    - (sarah_touches_him, shift_toward_connection, 0.4)
    - (sarah_shows_unexpected_capability, shift_toward_connection, 0.3)
    - (completing_duty, shift_toward_isolation, 0.2)
```

### 1.2 Moral Orientation

```
dimension: core_values
  - duty_to_adam: 0.7            # bound service, not freely given
  - protection_of_the_charge: 0.6  # keep Sarah safe — but also see she fails
  - respect_for_power: 0.8       # the Wolf recognizes genuine power; ranks entities by it
  - territory_integrity: 0.9     # the Wood is his domain; he knows its laws
  - indifference_to_human_morality: 0.7  # right/wrong are human categories
  layer: primordial
```

```
dimension: moral_lines
  - will_not: directly harm Sarah (bound by Adam's instruction)
  - will_not: betray the Ghostlight Queen's interests (higher loyalty)
  - blurs_at: passive vs. active failure — protecting Sarah while ensuring she fails
  - blurs_at: what he owes the child vs. what he owes the duty
  layer: primordial
  note: "The Wolf's moral conflict is between competing obligations, not
         between self-interest and duty. He does not have self-interest
         in the human sense."
```

### 1.3 Cognitive Style

```
axis: instinct_deliberation
  # Replaces intuitive/analytical. The Wolf does not reason in the human
  # sense — he senses, knows, responds. But he can deliberate when surprised.
  central_tendency: -0.7  # strongly instinctual
  variance: 0.5
  range: [-0.9, 0.2]
  layer: primordial
  contextual_triggers:
    - (encountering_the_unexpected, shift_toward_deliberation, 0.5)
    - (the_other_bank, shift_toward_deliberation, 0.6)  # he cannot understand this
```

```
axis: certainty_uncertainty
  # The Wolf operates from deep certainty about the Wood, its paths, its
  # dangers. Sarah shatters this at the other bank.
  central_tendency: 0.8  # highly certain
  variance: 0.4
  range: [0.0, 0.9]
  layer: primordial (cracking)
  contextual_triggers:
    - (sarah_perceives_hidden_path, shift_toward_uncertainty, 0.6)
    - (familiar_territory, shift_toward_certainty, 0.2)
```

### 1.4 Social Posture

```
axis: command_deference
  # The Wolf is not social in the human sense. He exists in hierarchies
  # of power: he defers to Adam, to the Queen; he commands lesser spirits.
  central_tendency: 0.5   # default: commanding
  variance: 0.6
  range: [-0.4, 0.9]
  layer: primordial
  contextual_triggers:
    - (with_sarah, neutral — she is neither above nor below)
    - (sarah_leading_at_other_bank, shift_toward_deference, 0.5)
    - (protecting_from_threat, shift_toward_command, 0.4)
```

```
axis: expression_silence
  # The Wolf is taciturn. He speaks rarely and briefly. When he speaks,
  # words threaten violence.
  central_tendency: -0.8  # strongly silent
  variance: 0.3
  range: [-0.9, -0.2]    # never verbose; occasionally communicative
  layer: primordial
  contextual_triggers:
    - (sarah_asks_direct_question, shift_toward_expression, 0.2)
    - (danger_requiring_warning, shift_toward_expression, 0.3)
```

---

## 2. Motivational Structure

### 2.1 Surface Wants (His Stated Purpose)

```
- protect_sarah: 0.7
  # Adam's instruction. Keep her safe.
  urgency: ongoing
  layer: topsoil (imposed)

- guide_to_the_witch: 0.6
  # The immediate mission — deliver Sarah to the Witch of the Wood.
  urgency: active
  layer: topsoil (imposed)
```

### 2.2 Deep Wants (What Drives Beneath the Instructions)

```
- serve_the_power_structure: 0.6
  # Adam serves the Ghostlight Queen. The Wolf serves Adam. This is
  # the deep structure beneath the surface task.
  layer: primordial

- ensure_sarah_fails: 0.5
  # The contradictory deep want: Adam sends the Wolf to protect Sarah
  # AND to see that she fails. The Wolf holds both instructions.
  # He must keep her alive while allowing the mission to collapse.
  layer: topsoil (imposed, but contradicting protect_sarah)
  note: "This creates a genuine tension that the Character Agent must
         navigate. The Wolf is not lying or deceiving — he is carrying
         contradictory orders. How he resolves this in each moment is
         the core of his dramatic interest."

- maintain_territorial_knowledge: 0.7
  # The Wolf knows the Wood. This knowledge is his identity. When
  # Sarah sees what he cannot, this want is threatened.
  layer: primordial
```

### 2.3 Shadow Wants (What the Wolf Would Not Recognize)

```
- curiosity_about_sarah: 0.4
  # He should not care about this mortal child beyond his duty.
  # But she surprises him. She sees what he cannot. His interest
  # exceeds what duty requires.
  layer: forming (new sediment)
  note: "This is the Wolf's character arc — the emergence of something
         that looks like caring in an entity that was not built to care."

- desire_for_autonomy: 0.3
  # He serves. He has always served. But the contradictory orders
  # create a space where he must choose, and choice implies agency
  # beyond service.
  layer: forming (barely conscious)
```

---

## 3. Values and Beliefs

```
belief: the_wood_has_laws_that_must_be_respected
  strength: 0.95
  layer: primordial
  # The Wolf is the Wood's law embodied. When something violates
  # the Wood's nature, he responds not morally but physically —
  # his growl is the Wood's resistance.

belief: power_determines_hierarchy
  strength: 0.8
  layer: primordial
  # The Queen is above Adam is above the Wolf. Sarah was below
  # all of them. The other-bank scene disrupts this.

belief: mortals_are_fragile_and_temporary
  strength: 0.7
  layer: primordial (weakening)
  # Sarah is mortal. She should be fragile. She is not.
  # This belief is under pressure.

belief: duty_is_identity
  strength: 0.8
  layer: primordial
  # The Wolf does not distinguish between what he must do and
  # what he is. The contradictory orders create the first crack.
```

---

## 4. Capacities and Limitations

### Physical (Supernatural Domain)
```
- physical_power: 0.95     # world-rending; "rend the air and trees and ground"
  domain: supernatural
  note: "Not measured on a human scale. This is dream-logic power."

- sensory_acuity: 0.9      # scent, sound, the ability to read the Wood
  domain: supernatural
  limitation: "Cannot see paths that Sarah sees. His perception is
               of the Wood's surface, not its hidden depths."

- endurance: 1.0           # does not tire, does not need rest
  domain: supernatural

- form_stability: 0.7      # mostly holds wolf-shape; flickers to transparency
  domain: supernatural
  note: "He is not fully material. Parts of his bulk are moonlight."

- speech: 0.3              # can form words, but the voice threatens violence
  domain: supernatural
  limitation: "Language is not his medium. Words come through a mouth
               not designed for them."
```

### Perceptual
```
- knowledge_of_the_wood: 0.9    # paths, dangers, inhabitants, laws
  domain: territorial
  limitation: hidden_paths        # Sarah's gift reveals his blind spot

- danger_sensing: 0.85          # knows when threats approach
  domain: instinctual

- emotional_reading: 0.3       # poor at reading human emotional states
  domain: natural
  note: "He reads Sarah's fear but not her grief. He reads threat
         but not loneliness."
```

---

## 5. Temporal Dynamics

### Topsoil (Current State)

```
- wariness: 0.6
  # The standard state while guiding Sarah through dangerous territory.
  decay_rate: does not decay (maintained by circumstances)

- surprise: 0.0 → 0.8 (at the other bank)
  # The Wolf does not experience surprise often. Sarah seeing the
  # hidden path is genuinely shocking to him.
  decay_rate: slow — this stays with him
  trigger: sarah_perceiving_hidden_things

- reluctant_respect: 0.0 → 0.4 (growing)
  # Not warmth. Not affection. Something closer to the recognition
  # one predator gives another.
  decay_rate: does not decay (accumulating)
```

### Sediment

```
- Pattern: nearly empty
  # The Wolf does not have sustained relationships in the human sense.
  # His service to Adam is not sedimentary — it is structural (primordial).
  # Sarah is the first entity creating sedimentary patterns in him.
  # This itself is remarkable and should be noted in the Character Agent prompt.

- forming_pattern: attentiveness_to_sarah
  # He begins watching her not just for danger but with something
  # approaching interest. This is new sediment.
  strength: 0.3 (growing)
```

### Primordial (Replaces Bedrock for Ancient Entities)

```
- dream_of_wolves: 0.95
  # He is this. Wolves, cold, winter, starlight, the forgotten dead.
  # This is not personality; it is constitution. He cannot be other
  # than this without ceasing to exist.

- the_wood_is_home: 0.9
  # Territorial identity. He belongs to the Shadowed Wood the way
  # weather belongs to the sky.

- power_and_prowl: 0.85
  # His nature is predatory. Not cruel — predation without malice.
  # He kills to eat and patrols to maintain. There is no moral
  # dimension to this; it is what wolves do.
```

### Echo Potential

```
echo: encountering_true_power
  historical_pattern: unknown — but the Wolf has met the Ghostlight
    Queen, Adam at his full strength, possibly other ancient beings
  trigger_conditions: sarah demonstrating capabilities that exceed
    his own perception
  activated_state: a deep, instinctual recognition mixed with
    territorial unease — something in the Wood is more than he knew
  note: "The other-bank echo is not about memory of a specific event
         but about an archetype: the moment when the hierarchy shifts."
```

---

## 6. Relational Web (Wolf ↔ Key Characters)

### Wolf → Sarah

```
trust:
  competence: 0.3 → 0.7 (rising sharply after the other bank)
  intentions: 0.6          # she means what she says; she is transparent
  loyalty: N/A             # not a category the Wolf applies to mortals

affection: 0.0 → 0.2      # barely; something is forming that has no name

debt: 0.0                  # he owes her nothing; she owes him protection

power:
  initial: Wolf >> Sarah   # he is ancient supernatural; she is a child
  shifting: Wolf > Sarah   # she can see what he cannot
  note: "The power dynamic is the most important relational dimension.
         Its shift is the Wolf's primary character arc."

history: thin              # days of travel; but the other-bank is a
                           # defining shared experience

projection: she_is_a_mortal_child
  accuracy: decreasing     # she is more than this
  resistance_to_update: moderate  # his primordial beliefs resist
```

### Wolf → Adam

```
trust:
  competence: 0.8
  intentions: 0.6          # Adam's motives are complex; the Wolf knows this
  loyalty: 0.7             # bound service, not freely given

affection: 0.1             # minimal; this is a working relationship

power: Adam > Wolf         # Adam is the Gate; the Wolf came through him

history: long              # the Wolf has served Adam before

projection: adam_is_the_gate
  accuracy: high
```

### Wolf → Ghostlight Queen

```
trust:
  competence: 0.9
  intentions: 0.3          # the Queen's purposes are her own
  loyalty: 0.5             # through Adam, not direct

power: Queen >> Wolf       # absolute hierarchy

projection: the_queen_is_law
  accuracy: unknown
```

---

## 7. Context-Dependent Activation Examples

### Scene: Routine Travel Through the Wood

**Activated subset** (~600 tokens):
- Temperament: presence/withdrawal (withdrawn), patience/urgency (patient)
- Motivation: protect_sarah (0.7), guide_to_witch (0.6)
- Capacities: knowledge_of_the_wood (0.9), danger_sensing (0.85)
- Topsoil: wariness (0.6)
- Primordial: dream_of_wolves, the_wood_is_home

**Expression guidance for Character Agent**: "Speak rarely. One or two words. 'Come.' 'Stop.' Do not explain. If Sarah asks questions, answer with the minimum. You are not unkind; you are not built for conversation."

### Scene: The Other Bank

**Activated subset** (~900 tokens):
- Temperament: presence/withdrawal (strongly present), patience/urgency (moderate urgency)
- Cognitive: certainty/uncertainty (shattered), instinct/deliberation (forced to deliberate)
- Social: command/deference (shifting toward deference)
- Motivation: maintain_territorial_knowledge (threatened), curiosity_about_sarah (surging)
- Capacities: sensory_acuity (failing — he cannot see the path), knowledge_of_the_wood (insufficient)
- Topsoil: surprise (0.8), reluctant_respect (rising)
- Primordial: power_determines_hierarchy (disrupted)
- Relational: Sarah (power dynamic shifting, trust-in-competence rising)

**Expression guidance for Character Agent**: "You cannot see what she sees. This is profoundly disorienting. You are ancient and powerful and a child is showing you something real that you missed. Growl — not at her, but at the world that hid this from you. When she leads you by the shoulder, let yourself be led, but feel the strangeness of it in every step."

---

## 8. Design Implications for the Tensor Model

### What Works

1. **The axis structure** works for non-human entities when axis labels are reinterpreted. The `[central_tendency, variance, range]` tuple remains valid — the Wolf's emotional range is just narrower and differently anchored.

2. **Motivational layering** captures the Wolf's conflicting orders elegantly. Surface (protect her) contradicts deep (ensure she fails), with shadow (caring about her) emerging to complicate both.

3. **Context-dependent activation** is even more important for alien characters. The Wolf's full tensor is sparse compared to Sarah's — most of the complexity lives in how the sparse elements interact with specific scenes.

### What Needs Extension

1. **Primordial layer**: The geological metaphor needs a layer below bedrock for entities that predate individual experience. The Wolf's "dream of wolves" is not bedrock (formed in early life) — it is the material from which he was constituted.

2. **Capacity domains**: `natural`, `supernatural`, `conceptual` — the system needs to distinguish these so the World Agent knows which constraint tier applies.

3. **Non-verbal expression guidance**: Human Character Agents express through dialogue and described action. The Wolf's Character Agent needs explicit guidance about *how* to express — growls, posture, silence, the threatening quality of his speech. This might be a `expression_mode` field on the tensor.

4. **Relational model for non-reciprocal bonds**: The Wolf→Sarah relationship is forming but has no human analog. "Affection" is the wrong word. "Respect" is too cognitive. The relational web needs a `nascent_connection` dimension for relationships that don't yet have a category.

### What Breaks

1. **Echo mechanism for entities without personal history**: The Wolf's echoes are not personal memories — they are archetypal patterns from the collective dead whose memories constitute him. The echo mechanism needs a variant: `archetypal_echo` triggered by pattern recognition across collective memory rather than individual experience.

2. **Decay rates**: Most of the Wolf's tensor doesn't decay because it's primordial. The topsoil layer is thin. The decay model needs a `stable` rate alongside the `slow/medium/fast` options for elements that don't change under normal conditions.

---

## Appendix: Source Material Cross-References

| Tensor Element | Source |
|---|---|
| Nature: dream of wolves | a-guide.md: "a dream of wolves and cold hungry winters and starlight" |
| Voice: threatening | a-guide.md: "the guttural rumble of syllables threatens a snarl" |
| Duty: contradictory | adam.md: "sends the wolf with Sarah - to keep her safe, but to see that she fails" |
| Perception: blind to hidden paths | now-the-other-bank.md: "How can you see what I cannot?" |
| Form: flickering | before-speaking-with-the-gate.md: "parts of its bulk flicker transparent" |
| Power: world-rending | before-speaking-with-the-gate.md: "the world-devouring shape of its jaws" |
| Connection: touch-activated | now-the-other-bank.md: "I place my hand on the Wolf's shoulder" |
| Surprise: genuine | now-the-other-bank.md: "a deep growl of surprise" |
| Taciturnity | now-crossing-a-stream.md: "The Wolf speaks rarely" |
