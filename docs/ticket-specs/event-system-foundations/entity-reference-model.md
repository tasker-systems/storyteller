# Entity Reference Model

## Purpose

This document addresses how entities are identified, referenced, and promoted through event participation. It extends the existing entity type system (`EntityId`, `EntityOrigin`, `PersistenceMode`) with a reference model that handles the fundamental identity problem: when the prose mentions "the flowers," how does the system know which flowers?

The entity reference model is the bridge between natural language (where identity is contextual and ambiguous) and the relational graph (where identity is discrete and tracked). Events are the mechanism that drives entities across this bridge — from unresolved mentions in prose to tracked entities in the graph.

### Relationship to Other Documents

- **`event-grammar.md`** (companion document) defines `EventAtom` and its `Participant` type, which uses `EntityRef`. Events are the mechanism that creates the relational weight driving entity promotion.
- **`docs/technical/entity-model.md`** describes the unified Entity model with promotion/demotion lifecycle and communicability gradient. This document operationalizes the promotion lifecycle with concrete reference types and weight thresholds.
- **`storyteller-core/src/types/entity.rs`** contains the current stub types (`EntityId`, `EntityOrigin`, `PersistenceMode`). This document extends them.
- **`docs/foundation/world_design.md`** establishes the communicability gradient (Characters → Presences → Conditions → Props) as a spectrum, not a hierarchy. Entity promotion is movement along this spectrum.

---

## The Identity Problem

Consider the problem statement's example:

> In this scene, description, we have a number of possible entities and events... The coffee cup having a chip in it that seems to be part of why she cries is meaningful. But we didn't express any relationships with the table, or the implied chair she would be sitting on.

The system must handle three distinct identity challenges:

### 1. First Mention

When the Narrator writes "a cup of coffee on the table," the cup has no `EntityId`. It exists only as text. The system must decide: is this worth tracking? The answer depends on what happens next — which means the system needs to store the mention and evaluate it retroactively.

### 2. Repeated Reference

When the Narrator later writes "she picked up the cup," does "the cup" refer to the same cup? In this case, spatial and temporal context make it obvious. But "the flowers" might refer to different flowers in different paragraphs — the pressed flowers from a book vs. the wildflowers outside the window.

### 3. Implicit Entities

Some entities are never mentioned but are implied by context. "She sat at the table" implies a chair. "He walked through the door" implies the door was open or he opened it. The system must decide which implicit entities matter.

---

## Entity Reference Types

### Type Sketch

```rust
/// A reference to an entity that may or may not have been resolved to
/// a tracked EntityId.
///
/// Events use EntityRef to identify participants. An event can reference
/// an entity before the system has decided whether to track it. Resolution
/// happens when enough relational context accumulates to either match an
/// existing tracked entity or promote a new one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EntityRef {
    /// Resolved — this entity has an EntityId and is being tracked.
    /// The system knows exactly which entity this refers to.
    Resolved(EntityId),

    /// Unresolved — the entity has been mentioned but not yet matched
    /// to a tracked entity. Carries enough context for later resolution.
    Unresolved {
        /// The mention text — how the entity was referred to.
        /// "the cup," "flowers," "a tall stranger."
        mention: String,
        /// Contextual information that may help resolve the reference.
        context: ReferentialContext,
    },

    /// Implicit — the entity was not mentioned but is implied by context.
    /// "She sat at the table" implies a chair; "the door opened" implies
    /// someone or something opened it.
    Implicit {
        /// What entity is implied.
        implied_entity: String,
        /// What implies it.
        implication_source: String,
    },
}

impl EntityRef {
    /// Returns the EntityId if this reference has been resolved.
    pub fn entity_id(&self) -> Option<EntityId> {
        match self {
            EntityRef::Resolved(id) => Some(*id),
            _ => None,
        }
    }

    /// Whether this reference has been resolved to a tracked entity.
    pub fn is_resolved(&self) -> bool {
        matches!(self, EntityRef::Resolved(_))
    }
}
```

### Referential Context

What we know about an unresolved entity — enough information to attempt resolution when relational context accumulates.

```rust
/// Contextual information about an unresolved entity reference.
///
/// This is not a full entity description — it's the minimum context
/// needed for resolution. The context accumulates across events: the
/// first mention of "the cup" might have only spatial context; the
/// second mention might add a possessive context; the third might
/// add descriptive detail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReferentialContext {
    /// Descriptive details mentioned — "chipped," "old," "silver."
    pub descriptors: Vec<String>,

    /// Spatial context — where this entity was mentioned.
    /// "on the table," "in her hand," "by the stream."
    pub spatial_context: Option<String>,

    /// Possessive context — who this entity belongs to.
    /// "Tanya's phone," "the Wolf's ears."
    pub possessor: Option<EntityRef>,

    /// Prior mentions in the current scene — indices into the
    /// event ledger where this entity was previously referenced.
    /// Used for anaphoric resolution ("she picked it up" — "it"
    /// refers to the most recent compatible mention).
    pub prior_mentions: Vec<EventId>,

    /// The scene in which this entity was first mentioned.
    pub first_mentioned_scene: SceneId,

    /// The turn in which this entity was first mentioned.
    /// Together with `first_mentioned_scene`, provides full
    /// provenance: which Turn → which Scene. Enables efficient
    /// query scoping when resolving entity references — the system
    /// can narrow searches to the turn's event window rather than
    /// scanning the entire scene.
    pub first_mentioned_turn: TurnId,
}
```

---

## The Promotion Lifecycle

Entity promotion is the process by which untracked mentions become fully tracked entities with `EntityId`, relational graph presence, and persistence across scene boundaries. Promotion is driven by relational weight, which is accumulated from **committed events** only.

**The provisional-until-committed principle applies here.** Entities mentioned in ML predictions or Narrator prose do not accumulate promotion weight until the turn is committed (the player responds). This means promotion decisions reflect the narrative as the player actually experienced it, not as the system provisionally generated it. If a Narrator rendering is rejected (X-card, content concern, client disconnect), any entities mentioned in the discarded prose accumulate no weight. See `event-grammar.md`, "The Turn as Atomic Unit of Event Extraction" for the full lifecycle.

**Turn → Scene provenance scoping.** Every promotion event is anchored to the `TurnId` that produced it. Since every Turn belongs to a Scene, the system has a two-level hierarchy for tracking entity provenance: which Turn committed the events that promoted this entity, and which Scene that Turn belongs to. This enables:

- **Efficient query scoping**: "Show me all entities promoted during turns 4-7 of this scene" is a direct ledger query on `turn_id`.
- **Scene gravity computation**: The density of entity promotions per turn, aggregated across a scene, contributes to post-hoc scene gravity — a scene where many entities get promoted is experientially denser than one where few do.
- **Cross-scene persistence decisions**: An entity promoted in multiple scenes (identified by distinct `scene_id` values across its event history) has stronger evidence for `Persistent` tier than one promoted only within a single scene.

### Promotion Tiers

The promotion lifecycle extends the existing `PersistenceMode` (Permanent, SceneLocal, Ephemeral) with finer-grained pre-tracking tiers:

```rust
/// The entity's current position in the promotion lifecycle.
///
/// Entities progress upward through event participation and relational
/// weight accumulation. They can also be demoted when relationships
/// decay. The lifecycle extends the existing PersistenceMode with
/// pre-tracking tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
         serde::Serialize, serde::Deserialize)]
pub enum PromotionTier {
    /// Not mentioned in any event. Does not exist in the system.
    /// This is the "below the waterline" state — entities start here
    /// before they are ever referenced.
    Unmentioned,

    /// Mentioned in prose or events but with no relational implications.
    /// "A cup of coffee on the table." The cup exists as text but has
    /// no relationships. Stored as an unresolved EntityRef in the
    /// event ledger — lightweight, no EntityId.
    Mentioned,

    /// Participates in events with relational implications but has not
    /// accumulated enough weight for full tracking. "She looked at the
    /// cup." The look creates an Attention implication, but one look
    /// isn't enough weight for promotion.
    Referenced,

    /// Has accumulated sufficient relational weight to earn an EntityId
    /// and a node in the relational graph. "She picked up the cup,
    /// noticed the chip, and began to cry." The cumulative relational
    /// implications (possession, attention, emotional connection)
    /// cross the weight threshold.
    ///
    /// Tracked entities persist within the current scene (equivalent
    /// to existing PersistenceMode::SceneLocal).
    Tracked,

    /// Persists across scene boundaries. Requires either authored
    /// importance (PersistenceMode::Permanent from story design) or
    /// sufficient cross-scene relational weight (entity appears in
    /// events across multiple scenes).
    ///
    /// Equivalent to existing PersistenceMode::Permanent.
    Persistent,
}
```

### Promotion Criteria

An entity is promoted when its accumulated relational weight crosses a threshold:

```rust
/// Accumulated relational weight for an entity reference, computed
/// from the events it has participated in.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationalWeight {
    /// The entity being tracked (may be unresolved).
    pub entity: EntityRef,

    /// Total weight from relational implications across all events.
    /// This is the sum of `RelationalImplication::weight` for all
    /// implications involving this entity.
    pub total_weight: f32,

    /// Number of distinct events this entity participates in.
    pub event_count: u32,

    /// Number of distinct other entities this entity has relationships with.
    pub relationship_count: u32,

    /// Weight from player-initiated events specifically. Player attention
    /// is a strong promotion signal — if the player interacts with an
    /// entity, it matters even if its total weight is low.
    pub player_interaction_weight: f32,
}
```

Promotion thresholds:

| Transition | Condition |
|---|---|
| `Unmentioned` → `Mentioned` | Entity appears in any event as a participant |
| `Mentioned` → `Referenced` | Entity participates in at least one event with a `RelationalImplication` |
| `Referenced` → `Tracked` | `total_weight >= TRACKING_THRESHOLD` OR `player_interaction_weight > 0` |
| `Tracked` → `Persistent` | `total_weight >= PERSISTENCE_THRESHOLD` AND `event_count >= MIN_PERSISTENCE_EVENTS` |

`[PLACEHOLDER]` Specific threshold values need calibration from play data. Initial values:
- `TRACKING_THRESHOLD`: 0.5 (approximately: one strong implication or three weak ones)
- `PERSISTENCE_THRESHOLD`: 2.0
- `MIN_PERSISTENCE_EVENTS`: 3

**Player interaction override**: Any direct player interaction (the player references, examines, picks up, or speaks about an entity) immediately promotes to `Tracked` regardless of weight. The player's attention is the strongest signal that something matters.

**Authored override**: Story designers can mark entities as `Tracked` or `Persistent` from the start, bypassing the weight accumulation process. This is the existing `EntityOrigin::Authored` path — authored entities start at their designated tier.

### Demotion

Entities that lose all active relationships decay toward lower tiers:

```rust
/// Demotion criteria — how entities lose their tracking status.
///
/// Demotion is gradual, not immediate. An entity that was important
/// three scenes ago but has had no events since doesn't instantly
/// vanish — it decays through tiers, giving the system time to
/// recognize if it becomes relevant again.
```

| Transition | Condition |
|---|---|
| `Persistent` → `Tracked` | No events involving this entity for `DEMOTION_SCENE_COUNT` scenes AND no active relationships (all edge weights below decay threshold) |
| `Tracked` → `Referenced` | No events involving this entity for `DEMOTION_TURN_COUNT` turns within the current scene |
| `Referenced` → `Mentioned` | All relational implications resolved or decayed |

`[PLACEHOLDER]` Demotion timing values need calibration. Initial values:
- `DEMOTION_SCENE_COUNT`: 3 scenes
- `DEMOTION_TURN_COUNT`: 10 turns

**Demotion protection**: Authored entities (`EntityOrigin::Authored`) cannot be demoted below their authored tier. A main character cannot decay to `Mentioned`.

---

## Retroactive Promotion

One of the key insights from the problem statement: later events can promote earlier entity mentions. The coffee cup is texture until the player asks about the chip — then it retroactively becomes important.

### How It Works

1. **Initial mention**: The Narrator describes "a cup of coffee on the table, with a small chip." When the player responds to this turn (committing it), the turn extraction pipeline records a lightweight mention — an `EventAtom` with the cup as an `EntityRef::Unresolved` participant, `EventKind::StateAssertion`, and no relational implications. The cup is at `Mentioned` tier.

2. **Promotion trigger**: The player says "I look at the chip in the cup." On the *next* turn commit, the extraction pipeline produces an `EventAtom` with the cup as Target, `EventKind::ActionOccurrence(Examine)`, with a relational implication (Attention). The player-interaction override kicks in — the cup is immediately promoted to `Tracked`.

3. **Retroactive resolution**: The system creates an `EntityId` for the cup. It then walks back through the event ledger, finds the original mention, and resolves the `EntityRef::Unresolved` to `EntityRef::Resolved(new_id)`. The original `StateAssertion` atom is updated with the resolved reference.

4. **Graph insertion**: The newly promoted entity gets a node in the relational graph. Its relational implications from all events (the original mention + the player's examination) are materialized as edges.

### Ledger Storage for Unresolved Mentions

The event ledger must store `EventAtom` instances with `EntityRef::Unresolved` participants efficiently. These are lightweight — they don't need graph nodes or full entity records. But they must be queryable for retroactive promotion.

```rust
/// A lightweight record of an unresolved entity mention in the ledger.
///
/// These are stored alongside full EventAtoms but indexed differently.
/// When an entity is promoted, the system queries this index to find
/// all prior mentions and resolve them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedMention {
    /// The event that contains this mention.
    pub event_id: EventId,
    /// The mention text.
    pub mention: String,
    /// The referential context at the time of mention.
    pub context: ReferentialContext,
    /// The scene in which the mention occurred.
    pub scene_id: SceneId,
    /// The turn in which the mention occurred.
    /// Provides temporal scoping within the scene — retroactive
    /// promotion can walk the event ledger from this turn forward
    /// to find subsequent mentions of the same entity.
    pub turn_id: TurnId,
    /// Index into the event's participants list.
    pub participant_index: usize,
}
```

`[INVESTIGATION NEEDED]` Ledger storage format for unresolved mentions. Options:

1. **Inline in EventAtom**: Unresolved mentions are just `EntityRef::Unresolved` variants in the participants list. Resolution updates the atom in place. Simple but requires ledger mutation (events are normally append-only).

2. **Separate mention index**: A side-table of `UnresolvedMention` records that reference event atoms by ID. Resolution creates a new "resolution event" that links the mention to the new EntityId. Preserves ledger immutability.

3. **Resolution overlay**: The ledger is append-only. Resolution adds a `ResolutionEvent` that maps `(event_id, participant_index) → EntityId`. Consumers overlay resolutions when reading the ledger. Most architecturally clean but adds read complexity.

**Recommended starting point**: Option 2 (separate mention index). It preserves ledger immutability while keeping resolution queries simple. Option 3 is the eventual architecture if ledger immutability proves important for replay guarantees.

---

## Resolution Strategies

When an `EntityRef::Unresolved` needs to be matched to an existing entity, the system applies resolution strategies in order:

### 1. Possessive Resolution

"Jane's flowers" → `flowers.possessor = Jane`. If Jane is a tracked entity, the flowers are resolved relative to her. This is the strongest resolution signal — possessive constructions are rarely ambiguous.

```rust
// Resolution logic (conceptual):
if let Some(possessor) = &context.possessor {
    if let Some(possessor_id) = possessor.entity_id() {
        // Look for existing entities owned by this possessor
        // with matching descriptors
        return find_possessed_entity(possessor_id, &context.descriptors);
    }
}
```

### 2. Spatial Resolution

"The cup on the table" → `cup.location = table`. If the table is a tracked entity (or a known location in the scene), the cup is resolved relative to it.

```rust
// Resolution logic (conceptual):
if let Some(spatial) = &context.spatial_context {
    // Look for entities at this location with matching descriptors
    return find_entity_at_location(spatial, &context.descriptors, scene_id);
}
```

### 3. Anaphoric Resolution

"She picked it up" → "it" refers to the most recently mentioned compatible entity. This requires tracking the referent chain — the sequence of entity mentions in the conversation.

```rust
// Resolution logic (conceptual):
if mention_is_pronoun(&mention) {
    // Walk back through prior_mentions, find the most recent
    // entity that is compatible (gender, number, animacy)
    return resolve_pronoun(&context.prior_mentions, &mention);
}
```

### 4. Descriptive Resolution

"The old cup with the chip" → match descriptors against known entities. This is the weakest resolution strategy — descriptors can be ambiguous (multiple old cups) or evolving (an entity gains descriptors over time).

```rust
// Resolution logic (conceptual):
// Score each known entity by descriptor overlap
let candidates = find_entities_with_descriptors(
    &context.descriptors,
    scene_id,
);
if candidates.len() == 1 {
    return Some(candidates[0]);
}
// Multiple candidates — ambiguous, remain unresolved
```

### 5. Player Clarification (Fallback)

If resolution is ambiguous and the player's intent depends on which entity is meant, the system can ask:

> "Which flowers do you mean — the ones in the vase, or the pressed ones from the book?"

This is the fallback of last resort. The system should resolve silently whenever possible. Player clarification is appropriate only when the player's action would have materially different consequences depending on the referent.

`[INVESTIGATION NEEDED]` How much of entity resolution should be rule-based vs. ML-based? The strategies above are described as rule-based (pattern matching on context fields). An ML classifier could handle more nuanced cases — understanding that "she turned back to the old thing" probably refers to the cup, not the table, based on conversation flow. The tradeoff is latency vs. accuracy. Start rule-based; measure resolution quality; add ML if the error rate is unacceptable.

---

## Entity Budget and Scene Limits

Scenes have a finite entity budget — the system can meaningfully track only so many entities within a single scene before context becomes overwhelming. The entity budget is a soft limit that guides promotion decisions.

```rust
/// Scene-level entity budget — how many entities the scene
/// can meaningfully track.
///
/// The budget is not a hard cap — it's a threshold above which
/// the system becomes more conservative about promotion. Below
/// budget, any entity with sufficient weight is promoted. Above
/// budget, only entities with strong player interaction or high
/// weight are promoted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityBudget {
    /// Soft limit on tracked entities in a scene.
    pub soft_limit: u32,
    /// Current count of tracked entities.
    pub current_count: u32,
}
```

`[PLACEHOLDER]` Entity budget values need calibration. Initial estimate: 12-20 tracked entities per scene (based on cognitive load research and token budget constraints). Main cast members (3-6) plus scene-local entities (6-14).

---

## Interaction with the Relational Graph

When an entity is promoted to `Tracked`, it gets a node in the relational graph. Its relationships are materialized from the accumulated relational implications of its events.

### Graph Insertion

```rust
// When an entity is promoted to Tracked:
// 1. Create EntityId
// 2. Create graph node with EntityOrigin::Promoted
// 3. Materialize relational implications as DirectedEdge instances
// 4. Resolve all prior UnresolvedMention records
// 5. Emit EntityLifecycle::Promoted event

fn promote_entity(
    mention: &UnresolvedMention,
    accumulated_implications: &[RelationalImplication],
) -> EntityId {
    let entity_id = EntityId::new();

    // Create initial relational edges from accumulated implications
    for implication in accumulated_implications {
        if let Some(target_id) = implication.target.entity_id() {
            create_edge(entity_id, target_id, &implication.implication_type);
        }
    }

    entity_id
}
```

### Edge Weight from Implications

Relational implications (`ImplicationType`) map to substrate dimensions (`RelationalSubstrate`) at promotion time:

| `ImplicationType` | Substrate dimension(s) affected |
|---|---|
| `Possession` | (spatial relationship, not substrate) |
| `Proximity` | (spatial relationship, not substrate) |
| `Attention` | No direct substrate effect — attention alone doesn't create a relational edge. But accumulated attention contributes to promotion weight |
| `EmotionalConnection` | `affection` (positive valence) or initial signal for trust dimensions (negative valence) |
| `TrustSignal` | `trust_reliability`, `trust_benevolence` |
| `InformationSharing` | `InformationState.known_facts` |
| `Conflict` | `trust_benevolence` (negative), `affection` (negative) |
| `Care` | `trust_benevolence` (positive), `affection` (positive) |
| `Obligation` | `debt` |

Not all implications create substrate edges. Possession and Proximity are spatial relationships tracked by the world model, not the relational web. Attention accumulates promotion weight but doesn't create a relational edge on its own.

---

## Design Principles

1. **Late resolution over early commitment.** The system defers entity identity resolution as long as possible. It's better to store an `Unresolved` reference and resolve it later with more context than to commit to a wrong identity early.

2. **Player attention is the strongest signal.** If the player interacts with an entity — even one that has no relational weight from the narrative — it becomes important. The player's focus defines what matters.

3. **Promotion is gradual, demotion is slow.** Entities earn their place through accumulated weight; they lose it only through sustained absence. This prevents narrative discontinuity where entities pop in and out of existence.

4. **The ledger is the source of truth.** Entity promotion decisions are derived from the event ledger. If you replay the ledger, you get the same promotion decisions. This makes the system debuggable and replayable.

5. **Texture is not failure.** Most entities in a scene will remain at `Mentioned` or `Referenced`. This is correct — a rich scene has many details, and the system should not try to track all of them. The promotion system is a *filter*, not a collector.
