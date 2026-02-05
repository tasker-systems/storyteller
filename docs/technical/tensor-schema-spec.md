# Tensor Schema Specification

## Purpose

This document defines the formal type system for character tensors, relational edges, and scene contexts — the structured representation that bridges the philosophical case studies and the computational layer.

The tensor case studies (`tensor-case-study-sarah.md`, `tensor-case-study-wolf.md`) establish *what* the system should capture and *why*. This document specifies *how* it is structured for computation. The case studies remain the authoritative source for design intent; this schema is accountable to them.

### The Naming Problem

The case studies use semantically rich names like `fear_that_she_is_not_enough` or `comfort_with_physicality`. These carry enormous meaning for human readers and LLMs — the name *is* the specification. But for a computational layer (particularly a dedicated ML inference model), a hash key is just a string. The semantic weight of the name is invisible.

This schema solves this by making the semantic structure that the names carry implicitly into explicit typed fields, relationships, and categories. The name remains as human-readable metadata; the type system carries the computational meaning.

### The Intertwining Problem

Merleau-Ponty's phenomenological insight that the boundary between subject and world is permeable — that we are both bodies-experiencing and bodies-experienced — shows up here as an architectural challenge. A character's inner state (tensor) shapes their relational presence (web edges), and their relational context (web configurations) shapes their inner state. The schema must model this bidirectional permeability through well-defined ports rather than blurring the boundary into incoherence.

---

## Element Types

Every element in a character tensor is typed. The type determines its structure, its temporal behavior, its role in frame computation, and how it participates in the intertwining with the relational web.

### 1. PersonalityAxis

Bipolar spectra that describe enduring dispositional tendencies. These are the most structurally regular elements — they share a common representation format.

```
PersonalityAxis {
  id: string                     // unique identifier
  label: (string, string)        // pole names, e.g., ("optimism", "pessimism")
  category: AxisCategory         // temperament | moral | cognitive | social
  central_tendency: f32          // [-1.0, 1.0] — default position
  variance: f32                  // [0.0, 1.0] — how much it shifts under pressure
  range: (f32, f32)              // floor and ceiling
  temporal_layer: TemporalLayer  // bedrock | sediment | topsoil | primordial
  contextual_triggers: [Trigger] // what shifts the axis

  // Non-human extension: axis labels may be reinterpreted
  // e.g., warmth/reserve → connection/isolation for the Wolf
  // The structure is identical; the semantic poles differ.
  reinterpretation_note: string? // optional note for non-standard semantics
}

AxisCategory = temperament | moral | cognitive | social
```

**Computational semantics**: The `central_tendency` is where the character sits by default. The `variance` describes the width of likely shifts. The `range` is the hard boundary — the character cannot be pushed beyond this regardless of context. Contextual triggers modulate the position within the range for a specific scene.

**Example (Sarah)**:
```
{
  id: "warmth_reserve",
  label: ("warmth", "reserve"),
  category: social,
  central_tendency: -0.1,   // slight reserve
  variance: 0.5,            // high — she shifts dramatically by context
  range: (-0.7, 0.8),
  temporal_layer: sediment,  // with topsoil surges
  contextual_triggers: [
    { condition: CharacterPresent("family"), shift: +0.6 },
    { condition: CharacterPresent("stranger"), shift: -0.3 },
    { condition: EmotionalDisplay("vulnerability"), shift: +0.4 }
  ]
}
```

**Example (Wolf — reinterpreted)**:
```
{
  id: "connection_isolation",
  label: ("connection", "isolation"),
  category: social,
  central_tendency: -0.7,   // strongly isolated
  variance: 0.4,
  range: (-0.9, 0.1),       // can be moved toward connection; never warm
  temporal_layer: primordial,
  contextual_triggers: [
    { condition: PhysicalContact("sarah"), shift: +0.4 },
    { condition: CapabilityDemonstrated("sarah"), shift: +0.3 }
  ],
  reinterpretation_note: "Not warmth/reserve in the human sense.
    Whether the entity permits connection at all."
}
```

### 2. Capacity

Unipolar intensities describing what the character can do, marked by domain.

```
Capacity {
  id: string
  domain: CapacityDomain         // physical | intellectual | social |
                                 //   creative | supernatural | territorial
  level: f32                     // [0.0, 1.0]
  temporal_layer: TemporalLayer  // typically sediment or bedrock
  limitations: [string]          // typed constraints on the capacity

  // Non-human extension
  capacity_domain_note: string?  // e.g., "supernatural — not measured on human scale"
}

CapacityDomain = physical | intellectual | social | creative | supernatural | territorial
```

**Computational semantics**: Capacities determine what actions are *possible* for the character, feeding into the World Agent's constraint framework and the Reconciler's conflict resolution. They also contribute to the power framework — a character's capabilities in a domain affect the emergent power configuration.

**Example (Sarah)**:
```
{
  id: "supernatural_perception",
  domain: supernatural,
  level: 0.8,
  temporal_layer: bedrock,
  limitations: ["innate, not learned", "not consciously controlled",
                "activated by liminal spaces, especially water"]
}
```

### 3. Value

Directional commitments that filter perception and shape behavior. Values are not bipolar — they are convictions with varying strength that the character holds (consciously or not).

```
Value {
  id: string
  strength: f32                  // [0.0, 1.0]
  temporal_layer: TemporalLayer  // typically bedrock or sediment
  conscious: bool                // does the character know they hold this?

  // How this value filters perception:
  perception_filter: PerceptionFilter?

  // Relationships to other elements:
  supports: [ElementRef]         // motivations or behaviors this value reinforces
  suppresses: [ElementRef]       // shadow wants or impulses this value holds down
  challenged_by: [TypedCondition] // conditions that pressure this value
}

PerceptionFilter {
  domain: string                 // what kind of events this filters
  bias: string                   // how it colors interpretation
  // e.g., domain: "others_actions", bias: "judges by whether they act"
}
```

**Computational semantics**: Values are the character's interpretive lenses. When the ML inference layer computes a frame, values determine how the character will *read* a situation — not just what they feel but what they notice, what they judge, what they dismiss. The `perception_filter` is what makes values computationally active rather than decorative.

**Example (Sarah)**:
```
{
  id: "people_should_do_what_needs_doing",
  strength: 0.9,
  temporal_layer: bedrock,
  conscious: true,   // she could articulate this
  perception_filter: {
    domain: "others_actions",
    bias: "judges others by whether they act when action is needed"
  },
  supports: ["find_tommy", "identity_as_someone_who_acts"],
  suppresses: [],
  challenged_by: [Condition::Exhaustion, Condition::Helplessness]
}
```

### 4. Motivation

Goal-directed states with explicit layering (surface/deep/shadow), urgency, and inter-motivation relationships.

```
Motivation {
  id: string
  layer: MotivationLayer         // surface | deep | shadow
  intensity: f32                 // [0.0, 1.0]
  urgency: f32                   // [0.0, 1.0] — time pressure
  temporal_layer: TemporalLayer

  // Relationships to other motivations:
  mirrors: ElementRef?           // shadow ↔ deep mirror
  supports: [ElementRef]         // motivations that reinforce this one
  contradicts: [ElementRef]      // motivations that create tension
  served_by: [ElementRef]        // surface wants that serve this deep want

  // Activation conditions for shadow wants:
  activation_conditions: [TypedCondition]?  // when does this surface?
}

MotivationLayer = surface | deep | shadow
```

**Computational semantics**: The motivation structure is where the tensor becomes most dramatically productive. The ML inference layer reads the motivation graph to compute *what the character is trying to do* in a scene — and, crucially, what they are trying to do *without knowing it*. Shadow motivations with `activation_conditions` produce the "disproportionate response" moments that make characters feel real.

**Example (Sarah)**:
```
{
  id: "anger_at_tommy_for_leaving",
  layer: shadow,
  intensity: 0.4,
  urgency: 0.0,   // not time-pressured — it's suppressed
  temporal_layer: sediment,
  mirrors: "restore_connection_with_tommy",
  supports: [],
  contradicts: ["find_tommy"],  // she wants to save him AND she's furious at him
  activation_conditions: [
    Condition::Revelation("tommy_secret_life"),
    Condition::EncounteringSomeoneWhoConceals,
    Condition::AccumulatedStress(threshold: 0.7)
  ]
}
```

### 5. EmotionalState

Current conditions with decay models. These are always topsoil — volatile, responsive, the character's weather rather than their climate. However, sustained emotional states can transition to sediment.

```
EmotionalState {
  id: string
  valence: Valence               // positive | negative | mixed
  intensity: f32                 // [0.0, 1.0]
  decay_model: DecayModel
  temporal_layer: topsoil        // always starts as topsoil

  triggers: [TypedCondition]     // what sustains or reactivates
  sediment_threshold: f32?       // intensity * duration at which this
                                 // transitions from topsoil to sediment
}

Valence = positive | negative | mixed

DecayModel {
  rate: DecayRate                // fast | slow | stable | accumulating
  half_life_scenes: u32?         // for fast/slow: how many scenes to halve
  sustained_by: [TypedCondition] // conditions that prevent decay
}

DecayRate = fast | slow | stable | accumulating
```

**Computational semantics**: Emotional states are the most volatile tensor elements. They decay between scenes (unless sustained), accumulate under prolonged conditions, and occasionally cross the sediment threshold to become sustained patterns. The decay model is what the Storykeeper uses to update tensors between scenes.

**Example (Sarah)**:
```
{
  id: "grief",
  valence: negative,
  intensity: 0.8,
  decay_model: {
    rate: slow,
    half_life_scenes: 8,
    sustained_by: [Condition::TommyStillLost, Condition::SeeingSickbed]
  },
  temporal_layer: topsoil,
  triggers: [Condition::TommyMentioned, Condition::SeeingSickbed],
  sediment_threshold: 0.6   // if sustained above 0.6 for many scenes,
                             // becomes sedimentary grief-pattern
}
```

### 6. EchoPattern

Dormant resonance structures that activate when current experience maps onto historical patterns. Echoes live in sediment or bedrock but activate *into* topsoil, producing sudden emotional surges and — critically — power configuration shifts.

```
EchoPattern {
  id: string
  historical_pattern: string      // what happened — human-readable description
  dormant_layer: TemporalLayer    // sediment | bedrock | primordial

  trigger_conditions: [TypedCondition]  // what activates this echo
  trigger_threshold: f32          // [0.0, 1.0] — how many conditions must match

  activated_state: EmotionalState // what surfaces when the echo fires
  resonance_dimensions: [ResonanceDimension]

  // Power framework integration:
  configuration_shift: ConfigShift?  // how this echo changes relational dynamics

  // For non-human entities:
  echo_type: EchoType            // personal | archetypal
}

ResonanceDimension = sensory | emotional | relational | thematic | spatial

EchoType = personal | archetypal
// personal: from individual experience (Sarah's stream memories)
// archetypal: from collective/primordial patterns (Wolf's hierarchy disruption)

ConfigShift {
  affected_relationships: [RelationshipRef]
  shift_description: string       // how the configuration changes
  // e.g., "supernatural_perception activates → power dynamic with Wolf inverts"
}
```

**Computational semantics**: The echo mechanism is where the temporal layers interact most dramatically. A bedrock pattern (formed in childhood, or primordial) can surge through sediment and topsoil when current conditions rhyme with historical ones. The ML inference layer must detect echo conditions as part of its frame computation pipeline: check trigger conditions → fire echo → recompute emotional state → recompute relational configuration → produce updated frame.

---

## Temporal Layer Semantics

```
TemporalLayer = topsoil | sediment | bedrock | primordial

topsoil {
  change_rate: high               // shifts within and between scenes
  decay: yes                      // emotional states fade
  half_life: scenes               // measured in scene count
  transition_to_sediment: yes     // sustained states can crystallize
}

sediment {
  change_rate: low                // shifts over many scenes/sessions
  decay: very_slow                // patterns erode but slowly
  half_life: sessions_to_arcs    // measured in story arcs
  resistance_to_change: high      // single events don't move sediment
  transition_to_bedrock: rare     // only under extraordinary pressure
}

bedrock {
  change_rate: minimal            // shifts only under extraordinary conditions
  decay: none                     // these are the ruts experience flows through
  resistance_to_change: extreme
  crack_conditions: [TypedCondition]  // what could fracture this
  // e.g., for Sarah's "family_bonds_are_the_strongest_thing":
  //   crack_condition: Revelation("tommy_chose_beth_over_family")
}

primordial {
  // For entities older than individual life
  change_rate: geological         // effectively zero under normal conditions
  decay: none
  constitutive: true              // removing this element unmakes the entity
  // e.g., the Wolf's "dream_of_wolves" — he cannot be other than this
  //   without ceasing to exist
}
```

---

## Inter-Element Relationships

The tensor is not a flat collection. Elements relate to each other, and these relationships are computationally meaningful — they determine how activations cascade, how contradictions produce dramatic tension, and how the ML inference layer constructs frames.

```
ElementRelationship =
  | Mirrors(a: ElementRef, b: ElementRef)
    // Shadow ↔ deep want mirroring
    // e.g., fear_that_she_is_not_enough mirrors prove_herself_capable

  | Supports(source: ElementRef, target: ElementRef)
    // One element reinforces another
    // e.g., identity_as_someone_who_acts supports find_tommy

  | Contradicts(a: ElementRef, b: ElementRef)
    // Tension between elements — dramatically productive
    // e.g., protect_sarah contradicts ensure_sarah_fails (Wolf)

  | Activates(trigger: ElementRef, target: ElementRef)
    // One element's presence/activation produces another
    // e.g., echo(uncanny_at_stream) activates supernatural_perception

  | Suppresses(suppressor: ElementRef, suppressed: ElementRef)
    // One element actively holds another down
    // e.g., loyalty_to_family suppresses anger_at_tommy

  | Filters(filter: ElementRef, domain: string)
    // A value or belief that colors perception of a domain
    // e.g., adults_often_cannot_see_what_matters filters perception of authority

  | Enables(capacity: ElementRef, action_domain: string)
    // A capacity that makes certain actions possible
    // e.g., supernatural_perception enables seeing_hidden_paths

  | GroundedIn(element: ElementRef, foundation: ElementRef)
    // An element that rests on a deeper-layer foundation
    // e.g., self_reliance is grounded_in identity_as_someone_who_acts
```

**Computational semantics**: These relationships form a directed graph within the tensor. The ML inference layer traverses this graph when computing frames: if `find_tommy` is activated (scene-relevant), and `find_tommy` is supported by `identity_as_someone_who_acts` (bedrock) and contradicted by `anger_at_tommy_for_leaving` (shadow), then the frame should convey both the determination and the buried tension.

---

## The Intertwining: Ports Between Inner and Outer

The character tensor (inner representation) and the relational web (outer representation) are not independent data structures. They are connected through defined ports — interfaces where inner state becomes outer presence and outer conditions become inner experience.

### Outward Port: Tensor → Relational Presence

How the character's inner state shapes their relational behavior:

```
OutwardPort {
  // Values shape how trust is extended:
  trust_formation: fn(Values, History) -> TrustProfile
  // e.g., Sarah's "adults_often_cannot_see_what_matters" →
  //   lower default trust[competence] for adult authority figures

  // Motivations shape relational approach:
  relational_approach: fn(Motivations, RelationalConfig) -> ApproachStyle
  // e.g., surface_want "find_tommy" + shadow "anger_at_tommy" →
  //   approach combines urgency with suppressed ambivalence

  // Capacities contribute to power configurations:
  power_contribution: fn(Capacities, SceneContext) -> CapabilityProfile
  // e.g., supernatural_perception in a liminal scene →
  //   capability that exceeds the Wolf's, shifting configuration

  // Emotional states color relational behavior:
  emotional_coloring: fn(EmotionalStates) -> BehavioralTone
  // e.g., grief(0.8) + determination(0.9) → fierce, compressed affect

  // Beliefs filter interpretation of others' actions:
  interpretation_filter: fn(Values, Beliefs, OtherAction) -> Interpretation
  // e.g., "people_should_do_what_needs_doing" + Adam's evasiveness →
  //   suspicion (he is not acting straightforwardly)
}
```

### Inward Port: Relational/Scene Context → Tensor Updates

How relational conditions and scene events affect inner state:

```
InwardPort {
  // Relational configurations produce emotional responses:
  emotional_response: fn(RelationalConfig, SceneContext) -> [EmotionalState]
  // e.g., wary_dependence configuration with Adam →
  //   produces wariness(topsoil) + determination(sustained)

  // Information revelations update beliefs and projections:
  revelation_update: fn(Information, Beliefs, Projections) -> TensorDelta
  // e.g., learning about Beth and the child →
  //   crack "family_bonds_are_the_strongest_thing",
  //   shatter projection of Tommy,
  //   activate echo "tommy_withdrawing" with full force,
  //   surface shadow_want "anger_at_tommy"

  // Power configuration shifts challenge identity:
  identity_pressure: fn(ConfigShift, BedrockElements) -> StressTensor
  // e.g., Sarah leading the Wolf at the Other Bank →
  //   reinforces "identity_as_someone_who_acts",
  //   strengthens "the_world_has_hidden_depths",
  //   shifts dominant_deferential axis

  // Scene conditions activate contextual triggers:
  trigger_activation: fn(SceneProperties, ContextualTriggers) -> [AxisShift]
  // e.g., scene at water + characters_present(wolf) →
  //   shift intuitive_analytical toward intuitive,
  //   activate echo "uncanny_at_the_stream"

  // Accumulated experience transitions temporal layers:
  layer_transition: fn(Duration, Intensity, Threshold) -> LayerChange?
  // e.g., grief sustained above 0.6 for 8+ scenes →
  //   transitions from topsoil to sediment
}
```

### The Port Contract

The ports define a **contract** between the tensor and the relational web:

1. **The tensor never directly reads the relational web.** It receives processed signals through the inward port — emotional responses to configurations, information revelations, identity pressures, trigger activations. The tensor doesn't know the topology; it knows the *felt effects* of the topology.

2. **The relational web never directly reads the tensor.** It receives processed signals through the outward port — trust profiles, approach styles, capability profiles, behavioral tones. The web doesn't know the bedrock structure; it knows the *relational surface* the bedrock produces.

3. **The ML inference layer reads both.** The inference layer has access to both the tensor (full persistence layer) and the relational web (substrate + topology) and the scene context. It is the only component that holds the complete picture. It produces the psychological frame by integrating all three.

4. **The Storykeeper manages both through the ports.** When updating a character between scenes, the Storykeeper uses the inward port to translate relational events into tensor changes, and the outward port to translate tensor state into relational updates (e.g., updating projection accuracy after a revelation).

---

## Scene Context as Typed Property Set

For the trigger system and the ML inference layer to function, scenes need typed properties that can be matched against tensor conditions.

```
SceneContext {
  // Physical setting
  setting_features: [SettingFeature]
  // e.g., [Water, Forest, Night, Cold, LiminalSpace]

  // Who is present
  characters_present: [CharacterId]

  // Relational state (computed from web for characters present)
  active_configurations: [RelationalConfiguration]

  // Emotional register of the scene
  emotional_register: [EmotionalRegister]
  // e.g., [Tense, Grieving, Wondrous]

  // What is at stake
  stakes: [Stake]
  // e.g., [PhysicalSafety, Relationship("tommy"), Information("kate_nature")]

  // Thematic resonance
  thematic_resonance: [Theme]
  // e.g., [Loss, Homecoming, Boundary, Discovery]

  // Temporal context
  temporal_context: TemporalContext
  // e.g., { time_of_day: night, season: autumn, story_phase: early_quest }

  // Information available for revelation
  available_revelations: [Revelation]
  // e.g., [Revelation("tommy_has_a_child", gate: "encounter_with_beth")]
}
```

### Trigger Matching

Contextual triggers on tensor elements are matched against scene properties:

```
TypedCondition =
  | SettingFeature(feature: SettingFeature)
  | CharacterPresent(character: CharacterId)
  | ConfigurationType(config: ConfigType)
  | EmotionalRegister(register: EmotionalRegister)
  | ThematicResonance(theme: Theme)
  | StakeInvolved(stake: Stake)
  | PhysicalContact(character: CharacterId)
  | CapabilityDemonstrated(character: CharacterId)
  | EmotionalDisplay(display: string)
  | Revelation(content: string)
  | AccumulatedStress(threshold: f32)
  | Compound(conditions: [TypedCondition], mode: All | Any)

Trigger {
  condition: TypedCondition
  shift_axis: ElementRef?        // which axis to shift (for personality axes)
  shift_direction: f32           // positive or negative
  magnitude: f32                 // how much to shift
  activates: ElementRef?         // which element to activate (for echoes, etc.)
}
```

---

## Frame Computation Pipeline

The ML inference layer computes the psychological frame through a defined pipeline:

```
Frame = compute_frame(
  tensor: FullTensor,             // persistence layer
  relational_web: RelationalWeb,  // substrate + topology
  scene_context: SceneContext,    // typed scene properties
) -> PsychologicalFrame

Pipeline:
  1. TRIGGER MATCHING
     Match scene_context against tensor.contextual_triggers
     → produces [AxisShift] for personality axes
     → produces [Activation] for echoes

  2. ECHO DETECTION
     Check activated echoes against trigger thresholds
     → fire qualifying echoes
     → add echo emotional states to topsoil
     → note configuration shifts for step 4

  3. TENSOR ACTIVATION
     Select scene-relevant elements from full tensor
     → apply axis shifts from step 1
     → foreground motivations relevant to scene stakes
     → activate values relevant to scene themes
     → include relevant capacities for scene actions
     → produce activated_subset (~800-1200 tokens as data)

  4. CONFIGURATION COMPUTATION
     Read relational web substrate for characters present
     Compute network topology features (position, bridging, clustering)
     Apply echo-driven configuration shifts from step 2
     → produce relational_configurations for this scene

  5. FRAME SYNTHESIS
     Integrate activated_subset + relational_configurations
     Apply outward port (how inner state shapes relational presence)
     Apply inward port (how configurations shape emotional state)
     Compress to frame register appropriate to entity type:
       - human characters: introspective, emotional language
       - non-human entities: somatic, spatial, instinctual language
     → produce PsychologicalFrame (~200-400 tokens as natural language)

PsychologicalFrame {
  content: string                 // natural language frame for Character Agent
  register: FrameRegister         // human | non_human | other
  activated_relationships: [RelationshipRef]  // which relationships are in play
  active_echoes: [EchoRef]        // which echoes fired
  dominant_configuration: ConfigType  // the primary relational dynamic
  token_count: u32                // budget tracking
}
```

---

## Validation Against Case Studies

The schema is validated by demonstrating that it can represent the full tensor for both Sarah and the Wolf without losing what makes them work.

### Sarah Validation Checklist

| Case Study Element | Schema Type | Captured? |
|---|---|---|
| `warmth_reserve` axis with triggers | PersonalityAxis | Yes — bipolar, contextual triggers typed |
| `loyalty_to_family: 0.95` | Value with supports/suppresses | Yes — supports find_tommy, suppresses anger_at_tommy |
| `fear_that_she_is_not_enough` | Motivation (shadow) with mirrors | Yes — mirrors prove_herself_capable, activation_conditions typed |
| `supernatural_perception: 0.8` | Capacity (supernatural domain) | Yes — enables seeing_hidden_paths |
| `grief: 0.8` with decay | EmotionalState with DecayModel | Yes — slow decay, sustained_by conditions |
| `echo: tommy_withdrawing` | EchoPattern with config_shift | Yes — trigger conditions typed, configuration shift on fire |
| The Other Bank activation | Frame via pipeline | Yes — trigger matching → echo → activation → configuration → frame |

### Wolf Validation Checklist

| Case Study Element | Schema Type | Captured? |
|---|---|---|
| `connection_isolation` reinterpreted axis | PersonalityAxis with reinterpretation_note | Yes |
| `duty_is_identity` primordial value | Value with temporal_layer: primordial | Yes |
| `ensure_sarah_fails` contradicting `protect_sarah` | Motivation.contradicts relationship | Yes |
| `curiosity_about_sarah` shadow want | Motivation (shadow, forming) | Yes |
| World-rending power (supernatural) | Capacity with domain: supernatural | Yes |
| Archetypal echo | EchoPattern with echo_type: archetypal | Yes |
| Nascent connection (emergent) | Not a stored element — emergent from configuration | Correct — not represented as element |
| Frame register: somatic/spatial | PsychologicalFrame.register: non_human | Yes |

---

## Open Questions

1. **Vocabulary enumeration**: The typed conditions (SettingFeature, EmotionalRegister, Theme, etc.) need concrete vocabulary lists. How open or closed should these vocabularies be? Closed vocabularies enable reliable trigger matching but may be too rigid; open vocabularies are flexible but make matching harder.

2. **Inter-element relationship inference**: Should the story designer specify all element relationships explicitly, or should some be inferred? A shadow motivation that mirrors a deep want could potentially be detected by the system rather than authored.

3. **Entity type taxonomy**: The schema handles human and non-human (Wolf) characters. What about Presences (the Shadowed Wood), Conditions (the economy), or Props elevated to narrative significance? Do they use a subset of the same schema or a different one?

4. **Training data for the ML layer**: The frame computation pipeline is specified but the ML model needs training data. The case studies provide 2-4 examples each. How do we generate sufficient training data? Options include: synthetic generation from authored tensors, manual frame authoring for diverse scenarios, or bootstrapping from LLM-generated frames that are then validated.

5. **Trigger compositionality**: Complex trigger conditions (Compound with All/Any) could become arbitrarily complex. What is the practical limit? Do we need a trigger language or is the typed condition set sufficient?

6. **Schema evolution**: As more case studies are developed (Bramblehoof, Vretil characters), the schema will face new pressures. The type system should be extensible without breaking existing representations. How do we manage schema versioning?

7. **Performance budgets**: The frame computation pipeline has 5 steps. What are the latency and token budgets for each step? The pipeline must complete within acceptable time for interactive play.
