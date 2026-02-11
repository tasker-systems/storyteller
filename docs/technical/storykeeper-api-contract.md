# Storykeeper API Contract

## Purpose

This document defines the **Storykeeper** as a domain-level API contract — a facade over all persistent data access in the storyteller system. Every read and write the system performs during play is framed as a question the Storykeeper answers or a fact the Storykeeper records.

The Storykeeper was originally conceived as an LLM agent — "the memory and map of the story" (`system_architecture.md`). The narrator-centric pivot (`narrator-architecture.md`, Feb 7 2026) moved it away from being an LLM, but the domain concept remains the most powerful organizing principle for our data access patterns. This document reclaims that concept as an **API contract**: the Storykeeper is not an agent anymore, but it is still the single conceptual authority over narrative state, information flow, and the persistence of story.

### The Problem This Solves

Without a domain-level facade, persistence logic scatters across Bevy systems. The `commit_previous_system` would directly call `ledger_writer.append_event()`, `graph_service.update_edge()`, `entity_store.update_weight()` — and the domain semantics of "what constitutes committing a turn" would dissolve into implementation plumbing. The question "why are we writing this data?" becomes impossible to answer from the code alone.

The Storykeeper makes that question answerable. Every data operation maps to a domain action with a clear purpose: "the Storykeeper records what happened," "the Storykeeper assembles what the Narrator needs to know," "the Storykeeper propagates consequences through the relational web."

### Relationship to Other Documents

- **`system_architecture.md`** — The philosophical foundation. The Storykeeper as "memory and map of the story," guardian of mystery, arbiter of information flow. This document operationalizes those concepts into a trait-level API.
- **`narrator-architecture.md`** — The Storykeeper's revised role as context assembly system. This document extends that role to cover the write side (commitment, cascade, truth set) in equal detail.
- **`infrastructure-architecture.md`** — Data lifecycle, durability model, scene entry/exit pipelines. This document defines the domain operations those pipelines implement.
- **`event-system.md`** — Event taxonomy, truth set, priority tiers, cascade management. This document defines how the Storykeeper manages these as domain concerns.
- **`event-dependency-graph.md`** — DAG architecture for combinatorial triggers. The Storykeeper evaluates trigger preconditions as part of its query interface.

---

## The Storykeeper as Domain Facade

### What the Storykeeper Is

The Storykeeper is a **domain service** — not a data access layer, not a repository, not a message router. It is the single conceptual authority that:

1. **Knows the semantics of narrative state.** When it commits a turn, it doesn't just write to a database — it understands that commitment means updating entity weights, cascading relational changes, checking gate conditions, evaluating trigger predicates, and updating the truth set.

2. **Enforces information boundaries as a first-class concern.** Boundaries are not a post-hoc filter on query results. They are integral to every operation — the Storykeeper queries with boundaries built in, and it commits with boundary updates as part of the transaction.

3. **Owns cascade policy.** When a relational edge updates, how far do consequences propagate? How does signal attenuate with network distance? These are Storykeeper-level design decisions that reflect the story's nature, not the database's capabilities.

4. **Mediates between hot (in-memory) and cold (persistent) state.** During active play, the Storykeeper works with Bevy ECS resources. At scene boundaries, it manages the transition to and from durable storage. The rest of the system never needs to know which state is authoritative at any given moment.

### What the Storykeeper Is Not

- **Not a database abstraction.** It does not expose SQL, Cypher, or any storage-level query language. Its interface is domain operations: "what matters about this character right now?" not "SELECT * FROM entities WHERE..."
- **Not a single struct or service.** The Storykeeper is an API contract — a set of trait definitions that may be implemented by multiple collaborating types. The trait surface defines what the system can ask and tell; the implementation chooses how.
- **Not the Narrator's context builder alone.** Context assembly for the Narrator is one of the Storykeeper's read operations, but the Storykeeper also serves the resolver (constraint context), the prediction pipeline (relational features), and the commitment pipeline (write operations). It is the domain authority for all of them.

### Underlying Store Independence

The Storykeeper's interface makes no assumptions about where data lives. Each domain operation maps to one or more underlying stores, but the mapping is an implementation detail:

| Domain Concept | Current (In-Memory) | Future (Persistent) |
|---|---|---|
| Event history | `TurnHistory.turns` | PostgreSQL event ledger (append-only) |
| Relational web | `CharacterSheet` backstory fields | Apache AGE graph (directed edges with substrate dimensions) |
| Narrative graph | `SceneData` hardcoded in workshop | Apache AGE graph (scenes with gravitational mass) |
| Entity lifecycle | `PromotionConfig` in-memory weights | PostgreSQL entity table + event-derived weight history |
| Truth set | Not yet implemented | In-memory during play, checkpointed to PostgreSQL |
| Information boundaries | `CharacterSheet.does_not_know` | PostgreSQL + event ledger (revelation tracking) |
| Scene journal | `JournalResource` (Bevy Resource) | In-memory during play, included in checkpoint |
| Character tensors | `CharacterSheet` hardcoded in workshop | PostgreSQL JSONB (versioned per scene) |

The Storykeeper trait surface is identical regardless of which column is active. Tests use the in-memory implementations; production uses the persistent ones. The engine never knows the difference.

---

## Domain Operations Catalog

The Storykeeper's operations fall into four lifecycle phases. Each operation has a clear domain purpose, defined inputs and outputs, and a mapping to underlying data concerns.

### Phase 1: Scene Entry — "Prepare the World"

When a scene begins, the Storykeeper loads everything needed for real-time play. After scene entry completes, the engine does not query cold storage during the turn cycle (with the exception of the command sourcing write — see Phase 2).

| Operation | Purpose | Inputs | Outputs |
|---|---|---|---|
| **load_scene** | Retrieve the scene definition and its authored structure | Scene ID | `SceneDefinition` (type, setting, cast, stakes, entity budget, authored mass) |
| **load_cast** | Retrieve current state of all characters in the scene | Cast entity IDs | `Vec<CharacterSheet>` with current tensors, emotional states, information boundaries |
| **load_relational_context** | Retrieve the relational web subgraph for the scene's cast | Cast entity IDs | Directed edges with substrate dimensions for every cast pair |
| **load_narrative_position** | Retrieve the current position in the narrative graph | Scene ID, session context | Reachable scenes, gravitational weights, departure conditions, attractor basin |
| **load_entity_histories** | Retrieve recent event history for entities in the scene | Entity IDs, turn window | Event atoms mentioning these entities, weighted by recency |
| **load_truth_set** | Reconstruct the active truth set for this scene | Session checkpoint + ledger delta | `TruthSet` with factual propositions, interpretive propositions, accumulators |
| **load_information_state** | Retrieve per-entity information boundaries | Cast entity IDs | Who knows what, pending gate conditions, revelation history |

The load operations compose into the scene entry pipeline described in `infrastructure-architecture.md`. After loading, the Storykeeper populates Bevy ECS resources that the turn cycle systems read from.

### Phase 2: Active Play — Reads ("What Matters Right Now?")

During the turn cycle, the Storykeeper answers questions about current narrative state. These are synchronous reads from hot (in-memory) state — no database round-trips on the critical path.

| Operation | Purpose | Called By | Inputs | Outputs |
|---|---|---|---|---|
| **query_relational_context** | What is the relationship between these characters, filtered by what's knowable? | Prediction pipeline, context assembly | Character pair, observer's information state | Substrate dimensions visible to the observer, with annotations |
| **query_entity_relevance** | What matters about this entity right now, given the scene context? | Context assembly (Tier 3) | Entity ID, scene context, recent turns | Weighted relevance items: backstory, events, relational edges, emotional context |
| **query_truth_set** | What propositions are currently active that match this predicate? | Trigger evaluation, gate checking | Predicate pattern | Matching propositions with confidence weights |
| **query_gate_proximity** | Which gates are close to triggering? | Context assembly (Tier 3), narrative priming | Scene context, truth set state | Gates with satisfaction percentage and remaining conditions |
| **query_information_boundary** | Is this entity allowed to know this fact? | Context assembly (all tiers), prediction pipeline | Entity ID, fact/proposition | Permitted / withheld / partially revealed |
| **assemble_narrator_context** | Compose the full three-tier context for the Narrator | Context assembly system | Scene, cast, journal, resolver output, player input | `NarratorContextInput` (preamble + journal + retrieved + resolver + summary) |
| **source_command** | Record player input to the durable ledger before processing begins | Input receipt (before turn cycle) | Player input, session context, turn number | Confirmation of durable write |

**`source_command`** is the one write operation that occurs during the turn cycle's critical path. It implements the command sourcing guarantee: the player's input is durably stored before the system processes it. If the server crashes mid-turn, the input survives for replay.

### Phase 3: Active Play — Writes ("What Happened This Turn?")

When a turn commits (triggered by the player's next input), the Storykeeper processes the completed turn's full context. This is the domain transaction — not a database transaction, but a coherent set of domain operations that together constitute "recording what happened."

| Operation | Purpose | Inputs | Outputs / Effects |
|---|---|---|---|
| **commit_turn** | The primary write operation — process a completed turn | `CompletedTurn` with all classification, predictions, events, rendering | Event ledger entries, entity updates, cascade effects |
| **append_events** | Record classified events to the durable ledger | `Vec<EventAtom>`, `Vec<CompoundEvent>`, turn context | Ledger entries with turn provenance and timestamps |
| **update_entity_weights** | Adjust entity weights based on event mentions | Event atoms with entity references | Weight changes, promotion/demotion triggers |
| **cascade_relational_changes** | Propagate relational shifts through the graph with friction | Relational implications from events | Updated edges (immediate), deferred cascade queue (distant) |
| **update_information_state** | Record revelations and check gate conditions | Events that reveal or transfer information | Updated boundary state, newly opened gates |
| **mutate_truth_set** | Add/remove/update propositions based on committed events | Event atoms, interpretive judgments | Updated truth set, newly fired triggers |
| **update_journal** | Add the committed turn to the rolling scene journal | Turn rendering, classification output | Compressed journal with progressive recency weighting |

**`commit_turn`** orchestrates the others. It is the Storykeeper's answer to "what happened?" — a single domain operation that the `commit_previous_system` calls, rather than the system itself managing six different writes.

### Phase 4: Scene Exit — "Settle the World"

When a scene ends, the Storykeeper flushes accumulated state changes to durable storage and processes deferred effects.

| Operation | Purpose | Inputs | Outputs / Effects |
|---|---|---|---|
| **flush_entity_snapshots** | Persist current entity state (tensor topsoil changes, emotional states, weights) | All tracked entities | PostgreSQL entity snapshots |
| **flush_relational_web** | Persist confirmed relational edge changes from this scene | Accumulated edge deltas | AGE graph edge updates |
| **process_deferred_cascade** | Propagate relational changes beyond immediate neighbors | Deferred cascade queue from Phase 3 | Distant edge updates with attenuation applied |
| **process_offscreen_propagation** | Update state for characters not present in the scene | Elapsed time, relevant events | Tensor updates, goal progress, relational shifts for absent characters |
| **recalculate_narrative_graph** | Update gravitational mass and reachability based on scene events | Scene events, truth set state | Updated narrative graph weights, new attractor basins |
| **write_checkpoint** | Snapshot full state for crash recovery | All current state | Checkpoint record in PostgreSQL |
| **dispatch_deferred_processing** | Send long-running effects to tasker-core | Deferred work items | RabbitMQ messages for workflow execution |

---

## The Read Interface in Detail

### "What matters right now about this character?"

This is the central question of the read interface. When the prediction pipeline needs features for a character, or the context assembly system needs Tier 3 content, the question is always: **given the context of this scene, recent turns, and identified events — what is relevant about this character's relationships, history, and state?**

The Storykeeper answers by composing multiple data sources:

```
query_entity_relevance(entity_id, scene_context, recent_turns) → RelevanceResult

RelevanceResult {
    // Relational context: edges to other cast members, filtered by information boundaries
    relational_edges: Vec<BoundedRelationalEdge>,

    // Recent event history: events mentioning this entity, weighted by recency and narrative weight
    recent_events: Vec<WeightedEventReference>,

    // Emotional context: current emotional state with awareness annotations
    emotional_context: EmotionalSnapshot,

    // Active truth set propositions involving this entity
    active_propositions: Vec<WeightedProposition>,

    // Backstory items permitted by information boundaries
    retrieved_backstory: Vec<RetrievedContext>,

    // Gate proximity: how close any gates involving this entity are to triggering
    gate_proximity: Vec<GateProximityItem>,
}
```

The key design principle: **this is a single conceptual query, not a scatter-gather across stores.** The Storykeeper knows how to compose relational context + event history + emotional state + information boundaries into a coherent answer. The calling system doesn't need to know that relational edges come from a graph, events from a ledger, and boundaries from an information state table.

### Information Boundaries as Integral, Not Filtered

Every read operation respects information boundaries natively. The Storykeeper does not retrieve all data and then filter — it queries with boundaries as a parameter.

```
query_relational_context(
    observer: EntityId,      // whose perspective are we assembling?
    target: EntityId,        // who are we asking about?
    scene_context: &Scene,   // what scene are we in?
) → BoundedRelationalEdge

BoundedRelationalEdge {
    // Only the substrate dimensions the observer could plausibly know
    visible_substrates: PartialSubstrate,

    // Annotation of what the observer perceives vs. what's actually true
    // (e.g., the observer thinks trust is high, but it's actually eroding)
    perception_gap: Option<PerceptionGap>,

    // Whether any information about this relationship was revealed this scene
    recently_revealed: Vec<RevelationEvent>,
}
```

This matters because information boundaries are not cosmetic — they directly affect what the ML prediction pipeline receives as input features. If character A doesn't know that character B distrusts them, A's prediction model should receive a different relational feature vector than if A does know. The Storykeeper makes this distinction at query time, not after.

### Tier 3 Context as Storykeeper Query

The current `context/retrieval.rs` hardcodes backstory lookup against `CharacterSheet` fields. In the Storykeeper model, Tier 3 context assembly becomes a structured query:

```
retrieve_narrator_context(
    scene: &SceneData,
    cast: &[CharacterSheet],
    recent_events: &[EventAtom],
    player_input_classification: &ClassificationOutput,
    token_budget: usize,
) → Vec<RetrievedContext>
```

The Storykeeper decides what to retrieve based on:

1. **Entity references** in the current turn — traverse the relational web for relevant edges, pull characterization context
2. **Emotional state changes** from character predictions — relevant self-edge data, awareness-level context
3. **Narrative graph position** — if approaching a gate or attractor, pull tension/stakes context
4. **Information reveals** — if the resolver determines something is newly knowable, pull the revelation context
5. **Thematic echoes** — if the current beat rhymes with an earlier one, pull the echo context for resonance
6. **Gate proximity** — if a gate is close to triggering, bias retrieval toward context that would make the trigger meaningful when it fires

The retrieval is ranked by narrative relevance (not just similarity) and trimmed to fit the token budget. Relevance considers: direct response to player input > active emotional thread > thematic echo > gate proximity > background context.

---

## The Write Interface in Detail

### "What happened this turn?" — The Commit Operation

`commit_turn` is the Storykeeper's primary write operation. It takes a `CompletedTurn` — the full record of what happened — and processes it into durable state changes.

```
commit_turn(
    completed: &CompletedTurn,
    scene_context: &SceneData,
    current_truth_set: &mut TruthSet,
) → CommitResult

CommitResult {
    // Events appended to the ledger
    ledger_entries: Vec<LedgerEntry>,

    // Entity weight changes (promotions, demotions, weight adjustments)
    entity_effects: Vec<EntityEffect>,

    // Relational edge changes (immediate)
    immediate_relational_changes: Vec<RelationalChange>,

    // Relational changes deferred to scene boundary (distant cascade)
    deferred_cascade: Vec<DeferredCascadeItem>,

    // Truth set mutations (new propositions, expired propositions, trigger fires)
    truth_set_mutations: Vec<TruthSetMutation>,

    // Information boundary updates (revelations, gate state changes)
    boundary_updates: Vec<BoundaryUpdate>,

    // Triggers that fired as a result of this commit
    fired_triggers: Vec<FiredTrigger>,
}
```

The commit is a **domain transaction**: all effects are computed together because they may interact. A relational shift might cross a threshold that opens a gate that reveals information that changes what the Narrator should know next turn. The Storykeeper sees the whole picture; no individual store operation could.

### Event Ledger Append

Events are the Storykeeper's durable memory. The ledger is append-only — events are never modified or deleted.

```
append_events(
    events: &[EventAtom],
    compounds: &[CompoundEvent],
    turn_context: &TurnContext,
) → Vec<LedgerEntry>
```

Each ledger entry carries:
- The event atom (kind, participants, confidence, implications)
- Turn provenance (turn number, scene ID, session ID)
- Temporal position (monotonic narrative timestamp for temporal predicate queries)
- Source (which pipeline stage produced this classification)

The ledger serves multiple purposes:
- **Replay**: Reconstruct any turn's state from the last checkpoint
- **Temporal queries**: Support `After`, `Since`, `Sequence` predicates for trigger evaluation
- **Audit**: Complete record of everything that happened, queryable for debugging and training data extraction
- **Cross-scene continuity**: Propositions that carry across scenes identified by querying non-expired events

### Entity Weight Updates

Events mention entities. Each mention contributes to the entity's relational weight, which drives the promotion lifecycle (Unmentioned → Referenced → Tracked → Persistent).

```
update_entity_weights(
    event_atoms: &[EventAtom],
    current_weights: &mut EntityWeightMap,
    config: &PromotionConfig,
) → Vec<EntityEffect>

EntityEffect =
    | WeightAdjusted { entity: EntityRef, delta: f32, new_weight: f32 }
    | Promoted { entity: EntityRef, from: PromotionTier, to: PromotionTier }
    | Demoted { entity: EntityRef, from: PromotionTier, to: PromotionTier }
```

Weight computation follows the existing `promotion` module logic — mention frequency, recency, participant role (Actor > Target > Witness), and narrative weight of the containing event.

### Truth Set Mutation

Events produce propositions. Propositions are what the truth set tracks — not "what happened" but "what is currently true."

```
mutate_truth_set(
    events: &[EventAtom],
    judgments: &[InterpretiveJudgment],
    truth_set: &mut TruthSet,
) → Vec<TruthSetMutation>

TruthSetMutation =
    | PropositionAdded { proposition: Proposition, confidence: f32 }
    | PropositionExpired { proposition: Proposition, reason: ExpirationReason }
    | PropositionRefined { proposition: Proposition, old_confidence: f32, new_confidence: f32 }
    | AccumulatorUpdated { key: AccumulatorKey, delta: f32, new_value: f32 }
    | TriggerFired { trigger: TriggerRef, matched_propositions: Vec<PropositionRef> }
```

The translation from event to proposition follows the rules in `event-system.md`:
- `ActionOccurrence` → momentary proposition (in temporal index) + persistent state change if applicable
- `RelationalShift` → weighted proposition (confidence from classification) + accumulator update
- `InformationTransfer` → boundary update + persistent proposition
- `EntityLifecycle` → persistent proposition (entity exists/promoted/demoted)

### Relational Cascade with Traversal Friction

This is where the Storykeeper's domain knowledge matters most. When a relational edge updates — Sarah's trust in Adam shifts — the Storykeeper must decide what propagates, how far, and with what attenuation.

```
cascade_relational_changes(
    primary_changes: &[RelationalImplication],
    relational_web: &RelationalWeb,
    friction_model: &FrictionModel,
) → CascadeResult

CascadeResult {
    immediate: Vec<RelationalChange>,      // apply this turn
    deferred: Vec<DeferredCascadeItem>,     // apply at scene boundary
}
```

#### The Friction Model

The friction model governs how relational changes propagate through the graph:

**Attenuation**: Signal strength decreases with each hop. The attenuation function is configurable per story, but the default follows a geometric decay:

```
signal_at_distance(d) = signal_at_source * friction_factor^d

where friction_factor ∈ (0.0, 1.0), default 0.5
```

At `friction_factor = 0.5`:
- Distance 1: 50% of original signal (direct neighbor)
- Distance 2: 25% of original signal (friend-of-friend)
- Distance 3: 12.5% of original signal (typically below significance threshold)

**Distortion**: Information changes character as it propagates. The Storykeeper models this as a **category shift** at each hop:

| Hop | Information Character | Example |
|---|---|---|
| 0 (source) | Direct observation | "Sarah told Adam she doesn't trust his guidance" |
| 1 | Secondhand knowledge | "I heard Sarah has concerns about Adam" |
| 2 | Social inference | "There might be tension in the family" |
| 3+ | Vague awareness | "Something is wrong with the family" (below threshold, typically dropped) |

Distortion affects not just the magnitude but the **type** of the relational change. A trust shift at distance 0 is a trust shift. At distance 2, it becomes a generalized "tension" that affects the `history` substrate dimension rather than `trust` specifically.

**Permeability**: Not all edges propagate equally. The propagation strength through an edge depends on:

- **Trust level**: High-trust edges propagate more signal. Information flows more freely between entities that trust each other.
- **History depth**: Long-established relationships carry information better than new ones.
- **Opacity markers**: Some relationships are explicitly opaque — information doesn't flow through them. (A character who keeps secrets from everyone has low permeability on all outgoing edges.)

```
permeability(edge) = base_permeability
    * trust_factor(edge.trust_competence)
    * history_factor(edge.history)
    * opacity_modifier(edge)

effective_signal = signal * permeability(edge)
```

**Immediate vs. Deferred**: The cascade boundary is:
- **Immediate** (within the turn): Changes to edges between entities present in the current scene. These are narratively visible — if Adam is in the scene when Sarah's trust shifts, Adam might sense it.
- **Deferred** (scene boundary): Changes to edges involving entities not present. Tom isn't here; he'll learn about the tension later, processed as part of the scene-exit batch.

This boundary is a design choice, not a technical constraint. It reflects the narrative principle that **what happens in the room happens now; what ripples outward happens later.**

---

## Information Boundary Model

### Core Concepts

Information boundaries are the Storykeeper's most critical responsibility. They implement the foundational principle from `system_architecture.md`: "A story in which all truths are immediately available is a story without mystery. The Storykeeper guards the mystery."

The information boundary model tracks three things:

1. **What each entity knows** — per-entity information state
2. **What has been revealed** — revelation events with provenance
3. **What conditions enable future revelations** — gate predicates over the truth set

### Per-Entity Information State

Each entity has an information state that tracks which facts they are aware of:

```
EntityInformationState {
    entity_id: EntityId,

    // Facts this entity knows (with provenance: how and when they learned it)
    known_facts: Vec<KnownFact>,

    // Facts this entity explicitly does not know (authored, not inferred)
    authored_unknowns: Vec<AuthoredUnknown>,

    // Facts this entity suspects but hasn't confirmed
    suspicions: Vec<Suspicion>,
}

KnownFact {
    fact: Proposition,
    learned_at: TurnId,
    learned_how: LearningMethod,    // direct_observation, told_by(entity), inferred, revealed_by_gate
}

LearningMethod =
    | DirectObservation              // entity was present when it happened
    | ToldBy { source: EntityId }    // another entity communicated it
    | Inferred { from: Vec<Proposition> }  // entity deduced it from other known facts
    | RevealedByGate { gate: GateId }      // a narrative gate opened
    | AuthoredKnowledge              // the entity has always known (backstory)
```

### Revelation Events

When information transfers — through dialogue, observation, gate opening, or inference — the Storykeeper records a revelation event:

```
RevelationEvent {
    fact: Proposition,
    recipient: EntityId,
    source: RevelationSource,
    turn_id: TurnId,
    scene_id: SceneId,
}

RevelationSource =
    | DialogueReveal { speaker: EntityId }
    | ObservationReveal { what_was_observed: String }
    | GateReveal { gate_id: GateId }
    | InferenceReveal { supporting_facts: Vec<Proposition> }
```

Revelation events are appended to the event ledger and update the entity's information state. They are never retracted — once an entity knows something, they know it (though they might later learn it was false).

### Gate Conditions

Gates are predicates over the truth set that, when satisfied, release information:

```
InformationGate {
    gate_id: GateId,
    guarded_facts: Vec<Proposition>,      // what becomes available when the gate opens
    predicate: TriggerPredicate,          // conditions for opening
    recipients: GateRecipients,           // who learns the guarded facts
    narrative_framing: Option<String>,     // how the revelation should feel
}

GateRecipients =
    | Specific(Vec<EntityId>)            // only these entities learn
    | AllPresent                          // everyone in the scene learns
    | Observer(EntityId)                  // only the entity who triggered the gate
```

The Storykeeper evaluates gate predicates as part of truth set mutation. When a commit produces truth set changes, the Storykeeper checks all active gates against the updated truth set. Newly satisfied gates produce revelation events.

### Boundary Enforcement in Queries

Every Storykeeper query that returns narrative content checks information boundaries:

```
// Pseudocode for boundary-aware retrieval
fn retrieve_for_entity(entity_id, facts) -> Vec<BoundedFact> {
    let info_state = self.load_information_state(entity_id);

    facts.iter().filter_map(|fact| {
        if info_state.knows(fact) {
            Some(BoundedFact::Known(fact))
        } else if info_state.suspects(fact) {
            Some(BoundedFact::Suspected(fact, confidence))
        } else if info_state.is_authored_unknown(fact) {
            None  // explicitly withheld
        } else {
            // Default: assume unknown unless evidence of knowledge
            None
        }
    }).collect()
}
```

The current implementation (`CharacterSheet.does_not_know` with substring matching) is a prototype stand-in for this model. The transition path: move from hardcoded lists to event-driven information state, where `KnownFact` entries accumulate through play and `AuthoredUnknown` entries come from the character sheet.

---

## Truth Set Management

### Structure

The truth set is the Storykeeper's working memory — the real-time state of "what is currently true" against which trigger predicates evaluate.

```
TruthSet {
    // Factual propositions — binary, either true or not
    factual: IndexedPropositionStore,

    // Interpretive propositions — true with a confidence weight
    interpretive: WeightedPropositionStore,

    // Accumulators — running counters for threshold tracking
    accumulators: HashMap<AccumulatorKey, f32>,

    // Temporal index — when propositions became true (for After/Since/Sequence predicates)
    temporal_index: BTreeMap<NarrativeTimestamp, Vec<PropositionRef>>,
}
```

### Lifecycle

The truth set is:
- **Reconstructed** at scene entry from the most recent checkpoint + ledger events since checkpoint
- **Mutated** during play by `commit_turn` as events produce/expire propositions
- **Checkpointed** at scene exit as part of the state snapshot
- **Carried across scenes** for propositions that persist (entity states, revealed information, relational shifts)
- **Reset for scene-specific propositions** that don't persist (momentary actions, scene-local state)

### Temporal Predicates

The truth set supports temporal predicates needed by the event dependency DAG:

- **`After(event)`**: True if a matching event exists in the temporal index after the specified event
- **`Since(event, duration)`**: True if a matching event exists within `duration` turns/time of the specified event
- **`Sequence([events])`**: True if all events in the list occurred in order
- **`Within(events, window)`**: True if all events occurred within `window` turns of each other

These are evaluated against the temporal index, not against the full event ledger. The index is compact (proposition references only) and supports efficient range queries.

### Confidence-Weighted Evaluation

Trigger predicates that include interpretive atoms use confidence weights:

- Factual propositions have implicit confidence 1.0
- Interpretive propositions carry explicit confidence from classification
- A configurable minimum confidence threshold (default 0.5) gates interpretive propositions from trigger participation
- Threshold-type triggers sum confidence weights: `Sum(matched_confidences) >= threshold`

---

## Scene Boundary Operations

### Scene Entry: Cold → Hot

Scene entry is the Storykeeper loading the world into working memory. The operations compose into the pipeline described in `infrastructure-architecture.md`:

```
1. load_scene()                    → SceneDefinition          ~10ms
2. load_cast()                     → Vec<CharacterSheet>      ~20-50ms
3. load_relational_context()       → RelationalSubgraph       ~20-50ms
4. load_narrative_position()       → NarrativePosition        ~10-20ms
5. load_entity_histories()         → Vec<EntityHistory>       ~10-20ms
6. load_truth_set()                → TruthSet                 ~20-50ms
7. load_information_state()        → Vec<EntityInfoState>     ~10-20ms

Total: ~100-210ms (before frame computation)
```

After loading, the Storykeeper populates Bevy ECS resources. The turn cycle systems read from these resources — they never call Storykeeper load operations during active play.

### Scene Exit: Hot → Cold

Scene exit is the Storykeeper settling the world. The immediate operations flush data that must be consistent before the next scene; the deferred operations handle effects that can propagate asynchronously.

**Immediate** (before scene transition):
```
1. flush_entity_snapshots()        → Entity state to PostgreSQL
2. flush_relational_web()          → Edge updates to AGE
3. process_deferred_cascade()      → Distant edge updates with friction
4. write_checkpoint()              → Full state snapshot
```

**Deferred** (dispatched to tasker-core via RabbitMQ):
```
5. process_offscreen_propagation() → Off-screen character updates
6. recalculate_narrative_graph()   → Gravitational mass recalculation
7. social_graph_ripple()           → Propagation beyond immediate cascade
8. world_state_updates()           → Environmental, economic, temporal changes
```

The deferred operations follow the DAG workflow pattern from `infrastructure-architecture.md`: some execute in parallel, some depend on others, tasker-core manages the dependency graph and retry logic.

### Between Sessions: Checkpoint + Suspend

When a session suspends:
1. Scene exit operations execute (same as above)
2. Session state marked as suspended in PostgreSQL
3. Bevy ECS resources released

When a session resumes:
1. Load checkpoint from PostgreSQL
2. Check for completed deferred workflows (apply results)
3. Execute scene entry pipeline
4. Resume from checkpoint position

---

## Command Sourcing Guarantee

The command sourcing guarantee is a specific Storykeeper responsibility: **the player's input is durably stored before the system processes it.** This is the synchronous write on the critical path — everything else is async or deferred.

```
source_command(
    player_input: &str,
    session_id: SessionId,
    scene_id: SceneId,
    turn_number: u32,
) → StorytellerResult<CommandSourced>
```

This write must complete before the turn cycle begins. If it fails, the system must not process the input — better to ask the player to retry than to process input that might be lost.

The command sourcing write is minimal:
- Player input text
- Session/scene/turn identifiers
- Timestamp
- Status: `received` (updated to `processed` when the turn cycle completes)

On crash recovery, the system replays unprocessed commands from the ledger against the most recent checkpoint. LLM calls are non-deterministic, so the rendering may differ from the original — but the factual events and state changes are deterministic, so narrative consistency is preserved.

---

## Trait Signatures

The Storykeeper API contract is expressed as Rust traits. The exact shape will be refined during implementation, but the domain-level surface is:

```rust
/// The Storykeeper's read interface — "what matters right now?"
#[async_trait]
pub trait StorykeeperQuery: Send + Sync {
    /// Assemble the full three-tier context for the Narrator
    async fn assemble_narrator_context(
        &self,
        scene: &SceneData,
        cast: &[CharacterSheet],
        journal: &SceneJournal,
        resolver_output: &ResolverOutput,
        player_input: &str,
    ) -> StorytellerResult<NarratorContextInput>;

    /// What is relevant about this entity right now?
    async fn query_entity_relevance(
        &self,
        entity_id: &EntityRef,
        scene_context: &SceneData,
        recent_turns: &[CompletedTurn],
    ) -> StorytellerResult<RelevanceResult>;

    /// What is the bounded relational context between these entities?
    async fn query_relational_context(
        &self,
        observer: &EntityRef,
        target: &EntityRef,
        scene_context: &SceneData,
    ) -> StorytellerResult<BoundedRelationalEdge>;

    /// Which gates are close to triggering?
    async fn query_gate_proximity(
        &self,
        scene_context: &SceneData,
        truth_set: &TruthSet,
    ) -> StorytellerResult<Vec<GateProximityItem>>;

    /// Is this entity permitted to know this fact?
    fn check_information_boundary(
        &self,
        entity_id: &EntityRef,
        fact: &Proposition,
    ) -> BoundaryCheck;
}

/// The Storykeeper's write interface — "what happened this turn?"
#[async_trait]
pub trait StorykeeperCommit: Send + Sync {
    /// Record player input before processing (command sourcing guarantee)
    async fn source_command(
        &self,
        input: &PlayerInput,
        session_context: &SessionContext,
    ) -> StorytellerResult<CommandSourced>;

    /// Process a completed turn into durable state changes
    async fn commit_turn(
        &self,
        completed: &CompletedTurn,
        scene_context: &SceneData,
        truth_set: &mut TruthSet,
        friction_model: &FrictionModel,
    ) -> StorytellerResult<CommitResult>;
}

/// The Storykeeper's lifecycle interface — scene transitions and sessions
#[async_trait]
pub trait StorykeeperLifecycle: Send + Sync {
    /// Load everything needed for a scene
    async fn enter_scene(
        &self,
        scene_id: &SceneId,
        session_context: &SessionContext,
    ) -> StorytellerResult<SceneLoadResult>;

    /// Flush accumulated state and process deferred effects
    async fn exit_scene(
        &self,
        scene_context: &SceneData,
        truth_set: &TruthSet,
        deferred_queue: &[DeferredCascadeItem],
    ) -> StorytellerResult<SceneExitResult>;

    /// Snapshot state for crash recovery
    async fn write_checkpoint(
        &self,
        session_context: &SessionContext,
        scene_context: &SceneData,
        truth_set: &TruthSet,
    ) -> StorytellerResult<CheckpointId>;

    /// Restore from checkpoint + replay ledger delta
    async fn resume_from_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> StorytellerResult<SceneLoadResult>;
}
```

These three traits may be implemented by a single type or by separate types that compose. The split reflects the domain: reading, writing, and lifecycle management are distinct concerns even though they share underlying state.

---

## Relationship to Existing Implementation

### What Currently Implements Storykeeper Logic

The Storykeeper's responsibilities are currently distributed across several modules without a unifying abstraction:

| Current Location | Storykeeper Operation | What Changes |
|---|---|---|
| `context/mod.rs` — `assemble_narrator_context()` | `StorykeeperQuery::assemble_narrator_context` | Becomes a method on the Storykeeper trait; gains graph-backed Tier 3 retrieval |
| `context/retrieval.rs` — `retrieve_context()` | `StorykeeperQuery::query_entity_relevance` | Replaces hardcoded backstory lookup with graph traversal + boundary enforcement |
| `context/journal.rs` — `add_turn()` | `StorykeeperCommit::update_journal` | Absorbed into `commit_turn` orchestration |
| `systems/turn_cycle.rs` — `commit_previous_system` | `StorykeeperCommit::commit_turn` | System calls `storykeeper.commit_turn()` instead of managing writes directly |
| `components/turn.rs` — `TurnHistory` | `StorykeeperCommit::append_events` | In-memory vec becomes a trait call; impl can be in-memory or PostgreSQL |
| Not yet implemented | `StorykeeperCommit::cascade_relational_changes` | New — relational cascade with friction model |
| Not yet implemented | Truth set management | New — `TruthSet` type and mutation logic |
| Not yet implemented | Information boundary tracking | New — replaces `does_not_know` substring matching |
| Not yet implemented | Gate condition evaluation | New — predicate evaluation over truth set |
| Not yet implemented | Scene entry/exit lifecycle | New — load/flush operations for persistent state |
| Not yet implemented | Command sourcing | New — synchronous durable write before processing |

### Migration Path

The migration is incremental:

1. **Define the traits** in `storyteller-core/src/traits/` (alongside existing `llm.rs`)
2. **Implement `InMemoryStorykeeper`** that wraps the current in-memory behavior (TurnHistory, JournalResource, hardcoded retrieval) behind the new trait surface
3. **Refactor engine systems** to depend on `dyn StorykeeperQuery + StorykeeperCommit` instead of directly manipulating resources
4. **Implement `PostgresStorykeeper`** when the schema and migrations are ready
5. **Swap implementations** via Bevy Resource configuration — tests use in-memory, production uses PostgreSQL

Step 2 is the critical one: it proves the trait surface is correct without requiring any persistence work. The in-memory implementation is a mechanical refactor of existing code behind the new interface.

---

## Implications for Implementation Tickets

### Tickets That Need Revision

**TAS-242 (Schema Design)**: Should be revised to derive the schema from the Storykeeper's domain operations, not from a table-first perspective. The schema serves the Storykeeper — tables map to domain concepts (event ledger → `append_events`, turns → `commit_turn`, entities → `update_entity_weights`).

**TAS-243 (LedgerWriter Trait)**: The `LedgerWriter` trait becomes an internal implementation detail of the Storykeeper, not a public-facing abstraction. The engine talks to the Storykeeper; the Storykeeper talks to the LedgerWriter. The trait may still exist, but its consumer is the Storykeeper, not the engine systems.

**TAS-246 (Ledger Actor Service)**: The actor/mpsc pattern is correct for the LedgerWriter's implementation, but the service is an internal component of the Storykeeper, not a standalone service that engine systems address directly.

**TAS-247 (GraphRAG Retriever)**: Becomes an internal component of the Storykeeper's query implementation. The retriever is how the Storykeeper answers `query_entity_relevance` and assembles Tier 3 context — but the engine never calls the retriever directly.

### New Tickets Needed

1. **Define Storykeeper traits** — The three trait definitions (`StorykeeperQuery`, `StorykeeperCommit`, `StorykeeperLifecycle`) in `storyteller-core/src/traits/`
2. **Implement InMemoryStorykeeper** — Wraps current in-memory behavior behind the trait surface
3. **Refactor engine systems** — `commit_previous_system`, `assemble_context_system`, and other turn cycle systems to use Storykeeper traits
4. **Design truth set types** — `TruthSet`, `Proposition`, `TriggerPredicate`, accumulator types
5. **Design information boundary types** — `EntityInformationState`, `KnownFact`, `InformationGate`, `RevelationEvent`
6. **Design friction model types** — `FrictionModel`, attenuation functions, permeability computation
7. **Implement PostgresStorykeeper** — After schema design (TAS-242) and migrations (TAS-243)

### Resequencing

The revised sequence:

```
TAS-266 (this document) — Storykeeper API contract design
    ↓
New: Define Storykeeper traits + InMemoryStorykeeper
    ↓
New: Refactor engine systems to use Storykeeper traits
    ↓
TAS-267 — Knowledge graph domain model (informs schema)
    ↓
TAS-242 (revised) — Schema design, derived from Storykeeper operations
    ↓
TAS-244 — AGE spike (informed by TAS-267 query patterns)
    ↓
TAS-243 — Migrations + internal LedgerWriter (component of Storykeeper)
    ↓
TAS-245 — Graph schema (informed by TAS-244 results)
    ↓
New: PostgresStorykeeper implementation
    ↓
TAS-246 — Ledger actor (internal to PostgresStorykeeper)
    ↓
TAS-247 — GraphRAG retriever (internal to PostgresStorykeeper)
```

This sequence ensures we understand the domain (266, 267) before designing the schema (242), verify the technology (244) before committing to it (245), and build the abstraction layer (traits + InMemory) before the persistence implementation (Postgres).

---

## Appendix: The Storykeeper's Philosophical Grounding

From `system_architecture.md`:

> "A story in which all truths are immediately available is a story without mystery. The Storykeeper guards the mystery. But the Storykeeper is not adversarial. Its purpose is not to prevent the player from learning things, but to ensure that learning them *means something* — that the moment of discovery carries the weight it was designed to carry, or better still, a weight that emerges organically from the particular path the player took to arrive there."

This is not just philosophy — it is an architectural constraint. Every Storykeeper operation must preserve this property: information is never arbitrarily withheld, and it is never arbitrarily revealed. The mechanisms described in this document — gates, boundaries, friction, cascade attenuation — are all implementations of this single principle.

The Storykeeper is the faithful steward of authorial intent. But "faithful does not mean rigid." When player actions create emergent situations the designer did not anticipate, the Storykeeper must adjudicate. The truth set, trigger predicates, and gate conditions provide the structured machinery for this adjudication — but the Storykeeper's design must always leave room for the possibility that the most meaningful revelation is the one nobody planned.
