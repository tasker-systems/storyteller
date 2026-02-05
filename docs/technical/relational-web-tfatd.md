# Relational Web: "The Fair and the Dead" Characters

## Purpose

This document maps the relational web between all 6 characters in TFATD using the dimensions from `character_modeling.md`: trust (textured: competence, intentions, loyalty), affection, debt, power, history, and projection. Each relationship is **asymmetric** — A's view of B differs from B's view of A. The Wolf's relationships are documented separately in the Wolf tensor case study for brevity; this document covers the 6 human characters and their web.

This is the companion document to the Sarah and Wolf tensor case studies, completing the relational portion of Track B.

---

## Design Decisions

1. **Relationship representation**: Each directed edge (A→B) is a struct with named fields. This is more readable and debuggable than a vector representation, though it costs more tokens. For context-dependent activation, the Storykeeper selects only the relationships relevant to the current scene.

2. **Asymmetry is the default**: Every relationship is modeled twice (A→B and B→A). Symmetric relationships are a special case where both directions happen to be similar, not a distinct type.

3. **Unknown/unspecified values**: Many relationships are underspecified in the source material (e.g., John→Beth). These are marked as `unknown` rather than defaulted. The system should handle missing relational data gracefully — a character with no specified relationship to another simply has no strong feelings about them.

4. **"Debt" is not always financial**: Debt in this web includes emotional debts (obligations, promises, unspoken agreements) and spiritual debts (the transactions at the heart of the plot).

---

## The Web

### Sarah → Tom (Tommy)

```yaml
trust:
  competence: 0.8     # He is strong, capable, her older brother
  intentions: 0.6     # She senses he has been hiding something
  loyalty: 0.9        # He is family; of course he is loyal
  note: "Shaken by his withdrawal — she trusts his love but not his openness"

affection: 0.95       # Deep, uncomplicated sibling love
  note: "The purest affection in the web. She loves him without condition."

debt: 0.3             # She owes him years of care, protection, companionship
  type: emotional
  note: "But the quest is repaying this — she is now protecting him"

power:
  direction: Tommy > Sarah (perceived)
  magnitude: 0.4
  note: "He is 17, she is 12. He is bigger, older, more knowledgeable.
         But this dynamic is in flux — his helplessness inverts it."

history:
  depth: 0.9          # A lifetime of shared experience
  quality: warm → strained
  key_events:
    - shared_childhood: warm, close
    - tommy_withdrawing: painful, confusing
    - tommy_falling_ill: devastating
  unresolved: "Why did he pull away? What was he hiding?"

projection:
  sarah_projects_onto_tommy: "He is still my brother, the one who
    walks the woods with me. If I can find him, we will be as we were."
  accuracy: low        # Tommy has been fundamentally changed; he may
                       # not be able to return to what Sarah imagines
```

### Tom → Sarah

```yaml
trust:
  competence: 0.5     # She's twelve — capable but young
  intentions: 0.9     # She is transparent and loving
  loyalty: 0.9

affection: 0.85       # Deep, but complicated by guilt
  note: "He loves her but pulled away because he couldn't share
         his secret life (Beth, the child). His affection is
         shadowed by the distance he created."

debt: 0.7             # He owes her — for the distance, for leaving
  type: emotional/spiritual
  note: "He did not tell her. He made a choice that took him from
         her without explanation. The debt is enormous."

power:
  direction: Tommy > Sarah
  magnitude: 0.3      # Diminishing — his choice made him helpless

history: (mirrors Sarah → Tom)

projection:
  tommy_projects_onto_sarah: "She is still my little sister.
    She cannot follow where I have gone."
  accuracy: very_low   # Sarah is already following; she can see
                       # paths that even the Wolf cannot
```

### Sarah → Kate

```yaml
trust:
  competence: 0.7     # Her mother knows things — the prayers, the water
  intentions: 0.9     # Her mother loves her and wants her safe
  loyalty: 0.9

affection: 0.85       # Deep but different from her affection for Tommy
  note: "Less playful, more grounded. Kate is comfort and safety."

debt: 0.6             # A child's debt to a parent — unspoken, enormous
  type: existential

power:
  direction: Kate > Sarah (complicated)
  magnitude: 0.3
  note: "Kate has authority but uses it gently. Sarah asked: 'If I
         forbid you, would you listen?' Kate chose not to forbid."

history:
  depth: 0.9
  quality: warm, slightly mysterious
  key_events:
    - watching_kate_at_the_stream: hundreds of times
    - kate_singing_prayers: constant background of childhood
    - the_water_blessing: a revelation
  unresolved: "What is my mother, really?"

projection:
  sarah_projects_onto_kate: "My mother is gentle and a little
    otherworldly, but she is my mother."
  accuracy: partial    # Kate is more than Sarah knows — the
                       # "not precisely a normal wife and homemaker"
                       # is just beginning to register
```

### Kate → Sarah

```yaml
trust:
  competence: 0.8     # Kate sees Sarah more clearly than Sarah sees herself
  intentions: 0.9
  loyalty: 0.95

affection: 0.95       # Fierce maternal love
  note: "'I cannot lose you both.'"

debt: 0.0             # Kate feels she owes Sarah nothing — only gives
  note: "But she gave Sarah the blessing, which is a form of debt
         to forces beyond the family"

power:
  direction: Kate > Sarah (but Kate yielded it)
  magnitude: 0.2
  note: "Kate chose not to stop Sarah. This deliberate yielding
         of parental authority is itself a form of power — and trust."

history:
  depth: 0.9
  quality: loving, protective, with a thread of distance
  key_events:
    - raising_sarah: twelve years
    - kates_own_otherworldliness: a gap Sarah didn't name until now
  unresolved: "Can my daughter survive what I cannot follow her into?"

projection:
  kate_projects_onto_sarah: "She is my daughter, and her father's
    even more — grounded, practical, brave. She has eyes to see."
  accuracy: high       # Kate understands Sarah well — perhaps
                       # better than anyone
```

### Sarah → John

```yaml
trust:
  competence: 0.7     # Her father is strong and practical
  intentions: 0.9
  loyalty: 0.9

affection: 0.7        # Warm but less intense than Tommy or Kate
  note: "Sarah gets her jaw and brow from her father. The
         resemblance is love expressed in bone."

power:
  direction: John > Sarah
  magnitude: 0.4      # Standard parental authority
  note: "But John 'cannot enter the deep Wood' — his power is
         limited to the mortal world"

projection:
  sarah_projects_onto_john: "My father is a good man, but he
    cannot help me here."
  accuracy: high
```

### John → Sarah

```yaml
trust:
  competence: 0.5     # She's twelve; he underestimates her
  intentions: 0.9
  loyalty: 0.9

affection: 0.8        # A father's love — protective, proud

power:
  direction: John > Sarah (perceived by John)
  note: "He would stop her if he knew. Kate chose not to tell him."

projection:
  john_projects_onto_sarah: "My little girl. She needs protecting."
  accuracy: very_low   # Sarah is capable of things John cannot imagine
```

### Kate → John

```yaml
trust:
  competence: 0.7     # Practical, reliable, hardworking
  intentions: 0.9     # He is good and kind
  loyalty: 0.9

affection: 0.8        # "She loves him too, fully, though with a
                       #  not-entirely-conscious distance"

power:
  direction: Kate ≈ John (surface) / Kate > John (hidden)
  magnitude: 0.1 (surface)
  note: "Kate holds knowledge and capability that John doesn't
         understand. The power asymmetry is hidden from both of them."

history:
  quality: loving, stable, with an unexamined gap
  key: "He does not really see or understand the things that matter
        most to her. Not because he does not want to, but because he
        has the strong-backed practical perspective."
  unresolved: "The gap between what Kate is and what John can see"

projection:
  kate_projects_onto_john: "He is my anchor. He is good. He cannot
    see what I see, and I love him for the solidity of that."
  accuracy: moderate   # She underestimates his capacity to learn
```

### John → Kate

```yaml
trust:
  competence: 0.6     # He sees her as slightly dreamy, not quite practical
  intentions: 0.9
  loyalty: 0.9

affection: 0.9        # "John loves her devotedly and without question"

power:
  direction: John > Kate (perceived by John)
  note: "He doesn't realize she is the more powerful of them."

projection:
  john_projects_onto_kate: "My wife, half naiad, always praying
    at the stream. Gentle, loving, maybe a little dreamy."
  accuracy: low        # He has no idea of her true nature
```

### Adam → Sarah

```yaml
trust:
  competence: 0.3     # She is a mortal child
  intentions: 0.7     # She is transparent; her love for Tommy is obvious
  loyalty: N/A

affection: 0.3        # He is kind but not attached
  note: "He does not want her hurt. But his loyalty is elsewhere."

debt: -0.3            # Sarah may owe Adam — he gave her the Wolf,
                      # he told her about Tommy. These are debts.
  type: spiritual

power:
  direction: Adam >> Sarah
  magnitude: 0.8
  note: "He is a Gate. She is a child. The asymmetry is vast."

projection:
  adam_projects_onto_sarah: "A brave child who will fail. Her fear
    will weaken her resolve."
  accuracy: very_low   # He is wrong about everything
  note: "Adam's greatest misreading. He expects her to be merely brave.
         She is something else entirely."
```

### Sarah → Adam

```yaml
trust:
  competence: 0.7     # He knows things about Tommy's condition
  intentions: 0.4     # She senses something is off — "kind, but with his own motives"
  loyalty: 0.2        # She has no reason to believe he is on her side

affection: 0.2        # Wary respect, not warmth

power:
  direction: Adam > Sarah (but she went to him)
  magnitude: 0.5

projection:
  sarah_projects_onto_adam: "A strange man who knows things. He
    helped me, or seemed to. I do not fully trust him."
  accuracy: moderate   # She correctly senses his mixed motives
                       # but doesn't know about the Ghostlight Queen
```

### Tom → Beth

```yaml
trust:
  competence: 0.6
  intentions: 0.9     # She loves him and their child
  loyalty: 0.9

affection: 0.9        # Deep romantic love — the relationship
                       # that drove his sacrifice

debt: 0.9             # He feels he owes her everything — the child
                      # died, she is dying of grief. His debt is
                      # what sent him to the Witch.
  type: emotional/spiritual
  note: "This is the debt that drives the plot."

power:
  direction: Tom ≈ Beth (before crisis) / Tom < Beth's grief (now)
  note: "Her grief has more power over him than any person."

history:
  depth: 0.7          # Young love — intense but not long
  quality: passionate → devastating
  key_events:
    - falling_in_love: secret, joyful
    - pregnancy: hopeful
    - stillbirth: devastating
    - fair_folk_bargain: desperate
    - beth_declining: unbearable

projection:
  tom_projects_onto_beth: "I cannot let her die. If the child
    returns, she will live."
  accuracy: unknown    # We don't know if the changeling will save Beth
```

### Beth → Tom

```yaml
trust:
  competence: 0.8
  intentions: 0.9
  loyalty: 0.9

affection: 0.9        # She loves him

debt: 0.3             # She doesn't know what he has done for her

power:
  direction: Beth < Tom (perceived) / Beth's grief > Tom (actual)

projection:
  beth_projects_onto_tom: unknown  # She is too deep in grief to project
  note: "Beth is barely characterized in the source material beyond
         her grief and her role as the emotional anchor for Tom's sacrifice."
```

### Adam → Tom

```yaml
trust:
  competence: 0.4     # Tom is a mortal farmhand
  intentions: 0.6     # Tom's sacrifice was genuine
  loyalty: 0.1        # Tom is not Adam's concern

affection: 0.1

debt: 0.4             # Adam facilitated Tom's journey to the Witch
  type: spiritual
  note: "Adam carries some responsibility for Tom's state."

power:
  direction: Adam >> Tom

projection:
  adam_projects_onto_tom: "A brave young man whose sacrifice
    serves the Queen's purposes, whether he knows it or not."
  accuracy: moderate
```

---

## Web Visualization

```
                    ┌─────────┐
              ┌────►│  KATE   │◄────┐
              │     │ (mother) │     │
              │     └─┬───┬───┘     │
              │       │   │         │
         love,│  love,│   │love,    │love,
         trust│  yield│   │mystery  │devotion
              │       │   │         │
              │       ▼   ▼         │
         ┌────┴──┐ ┌───────┐ ┌─────┴──┐
         │ SARAH │ │ TOMMY │ │  JOHN  │
         │(quest)│ │(lost) │ │(father)│
         └───┬───┘ └───┬───┘ └────────┘
             │         │
        wary │    love,│
       trust │    debt │
             │         │
         ┌───▼───┐ ┌───▼───┐
         │ ADAM  │ │ BETH  │
         │(gate) │ │(grief)│
         └───────┘ └───────┘

Key tensions:
  Sarah ←→ Tommy:  love vs. distance vs. the impossibility of return
  Sarah ←→ Adam:   wary trust vs. hidden betrayal
  Kate  ←→ John:   love with an unexamined gap
  Tommy ←→ Beth:   love, debt, sacrifice — the engine of the plot
  Adam  ←→ Queen:  service, power, complicity (off-graph)
```

---

## Design Implications

1. **Token budget for relational data**: Each directed edge (as specified above) is ~150-250 tokens. A character with 5 relationships = 750-1,250 tokens of relational data alone. Context-dependent activation must select only the 1-3 relationships relevant to the current scene.

2. **The "projection" field is narratively powerful but hard to maintain**: Projections change as characters learn. The Storykeeper must update projections when information gates open. Example: when Sarah discovers Tommy's secret (Beth, the child), her projection of Tommy shatters. This is an echo trigger AND a projection update simultaneously.

3. **Missing/sparse characters work**: John and Beth have minimal characterization but still function in the web. The system should handle sparse tensor data without hallucinating — better to say "John feels worry and helplessness" than to invent personality traits not in the source material.

4. **The debt dimension is the plot engine**: Tommy→Beth (emotional debt = sacrifice), Adam→Queen (service debt = betrayal), Sarah→quest (family debt = journey). Tracking debts and their repayment/default is how the Storykeeper monitors plot progression.

5. **Asymmetry creates dramatic irony**: Adam projects Sarah will fail; she won't. John projects Kate as dreamy; she has real power. Tommy projects Sarah as too young; she sees what the Wolf cannot. The system can surface these asymmetries as dramatic potential for the Reconciler and Narrator.
