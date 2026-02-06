# Scene Model

## Purpose

This document specifies the **Scene** as the fundamental unit of play in the storyteller system. Where the narrative graph (`narrative_graph.md`, `narrative-graph-case-study-tfatd.md`) defines the gravitational landscape, this document specifies what happens *inside* each gravitational body — how a scene is structured, how it operates during play, and how it mediates between the authored story and the player's experience.

The Scene is both a structural unit and a creative constraint. It bounds what is possible, what is present, and what matters — and in doing so, it gives the player meaningful action and gives the system's agents manageable context. Constraint here is a gift: it is what makes play productive rather than aimless, and what makes agent coordination tractable rather than overwhelming.

### Relationship to Other Documents

- **`narrative_graph.md`** defines scenes as gravitational bodies with mass, approach vectors, and departure trajectories — the *external* properties that determine how scenes relate to each other.
- **`narrative-graph-case-study-tfatd.md`** applies these properties to real scenes from TFATD — scene inventories, mass calculations, information gates.
- **`agent-message-catalog.md`** specifies the messages that flow between agents during a scene — the communication protocol.
- **`tensor-schema-spec.md`** specifies the frame computation pipeline — what happens at scene entry to prepare agents for play.
- **`entity-model.md`** specifies the Entity lifecycle — how entities are promoted, tracked, and decayed within scenes.
- **This document** specifies what a scene *is* from the inside: its anatomy, its lifecycle, its action space, and its relationship to the player.

---

## The Scene as Unit of Play

### Why the Scene

This is a storytelling engine, not an open-world system. The player is not navigating an infinite possibility space — they are participating in a story that has shape, intention, and direction. The scene gives that shape its operational form.

Without scene boundaries, two problems dominate:

**Aimlessness.** An unstructured space of infinite possibility produces paralysis, not freedom. The player who can "do anything" often does nothing — or does something trivial, because there is no signal about what matters. Meaningful action requires a frame within which some things are more interesting, more consequential, more responsive than others. The scene provides this frame.

**Context overwhelm.** The system's agents — Narrator, Character Agents, Storykeeper, World Agent, Reconciler — must reason about the current situation with bounded context. An unbounded situation means unbounded context: every character who might be relevant, every entity that might matter, every relationship that might activate, every piece of history that might surface. The scene bounds all of these: *these* characters are present, *these* entities are in play, *this* history is relevant, *these* goals are achievable. The agents can focus.

The scene is the narrative equivalent of a theatrical scene: a unit of dramatic action with entrances, exits, stakes, and a shape. It has a beginning (scene entry), a middle (active play), and an end (scene exit, which selects the next scene). Within it, the player has freedom. But the freedom is rendered — it has edges, texture, and direction.

### Constraint as Creative Gift

Constraints in this system are not limitations imposed on the player. They are the architecture that makes meaningful play possible.

A scene that says "you are in a small room with a dying boy and you can hear two men arguing outside the door" gives the player an enormous amount to work with — emotional stakes, sensory detail, mysteries (who are the men? why are they arguing? what is wrong with the boy?), potential actions (listen, comfort the boy, open the door, look around the room). The player's freedom is real, but it is *situated*. The situation gives freedom its meaning.

This is how all good storytelling works. A scene in a novel is not an arbitrary slice of time — it is a bounded unit in which specific things are at stake, specific characters are present, and specific possibilities exist. Our scenes follow the same logic, extended to accommodate player agency.

---

## Scene Anatomy

A scene is a structured object with the following components. Not all components are present in every scene — connective scenes may have minimal stakes and no information gates, while gravitational scenes may have rich specifications across every dimension.

### Cast

The characters present or available in the scene.

```
SceneCast {
  player: PlayerPresence,                   // always present
  required_characters: Vec<CharacterRef>,   // must be present for this scene to function
  contingent_characters: Vec<ContingentPresence>, // may be present based on prior state
  potential_arrivals: Vec<ArrivalCondition>, // characters who might enter mid-scene
}

PlayerPresence {
  current_frame: PsychologicalFrame,        // computed at scene entry
  active_tensor_subset: ActivatedTensor,    // scene-relevant player-character elements
  information_state: InformationLedger,     // what the player knows
}

ContingentPresence {
  character: CharacterRef,
  condition: TriggerPredicate,              // what must be true for them to appear
  arrival_mode: ArrivalMode,                // pre-positioned, enters during scene, summoned
}

ArrivalCondition {
  character: CharacterRef,
  trigger: TriggerPredicate,                // event or state that causes arrival
  narrative_signal: String,                 // how the Narrator introduces them
}
```

The cast bounds the relational computation. Only present characters need psychological frames. Only edges between present characters (and the player) need activation. This is the primary mechanism for controlling context size: a scene with 2 characters is dramatically simpler for the system than a scene with 6.

**Character Agent instantiation**: Character Agents are ephemeral — instantiated at scene entry from the Storykeeper's tensor data, relational web subset, and computed psychological frame. They exist for the duration of the scene and are released at scene exit. Their frame is their entire world.

### Setting

The place where the scene occurs.

```
SceneSetting {
  setting_type: SettingType,
  description: String,                      // authored or generated scene description
  features: Vec<SettingFeature>,            // from the closed vocabulary (tensor-schema-spec.md, Decision 1)
  atmosphere: AtmosphericProfile,           // mood, weather, light, sound, temperature
  spatial_affordances: Vec<SpatialAffordance>, // what the space allows (paths, doors, objects, vistas)
  constraints: Vec<SettingConstraint>,      // what the space prevents (walls, darkness, distance)
}

SettingType =
  | Authored { scene_id: SceneId }          // designed by story designer with specific properties
  | Connective { template: GenreTemplate, context: NarrativeContext }
                                            // generated from genre patterns + current state
  | Threshold { from_zone: ZoneId, to_zone: ZoneId }
                                            // transitional space between narrative zones

AtmosphericProfile {
  tonal_signature: Vec<TonalQuality>,       // elegiac, tense, warm, uncanny, etc.
  sensory_details: String,                  // for Narrator rendering
  mood_shift_potential: Vec<MoodShift>,     // how the atmosphere can change based on events
}

SpatialAffordance {
  element: String,                          // "the door," "the stream," "the road north"
  interaction_type: InteractionType,        // traversal, examination, manipulation, conversation_prop
  narrative_weight: f32,                    // how much attention the Narrator should give this
  entity_ref: Option<EntityId>,             // if this affordance is a tracked entity
}
```

**Authored settings** are fully specified by the story designer — the Fever Bed, the Other Bank. They carry rich atmospheric detail, specific spatial affordances, and authored narrative weight.

**Connective settings** are generated from genre templates modified by current narrative context. A journey through the Shadowed Wood between authored scenes doesn't need full specification — it needs genre-appropriate atmosphere (dark, cold, misty), spatial affordances (the path, the trees, the sounds), and enough responsiveness to feel alive. The World Agent generates connective settings from the story's genre contract and current world state.

**Threshold settings** are transitional — the doorway between the house and the Wood, the riverbank between safety and the unknown. They often correspond to `threshold` scene types in the narrative graph and carry ritual or symbolic weight.

### Stakes and Goals

What the scene exists to accomplish or make possible.

```
SceneStakes {
  primary_goals: Vec<SceneGoal>,            // what the scene is designed to achieve
  secondary_goals: Vec<SceneGoal>,          // enriching but not essential outcomes
  information_gates: Vec<InformationGate>,  // revelations available in this scene
  relational_potentials: Vec<RelationalShift>, // relationship changes this scene can produce
  entity_transformations: Vec<EntityTransformation>, // state changes to entities
}

SceneGoal {
  id: GoalId,
  description: String,                      // what this goal achieves narratively
  completion_condition: TriggerPredicate,   // when this goal is satisfied
  state_changes: Vec<StateChange>,          // what changes when completed
  unlocks: Vec<SceneNodeRef>,               // posterior scene-nodes this enables
  visibility: GoalVisibility,               // how aware the player should be
}

GoalVisibility =
  | Overt                                   // the player should understand this is available
  | Signaled                                // the Narrator signals its presence through craft
  | Hidden                                  // discoverable only through exploration or insight
  | Structural                              // fires regardless of player action (scene always produces this)

InformationGate {
  gate_id: GateId,
  content: String,                          // what information is released
  condition: TriggerPredicate,              // what must happen for the gate to open
  structural: bool,                         // if true, always fires in this scene
  delivery_mode: DeliveryMode,              // dialogue, observation, atmosphere, direct narration
  downstream_gates: Vec<GateId>,            // gates in future scenes that this primes
}

RelationalShift {
  between: (EntityId, EntityId),            // which relationship
  dimension: SubstrateDimension,            // which substrate dimension (trust, affection, debt, etc.)
  direction: ShiftDirection,                // increase, decrease, or transform
  trigger: TriggerPredicate,                // what causes the shift
  magnitude: f32,                           // how significant
}
```

**Goals are not quests.** They are not items on a checklist. They are the scene's authored intentions — the changes in state, relationship, information, or understanding that the story designer has placed within the scene's possibility space. Some are overt (the scene is clearly about finding the witch). Some are signaled (the Narrator emphasizes the cup on the table; the player may or may not investigate). Some are hidden (a kindness shown here will matter three scenes later). Some are structural (the scene always reveals that Tommy is lost, regardless of what the player does).

### Entity Budget

The entities present or available in the scene, governed by the Entity model's lifecycle.

```
SceneEntityBudget {
  authored_entities: Vec<EntityRef>,        // placed by the story designer, always present
  promoted_entities: Vec<EntityRef>,        // promoted from narrative description during play
  max_tracked_entities: u32,                // budget limit for this scene
  promotion_threshold: f32,                 // narrative weight required for new promotion
  decay_suspended: Vec<EntityId>,           // entities whose decay is paused during this scene
}
```

The entity budget is a practical constraint. In a richly described scene, dozens of things could be promoted to tracked entities. The budget limits active tracking to a manageable number, with priority based on narrative weight and recent interaction. When the budget is full, a new promotion requires either an existing entity to decay or the new entity to have higher narrative weight than the lowest-weight tracked entity.

**Authored entities** are always present and do not count against the promotion budget. The sickbed in the Fever Bed, the river in the Crossing — these are part of the scene's definition.

### Graph Position

Where this scene sits in the narrative DAG and what that position implies.

```
SceneGraphPosition {
  scene_id: SceneId,
  scene_type: SceneType,                    // gravitational, connective, gate, threshold
  narrative_mass: NarrativeMass,            // computed mass (authored + structural + dynamic)

  anterior_nodes: Vec<AnteriorConnection>,  // scenes that could have led here
  posterior_nodes: Vec<PosteriorConnection>,  // scenes that might follow

  active_approach_vector: ApproachVector,   // how the player actually arrived
  available_departures: Vec<DepartureTrajectory>, // currently reachable exits

  narrative_context: NarrativeContext,      // the accumulated thematic, emotional, and historical situation
}

AnteriorConnection {
  scene_ref: SceneNodeRef,
  path_taken: bool,                         // did the player actually come this way?
  thematic_inheritance: Vec<Theme>,         // themes carried forward from this predecessor
  emotional_residue: Vec<EmotionalState>,   // emotional states that persist from this path
  information_inherited: Vec<GateId>,       // gates that fired on the way here
}

PosteriorConnection {
  scene_ref: SceneNodeRef,
  unlock_condition: TriggerPredicate,       // what must be true for this exit to be available
  currently_unlocked: bool,                 // is this exit currently reachable?
  gravitational_pull: f32,                  // how strongly the story pulls toward this exit
  momentum_required: MomentumType,          // what kind of energy carries the player there
}

NarrativeContext {
  active_themes: Vec<Theme>,                // themes currently in play
  emotional_register: Vec<EmotionalState>,  // the emotional tone the player carries
  unresolved_tensions: Vec<TensionRef>,     // tensions from prior scenes still active
  information_state: InformationSummary,    // what the player knows, compressed
  relationship_summary: RelationshipSummary, // state of key relationships, compressed
  time_in_story: NarrativeTimestamp,        // where we are in story-time
}
```

The graph position carries with it everything the scene needs to know about *how the player got here* — which themes are active, what emotional state they carry, what tensions are unresolved, what they know. This is the "situation" that grounds the scene.

**Anterior nodes** provide thematic inheritance — the themes, emotions, and information that flow forward from previous scenes. A player who arrives at the Other Bank having witnessed Kate's water blessing carries different thematic material than one who rushed past it.

**Posterior nodes** are the possible futures. Not all are currently unlocked. Some require goals to be completed, gates to fire, or relationships to shift. The set of unlocked posterior nodes changes during the scene as the player acts — completing a goal might unlock a new exit, while failing to engage might leave only the default continuation.

### Warmed Data

The context package assembled at scene entry for the Narrator and other agents.

```
WarmedData {
  // For the Narrator (see MSG-SK01: SceneContext):
  scene_description: String,
  tonal_signature: Vec<TonalQuality>,
  thematic_register: Vec<Theme>,
  present_characters: Vec<CharacterBrief>,  // visible state + relationship to player
  available_interactions: Vec<String>,      // what the player can do here
  atmospheric_notes: String,               // sensory details, mood
  narrator_guidance: String,               // specific instructions for this scene
  recent_history: Vec<NarrativeEvent>,     // last N events for continuity
  significance_map: Vec<SignificanceHint>, // what to emphasize (without revealing why)

  // For the Storykeeper:
  full_graph_position: SceneGraphPosition,
  all_gates: Vec<InformationGate>,
  all_goals: Vec<SceneGoal>,
  active_triggers: Vec<TriggerSubscription>,

  // For the World Agent:
  world_state_slice: WorldStateSlice,       // relevant world state for this scene
  active_entities: Vec<EntityState>,        // entity states requiring tracking
  constraint_set: Vec<WorldConstraint>,     // hard constraints active here

  // For Character Agents (per character):
  psychological_frame: PsychologicalFrame,  // computed at scene entry
  scene_awareness: String,                  // what this character knows about the scene
  relational_context: Vec<RelationalEdge>,  // relevant relationships
  character_goals: Vec<CharacterGoal>,      // what this character wants in this scene
}
```

The warmed data is the "everything you need to run this scene" bundle. It is computed at scene entry — this is where the frame computation pipeline runs, where the Storykeeper assembles context, where the World Agent loads relevant state. The goal is that once play begins, no agent needs to "go look something up." Everything needed for fluent scene execution is in the warmed cache.

**The Narrator's warmed data** is especially critical. The Narrator knows nothing beyond what the Storykeeper provides (see agent-message-catalog.md, MSG-SK01). The warmed data is the Narrator's entire world for this scene — rich enough to render the scene vividly, constrained enough to prevent information leakage.

---

## The Rendered Space

### What the Player Can Do

The rendered space is the bounded set of meaningful actions available to the player within a scene. It is not a list of commands. It is the narrative space within which action is responsive — where what the player does *matters* and the world *responds*.

The rendered space is defined by the intersection of:

1. **Setting affordances** — what the physical/spatial environment allows. Doors to open, paths to follow, objects to examine, vistas to observe.
2. **Character availability** — who the player can talk to, observe, follow, confront, comfort.
3. **Entity presence** — things that can be interacted with, picked up, used, investigated.
4. **Information horizon** — what the player can perceive, remember, infer. You can't investigate what you don't know exists (but you can explore and discover).
5. **Constraint framework** — hard constraints (physics), soft constraints (character capacity), perceptual constraints (what can be sensed). From `system_architecture.md`.

Within this space, the player has genuine freedom. They can talk to any present character about anything. They can examine any affordance. They can attempt actions the designer didn't anticipate — and the system should respond meaningfully, even if the response is a constraint ("the door won't budge; it feels less like a locked door and more like a door that was never meant to open").

### What Lies Outside

Actions that fall outside the rendered space are not forbidden — they are *unresponsive*. The system doesn't say "you can't do that." It says, through the Narrator, that the world doesn't yield much in that direction.

A player who counts blades of grass in a scene about a dying boy gets a response — but a thin one. The Narrator acknowledges the action and gently redirects attention: "The grass is damp with morning dew. Inside, you can hear Tom's breathing — shallow, uneven." The rendered space has texture and grain; the interesting paths are *more responsive* than the uninteresting ones. This is how the system guides without railroading.

**The edge of the rendered space** is where the scene meets the narrative graph. "I leave the house and walk north" is a valid action, but it moves the player toward a scene exit — a departure trajectory. If the trajectory is unlocked, the scene transitions. If it's not (because a gate hasn't fired, or conditions aren't met), the world offers gentle resistance: "The road north disappears into mist — thick, the kind that turns you around. Something about this place isn't finished yet."

### The Three-Tier Constraint Framework in Practice

Within the rendered space, the constraint framework from `system_architecture.md` operates:

**Hard constraints** — the world's physics. In a story without flight, you can't fly. In a story with magic, magic works according to its rules. The World Agent enforces these, and the Narrator renders them as world-responses, never as bare refusals. "You try to lift the boulder. It doesn't move. It would take three strong men, and you are a twelve-year-old girl — but you feel something in the stone, a warmth, as if it noticed you trying."

**Soft constraints** — character capacity in context. Your character *could* do this, but it would be difficult, costly, or out of character. The Narrator renders these as internal friction: "You could say it. The words are right there. But something stops you — a tightness in your chest, the memory of the last time you said something you couldn't take back."

**Perceptual constraints** — what can be sensed. You can't see what's behind a closed door. You can't hear a whispered conversation from across a room. But you can *try* — and the system rewards the attempt with whatever partial information is available. "You press your ear to the door. You catch fragments — a man's voice, angry, and another voice you almost recognize, calm as still water."

---

## Scene Lifecycle

### Entry

Scene entry is the most computationally expensive moment in the play loop. It is where the system prepares everything needed for fluent scene execution.

```
Scene Entry Pipeline:
  1. SCENE SELECTION
     Storykeeper evaluates departure trajectory from previous scene
     → selects next scene based on player state + narrative gravity
     → determines scene_type and loads scene definition

  2. CAST ASSEMBLY
     Storykeeper determines required + contingent presences
     → evaluates contingent conditions against current state
     → assembles cast list

  3. FRAME COMPUTATION (per character)
     For each present character (including player):
       Run tensor-schema-spec.md Frame Computation Pipeline (Steps 1-5)
       → trigger matching, echo detection, tensor activation,
         configuration computation, frame synthesis
       → produce PsychologicalFrame (~200-400 tokens)

  4. ENTITY INITIALIZATION
     World Agent loads authored entities for this scene
     → initializes entity budget
     → suspends decay for scene-relevant entities
     → resolves setting description and affordances

  5. CONTEXT WARMING
     Storykeeper assembles Narrator context (MSG-SK01: SceneContext)
     Storykeeper assembles per-character context
     World Agent provides world state slice + constraints
     → produce WarmedData bundle

  6. SCENE PRESENTATION
     Narrator receives WarmedData and renders the scene opening
     → atmospheric description, character introductions, situation framing
     → the player sees the scene for the first time
```

**The scene-change UX**: Scene entry is the natural moment for a "scene change" in the player's experience. This might be a brief transition (a visual or textual scene break), a loading moment (during which the pipeline runs), or a narrative passage ("Three days of walking through the mist, and then — a clearing, a stream, and something on the other bank that shouldn't be there"). The transition signals to the player that something has shifted — new constraints, new possibilities, a new frame.

### Active Play

During active play, the turn cycle runs:

```
Turn Cycle (per player input):
  1. PLAYER INPUT
     Player enters text
     → broadcast to all agents in parallel (tensor-schema-spec.md, Decision 7)

  2. CLASSIFICATION
     Classifier agents parse input into typed events
     → action_type (speech, action, examination, question, meta)
     → resolved targets (characters, entities, locations)
     → inferred intent

  3. EVENT PROCESSING
     Events enter the truth set (tensor-schema-spec.md, Decision 5)
     → trigger matching runs against active subscriptions
     → echo detection runs for activated triggers
     → entity promotion evaluated (did the player engage something new?)

  4. AGENT COORDINATION
     Storykeeper evaluates:
       - Did a gate condition fire? → prepare InformationRelease (MSG-SK02)
       - Did a constraint activate? → prepare ConstraintGuidance (MSG-SK03)
       - Did a goal complete? → update posterior node availability
       - Did a relationship shift? → update relational web
     World Agent evaluates:
       - Is the action physically possible? → constraint check
       - Does the world state change? → state update
     Character Agents (if addressed):
       - Respond through their psychological frame
       - Express intent to Narrator (via Reconciler if multi-character)

  5. NARRATIVE RENDERING
     Narrator receives all agent outputs
     → synthesizes into a single narrative response
     → renders the world's response to the player's action
     → signals significance through craft (description weight, atmosphere, pacing)

  6. INCREMENTAL UPDATES
     Frames updated incrementally (not recomputed from scratch)
     Entity states updated (promotion, decay adjustments, weight changes)
     Truth set updated with new facts
     Scene goal progress tracked
```

**Frame updates during play**: The full frame computation pipeline runs at scene entry. During the scene, events trigger incremental modifications — an axis shift from a trigger, an echo activation, a relational configuration change. Only if a sufficiently dramatic event occurs (a major revelation, an echo cascade) does the system recompute from scratch mid-scene.

### Exit

Scene exit occurs when conditions indicate the scene has reached its conclusion.

```
Exit Conditions:
  | GoalCompletion      — a primary goal's completion condition is satisfied
  | TrajectorySelection — the player takes an action that activates a departure trajectory
  | NarrativeExhaustion — the Storykeeper determines the scene has no remaining active potential
  | TemporalAdvance     — story-time conditions require a scene change (dawn arrives, etc.)
  | PlayerInitiated     — the player explicitly moves to leave
  | StorykeepersJudgment — the Storykeeper recognizes a dramatic beat that calls for transition

Exit Process:
  1. Storykeeper evaluates which posterior nodes are now unlocked
  2. Storykeeper selects the departure trajectory (or the player's explicit choice)
  3. State is checkpointed:
     - Updated tensor elements persisted
     - Relational web changes committed
     - Entity states saved (or dissolved for decaying entities)
     - Event ledger updated
     - Information ledger updated
  4. Character Agents are released
  5. Scene transition begins → Entry pipeline for next scene
```

**When a scene has no clean exit**: If no goals complete, no departure trajectories unlock, and the player hasn't chosen to leave, the scene is in a state of *narrative stasis*. This is the "unsuccessful engagement" problem. See below.

---

## Scene Types

### Authored Gravitational Scenes

High-mass scenes with rich authorial specification. The story's pivotal moments.

- Full cast specification (required + contingent + potential arrivals)
- Rich setting description with authored atmospheric detail
- Multiple primary and secondary goals
- Information gates, some structural (always fire)
- Strong departure trajectories with clear momentum
- High narrative mass → strong gravitational pull from surrounding scenes

Examples from TFATD: S1 (Tom Lies Still and Dying), S3 (A Mother's Prayer), S6 (The Other Bank).

### Authored Gate Scenes

Medium-mass scenes whose primary function is information delivery or threshold crossing.

- Focused cast (often just player + one key character)
- Information gates are the primary content
- Goals are often structural (the gate fires by being in the scene)
- Setting may be simpler but carries symbolic weight
- Often serve as the "door" between narrative zones

Examples: S2 (Speaking with the Gate) — Adam delivers critical information; the scene is defined by what is revealed.

### Connective Scenes

Low-to-medium mass scenes that provide texture, relationship-building, and travel between authored scenes.

```
ConnectiveScene {
  template: GenreTemplate,                  // dark_forest_travel, market_exploration, campfire_rest
  duration: ConnectiveDuration,             // brief (1-3 turns), standard (3-8), extended (8+)
  atmosphere_seed: AtmosphericProfile,      // generated from genre + current state
  interaction_opportunities: Vec<InteractionOpp>, // procedurally selected from available characters/entities
  narrative_drift_direction: Vec<Theme>,    // which themes the connective space subtly advances
  gravitational_influence: Vec<(SceneId, f32)>, // nearby high-mass scenes exerting pull
}
```

Connective scenes are the narrative's interstitial material. They are not fully authored — they are generated from genre templates, modified by current narrative state, and responsive to player action. But they are not empty. TFATD's connective material (walking with the Wolf through the mist, exhaustion, the sounds of the Wood) carries significant emotional and relational weight.

**Key design point from the narrative graph case study**: "Connective space is not 'low mass' — it is differently massed, with mass distributed across texture rather than concentrated in events." The player's experience of connective space should feel alive and responsive, even without authored plot beats. This is where the Narrator's craft and the World Agent's translation of the environment matter most.

### Threshold Scenes

Transitional scenes that mark a boundary crossing — moving between narrative zones, from safety to danger, from known to unknown.

Often brief. Often carry ritual or symbolic weight. Kate's farewell at the edge of the Wood is a threshold — it marks the irreversible departure from home. Thresholds may have a single structural gate ("you have now crossed into the Wood") and strong emotional resonance.

---

## The Ludic Contract

### On Being a Ludic Narrative

This system is not pretending to be a novel. It is not pretending to be a video game. It is a *ludic narrative* — a story that is also play, play that is also story — and it should lean into this nature rather than hiding it.

The ludic contract is the implicit agreement between the system and the player: *you are in a story, and the story is responsive to you, and there are directions that are more interesting than others, and we will help you find them without taking away your freedom to explore.*

This contract is expressed through several mechanisms, all operating within the fiction rather than breaking it.

### Narrator as Active Guide

The Narrator is not a passive describer waiting for the player to trigger events. The Narrator is an active participant in the ludic contract — a guide who shapes the player's attention through the craft of storytelling.

**Descriptive weight**: The Narrator gives more attention, more sensory detail, more lingering description to things that matter. The cup on the table in a scene about loss is described with care; the chair beside it may get a word. This asymmetry of attention is the primary signal to the player that something is worth engaging with.

**Atmospheric pressure**: The Narrator can shift the mood to signal that something is approaching. Tension building before a revelation. Warmth before a moment of connection. Unease before danger. The player feels the story's direction through tone before they see it through events.

**Responsive depth**: Actions that engage with the scene's rendered space get richer responses. Asking the Wolf about his nature produces something deep and strange. Asking the Wolf about the weather produces something brief and deflecting. The system is more eloquent where the story is more alive.

**Gentle prompting**: When the player seems uncertain, the Narrator can offer the world's invitations without dictating action. "The door is ajar. Beyond it, voices — one calm, one heated." This is not "go through the door." It is the world being interesting in a specific direction.

### Topographic Signaling

The player exists in a narrative space that has shape — and they should have some awareness of that shape. Not full knowledge of the narrative graph (that would destroy mystery and discovery), but a sense of *where they are* and *what is nearby*.

```
NarrativeTopography {
  current_position: TopographyNode,
  visible_landmarks: Vec<Landmark>,         // narrative "points of interest" the player is aware of
  felt_directions: Vec<FeltDirection>,      // vague senses of what lies in different directions
  narrative_momentum: MomentumIndicator,    // the story's current energy and direction
}

Landmark {
  description: String,                      // "the witch who lives at the borders"
  direction: DirectionHint,                 // vague spatial/narrative orientation
  source: LandmarkSource,                   // how the player learned about this
  gravitational_pull: f32,                  // how strongly the story pulls toward it
  mystery_level: f32,                       // how much the player doesn't know about it
}

FeltDirection {
  description: String,                      // "the Wood grows darker to the north"
  quality: DirectionQuality,                // inviting, foreboding, mysterious, urgent, quiet
  // NOT a named destination — a feeling about a direction
}
```

**Dynamic topography display**: The player might have access to a visual or textual representation of their narrative position — not a map in the geographic sense, but a *topographic sense* of the story's landscape. Points of interest they've learned about. A feeling for what lies in different directions. The current momentum of the narrative. This is not a quest log and not a to-do list. It is a *sense of place in the story*.

Think of it as the narrative equivalent of standing on a hill and looking around. You can see some landmarks. You have a feeling about which directions are interesting. You know where you came from. You don't know exactly what's ahead, but the landscape has shape that you can perceive.

**What this is not**: It is not a checklist of objectives. It is not a progress bar. It is not "3 of 7 quests completed." The system's relationship to the player is narrative, not transactional. The topographic display serves *orientation* — helping the player feel situated in a story with direction — not *optimization*.

### Narrative Position Awareness

The player should have a sense of where they are *narratively*, not just spatially:

- **Momentum**: Is the story accelerating toward something? Resting? Drifting? The Narrator communicates this through pacing and tone, but the topographic display can reinforce it — the felt sense that something important is near, or that the story is in a breathing space between intense moments.

- **Unresolved threads**: What questions remain open? What relationships are unsettled? What mysteries haven't been solved? These are not displayed as objectives but as *resonances* — the player's sense of what the story is *about* right now. "Something about the Wolf's contradictory behavior still pulls at you." "Kate's words about the water — you haven't fully understood them yet."

- **Thematic register**: What themes is the story currently exploring? The player doesn't see "Theme: Loss" — they feel it through the Narrator's tone, through the characters' behavior, through the atmospheric pressure. But the topographic display can reinforce thematic awareness at a meta level: "The story is in a register of departure and courage."

### Gravitational Expression

The attractor basin model from `narrative_graph.md` operates within scenes as well as between them. During play, the system expresses gravitational pull through narrative means:

**Character behavior**: NPCs can embody gravitational pull. Adam mentions the witch at the borders — not because the system is railroading, but because Adam *would* mention her; it's in his character. The Wolf's unease near water signals that water matters. Characters are the most natural vehicle for gravitational expression because they have their own reasons for saying what they say.

**Environmental signals**: The world responds to narrative gravity. As the player moves through connective space toward a high-mass scene, the environment shifts — the mist thickens, the path narrows, the sounds change. These atmospheric shifts are the World Agent translating gravitational pull into sensory experience.

**Narrative coincidence**: In a ludic narrative, coincidence is a legitimate tool — *within bounds*. The letter that arrives at the right moment, the stranger who mentions a name the player has been wondering about, the door that is now unlocked that was locked before. These are the Storykeeper allowing the narrative graph's gravity to manifest as events. The key constraint: coincidences should feel like the world being alive, not like a designer pulling strings. They should be consistent with the world's internal logic.

**Momentum and pacing**: The turn cycle itself can express gravitational pull. Scenes near high-mass gravitational scenes have tighter pacing — events move faster, consequences arrive sooner, the atmosphere intensifies. Scenes far from gravitational centers have looser pacing — more room for exploration, conversation, texture.

---

## Unsuccessful Engagement

### The Problem

What happens when the player doesn't engage with anything that advances the story? No goals complete. No gates fire. No relationships shift. No departure trajectories unlock. The scene sits in narrative stasis.

This is not necessarily the player's fault. They may be exploring in a direction the designer didn't anticipate. They may be unsure what to do. They may be testing the system's boundaries. They may be disengaged (the good faith problem from `open_questions.md`). The system's response should be proportional and narrative, not mechanical.

### Graduated Response

The system responds to narrative stasis through a graduated escalation:

**1. Narrative invitation (immediate):** The Narrator leans into the scene's affordances. Descriptions become more evocative. The world offers its invitations more clearly. "The cup sits on the table, its rim still stained. Outside, the voices have stopped — the silence is louder than the argument was." The system makes the interesting paths more visible without pointing at them.

**2. Character initiative (after sustained inaction):** Present characters act on their own goals. If Adam is in the scene, Adam speaks — not because the system needs the player to respond, but because Adam has things to say. Character initiative breaks stasis naturally and offers the player something to react to. A Character Agent with a goal ("convince Sarah to enter the Wood") will pursue that goal independently of the player's initiative.

**3. World pressure (after extended inaction):** The World Agent introduces environmental pressure. Time passes. The light changes. Tommy's breathing grows shallower. The mist closes in. These are not punishments — they are the world being alive, which includes the world not waiting for the player. Time pressure is a legitimate narrative tool.

**4. Gravitational escalation (if stasis persists):** The Storykeeper increases the gravitational pull of nearby high-mass scenes. Events conspire to move the story forward. A messenger arrives. A sound from outside demands investigation. The door, previously closed, swings open. This is the gentlest form of railroading — the story asserting its own momentum when the player provides none.

**5. Narrative contraction (if disengagement is persistent):** If the player consistently fails to engage across multiple scenes, the story world contracts. Fewer characters appear. Settings become simpler. The narrative offers less complexity but clearer hooks. This is not punishment — it is the system adapting to the player's level of engagement, preserving a functional experience even when the player is not meeting the story halfway. The world *notices* when it is not being met with care (per `open_questions.md`, Question 1).

### What "Unsuccessful" Means

A scene without goal completion is not automatically unsuccessful. A scene where the player spends ten turns talking to the Wolf about loneliness, with no plot advancement, no gates firing, no goals completing — but with the Sarah-Wolf relationship deepening — is a *highly successful* scene. The system must distinguish between:

- **Narrative progress without goal progress**: relationship-building, emotional development, world exploration, thematic engagement. These are tracked as accumulated state changes and contribute to future scene activation even if the current scene's authored goals don't fire.

- **Genuine stasis**: no state changes of any kind — no relationships shifting, no emotions moving, no information gained, no entities promoted. This is what triggers the graduated response.

The Storykeeper evaluates engagement quality, not just goal completion. A rich scene with no authored goals but deep relational engagement is a success. An empty scene with all goals completed mechanically is a hollow success. The system should care about both.

---

## Relationship to Existing Specifications

### Narrative Graph Integration

The scene model operationalizes the narrative graph. Each scene in the graph corresponds to a Scene as specified here. The graph defines the *between* (how scenes relate, what connects them, how gravity pulls); this document defines the *within* (what happens inside a scene during play).

**Scene types** from the narrative graph case study (gravitational, connective, gate, threshold) determine the scene's default anatomy — how much specification it has, what kinds of goals it carries, how it's generated.

**Approach vectors** become the scene's `active_approach_vector` — the specific path the player took, carrying specific emotional, informational, and relational context. The scene adapts to its approach vector through the warmed data (which themes are active, what the player feels, what they know).

**Departure trajectories** become the scene's posterior connections — exits that unlock as goals complete and gates fire.

### Agent Message Catalog Integration

The scene lifecycle maps directly onto the message catalog:

| Scene Phase | Key Messages |
|---|---|
| Entry | MSG-SK01 (SceneContext), MSG-SK04 (CharacterInstantiation) |
| Active Play — player input | MSG-P01 (PlayerAction), MSG-S01 (ProcessedPlayerAction) |
| Active Play — gate fires | MSG-SK02 (InformationRelease) |
| Active Play — constraint hit | MSG-SK03 (ConstraintGuidance) |
| Active Play — character response | MSG-CA01 (CharacterResponse), MSG-R01 (SequencedExpression) |
| Exit | MSG-S02 (TemporalAdvance), state checkpoint |

### Entity Model Integration

The entity budget per scene is a direct application of the Entity model's lifecycle. Entity promotion, decay, and dissolution all operate within the scene's context. Scene entry initializes the entity budget; scene exit resolves entity states (persist, decay, dissolve).

### Tensor Schema Integration

Frame computation at scene entry is the tensor schema's pipeline in action. The scene provides the `SceneContext` that the pipeline consumes. The scene's truth set is where trigger predicates are evaluated. The scene's events feed the incremental frame update process during active play.

---

## Open Considerations

1. **Connective scene generation depth**: How rich should procedurally generated connective scenes be? A template-based system risks feeling generic; a fully generative system risks inconsistency. The right balance probably involves genre-specific templates with narrative-state-dependent modification — but the specific generation pipeline needs design.

2. **Scene duration**: How long should a scene last in player turns? Gravitational scenes might run 10-30 turns; connective scenes might run 3-8. But these numbers should emerge from play-testing, not be prescribed. The exit condition system (goal completion, trajectory selection, Storykeeper judgment) should naturally produce appropriate durations.

3. **Multi-scene locations**: A location (the village, the forest) may host multiple scenes over time. How does the system handle returning to a place? The setting persists (with entity state changes), but the scene is new — different goals, different cast, different stakes. The warmed data should reflect the player's history with this place.

4. **Topographic display implementation**: The narrative topography concept needs concrete UI design. What does the player actually see? A textual summary? A visual map? An atmospheric overlay? This is a UX question with architectural implications — the system must produce the data that the display consumes.

5. **Scene nesting**: Can scenes contain sub-scenes? A conversation within a larger scene, a flashback triggered by an echo, a brief interaction that's part of a longer sequence. Nesting adds expressiveness but complicates the lifecycle model. It may be simpler to model these as state changes within a single scene rather than nested scenes.

6. **Player-initiated departure vs. story-initiated**: When the player says "I leave," the system must decide: is this a departure trajectory (move to next scene) or a rendered-space action (the player tries to leave but the world resists)? The answer depends on whether departure conditions are met and whether the story has reasons to hold the player. This is a Storykeeper judgment call.

7. **Graduated response calibration**: The unsuccessful engagement response system needs careful tuning. Too aggressive and the player feels railroaded. Too passive and they feel lost. The graduation rate should probably adapt to the player's established engagement pattern — a player who has been deeply engaged and suddenly goes quiet gets more patience than one who has been disengaged from the start.

8. **Connective scene mass distribution**: The narrative graph case study noted that connective space has "differently distributed mass." The scene model needs to operationalize this — how is mass distributed across turns in a connective scene? Is it uniform, or does it concentrate around specific interaction opportunities?
