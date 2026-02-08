# Event Grammar

## Purpose

This document defines the event grammar — the formal specification of what constitutes an *event* in the storyteller system, how events are represented, how they compose, and how they create the relational weight that makes entities worth tracking.

The event grammar sits at the intersection of several systems: the event classifier (which produces events from player input), the character prediction pipeline (which produces events from ML inference), the Narrator (which produces events embedded in prose), and the event ledger (which persists events for replay and graph construction). All of these systems need a shared language for what an event *is*.

### Relationship to Other Documents

- **`docs/technical/event-system.md`** describes the two-track classification pipeline, truth set, sensitivity map, and priority tiers. This document specifies the *data model* that flows through that pipeline. The existing event-system.md was designed for the multi-agent architecture and will be revised (Phase F of the implementation plan) to incorporate this grammar.
- **`docs/technical/narrator-architecture.md`** describes the narrator-centric turn cycle. Events are produced at multiple stages of that cycle.
- **`storyteller-core/src/types/event.rs`** contains the current stub types (`EventId`, `EventPriority`, `NarrativeEvent`). The event grammar extends and refines these.
- **`storyteller-core/src/types/prediction.rs`** contains `EventType`, `EmotionalRegister`, `ClassifiedEvent` — the current player-input classification types. The event grammar subsumes these into a broader taxonomy.
- **`entity-reference-model.md`** (companion document) defines how entities are referenced and promoted through event participation.

---

## The Relational Weight Principle

The central insight: **events create relationships, and relationships create entities that matter.**

An entity earns its existence in the system — its `EntityId`, its place in the relational graph, its persistence across scene boundaries — by participating in events that create or modify relationships. Without relationships, an entity has no narrative weight. Without narrative weight, an entity is texture: present in the prose, absent from the graph.

This is not a constraint on what the Narrator can mention. The Narrator can describe a room full of objects, a crowd of faces, a landscape of details. But the system only *tracks* entities that participate in relational events. The coffee cup on the table is texture until someone picks it up, asks about it, or cries because of the chip in its rim. The act of crying-because-of-the-chip creates a relationship between the character and the cup, and that relationship gives the cup enough weight to earn an `EntityId`.

This principle has architectural consequences:

1. **Events are the gateway to entity promotion.** An entity cannot be promoted from `Mentioned` to `Tracked` without participating in at least one event that carries relational implications. See `entity-reference-model.md` for the full promotion lifecycle.

2. **Relational weight is computable from events.** Given the events an entity has participated in, we can compute a scalar weight that determines its persistence tier. This weight is the sum of the relational implications of those events, not a subjective judgment.

3. **Events without relational implications are scene texture.** "Tanya sits at the table" is a state assertion. It places Tanya spatially but creates no new relationship. It enters the truth set (Tanya is at the table) but does not promote the table unless the table already has relationships or acquires them through subsequent events.

4. **The event ledger is the source of truth for entity weight.** Entity promotion decisions are derived from the ledger, not from ad hoc heuristics. This makes promotion deterministic and replayable.

---

## Event Sources

Events originate from five sources, each with different characteristics:

### 1. Player Input (Classified)

Player text is classified by the event classifier (currently naive keyword-based in `classify_player_input()`, eventually ML) into structured event data. This is the existing `ClassifiedEvent` pathway.

**Characteristics**: Always has a player-character as actor. Confidence depends on classifier quality. Produces a single `EventAtom` or a small set of atoms.

### 2. Character Predictions (ML Output — Provisional)

The character prediction pipeline (`CharacterPredictor` + enrichment) produces structured intents for each non-player character. These intents — act, speak, think — inform context assembly and Narrator rendering.

**Characteristics**: One per character per turn. High structure (the ML model outputs typed fields). Predictions are **provisional** — they are working state used to assemble the Narrator's context, not committed events. They become evidence for event extraction at turn commit time (see "The Turn as Atomic Unit" below).

### 3. Committed Turn Extraction (Post-Hoc)

The primary event extraction pathway. When a player responds to the Narrator's rendered prose, the turn is committed and the extraction pipeline processes the complete turn unit: Narrator prose + player response + ML prediction metadata.

**Characteristics**: This is a post-hoc classification of the completed turn, using the same event grammar and ML modeling approach as player input classification. The Narrator's prose and the player's input are both generative text from non-deterministic sources with distinct perspectives — they are treated symmetrically by the classifier. See "The Turn as Atomic Unit" below for the full design.

### 4. System Events (Deterministic)

Scene lifecycle events (scene entry, scene exit), entity lifecycle events (promotion, demotion, dissolution), and world state changes. These are produced by the engine's deterministic systems.

**Characteristics**: High confidence (system-produced). No classification needed. Enter the truth set immediately.

### 5. Composed Events (Derived)

Compound events detected from patterns across multiple atoms. "Tanya looks at the chipped cup and begins to cry" is two atoms (look, cry) with a causal composition that carries more narrative weight than either atom alone.

**Characteristics**: Derived, not observed. Detection is a separate pipeline stage. See Composition Rules below.

---

## The Event Atom

An `EventAtom` is the minimal unit of event in the system. Everything the event system processes is either an atom or a composition of atoms.

### Type Sketch

```rust
/// The minimal unit of event in the system.
///
/// Every event — player action, character prediction, narrator description,
/// system lifecycle — is represented as one or more EventAtoms. Atoms are
/// the entries in the event ledger and the inputs to trigger evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventAtom {
    /// Unique identifier for this atom in the ledger.
    pub id: EventId,
    /// When this event was recorded.
    pub timestamp: DateTime<Utc>,
    /// What kind of event this is.
    pub kind: EventKind,
    /// Who or what is involved.
    pub participants: Vec<Participant>,
    /// What relationships this event creates, modifies, or implies.
    pub relational_implications: Vec<RelationalImplication>,
    /// Where this event came from.
    pub source: EventSource,
    /// How confident we are that this event occurred.
    pub confidence: EventConfidence,
    /// Processing priority (from existing EventPriority).
    pub priority: EventPriority,
    /// The scene in which this event occurred.
    pub scene_id: SceneId,
    /// The turn in which this event occurred (if applicable).
    /// System events (scene lifecycle) may not belong to a specific turn.
    pub turn_id: Option<TurnId>,
}
```

### Event Kind

The semantic taxonomy of events. This is broader than the existing `EventType` (which classifies player input actions) — it covers all event sources.

```rust
/// What kind of event occurred — the semantic category.
///
/// This taxonomy covers events from all sources: player input, character
/// predictions, narrator output, system events, and composed events.
/// The existing `EventType` (Speech, Action, Movement, etc.) maps to a
/// subset of these kinds for player-input events specifically.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    /// An assertion about current state — "Tanya is sitting at the table."
    /// These establish facts in the truth set but often carry no relational
    /// implications on their own.
    StateAssertion {
        /// What is being asserted.
        assertion: String,
    },

    /// An action that occurred — "Sarah picked up the stone."
    /// Most actions carry relational implications (actor→target, actor→instrument).
    ActionOccurrence {
        /// The type of action (maps to existing ActionType where applicable).
        action_type: ActionType,
    },

    /// A spatial change — "The Wolf crossed the stream."
    /// May carry relational implications if the movement is toward or away
    /// from another entity.
    SpatialChange {
        /// Where from (if known).
        from: Option<String>,
        /// Where to (if known).
        to: Option<String>,
    },

    /// An emotional expression — visible to others in the scene.
    /// "Tanya begins to cry." Creates relational implications with witnesses.
    EmotionalExpression {
        /// Which emotion, if identifiable.
        emotion_hint: Option<String>,
        /// Intensity of expression. Range: [0.0, 1.0].
        intensity: f32,
    },

    /// Information transfer — one entity reveals something to another.
    /// "Adam tells Sarah about the path." Always relational.
    InformationTransfer {
        /// What was communicated (summary, not content).
        content_summary: String,
    },

    /// A speech act — someone speaks.
    /// Maps directly to existing EventType::Speech for player input.
    SpeechAct {
        /// The register of speech delivery.
        register: SpeechRegister,
    },

    /// A relational shift — the relationship between entities changes.
    /// "Trust between Sarah and the Wolf eroded." This is typically
    /// produced by the interpretive track, not the factual track.
    RelationalShift {
        /// Which substrate dimension shifted.
        dimension: String,
        /// Direction and magnitude.
        delta: f32,
    },

    /// An environmental or world-state change — "Rain begins to fall."
    /// May carry relational implications if it affects entities in the scene.
    EnvironmentalChange {
        /// What changed.
        description: String,
    },

    /// A scene lifecycle event — entry, exit, transition.
    SceneLifecycle {
        /// What happened in the lifecycle.
        lifecycle_type: SceneLifecycleType,
    },

    /// An entity lifecycle event — promotion, demotion, dissolution.
    EntityLifecycle {
        /// What happened to the entity.
        lifecycle_type: EntityLifecycleType,
    },
}

/// Scene lifecycle subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SceneLifecycleType {
    Entered,
    Exited,
    CharacterArrival,
    CharacterDeparture,
}

/// Entity lifecycle subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntityLifecycleType {
    Promoted,
    Demoted,
    Dissolved,
}
```

### Participants

Who or what is involved in an event. Participants carry a role that describes their relationship to the event.

```rust
/// An entity participating in an event, with its role.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Participant {
    /// Reference to the entity — may be resolved (has EntityId) or
    /// unresolved (mentioned by name/description, identity pending).
    /// See entity-reference-model.md for the EntityRef type.
    pub entity: EntityRef,
    /// What role this entity plays in the event.
    pub role: ParticipantRole,
}

/// The role an entity plays in an event.
///
/// Roles determine how the event's relational implications flow. An Actor
/// has agency; a Target receives the action; a Witness observes but is
/// not directly involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParticipantRole {
    /// The entity performing the action. Every ActionOccurrence and
    /// SpeechAct has exactly one Actor.
    Actor,
    /// The entity receiving the action or speech. May be absent
    /// (actions without a target).
    Target,
    /// An object or tool used in the action — "picked up the stone."
    /// The stone is the Instrument.
    Instrument,
    /// Where the event takes place. Typically a spatial entity —
    /// "at the table," "by the stream."
    Location,
    /// An entity that observes the event without direct participation.
    /// Witnesses may gain information and have emotional reactions,
    /// creating relational implications with lower weight than
    /// Actor or Target.
    Witness,
    /// The entity being discussed or referenced without being present
    /// or active — "She spoke about her brother."
    Subject,
}
```

### Relational Implications

What relationships an event creates or modifies. This is the bridge between events and the relational web — the mechanism by which events create the weight that promotes entities.

```rust
/// A relational implication of an event — what relationship it creates
/// or modifies between participants.
///
/// An event may have zero or many relational implications. "Tanya sits at
/// the table" has no relational implications (state assertion, no new
/// relationship). "Tanya picks up the cup" has one (Tanya → cup: possession).
/// "Tanya throws the cup at John" has several (Tanya → cup: release,
/// Tanya → John: aggression, John → cup: incoming threat).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationalImplication {
    /// The source entity in the directed relationship.
    pub source: EntityRef,
    /// The target entity in the directed relationship.
    pub target: EntityRef,
    /// What kind of relational change this implies.
    pub implication_type: ImplicationType,
    /// How strong this relational implication is. Range: [0.0, 1.0].
    /// Determines how much weight this event contributes to the
    /// entities' relational profiles.
    pub weight: f32,
}

/// What kind of relational change an event implies.
///
/// These map to (but are not identical to) the substrate dimensions in
/// `RelationalSubstrate`. An implication is a *signal* that a substrate
/// dimension may need updating; the Storykeeper decides whether and how
/// much to actually shift the substrate values.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ImplicationType {
    /// Possession or ownership — "picked up the cup."
    Possession,
    /// Spatial proximity — "approached the fence."
    Proximity,
    /// Attention or focus — "looked at the flowers."
    Attention,
    /// Emotional connection (positive or negative) — "cried because of the chip."
    EmotionalConnection {
        /// Positive or negative valence.
        valence: f32,
    },
    /// Trust signal — "told her the truth" or "lied about the path."
    TrustSignal {
        /// Direction of trust change.
        direction: f32,
    },
    /// Information sharing — "revealed the secret."
    InformationSharing,
    /// Conflict or opposition — "resisted the Wolf's guidance."
    Conflict,
    /// Care or protection — "shielded the child."
    Care,
    /// Debt or obligation — "saved his life" or "borrowed the flute."
    Obligation {
        /// Positive = debt owed to source, negative = debt owed to target.
        direction: f32,
    },
}
```

### Event Source

Where the event came from — important for provenance tracking and confidence calibration.

```rust
/// Where an event originated — for provenance and confidence calibration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventSource {
    /// Classified from player input text.
    PlayerInput {
        /// The original player text (or a reference to it).
        raw_input: String,
        /// Which classifier produced this event.
        classifier: ClassifierRef,
    },
    /// Extracted from committed turn content — Narrator prose + player
    /// response processed together as a turn unit.
    TurnExtraction {
        /// The turn this event was extracted from.
        turn_id: TurnId,
        /// Which classifier processed the turn unit.
        classifier: ClassifierRef,
    },
    /// Originally hypothesized by an ML character prediction, confirmed
    /// by turn extraction. The prediction metadata is preserved as
    /// supporting evidence.
    ConfirmedPrediction {
        /// Which character's prediction originally hypothesized this event.
        character_id: EntityId,
        /// The confidence of the original ML prediction.
        prediction_confidence: f32,
    },
    /// Produced by the engine's deterministic systems.
    System {
        /// Which system component produced this.
        component: String,
    },
    /// Derived from composition of other atoms.
    Composed {
        /// The atoms this was composed from.
        source_atom_ids: Vec<EventId>,
    },
}

/// Reference to a classifier model.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassifierRef {
    /// Name of the classifier.
    pub name: String,
    /// Version or model identifier.
    pub version: String,
}
```

### Event Confidence

Confidence with provenance — not just how confident we are, but why.

```rust
/// How confident the system is that this event occurred, with provenance.
///
/// Confidence is not a single number — it carries the reasoning that
/// produced it. This allows downstream consumers (trigger evaluation,
/// Storykeeper) to weigh events appropriately.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventConfidence {
    /// Scalar confidence. Range: [0.0, 1.0].
    pub value: f32,
    /// What evidence supports this confidence.
    pub evidence: ConfidenceEvidence,
}

/// What supports the confidence assessment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConfidenceEvidence {
    /// The classifier produced this confidence directly.
    ClassifierOutput {
        /// Classifier model name.
        classifier: String,
        /// Latency of classification in milliseconds.
        latency_ms: u32,
    },
    /// System-produced events have inherent confidence.
    SystemProduced,
    /// Sensitivity map keyword match.
    SensitivityMatch {
        /// Which sensitivity entry matched.
        entry_id: String,
        /// The baseline confidence from the sensitivity entry.
        baseline: f32,
    },
    /// Refined by deep interpretation (Stage 4).
    DeepInterpretation {
        /// The provisional confidence before refinement.
        provisional: f32,
        /// Supporting event references.
        supporting_events: Vec<EventId>,
    },
    /// Composed from child atom confidences.
    ComposedConfidence {
        /// The individual atom confidences.
        atom_confidences: Vec<f32>,
    },
}
```

---

## Composition Rules

Events rarely stand alone. A character looking at a chipped cup and then crying is two atoms — but their conjunction carries more narrative weight than either alone. The composition system detects meaningful patterns across atoms and creates compound events.

### Compound Event

```rust
/// A compound event — an ordered set of atoms with a composition type
/// that together carry more narrative weight than the sum of their parts.
///
/// Compound events are derived, not observed. The composition detector
/// identifies them from sequential atoms based on temporal proximity,
/// participant overlap, and causal/thematic patterns.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompoundEvent {
    /// Unique identifier for this compound event.
    pub id: EventId,
    /// The atoms that compose this event, in order.
    pub atoms: Vec<EventId>,
    /// How the atoms are related.
    pub composition_type: CompositionType,
    /// The emergent relational implications — may differ from or exceed
    /// the sum of the individual atoms' implications.
    pub emergent_implications: Vec<RelationalImplication>,
    /// Confidence in the composition itself (not in the individual atoms).
    pub composition_confidence: f32,
    /// Processing priority — inherited from the highest-priority atom,
    /// or elevated if the composition has narrative urgency.
    pub priority: EventPriority,
}

/// How atoms in a compound event are related.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CompositionType {
    /// A caused B — "she saw the chip and began to cry."
    /// The first atom is the cause, subsequent atoms are effects.
    Causal {
        /// Brief description of the causal relationship.
        mechanism: String,
    },
    /// A then B — temporal sequence without explicit causation.
    /// "He picked up the stone. Then he crossed the stream."
    Temporal,
    /// A enabled B — A created conditions for B to occur.
    /// "She opened the door, revealing the garden beyond."
    Conditional {
        /// What condition was created.
        condition: String,
    },
    /// A echoes B — thematic resonance without causal or temporal link.
    /// "The chip in the cup mirrors the crack in her composure."
    Thematic {
        /// What theme connects the atoms.
        theme: String,
    },
}
```

### Composition Detection

Compound events are detected by the composition system, which runs after individual atoms have been classified. Detection uses:

1. **Temporal proximity** — atoms within the same turn or adjacent turns are candidates for composition.
2. **Participant overlap** — atoms sharing participants (especially when one entity is Actor in both) are composition candidates.
3. **Causal patterns** — known causal patterns (see → react, reveal → respond, approach → greet) trigger composition detection.
4. **Thematic resonance** — `[PLACEHOLDER]` This requires the sensitivity map's thematic pattern matching and is the least well-defined detection mechanism.

### Emergent Weight

A compound event's relational weight is not simply the sum of its atoms' weights. Composition creates emergent meaning:

- **Causal composition**: The compound's weight is typically *greater* than the sum. "She saw the chip and cried" carries more relational weight between the character and the cup than "she saw the chip" + "she cried" separately, because the causal link establishes emotional significance.

- **Temporal composition**: The compound's weight is typically *equal* to the sum. Temporal sequence without causation doesn't amplify meaning.

- **Conditional composition**: The compound's weight depends on the significance of the enabled action. "She opened the door" is low-weight; "she opened the door, revealing the hidden garden" is high-weight because the enabled discovery carries the narrative load.

- **Thematic composition**: The compound's weight is contextual — thematic echoes are significant to the Storykeeper and Narrator but may not affect entity promotion directly. Their weight is applied to narrative mass calculations rather than entity relational weight.

`[PLACEHOLDER]` The specific weight formulas need calibration from play data. Initial implementation should use simple additive weights with a composition multiplier (e.g., causal = 1.5x, temporal = 1.0x, conditional = variable, thematic = 0.5x for entity weight but 2.0x for narrative mass).

`[PLACEHOLDER]` Maximum composition depth. Initial implementation should cap at depth 2 (a compound can contain atoms but not other compounds). Deeper compositions may be needed for complex narrative sequences but risk combinatorial explosion in detection.

---

## Alignment with Existing Types

The event grammar is designed to extend, not replace, existing working types.

### `EventType` (7 variants) → `EventKind`

The existing `EventType` classifies player-input actions specifically. Each variant maps to an `EventKind`:

| `EventType` | `EventKind` |
|---|---|
| `Speech` | `SpeechAct { register }` |
| `Action` | `ActionOccurrence { action_type: Perform }` |
| `Movement` | `SpatialChange { from, to }` |
| `Observation` | `ActionOccurrence { action_type: Examine }` |
| `Interaction` | `ActionOccurrence { action_type }` (varies) |
| `Emote` | `EmotionalExpression { emotion_hint, intensity }` |
| `Inquiry` | `InformationTransfer { content_summary }` |

`EventType` is not replaced — it remains the classifier's output vocabulary for player input. The classifier produces an `EventType`, which the event pipeline maps to an `EventKind` when constructing the `EventAtom`. This keeps the classifier's contract stable while the event grammar evolves.

### `ClassifiedEvent` → Extends to Carry `EventAtom`

The existing `ClassifiedEvent` has: `event_type`, `targets`, `inferred_intent`, `emotional_register`, `confidence`. The extended version wraps this in an `EventAtom`:

```rust
// The classifier still produces ClassifiedEvent.
// The event pipeline constructs an EventAtom from it:

fn classified_event_to_atom(
    classified: &ClassifiedEvent,
    player_id: EntityId,
    scene_id: SceneId,
    turn_id: TurnId,
) -> EventAtom {
    let kind = event_type_to_kind(classified.event_type);
    let participants = build_participants(player_id, &classified.targets);
    let implications = infer_implications(&participants, &kind);

    EventAtom {
        id: EventId::new(),
        timestamp: Utc::now(),
        kind,
        participants,
        relational_implications: implications,
        source: EventSource::PlayerInput {
            raw_input: classified.inferred_intent.clone(),
            classifier: ClassifierRef { /* ... */ },
        },
        confidence: EventConfidence {
            value: classified.confidence,
            evidence: ConfidenceEvidence::ClassifierOutput { /* ... */ },
        },
        priority: EventPriority::High,
        scene_id,
        turn_id: Some(turn_id),
    }
}
```

### `NarrativeEvent` → Typed Payload

The existing `NarrativeEvent` has a `serde_json::Value` payload. This is the stub that the event grammar replaces:

```rust
// Current:
pub struct NarrativeEvent {
    pub id: EventId,
    pub timestamp: DateTime<Utc>,
    pub priority: EventPriority,
    pub payload: serde_json::Value,  // untyped
}

// Revised:
pub struct NarrativeEvent {
    pub id: EventId,
    pub timestamp: DateTime<Utc>,
    pub priority: EventPriority,
    pub payload: EventPayload,  // typed
}

/// The typed payload of a narrative event.
pub enum EventPayload {
    /// A single event atom.
    Atom(EventAtom),
    /// A compound event (composition of atoms).
    Compound(CompoundEvent),
    /// Legacy untyped payload — for migration compatibility.
    /// [PLACEHOLDER] Remove once all event sources produce typed payloads.
    Untyped(serde_json::Value),
}
```

### `EventPriority` → Unchanged

The existing `EventPriority` (Immediate, High, Normal, Low, Deferred) applies directly to atoms and compounds. No changes needed.

### `CharacterPrediction` → Provisional Hypothesis, Not Event Source

In the original design, each `CharacterPrediction` was treated as an event source. In the turn-unit model, predictions are **hypotheses** — working state that informs Narrator rendering but does not enter the event ledger directly. The extraction pipeline at turn commit time may *confirm* a prediction (the Narrator rendered the predicted flinch and the player responded to it), in which case the resulting `EventAtom` carries `EventSource::ConfirmedPrediction` with the original prediction metadata as supporting evidence.

The prediction-to-event flow:

- `ActionPrediction` → **Hypothesized** during the turn. If the Narrator renders the action and the player's response doesn't contradict it, turn extraction produces an `EventAtom` with `EventKind::ActionOccurrence` and `EventSource::ConfirmedPrediction`.
- `SpeechPrediction` → Same flow. If the Narrator renders dialogue and the player engages with it, turn extraction confirms.
- `ThoughtPrediction` → Internal state, not externally observable. Does not produce events. May influence the Narrator's rendering (subtext, body language) but the observable *effects* are what get extracted as events, not the thought itself.
- `EmotionalDelta` → May be observable if the Narrator renders it ("her face fell"). The rendered expression is what gets extracted, not the delta directly.

---

## The Turn as Atomic Unit of Event Extraction

### The Provisional-Until-Committed Principle

In the ludic narrative sense, nothing "happens" until a player responds to it. The Narrator's rendered prose is a proposal — it becomes narrative fact only when the player witnesses it and acts. This is not merely a game-state convenience; it reflects a fundamental principle of interactive narrative: the player's engagement is what makes content real.

This principle has concrete architectural consequences:

1. **ML character predictions are working state.** They inform the Narrator's context assembly but are not committed to the event ledger. The prediction said "Character X would flinch" — the Narrator may render the flinch, soften it, amplify it, or ignore it. The prediction is the hypothesis; the committed turn is the observation.

2. **Narrator prose is provisional.** If the player's client disconnects before they see the Narrator's output, that output can be regenerated from the same event state and ML predictions (possibly producing different prose — the Narrator is non-deterministic). No phantom events exist from a rendering the player never witnessed.

3. **The player's response commits the turn.** When the player responds, the system knows: this is the content the player experienced and engaged with. The turn is now a complete unit — Narrator prose + player response — ready for event extraction.

4. **Player rejection resets without cost.** If the player X-cards a scene moment or marks a rendering as inappropriate, the turn was never committed. No events were extracted, no substrate shifts happened, no entities were promoted. The system re-renders with adjusted constraints (the Storykeeper adds safety/content constraints to the Narrator's context). This breaks immersive frame but is a psychological safety necessity, and the architecture supports it cleanly because the provisional model means rejection is cheap.

### The Three-State Event Lifecycle

Events progress through three states, which may formalize as a state machine:

```
Hypothesized ──→ Rendered ──→ Committed
    │                │             │
    │                │             ▼
    │                │         Events extracted,
    │                │         ledger updated,
    │                │         truth set modified
    │                │
    │                ▼
    │            Player sees prose.
    │            Provisional — can be
    │            rejected/re-rendered.
    │
    ▼
ML predictions computed.
Working state only — informs
Narrator context assembly.
```

**Hypothesized**: ML character predictions are computed. They are structured intent data (act/say/think with confidence) used to assemble the Narrator's context. They do not enter the event ledger. If the system crashes here, nothing is lost — predictions can be recomputed from the same inputs.

**Rendered**: The Narrator has produced prose. The player may or may not have seen it. The prose is stored as part of the turn's working state but is not yet committed. If the player rejects the rendering (X-card, content concern, client disconnect), the system discards it and re-renders. No events have been extracted.

**Committed**: The player has responded. The turn is complete. The event extraction pipeline processes the full turn unit:

- **Narrator prose** — what was rendered as the narrative moment
- **Player response** — what the player did with it
- **ML prediction metadata** — what the models predicted, as supporting evidence for extraction confidence

From this bundle, the extraction pipeline produces the turn's `EventAtom` instances. These are committed to the event ledger. The truth set is updated. Entity promotion runs. Subscriber handlers fire.

### The Turn Model

The Turn is a first-class type in the system — the unit of player interaction that owns committed events and scopes entity promotion. Turns belong to Scenes, providing a two-level hierarchy for provenance tracking and query scoping.

```rust
/// Unique identifier for a turn within a scene.
///
/// Uses UUID v7 (time-ordered) for efficient BTree indexing and
/// natural temporal ordering, following the same pattern as
/// EventId and SceneId.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
    serde::Serialize, serde::Deserialize,
)]
pub struct TurnId(pub Uuid);

impl TurnId {
    /// Create a new time-ordered turn ID (UUID v7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

/// A turn — the unit of player interaction within a scene.
///
/// A turn captures the full cycle: system generates content (predictions +
/// Narrator rendering), player witnesses and responds, events are extracted.
/// The turn is the atomic unit of event extraction — nothing is committed
/// to the event ledger until a turn completes.
///
/// In Bevy, the active turn is a Resource. Scene entry creates the first
/// turn; each player response commits the current turn and creates the next.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Turn {
    /// Unique identifier.
    pub id: TurnId,
    /// The scene this turn belongs to.
    pub scene_id: SceneId,
    /// Ordinal position within the scene (1-indexed).
    /// Human-readable sequential number; TurnId provides the
    /// globally unique, time-ordered identity.
    pub turn_number: u32,
    /// Current lifecycle state.
    pub state: TurnState,
    /// When the turn was created (predictions begin).
    pub created_at: DateTime<Utc>,
    /// When the Narrator's rendering was completed.
    pub rendered_at: Option<DateTime<Utc>>,
    /// When the player responded (turn committed).
    pub committed_at: Option<DateTime<Utc>>,
}
```

**Why TurnId matters:**

1. **Provenance**: Every `EventAtom` carries a `turn_id`, linking it to the turn that produced it. Combined with `scene_id`, this gives full hierarchical provenance: which Turn → which Scene → which events.

2. **Query scoping**: SQL and graph queries can scope by `turn_id` (events within a turn) or `scene_id` (events within a scene). Knowing which Turn committed which events narrows queries efficiently.

3. **Scene gravity computation**: The density and reach of events per turn, aggregated across a scene, provides a post-hoc measure of scene gravity. A scene with many high-weight turns (dense event extraction, many entity promotions) has higher actual gravity than its authored mass alone. This feeds back into the narrative graph's gravitational landscape.

4. **Bevy integration**: The active `Turn` is a Bevy Resource. `TurnPhase` and `TurnPhaseKind` (already in `message.rs`) track pipeline progress within the current turn. The existing `PlayerInput.turn_number` and `TurnPhase.turn_number` fields migrate to `TurnId` references, preserving `turn_number` as a human-readable ordinal.

**Migration note**: The existing `PlayerInput.turn_number: u32` and `TurnPhase.turn_number: u32` in `message.rs` will migrate to use `TurnId` as the authoritative identifier. The `turn_number: u32` ordinal is preserved on `Turn` for human readability and sequential ordering, but `TurnId` is the canonical reference used in events, entity references, and ledger queries.

### Turn-Unit Extraction Pipeline

The extraction pipeline processes a committed turn as a single unit, using the same event grammar and classification approach as player input:

```rust
/// A committed turn — the atomic unit of event extraction.
///
/// Contains everything needed to extract events: what was rendered,
/// what the player did, and what the ML models predicted. Once a
/// turn is committed, its events are extracted exactly once
/// (idempotency guarantee).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommittedTurn {
    /// Unique identifier for this turn.
    pub turn_id: TurnId,
    /// The scene this turn occurred in.
    pub scene_id: SceneId,
    /// Ordinal position within the scene (1st turn, 2nd turn, etc.).
    /// Preserved for human readability and sequential ordering.
    pub turn_number: u32,
    /// The Narrator's rendered prose for this turn.
    pub narrator_prose: String,
    /// The player's response text.
    pub player_response: String,
    /// ML prediction metadata — not events themselves, but evidence
    /// that supports extraction confidence.
    pub prediction_metadata: Vec<PredictionMetadata>,
    /// Whether events have already been extracted for this turn.
    /// Once true, the turn is not re-processed (idempotency).
    pub events_extracted: bool,
}

/// Metadata from an ML character prediction, preserved as evidence
/// for turn extraction. Not an event source — evidence for one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PredictionMetadata {
    /// Which character this prediction was for.
    pub character_id: EntityId,
    /// What the model predicted (action type, speech, emotional state).
    /// Summarized — not the full CharacterPrediction.
    pub predicted_action_type: Option<ActionType>,
    pub predicted_speech: bool,
    pub predicted_emotional_shift: bool,
    /// The model's confidence in its predictions.
    pub confidence: f32,
}
```

The extraction pipeline itself uses the same classifier infrastructure as player input. Both player text and Narrator prose are generative text from non-deterministic sources with distinct perspectives — they are treated symmetrically:

```rust
fn extract_turn_events(
    turn: &CommittedTurn,
    known_entities: &[TrackedEntity],
    scene_context: &SceneContext,
) -> Vec<EventAtom> {
    // 1. Classify the player response (same as existing classify_player_input)
    let player_atoms = classify_text(
        &turn.player_response,
        EventSource::PlayerInput { /* ... */ },
        known_entities,
    );

    // 2. Classify the Narrator prose (same classifier, different source)
    let narrator_atoms = classify_text(
        &turn.narrator_prose,
        EventSource::TurnExtraction { turn_id: turn.turn_id, /* ... */ },
        known_entities,
    );

    // 3. Cross-reference with prediction metadata to confirm/boost confidence
    let confirmed_atoms = confirm_predictions(
        &narrator_atoms,
        &turn.prediction_metadata,
    );

    // 4. Combine, deduplicate, and return
    combine_and_deduplicate(player_atoms, confirmed_atoms)
}
```

### What This Architecture Provides

**Idempotency**: All content for event evaluation is associated with a turn. Once a turn has been processed, it is not processed again. The `events_extracted` flag ensures this.

**Clean replay**: If the system crashes after ML prediction but before turn commit, the prediction is recomputed and the Narrator re-renders. If it crashes after commit, the extracted events are already in the ledger.

**Natural context assembly hook**: Events extracted at turn commit determine what feeds forward into the Narrator's context for the *next* turn. The Storykeeper decides which extracted events are relevant to the Narrator's context vs. which are simply ledgered. The Narrator doesn't need to know that we graduated the attention and narrative relevance of an entity through a series of event subscriber handlers — it sees the *effect* (updated relational dynamic, new information boundary) not the *mechanism*.

**Information boundary enforcement**: The Narrator is not omniscient. When turn extraction detects that a player casually mentioned hunting a stag, and the event system knows (from the relational graph) that a character in the scene has a sworn-secrecy relationship with that stag, the substrate shifts (the character becomes guarded). But the Narrator's context for the next turn shows only the *effect* — "Character X is noticeably more guarded" — not the *cause*. The Narrator renders the guardedness authentically *because* it doesn't know why. Information boundaries are preserved not by filtering the Narrator's output, but by controlling its input.

`[INVESTIGATION NEEDED]` The classifier architecture for turn-unit extraction. The same ML/rule-based classifier used for player input should work for Narrator prose, but the characteristics differ: Narrator prose is longer, more descriptive, contains multiple characters' actions, and uses literary rather than conversational language. The classifier may need tuning or a separate feature set for prose input vs. player input. Start with the same classifier and measure extraction quality; add prose-specific features if accuracy is insufficient.

---

## Connection to the Truth Set

Events enter the truth set (described in `docs/technical/event-system.md`) through a translation step. The event grammar defines what an event *is*; the truth set defines what is *currently true*. The translation:

- `EventKind::StateAssertion` → persistent proposition (true until contradicted)
- `EventKind::ActionOccurrence` → momentary proposition (true at timestamp, in temporal index)
- `EventKind::SpatialChange` → persistent proposition (new location replaces old)
- `EventKind::EmotionalExpression` → momentary proposition + accumulator update
- `EventKind::InformationTransfer` → persistent proposition (information once known stays known)
- `EventKind::SpeechAct` → momentary proposition (speech occurred)
- `EventKind::RelationalShift` → persistent proposition + substrate update
- `EventKind::EnvironmentalChange` → persistent proposition (rain continues until it stops)
- `EventKind::SceneLifecycle` → persistent proposition (character present/absent)
- `EventKind::EntityLifecycle` → persistent proposition (entity at new tier)

The confidence value from `EventConfidence` determines whether the proposition enters the factual store (confidence >= threshold, default 0.8 for factual events) or the weighted store (confidence < threshold, for interpretive events).

---

## Design Principles

1. **Nothing happens until the player responds.** Events are extracted from committed turns, not from provisional state. ML predictions are hypotheses; Narrator prose is a proposal; only the player's engagement makes content real. This is the foundational principle of the event lifecycle.

2. **The turn is the atomic unit.** Event extraction processes a complete turn (Narrator prose + player response + prediction metadata) exactly once. This provides idempotency, clean replay semantics, and natural turn-boundary hooks for context assembly.

3. **Atoms are cheap, compositions are meaningful.** Creating an atom should be a low-cost operation. The system should err on the side of creating too many atoms rather than too few — the composition and promotion systems will determine which matter.

4. **Relational implications are the currency of weight.** An event without relational implications is valid but lightweight. The system tracks it in the ledger but doesn't promote entities based on it.

5. **Confidence is not binary.** Events exist on a confidence spectrum. The trigger system (truth set evaluation) handles this naturally through weighted propositions.

6. **Source provenance is preserved.** Every event knows where it came from. This enables auditing, debugging, and confidence calibration over time.

7. **The grammar is extensible.** New `EventKind` variants, new `ImplicationType` variants, and new `CompositionType` variants can be added without breaking existing events. The `serde` derives support forward-compatible deserialization.
