# Agent Message Type Catalog

> **Architectural note (Feb 2026):** This document was written for the original multi-agent architecture where Character Agents, the Reconciler, and the World Agent were all LLM-based agents exchanging messages. The **narrator-centric pivot** (`narrator-architecture.md`, Feb 7 2026) replaced this model:
>
> - **Character Agents** are now ML prediction models (ONNX inference), not LLM agents
> - **The Reconciler** is now a deterministic rules engine resolver, not an LLM coordinator
> - **The World Agent** role is absorbed into the resolver and Storykeeper logic
> - **Only the Narrator** makes LLM calls (one per turn)
>
> **What remains valid:** Message format design principles, token budget thinking, information boundary concepts, the Narrator ↔ Player protocol (MSG-N01/N02), and the Player → System messages (MSG-P01/P02). The Storykeeper → Narrator messages (MSG-SK01 through SK04) map directly to the three-tier context assembly system.
>
> **What is superseded:** Sections 4-6 (CharacterAgent and Reconciler messages), the multi-agent turn cycle, and any reference to serial LLM calls beyond the single Narrator call. See section-level notes below.
>
> See `narrator-architecture.md` and `turn-cycle-architecture.md` for the current architecture.

## Purpose

This document enumerates every message type exchanged between agents in the storyteller system, derived from the information flow diagram in `system_architecture.md:186-244`. For each message type: sender, receiver, content structure, when sent, and information boundary constraints.

This catalog is a prerequisite for:
- The agent communication protocol specification (Track D)
- Context construction templates for each agent role (Track F)
- The orchestration layer implementation (Stream 4)

---

## Design Decisions Made During This Catalog

1. **Messages are hybrid**: structured metadata (JSON-serializable) + natural language content. The metadata enables routing, filtering, and state tracking. The natural language content is what agents actually reason about.

2. **Every message carries a context budget**: the maximum token count for the natural language portion. This is enforced by the orchestration layer, not the agents.

3. **Messages are typed, not free-form**: the system does not pass arbitrary text between agents. Each message type has a defined schema. This enables the orchestration layer to validate messages and the Storykeeper to enforce information boundaries.

4. **The information boundary is the most critical design element**: each message type specifies what the sender is *permitted* to include and what the receiver is *not allowed* to learn from it. The Storykeeper enforces these boundaries by constructing messages for downstream agents rather than allowing agents to communicate directly.

5. **Turn structure**: A single player turn triggers a cascade of messages. The full cascade for a standard scene with one character present:
   ```
   Player → System → Storykeeper → CharacterAgent → Narrator → Player
                  ↕                        ↕
              WorldAgent              Storykeeper (validation)
   ```
   For multi-character scenes, add Reconciler between CharacterAgents and Narrator.

---

## Message Type Inventory

### Overview by Flow Direction

| Direction | Count | Purpose |
|-----------|-------|---------|
| Player → System | 2 | Player input, meta-commands |
| System → Storykeeper | 3 | Processed input, state queries, temporal advance |
| Storykeeper → Narrator | 4 | Scene context, information release, constraint guidance, meta-direction |
| Storykeeper → CharacterAgent | 2 | Character instantiation, scene update |
| CharacterAgent → Storykeeper | 2 | Character response, state change request |
| CharacterAgent → Reconciler | 1 | Character expression |
| Reconciler → Narrator | 1 | Sequenced scene expression |
| Storykeeper ↔ WorldAgent | 4 | State queries, constraint checks, world updates |
| Narrator → Player | 2 | Narrative output, meta-information |
| System → Storykeeper (feedback) | 2 | Player action results, Narrator response record |

---

## Detailed Message Specifications

### 1. Player → System

#### MSG-P01: PlayerAction

The player's raw input — what they say, do, ask, or attempt.

```yaml
type: PlayerAction
sender: Player
receiver: InputProcessor (pre-Storykeeper)
when: Every player turn

content:
  raw_text: string              # The player's input as typed/spoken
  action_type: enum             # inferred by input processor:
    - speech                    # "I say to the Wolf..."
    - action                    # "I cross the stream"
    - examination               # "I look at the door more closely"
    - question                  # "What do I remember about this place?"
    - meta                      # System commands (save, settings, etc.)
  inferred_targets: [entity_id] # Characters, objects, locations mentioned
  inferred_intent: string       # Brief classifier output (optional, aids routing)

token_budget: N/A (player input is raw)

information_boundary:
  The player has no information boundary — they can attempt anything.
  The system's job is to determine what *happens* in response.
```

#### MSG-P02: PlayerMetaCommand

Out-of-fiction system commands.

```yaml
type: PlayerMetaCommand
sender: Player
receiver: System
when: Player issues a meta-command (save, X-card, settings, unwind)

content:
  command: enum
    - save
    - x_card                    # Safety signal — immediate scene transition
    - unwind                    # Request to roll back
    - settings                  # Configuration
    - quit
  parameters: map               # Command-specific parameters

token_budget: N/A

information_boundary: None — meta-commands bypass narrative boundaries.
```

---

### 2. System → Storykeeper

#### MSG-S01: ProcessedPlayerAction

Player action after input processing, with entity resolution and classification.

```yaml
type: ProcessedPlayerAction
sender: InputProcessor
receiver: Storykeeper
when: Every player turn (except meta-commands)

content:
  action_summary: string        # Cleaned, classified action description
  action_type: enum             # (from MSG-P01)
  resolved_targets:
    - entity_id: string
      entity_type: enum         # character, object, location, concept
      confidence: float
  scene_context:
    current_scene_id: string
    present_characters: [character_id]
    available_objects: [object_id]
  player_state_snapshot:
    emotional_impression: string  # What the player-character appears to feel
    information_state_hash: string # For quick comparison
    physical_state: map

token_budget: ~500 tokens

information_boundary:
  This message carries NO information the player doesn't have.
  It is a structured representation of the player's own action.
```

#### MSG-S02: TemporalAdvance

Notification that narrative time is advancing (designer-triggered or cyclical).

```yaml
type: TemporalAdvance
sender: System/Storykeeper (self-triggered)
receiver: Storykeeper, WorldAgent
when: Scene transitions, story-time passage, designer-triggered events

content:
  previous_time: timestamp
  new_time: timestamp
  advance_type: enum
    - scene_transition          # Moving between scenes
    - passage                   # "Three days later..."
    - cyclical                  # Day-night, seasonal
  narrative_context: string     # Why time is advancing

token_budget: ~200 tokens
```

#### MSG-S03: StateQuery

Request for current state of a specific entity or relationship.

```yaml
type: StateQuery
sender: Any agent (via Storykeeper)
receiver: Storykeeper
when: As needed during turn processing

content:
  query_type: enum
    - character_tensor
    - relationship_state
    - information_state
    - scene_properties
    - narrative_position
  target: string                # entity_id or relationship pair
  context: string               # Why this query is being made
  requester: agent_id
  requester_perspective: bool   # If true, filter result through requester's information boundary

token_budget: ~200 tokens
```

---

### 3. Storykeeper → Narrator

#### MSG-SK01: SceneContext

The primary message the Narrator receives to render a scene. This is the Narrator's *world* — it knows nothing beyond what this message contains.

```yaml
type: SceneContext
sender: Storykeeper
receiver: Narrator
when: Scene initialization and significant scene state changes

content:
  scene_id: string
  scene_description: string     # What the Narrator is allowed to describe
  tonal_signature: string       # How to describe it (elegiac, tense, warm, etc.)
  thematic_register: [string]   # Active themes
  present_characters:
    - character_id: string
      visible_state: string     # What the Narrator can observe about them
      relationship_to_player: string # How they relate to the player-character
  available_interactions: [string]  # What the player can do here
  atmospheric_notes: string     # Sensory details, mood, weather, ambient sound
  narrator_guidance: string     # Specific instructions for this scene
    # e.g., "Signal that the cup on the table is significant through
    #        descriptive weight, but do not explain why."

token_budget: 800-1200 tokens

information_boundary:
  CRITICAL: The Narrator does NOT receive:
  - The narrative graph (no knowledge of future scenes or possibilities)
  - Character tensors (only outward-facing descriptions)
  - Information the Storykeeper has not released
  - Other characters' private thoughts or motivations
  - The player's internal state (only what can be observed)

  The Narrator CAN receive:
  - Anything the player-character could perceive in the scene
  - Atmospheric and sensory detail the designer has authored or permitted
  - Guidance about significance signaling (without revealing the significance)
```

#### MSG-SK02: InformationRelease

Tells the Narrator that a gate has opened — new information may now be communicated.

```yaml
type: InformationRelease
sender: Storykeeper
receiver: Narrator
when: Player action satisfies a gate condition

content:
  gate_id: string
  released_information: string  # What the Narrator may now reveal
  delivery_guidance: string     # How to reveal it:
    - "Through character dialogue" — a character says it
    - "Through observation" — the player notices something
    - "Through atmospheric shift" — the mood changes
    - "Through direct narration" — the Narrator tells
  urgency: enum
    - immediate                 # Reveal now
    - when_natural              # Reveal when it fits the flow
    - if_prompted               # Only if the player asks or looks
  related_gates: [gate_id]      # Gates that this release may prime

token_budget: 300-500 tokens
```

#### MSG-SK03: ConstraintGuidance

Tells the Narrator how to handle an impossible or constrained player action.

```yaml
type: ConstraintGuidance
sender: Storykeeper (with WorldAgent input)
receiver: Narrator
when: Player attempts something that encounters a constraint

content:
  constraint_type: enum
    - hard                      # World physics violation — cannot happen
    - soft                      # Character capacity — difficult or costly
    - perceptual                # Cannot perceive this information
  player_action: string         # What the player tried
  constraint_reason: string     # Why it's constrained (for Narrator's understanding)
  delivery_guidance: string     # How to communicate:
    - "Render the attempt and its failure artfully"
    - "Show what the world offers instead"
    - "Let the character's limitations be felt through the attempt"
  alternative_suggestions: [string]  # What the player could do instead

token_budget: 400-600 tokens

information_boundary:
  The Narrator receives ENOUGH to render the constraint artfully.
  It does NOT receive the underlying mechanics (mass values,
  tensor numbers, gate conditions).
```

#### MSG-SK04: MetaDirection

High-level narrative direction from the Storykeeper — not scene-specific but session-level guidance.

```yaml
type: MetaDirection
sender: Storykeeper
receiver: Narrator
when: Session start, tone shifts, pacing adjustments

content:
  direction_type: enum
    - pacing                    # "Slow down — let the player breathe"
    - tone_shift                # "The tone is about to darken"
    - significance_signal       # "The next few interactions are building toward something"
    - player_state_awareness    # "The player seems disengaged — increase atmospheric richness"
    - safety_response           # "Gently close this scene and transition"
  guidance: string

token_budget: 200-300 tokens
```

---

### 4. Storykeeper → CharacterAgent

> **Superseded:** In the narrator-centric architecture, Character Agents are ML prediction models, not LLM agents. There is no CharacterInstantiation message — instead, the `CharacterPredictor` loads ONNX models and predicts character behaviors from tensor data. The information boundary concepts (what a character knows vs. doesn't know) remain valid as constraints on the prediction pipeline's input features. See `storyteller-engine/src/inference/predictor.rs`.

#### MSG-SK05: CharacterInstantiation

Creates a Character Agent for a scene — the most complex message in the system.

```yaml
type: CharacterInstantiation
sender: Storykeeper
receiver: CharacterAgent (new instance)
when: Character enters a scene

content:
  character_id: string
  character_name: string

  activated_tensor:             # Context-dependent activation output
    personality_axes: [axis_spec]   # Only activated axes
    motivations: map                # Active motivations with weights
    values: [value_spec]            # Relevant values
    current_emotional_state: map    # Topsoil layer
    active_echoes: [echo_spec]      # Any echoes currently firing

  relational_context:
    - target_id: string
      target_name: string
      relationship_summary: string  # From THIS character's perspective
      key_dimensions: map           # trust, affection, power, etc.

  scene_context:
    setting: string                 # Where this is happening
    other_presences: [string]       # Who else is here (names, visible states)
    recent_events: string           # What just happened
    what_character_knows: string    # Information this character has access to

  expression_guidance: string       # How this character communicates:
    # "Speak plainly and directly. Short sentences. You are twelve
    #  and not given to eloquence but your words carry weight."
    # OR
    # "Speak rarely. One or two words. Growl. You are not built for conversation."

  constraints:
    what_character_cannot_know: [string]   # Explicit negatives
    what_character_would_never_do: [string] # Bedrock violations

token_budget: 800-1500 tokens (varies by character complexity)

information_boundary:
  CRITICAL: The Character Agent does NOT receive:
  - The narrative graph or any knowledge of story structure
  - Other characters' private tensor data
  - Information the Storykeeper has not released to this character
  - Any awareness that they are in a story
  - Player meta-state or system state
```

#### MSG-SK06: SceneUpdate

Updates a Character Agent mid-scene when something changes.

```yaml
type: SceneUpdate
sender: Storykeeper
receiver: CharacterAgent (existing instance)
when: Something in the scene changes that the character would notice

content:
  update_type: enum
    - new_information            # Something was said or revealed
    - emotional_shift            # The scene's tone has changed
    - new_presence               # Someone entered or left
    - physical_change            # Something happened in the environment
    - player_action_result       # What the player just did
  update_content: string
  relevance_note: string         # Why this matters to this character

token_budget: 200-400 tokens
```

---

### 5. CharacterAgent → System

> **Superseded:** Character output is now produced by ML prediction models (`CharacterPredictor`), not LLM agents. The prediction output is structured data (action type, confidence, description) rather than free-form natural language. The concept of separating character *intent* from narrative *rendering* remains central — ML predictions are intent; the Narrator renders them as prose. See `storyteller-engine/src/context/prediction.rs`.

#### MSG-CA01: CharacterExpression

The Character Agent's output — what the character says, does, and feels.

```yaml
type: CharacterExpression
sender: CharacterAgent
receiver: Reconciler (multi-character) OR Narrator (single character)
when: In response to SceneUpdate or turn cycle

content:
  character_id: string
  expression_type: enum
    - dialogue                   # Words spoken
    - action                     # Physical action taken
    - reaction                   # Emotional or physical response
    - internal                   # What the character feels but does not show
    - intent                     # What the character intends to do next
  expression_content: string     # Natural language description
    # Example: "[Sarah clenches her fists. She wants to ask the Wolf
    #  why he won't answer her properly but she is afraid that the
    #  answer would be worse than the silence. She says: 'Come on, then.'
    #  And she walks forward, not looking back.]"
  emotional_state_update: map    # How the character's topsoil has shifted
  urgency: float                 # How strongly the character needs to act NOW

token_budget: 300-600 tokens

information_boundary:
  The CharacterExpression may contain internal thoughts that the character
  would not reveal. These are sent for the Storykeeper's records (tensor
  update) and the Narrator's rendering (showing vs. telling). The Narrator
  decides how much of the internal state to make visible through description.
```

#### MSG-CA02: StateChangeRequest

The Character Agent requests a change to the world or narrative state.

```yaml
type: StateChangeRequest
sender: CharacterAgent
receiver: Storykeeper
when: Character attempts an action that changes state

content:
  character_id: string
  requested_change: string      # What the character is trying to do
  change_domain: enum
    - physical                   # Moving, building, breaking, taking
    - relational                 # Changing how they relate to someone
    - informational              # Sharing or withholding information
  justification: string          # Why this is in-character

token_budget: 200-300 tokens

note: "The Storykeeper validates this against constraints and either
       applies the change or redirects through ConstraintGuidance."
```

---

### 6. Reconciler Messages

> **Superseded:** The Reconciler is now a deterministic rules engine (`ResolverOutput` in `storyteller-core/src/types/resolver.rs`), not an LLM agent. It sequences character actions by initiative order, resolves conflicts via graduated success outcomes (FullSuccess/PartialSuccess/FailureWithConsequence/FailureWithOpportunity), and produces structured data for the Narrator's context assembly. The concept of structuring multi-character interactions without adding content remains valid.

#### MSG-R01: CharacterExpressionBatch

The Reconciler receives all character expressions for a turn and produces a sequenced scene.

```yaml
type: ReconcilerInput
sender: Orchestration layer (collects MSG-CA01 from all active CharacterAgents)
receiver: Reconciler
when: Multi-character scene, after all CharacterAgents have responded

content:
  scene_id: string
  scene_context: string          # Current scene state
  character_expressions: [MSG-CA01]  # All character outputs for this turn
  player_action: string          # What the player did that prompted these responses
  scene_dynamics_note: string    # Storykeeper's guidance on dramatic potential

token_budget: sum of character expressions + 300 tokens overhead
```

#### MSG-R02: SequencedScene

The Reconciler's output — the ordered, resolved multi-character scene.

```yaml
type: SequencedScene
sender: Reconciler
receiver: Narrator
when: After processing ReconcilerInput

content:
  sequence: [event]             # Ordered list of events in the turn
    - event_type: enum          # dialogue, action, reaction, beat (pause)
      actor: character_id
      content: string
      dramatic_note: string     # "ironic", "tense", "interrupted", "simultaneous"
  unresolved_conflicts: [string] # Actions that need resolution next turn
  pacing_guidance: string        # "Let this breathe" or "Pick up the pace"
  dramatic_potential: string     # What the Reconciler noticed:
    # "Sarah and the Wolf are both looking at the door but for
    #  opposite reasons — she wants to enter, he wants to pass by.
    #  The tension is productive."

token_budget: 400-800 tokens

information_boundary:
  The Reconciler does NOT add content — it structures what CharacterAgents
  produced. It may identify dramatic potential but does not invent events
  or dialogue. It does not know the narrative graph.
```

---

### 7. Storykeeper ↔ WorldAgent

#### MSG-W01: WorldStateQuery

```yaml
type: WorldStateQuery
sender: Storykeeper
receiver: WorldAgent
when: Storykeeper needs physical/material facts

content:
  query: string                  # "Is the bridge still intact?"
  location: string               # Where in the world
  time: timestamp                # When (accounting for temporal advance)
  context: string                # Why this matters narratively

token_budget: 200 tokens
```

#### MSG-W02: WorldStateResponse

```yaml
type: WorldStateResponse
sender: WorldAgent
receiver: Storykeeper
when: In response to WorldStateQuery

content:
  answer: string                 # "The bridge was burned during the escape. The river cannot be crossed here."
  state_facts: [fact]            # Structured facts about current world state
  narrative_implications: string # "Any scene set at the low ford should account for the flood."
  constraint_flags: [constraint] # Any hard constraints relevant to the query

token_budget: 300-500 tokens
```

#### MSG-W03: WorldStateUpdate

```yaml
type: WorldStateUpdate
sender: Storykeeper
receiver: WorldAgent
when: Player actions or narrative events change the physical world

content:
  change_description: string     # "The player built a cabin near the creek."
  change_type: enum
    - construction
    - destruction
    - modification
    - movement
    - temporal                   # Season change, aging, decay
  location: string
  time: timestamp
  persistence_tier: enum
    - permanent                  # Structure, major geographic change
    - durable                   # Modification that ages/decays
    - transient                 # Temporary mark, will fade

token_budget: 200-300 tokens
```

#### MSG-W04: ConstraintCheck

```yaml
type: ConstraintCheck
sender: Storykeeper
receiver: WorldAgent
when: Player attempts an action that may violate world rules

content:
  attempted_action: string
  actor: entity_id
  location: string
  time: timestamp

response:
  permitted: bool
  constraint_tier: enum         # hard, soft, perceptual
  explanation: string           # For the Storykeeper's reasoning
  alternative_options: [string] # What the world allows instead

token_budget: 200-300 tokens
```

---

### 8. Narrator → Player

#### MSG-N01: NarrativeOutput

The Narrator's rendered prose — the player's primary experience.

```yaml
type: NarrativeOutput
sender: Narrator
receiver: Player
when: Every turn

content:
  prose: string                  # The rendered narrative — this IS the game
  embedded_signals:              # Metadata for the system (not shown to player)
    significance_markers: [string]  # Objects/details the Narrator weighted heavily
    emotional_register: string      # Current scene mood
    open_threads: [string]          # Things the Narrator left unresolved

token_budget: 200-800 tokens (varies by scene density)

information_boundary:
  The Narrator's output is filtered through its own knowledge.
  It cannot reveal what it doesn't know. It may hint, suggest,
  describe atmospherically — but it cannot state facts it hasn't
  received from the Storykeeper.
```

#### MSG-N02: MetaInformation

Out-of-fiction information for the player (used sparingly).

```yaml
type: MetaInformation
sender: System (through Narrator's voice or system UI)
receiver: Player
when: Onboarding, safety responses, system messages

content:
  message_type: enum
    - onboarding_hint            # "In this story, you can speak to characters..."
    - safety_acknowledgment      # "The scene fades..."
    - system_status              # "Saving..." / "Session ending"
    - unwind_confirmation        # "The thread unravels..."
  content: string

token_budget: 100-200 tokens
```

---

## Turn Cycle: Full Message Sequence

> **Superseded:** The turn cycle below describes the multi-agent architecture with 3-4 serial LLM calls per turn. The current architecture uses a single LLM call (Narrator only) with an 8-stage Bevy ECS pipeline: AwaitingInput → CommittingPrevious → Classifying → Predicting → Resolving → AssemblingContext → Rendering → AwaitingInput. See `turn-cycle-architecture.md` and `storyteller-engine/src/systems/turn_cycle.rs`.

### Standard Turn (Single Character Scene)

```
1. Player types input
   └─→ MSG-P01: PlayerAction

2. Input processor classifies and resolves
   └─→ MSG-S01: ProcessedPlayerAction → Storykeeper

3. Storykeeper evaluates:
   a. Checks constraints → MSG-W04: ConstraintCheck → WorldAgent
   b. WorldAgent responds → MSG-W04 response
   c. Checks information gates (any triggered?)
   d. Updates player-character tensor

4. If gates triggered:
   └─→ MSG-SK02: InformationRelease → Narrator

5. If constraints hit:
   └─→ MSG-SK03: ConstraintGuidance → Narrator

6. Storykeeper updates scene state
   └─→ MSG-SK06: SceneUpdate → CharacterAgent(s)

7. CharacterAgent responds
   └─→ MSG-CA01: CharacterExpression → Narrator

8. Narrator renders
   └─→ MSG-N01: NarrativeOutput → Player

Total messages: 6-8 per turn
Estimated serial LLM calls: 3 (Storykeeper reasoning, CharacterAgent, Narrator)
```

### Multi-Character Turn

```
Same as above through step 6, but:

6. Storykeeper sends MSG-SK06 to ALL active CharacterAgents (parallel)

7. Each CharacterAgent responds with MSG-CA01 (parallel)

8. Orchestration collects responses → MSG-R01 → Reconciler

9. Reconciler sequences → MSG-R02: SequencedScene → Narrator

10. Narrator renders → MSG-N01 → Player

Total messages: 8-12 per turn
Serial LLM calls: 4 (Storykeeper, CharacterAgents[parallel], Reconciler, Narrator)
```

---

## Open Questions Surfaced by This Catalog

> **Resolution note:** Questions 1-3 below were resolved by the narrator-centric pivot. The Storykeeper is now deterministic logic (not an LLM call). The World Agent is a rules engine (not an LLM). CharacterAgent prompts are replaced by ML tensor input features. Questions 4-7 remain relevant as conceptual concerns even though their implementation context has changed.

1. **Storykeeper reasoning as LLM call**: The Storykeeper's evaluation (step 3) requires complex reasoning — checking gates, constraints, updating tensors. Is this one LLM call or multiple? If one, the prompt is enormous. If multiple, latency compounds. This needs benchmarking.

2. **WorldAgent as LLM or rules engine**: For simple constraint checks ("Can a person fly?"), an LLM call is wasteful. The WorldAgent might be a hybrid: a rules engine for hard constraints with an LLM fallback for soft/ambiguous constraints. This has architectural implications.

3. **CharacterAgent prompt size**: MSG-SK05 (CharacterInstantiation) is the system's most token-intensive message. The activated tensor alone is 800-1500 tokens before scene context. If the model's context window is small, the character performance degrades. This directly connects to Open Question #8 (Model Capability).

4. **Reconciler scope**: Does the Reconciler only handle same-scene multi-character interactions, or does it also handle cross-scene coordination (e.g., two characters in adjacent locations whose actions affect each other)? The current catalog assumes same-scene only.

5. **Narrator memory across turns**: The Narrator needs to maintain voice and awareness across the full session. But each turn's MSG-SK01 provides only current scene context. How does the Narrator remember what it said three turns ago? Options: rolling context window, explicit memory injection by Storykeeper, or persistent Narrator state.

6. **Message ordering guarantees**: In the multi-character flow, CharacterAgents respond in parallel. The Reconciler sequences them. But what if one CharacterAgent's response depends on another's? The current design assumes independence within a turn. This may be insufficient for rapid-fire dialogue scenes.

7. **Feedback loop**: After the Narrator renders, the Storykeeper needs to record what was actually said (for information state tracking). This requires a feedback message (MSG-N01 echoed back to Storykeeper) not currently in the catalog. Adding it.

---

## Appendix: Message Flow Diagram

```
                    ┌──────────┐
                    │  PLAYER  │
                    └────┬─────┘
                         │ MSG-P01
                         ▼
                 ┌───────────────┐
                 │ Input Process │
                 └───────┬───────┘
                         │ MSG-S01
                         ▼
              ┌──────────────────────┐        ┌─────────────┐
              │     STORYKEEPER      │◄──────►│ WORLD AGENT │
              │                      │ W01-W04│             │
              │  evaluate action     │        └─────────────┘
              │  check gates         │
              │  update state        │
              └──┬────┬────┬────┬───┘
                 │    │    │    │
         SK01    │    │    │    │ SK05/SK06
         SK02    │    │    │    │
         SK03    │    │    │    ▼
         SK04    │    │    │  ┌──────────────┐
                 │    │    │  │ CHAR AGENT 1 │──┐
                 │    │    │  └──────────────┘  │ CA01
                 │    │    │  ┌──────────────┐  │
                 │    │    │  │ CHAR AGENT 2 │──┤
                 │    │    │  └──────────────┘  │
                 │    │    │  ┌──────────────┐  │
                 │    │    │  │ CHAR AGENT N │──┤
                 │    │    │  └──────────────┘  │
                 │    │    │                    │
                 │    │    │    ┌───────────┐   │
                 │    │    └───►│RECONCILER │◄──┘
                 │    │         │           │
                 │    │         └─────┬─────┘
                 │    │               │ R02
                 ▼    ▼               ▼
              ┌──────────────────────────┐
              │        NARRATOR          │
              │  render prose from:      │
              │  - scene context (SK01)  │
              │  - info releases (SK02)  │
              │  - constraints (SK03)    │
              │  - char expressions      │
              │  - reconciled scene      │
              └────────────┬─────────────┘
                           │ N01
                           ▼
                    ┌──────────┐
                    │  PLAYER  │
                    └──────────┘
```
