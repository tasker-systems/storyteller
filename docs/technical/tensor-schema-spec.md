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

**Naming conventions**: Type names are chosen to avoid collision with common language keywords and to be self-documenting in code. `PersonalValue` rather than `Value` (a keyword in many languages and ambiguous); `CharacterCapacity` rather than `Capacity` (similarly overloaded). The case studies use shorter names for readability; implementation uses the qualified names.

### 1. PersonalityAxis

Bipolar spectra that describe enduring dispositional tendencies. These are the most structurally regular elements — they share a common representation format.

```
PersonalityAxis {
  id: AxisId                     // unique identifier, the computational lookup key
  labels: (String, String)       // pole names for humans/LLMs, e.g., ("optimism", "pessimism")
  category: AxisCategory         // temperament | moral | cognitive | social
  tags: HashSet<AxisTag>         // semantic tags for compositional trigger matching
  central_tendency: f32          // [-1.0, 1.0] — default position
  variance: f32                  // [0.0, 1.0] — how much it shifts under pressure
  range: (f32, f32)              // floor and ceiling
  temporal_layer: TemporalLayer  // bedrock | sediment | topsoil | primordial
  contextual_triggers: [Trigger] // what shifts the axis

  // Non-human extension: axis labels may be reinterpreted
  // e.g., warmth/reserve → connection/isolation for the Wolf
  // The structure is identical; the semantic poles differ.
  reinterpretation_note: String? // optional note for non-standard semantics
}

AxisCategory = temperament | moral | cognitive | social

// Tags enable compositional trigger matching — triggers can target by tag
// rather than by specific axis id, allowing more general rules like
// "any situation involving emotional vulnerability shifts social axes toward warmth"
AxisTag = Social | Emotional | Cognitive | VulnerabilityResponsive | ThreatResponsive
        | InterpersonalTrust | SelfRegulation | ...  // extensible vocabulary
```

**On labels vs. id**: The `id` is the computational lookup key — how the system finds and references this axis. The `labels` tuple is metadata for human readers, LLM prompts, and frame synthesis. These serve different purposes: `id: "warmth_reserve"` is for code; `labels: ("warmth", "reserve")` is for generating natural language like "Sarah's reserve melts into warmth."

**On tags**: Tags enable triggers to work semantically rather than by explicit axis reference. A trigger authored as `{ condition: EmotionalVulnerability, affects: Tag(Social), shift: +0.3 }` would shift *all* social axes toward their positive pole, without the author needing to enumerate them. This supports compositional matching and makes tensors more maintainable as they grow.

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

### 2. CharacterCapacity

Unipolar intensities describing what the character can do, marked by domain.

```
CharacterCapacity {
  id: CapacityId
  domain: CapacityDomain         // physical | intellectual | social |
                                 //   creative | supernatural | territorial
  level: f32                     // [0.0, 1.0]
  temporal_layer: TemporalLayer  // typically sediment or bedrock

  // Typed limitations for computational use (World Agent, Reconciler):
  limitations: [CapacityLimitation]

  // Textual context for frame synthesis (LLM consumption):
  limitation_notes: [String]     // human-readable elaboration

  // Non-human extension
  capacity_domain_note: String?  // e.g., "supernatural — not measured on human scale"
}

CapacityDomain = physical | intellectual | social | creative | supernatural | territorial

// Typed limitations enable computational reasoning about what's possible
CapacityLimitation =
  | Innate                                    // cannot be learned or taught
  | NotConsciouslyControlled                  // triggers but can't be directed
  | ContextDependent(conditions: [SettingFeature])  // only works in certain conditions
  | Exhaustible { recovery_rate: f32 }        // depletes with use, recovers over time
  | RequiresCondition(condition: TypedCondition)    // must be met to use
  | Opposed(by: CapacityId)                   // another capacity can counter this
  | ...  // extensible
```

**On typed vs. textual limitations**: Limitations serve two audiences. The World Agent needs typed limitations to validate actions ("Can Sarah use supernatural perception here? Check `ContextDependent([Water, LiminalSpace])`"). The frame synthesis step needs textual notes to produce natural language ("Your sight has always been different — it comes unbidden, especially near water"). Both are necessary; conflating them loses either computational precision or narrative richness.

**Computational semantics**: Capacities determine what actions are *possible* for the character, feeding into the World Agent's constraint framework and the Reconciler's conflict resolution. They also contribute to the power framework — a character's capabilities in a domain affect the emergent power configuration.

**Example (Sarah)**:
```
{
  id: "supernatural_perception",
  domain: supernatural,
  level: 0.8,
  temporal_layer: bedrock,
  limitations: [
    Innate,
    NotConsciouslyControlled,
    ContextDependent([Water, LiminalSpace])
  ],
  limitation_notes: [
    "innate, not learned — she has always seen things others cannot",
    "not consciously controlled — it comes when it comes",
    "activated by liminal spaces, especially water"
  ]
}
```

### 3. PersonalValue

Directional commitments that filter perception and shape behavior. Values are not bipolar — they are convictions with varying strength that the character holds at varying levels of awareness.

```
PersonalValue {
  id: ValueId
  strength: f32                  // [0.0, 1.0]
  temporal_layer: TemporalLayer  // typically bedrock or sediment
  awareness: AwarenessLevel      // gradient of conscious accessibility

  // How this value filters perception:
  perception_filter: PerceptionFilter?

  // Relationships to other elements:
  supports: [ElementRef]         // motivations or behaviors this value reinforces
  suppresses: [ElementRef]       // shadow wants or impulses this value holds down
  challenged_by: [TypedCondition] // conditions that pressure this value
}

// Awareness is a gradient, not a binary — this affects how values surface
// in frames and how they interact with revelations and echoes
AwarenessLevel =
  | Articulate      // can state this value explicitly ("I believe people should act")
  | Recognizable    // would recognize if pointed out ("I guess I do judge people who don't act")
  | Preconscious    // could surface with right prompting, not yet recognized
  | Defended        // actively hidden from self, would resist recognition
  | Structural      // below the threshold of possible awareness (bedrock/primordial)

PerceptionFilter {
  domain: String                 // what kind of events this filters
  bias: String                   // how it colors interpretation
  // e.g., domain: "others_actions", bias: "judges by whether they act"
}
```

**On awareness as gradient**: A boolean `conscious` field loses important distinctions. A `Defended` value that surfaces through an echo is dramatically different from a `Preconscious` value becoming `Recognizable` through conversation. The awareness level affects:
- How the value appears in psychological frames (explicit vs. undertone vs. absent)
- How the character responds to having the value pointed out (agreement, recognition, resistance, incomprehension)
- What kinds of events can shift the awareness level (echoes can surface `Defended` values; gentle conversation might surface `Preconscious` ones)

**Computational semantics**: Values are the character's interpretive lenses. When the ML inference layer computes a frame, values determine how the character will *read* a situation — not just what they feel but what they notice, what they judge, what they dismiss. The `perception_filter` is what makes values computationally active rather than decorative.

**Example (Sarah)**:
```
{
  id: "people_should_do_what_needs_doing",
  strength: 0.9,
  temporal_layer: bedrock,
  awareness: Articulate,   // she could state this explicitly
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
  id: MotivationId
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
  activation_conditions: [ActivationCondition]?  // when does this surface?
}

MotivationLayer = surface | deep | shadow
```

#### Factual vs. Interpretive Conditions

Activation conditions fall into two categories that are processed differently:

```
ActivationCondition =
  // === FACTUAL CONDITIONS ===
  // Checked by event system / state queries. These are objective facts
  // about the world state that can be determined with certainty.
  // The World Agent can validate these; they don't require judgment.

  | Factual(TriggerCondition)
    // Examples:
    //   Factual(CharacterPresent("adam"))        — Adam is in the scene
    //   Factual(Revelation("tommy_has_child"))   — this info was revealed
    //   Factual(AccumulatedStress(0.7))          — stress crossed threshold
    //   Factual(SettingFeature(Water))           — water is present

  // === INTERPRETIVE CONDITIONS ===
  // Evaluated by ML inference layer during frame computation. These require
  // subjective judgment about meaning, behavior, or context. They cannot
  // be reduced to world-state queries.

  | Interpretive {
      description: String,              // human-readable for authoring
      semantic_pattern: SemanticPattern, // structured hint for ML inference
      confidence_threshold: f32,        // [0.0, 1.0] — how certain must inference be?
  }

SemanticPattern =
  | BehaviorReading { behavior_type: String }
    // "concealment", "evasion", "vulnerability", "aggression"
    // ML layer observes character behavior and infers pattern

  | ThematicResonance { theme: String }
    // "abandonment", "betrayal", "sacrifice", "homecoming"
    // ML layer recognizes when scene/situation rhymes with theme

  | RelationalDynamic { dynamic: String }
    // "power_imbalance", "growing_intimacy", "trust_erosion", "dependence"
    // ML layer reads the relational configuration and infers dynamic

  | EmotionalUndercurrent { emotion: String }
    // "suppressed_anger", "hidden_grief", "denied_fear", "unacknowledged_love"
    // ML layer infers emotional subtext beneath surface interaction

  | SituationalPattern { pattern: String }
    // "being_left_behind", "forced_to_wait", "excluded_from_knowledge"
    // ML layer recognizes structural similarity to historical patterns
```

**Why this distinction matters**:

1. **Factual conditions** fire immediately when events occur — they subscribe to the event bus and react in real-time. The World Agent can validate them. They're objective.

2. **Interpretive conditions** are evaluated during frame computation — the ML layer reads the full context (scene, tensor, relational web) and *judges* whether the pattern is present. They're subjective, require inference, and may fire with varying confidence.

3. **Shadow want activation is interpretive-heavy** — a shadow want surfaces not because a specific event occurred, but because the *meaning* of the situation resonates with something buried. This is appropriate: shadows emerge through felt sense, not logical trigger.

4. **The ledger records both** — factual events as facts, interpretive judgments as metadata ("ML layer inferred concealment_behavior from Adam's responses with confidence 0.73"). This enables debugging and understanding why a shadow surfaced.

5. **Authoring flexibility** — story designers can author interpretive conditions in natural language (`description: "encountering someone who seems to be hiding something"`), with the `semantic_pattern` providing structured guidance to the ML layer. This allows depth for both carefully authored major characters and LLM-generated incidental characters (a shopkeeper, a traveling merchant) who still need psychological texture.

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
    // Factual: specific revelation fires this immediately
    Factual(Revelation("tommy_secret_life")),

    // Factual: accumulated stress crosses threshold
    Factual(AccumulatedStress(threshold: 0.7)),

    // Interpretive: encountering concealment behavior (requires ML judgment)
    Interpretive {
      description: "encountering someone who seems to be hiding something",
      semantic_pattern: BehaviorReading { behavior_type: "concealment" },
      confidence_threshold: 0.6
    },

    // Interpretive: situation that rhymes with being left behind
    Interpretive {
      description: "being in a situation where someone has left or is leaving",
      semantic_pattern: SituationalPattern { pattern: "being_left_behind" },
      confidence_threshold: 0.7
    }
  ]
}
```

### 5. EmotionalState

Current conditions with decay models. These are always topsoil — volatile, responsive, the character's weather rather than their climate. However, sustained emotional states can transition to sediment.

```
EmotionalState {
  id: EmotionalStateId
  valence: Valence               // positive | negative | mixed
  intensity: f32                 // [0.0, 1.0]
  decay_model: DecayModel
  temporal_layer: topsoil        // always starts as topsoil

  triggers: [ActivationCondition]     // what sustains or reactivates (factual or interpretive)
  sediment_threshold: f32?            // intensity * duration at which this
                                      // transitions from topsoil to sediment
}

Valence = positive | negative | mixed

DecayModel {
  rate: DecayRate                     // fast | slow | stable | accumulating
  half_life_scenes: u32?              // for fast/slow: how many scenes to halve
  sustained_by: [ActivationCondition] // conditions that prevent decay
}

DecayRate = fast | slow | stable | accumulating
```

**On factual vs. interpretive conditions for emotions**: Emotional states can be sustained or triggered by both types:
- **Factual**: `TommyStillLost` is a world-state query — is Tommy still missing? The World Agent knows this.
- **Factual**: `SettingFeature(Sickbed)` — the sickbed is physically present in this scene.
- **Interpretive**: The *emotional weight* of seeing the sickbed — not just its presence but its meaning to Sarah — requires ML judgment about resonance.

This matters because grief might be sustained by the factual condition (Tommy is still lost) but *intensified* by the interpretive condition (the scene feels like loss, even if Tommy isn't mentioned).

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
    sustained_by: [
      // Factual: world-state query — is Tommy still missing?
      Factual(WorldState { query: "tommy_status", equals: "lost" }),

      // Factual: the sickbed is physically in this scene
      Factual(SettingFeature(Sickbed)),

      // Interpretive: scene feels like loss/absence even without explicit trigger
      Interpretive {
        description: "being in a place or situation that feels like absence",
        semantic_pattern: ThematicResonance { theme: "loss" },
        confidence_threshold: 0.5
      }
    ]
  },
  temporal_layer: topsoil,
  triggers: [
    // Factual: Tommy is mentioned
    Factual(EventOccurred(TopicMentioned { topic: "tommy" })),

    // Factual: seeing the sickbed
    Factual(SettingFeature(Sickbed)),

    // Interpretive: encountering reminders of what's lost
    Interpretive {
      description: "encountering something that reminds her of Tommy or home",
      semantic_pattern: SituationalPattern { pattern: "reminder_of_loss" },
      confidence_threshold: 0.6
    }
  ],
  sediment_threshold: 0.6   // if sustained above 0.6 for many scenes,
                             // becomes sedimentary grief-pattern
}
```

### 6. EchoPattern

Dormant resonance structures that activate when current experience maps onto historical patterns. Echoes live in sediment or bedrock but activate *into* topsoil, producing sudden emotional surges and — critically — power configuration shifts.

```
EchoPattern {
  id: EchoId
  historical_pattern: String      // what happened — human-readable description
  dormant_layer: TemporalLayer    // sediment | bedrock | primordial

  trigger_conditions: [ActivationCondition]  // what activates this echo
  trigger_threshold: f32          // [0.0, 1.0] — proportion of conditions that must match

  activated_state: EmotionalState // what surfaces when the echo fires
  resonance_dimensions: [ResonanceDimension]

  // Power framework integration:
  configuration_shift: ConfigShift?  // how this echo changes relational dynamics

  // For non-human entities:
  echo_type: EchoType            // personal | archetypal
}
```

**On echo triggers**: Echoes are paradigmatically interpretive — they fire when current experience *rhymes with* historical patterns, which is a judgment about meaning, not a fact about the world. However, echoes can also have factual triggers:
- **Factual**: `SettingFeature(Water)` — water is present, and water is where Sarah's uncanny perception activates
- **Interpretive**: The situation *feels like* abandonment — requires ML judgment about thematic resonance

The `trigger_threshold` determines how many conditions must match. For interpretive conditions, the confidence scores are factored in: a condition with 0.5 confidence contributes 0.5 toward the threshold, not 1.0.

ResonanceDimension = sensory | emotional | relational | thematic | spatial

```
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

### Event-Driven Architecture

The trigger system manages four distinct concepts:

1. **Narrative Events** — things that happen in the story (objective facts, recorded in a ledger)
2. **Interpretive Judgments** — ML inferences about meaning (subjective, confidence-weighted, also recorded)
3. **Trigger Conditions** — patterns that match against events or judgments (predicates, authored on tensors)
4. **Mandated Shifts** — narrative beats that *must* produce specific changes (guarantees, authored on narrative graph)

```
// === NARRATIVE EVENTS (FACTUAL) ===
// Events are published to a bus when things happen in the story.
// These are objective facts that can be determined with certainty.
// The World Agent can validate these.

NarrativeEvent =
  | SceneEntered { scene_id: SceneId, characters: Vec<CharacterId> }
  | SceneExited { scene_id: SceneId, character: CharacterId }
  | ActionResolved { actor: CharacterId, action: Action, outcome: Outcome }
  | InformationRevealed { recipient: CharacterId, content: RevelationId }
  | RelationshipShifted { from: CharacterId, to: CharacterId, dimension: Dimension, delta: f32 }
  | EmotionExpressed { character: CharacterId, emotion: EmotionType, intensity: f32 }
  | CapabilityDemonstrated { character: CharacterId, capacity: CapacityId }
  | PhysicalContact { characters: (CharacterId, CharacterId) }
  | ThresholdCrossed { character: CharacterId, threshold: ThresholdId }
  | ... // extensible

// === INTERPRETIVE JUDGMENTS ===
// Produced by the ML inference layer during frame computation.
// These are subjective inferences about meaning, behavior, or context.
// They require judgment and have varying confidence levels.

InterpretiveJudgment {
  id: JudgmentId
  timestamp: Timestamp
  scene_context: SceneId
  character_evaluated: CharacterId      // whose behavior/situation was interpreted
  evaluating_for: CharacterId           // whose tensor conditions are being checked
  pattern_matched: SemanticPattern      // what pattern was inferred
  confidence: f32                       // [0.0, 1.0] — how certain is the inference?
  evidence_summary: String              // human-readable explanation of why
  triggered_conditions: Vec<ConditionRef>  // which interpretive conditions this satisfied
}

// Examples of interpretive judgments:
//   "Adam's responses suggest concealment (confidence: 0.73)"
//   "Scene resonates with abandonment theme (confidence: 0.81)"
//   "Relational dynamic shows growing trust erosion (confidence: 0.65)"

// === THE EVENT LEDGER ===
// Maintains history of both factual events and interpretive judgments.
// Both are queryable, but they have different epistemic status.

EventLedger {
  // Factual events — objective, World Agent validated
  events: Vec<(Timestamp, NarrativeEvent)>

  // Interpretive judgments — subjective, ML inference produced
  judgments: Vec<InterpretiveJudgment>

  // Supports queries like:
  //   "what revelations has Sarah received?" (factual)
  //   "when did Sarah last interact with Adam?" (factual)
  //   "has the ML layer ever inferred concealment from Adam?" (interpretive)
  //   "what patterns have been detected in Sarah's situation?" (interpretive)
}

// === TWO PROCESSING PATHS ===
//
// FACTUAL PATH (real-time, event-driven):
//   NarrativeEvent published → Event bus → Factual subscriptions check →
//   Matching subscriptions fire → Effects applied → Ledger updated
//
// INTERPRETIVE PATH (during frame computation):
//   Frame computation begins → ML layer reads context →
//   Interpretive conditions evaluated → Judgments produced →
//   Matching conditions fire → Effects applied → Ledger updated
//
// Both paths can trigger the same effects (axis shifts, echo activation, etc.)
// but they operate on different timescales and with different certainty.

// === TRIGGER SUBSCRIPTIONS ===
// Tensor elements subscribe to patterns. Factual subscriptions react
// immediately to events; interpretive subscriptions are evaluated
// during frame computation.

TriggerSubscription {
  subscriber: ElementRef           // which tensor element owns this subscription
  condition: ActivationCondition   // Factual(...) or Interpretive {...}
  on_match: TriggerEffect          // what happens when matched
  persistence: SubscriptionPersistence  // one-shot vs. ongoing
}

SubscriptionPersistence = OneShot | Ongoing | UntilCondition(ActivationCondition)

TriggerEffect =
  | ShiftAxis { axis: AxisRef, direction: f32, magnitude: f32 }
  | ActivateElement { element: ElementRef }
  | SurfaceEmotion { state: EmotionalState }
  | FireEcho { echo: EchoRef }
  | TransitionAwareness { value: ValueRef, to: AwarenessLevel }
  | SurfaceShadow { motivation: MotivationRef }  // bring shadow want to awareness
  | ... // extensible

// === MANDATED SHIFTS ===
// Authored on the narrative graph, not the tensor. These are guarantees
// that the system must honor when conditions are met. Mandates can use
// either factual or interpretive conditions.

NarrativeMandatedShift {
  id: MandateId
  trigger_scene: SceneId?              // fires when this scene is reached
  trigger_condition: ActivationCondition?  // factual or interpretive
  effects: Vec<MandatedEffect>         // what must happen
  narrative_justification: String      // why this is mandated (for debugging/authoring)
}

MandatedEffect =
  | TensorShift { character: CharacterId, element: ElementRef, delta: f32 }
  | RelationshipShift { from: CharacterId, to: CharacterId, dimension: Dimension, delta: f32 }
  | RevealInformation { to: CharacterId, content: RevelationId }
  | ActivateEcho { character: CharacterId, echo: EchoRef }
  | SurfaceShadow { character: CharacterId, motivation: MotivationRef }
  | ... // extensible
```

**The separation matters because**:
- **Factual events** are *what happened* — objective, World Agent validated, immediately processed, queryable ground truth
- **Interpretive judgments** are *what was inferred* — subjective, ML-produced, confidence-weighted, evaluated during frame computation, also queryable but with different epistemic status
- **Subscriptions** are *what would cause a shift* — can be factual (real-time reactive) or interpretive (frame-time evaluative), part of the tensor, character-specific
- **Mandates** are *what must happen* — authored guarantees, part of the narrative graph, story-level requirements, can use either condition type

This enables rich debugging:
- "Why did Sarah's shadow anger surface?" → Trace to: interpretive judgment (concealment detected from Adam, confidence 0.73) matched her `BehaviorReading { "concealment" }` condition
- "Why did the Wolf's protection instinct activate?" → Trace to: factual event `PhysicalContact(sarah, threat)` matched his factual subscription
- "Why did Tommy's revelation hit so hard?" → Trace to: mandated shift on scene "beth_encounter" guaranteed echo activation

### Trigger Matching

Contextual triggers on tensor elements are matched against scene properties and events:

```
// TriggerCondition is what factual subscriptions match against.
// These are objective conditions that the World Agent can validate.
TriggerCondition =
  // Match against current scene state:
  | SettingFeature(feature: SettingFeature)
  | CharacterPresent(character: CharacterId)
  | ConfigurationType(config: ConfigType)
  | EmotionalRegister(register: EmotionalRegister)
  | StakeInvolved(stake: Stake)

  // Match against events:
  | EventOccurred(event_pattern: EventPattern)
  | EventSequence(patterns: Vec<EventPattern>, within: Duration?)
  | TopicMentioned { topic: String }  // a topic/name was spoken or referenced

  // Match against world state (World Agent queryable):
  | WorldState { query: String, equals: String }  // e.g., "tommy_status" == "lost"
  | Revelation(content: RevelationId)             // specific information was revealed

  // Match against accumulated state:
  | AccumulatedStress(threshold: f32)
  | RelationshipState { with: CharacterId, dimension: Dimension, threshold: f32 }
  | LedgerQuery(query: LedgerQuery)  // "has X ever happened?"

  // Composition (see Design Decision 5 for the full set-theoretic model):
  // Atomic TriggerConditions are composed via TriggerPredicate operations
  // (All, Any, Not, Difference, Threshold) rather than inline Compound.
  // The old Compound(All | Any | Sequence) is superseded.
  // Temporal ordering uses TemporalPredicate (After, Sequence, Since).

// EventPattern matches against NarrativeEvents
EventPattern =
  | AnyEvent(event_type: EventType)
  | SpecificEvent(event: NarrativeEvent)
  | EventWithActor(actor: CharacterId, event_type: EventType)
  | EventInvolving(character: CharacterId)  // actor or recipient
  | TopicMentioned { topic: String }        // convenience for topic events
```

**Legacy Trigger struct** (retained for backward compatibility with existing case studies, but now understood as shorthand for a TriggerSubscription):

```
Trigger {
  condition: TriggerCondition
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
| `warmth_reserve` axis with triggers | PersonalityAxis | Yes — bipolar, tags for compositional matching, contextual triggers as subscriptions |
| `loyalty_to_family: 0.95` | PersonalValue with supports/suppresses | Yes — supports find_tommy, suppresses anger_at_tommy, awareness: Articulate |
| `fear_that_she_is_not_enough` | Motivation (shadow) with mirrors | Yes — mirrors prove_herself_capable, activation_conditions typed |
| `supernatural_perception: 0.8` | CharacterCapacity (supernatural domain) | Yes — typed limitations + notes, enables seeing_hidden_paths |
| `grief: 0.8` with decay | EmotionalState with DecayModel | Yes — slow decay, sustained_by conditions |
| `echo: tommy_withdrawing` | EchoPattern with config_shift | Yes — trigger conditions typed, configuration shift on fire |
| The Other Bank activation | Frame via pipeline | Yes — trigger matching → echo → activation → configuration → frame |

### Wolf Validation Checklist

| Case Study Element | Schema Type | Captured? |
|---|---|---|
| `connection_isolation` reinterpreted axis | PersonalityAxis with reinterpretation_note | Yes |
| `duty_is_identity` primordial value | PersonalValue with temporal_layer: primordial, awareness: Structural | Yes |
| `ensure_sarah_fails` contradicting `protect_sarah` | Motivation.contradicts relationship | Yes |
| `curiosity_about_sarah` shadow want | Motivation (shadow, forming) | Yes |
| World-rending power (supernatural) | CharacterCapacity with domain: supernatural | Yes |
| Archetypal echo | EchoPattern with echo_type: archetypal | Yes |
| Nascent connection (emergent) | Not a stored element — emergent from configuration | Correct — not represented as element |
| Frame register: somatic/spatial | PsychologicalFrame.register: non_human | Yes |

---

## Design Decisions

The following questions were identified during schema development and have been resolved. Each decision has architectural implications that propagate through the system.

### 1. Vocabulary Architecture: Closed, Extensible Enums

**Decision**: Typed vocabularies (SettingFeature, EmotionalRegister, Theme, CapacityDomain, etc.) are **closed enums** with a **pluggable extension mechanism** at compile or cross-linking time.

**Rationale**: The event-driven trigger system, the Rust type system, and the need for computational efficiency all favor closed vocabularies. Enum-based matching is fast, exhaustive, and caught by the compiler. The trigger system's set-theoretic evaluation (see below) requires that conditions be discrete, matchable atoms — open-ended string matching would undermine both performance and reliability.

**Extension mechanism**: Story-specific vocabulary extensions are registered at compile time via a trait-based plugin system. A story designer working on Bramblehoof can define `SettingFeature::FeyWild` or `Theme::CrushedCreativity` without modifying the core vocabulary. The trigger matching system handles unknown extension variants via a generic fallback path.

**Implication**: Vocabulary definitions become part of the story designer's deliverable, alongside narrative graphs, tensors, and scene definitions. The core vocabulary provides a substantial base; extensions are additive.

```
// Core vocabulary — closed enum in storyteller-core
SettingFeature =
  | Water | Forest | Mountain | Cave | Building | Road | Bridge
  | LiminalSpace | Sickbed | Hearth | Battlefield | Market
  | Night | Dawn | Dusk | Storm | Calm | Cold | Heat
  | ...  // enumerated, not open-ended

// Story extension — registered at compile/link time
// e.g., in a Bramblehoof story crate:
SettingFeature::ext =
  | FeyWild | CorruptedLeyLine | TavernStage | ForgottenShrine

// Each vocabulary term can carry structured metadata:
VocabularyEntry<T> {
  variant: T,                    // the enum variant itself
  semantic_tags: HashSet<Tag>,   // for compositional trigger matching
  description: String,           // for LLM consumption / frame synthesis
  implied_features: Vec<T>,      // e.g., Cave implies Darkness, Enclosed
}
```

**Metadata within vocabularies**: Each vocabulary term can carry structured metadata — semantic tags (for compositional matching), human-readable descriptions (for frame synthesis), and implied features (Cave implies Darkness). This gives closed vocabularies the specificity of open ones without sacrificing type safety.

### 2. Authoring Model: Sensible Defaults, Deep Management Available

**Decision**: The system provides **sensible defaults and low-friction authoring** at every level. Fine-grained nuance is possible but never required. Inter-element relationships, contextual triggers, and even character traits can be inferred from common types or filled with structured pseudo-random generation, then refined by the author if desired.

**Rationale**: A character with a `Motivation(shadow)` that mirrors a `Motivation(deep)` can be detected by structural analysis — if two motivations have overlapping domains and opposing valences, the system can propose a `Mirrors` relationship. Similarly, a character with high `loyalty_to_family` and a missing family member can generate default emotional states and echo patterns. The author's role is to confirm, override, or deepen — not to specify every relationship from scratch.

**Provenance tracking**: Every inferred or defaulted element carries a provenance tag:

```
Provenance =
  | Authored                     // designer specified explicitly
  | Inferred { rule: InferenceRule, confidence: f32 }
  | Generated { seed: u64, template: TemplateId }
  | Confirmed { original: Box<Provenance> }  // author reviewed and accepted
  | Overridden { original: Box<Provenance> }  // author reviewed and changed
```

This enables the authoring toolchain to surface "things the system guessed" for review, distinguish between "author hasn't looked at this" and "author confirmed this," and support iterative refinement without losing the history of how elements were produced.

**Implication**: The tensor schema's element types (PersonalityAxis, Motivation, etc.) all gain an optional `provenance: Provenance` field. Inference rules are defined alongside the schema as a companion system.

### 3. Entity Model: Everything Is an Entity

**Decision**: **Everything in the system is an Entity.** A cup on a shelf, a crying widow, a mountain range, and a protagonist all share a single underlying Entity type. What differs is their *component configuration* — which capabilities, communicability dimensions, and persistence characteristics they currently have.

This collapses the taxonomy from `world_design.md` (Characters → Presences → Conditions → Props) into a **promotion/demotion lifecycle** on a single type.

**Lifecycle**:

```
(not yet an entity)        — part of narrative description, no persistence
       ↓ interaction/engagement
Ephemeral Entity           — created when engaged with, minimal components
       ↓ sustained interaction / narrative weight
Persistent Entity          — tracked, may have tensor elements, decay-resistant
       ↓ accumulated meaning / communicability
Full Entity                — rich tensor, relational web edges, frame-eligible

// And the reverse:
Full Entity → Persistent → Ephemeral → (dissolved)
  via decay, distance, disengagement, narrative irrelevance
```

**Decay**: Entities that are not interacted with or retained decay along a distance/time curve. The flower given away to a stranger, the stone skipped across water — these dissolve back into narrative description. Decay rates are configurable per entity and per story.

**Communicability as configuration**: The four dimensions from `world_design.md` — surface area, translation friction, timescale, capacity to turn toward — become **component values** on an entity, not a separate type hierarchy. A mountain has high surface area, high translation friction, geological timescale, and turns toward everything equally. These values determine what the entity can do in the system: whether it can participate in power dynamics, whether it can have relational web edges, whether it needs a psychological frame.

**Bevy ECS implication**: Entities are literal ECS entities. Their "tier" is determined by which components are attached. Promotion adds components (a tensor slice, a relational edge, a communicability profile). Demotion removes them. The system queries for entities by component presence, not by type tag.

**Implication for this schema**: The tensor element types defined in this document are components that *any* entity can acquire — not just characters. A Presence (the Shadowed Wood) might acquire a small set of PersonalityAxes (mood, hostility) and EmotionalStates (darkening) without needing the full Character tensor. The schema is already component-shaped; the Entity model makes this explicit.

See `entity-model.md` for the full Entity lifecycle specification.

### 4. Training Data: Combinatorial Generation Matrices

**Decision**: Training data for the ML inference layer is produced through **combinatorial generation matrices** — structured dimensions (genre, tone, narrative type, common tensions, relational dynamics and their inverses) that can be combined and recombined to produce significantly different patterns.

**Approach**:

1. **Define generation dimensions**: genre (dark fantasy, literary, comedic, etc.), tone (tense, lyrical, bleak, warm), narrative tension types (betrayal, sacrifice, homecoming, loss), relational dynamics (trust-building, power-inversion, dependence, separation), character archetypes (with tensor templates).

2. **Build combinatorial matrices**: Each combination of dimensions produces a scenario skeleton — a character configuration, a scene context, and a set of expected frame outputs.

3. **Use LLM sub-agents to generate**: Given a scenario skeleton, LLM agents produce concrete tensor data, scene contexts, and candidate psychological frames as serializable outputs (JSONL or similar structured format).

4. **Validate against schema**: Generated data must conform to the formal schema (see Decision 6). Non-conforming output is rejected automatically. Conforming output enters a review pipeline.

5. **Human review and refinement**: Generated scenarios are reviewed for quality, plausibility, and narrative coherence. Accepted scenarios become training data; rejected ones inform the generation templates.

**Plot mechanics**: The attractor basin model and power framework provide structural templates for plot dynamics — how tensions escalate, where power configurations shift, when echoes should fire. These structural patterns, combined with the generation dimensions, produce training scenarios that are both diverse and narratively grounded.

**Implication**: The tensor schema must be serialization-friendly from the start. Every type needs a clean JSON representation, reinforcing Decision 6.

### 5. Trigger Language: Set-Theoretic Composition of Atomic Conditions

**Decision**: Trigger conditions are **atomic, discrete predicates** composed via **set-theoretic operations** (union, intersection, difference, complement). The `Compound` type is replaced with explicit set operations. Evaluation is bounded: each trigger predicate is evaluated against the **truth set** — the current state of all conditions known to be true at evaluation time.

**The truth set**: At any moment, the system maintains a set of propositions that are currently true — characters present, setting features active, events that have occurred, world state values, accumulated thresholds crossed. This is the factual truth set. The ML inference layer extends it with confidence-weighted interpretive judgments during frame computation.

**Atomic conditions**: Each `TriggerCondition` and `ActivationCondition` variant is a single, discrete predicate that either matches or does not match against the truth set. These are the atoms.

**Set composition**:

```
TriggerPredicate =
  | Atom(condition: TriggerCondition)      // single condition
  | All(predicates: Vec<TriggerPredicate>) // intersection — all must match
  | Any(predicates: Vec<TriggerPredicate>) // union — at least one must match
  | Not(predicate: TriggerPredicate)       // complement
  | Difference(include: TriggerPredicate, exclude: TriggerPredicate)
                                           // include AND NOT exclude
  | Threshold(predicates: Vec<TriggerPredicate>, min_match: u32)
                                           // at least N of M must match

// Replaces the old Compound(conditions, mode: All | Any | Sequence)
// Sequence is handled separately — see temporal constraints below.
```

**Temporal constraints**: Sequence (A then B then C within duration D) is not a set operation — it requires ordered evaluation against the event ledger. Temporal predicates query the ledger directly:

```
TemporalPredicate =
  | After(event: EventPattern, predicate: TriggerPredicate)
    // predicate must be true AND event must have occurred before now
  | Sequence(events: Vec<EventPattern>, within: Duration?)
    // events must have occurred in order, optionally within a time window
  | Since(event: EventPattern, predicate: TriggerPredicate)
    // predicate has been continuously true since event occurred
```

**Evaluation model**: Factual predicates are evaluated against the truth set in real-time as events arrive — this is a bounded set membership check, computationally equivalent to a SQL query against an indexed table. Interpretive predicates are evaluated during frame computation when the ML layer produces judgments that enter the truth set with confidence weights. For interpretive atoms, confidence is factored into threshold calculations: an atom with 0.6 confidence contributes 0.6 toward a `Threshold(min_match: 2)` requirement, not 1.0.

**Practical limit**: Trigger predicates are constrained to a maximum nesting depth (configurable, default 4). This prevents pathological composition while allowing expressive conditions. The most common patterns — "A and B," "any of A, B, C," "A and not B" — are depth 1-2.

### 6. Schema Specification: Formal, Additive, Extensible

**Decision**: All schema types are formally specified using **JSONSchema** (or OpenAPI-compatible schema definitions). Schema evolution is **additive** — new fields, new enum variants, new element types. Breaking changes require a versioned migration.

**Generation strategy**: The canonical type definitions live in Rust. JSONSchema is **derived from the Rust types** (via `schemars` or equivalent), not maintained separately. This ensures a single source of truth. The derived schemas are used by:

- Authoring tools (validation of designer input)
- Training data pipeline (validation of generated output)
- LLM sub-agents (structured output schemas for generation)
- Story import/export (serialization/deserialization)

**Extensibility contract**: The schema distinguishes between:

- **Core types** — defined in `storyteller-core`, stable, backwards-compatible
- **Extension types** — defined per-story or per-plugin, registered at compile time (consistent with Decision 1)
- **Optional fields** — new fields added with defaults, old data remains valid

**Versioning**: Schema versions follow a `major.minor` convention. Minor versions are additive (new optional fields, new enum variants). Major versions may restructure (with migration tooling). The goal is to reach a stable `1.0` schema before significant content is authored against it.

### 7. Performance Architecture: Scene-Boundary Framing, Parallel Input, Warmed Cache

**Decision**: The performance problem is addressed architecturally rather than through per-step latency budgets. Three key mechanisms:

**Scene-boundary framing**: Agents are not constantly refreshed. The foundational frame (tensor activation, relational configuration, psychological frame) is computed at **scene entry** — the UX scene-change mechanic provides a natural boundary for this work. Within a scene, incremental updates (event-driven axis shifts, echo activations) modify the frame rather than recomputing it from scratch. This amortizes the expensive pipeline over the scene duration.

**Parallel input processing**: The player's text input is **not** a chat message to the Narrator. It is a signal broadcast to all foundational agents simultaneously:

```
PlayerInput → [
  ClassifierAgents  → typed events (NarrativeEvent, action parsing)
  Narrator          → narrative rendering (with scene context + frame)
  Storykeeper       → narrative graph evaluation (position, gravity, gates)
  Reconciler        → multi-character coordination (if applicable)
  WorldAgent        → constraint validation, world-state updates
]
```

Classifier agents are a pre-processing layer that parse player input into typed events before the other agents see it. This enables the event-driven trigger system to fire in real-time as the player acts, without waiting for the full agent processing pipeline.

**Warmed cache**: The Narrator (and other agents) maintain access to **local cached context** — recent scene history, the current psychological frame, active relational configurations, relevant tensor subsets. This context is built at scene entry and incrementally updated. The Narrator should never need to "go look something up" during scene execution — everything needed for fluent narration is in the warmed cache.

**Delegation model**: Most of the time, foundational agents are consciously delegating to one another. The Storykeeper recognizes that a player action is primarily a World Agent concern (physical constraint) and delegates; the World Agent recognizes that the constraint has narrative implications and signals the Storykeeper. This delegation is itself an event that other agents can observe and react to. The event bus carries both player-originated and agent-originated events.

**Implication for the frame computation pipeline**: Steps 1-4 (trigger matching through configuration computation) execute at scene entry and produce the base frame. Step 5 (frame synthesis) produces the initial psychological frame. During the scene, events trigger incremental updates that modify the frame without re-running the full pipeline. A full recomputation only occurs at scene boundaries or when a sufficiently dramatic event (echo activation, revelation) warrants it.

---

## Open Considerations

The design decisions above resolve the original open questions. The following considerations remain as areas for future specification — they are practical engineering questions that will be resolved during implementation rather than design questions that block architecture.

1. **Vocabulary enumeration**: The core enum variants for each vocabulary type need to be concretely listed. The architecture is decided (closed + extensible); the specific terms require a vocabulary definition pass, likely during Bramblehoof workshop development.

2. **Inference rule library**: The sensible-defaults system (Decision 2) needs a concrete library of inference rules — "if motivation X contradicts motivation Y, propose a Mirrors relationship." These rules will emerge from case study development.

3. **Entity decay curves**: The Entity lifecycle (Decision 3) specifies decay but not the specific mathematical curves. These need calibration against play-testing.

4. **Classifier agent design**: The parallel input architecture (Decision 7) introduces classifier agents as a pre-processing layer. Their design — how they parse natural language into typed events, what confidence thresholds they use, how they handle ambiguity — is a significant implementation concern.

5. **Truth set implementation**: The set-theoretic trigger language (Decision 5) requires an efficient truth set data structure. Options include bitsets (for small vocabularies), indexed stores (for larger sets), or a lightweight embedded query engine. The choice depends on cardinality measurements during implementation.
