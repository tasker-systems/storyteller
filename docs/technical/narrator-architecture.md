# Narrator-Centric Architecture

## Purpose

This document describes a fundamental architectural revision to the storyteller system's agent model, motivated by empirical evidence from the first playable scene (February 2026). The original design distributed LLM calls across multiple agents — Character Agents, Narrator, and eventually Storykeeper and World Agent. This document proposes collapsing the LLM-dependent agents to a **single Narrator agent**, replacing the others with purpose-built ML prediction models, deterministic rules engines, and structured context assembly.

The revision preserves everything that worked in the original architecture — information boundaries, parallel intent generation, the turn cycle pipeline, and the scene-as-unit-of-play model — while redistributing the computational work to match what each component is actually good at.

### What the Prototype Taught Us

The first playable scene ("The Flute Kept," Bramblehoof + Pyotir against local Ollama with Mistral 7B) proved:

1. **The pipeline shape is correct.** Player input → classification → character deliberation → reconciliation → narrative rendering. The information flows and agent boundaries produce coherent results.
2. **Character tensors translate to LLM behavior.** The tensor-to-natural-language pipeline shapes model output. Voice differentiation is real.
3. **Information boundaries hold.** Characters don't leak knowledge they shouldn't have.
4. **Parallel agent deliberation works.** Independent agents with distinct context produce distinct intents.

And it revealed:

1. **Multi-agent LLM calls are computationally prohibitive.** Even on high-end consumer hardware (M4, 64GB), minutes elapsed between turns. Two character agents + one narrator = three sequential-to-parallel LLM calls per turn, each with substantial context.
2. **LLMs are expensive, slow, inconsistent persona maintainers.** The character agents produced structured intent data (ACTION/BENEATH/THINKING) that was consumed by downstream agents. We were using LLMs as expensive lookup tables for "what would this character do" — exactly what a well-trained predictive model should handle.
3. **LLMs are good at prose synthesis.** The narrator, despite Mistral's limitations, demonstrated what LLMs do well: tonal control, scene rendering, showing over telling (when well-prompted). This is the work the LLM should be doing.
4. **The narrator parrots context.** When rich prose descriptions appear in context, the model pattern-matches and regurgitates rather than synthesizes. Context should be structured, not pre-rendered.

### Relationship to Other Documents

- **`agent-message-catalog.md`** describes the original multi-agent message flow. This document **supersedes** the turn cycle and agent roles described there, while preserving the message type inventory as a reference for what information flows between pipeline stages.
- **`infrastructure-architecture.md`** describes the system topology. The topology changes: ML inference becomes more central, LLM calls reduce to one per turn, the Storykeeper becomes a context assembly system.
- **`scene-model.md`** remains fully valid. Scenes are still the unit of play, with the same anatomy and lifecycle.
- **`event-system.md`** remains valid. Events still flow through the same two-track classification system.
- **Emotional model** (`docs/foundation/emotional-model.md`) becomes the theoretical basis for the character prediction model's feature space.
- **`tensor-schema-spec.md`** becomes more important, not less — the character tensor is the primary input to ML prediction.

---

## Revised System Architecture

### The Turn Cycle

```
Player Input
    │
    ▼
Event Classifier (ML, fast)
    │  natural language → typed events
    │  entity references, action type, inferred intent
    ▼
Character Prediction (ML, per-character, parallel)
    │  tensor + emotional state + relational context + scene → intent
    │  structured output: act / say / think with confidence
    ▼
Action Resolution (rules engine)
    │  world model constraints, character capabilities,
    │  genre physics, narrative precedence, spatial proximity
    ▼
Context Assembly (Storykeeper function)
    │  select what the Narrator needs to attend to
    │  relational web traversal, narrative graph position,
    │  information boundaries, emotional subtext
    ▼
Narrator (LLM — the only one)
    │  rich context, capable model, one call per turn
    ▼
Player reads prose
```

The computational graph funnels: many fast ML inferences → deterministic resolution → one expensive but high-quality LLM call. Each layer does what it is actually good at.

### What Changed, What Stayed

| Component | Original | Revised | Why |
|-----------|----------|---------|-----|
| Character Agents | LLM per character | ML prediction model | Persona maintenance is a prediction problem, not a generation problem |
| Narrator | One of several LLM agents | The only LLM agent | Prose synthesis is what LLMs are genuinely good at |
| Storykeeper | Deterministic filter for downstream agents | Context assembly system for the Narrator | Same information-boundary role, different consumer |
| Reconciler | Deterministic sequencer | Rules engine with world model integration | Needs richer constraint awareness for single-Narrator flow |
| World Agent | Planned as LLM agent | Interrogable world model (rules engine) | World constraints are deterministic, not generative |
| Event Classifier | Planned as ML classifier | ML classifier (unchanged) | This was always the right approach |
| Scene Model | Unit of play | Unit of play (unchanged) | Scenes still bound context and cast |
| Character Tensors | Input to LLM system prompts | Feature input to ML prediction | More important, not less — richer features = better predictions |
| Psychological Frames | Planned ML pre-processing step | The primary character-behavior path | Was always heading here; now it's the whole path, not a preprocessing step |

---

## The Narrator

### Role

The Narrator is the system's sole generative intelligence. It is the subjectivity behind the tale — how personas are expressed, what the prose shows vs. tells, where the camera rests, what silence means. Every other component in the system exists to prepare the right context for this one agent.

The Narrator receives structured inputs and produces literary prose. It does not decide what characters do — the prediction model does that. It does not decide what is physically possible — the resolver does that. It does not decide what information is available — the Storykeeper does that. The Narrator decides **how to tell it**.

### Identity

The Narrator's voice is defined by:

- **Genre and tone**: Persistent guidance on register, vocabulary, period, mood. ("Write in the voice of a fairy tale told around a fire — lyrical but grounded, never precious.")
- **Aesthetic constraints**: Anti-patterns and positive directives. ("Never state a character's emotion in summary. Never re-describe the setting after the opening beat. Let silence carry meaning.")
- **Scene-specific modulation**: Each scene can override or adjust tonal parameters. A scene of grief has different aesthetic constraints than a scene of discovery.

These identity elements live in the persistent preamble (Tier 1 of the context architecture, described below) and change only at scene boundaries.

### What the Narrator Does NOT Receive

The Narrator never receives raw character tensors, floating-point values, or pre-rendered prose descriptions. It receives:

- **Character intent summaries**: "Bramblehoof plays a melody from their shared past — testing whether Pyotir remembers. Beneath: longing, defended by performance."
- **Resolved action outcomes**: "Pyotir hears the melody and recognizes it. His hands tighten on the instrument case. He does not speak."
- **Emotional subtext annotations**: "Pyotir's grief is Defended — he acts from it but cannot name it. The melody opens a crack he will try to close."
- **Scene dynamics**: "Contrasting emotional postures: Bramblehoof extends through music, Pyotir withdraws into stillness."

This is structured narrative fact with emotional annotation — not prose, not numbers. The Narrator's job is to transform this into prose. If the context were already prose, the model would compress and parrot rather than create.

---

## The Three-Tier Context Architecture

The Narrator's context window is finite. The relevant narrative world grows with every turn. The fundamental tension: enough context for specificity and emotional truth, but not so much that the model drowns, contradicts itself, or parrots its inputs.

The solution is a three-tier context system where each tier has different update frequencies and compression characteristics.

### Tier 1: Persistent Preamble

**Updated**: At scene boundaries only.
**Size**: ~600-800 tokens.

The Narrator's identity and scene contract:

- Genre, tone, aesthetic register, authorial voice guidelines
- Anti-patterns ("do not summarize emotions," "do not re-describe settings," "do not parrot retrieved context")
- Scene setting (rendered once as structured affordances, not prose)
- Cast list with one-line characterizations: "Pyotir, a young minstrel carrying grief he can't name"
- Current scene phase and narrative temperature
- Scene-level information boundaries: what has been revealed, what must not be stated

This is the Narrator's ground truth. It is always present, changes rarely, and provides the frame within which the Narrator exercises creative freedom.

### Tier 2: Rolling Scene Journal

**Updated**: After every turn, with progressive compression.
**Size**: ~800-1200 tokens, growing slowly as compression keeps pace with accumulation.

The system's memory of what has happened in this scene. Not conversation history — a **maintained narrative record** built from typed events.

Each turn produces a set of classified events and resolved outcomes. The journal stores these at varying resolution:

- **Recent turns** (last 2-3): Full detail. Character intents, resolved actions, emotional shifts, player input, dynamics notes.
- **Earlier turns**: Compressed to key beats. "Bramblehoof played the melody; Pyotir recognized it but didn't speak."
- **Opening beats**: Compressed to facts. "They met at the crossroads. Bramblehoof arrived first."

The compression is not LLM-generated. It is structural: the journal stores typed events at varying resolutions. Recent events include the full event with emotional context. Older events store the event type, key entities, and outcome. The system maintains this journal using deterministic rules — recency, narrative weight, emotional significance — not generative summarization.

The journal gives the Narrator the feeling of continuity — "I know what's been happening" — without carrying the full conversational weight of every prior turn.

#### Progressive Compression Rules

1. **Recency**: Full detail decays to summary over 3-4 turns.
2. **Narrative weight**: High-weight events (revelations, emotional shifts, player choices with consequences) resist compression. They stay at higher resolution longer.
3. **Redundancy**: Repeated patterns ("Pyotir deflects again") compress into a single note rather than accumulating instances.
4. **Emotional state**: The current emotional temperature of each character is maintained as a running summary, not reconstructed from turn history.

### Tier 3: Retrieved Context

**Updated**: Per-turn, assembled fresh based on current turn's needs.
**Size**: ~400-800 tokens (variable).

This is where the graph comes in. Each turn, after the classifier processes player input and the resolver produces outcomes, the Storykeeper asks: **what does the Narrator need to know right now that isn't already in Tiers 1 and 2?**

Retrieval is triggered by:

- **Entity references** in player input → traverse relational graph for relevant edges, pull characterization context
- **Emotional state changes** from character prediction → relevant self-edge data, awareness-level context
- **Narrative graph position** → if approaching a gate scene or gravitational attractor, pull tension/stakes context
- **Information reveals** → if the resolver determines something is newly knowable, pull the revelation context with appropriate framing
- **Thematic echoes** → if the current beat rhymes with an earlier one (detected by the event classifier or a dedicated echo detector), pull the echo context so the Narrator can weave resonance

The retrieved context is always **structured narrative fact with emotional annotation**, never pre-rendered prose. Example:

```
Retrieved (player asked about Pyotir's family):
  Subject: Pyotir's family — farmers in the valley
  Revealed: His father died two winters ago (Pyotir knows, has not mentioned)
  Unrevealed: His sister runs the farm (Pyotir is proud but guilty about leaving)
  Emotional context: Family is Defended territory — Pyotir would deflect
    a direct question but might mention his sister if music is involved
  Information boundary: Bramblehoof does not know any of this
```

The Narrator receives this and decides how to render it — whether Pyotir deflects, goes quiet, changes the subject, or lets something slip. The Narrator creates; the system provides the materials.

### Token Budget

For a single Narrator turn:

| Tier | Content | Tokens |
|------|---------|--------|
| 1 | Persistent preamble | ~600-800 |
| 2 | Scene journal | ~800-1200 |
| 3 | Retrieved context | ~400-800 |
| Input | Classified events, resolved intents, player text | ~300-500 |
| **Total input** | | **~2100-3300** |
| **Output budget** | One narrative beat | **~500-800** |

This is a single LLM call with a manageable context window. A capable model (30B+ local, or a cloud API) handles this well. The turn latency becomes: ML inference (~50-200ms) + resolution (~10-50ms) + context assembly (~20-50ms) + one LLM call (~2-8s depending on model). A significant improvement over three sequential LLM calls.

### The Forgetting Problem

The scene journal's progressive compression is the forgetting mechanism. But the risk is losing something that becomes relevant again later.

The solution: the full event ledger exists outside the Narrator's context. The Storykeeper's retrieval system (Tier 3) can always reach back into the ledger when triggered by player input or narrative conditions. The Narrator "forgets" that Bramblehoof mentioned ley lines in turn 3 — but if the player says "what was that about corruption?" in turn 15, the classifier detects a reference to a previous topic, the Storykeeper finds the relevant event in the ledger, and it enters the Narrator's context for that turn.

The Narrator does not need to remember. The system remembers for it and provides on demand.

This mirrors how skilled human storytellers work. You hold the current beat, the recent flow, and the key emotional threads in working memory. When something triggers a connection, you recall the relevant detail. The rest is available but not active.

---

## Character Prediction

### The Shift from Generation to Prediction

The original architecture asked LLMs to *generate* character behavior: given a system prompt constructed from the character tensor, produce what the character would do, say, and think. This works — the prototype proved it — but it is slow, expensive, and inconsistent. LLMs are mediocre at consistent persona maintenance across turns and excellent at prose synthesis. We were asking them to do the wrong thing.

The revised architecture treats character behavior as a **prediction problem**. Given a character's tensor profile, current emotional state, active relational edges, scene context, and the classified player input, predict the character's intent: what they would do, say, and think, with confidence values.

This is the psychological frame concept from `tensor-schema-spec.md` and `docs/foundation/emotional-model.md`, promoted from a pre-processing step to the primary character-behavior path. The ML model reads structured features and produces structured output — no natural language generation, no context windows, no prompt engineering.

### Input Features

The prediction model consumes:

- **Character tensor** (activated subset): The relevant axes for this scene, selected by the frame computation pipeline. Not the full tensor (~2500-3000 tokens) but the activated frame (~200-400 tokens equivalent in feature dimensions).
- **Current emotional state**: Primary emotion intensities, active mood-vector, awareness levels per primary.
- **Active relational edges**: Substrate dimensions (trust, affection, debt, history, projection) for each relationship in the current scene. Topology position (gate, bridge, hub, peripheral).
- **Scene context**: Scene type (gravitational/connective/gate/threshold), narrative temperature, current phase, rendered space affordances.
- **Classified player input**: Typed event(s) from the classifier — action type, targets, inferred intent, emotional register.
- **Recent intent history**: The character's own intents from the last 2-3 turns (prevents repetition, enables trajectory).

### Output Structure

The model predicts:

```
CharacterIntent {
  act: ActionPrediction {
    description: string,        // "Plays a melody from their shared past"
    confidence: f32,            // 0.0-1.0
    action_type: ActionType,    // perform, speak, move, examine, wait, resist
  },
  say: Option<SpeechPrediction> {
    content_direction: string,  // "References the melody's origin without naming it"
    register: SpeechRegister,   // whisper, conversational, declamatory, internal
    confidence: f32,
  },
  think: ThoughtPrediction {
    emotional_subtext: string,  // "Longing, defended by performance"
    awareness_level: AwarenessLevel,  // Articulate, Recognizable, Preconscious, Defended
    internal_conflict: Option<string>, // "Wants to be recognized but fears rejection"
  },
  emotional_shift: Option<EmotionalDelta>,  // predicted change to emotional state
}
```

This is structured data, not prose. The Narrator transforms it into story. The resolver checks it against world constraints. The Storykeeper uses it to maintain the information ledger.

### Training Data: The Combinatorial Matrix

Training a prediction model requires examples: "given these features, this is what the character does." The bootstrap problem is generating enough high-quality examples.

#### The Three-Axis Generation Matrix

The training data generation approach uses three axes of combinatorial variation:

**Axis 1: Character Profiles (narrative archetype → tensor template)**

Narrative archetypes serve as tensor generation templates. Not Jungian essences — generative starting points that produce characteristic tensor profiles with variation.

Examples:
- Byronic hero → high defiance, defended grief, structural pride, low trust-of-institutions
- Brave mother → high protective-instinct, articulate fear, bedrock loyalty, topsoil anxiety
- Wise elder → high patience, sediment-layer knowledge, defended regret, low impulsivity
- Clever trickster → high adaptability, preconscious mischief, surface-level trust, deep curiosity
- Grieving youth → defended anger, structural joy (former), topsoil numbness, preconscious longing
- Reluctant leader → high competence, defended ambition, articulate duty, internal conflict: desire for freedom vs. responsibility

Each archetype generates a tensor profile with stochastic variation across axes — no two "byronic heroes" are identical, but they share structural characteristics.

**Axis 2: Relational Dynamics (dyad templates → edge configurations)**

Common relational patterns, expressed as substrate dimension configurations:

- Mentor / student → asymmetric trust (high → low, growing), projection (student idealizes), low debt
- Siblings (older / younger) → high history, moderate affection, complex trust, debt varies
- Rivals with respect → moderate trust, low affection, high projection (onto capabilities), competitive debt
- Strangers with shared grief → low history, emergent trust, high projection (recognition of shared condition)
- Authority / subject → structural trust asymmetry, institutional debt, projection varies by legitimacy
- Former friends, estranged → high history, fractured trust, defended affection, unresolved debt

**Axis 3: Scene Profiles (situation templates → constraint + affordance sets)**

Common narrative situations with defined constraints and affordances:

- Confrontation over a betrayal → high tension, constrained space, verbal affordance dominant
- Vulnerable admission of grief → low tension, intimate space, emotional affordance dominant
- Celebration interrupted by threat → shifting tension, open space, social → survival affordance transition
- First meeting with hidden agendas → moderate tension, neutral space, information affordance dominant
- Physical challenge requiring cooperation → high stakes, environmental constraint, action affordance dominant
- Quiet aftermath of violence → low tension but high weight, reduced space, reflection affordance dominant

#### Generation Process

For each cell in the matrix (character profile × relational dynamic × scene profile):

1. **Generate tensor**: Instantiate the archetype template with stochastic variation. Apply relational edge configuration. Set emotional state appropriate to scene entry.
2. **Generate intent**: Use an LLM (Ollama locally, or a cloud API for quality) to produce the structured CharacterIntent given the full feature set. The prompt template specifies the output format exactly.
3. **Validate coherence**: Programmatic checks against the generated tensor:
   - Does the predicted action align with the character's active emotional state?
   - Do emotional shifts track with the relational substrate? (A character with defended grief shouldn't suddenly become articulate about loss without a trigger.)
   - Is the awareness level of the thought consistent with the tensor's awareness-level assignments?
   - Does the action fall within the scene's affordance constraints?
   - Do geological strata show appropriate stability? (Bedrock values shouldn't shift from a single scene event. Topsoil should be responsive.)
4. **Score and filter**: Assign coherence scores. Discard incoherent examples. Flag borderline cases for human review.

#### Validation Criteria

Coherence validation operates on the floating-point feature space:

- **Emotional consistency**: The euclidean distance between the predicted emotional shift and the character's current state should fall within the variance bounds of the relevant axes. Sudden jumps flag incoherence.
- **Relational alignment**: Actions directed at another character should be consistent with the substrate dimensions of that edge. High-trust relationships afford vulnerability; low-trust relationships constrain disclosure.
- **Temporal stability**: Across a sequence of generated intents for the same character, bedrock values remain stable, sediment values shift slowly, topsoil values respond to immediate context.
- **Awareness discipline**: A character with Defended awareness of an emotion should not articulate it directly. A character with Structural awareness should not be conscious of it at all.
- **Echo detection**: Across similar scene profiles with the same character profile, the model should produce recognizably consistent patterns — the "signature" of a character type should be detectable without being mechanical.

#### Scale and Iteration

The initial matrix does not need to be exhaustive. A manageable starting set:

- 10-15 character archetype templates
- 8-10 relational dynamic templates
- 8-10 scene profile templates

This produces 640-1500 matrix cells. With 3-5 variations per cell (stochastic tensor generation), that's 2000-7500 training examples. Enough to train a first model and evaluate whether the approach produces coherent predictions.

The validation pipeline can be automated — coherence checks are quantitative. Human review focuses on the qualitative question: "does this feel like what this character would actually do?" A modest review pass over a sample set calibrates confidence.

Subsequent iterations expand the matrix, add archetype templates discovered through play, and incorporate actual play data as the system generates scenes.

---

## The Resolver

### Purpose

The Resolver replaces both the original Reconciler (which sequenced character intents) and the planned World Agent (which would have enforced world constraints via LLM). It is a deterministic rules engine that answers: **given what each character intends to do, what actually happens?**

The Resolver operates on structured data — predicted character intents, world model state, character capabilities — and produces structured outcomes. No LLM involvement.

### Responsibilities

1. **Action possibility**: Can this character do this thing? Checked against capabilities, physical constraints, spatial proximity.
2. **Action precedence**: When multiple characters act simultaneously, what happens first? Initiative ordering, interruption, preemption.
3. **Conflict resolution**: When actions conflict (two characters reach for the same object, one character tries to leave while another blocks the exit), who succeeds and to what degree?
4. **Outcome determination**: What are the consequences of each resolved action? State changes, new information revealed, emotional impact.
5. **Multi-character sequencing**: In scenes with multiple characters, structure the resolved actions into a coherent sequence that the Narrator can render.

### The World Model

The World Model is an interrogable data structure, not an agent. It encodes constraints that the Resolver queries:

**Genre physics**: Gravity is a given unless the story is set in space. Magic exists if the genre includes it, subject to authored rules. Technology level constrains available actions. These are story-level constraints, set at story creation, rarely modified during play.

**Spatial proximity**: Rather than grid-based positioning, the system uses **narrative distance zones** — affordance-based proximity that describes what kinds of interactions are possible:

| Zone | Description | Affordances |
|------|-------------|-------------|
| Intimate | Close enough to whisper, touch | All interpersonal actions |
| Conversational | Normal speech, gesture visible | Speech, gesture, shared attention |
| Awareness | Can see and hear | Observation, loud speech, movement toward |
| Peripheral | Edge of awareness | Detection, recognition at distance |
| Absent | Not in rendered space | No direct interaction |

These zones compose with the communicability gradient. A character can be physically Intimate but communicatively Absent (unconscious, dissociated). Or physically at Awareness range but communicatively Intimate (shared memory, a letter being read). The Resolver checks both spatial and communicative proximity when evaluating action possibility.

**Environmental constraints**: What the space affords and prevents. A small room constrains movement. A roaring river prevents quiet conversation. A locked door blocks passage until resolved. These are scene-level constraints, loaded at scene entry.

### Character Capabilities

The original design modeled character personality (tensor), emotion, values, and motivation — but not capability. The Resolver needs to know whether a character *can* do something, not just whether they *would*.

The capability model borrows from RPG attribute/skill systems, hidden from the player:

**Attributes**: Broad innate or earned capabilities. Not necessarily the classic six (STR/DEX/CON/INT/WIS/CHA), but a set appropriate to the story's genre and concerns. A literary fiction story might use: Physical, Perceptive, Learned, Social, Intuitive, Resilient. A fantasy adventure might use something closer to traditional RPG attributes.

**Skills**: Specific competencies tagged to one or more attributes. "Pick a lock" falls under Dexterity/Physical + a relevant skill. "Persuade a guard" falls under Social + a relevant skill. Skills are authored as part of the character sheet.

**Resolution mechanic**: When the Resolver needs to determine whether an action succeeds, it evaluates: attribute value + relevant skill + situational modifiers vs. difficulty. The result is not binary (success/failure) but graduated: full success, partial success, failure with consequence, failure with opportunity. This gives the Narrator richer material — a "partial success" produces more interesting prose than a simple pass/fail.

The player never sees the mechanics. "You strain against the door; it resists, then gives with a splintering crack and you stumble through" is a partial success rendered as narrative. The dice (or their deterministic equivalent) roll behind the curtain.

**Initiative and action order**: For multi-character scenes, the Resolver determines action order based on attributes, situational context, and narrative priority. Characters with higher relevant attributes or advantageous position act first. This ordering is invisible to the player — the Narrator renders simultaneous or sequential action as the prose demands — but it determines which actions succeed when conflicts arise.

### The `GameDesignSystem` Trait

The specific attribute set, skill taxonomy, resolution mechanic, and initiative system are genre-dependent. A story set in Appalachian folk horror has different mechanical needs than a far-future space opera. The Resolver should be parameterized:

```rust
trait GameDesignSystem {
    type Attribute;
    type Skill;

    fn resolve_action(&self, actor: &Entity, action: &Action, context: &SceneContext) -> ActionOutcome;
    fn determine_initiative(&self, actors: &[Entity], context: &SceneContext) -> Vec<InitiativeEntry>;
    fn check_possibility(&self, actor: &Entity, action: &Action, world: &WorldModel) -> Possibility;
}
```

This is an eventual destination, not an immediate implementation target. For now, building one concrete implementation that works for the Bramblehoof workshop content gives us the shape. The trait emerges from that experience.

---

## The Storykeeper

### Revised Role

In the original architecture, the Storykeeper held complete narrative state and filtered information downstream to multiple LLM agents. In the revised architecture, the Storykeeper's consumer narrows from many agents to one: the Narrator. But the assembly logic becomes more sophisticated.

The Storykeeper is now a **context assembly system**. Its responsibilities:

1. **Maintain the event ledger**: Every classified event, resolved outcome, and emotional shift is recorded. The ledger is the system's complete memory.
2. **Maintain information boundaries**: Track what has been revealed, what is known by whom, what must remain hidden. This is unchanged from the original design.
3. **Assemble Tier 2 (scene journal)**: After each turn, update the rolling scene journal with progressive compression. Determine which events resist compression based on narrative weight.
4. **Assemble Tier 3 (retrieved context)**: For each turn, determine what the Narrator needs beyond Tiers 1 and 2. Query the relational graph, event ledger, narrative graph, and information boundary model to produce structured context.
5. **Manage the information horizon**: As the scene progresses, track what the player has learned, what characters have revealed, and what thematic threads are active. Use this to bias retrieval toward contextually relevant information.

### Context Assembly as Graph Traversal

The Storykeeper's Tier 3 assembly is a graph problem. The relational web (character relationships), narrative graph (scene connectivity and gravitational landscape), and information model (what is known, by whom, when revealed) are all graph structures stored in PostgreSQL + Apache AGE.

When the classifier identifies an entity reference, emotional trigger, or thematic connection in the player's input, the Storykeeper:

1. **Identifies the query**: What does the Narrator need to know? An entity's relationships? An emotional context? A narrative echo?
2. **Traverses the relevant graph**: Follow edges in the relational web, check positions in the narrative graph, search the event ledger for matching events.
3. **Applies information boundaries**: Filter traversal results through the information model. The Narrator receives only what is currently knowable — not everything that's true.
4. **Structures the result**: Package the retrieved information as structured narrative fact with emotional annotation. Never as raw data, never as pre-rendered prose.
5. **Fits the token budget**: The Tier 3 budget is finite (~400-800 tokens). If retrieval produces more than fits, the Storykeeper ranks by relevance and truncates. Relevance considers: direct response to player input > active emotional thread > thematic echo > background context.

This is GraphRAG with narrative-specific retrieval logic. The graph provides provenance (where did this fact come from?), traversal (what is connected to what?), and boundary enforcement (what can be known?). The narrative specificity comes from the ranking: not just "most similar" but "most narratively operative."

---

## Event Classification

### Unchanged Role

The event classifier was always planned as an ML model, not an LLM. This remains the case. The classifier transforms natural language player input into typed events that the rest of the pipeline consumes.

### Expanded Responsibilities

In the single-Narrator architecture, the classifier takes on additional importance:

1. **Entity resolution**: Identify which entities the player is referencing, including indirect references ("the melody from before" → the event where Bramblehoof played, and by extension, Bramblehoof and Pyotir).
2. **Topic detection**: Identify when the player references a previously discussed topic, enabling the Storykeeper to retrieve relevant context from the event ledger.
3. **Emotional register detection**: Classify the emotional tenor of the player's input. A player who writes "I cautiously approach" vs. "I stride toward them" is communicating different emotional states that should influence character predictions and Narrator rendering.
4. **Echo detection**: Identify when the current moment rhymes with an earlier one — a repeated gesture, a returned-to location, a phrase that echoes something said before. This enables thematic resonance in the Narrator's rendering.

These classification tasks are well-suited to small, fast ML models (potentially multiple specialized classifiers rather than one monolithic model). They run before the character prediction step, enriching the feature set available to the prediction model and the Storykeeper.

---

## Implications for the Technology Stack

### What Changes

- **LLM calls**: Reduced from 3+ per turn to 1. The Narrator uses a more capable model (30B+ local, or cloud API). Token budget per call is well-bounded (~2100-3300 input, ~500-800 output).
- **ML inference (ort/ONNX)**: More central. Character prediction, event classification, echo detection — multiple small models running per turn, all fast (~50-200ms total).
- **Training pipeline**: New requirement. Need to generate, validate, and iterate on training data for the character prediction model. This is a development-time concern, not a runtime concern.
- **Rules engine**: New component. The Resolver needs structured world model data, character capability data, and resolution logic. This is deterministic Rust code, not ML.
- **Context assembly**: More sophisticated than the original Storykeeper filter. Graph traversal, event ledger search, information boundary checking, relevance ranking, token budget management. This is the Storykeeper reimagined as a RAG system with narrative-specific retrieval logic.

### What Stays

- **Bevy ECS**: Still the runtime. Scene lifecycle, entity management, event pipeline — all unchanged.
- **PostgreSQL + AGE**: Still the persistence layer. Event ledger, graph data, session state. AGE's graph traversal capabilities become more important for Tier 3 context assembly.
- **RabbitMQ**: Still the messaging layer for tasker-core integration.
- **Scene model**: Unchanged. Scenes still bound context, cast, and play.
- **Character tensors**: More important. The tensor is the primary feature source for prediction.
- **Emotional model**: The theoretical foundation for the prediction model's feature space and validation criteria.

### Estimated Turn Latency

| Stage | Time | Notes |
|-------|------|-------|
| Event classification | ~20-50ms | Small ML model(s), ort |
| Character prediction | ~50-150ms | Per character, parallel, ort |
| Action resolution | ~10-50ms | Deterministic rules engine |
| Context assembly | ~20-80ms | Graph traversal + ledger query |
| Narrator LLM call | ~2-8s | Single call, capable model |
| **Total** | **~2-8.5s** | Dominated by the one LLM call |

Compare to the original: 3+ LLM calls × 30-90s each with Mistral 7B = minutes between turns. Even with a more capable (and therefore slower) model for the Narrator, the single-call architecture is dramatically faster.

---

## Open Questions

### Training Data Scale

How much training data does the character prediction model need? The initial matrix (10-15 archetypes × 8-10 dynamics × 8-10 scenes × 3-5 variations) produces 2000-7500 examples. Is this enough for a first model? What architecture (small transformer, MLP, gradient-boosted trees on structured features) is appropriate for this prediction task?

### Narrator Model Selection

What model serves the Narrator best? The task is: given structured context (~2500 tokens), produce literary prose (~500-800 tokens) in a specific voice. This favors models with strong instruction-following and creative writing capability. Local options (Llama 3 70B quantized, Mixtral 8x22B) vs. cloud APIs (Claude, GPT-4) — different latency/quality/cost tradeoffs.

### Scene Journal Compression

The progressive compression rules (recency, narrative weight, redundancy, emotional state) need calibration. How aggressively should older turns compress? What constitutes "narrative weight" in quantitative terms? This likely requires experimentation with real scene data.

### Resolver Complexity

How rich does the capability model need to be for the initial implementation? The Bramblehoof workshop content features two characters in a quiet encounter — the Resolver barely needs to resolve anything. A scene with combat, environmental hazards, or multi-party negotiation would stress the Resolver much more. Build for the simple case first, design the trait for the general case.

### Echo Detection

Thematic echo detection — recognizing when the current moment rhymes with an earlier one — is the most ambitious classifier task. How much of this can be done with structured feature matching (same entities, similar emotional states, same location) vs. requiring semantic understanding? This may be an area where a small LLM classifier is appropriate, trading speed for capability.

### Authorial Voice Preservation

How much of the Narrator's voice comes from the system prompt vs. the model's own tendencies? A system prompt can establish tone and constraints, but the actual prose quality depends on the model. Different models produce different voices even with identical prompts. How much voice variation across models is acceptable?

---

## Relationship to the Original Vision

This revision does not abandon the original system architecture — it refines the computational distribution. The core insights remain:

- **Imperfect information by design**: No single component has complete knowledge. Information boundaries are still the defining architectural choice.
- **Scene as unit of play**: Scenes still bound context, cast, and meaningful action.
- **Narrative gravity**: The gravitational landscape, narrative mass, approach vectors — all unchanged.
- **Character as tensor**: The tensor representation becomes more important, not less, as the primary feature source for prediction.
- **Communicability gradient**: Still applies to entities, self-edges, spatial proximity, and the Narrator's own awareness.
- **Command sourcing**: Player input persisted before processing. Recovery via checkpoint + ledger replay.

What changes is *who does what*. The LLM was asked to do everything: maintain persona, generate behavior, synthesize prose, enforce constraints. Now each capability is handled by the component best suited to it: ML models predict, rules engines resolve, graph queries retrieve, and the LLM — freed from persona maintenance and constraint enforcement — does what it does best: tell a story.

The Narrator, given the right materials, has room to create. That is the architectural bet.
