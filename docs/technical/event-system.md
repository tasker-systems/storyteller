# Event System

## Purpose

This document specifies the event lifecycle — how narrative events are identified, classified, routed, and processed across the system's agents. It addresses the fundamental tension between the subjectivity of narrative experience (did the player's words constitute a betrayal?) and the system's need for discrete, processable events that enter a truth set and fire triggers.

The event system is the bridge between the flow of play (natural language, ambiguous, continuous) and the computational machinery (triggers, truth sets, tensor updates, agent coordination). Getting this bridge right is critical to both narrative quality and computational efficiency.

### Relationship to Other Documents

- **`tensor-schema-spec.md`** defines the event-driven architecture — NarrativeEvents, InterpretiveJudgments, TriggerSubscriptions, and the two processing paths (factual and interpretive). This document operationalizes that architecture into a runtime system.
- **`scene-model.md`** defines the turn cycle and scene lifecycle — when events are processed during play. This document specifies *how* they are processed within that lifecycle.
- **`agent-message-catalog.md`** defines the messages that carry event information between agents.
- **`entity-model.md`** defines entity promotion as an event-driven process.

---

## The Classification Problem

### Why This Is Hard

In a narrative system, determining *what happened* is not always straightforward. Some events are objective and immediately identifiable:

- "The player picked up the stone." (Action + target, deterministic)
- "Adam entered the room." (Character arrival, factual)
- "Sarah crossed the stream." (Physical action, World Agent validatable)

Others require judgment:

- "The player's words were cruel." (Tone interpretation)
- "Trust between Sarah and the Wolf eroded." (Relational inference)
- "The conversation shifted from warm to tense." (Emotional register detection)
- "The player's action constituted a rejection of Adam's offer." (Intent classification)

The first kind can be determined by pattern matching and rule engines in microseconds. The second kind requires LLM-level understanding of context, tone, and meaning — which is expensive and introduces latency.

But the trigger system (tensor-schema-spec.md, Decision 5) treats both as atoms in the truth set. A trigger predicate like `All(CharacterPresent(wolf), InterpretiveCondition("trust_erosion"))` requires both a cheap factual check and an expensive interpretive judgment. The system must handle this mismatch without blocking the turn cycle on expensive computation.

### The Two-Track Architecture

The system processes events on two parallel tracks with different speeds, different certainties, and different resolution timescales:

```
Track 1: FACTUAL (fast, deterministic, immediate)
  Input  → Classifier agents + World Agent
  Output → NarrativeEvent entries in truth set
  Speed  → milliseconds (rule-based pattern matching)
  Certainty → high (objective, validatable)
  Resolution → this turn

Track 2: INTERPRETIVE (slower, probabilistic, may lag)
  Input  → Scene context + recent events + character frames
  Output → InterpretiveJudgment entries in truth set
  Speed  → hundreds of milliseconds to seconds (LLM inference)
  Certainty → variable (confidence-weighted)
  Resolution → this turn (if fast enough) or next turn
```

Both tracks feed the same truth set. Both can fire the same triggers. But they operate on different timescales and with different epistemic status. The system is designed so that the turn cycle never *blocks* on Track 2 — if interpretive classification hasn't completed when the Narrator needs to render, the system proceeds with factual events and applies interpretive results when they arrive.

---

## Event Taxonomy

### Factual Events

Events that can be determined with certainty from the observable state of the world. The `NarrativeEvent` types from `tensor-schema-spec.md`:

```
NarrativeEvent =
  // Player actions (produced by classifier agents):
  | ActionPerformed { actor: EntityId, action: ActionType, target: Option<EntityId> }
  | SpeechAct { speaker: EntityId, addressee: Option<EntityId>, content_hash: ContentRef }
  | MovementAction { actor: EntityId, from: LocationRef, to: LocationRef }
  | ExaminationAction { actor: EntityId, target: EntityId }

  // System events (produced by agents):
  | SceneEntered { scene_id: SceneId, characters: Vec<EntityId> }
  | SceneExited { scene_id: SceneId, character: EntityId }
  | CharacterArrival { character: EntityId, scene_id: SceneId }
  | CharacterDeparture { character: EntityId, scene_id: SceneId }
  | InformationRevealed { gate_id: GateId, recipient: EntityId }
  | ConstraintViolation { actor: EntityId, action: ActionType, constraint: ConstraintRef }

  // Entity events (produced by World Agent / entity lifecycle):
  | EntityPromoted { entity: EntityId, to_tier: PersistenceTier }
  | EntityDemoted { entity: EntityId, from_tier: PersistenceTier }
  | EntityDissolved { entity: EntityId }
  | WorldStateChanged { key: WorldStateKey, old_value: Value, new_value: Value }

  // Relational events (produced by Storykeeper after confirmation):
  | RelationshipShifted { from: EntityId, to: EntityId, dimension: SubstrateDimension, delta: f32 }
  | GoalCompleted { goal_id: GoalId, scene_id: SceneId }
  | GateFired { gate_id: GateId, scene_id: SceneId }
  | ThresholdCrossed { entity: EntityId, threshold: ThresholdRef }
```

Factual events are objective. The World Agent can validate them. They enter the truth set immediately and trigger matching runs against them in real-time.

**Note on RelationshipShifted**: This is listed as factual because by the time it's published, the Storykeeper has confirmed the shift. But the *detection* of whether a shift occurred may require interpretive analysis (see below). The factual event is the confirmed result; the interpretive judgment is how the system determined that the shift happened.

### Interpretive Events

Events that require judgment about meaning, tone, intent, or significance. These are produced by the ML inference layer or by classifier agents running interpretive models:

```
InterpretiveJudgment {
  id: JudgmentId,
  timestamp: Timestamp,
  scene_id: SceneId,
  source: JudgmentSource,                  // which agent/model produced this
  category: JudgmentCategory,
  confidence: f32,                         // [0.0, 1.0]
  evidence: Vec<EventRef>,                 // which factual events support this judgment
  description: String,                     // human-readable explanation
}

JudgmentCategory =
  // Relational judgments:
  | TrustShift { between: (EntityId, EntityId), direction: ShiftDirection, magnitude: f32 }
  | AffectionShift { between: (EntityId, EntityId), direction: ShiftDirection, magnitude: f32 }
  | PowerDynamicShift { context: String }

  // Emotional judgments:
  | EmotionalRegisterShift { from: EmotionalRegister, to: EmotionalRegister }
  | EmotionExpressed { character: EntityId, emotion: EmotionType, intensity: f32 }
  | ToneDetected { quality: ToneQuality, in_speech_by: EntityId }

  // Thematic judgments:
  | ThematicResonance { theme: Theme, intensity: f32 }
  | EchoResonance { echo_pattern: EchoRef, character: EntityId }

  // Behavioral judgments:
  | IntentInferred { actor: EntityId, inferred_intent: String }
  | DeceptionDetected { actor: EntityId, confidence: f32 }
  | VulnerabilityExpressed { character: EntityId }

  // Scene-level judgments:
  | EngagementQuality { level: EngagementLevel }
  | NarrativeStasis { duration_turns: u32 }
  | DepartureSignificance { departure_type: DepartureType }

JudgmentSource =
  | ClassifierAgent { model: ModelRef, latency_ms: u32 }
  | FrameComputation { step: PipelineStep }
  | StorykeeperInference
  | NarratorObservation
```

Interpretive judgments enter the truth set with their confidence weight. Trigger predicates that involve interpretive conditions use the confidence in their evaluation — see "Confidence-Weighted Trigger Evaluation" below.

---

## The Truth Set

The truth set is the real-time state of "what is currently true" in the scene. It is the data structure against which trigger predicates are evaluated (tensor-schema-spec.md, Decision 5).

### Structure

```
TruthSet {
  // Factual propositions — binary, either true or not:
  factual: IndexedPropositionStore,

  // Interpretive propositions — true with a confidence weight:
  interpretive: WeightedPropositionStore,

  // Accumulated state — thresholds and counters:
  accumulators: HashMap<AccumulatorKey, f32>,

  // Temporal index — when propositions became true:
  temporal_index: BTreeMap<Timestamp, Vec<PropositionRef>>,
}
```

### How Events Enter the Truth Set

Events don't enter the truth set directly — they are translated into **propositions**. An event is something that happened; a proposition is something that is currently true. Some events create propositions that persist (a character is present until they leave). Others create propositions that are momentary (an action was performed, but "performing" is not an ongoing state).

```
Event → Proposition translation:

  ActionPerformed { actor: player, action: pick_up, target: stone }
  → Proposition: PlayerHolding(stone)          [persistent until dropped]
  → Proposition: ActionOccurred(pick_up, stone) [momentary, in temporal index]

  CharacterArrival { character: wolf, scene: s5 }
  → Proposition: CharacterPresent(wolf)         [persistent until departure]

  TrustShift { from: player, to: wolf, direction: negative, magnitude: 0.15 }
    (from InterpretiveJudgment, confidence: 0.72)
  → Proposition: TrustEroding(player, wolf)     [weighted: 0.72]
  → Accumulator: trust_delta(player, wolf) -= 0.15
```

### Confidence-Weighted Trigger Evaluation

When a trigger predicate includes interpretive atoms, the confidence weight affects evaluation:

```
// Simple case — factual atoms only:
All(CharacterPresent(wolf), SettingFeature(water))
→ binary: both present or not. Fast.

// Mixed case — factual + interpretive:
All(CharacterPresent(wolf), InterpretiveCondition(trust_erosion))
→ CharacterPresent(wolf): binary, true
→ trust_erosion: weighted, confidence 0.72
→ Predicate evaluates as: true with effective_confidence 0.72

// Threshold interaction:
Threshold(predicates: [A, B, C], min_match: 2)
  where A is factual (1.0), B is interpretive (0.72), C is interpretive (0.45)
→ Effective matches: 1.0 + 0.72 + 0.45 = 2.17 ≥ 2.0 → fires
→ But if C drops to 0.2: 1.0 + 0.72 + 0.2 = 1.92 < 2.0 → does not fire

// Confidence threshold:
// The system has a configurable minimum confidence (default 0.5) below which
// interpretive propositions are not considered "true enough" to participate
// in trigger evaluation. This prevents low-confidence noise from firing triggers.
```

---

## The Classification Pipeline

### Overview

The classification pipeline translates the raw flow of play into discrete events. It operates in stages, each producing events at a different level of abstraction:

```
Player Input (raw text)
    │
    ▼
┌─────────────────────────────────┐
│  Stage 1: STRUCTURAL PARSING    │  Fast, rule-based
│  Action type, target resolution │  ~10-50ms
│  Produces: ActionType, targets  │
└──────────────┬──────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Stage 2: FACTUAL CLASSIFICATION│  Fast, pattern matching
│  World Agent validation         │  ~10-50ms
│  Produces: NarrativeEvents      │  → enter truth set IMMEDIATELY
└──────────────┬──────────────────┘
    │                    ╲
    ▼                     ╲ (parallel)
┌─────────────────────────────────┐
│  Stage 3: SENSITIVITY MATCHING  │  Fast, pre-computed targets
│  Match against sensitivity map  │  ~20-100ms
│  Produces: provisional judgments│  → enter truth set as PROVISIONAL
└──────────────┬──────────────────┘
    │                    ╲
    ▼                     ╲ (async, may complete later)
┌─────────────────────────────────┐
│  Stage 4: DEEP INTERPRETATION   │  Slow, LLM-mediated
│  Full contextual analysis       │  ~500ms-2s
│  Produces: refined judgments    │  → REPLACE provisional entries
└─────────────────────────────────┘
```

### Stage 1: Structural Parsing

The first classifier agent performs fast structural analysis of the player's input:

```
Input: "I tell the Wolf that I don't trust his guidance anymore"

Output:
  action_type: Speech
  speaker: player
  addressee: wolf
  content_summary: "expressing distrust of Wolf's guidance"
  resolved_targets: [wolf]
  keywords: [trust, guidance, negation]
```

This is primarily pattern matching and entity resolution. It can be done by a small, fast model or even by rule-based NLP. The goal is to produce enough structure for the subsequent stages to work with.

### Stage 2: Factual Classification

The World Agent and system validate the action and produce factual events:

```
Input: Stage 1 output

Checks:
  - Is the player in a scene with the Wolf? → yes
  - Is speech physically possible here? → yes (no constraint)
  - Are there any hard constraints on this action? → no

Output:
  NarrativeEvent::SpeechAct {
    speaker: player,
    addressee: wolf,
    content_hash: ref_to_original_input
  }

  → enters truth set IMMEDIATELY
  → trigger matching runs: any subscriptions to SpeechAct(player, wolf)?
```

Stage 2 events are the ground truth. They represent what objectively happened — the player spoke to the Wolf. They say nothing about the *meaning* of the speech.

### Stage 3: Sensitivity Matching

This is the key mechanism for making interpretive classification fast enough for real-time play. At scene entry, when the warmed data is assembled, the system pre-computes a **sensitivity map** — a set of concrete patterns that, if detected, would satisfy currently active trigger subscriptions.

```
Sensitivity Map (computed at scene entry):

  Active subscription: Wolf's echo_pattern "trust_betrayal"
    trigger: InterpretiveCondition("trust erosion from companion")
    → Sensitivity entry: {
        pattern: "player expresses distrust, doubt, or rejection toward Wolf",
        keywords: [trust, doubt, distrust, reject, lie, deceive, mislead],
        target_subscription: wolf.echo_trust_betrayal,
        confidence_if_matched: 0.7  // baseline confidence for keyword match
      }

  Active subscription: Sarah's value "loyalty_to_family"
    trigger: InterpretiveCondition("family bond invoked")
    → Sensitivity entry: {
        pattern: "mention of Tom, Kate, John, home, family, brother",
        keywords: [Tom, Tommy, Kate, mother, father, John, home, family, brother],
        target_subscription: sarah.value_loyalty,
        confidence_if_matched: 0.8
      }
```

Sensitivity matching compares the Stage 1 output against the pre-computed map. This is essentially keyword/pattern matching against a known target set — much cheaper than open-ended LLM interpretation.

```
Input: Stage 1 output (keywords: [trust, guidance, negation])

Matching:
  → Matches sensitivity entry for Wolf's echo_trust_betrayal
    (keywords [trust] intersect, negation present)
  → Provisional judgment produced:
    InterpretiveJudgment {
      category: TrustShift(player, wolf, negative, 0.15),
      confidence: 0.7,  // from sensitivity entry baseline
      source: ClassifierAgent(sensitivity_matcher),
    }

  → enters truth set as PROVISIONAL
  → trigger matching runs with provisional confidence
```

**Why this works**: The sensitivity map constrains the classification problem from "understand everything about this input" to "does this input match any of these specific patterns?" This is a much easier problem. Most of the computational work happened at scene entry when the map was built — during play, it's fast pattern matching.

**Limitations**: Sensitivity matching catches the obvious cases — direct expressions of trust, clear mentions of family, explicit emotional language. It misses subtlety — a veiled insult, an implied threat, a kindness that masks manipulation. These require Stage 4.

### Stage 4: Deep Interpretation

Full LLM-mediated analysis of the interaction in context. This runs asynchronously — it starts when the turn begins but may not complete before the Narrator needs to render.

```
Input: Full turn context — player input, Stage 1 output, current scene state,
       character frames, recent conversation history

Output:
  InterpretiveJudgment {
    category: TrustShift(player, wolf, negative, 0.22),  // refined magnitude
    confidence: 0.85,  // higher confidence than sensitivity match
    evidence: [speech_act_ref, prior_trust_level, conversation_context],
    description: "Player directly challenged Wolf's guidance after three
                  days of following it without question. The shift is
                  significant because it breaks an established pattern of
                  deference. The Wolf's echo_trust_betrayal should fire."
    source: ClassifierAgent(deep_interpreter, latency_ms: 1200),
  }

  → REPLACES provisional entry in truth set
  → trigger matching re-runs with refined confidence
  → if new triggers fire, effects apply in next turn
```

**Timing**: Stage 4 targets completion within the current turn cycle when possible (~1-2 seconds). When the interaction is complex or ambiguous, it may complete after the Narrator has already rendered — in which case its results apply to the *next* turn. This one-turn lag is narratively acceptable: the Wolf's reaction to a cutting remark can manifest in his next response rather than requiring the current response to reflect the full interpretation.

**When Stage 4 changes the outcome**: If Stage 4 produces a significantly different judgment than Stage 3's provisional result (different direction, much higher or lower confidence), the system must reconcile:

- If the provisional judgment fired a trigger that the refined judgment would not have: the trigger effect is noted as "provisional, may be revised." The Storykeeper can dampen or reverse effects at scene boundary.
- If the refined judgment fires a trigger that the provisional did not: the effect applies in the next turn, naturally woven into the narrative.
- In practice, Stage 3 and Stage 4 agree on direction (positive/negative) in the vast majority of cases. They differ primarily on magnitude and confidence.

---

## Priority Tiers and Timing

### The Three Tiers

Events and their effects are processed at three different speeds, corresponding to how urgently they affect the current scene:

### Tier 1: Immediate (must resolve this turn)

Events that change what is currently possible or happening in the scene. The turn cycle cannot complete without resolving these.

```
Immediate events:
  - Constraint violations (World Agent: "you can't do that")
  - Safety boundaries (system-level: X-card, content limits)
  - Information gate triggers (Storykeeper: new information available)
  - Character arrival/departure (cast change)
  - Scene exit conditions met (departure trajectory unlocked)
  - Mandated narrative shifts (authored guarantees)

Processing: synchronous, within the turn cycle
Agents involved: World Agent, Storykeeper (always); Narrator (for rendering)
```

Almost all immediate events are **factual**. They are produced by Stage 2 classification and don't require interpretive analysis. The World Agent validates physical possibility; the Storykeeper checks gate conditions; the system enforces safety boundaries. These checks are fast because they operate on the truth set via set-theoretic predicate evaluation.

### Tier 2: Scene-Local (should resolve within the scene)

Events that affect the ongoing scene but don't need to resolve in the current turn. They can tolerate one-turn latency.

```
Scene-local events:
  - Relational shifts between present characters
  - Emotional state changes
  - Echo activations
  - Entity promotions (player interacted with something new)
  - Character Agent frame updates (incremental, not full recomputation)
  - Goal progress updates

Processing: asynchronous, may complete this turn or next
Agents involved: Storykeeper, Character Agents, World Agent (for entity lifecycle)
```

Scene-local events are a mix of factual and interpretive. A relational shift requires interpretive judgment (did trust erode?) before the factual event (RelationshipShifted) can be confirmed. The sensitivity map and deep interpretation stages handle this — provisional results let the system proceed, refined results adjust on the next turn.

**The one-turn lag in practice**: A player says something cutting to the Wolf. Stage 3 (sensitivity matching) provisionally identifies a trust shift. The Wolf's Character Agent receives a provisional frame update: "trust may have shifted." The Wolf's response this turn reflects subtle unease — a slight change in tone, a pause. Stage 4 confirms the shift with higher confidence, and the Wolf's next response fully reflects the erosion. From the player's perspective, the Wolf's reaction builds naturally across turns rather than snapping instantly. This is narratively *better* than instantaneous reaction.

### Tier 3: Deferred (scene-boundary batch processing)

Events whose effects propagate beyond the current scene or affect entities not currently active. These are collected during the scene and processed in batch at scene exit.

```
Deferred events:
  - Tensor updates to non-topsoil layers (sediment, bedrock shifts)
  - Relational shifts to characters NOT present in the scene
  - Narrative graph recalculation (position, available posterior nodes)
  - Off-screen character state propagation
  - Entity decay processing (for entities outside the scene)
  - Topographic display updates
  - World state changes with broad implications (economic shifts, seasonal change)
  - Social graph propagation at distance

Processing: batch, at scene boundary
Agents involved: Storykeeper (primary), World Agent, (no Character Agents — they've been released)
```

Deferred processing is where the system maintains eventual consistency. A major falling out between the player and a character has immediate effects on the relationship between those two (Tier 2), but its ripple effects — how other characters hear about it, how it shifts alliances, how it modifies the narrative graph's gravitational landscape — are computed at scene boundary.

**The exception**: Some deferred-class events need to be pulled forward to Tier 2 when their impact is narratively critical. A major betrayal that the Storykeeper recognizes as a narrative-graph-altering event gets promoted from deferred to scene-local. The Storykeeper makes this judgment: "this is too important to wait."

```
DeferredEvent {
  event: NarrativeEvent | InterpretiveJudgment,
  effects: Vec<DeferredEffect>,
  priority_override: Option<PromotionReason>,  // if non-None, process as scene-local
}

PromotionReason =
  | NarrativeGraphAltering        // changes available posterior nodes NOW
  | SafetyCritical                // player or agent wellbeing
  | CastChanging                  // a character not present needs to arrive
  | IrreversibleConsequence       // once deferred, cannot be undone correctly
```

---

## The Subscriber Model

### Agent Subscriptions

Agents don't receive all events — they subscribe to event patterns at specific priority tiers. The event bus routes events to subscribers based on pattern match and priority.

```
Subscription {
  subscriber: AgentRef,
  pattern: EventPattern,                    // what events this subscription matches
  priority_tier: PriorityTier,              // immediate, scene_local, or deferred
  filter: Option<FilterPredicate>,          // additional conditions for routing
  handler: HandlerRef,                      // what the agent does with matched events
}
```

### Default Agent Subscriptions

```
World Agent:
  - ActionPerformed(*)         → Tier 1 (validate physical possibility)
  - MovementAction(*)          → Tier 1 (validate traversal, update location)
  - EntityPromoted(*)          → Tier 2 (entity lifecycle management)
  - WorldStateChanged(*)       → Tier 2 (constraint recalculation)
  - EntityDissolved(*)         → Tier 3 (cleanup)

Storykeeper:
  - GateFired(*)               → Tier 1 (information release)
  - GoalCompleted(*)           → Tier 1 (posterior node evaluation)
  - ThresholdCrossed(*)        → Tier 1 (narrative graph update)
  - RelationshipShifted(*)     → Tier 2 (relational web update)
  - InterpretiveJudgment(*)    → Tier 2 (frame update evaluation)
  - SceneExited(*)             → Tier 1 (scene transition initiation)
  - DepartureSignificance(*)   → Tier 2 (consequence computation)
  - NarrativeStasis(*)         → Tier 2 (graduated response initiation)
  - ALL events                 → Tier 3 (ledger recording, graph recalculation)

Character Agents:
  - SpeechAct(addressee=self)  → Tier 2 (respond to being spoken to)
  - ActionPerformed(target=self or nearby) → Tier 2 (react to relevant actions)
  - EmotionExpressed(character=nearby) → Tier 2 (react to emotional cues)
  - InterpretiveJudgment(involving=self) → Tier 2 (frame adjustment)
  // Character Agents do NOT subscribe to Tier 1 — they respond through
  // the Agent Coordination step, after factual validation is complete.

Reconciler (when active):
  - CharacterAgent output (all) → Tier 2 (sequence and coordinate)
  // Active only in multi-character scenes

Narrator:
  // The Narrator does NOT subscribe to events directly.
  // It receives the synthesized output of Agent Coordination (Step 4 of turn cycle).
  // This is by design — the Narrator's world is what the Storykeeper provides.
```

### Why This Routing Matters

Consider a player attempting to fly in a world without flight:

1. `ActionPerformed(player, fly, nil)` → World Agent receives at Tier 1
2. World Agent determines: hard constraint violation
3. `ConstraintViolation(player, fly, no_flight)` → Storykeeper receives at Tier 1
4. Storykeeper prepares ConstraintGuidance (MSG-SK03) for Narrator
5. Narrator renders the constraint artfully

The event never reaches Character Agents because nothing happened narratively. The Storykeeper records the attempt in the ledger (Tier 3) but doesn't update any narrative state. No interpretive classification runs because there's nothing to interpret.

Contrast with a player saying something cruel to a character:

1. `SpeechAct(player, wolf, content_ref)` → enters truth set (Tier 1, factual)
2. Sensitivity matching identifies potential trust_shift (Tier 2, provisional)
3. Wolf's Character Agent receives the speech act (Tier 2) and responds through its frame
4. Deep interpretation confirms trust_shift (Tier 2, refines provisional)
5. Storykeeper confirms RelationshipShifted (Tier 2) and updates relational web
6. At scene boundary: propagation to distant social graph, tensor sediment updates (Tier 3)

Every agent that needs to know is notified at the appropriate speed. No agent receives information it doesn't need.

---

## The Sensitivity Map

### Construction

The sensitivity map is built at scene entry as part of the warmed data assembly (scene-model.md, Scene Entry Pipeline, Step 5). It pre-computes what the classification pipeline should be looking for.

```
SensitivityMap {
  entries: Vec<SensitivityEntry>,
  // Rebuilt at scene entry and when significant state changes occur mid-scene
}

SensitivityEntry {
  id: SensitivityId,
  source_subscription: SubscriptionRef,     // which trigger subscription this serves
  pattern: SensitivityPattern,              // what to look for
  baseline_confidence: f32,                 // confidence if pattern matches
  priority_tier: PriorityTier,             // how urgently to process a match
}

SensitivityPattern =
  | KeywordMatch {
      keywords: Vec<String>,               // words/phrases to detect
      context_modifiers: Vec<ContextModifier>, // negation, emphasis, etc.
      semantic_field: SemanticField,        // trust, family, danger, etc.
    }
  | ActionPattern {
      action_type: ActionType,
      target_filter: Option<EntityFilter>,
      context: String,                      // e.g., "physical contact with Wolf"
    }
  | RelationalPattern {
      between: (EntityFilter, EntityFilter),
      dimension: SubstrateDimension,
      direction: ShiftDirection,
    }
  | ThematicPattern {
      themes: Vec<Theme>,
      resonance_threshold: f32,
    }
```

### What Goes Into the Map

The sensitivity map is derived from:

1. **Active trigger subscriptions** on present characters' tensor elements. Each subscription with an interpretive condition generates one or more sensitivity entries.

2. **Active scene goals** with interpretive completion conditions. "The player expresses compassion" becomes a sensitivity entry.

3. **Active echo patterns** for present characters. Each echo's trigger conditions become sensitivity entries.

4. **Storykeeper's narrative awareness** — the Storykeeper knows what it's watching for (approaching attractor basins, potential gate conditions, relational shifts that would unlock posterior nodes) and adds sensitivity entries accordingly.

### Map Refresh

The sensitivity map is primarily static within a scene — built at entry, used throughout. But significant mid-scene events can trigger a **map refresh**:

- A new character arrives (their trigger subscriptions need sensitivity entries)
- An echo fires (the fired echo's entries are removed; new post-echo sensitivities may be added)
- A goal completes (its sensitivity entries are removed; new goals may activate)
- A major state change (the Storykeeper determines that the scene's active concerns have shifted)

Map refresh is cheaper than full scene entry — it modifies the map incrementally rather than rebuilding it.

---

## Scene-Boundary Batch Processing

### What Happens at Scene Exit

When a scene ends, the deferred event queue is processed. This is the eventual-consistency resolution step.

```
Scene-Boundary Pipeline:
  1. DEFERRED EVENT RESOLUTION
     Process all Tier 3 events collected during the scene:
     → tensor updates (sediment, bedrock layers — slow-moving changes)
     → relational web updates (distant characters, indirect effects)
     → entity decay processing (for all tracked entities)
     → narrative graph recalculation

  2. OFF-SCREEN PROPAGATION
     Characters not present in the scene have been living their lives:
     → Storykeeper computes off-screen state changes based on
        character goals, world events, and time elapsed
     → Character tensors updated for off-screen characters
     → Relational web edges updated for off-screen relationships

  3. SOCIAL GRAPH RIPPLE
     Events from the scene propagate through the social graph:
     → A betrayal between player and Wolf affects Wolf's relationships
        with other characters (who may hear about it, sense it, etc.)
     → Propagation attenuates with network distance
     → Propagation speed depends on information channels
        (direct witness > rumor > inference)

  4. NARRATIVE GRAPH UPDATE
     Storykeeper recalculates:
     → which posterior nodes are now reachable
     → how narrative mass has shifted (dynamic_adjustment based on player state)
     → whether any attractor basins have gained or lost pull
     → the topographic display data for the next scene

  5. WORLD STATE UPDATE
     World Agent processes accumulated world changes:
     → time passage effects (weather, season, decay)
     → economic/political ripple effects (if applicable)
     → setting state updates for revisitable settings

  6. CHECKPOINT
     Full state snapshot for:
     → save/restore
     → unwind point
     → debugging / replay
```

### Eventual Consistency Guarantees

The deferred processing system makes the following guarantees:

1. **No deferred event is lost.** Every event recorded during the scene is processed at scene boundary, even if the scene exits abruptly.

2. **Order is preserved.** Deferred events are processed in the order they occurred, so causal chains are maintained (event A caused event B; A is processed before B).

3. **Effects are bounded.** A single deferred event cannot cascade indefinitely. The propagation system has a maximum depth (configurable, default 3) — an event can trigger a secondary effect, which can trigger a tertiary effect, but the chain stops there. This prevents runaway cascades from a single interaction.

4. **The next scene is consistent.** When scene entry begins for the next scene, all deferred effects from the previous scene have been resolved. The warmed data for the next scene reflects the full state of the world, not a partially-processed version.

---

## Workflow Orchestration: Tasker Core Integration

### Architectural Intent

The storyteller engine's unique value is the narrative system — agents, tensors, scenes, gravitational landscapes, the ludic contract. Workflow orchestration, job scheduling, retry logic, and distributed task management are not unique to storytelling. Rather than reinvent these capabilities, the system delegates non-real-time event processing to **Tasker Core** (`tasker-systems/tasker-core`), a Rust-based distributed workflow engine already proven in production.

This section identifies the natural integration points. It is architectural intent, not a detailed technical specification — the integration design will be refined during implementation.

### What Stays In-Process

Everything on the critical path of the turn cycle stays within the storyteller engine:

- **Tier 1 processing** (constraint validation, gate checks, mandates) — synchronous, must complete within the turn
- **Tier 2 processing** (sensitivity matching, provisional judgments, Character Agent coordination) — asynchronous within the turn, but latency-sensitive
- **Truth set management** — the source of truth for trigger evaluation, owned by the engine
- **Agent coordination and narrative rendering** — the core LLM orchestration that produces the player experience

These are real-time operations where even tens of milliseconds of overhead from external dispatch would degrade the experience.

### What Delegates to Tasker Core

Three categories of work naturally fit Tasker Core's workflow model:

**1. Scene-Boundary Batch Processing**

The six-step pipeline at scene exit (deferred event resolution → off-screen propagation → social graph ripple → narrative graph update → world state update → checkpoint) is a DAG of dependent tasks. Tasker Core's workflow step system models this directly:

```
Workflow: scene_boundary_processing
  Step 1: deferred_event_resolution     (depends on: nothing)
  Step 2: offscreen_propagation         (depends on: step 1)
  Step 3: social_graph_ripple           (depends on: step 1)
  Step 4: narrative_graph_update        (depends on: steps 2, 3)
  Step 5: world_state_update            (depends on: step 1)
  Step 6: checkpoint                    (depends on: steps 4, 5)
```

Steps 2, 3, and 5 can execute in parallel. Tasker Core handles the dependency resolution, retry logic, and state tracking. If step 3 (social graph ripple) fails or times out, the workflow can retry it without re-running steps 1 or 2.

**2. Deep Interpretation (Stage 4)**

When Stage 4 classification cannot complete within the turn cycle (~2s budget), it can be dispatched as a Tasker Core task. The result routes back via event notification and enters the truth set on arrival. This is particularly useful for:

- Complex multi-character interactions requiring extensive context analysis
- Thematic resonance detection across the full conversation history
- Cases where provisional sensitivity matching is sufficient for the current turn and deep interpretation can refine it asynchronously

**3. Cross-Session Processing**

Operations that span beyond a single play session:

- Long-running narrative graph analysis (rebalancing attractor basins based on aggregate player behavior)
- Training data generation from play session logs (combinatorial scenario skeletons, tensor-schema-spec.md Decision 4)
- Entity decay processing for dormant entities across inactive sessions
- Periodic world state evolution (seasons change, economies shift, off-screen characters pursue goals over elapsed real time)

### Messaging

Tasker Core supports both PGMQ (PostgreSQL-based, default) and RabbitMQ. For storyteller integration, **RabbitMQ is preferred** — its lower per-message latency (saving tens of milliseconds across multi-step workflows) compounds meaningfully when scene-boundary processing involves cascading dependent tasks. The latency difference is modest for individual messages but adds up across the six-step pipeline and any spawned sub-workflows.

### Integration Boundary

The boundary between in-process and delegated processing is defined by a single principle: **if the player is waiting, it stays in-process; if the player is not waiting, it delegates.** During a turn, the player is waiting — everything is in-process. Between scenes, the player may see a brief transition — batch processing delegates to Tasker Core. Between sessions, there is no player — background processing runs entirely through Tasker Core.

This keeps the storyteller engine focused on what only it can do: the narrative experience. Everything else leverages existing infrastructure.

---

## The Cascade Problem

### Bounded Cascades

A single event can trigger a chain of effects: an action fires a trigger, the trigger shifts a relationship, the shift crosses a threshold, the threshold fires an echo. This cascading is by design — it's how the system produces emergent narrative dynamics. But unbounded cascades are a computational and narrative problem.

```
Cascade Limits:
  - Maximum cascade depth: 4 (configurable per story)
  - Maximum events per cascade: 16 (configurable)
  - Cascade timeout: 500ms for Tier 1, 2000ms for Tier 2
  - If limits are exceeded: remaining effects are deferred to scene boundary

Cascade ordering:
  1. Factual effects resolve first (truth set updates)
  2. Trigger matching runs against updated truth set
  3. Newly fired triggers produce effects
  4. Repeat from step 1, incrementing depth counter
  5. Stop when depth limit reached or no new triggers fire
```

### Cascade Priority

When a cascade produces effects at different priority tiers, the higher tier wins:

- An echo activation (Tier 2) triggers a narrative mandate (Tier 1): the mandate is processed immediately, even though the echo was scene-local.
- A relational shift (Tier 2) triggers a distant social graph effect (Tier 3): the distant effect is deferred, even though its cause was scene-local.

### Preventing Oscillation

The system guards against trigger loops — where event A fires trigger X, which produces event B, which fires trigger Y, which produces event A again:

- **One-shot subscriptions** fire once and are removed. They cannot loop.
- **Ongoing subscriptions** have a refractory period — after firing, they cannot fire again for N turns (configurable, default 1). This prevents the same trigger from firing repeatedly in the same cascade.
- **Cycle detection**: The cascade processor tracks which (event, subscription) pairs have already fired in the current cascade. If a pair recurs, it is suppressed.

---

## Integration with the Turn Cycle

The event system operates within the turn cycle defined in `scene-model.md`. Here is the expanded turn cycle showing where each stage of event processing runs:

```
Turn Cycle (expanded with event system):

  1. PLAYER INPUT
     Player enters text
     → broadcast to all agents in parallel

  2. STRUCTURAL PARSING (Stage 1)                    ~10-50ms
     Classifier agent: action type, targets, keywords
     → output available for all subsequent stages

  3. FACTUAL CLASSIFICATION (Stage 2)                 ~10-50ms
     World Agent: physical validation
     → NarrativeEvents enter truth set IMMEDIATELY
     → Tier 1 trigger matching runs
     → Immediate effects resolved (constraints, gates, mandates)

  4. SENSITIVITY MATCHING (Stage 3)                   ~20-100ms
     Classifier agent: match against sensitivity map
     → Provisional InterpretiveJudgments enter truth set
     → Tier 2 trigger matching runs with provisional confidence
     → Scene-local effects begin processing

  5. AGENT COORDINATION                               ~500ms-2s (LLM calls)
     Storykeeper: evaluates gates, goals, constraints
     Character Agents: respond through (provisionally updated) frames
     World Agent: state updates
     Reconciler: coordinates multi-character responses
     → DEEP INTERPRETATION (Stage 4) runs IN PARALLEL with Agent Coordination

  6. NARRATIVE RENDERING                              ~500ms-2s (LLM call)
     Narrator synthesizes all agent outputs into narrative response
     → Player sees the result

  7. POST-TURN RECONCILIATION                         ~50-200ms
     If Stage 4 completed: replace provisional judgments with refined ones
     → Re-run Tier 2 trigger matching with refined confidence
     → Any newly fired triggers queued for next turn
     If Stage 4 still running: provisional results stand for now
     Tier 3 events queued for scene-boundary processing

  TOTAL TURN LATENCY TARGET: 1-4 seconds
    (dominated by LLM calls in steps 5 and 6)
```

**Key architectural decision**: Stages 1-4 of classification run *before* or *in parallel with* the expensive LLM calls in Agent Coordination. The factual truth set is fully updated and Tier 1 effects are resolved before any LLM agent receives the turn's context. This means Character Agents always have a consistent factual picture of what happened, even if interpretive nuance is still being refined.

---

## Open Considerations

1. **Classifier agent architecture**: The sensitivity matching stage (Stage 3) can potentially be implemented without LLM inference — as keyword/pattern matching against the pre-computed map. But the structural parsing stage (Stage 1) likely needs at least a small, fast model for entity resolution and action type classification. The boundary between rule-based and model-based classification needs calibration.

2. **Sensitivity map coverage**: The sensitivity map catches obvious cases but misses subtlety. How much coverage is "enough"? If 80% of interpretive events are caught by sensitivity matching and 20% require deep interpretation with one-turn lag, is that acceptable? Play-testing will determine the threshold.

3. **Provisional judgment accuracy**: If Stage 3 produces a provisional judgment that is later contradicted by Stage 4, the system has acted on incorrect information for one turn. How often does this happen, and how bad is it when it does? The dampening mechanism (Storykeeper can reverse provisional effects) provides a safety net, but the frequency of contradiction needs measurement.

4. **Truth set data structure**: The truth set needs to support fast predicate evaluation — essentially, "given this set of propositions, does this trigger predicate match?" For small truth sets (a scene with a few characters and a dozen active propositions), almost any data structure works. For larger sets, an indexed store or lightweight query engine may be needed. Bitset representations are possible if propositions can be mapped to a fixed vocabulary.

5. **Cross-scene event continuity**: When a scene exits and the next scene begins, the truth set is rebuilt for the new scene. But some propositions carry across (the player still holds the stone, the relationship is still strained). The scene-boundary pipeline must determine which propositions persist and which are scene-specific.

6. **Event ledger growth**: The event ledger records all factual events and interpretive judgments for the entire play session. Over a long session, this grows. The system needs a strategy for ledger management — archiving old events, summarizing distant history, maintaining queryability for temporal predicates like `After(event)` and `Since(event)`.

7. **Sensitivity map generation cost**: Building the sensitivity map at scene entry requires analyzing all active trigger subscriptions and translating their conditions into matchable patterns. For scenes with many present characters (each with many subscriptions), this could be expensive. The cost should be measured and, if necessary, bounded by limiting subscription activation to scene-relevant subsets.

8. **The ambiguity threshold**: When Stage 4 (deep interpretation) produces a judgment with low confidence (0.3-0.5), the system faces a choice: include it in the truth set (where it may influence trigger evaluation weakly) or discard it. The minimum confidence threshold (default 0.5) needs calibration — too high and subtle dynamics are missed; too low and noise triggers false events.
