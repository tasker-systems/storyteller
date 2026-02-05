# Relational Web: "The Fair and the Dead" Characters

## Purpose

This document maps the relational web between all 6 characters in TFATD. Each relationship is **asymmetric** — A's view of B differs from B's view of A. The Wolf's relationships are documented separately in the Wolf tensor case study.

This is the companion document to the Sarah and Wolf tensor case studies, completing the relational portion of Track B.

### Revision Note

This document has been revised to reflect the framework established in `docs/foundation/power.md`. The key change: **power is no longer a stored dimension on relational edges**. Power is emergent — it arises from the configuration of substrate dimensions (trust, affection, debt, history, projection, information state) and from structural position in the network. Each relationship now includes a `configuration` annotation that describes the emergent relational dynamic, including the power dynamics that arise from the interaction of substrate dimensions in context.

A new `information_state` dimension has been added to each edge, capturing the factual knowledge asymmetries that are distinct from the emotional/relational imagination captured by `projection`.

---

## Design Decisions

1. **Power is emergent, not stored.** The relational web stores *substrate dimensions* — trust (textured), affection, debt, history, projection, and information state. Power arises from the configuration of these dimensions in context and from structural position in the network. See `docs/foundation/power.md` for the full argument.

2. **Configuration annotations describe emergent dynamics.** Each directed edge includes a `configuration` field that identifies the qualitative relational dynamic arising from the interaction of substrate dimensions. These are descriptive, not prescriptive — they help the Storykeeper and the psychological frame computation layer understand the relational landscape, but do not mechanically determine character behavior.

3. **Information state is distinct from projection.** Projection captures what one character *imagines* about another, colored by emotional needs and relational history. Information state captures what one character factually *knows or doesn't know* about the other's circumstances, capabilities, and relationships. Sarah's projection of Tommy ("he is still my brother who walks the woods with me") is emotional imagination. Her information state regarding Tommy (she doesn't know about Beth, the child, or the fair folk bargain) is factual gap.

4. **Asymmetry is the default.** Every relationship is modeled twice (A→B and B→A). Symmetric relationships are a special case where both directions happen to be similar, not a distinct type.

5. **Unknown/unspecified values.** Many relationships are underspecified in the source material (e.g., John→Beth). These are marked as `unknown` rather than defaulted. The system should handle missing relational data gracefully.

6. **"Debt" is not always financial.** Debt includes emotional debts (obligations, promises, unspoken agreements) and spiritual debts (the transactions at the heart of the plot).

---

## Network Topology

Before examining individual edges, the structural shape of the web matters. Position in the network generates capacity — power that arises not from any single relationship but from where a character sits in the graph.

### Structural Map

```
                    ┌─────────┐
              ┌─────│  KATE   │─────┐
              │     │ (hidden │     │
              │     │ bridge) │     │
              │     └────┬────┘     │
              │          │          │
         ┌────┴──┐ ┌────┴──┐ ┌─────┴──┐
         │ SARAH │ │ TOMMY │ │  JOHN  │
         │(quest)│ │(lost) │ │(mortal │
         └───┬───┘ └───┬───┘ │periph.)│
             │         │     └────────┘
             │         │
         ┌───┴───┐ ┌───┴───┐
         │ ADAM  │ │ BETH  │
         │(gate/ │ │(grief/│
         │bridge)│ │periph)│
         └───┬───┘ └───────┘
             │
      ╔══════╧══════╗
      ║  OFF-GRAPH  ║
      ║ Ghostlight  ║
      ║ Queen, Wolf ║
      ╚═════════════╝
```

### Structural Positions

**Adam — Gate and Bridge.** Adam is the primary structural bottleneck. He connects the mortal world to the Shadowed Wood, and he connects the human characters to the off-graph entities (the Ghostlight Queen, the Wolf). His positional power is immense: remove him and Sarah has no path to Tommy, the Wolf has no handler, and the Queen's plan has no intermediary. His power over Sarah is not primarily on their edge — it is topological.

**Kate — Hidden Bridge.** Kate connects the mortal household to the otherworldly dimension of the story, but this connection is hidden from John and only partially visible to Sarah. Her structural position is as significant as Adam's, but its power is latent rather than exercised. She is the only character who could bypass Adam's gate — and she chose not to, sending Sarah instead.

**Sarah — Traveler.** Sarah's structural position is unique: she *moves through* the network. She starts in the household cluster, passes through Adam's gate, and enters the Wood. Her power increases as she moves — each new connection she forms (with the Wolf, with the landscape itself) reduces Adam's monopoly on bridging.

**Tommy — Displaced Hub.** Tommy was once central to the household cluster. His displacement — physically to the Shadowed Wood, relationally through his secret life with Beth — has created a vacuum that drives the plot. His structural absence is as significant as Adam's structural presence.

**Beth — Affective Periphery.** Beth is minimally connected to the main web. Her power is entirely indirect: through Tommy's love for her, through the grief that drives his sacrifice, through the child whose loss set everything in motion. She is the emotional engine of the plot from the network's edge.

**John — Mortal Periphery.** John is connected to Kate and Sarah but cut off from the otherworldly dimension. His structural position creates dramatic irony: he has authority in the mortal world (parent, provider) but is powerless in the domain where the story actually unfolds. He doesn't even know the story is happening.

### Clusters

- **Household**: Sarah, Kate, John — tight, warm, with hidden asymmetries in knowledge and capability
- **Sacrifice**: Tommy, Beth — intense, desperate, the relational engine of the plot
- **Gate**: Adam — bridges all clusters, controls information flow, serves off-graph interests
- **Off-graph**: Ghostlight Queen, the Wolf — connected through Adam, affect everything but are not directly in the human relational web

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

history:
  depth: 0.9          # A lifetime of shared experience
  quality: warm → strained
  temporal_layer: sediment  # This is a lifetime of accumulated relationship
  key_events:
    - shared_childhood: warm, close [bedrock]
    - tommy_withdrawing: painful, confusing [sediment]
    - tommy_falling_ill: devastating [topsoil, becoming sediment]
  unresolved: "Why did he pull away? What was he hiding?"

projection:
  sarah_projects_onto_tommy: "He is still my brother, the one who
    walks the woods with me. If I can find him, we will be as we were."
  accuracy: low        # Tommy has been fundamentally changed; he may
                       # not be able to return to what Sarah imagines

information_state:
  knows: Tommy is ill; Tommy withdrew; Tommy is lost in the Shadowed Wood
  does_not_know: Beth exists; the child existed; the fair folk bargain;
    why Tommy withdrew; what Tommy has become
  note: "The information gap is the plot's central mystery for Sarah.
         Every revelation will restructure this edge."

configuration: >
  Love across impossible distance. Deep affection and loyalty meet a
  massive information gap and a projection that cannot survive contact
  with reality. The traditional age/competence dynamic (17 > 12) is
  inverting — his helplessness and her capability are crossing over,
  though she does not yet know this. The emergent dynamic is desperate
  seeking: she acts from love and incomplete knowledge, spending
  everything to recover a version of her brother that may no longer exist.
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

history: (mirrors Sarah → Tom, same events, different weighting)
  note: "Tom carries the withdrawal as guilt; Sarah carries it as confusion"

projection:
  tommy_projects_onto_sarah: "She is still my little sister.
    She cannot follow where I have gone."
  accuracy: very_low   # Sarah is already following; she can see
                       # paths that even the Wolf cannot

information_state:
  knows: Sarah loves him; she is young and mortal
  does_not_know: Sarah is coming for him; Kate gave Sarah the blessing;
    Sarah can see things the Wolf cannot; Sarah went to Adam
  note: "Tom underestimates Sarah partly from love (protective older
         brother) and partly from information — he simply does not
         know what she has become since his departure."

configuration: >
  Guilt and underestimation. Deep love shadowed by the knowledge
  that he chose Beth's crisis over Sarah's trust. His debt to her
  is enormous and unpayable by conventional means. He projects her
  as incapable of following — this projection is both protective
  (he doesn't want her in danger) and self-serving (acknowledging
  her capability would make his secrecy less forgivable). The
  emergent dynamic is helpless guilt: he cannot reach her, cannot
  warn her, cannot undo what he has done.
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

history:
  depth: 0.9
  quality: warm, slightly mysterious
  temporal_layer: bedrock (the relationship itself) + topsoil (recent revelation)
  key_events:
    - watching_kate_at_the_stream: hundreds of times [bedrock]
    - kate_singing_prayers: constant background of childhood [bedrock]
    - the_water_blessing: a revelation [topsoil, cracking the bedrock]
  unresolved: "What is my mother, really?"

projection:
  sarah_projects_onto_kate: "My mother is gentle and a little
    otherworldly, but she is my mother."
  accuracy: partial    # Kate is more than Sarah knows — the
                       # "not precisely a normal wife and homemaker"
                       # is just beginning to register

information_state:
  knows: Kate prays at the stream; Kate knows blessings and water-magic;
    Kate chose not to forbid her
  does_not_know: The full extent of Kate's otherworldly nature; Kate's
    relationship to the forces of the Shadowed Wood; why Kate could
    have stopped her but didn't — not just kindness but knowledge
  note: "Sarah is beginning to see the edges of what she doesn't know.
         The water blessing was a crack in the comfortable projection."

configuration: >
  Protected autonomy in transition. Kate yielded parental authority
  deliberately — not from weakness but from knowledge. She sees Sarah
  more clearly than Sarah sees herself, trusts her capacity, and chose
  to empower rather than constrain. The emergent dynamic is a
  relationship crossing from protection to partnership: the mother
  releasing the child into a world the mother understands better
  than the child knows. Kate's deliberate yielding of authority is
  itself a profound expression of relational power — the power to
  let go when you could hold on.
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

history:
  depth: 0.9
  quality: loving, protective, with a thread of distance
  key_events:
    - raising_sarah: twelve years [bedrock]
    - kates_own_otherworldliness: a gap Sarah didn't name until now [sediment]
  unresolved: "Can my daughter survive what I cannot follow her into?"

projection:
  kate_projects_onto_sarah: "She is my daughter, and her father's
    even more — grounded, practical, brave. She has eyes to see."
  accuracy: high       # Kate understands Sarah well — perhaps
                       # better than anyone

information_state:
  knows: Sarah's character and capability; the nature of the Shadowed Wood;
    what the blessing does; what Sarah will face; that John doesn't know
  does_not_know: Whether Sarah will survive; what Sarah will become;
    what Adam's true motives are (or does she?)
  note: "Kate is the most informationally rich character in the household
         cluster. She knows the most and reveals the least. Her knowledge
         is itself a form of structural position."

configuration: >
  Fierce release. Kate holds more knowledge and capability than any
  other household character — she could have stopped Sarah, redirected
  her, or gone herself. She chose to bless and release. This is the
  configuration of a parent who sees clearly: love + superior knowledge
  + trust in the child's capacity + the discipline to yield authority.
  The emergent dynamic is not protection but empowerment from a position
  of hidden strength. Kate's grief ("I cannot lose you both") coexists
  with her trust. The tension between these is the emotional core of
  the maternal relationship.
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

debt: 0.4             # A child's debt, but less charged than Kate's
  type: familial

history:
  depth: 0.8
  quality: warm, uncomplicated
  temporal_layer: bedrock — stable, not in flux

projection:
  sarah_projects_onto_john: "My father is a good man, but he
    cannot help me here."
  accuracy: high

information_state:
  knows: John is mortal, practical, strong in the ordinary world
  does_not_know: Whether John knows she has gone; what John would do
    if he knew
  note: "Sarah's accurate assessment of John's limitations is itself
         a form of knowledge — she knows the boundary of his world."

configuration: >
  Warm irrelevance. Sarah loves her father but has already moved
  beyond the domain where his competence operates. The substrate
  dimensions are all positive but low-intensity — trust, affection,
  a calm history — with no debt pressure, no information tension,
  no projection distortion. The emergent dynamic is fond distance:
  he is part of the world she is leaving, and she knows it. No
  power struggle because the domains don't overlap.
```

### John → Sarah

```yaml
trust:
  competence: 0.5     # She's twelve; he underestimates her
  intentions: 0.9
  loyalty: 0.9

affection: 0.8        # A father's love — protective, proud

debt: 0.2             # Minimal — the comfortable economy of family life
  type: familial

history:
  depth: 0.8
  quality: warm, stable

projection:
  john_projects_onto_sarah: "My little girl. She needs protecting."
  accuracy: very_low   # Sarah is capable of things John cannot imagine

information_state:
  knows: Sarah is his daughter; she is twelve; she is brave
  does_not_know: Sarah has gone to the Shadowed Wood; Kate gave Sarah
    the blessing; Kate chose not to tell him; Sarah can see paths
    the Wolf cannot; Adam exists
  note: "John's information state is the most impoverished in the web.
         Kate's deliberate decision not to tell him creates the gap.
         He is the only character who doesn't know the story is happening."

configuration: >
  Protective blindness. John's love is genuine but his projection is
  almost entirely inaccurate, and his information state is the most
  impoverished in the web. He perceives himself as the protector of a
  child who needs protecting — a configuration that would give him
  relational authority in the mortal world but is structurally irrelevant
  in the domain where the story unfolds. The dramatic irony is total:
  the character with the strongest sense of parental authority is the
  one with the least knowledge and the least actual influence. His
  positional power (parent, provider) exists only in a domain the
  story has already left behind.
```

### Kate → John

```yaml
trust:
  competence: 0.7     # Practical, reliable, hardworking
  intentions: 0.9     # He is good and kind
  loyalty: 0.9

affection: 0.8        # "She loves him too, fully, though with a
                       #  not-entirely-conscious distance"

debt: 0.2             # The shared economy of a marriage
  type: mutual

history:
  quality: loving, stable, with an unexamined gap
  temporal_layer: sediment — years of accumulated partnership
  key: "He does not really see or understand the things that matter
        most to her. Not because he does not want to, but because he
        has the strong-backed practical perspective."
  unresolved: "The gap between what Kate is and what John can see"

projection:
  kate_projects_onto_john: "He is my anchor. He is good. He cannot
    see what I see, and I love him for the solidity of that."
  accuracy: moderate   # She underestimates his capacity to learn

information_state:
  knows: Everything about John — he is transparent to her
  does_not_know: Whether he could handle knowing what she is;
    whether his practical worldview would survive the revelation
  note: "The asymmetry is total: she sees him clearly, he cannot
         see her. But her decision to maintain this asymmetry is
         not deception — it is a judgment about what he can bear."

configuration: >
  Love across an unexamined gap. Kate holds knowledge and capability
  that John doesn't understand exists. The affection is real, the
  trust is genuine, the history is stable — but the entire
  relationship rests on Kate's ongoing decision not to reveal what
  she is. This is a configuration where informational asymmetry
  IS the structural foundation: the relationship's stability depends
  on the gap remaining unexamined. Kate's hidden capability creates
  latent relational power she never exercises — power that exists
  only as potential, as the knowledge that she could shatter his
  worldview but chooses not to. The equilibrium is loving but fragile.
```

### John → Kate

```yaml
trust:
  competence: 0.6     # He sees her as slightly dreamy, not quite practical
  intentions: 0.9
  loyalty: 0.9

affection: 0.9        # "John loves her devotedly and without question"

debt: 0.2             # The shared economy of a marriage
  type: mutual

history:
  quality: loving, stable
  temporal_layer: sediment — comfortable, unquestioned

projection:
  john_projects_onto_kate: "My wife, half naiad, always praying
    at the stream. Gentle, loving, maybe a little dreamy."
  accuracy: low        # He has no idea of her true nature

information_state:
  knows: Kate prays at the stream; Kate is gentle; Kate is a good mother
  does_not_know: Kate's otherworldly nature; Kate's true capabilities;
    that Kate chose not to tell him about Sarah; that Sarah is gone;
    that Kate's "dreaminess" is something far more substantial
  note: "John's information state regarding Kate mirrors his information
         state regarding Sarah — both are radically incomplete, and in
         both cases the incompleteness is maintained by others' choices."

configuration: >
  Devoted unknowing. John's love for Kate is the most uncomplicated
  affection in the adult web — warm, steady, unquestioning. But it
  is built on a model of Kate that is almost entirely inaccurate.
  His projection ("half naiad, a little dreamy") is an affectionate
  misreading that Kate permits because she values his solidity. The
  emergent dynamic is genuine love grounded in genuine ignorance —
  not deception by Kate (she is not hiding maliciously) but a
  structural gap between what he can perceive and what she is.
  If the story ever forces this gap open, the relationship's
  equilibrium will be fundamentally tested — not because the love
  is false but because the foundation it rests on is incomplete.
```

### Adam → Sarah

```yaml
trust:
  competence: 0.3     # She is a mortal child
  intentions: 0.7     # She is transparent; her love for Tommy is obvious
  loyalty: N/A

affection: 0.3        # He is kind but not attached
  note: "He does not want her hurt. But his loyalty is elsewhere."

debt:
  magnitude: -0.3     # Sarah owes Adam — he gave her the Wolf,
                       # he told her about Tommy. These are debts.
  type: spiritual
  directionality: "Adam has placed Sarah in his debt, deliberately."

history:
  depth: 0.2          # Brief encounter, but laden with consequence
  quality: transactional, weighted with hidden significance
  temporal_layer: topsoil — fresh, volatile, consequential

projection:
  adam_projects_onto_sarah: "A brave child who will fail. Her fear
    will weaken her resolve."
  accuracy: very_low   # He is wrong about everything
  note: "Adam's greatest misreading. He expects her to be merely brave.
         She is something else entirely."

information_state:
  knows: Sarah loves Tommy; she is mortal; she is twelve; she is brave;
    she came to him willingly
  does_not_know: Sarah can see paths the Wolf cannot; Sarah has Kate's
    blessing (or underestimates what this means); Sarah's capacity
    to act from something other than fear or bravery
  note: "Adam's information state is rich about the world and poor
         about Sarah specifically. He knows the rules of the game
         but has misread this particular player."

configuration: >
  Instrumental condescension. Adam holds vast structural advantage:
  he is a Gate, he controls the Wolf, he knows the cosmological
  rules, he serves the Queen. He has placed Sarah in spiritual
  debt and sent her with a guide that reports to him. Every substrate
  dimension points to asymmetry — low trust in her competence,
  deliberate debt creation, massive information advantage, structural
  gatekeeping. But his projection is catastrophically wrong. He
  has read the substrate correctly (she IS young, mortal, afraid)
  and drawn the wrong conclusion (she WILL fail). The emergent
  dynamic is confident control built on a misreading — the most
  dangerous configuration in the web, because the character with
  the most structural advantage is the one with the least accurate
  model of his counterpart.
```

### Sarah → Adam

```yaml
trust:
  competence: 0.7     # He knows things about Tommy's condition
  intentions: 0.4     # She senses something is off — "kind, but with his own motives"
  loyalty: 0.2        # She has no reason to believe he is on her side

affection: 0.2        # Wary respect, not warmth

debt:
  magnitude: 0.5      # She knows she owes him — for information, for the Wolf
  type: spiritual
  note: "She went to him. He helped. She is aware of the obligation."

history:
  depth: 0.2          # Brief
  quality: charged, uncertain
  temporal_layer: topsoil — everything is new and volatile

projection:
  sarah_projects_onto_adam: "A strange man who knows things. He
    helped me, or seemed to. I do not fully trust him."
  accuracy: moderate   # She correctly senses his mixed motives
                       # but doesn't know about the Ghostlight Queen

information_state:
  knows: Adam knows about Tommy; Adam gave her the Wolf; Adam is
    connected to the Shadowed Wood; Adam has motives she can't read
  does_not_know: Adam serves the Ghostlight Queen; the Wolf has
    contradictory orders; Adam expects her to fail; Adam is a Gate
    in the cosmological sense; the full scope of what he has set in motion
  note: "Sarah's information state is critically incomplete, but her
         intuitive read of Adam — trust his knowledge, suspect his
         motives — is the correct heuristic for this configuration."

configuration: >
  Wary dependence. Sarah needs what Adam knows and has provided,
  but trusts neither his motives nor his loyalty. The substrate
  shows competence-trust without intention-trust, acknowledged
  debt, and a projection that is partially accurate (she senses
  the mixed motives). Her information gaps are enormous but her
  intuition compensates: she uses what he offers while watching
  for the trap. The emergent dynamic is strategic caution — she
  accepts the help, carries the debt, and keeps her eyes open.
  This is the configuration Pete's push-hands metaphor illuminates:
  she maintains contact (she needs the connection) while reading
  his balance and direction (she does not commit her weight).
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

history:
  depth: 0.7          # Young love — intense but not long
  quality: passionate → devastating
  temporal_layer: sediment (love, compressed fast) + topsoil (crisis)
  key_events:
    - falling_in_love: secret, joyful [sediment]
    - pregnancy: hopeful [sediment]
    - stillbirth: devastating [topsoil becoming bedrock]
    - fair_folk_bargain: desperate [topsoil]
    - beth_declining: unbearable [topsoil]

projection:
  tom_projects_onto_beth: "I cannot let her die. If the child
    returns, she will live."
  accuracy: unknown    # We don't know if the changeling will save Beth

information_state:
  knows: Beth's grief; the child's death; Beth's declining state;
    what the Witch offered; what it cost
  does_not_know: Whether the bargain will work; whether Beth can
    survive even if the child returns; what his sacrifice has set
    in motion for others (Sarah, the family)
  note: "Tom knows his own sacrifice but not its consequences for
         the web. His information about Beth is intimate but his
         information about the broader situation is minimal."

configuration: >
  Desperate sacrifice. The most intense configuration in the web:
  overwhelming love + overwhelming debt + devastating shared history
  + a projection that may be a lifeline or a delusion. Tom's
  relational power is paradoxical — Beth's grief has more influence
  over him than any person, making her the structurally dominant
  party in a relationship where she is the more helpless. Debt
  is the engine: Tom's sense of what he owes Beth (for the child,
  for the grief, for the distance between what she needed and
  what happened) drove him to the Witch, into the Shadowed Wood,
  into whatever he has become. The emergent dynamic is love
  expressed as self-destruction — power exercised not over another
  but for another, at ruinous cost to oneself and unknowing cost
  to one's family.
```

### Beth → Tom

```yaml
trust:
  competence: 0.8
  intentions: 0.9
  loyalty: 0.9

affection: 0.9        # She loves him

debt: 0.3             # She doesn't know what he has done for her
  note: "The debt she doesn't know she owes is the tragic core of
         their relationship."

history:
  quality: passionate → devastating
  temporal_layer: sediment (love) + topsoil (grief that has swallowed everything)

projection:
  beth_projects_onto_tom: unknown  # She is too deep in grief to project
  note: "Beth is barely characterized in the source material beyond
         her grief and her role as the emotional anchor for Tom's sacrifice."

information_state:
  knows: Tom loves her; the child is dead; she is grieving
  does_not_know: The fair folk bargain; Tom's journey to the Witch;
    what Tom has become; that Tom's sacrifice was for her; that
    Sarah is searching for Tom
  note: "Beth's information state is the most impoverished regarding
         the plot. She is at the center of the story's emotional engine
         but knows almost nothing about what her grief has set in motion."

configuration: >
  Grief eclipsing everything. Beth's grief is so total that it
  restructures the relational dynamics around her. She loves Tom,
  trusts him, but is barely present as a relational agent — her
  grief has consumed the surface area through which she could
  engage. The emergent dynamic is indirect power: Beth's condition
  drives Tom's sacrifice, which drives Sarah's quest, which drives
  the plot — but Beth herself is passive within the web. She is
  the emotional center from the periphery: the character with the
  most narrative influence and the least relational agency. Her
  grief functions less as emotion and more as force — like weather
  or geography, a condition that shapes what others do.
```

### Adam → Tom

```yaml
trust:
  competence: 0.4     # Tom is a mortal farmhand
  intentions: 0.6     # Tom's sacrifice was genuine
  loyalty: 0.1        # Tom is not Adam's concern

affection: 0.1

debt:
  magnitude: 0.4      # Adam facilitated Tom's journey to the Witch
  type: spiritual
  note: "Adam carries some responsibility for Tom's state."

history:
  depth: 0.3
  quality: transactional
  temporal_layer: topsoil — recent, instrumental

projection:
  adam_projects_onto_tom: "A brave young man whose sacrifice
    serves the Queen's purposes, whether he knows it or not."
  accuracy: moderate

information_state:
  knows: Tom's condition; the Witch's bargain; the Queen's interest;
    how Tom's sacrifice serves the larger plan
  does_not_know: Whether Tom retains enough of himself to resist;
    how Tom's connection to Sarah might complicate the plan
  note: "Adam's information about Tom is instrumental — he knows
         Tom's role in the plan, not Tom's inner life."

configuration: >
  Instrumental facilitation. Adam helped Tom reach the Witch not
  from compassion but because Tom's sacrifice serves the Queen's
  purposes. The substrate shows minimal affection, minimal loyalty,
  moderate debt (Adam's complicity), and a projection that reduces
  Tom to his function in a larger plan. The emergent dynamic is
  that of handler to pawn — a relationship where one party sees
  the other as a means, not an end. Tom's unawareness of this
  dynamic is the information asymmetry that makes it possible.
```

---

## Emergent Power Dynamics: Summary

The power dynamics in this web are nowhere stored but everywhere present. They emerge from the interaction of substrate dimensions and structural position:

### By Configuration Type

**Structural gatekeeping** (Adam → Sarah, Adam → Tom): Power from topological position — controlling the only path, holding the information advantage, serving interests the other party cannot see. The power is vast but brittle: it depends on the controlled party having no alternative path and no knowledge of the controller's true motives.

**Hidden capability** (Kate → John, Kate → Sarah): Power from asymmetric knowledge and undisclosed capacity. Kate's relational power is latent — she holds more than anyone knows, exercises less than she could, and makes deliberate choices about what to reveal. This configuration is stable as long as the gap remains unexamined.

**Love as force** (Tom → Beth, Sarah → Tom): Power from affection so intense it drives sacrifice. This is not power *over* another but power *through* another — Beth's grief compels Tom's sacrifice; Sarah's love compels her quest. The power flows through the channel of affection, shaped by debt and history.

**Desperate seeking** (Sarah → Adam): Power from need — Sarah needs what Adam has and accepts the dependence this creates, while maintaining the intuitive wariness that may ultimately save her. The power dynamic is asymmetric (Adam holds more) but Sarah's strategic caution keeps it from becoming total.

**Protective blindness** (John → Sarah, John → Kate): The absence of power from the absence of information. John's relational authority (parent, provider, protector) is structurally irrelevant because it exists in a domain the story has left behind. His power is real in the mortal world and null in the Shadowed Wood.

**Grief as condition** (Beth → the web): Power not as relational dynamic but as environmental force. Beth's grief operates less like a character's emotion and more like weather — a condition that shapes what everyone else does without being directed at anyone.

### The Most Dangerous Configuration

The most dangerous power configuration in the web is Adam → Sarah: **confident control built on a misreading**. Adam has the structural advantage (Gate, information, the Queen's backing, the Wolf). He has correctly read the substrate (Sarah is young, mortal, in debt to him). But his projection — that she will fail — is catastrophically wrong. Power built on accurate structural advantage and inaccurate projection is the configuration most likely to produce reversal: the moment Sarah sees the path the Wolf cannot, Adam's entire calculus collapses, and structural advantage becomes irrelevant because the controlled party has found a way around the gate.

---

## Toward the Psychological Frame

The revision from stored power to emergent power creates an information architecture challenge. The old model (6 dimensions per edge, each a stored value) was computationally simple: the Storykeeper reads the values and passes relevant ones to Character Agents. The new model (5 substrate dimensions + information state + configuration + network position) is richer but requires *interpretation* — someone or something must read the substrate and compute the emergent dynamics.

This is where the psychological frame concept (see `docs/foundation/power.md`) becomes architectural:

1. **The substrate** (trust, affection, debt, history, projection, information state) is stored in the relational web. This is data.

2. **The network topology** (structural position, bridging, clustering, path length) is computed from the web's shape. This is structure.

3. **The configuration** (the emergent relational dynamic) arises from the interaction of substrate + topology in a specific scene context. This is interpretation.

4. **The psychological frame** is the configuration compressed into a form a Character Agent can inhabit: "You are wary, dependent, watching for the trap. You trust his knowledge but not his motives. You owe him a debt you did not choose. Your best strategy is to use what he offers while keeping your weight centered."

The frame is the interface between the computational-predictive layer (which reads substrate, topology, and context to compute configurations) and the agentic-generative layer (which performs the character). The substrate provides the landscape. The configuration describes the terrain. The frame orients the walker. The Character Agent walks.

### Token Budget (Revised)

Each directed edge with the new structure is ~250-400 tokens (larger than before due to information_state and configuration fields). A character with 5 relationships = 1,250-2,000 tokens of raw relational data. However, the psychological frame for a specific scene should compress the relevant relationships into ~200-400 tokens — a net reduction in what the Character Agent actually receives, despite richer underlying data.

The frame computation layer absorbs the complexity so the performing agent doesn't have to.

---

## Design Implications (Revised)

1. **Power is diagnostic, not prescriptive.** The configuration annotations in this document are analytical — they describe the emergent dynamics a reader can identify from the substrate. The system's frame computation layer should produce similar analyses, but the Character Agent should receive the *feeling* of the configuration, not the analysis. Sarah should feel "wary, dependent, watching" — not be told "you are in a wary-dependence configuration."

2. **The projection field remains the primary source of dramatic irony.** Projections that are inaccurate create the conditions for surprise, reversal, and growth. The system should track projection accuracy and flag moments when reality and projection are about to collide — these are high-mass narrative moments.

3. **Information state drives plot progression.** Every major plot beat in TFATD is an information revelation: Sarah learning about Beth, Sarah discovering Kate's true nature, Sarah seeing the path the Wolf cannot. The information_state field on each edge is the Storykeeper's primary tool for tracking what revelations are available and what their relational consequences will be.

4. **Network position is as important as edge properties.** Adam's power is primarily topological. Kate's power is primarily hidden capability. Beth's influence is primarily from the periphery. The Storykeeper must reason about characters' positions in the web, not just their dyadic relationships.

5. **Debt remains the plot engine.** Tommy→Beth (emotional debt = sacrifice), Adam→Queen (service debt = betrayal), Sarah→quest (family debt = journey), Sarah→Adam (spiritual debt = dependence). Tracking debts and their repayment/default is how the Storykeeper monitors plot progression.

6. **Missing/sparse characters still work.** John and Beth have minimal characterization but their configurations are clear and distinct. The system should handle sparse substrate data by computing configurations from what is available rather than hallucinating what isn't.

7. **The frame computation layer is a new architectural component.** Between the Storykeeper's relational data and the Character Agent's performance, there must be a system that reads substrate, topology, and scene context and produces a compressed psychological frame. This may be an ML inference model, a rules engine, or a hybrid — an open question for the technical specification.
