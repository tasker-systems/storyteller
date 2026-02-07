# The Emotional Model: Plutchik, Sedimentation, and the Self-Referential Edge

## Purpose

This document establishes the foundational framework for how the storyteller system models, represents, and translates emotional life into LLM-performable character behavior. It addresses what we call the **translation problem**: how numerical tensor values become natural language behavioral guidance that steers a Character Agent's performance.

The existing technical specifications (`tensor-schema-spec.md`, `tensor-case-study-sarah.md`) define the *structures* that hold emotional data --- EmotionalState, PersonalValue, Motivation, EchoPattern. This document provides the *theory* that grounds those structures: why these dimensions, how they interact, and how the system bridges the gap between computational representation and generative performance.

### The Translation Problem

Traditional NLP moves from language to numbers --- sentiment analysis, entity extraction, topic modeling. This system must move in the opposite direction: from structured numerical representations to language that an LLM can inhabit as a character. The challenge is not merely technical but philosophical. A number cannot feel. A tensor axis labeled "grief: 0.8" carries no phenomenological weight for a machine. Yet the LLM that receives a well-constructed psychological frame *does* know what grief sounds like, how it moves through language, how it constrains and colors what a person says.

The insight is that we do not need the system to *understand* emotions. We need it to construct a bridge from structured data to the LLM's existing capacity for emotional language. The bridge is the psychological frame --- and this document describes the theoretical architecture that makes the frame possible.

---

## Plutchik's Wheel of Emotions

### Why Plutchik

Robert Plutchik's psychoevolutionary theory of emotion (1980) proposes eight primary emotions arranged in opposing pairs, with intensity gradients and combinatorial composition rules. We adopt the *structural properties* of this model as the foundation for our first emotional grammar (see "Emotional Grammars" below). The specific primaries are one vocabulary, not the vocabulary --- but the structural properties are what we commit to:

1. **Dimensional parsimony.** Eight primary emotions yield a manageable dimensional space. The alternative --- enumerating discrete emotion categories --- produces a combinatorial explosion (Ekman's basic emotions expand through cultural variation into hundreds of named states). Plutchik keeps the primary space small while generating rich composite states through combination.

2. **Bipolar organization.** The opposing pairs map naturally to the tensor's existing bipolar axis representation. Joy-sadness, trust-disgust, fear-anger, surprise-anticipation are already structured as the kind of spectra the system models well.

3. **Intensity gradients.** Each primary emotion exists on an intensity spectrum (e.g., apprehension -> fear -> terror). This maps directly to the `[0.0, 1.0]` intensity range in the tensor's AxisValue representation. The gradient avoids the false precision of categorical labels while preserving semantic richness.

4. **Dyadic composition.** Plutchik defines composite emotions as combinations of adjacent or near-adjacent primaries: joy + trust = love; anger + anticipation = aggressiveness; fear + surprise = awe. This provides a *generative grammar* for emotional states --- the system can represent complex emotional textures as configurations of primary dimensions rather than requiring a separate type for every composite.

5. **LLM compatibility.** Plutchik's model is well-represented in training corpora. LLMs have encountered these concepts extensively and can generate behaviorally appropriate language when guided by Plutchik-derived frames. This is not incidental --- it means the bridge from tensor to performance has solid footing on both sides.

### The Eight Primary Emotions

| Pair | Positive Pole | Negative Pole | What It Governs |
|------|--------------|---------------|-----------------|
| 1 | **Joy** | **Sadness** | Hedonic valence --- the basic experience of gain/loss, connection/disconnection |
| 2 | **Trust** | **Disgust** | Social evaluation --- acceptance/rejection of others, self, and conditions |
| 3 | **Fear** | **Anger** | Threat response --- withdrawal/engagement when safety is threatened |
| 4 | **Surprise** | **Anticipation** | Temporal orientation --- openness to the unexpected vs. preparation for the expected |

### Intensity Gradients

Each primary occupies a continuum from mild to extreme:

```
serenity ---- joy -------- ecstasy
acceptance -- trust ------ admiration
apprehension- fear ------- terror
distraction - surprise --- amazement
pensiveness - sadness ---- grief
boredom ----- anticipation- vigilance
annoyance --- anger ------ rage
interest ---- disgust ---- loathing       (NB: interest is the mild form)
```

The tensor represents these as intensity values on the `[0.0, 1.0]` scale per pole. A character experiencing mild sadness sits at `sadness_intensity: 0.3`; one in the grip of grief sits at `sadness_intensity: 0.8`. The named gradations (serenity, joy, ecstasy) are not stored --- they are computed at frame synthesis time by mapping intensity to natural language register. This keeps the data layer clean while giving the frame synthesis step rich vocabulary.

### Dyadic Combinations

Plutchik defines three orders of combination:

**Primary dyads** (adjacent emotions):
- Joy + Trust = **Love**
- Joy + Anticipation = **Optimism**
- Trust + Fear = **Submission**
- Fear + Surprise = **Awe**
- Surprise + Sadness = **Disapproval** (or *Disappointment*)
- Sadness + Disgust = **Remorse**
- Disgust + Anger = **Contempt**
- Anger + Anticipation = **Aggressiveness**

**Secondary dyads** (one emotion apart):
- Joy + Fear = **Guilt**
- Trust + Surprise = **Curiosity**
- Fear + Sadness = **Despair**
- Surprise + Disgust = **Disbelief** (or *Unbelief*)
- Sadness + Anger = **Envy**
- Disgust + Anticipation = **Cynicism**
- Anger + Joy = **Pride**
- Anticipation + Trust = **Fatalism** (or *Hope*)

**Tertiary dyads** (two emotions apart):
- Joy + Surprise = **Delight**
- Trust + Sadness = **Sentimentality** (or *Nostalgia*)
- Fear + Disgust = **Shame**
- Surprise + Anger = **Outrage**
- Sadness + Anticipation = **Pessimism** (or *Anxiety*)
- Disgust + Joy = **Morbidity** (or *Ambivalence*)
- Anger + Trust = **Domination**
- Anticipation + Fear = **Anxiety**

**What the system does with co-activation --- mood-vectors, not named dyads:** The system does not store composite emotions as separate types, and --- critically --- it does not reduce co-activated primaries to named dyadic labels. Plutchik's named dyads (joy + trust = "love") are culturally specific interpretations that strip context. The same co-activation of joy and trust, directed at another person in a context of low debt and high projection, feels qualitatively different from joy and trust directed at self in a context of high self-doubt. Same primaries, entirely different emotional reality.

Instead, co-activated primaries form **mood-vectors** in emotional-dimensional space. The frame synthesis step describes the *felt quality* of the configuration ("something warm and wary at the same time") rather than naming a composite emotion. When multiple primaries are co-activated, the system notes the configuration as emotionally complex and synthesizes its quality from the full context --- the relational direction (self-referential? toward whom?), the values that filter it, the motivations that drive it, the scene conditions that evoke it. The dyadic tables above are reference material for frame synthesis, not computational categories. They help the LLM recognize what emotional territory a configuration occupies without collapsing it to a label.

This is the generative grammar: eight dimensions produce a rich emotional vocabulary through combination and context without categorical explosion or reductive naming.

---

## Emotional Grammars

### The Grammar as Parameter

The specific eight primaries of Plutchik's model are culturally situated in the Western psychological tradition. They are not universal. Traditional East Asian medicine models emotional life through organ systems and elemental correspondences (grief/lung/metal, fear/kidney/water, anger/liver/wood, joy/heart/fire, worry/spleen/earth) --- a framework with different primaries, different opposition relationships, and different composition rules, but the *same structural properties*: a bounded set, complementary and opposing pairs, intensity gradients, and generative combination.

The system should not hardcode Plutchik's eight as THE emotions. Instead, we introduce the concept of an **emotional grammar**: a pluggable vocabulary that defines how a particular cultural, species, or ontological context structures emotional experience.

### Grammar Structure

An emotional grammar specifies:

```
EmotionalGrammar {
  id: GrammarId
  name: String                           // e.g., "plutchik_western", "wu_xing_tcm", "fey_court"
  primaries: Vec<EmotionalPrimary>       // the foundational emotional dimensions
  oppositions: Vec<(PrimaryId, PrimaryId)>  // which primaries oppose each other
  intensity_range: (f32, f32)            // typically [0.0, 1.0]
  composition_rules: CompositionModel    // how primaries interact when co-activated
  register_vocabulary: RegisterMap       // intensity → natural language gradations
  cultural_context: String               // what tradition or mode of being this grammar models
}

EmotionalPrimary {
  id: PrimaryId
  name: String                           // human/LLM readable
  description: String                    // what this primary captures
  intensity_labels: Vec<(f32, String)>   // threshold → label mappings
  // e.g., [(0.3, "apprehension"), (0.6, "fear"), (0.9, "terror")]
}

CompositionModel =
  | MoodVector    // co-activated primaries form directional vectors in emotional space
  | Elemental     // primaries interact through generation/control cycles (wu xing)
  | Resonance     // primaries amplify/dampen through harmonic relationships
  | ... // extensible
```

### The First Grammar: Plutchik-Derived Western

The grammar we implement first uses Plutchik's eight primaries with the mood-vector composition model. This grammar is well-suited for human characters in Western-influenced narrative contexts and has the advantage of deep LLM familiarity.

Future grammars might include:
- **Wu Xing (Five Phases)**: For narratives grounded in East Asian cultural contexts, or for entities whose emotional architecture maps to elemental correspondences
- **Non-human grammars**: For entities like the Wolf, whose emotional life may not decompose along human lines at all. A grammar with primaries like hunger/satiation, pack/solitude, hunt/rest, obedience/defiance
- **Fey/Mythic grammars**: For entities operating on different ontological registers, where emotional primaries might be closer to aesthetic categories (beauty/grotesque, wonder/tedium, binding/dissolution)

The grammar is a parameter of the character tensor, not a global system setting. Different entities in the same scene can operate under different grammars. The frame synthesis step must account for cross-grammar interactions: when a character operating under a Western grammar encounters an entity operating under a non-human grammar, the communicability friction is partly a grammar translation problem.

### Design Decision: Grammar from the Outset

Even though the first implementation will only enumerate one grammar (Plutchik-derived Western), the system treats the grammar as a parameter from the beginning. This means:

- The tensor schema references grammar-relative primary IDs, not hardcoded emotion names
- Frame synthesis receives the grammar as context, not as assumption
- The self-referential edge's emotional dimensions are grammar-relative
- Test content (Bramblehoof, Pyotir) is tagged with its grammar

This costs almost nothing in implementation complexity and prevents the kind of hardcoding that becomes architectural debt later.

---

## The Tripartite Psychological Frame

The emotional model rests on three interdependent pillars. Each contributes a distinct kind of information to the psychological frame, and each operates on different timescales and at different levels of awareness.

### Pillar 1: Emotions (Plutchik 8D)

Current affective state --- the character's weather. Emotions are volatile, responsive, and always topsoil (though sustained emotions sediment). The eight primary dimensions plus their intensity gradients and dyadic interactions provide the immediate felt quality of experience.

**Timescale:** Scene-level. Emotions shift within scenes in response to events, revelations, and relational dynamics.

**Awareness:** Emotions vary in how accessible they are to the character. A character may be fully aware of their anger but blind to the fear underneath it. This is where the awareness dimension (below) intersects with the emotional layer.

**Computational role in frame:** Emotions determine the *tone* and *register* of the character's behavior --- whether they speak carefully or rashly, whether they lean toward or pull away, whether their voice carries warmth or edge. The frame synthesis step translates emotional configuration into behavioral register guidance.

### Pillar 2: Values (Directional Commitments)

What the character is committed to --- the interpretive lenses through which they read the world. Values are more stable than emotions and operate primarily at the sediment and bedrock layers. The existing `PersonalValue` type in the tensor schema captures this well: strength, temporal layer, awareness level, perception filter, support/suppression relationships.

Values do not need a Plutchik-style dimensional model because they are domain-specific rather than universal. A character's value of "people should do what needs doing" is not a point on a universal axis --- it is a specific commitment that filters perception in a specific way. The system represents values as the tensor schema already defines them: named commitments with strength, awareness, and relational connections to other tensor elements.

**Timescale:** Sediment to bedrock. Values shift slowly under sustained pressure or through revelatory experiences (bedrock cracks).

**Awareness:** The five-level gradient from the tensor schema (Articulate, Recognizable, Preconscious, Defended, Structural) applies directly. A character may articulate some values while being blind to others that powerfully shape their behavior.

**Computational role in frame:** Values determine *what the character notices and how they interpret it*. The perception filter mechanism means that two characters in the same scene will read the same events differently because their values foreground different aspects. The frame synthesis step translates active values into interpretive orientation --- "you see Adam's evasiveness as a refusal to act, because people should do what needs doing."

### Pillar 3: Motivations (Goal-Directed States)

What the character wants --- the engine of action. The existing surface/deep/shadow layering captures the essential structure: surface wants (conscious goals), deep wants (the needs those goals serve), and shadow wants (the desires the character would deny). Motivations are the most dramatically productive element because the friction between layers generates subtext.

**Timescale:** Variable. Surface wants can shift within a scene (a new piece of information changes the immediate goal). Deep wants operate at the sediment layer. Shadow wants are sediment or bedrock, surfacing only under specific activation conditions.

**Awareness:** Mirrors the surface/deep/shadow layering. Surface wants are typically Articulate. Deep wants may be Recognizable or Preconscious. Shadow wants are characteristically Defended or Structural.

**Computational role in frame:** Motivations determine *what the character is trying to do* --- both overtly and covertly. The frame synthesis step translates the motivational stack into behavioral orientation --- "you want to help Pyotir, but underneath that is a need to prove that creativity still matters, and underneath that is a fear that you failed him years ago."

### The Tripartite Integration

The three pillars are not independent channels. They interact:

- **Emotions activate motivations.** Anger may surface a shadow want for confrontation. Joy may reinforce a deep want for connection. The activation conditions on motivations (factual and interpretive) mediate this interaction.

- **Values filter emotions.** A character who values stoicism may suppress the expression of fear. A character who values authenticity may amplify emotional display. The suppression and support relationships between values and other tensor elements encode this.

- **Motivations color emotions.** The *meaning* of an emotional state depends on what the character wants. Sadness while pursuing a lost friend is different from sadness while mourning a failure. The motivational context determines how the frame synthesis step interprets the emotional configuration.

- **Echoes cut across all three.** An echo pattern can simultaneously activate an emotional surge (topsoil), surface a shadow motivation (sediment to topsoil), and challenge a value (pressure on bedrock). This is why echoes are the most dramatically powerful mechanism in the tensor system.

The frame computation pipeline integrates all three pillars simultaneously. It does not compute emotions, then values, then motivations in sequence. It reads the full tripartite configuration and produces a unified frame that weaves emotional tone, interpretive orientation, and behavioral direction into a single coherent prompt fragment.

---

## Awareness: The Orthogonal Dimension

The tensor-schema-spec already defines `AwarenessLevel` with five gradations (Articulate, Recognizable, Preconscious, Defended, Structural). This document extends that concept with a key insight: **awareness is orthogonal to the geological temporal layers.**

### The Intersection

A bedrock trait can be fully Articulate (Sarah's "I do what needs doing" --- she can state this explicitly). A topsoil emotion can be Defended (the anger at Tommy that she refuses to feel). The temporal layer describes *how resistant to change* an element is. The awareness level describes *how accessible to conscious experience* the element is. These are independent dimensions:

```
                     Awareness
                Articulate ←→ Structural
                     |
Temporal    topsoil  |  grief (Articulate)      suppressed anger (Defended)
Layer               |
           sediment  |  self-reliance (Recognizable)  growing awareness of loss (Preconscious)
                     |
           bedrock   |  "I do what needs doing" (Articulate)   fear of the boundary (Structural)
                     |
          primordial |  [for non-human entities: constitutive patterns]
```

A bedrock element at Structural awareness is the most deeply buried and the hardest to access --- it shapes everything but can never be articulated. A topsoil element at Articulate awareness is the most immediately present --- the character feels it and can name it. But a bedrock element can also be Articulate (core identity statements), and a topsoil element can be Defended (emotions the character is actively suppressing right now).

### Why This Matters for Frame Synthesis

The awareness level determines how an element appears in the psychological frame:

- **Articulate** elements appear as direct statements: "You are angry. You know you are angry."
- **Recognizable** elements appear as named but not foregrounded: "There is something that feels like jealousy, though you might not call it that."
- **Preconscious** elements appear as behavioral tendencies without explanation: "You find yourself watching the door, though you couldn't say why."
- **Defended** elements appear as their *absence* or their *compensation*: "You are absolutely fine. You are not thinking about it." The frame must convey the defense mechanism, not the defended content.
- **Structural** elements do not appear at all in the frame's explicit content. They shape the frame's *structure* --- the patterns of what is noticed and what is ignored, the assumptions that go unexamined. The Character Agent enacts them without being told about them.

This is a critical design principle: **the frame should not make the character more self-aware than they are.** If Sarah's anger at Tommy is Defended, the frame should not say "you are angry at Tommy." It should say something like "when someone mentions abandonment, something tightens in your chest and you change the subject." The Character Agent's generative capacity then produces behavior that *shows* the defended anger without naming it. This is how subtext works.

---

## The Self-Referential Edge

### Emotions as Relationship-to-Self

The relational web models directed edges between entities. Each edge carries a multi-dimensional substrate (trust, affection, debt, history, projection, information state). Power emerges from the *configuration* of these dimensions in context.

The key insight of this document: **the character's emotional tensor is a self-referential edge** --- a directed edge from the entity to itself. This is not a metaphor. It is a structural claim about how emotions should be modeled in the relational graph.

Consider: the relational edge from Sarah to Adam carries trust[competence] and distrust[intentions], affection and wariness, debt in both directions, historical patterns, and projections about who Adam really is. These dimensions interact to produce the felt quality of that relationship and the power dynamics that emerge from it.

Now consider Sarah's relationship to herself. She trusts her own competence (self-efficacy) but doubts whether she is *enough* (self-worth). She feels affection for herself in some registers (pride in action) and frustration in others (impatience with her own fear). She carries historical patterns of self-reliance that she can draw on, and projections about who she might become that she is not yet aware of. These interact to produce the felt quality of self-relationship --- what a psychologist might call self-concept, self-esteem, or internal working model.

If we model this as a loopback edge using the same relational schema:

```
Edge: Sarah → Sarah
  trust:
    competence: 0.7     -- she knows she can act
    intentions: 0.8     -- she trusts her own motives
    reliability: 0.5    -- she fears the boundary of her own thinking
  affection: 0.6        -- she is kind to herself, mostly
  debt: 0.0             -- no self-obligation (yet)
  history:
    pattern: "years of doing what needs doing"
    weight: 0.8
  projection:
    content: "someone who can handle this"
    accuracy: 0.6       -- she wants this to be true more than she knows it
  information_state:
    knows: "her own competence, her love for Tommy"
    does_not_know: "the depth of her anger at Tommy, the source of her gift"
```

This representation unifies emotional self-knowledge with the relational web's existing schema. The self-edge participates in all the same computational processes as inter-entity edges: it contributes to power configuration computation, it has information asymmetries (she doesn't know things about herself), and it can shift in response to events (a success raises self-trust; a failure erodes it).

### The Communicability Gradient Applied to Self

The foundation documents establish that communicability between entities exists on a gradient with four dimensions: surface area of communicability, friction of translation, timescale, and ability to turn toward (Buber's I/Thou). This gradient applies equally to the self-relationship:

**Surface area:** How much of the self is available for conscious experience? Some people have broad self-awareness (high surface area); others have narrow windows into their own inner life. Characters with Articulate awareness across many dimensions have high self-communicability. Characters with mostly Defended or Structural elements have low self-communicability.

**Friction of translation:** How easily can the character translate inner experience into conscious understanding? A character with high emotional intelligence has low friction --- they feel something and can name it. A character like the Wolf has enormous friction --- the entity's mode of being is so different from human consciousness that translating inner states into anything like understanding is a constitutive challenge.

**Timescale:** Self-knowledge operates on different timescales. Topsoil self-awareness shifts within scenes. The deeper understanding of "who I am" operates on sediment and bedrock timescales. A character may learn something about themselves in a moment of revelation, but integrating that knowledge takes time.

**Ability to turn toward (I/Thou):** Can the character attend to their own inner life with genuine curiosity and openness? Or do they flinch away from self-examination? The Defended awareness level represents precisely this --- a character who cannot or will not turn toward aspects of their own experience. The therapeutic traditions (and the contemplative ones) understand this as the central challenge of self-knowledge: not information gathering but willingness to look.

This means the self-referential edge is not merely a convenient modeling trick. It captures something real about the phenomenology of selfhood: that we are always in relationship with ourselves, that this relationship has the same structural features as our relationships with others, and that the communicability gradient --- the same gradient that makes it hard for Sarah to understand Adam --- also makes it hard for Sarah to understand Sarah.

### Implications for Frame Computation

The self-referential edge changes how the frame computation pipeline works:

1. **Step 4 (Configuration Computation)** now includes the self-edge alongside inter-entity edges. The power configuration for a scene includes the character's relationship to themselves --- self-doubt eroding capability, self-trust amplifying it.

2. **Step 5 (Frame Synthesis)** can draw on the self-edge to generate introspective frame content. Rather than only describing the character's emotional state as a flat list of active feelings, the frame can convey the *relationship* the character has to those feelings: "You feel the fear and you are not afraid of the fear" vs. "You feel the fear and it threatens to unmake you."

3. **Information asymmetry on the self-edge** produces the most dramatically interesting frame content. What the character does not know about themselves is what creates subtext. The frame must encode these asymmetries as behavioral tendencies without making the character explicitly aware of what is hidden.

---

## Sedimentation Mechanics

### How Emotional Topsoil Becomes Sediment

The geological temporal model establishes four layers (topsoil, sediment, bedrock, primordial) with different change rates and resistance. The tensor-schema-spec defines `sediment_threshold` on EmotionalState --- an intensity-duration product at which a topsoil emotion transitions to sediment. This section provides the full mechanics.

#### The Accumulation Model

Emotional sedimentation follows an accumulation model, not a threshold model. It is not the case that an emotion at intensity 0.7 for 8 scenes suddenly becomes sediment on scene 9. Rather, each scene of sustained emotional experience *deposits* a thin layer. When the accumulated deposit exceeds the sedimentation threshold, the emotional pattern has become part of the character's settled experience.

```
Sedimentation:
  deposit_per_scene = f(intensity, reinforcement_count, absence_of_contradiction)
  accumulated_deposit += deposit_per_scene
  if accumulated_deposit >= sedimentation_threshold:
    create sedimentary pattern from emotional state
    reduce topsoil intensity (the immediate feeling fades; the pattern remains)
```

**Intensity matters:** Higher-intensity emotions deposit faster. Grief at 0.8 deposits more per scene than melancholy at 0.3.

**Reinforcement matters:** An emotion that is re-triggered by events in each scene deposits faster than one that merely persists by inertia. Grief that is *sustained* by Tommy's absence deposits differently from grief that is *re-activated* by encountering reminders.

**Contradiction matters:** An emotion that encounters contradicting experiences deposits more slowly. Joy that is interrupted by fear does not sediment as cleanly as joy that accumulates without disruption. The deposit function should account for the presence of contradicting emotional states.

#### What Sedimentary Emotions Look Like

When a topsoil emotion sediments, it transforms. It is no longer a *feeling* --- it is a *pattern*. The character does not actively feel grief anymore; they carry a sedimentary pattern of grief that shapes their responses to loss-related situations. This is the difference between acute mourning and the way a person who has grieved deeply responds to sad movies ten years later.

Computationally, a sedimentary emotional pattern:

- Has a much lower decay rate than its topsoil origin (half-life measured in story arcs, not scenes)
- Contributes to contextual triggers rather than to current emotional state (it shifts other axes when activated, rather than being felt directly)
- May have a different awareness level than its topsoil origin (the original grief was Articulate; the sedimentary pattern may be Preconscious --- the character doesn't notice it shaping their reactions)
- Can be reactivated as topsoil by echo patterns (the sedimentary grief surges back when conditions rhyme with the original loss)

#### The Sediment-to-Bedrock Transition

Sedimentary patterns can, under extraordinary pressure or sustained accumulation, transition to bedrock. This is rare and represents a fundamental change in the character's identity. Sarah's "identity as someone who acts" is bedrock --- it was once a sedimentary pattern of repeated self-reliance that became constitutive of who she is.

The conditions for sediment-to-bedrock transition:

- Sustained presence across many story arcs (not just scenes)
- Consistent reinforcement without contradiction
- Integration with other bedrock elements (the pattern becomes structurally load-bearing)
- Often accompanied by a reduction in awareness level (what was once a conscious choice becomes an unexamined assumption)

This transition is typically authored rather than computed for major characters (the story designer establishes what is bedrock). For characters who develop through extended play, the system should be capable of recognizing candidates for bedrock transition, but the actual transition should require Storykeeper confirmation --- this is too important to happen automatically.

### Erosion and Bedrock Cracks

Sedimentation is not only additive. Sedimentary patterns can erode, and bedrock can crack.

**Sediment erosion** occurs when sustained contradicting experience wears away a pattern. A character's sedimentary distrust of strangers might erode through repeated positive encounters with unfamiliar people. Erosion is slow --- it requires many scenes of contradicting evidence --- and the eroded pattern leaves a trace (a reduced-strength version of itself, or a context-specific residual: "distrusts strangers in cities but not in the country").

**Bedrock cracks** are rare, dramatic events. They occur when a single experience is so powerful that it fractures a constitutive pattern. Learning that Tommy chose Beth over family could crack Sarah's "family bonds are the strongest thing" bedrock value. A bedrock crack does not destroy the element --- it introduces a fault line that permanently changes how the element functions. The cracked value still exists but now carries an exception, a wound, an asterisk.

Bedrock cracks are the most narratively significant tensor events. They should be authored on the narrative graph as mandated shifts (guaranteeing they happen when conditions are met) and should produce echo patterns that persist indefinitely.

### Three Timescales of Sedimentation

Sedimentation mechanics must account for three distinct timescales that do not necessarily align:

**Scene-distance** is topological position on the narrative graph. Two scenes may be adjacent on the graph (the player moves directly from one to the next) but separated by months of story-chronological time. Scene-distance governs how many processing cycles the system has executed --- it is the system's clock, not the character's clock.

**Story-chronology** is diegetic time within the narrative. How much time has passed for the characters between scenes. A time-skip of six months between adjacent scenes means the character has lived through half a year of unplayed experience. Story-chronology is what the character's body and mind measure.

**Historical-experiential time** is the accumulation of lived experience that actually drives sedimentation. This is closer to story-chronology than to scene-distance, but not identical: a character who spends six months in uneventful routine accumulates less experiential weight than one who spends six months in crisis.

Sedimentation should track historical-experiential time, not scene-distance. When the Storykeeper processes a scene transition with a chronological gap, it must account for unsimulated experience: "Six months have passed. Pyotir's despair has deposited further into sediment. The flicker of joy from Bramblehoof's visit has eroded without reinforcement." This between-scene processing is part of the Storykeeper's state management responsibility.

**Design decision:** Within a single story graph, organic bedrock-level change through sedimentation alone would be rare. Bedrock changes within a single narrative are almost always authorially intended (mandated shifts). Organic bedrock evolution through sedimentation would require either a story-series (multiple connected story graphs spanning significant chronological time) or explicit chronological leaps within the narrative. The system should support both, but the first implementation assumes that bedrock changes are authored.

---

## Mapping to the Existing Tensor Schema

This emotional model does not replace the tensor-schema-spec. It provides the theoretical grounding for several of its structures and extends them in specific ways.

### What Maps Directly

| This Document | tensor-schema-spec Element | Notes |
|---|---|---|
| Emotional grammar | (new) | New concept: EmotionalGrammar as a pluggable parameter on the character tensor. First grammar: Plutchik-derived Western. |
| Grammar primaries | EmotionalState | Extend EmotionalState to reference grammar-relative primary IDs. The existing `valence: positive/negative/mixed` becomes derivable from the primary configuration. |
| Intensity gradients | EmotionalState.intensity | Already present. The named gradations (serenity/joy/ecstasy) are frame synthesis vocabulary, not stored data. |
| Values as pillar | PersonalValue | Already well-specified. No changes needed. |
| Motivations as pillar | Motivation (surface/deep/shadow) | Already well-specified. No changes needed. |
| Awareness dimension | AwarenessLevel | Already defined with five gradations. Extend application to EmotionalState (currently only on PersonalValue). |
| Sedimentation threshold | EmotionalState.sediment_threshold | Already present. This document provides the full accumulation model. |
| Echo activation | EchoPattern | Already well-specified. This document adds the insight that echoes cut across all three pillars simultaneously. |

### What This Document Adds

1. **Emotional grammars as pluggable parameter.** The concept of interchangeable emotional vocabularies, each with primaries, oppositions, intensity ranges, and composition rules. Plutchik-derived Western grammar is the first implementation; others (wu xing, non-human, fey/mythic) are structurally supported from the outset.

2. **Plutchik-derived primaries as first grammar.** The tensor-schema-spec does not specify *which* emotions to model. This document establishes eight Plutchik-derived primaries as the foundational set for the first grammar.

3. **Mood-vectors over named dyads.** Co-activated primaries form directional vectors in emotional space rather than being reduced to named composite emotions. The felt quality depends on relational direction, values, motivations, and scene context.

4. **Tripartite integration model.** The tensor-schema-spec defines elements (emotions, values, motivations) with relationships between them. This document provides the theoretical argument for why these three and how they interact in frame computation.

5. **Self-referential edge.** New architectural concept. The character's emotional self-knowledge modeled as a loopback edge in the relational graph, using the same schema as inter-entity edges.

6. **Communicability gradient applied to self.** Extends the world-design concept to the internal domain.

7. **Full sedimentation mechanics.** The tensor-schema-spec mentions sediment_threshold. This document provides accumulation, erosion, and bedrock crack mechanics, plus the three-timescale model (scene-distance, story-chronology, historical-experiential time).

8. **Awareness as orthogonal to temporal layer.** Explicit treatment of the independence of these two dimensions, with implications for frame synthesis.

9. **Frame synthesis register guidance.** How awareness level determines the linguistic register of frame content (explicit statement vs. behavioral tendency vs. conspicuous absence).

10. **Coherence defaults for generated entities.** Authored characters can be intentionally incoherent (that's dramatically interesting). Generated characters default to coherence derivable from their tensor state --- aberrant behavior without narrative resonance is a system failure, not emergence.

---

## Case Study: Bramblehoof and Pyotir in "The Flute Kept"

### Bramblehoof's Emotional Configuration

Using the Plutchik primaries, Bramblehoof's emotional state entering the scene:

```
Primary emotions (topsoil):
  joy:          0.4  -- anticipation of return, the music in him, the wanderer's delight
  sadness:      0.5  -- the grief he carries from Illyana, from Svyoritch's decay
  trust:        0.6  -- openness to people, belief in connection
  disgust:      0.3  -- low-grade revulsion at what the death cult has done to the land
  fear:         0.2  -- background awareness of Whisperthorn's price, the corruption spreading
  anger:        0.3  -- at the forces that crush creative spirit, at what was done to the boy
  surprise:     0.3  -- the wanderer's openness to what he finds
  anticipation: 0.6  -- coming back to Svyoritch, wondering what he'll find

Active dyads (computed, not stored):
  joy + trust          = love (0.5)       -- warmth toward Svyoritch, toward people
  sadness + disgust    = remorse (0.4)    -- guilt about leaving, about not doing enough
  anger + anticipation = aggressiveness (0.4) -- but channeled as creative defiance
  joy + anticipation   = optimism (0.5)   -- the bard's fundamental orientation
```

**Self-referential edge (Bramblehoof -> Bramblehoof):**
```
  trust:
    competence: 0.7     -- he knows his craft, his music, his way with people
    intentions: 0.8     -- he trusts his own heart
    reliability: 0.5    -- he wanders; he cannot always be counted on to stay
  affection: 0.7        -- he likes himself, with gentle humor
  debt: 0.4             -- owes Whisperthorn; owes the places he's left behind
  history:
    pattern: "arriving too late, leaving too soon"
    weight: 0.6
  projection:
    content: "the one who brings the music back"
    accuracy: 0.5       -- is this grandiose? or is it what he's for?
  information_state:
    does_not_know: "whether his presence helps or just reminds people of what they've lost"
```

### Pyotir's Emotional Configuration

```
Primary emotions (topsoil):
  joy:          0.1  -- almost extinguished; flickers when music is mentioned
  sadness:      0.7  -- deep, settled, becoming sedimentary
  trust:        0.2  -- eroded by the village's rejection of his music
  disgust:      0.4  -- at himself for wanting more than the village allows
  fear:         0.5  -- of hoping again, of being noticed, of the death cult
  anger:        0.5  -- suppressed; at the village, at the lord, at his own compliance
  surprise:     0.1  -- nothing surprises him anymore; the world is predictable in its cruelty
  anticipation: 0.2  -- he expects nothing; expecting something is dangerous

Active dyads:
  sadness + anger      = envy (0.6)       -- watching others who have what he cannot
  fear + sadness       = despair (0.6)     -- the dominant emotional chord
  disgust + anger      = contempt (0.4)    -- but directed inward, at his own capitulation
  sadness + disgust    = remorse (0.5)     -- for giving up his music
```

**Self-referential edge (Pyotir -> Pyotir):**
```
  trust:
    competence: 0.3     -- he was good at music; he is adequate at farming
    intentions: 0.4     -- he doesn't trust his own desires anymore
    reliability: 0.6    -- he shows up; he does the work; that's what's left
  affection: 0.2        -- he is hard on himself
  debt: 0.3             -- owes his family labor; owes himself something he can't name
  history:
    pattern: "being told to put away childish things"
    weight: 0.8
  projection:
    content: "someone who works the land like everyone else"
    accuracy: 0.7       -- this is who he is now, and the accuracy of that makes it worse
  information_state:
    does_not_know: "that Bramblehoof remembers him; that his music mattered to anyone"
```

### The Scene as Emotional Collision

When Bramblehoof enters Svyoritch and finds Pyotir working the land, the emotional configurations collide:

**Bramblehoof's frame** (what the Character Agent receives):
> You have come back to Svyoritch and the first thing you see is the boy --- older now, broader, working a furrow with the patient resignation of a man twice his age. You remember a child who played as if the notes were alive. Something in your chest tightens. The optimism you carry everywhere --- the belief that music persists, that creativity is harder to kill than the death cults imagine --- meets the evidence of this field, this bent back, this silence where a flute used to sound. You want to reach out. You always want to reach out. But there is something in you that wonders, quietly, whether reaching out is what *you* need rather than what *he* needs. Whether the wanderer who arrives with warmth and music and then leaves again is helping or just reopening wounds.

**Pyotir's frame:**
> A satyr has come to town. You recognize him --- or rather, something in you recognizes the quality of his presence, the way the air seems slightly more alive around him. You were a child when he last came through. You played for him. That memory is buried under years of doing what you're told, but it's there, and it hurts in a way you weren't prepared for. Your hands are in the dirt and they should stay there. Wanting something else is dangerous --- the last person who stood out got noticed by the wrong people. You are angry and you don't know who you're angry at. You are afraid and the fear tastes like shame. If he asks about the flute, something in you might break open, and you cannot afford that.

Note how the frames operate at different awareness levels:
- Bramblehoof's self-doubt about whether he helps or harms is Recognizable (he can almost name it)
- Pyotir's anger is Defended (he feels it but "doesn't know who he's angry at")
- Pyotir's memory of playing music is Preconscious (it's "buried" and surfaces as hurt, not as narrative)
- Both characters have information asymmetries on their self-edges that create the scene's subtext

---

## From Theory to Implementation

### Stage 1: Prompt Engineering (Current Phase)

For the immediate prototype, emotional frame computation is done through prompt templates. The system:

1. Reads the character's tensor (Plutchik primaries, values, motivations)
2. Reads the relational web (including the self-referential edge)
3. Reads the scene context (setting, stakes, characters present)
4. Constructs a prompt that asks a "frame synthesis" LLM call to produce the psychological frame

This is the Storykeeper's work in the prototype. The Storykeeper receives the full scene state and produces frame fragments for each Character Agent. The quality of the frame depends on the quality of the prompt template and the capability of the LLM.

### Stage 2: Embedding Alignment (Future)

Once the system has accumulated enough examples of good frames (from Stage 1 + human evaluation), the translation can be partially automated through embedding alignment:

- Sentence-transformers map tensor configurations to embedding space
- Good frame examples establish target embeddings for tensor configurations
- New tensor configurations are mapped to nearby embeddings, guiding frame synthesis
- This does not replace the LLM call but constrains it --- providing exemplar frames that anchor the generation

### Stage 3: Trained Frame Model (Future)

The final architecture uses a dedicated ML model (ONNX via `ort`) for frame computation:

- Trained on accumulated tensor-configuration-to-frame pairs
- Runs on a dedicated rayon thread pool (not the LLM call path)
- Produces frame candidates that the Storykeeper can evaluate and refine
- The LLM becomes the final synthesis step rather than the entire pipeline

Each stage builds on the previous. Stage 1 generates training data for Stage 2. Stage 2 generates training data for Stage 3. The theoretical model (Plutchik primaries, tripartite frame, self-referential edge, awareness levels) remains constant across all three stages --- only the *mechanism* of frame computation changes.

---

## Relationship to Other Foundation Documents

- **character_modeling.md**: This document deepens the "Temporal Dynamics" and "Personality Axes / Temperament" sections with Plutchik's model and sedimentation mechanics. The temperament spectra (optimism/pessimism, excitability/steadiness, warmth/reserve) from character_modeling are *personality axes*, not emotions. Plutchik primaries operate at a different level --- they are the volatile weather, not the enduring climate.

- **power.md**: The self-referential edge extends the power framework. Power is computed from substrate configurations; the self-edge is a configuration that contributes to emergent power. Self-doubt erodes capability; self-trust amplifies it. The psychological frame concept from power.md is formalized here as the tripartite frame with explicit pillars.

- **world_design.md**: The communicability gradient applied to self-relationship comes directly from world_design's four-dimensional communicability model. The World Agent, which translates non-character entities, faces the same translation problem with extreme internal friction that a Defended or Structural self-element does.

- **anthropological_grounding.md**: The body-as-nexus concept and the geological temporal model are the intellectual antecedents of sedimentation mechanics. The mediation spectrum (tobacco/ayahuasca/Datura) maps loosely to awareness levels --- contact, translation, transport correspond to Articulate, Recognizable, and the deeper levels where translation into conscious experience becomes increasingly difficult.

- **tensor-schema-spec.md**: This document provides the theoretical foundation for several structures already defined there. It does not modify the schema but explains *why* those structures take the form they do and adds the self-referential edge, Plutchik vocabulary, sedimentation mechanics, and awareness-orthogonality concepts.

---

## Resolved Decisions

These questions were raised in the initial draft and have been resolved through discussion:

1. **Plutchik's cultural specificity → Emotional grammars.** Resolved by introducing the grammar concept. Plutchik provides the structural properties we want (bounded set, opposition, combination, intensity). The specific primaries are one grammar among many. The system treats the grammar as a pluggable parameter from the outset.

2. **Dyadic computation → Mood-vectors.** Resolved by rejecting named dyadic labels as reductive. Co-activated primaries form mood-vectors in emotional-dimensional space. The felt quality is synthesized from the full context (relational direction, values, motivations, scene conditions), not collapsed to a named composite emotion.

3. **Self-edge initialization → Coherence hypothesis for generated entities.** Resolved: authored characters can be intentionally incoherent (that is dramatically interesting). Generated characters derive self-edge from tensor state using coherence heuristics (self-trust ∝ self-efficacy values; self-affection ∝ inverse self-critical values; projection content from dominant motivations). Aberrant behavior in generated entities without narrative resonance is a system failure.

4. **Sedimentation rates → Empirical, with bedrock constraint.** Resolved: rates are discovered through playtesting. Key constraint: organic bedrock-level change through sedimentation alone is rare within a single story graph. Bedrock changes are almost always authorially intended. Organic bedrock evolution requires story-series or significant chronological leaps.

5. **Awareness level shifts → Authored events initially.** Resolved for early implementation: awareness shifts use authored trigger events via the event model. Post-scene processing handles sedimentation-driven awareness changes. The question of threshold-based vs. attention-aggregate models remains open (see below).

6. **East Asian medical model → Subsumed by emotional grammar concept.** A wu xing-derived grammar (grief/lung/metal, fear/kidney/water, anger/liver/wood, joy/heart/fire, worry/spleen/earth) with elemental composition rules is a natural second grammar. The grammar abstraction makes this a content authoring task, not an architectural change.

## Open Questions

1. **Grammar translation in cross-grammar scenes.** When entities operating under different emotional grammars share a scene, how does the frame synthesis step handle the translation? A human character experiencing "fear" encounters a fey entity whose grammar has no "fear" primary but has "dissolution-resistance" --- the Narrator must render this encounter legibly. Is this the World Agent's responsibility? The Narrator's? Both?

2. **Attention-aggregate awareness model.** Should awareness be modeled as a budget rather than independent levels? A finite attentional capacity for self-awareness, where confronting one Defended element to Recognizable might push other Recognizable elements to Preconscious (cognitive load from self-confrontation). This is sophisticated enough to defer past the prototype, but the budget model has interesting implications for scenes with multiple simultaneous revelations.

3. **Sedimentation calibration.** The accumulation model has three input variables (intensity, reinforcement, contradiction-absence) but no calibrated rates. What deposit rates produce natural-feeling character development? What is the ratio between scene-distance accumulation and chronological-gap accumulation? This is an empirical question for Stage 1 playtesting.

4. **Three-timescale processing.** The distinction between scene-distance, story-chronology, and historical-experiential time is established but not yet implemented. How does the Storykeeper's between-scene processing account for chronological gaps? What heuristics govern unsimulated experience during time-skips? How much authorial guidance does this require vs. how much can be inferred from the narrative graph's temporal annotations?

5. **Grammar design methodology.** What makes a good emotional grammar? We have structural requirements (bounded primaries, opposition, intensity, composition) but no methodology for authoring new grammars. Is there a validation procedure --- a set of test scenarios that a grammar must handle to be considered complete?

6. **Self-edge evolution under pressure.** How rapidly should the self-referential edge shift in response to dramatic events? A moment of unexpected competence should raise self-trust, but how much? The self-edge has the same sedimentation mechanics as any relational edge, but the dynamics of self-relationship may differ from inter-entity relationships in ways we haven't yet characterized.
