# Event Classification and Action Arbitration Design

**Date**: March 9, 2026
**Branch**: `jcoletaylor/event-classification-and-action-arbitration`
**Context**: Workshop UI is playable. DistilBERT event classifier hits its ceiling on natural prose. Zone 1 infrastructure (event grammar, entity model, turn cycle) needs to be wired up with reliable entity→action→entity decomposition before the event ledger writer (TAS-246) can produce meaningful data.

## The Problem

The current event classification pipeline uses two DistilBERT ONNX models: one for event kind classification (multi-label, 8 kinds) and one for NER entity extraction (BIO tagging, 7 categories). Both were trained on 8,000 combinatorially generated templates and achieve perfect F1 on templated data.

In practice, the classifier only recognizes events when player input closely matches template patterns — "I say to Pyotir" triggers SpeechAct, but natural prose like "the child laughs at the small sprite in the corner" fails to decompose into a meaningful entity→action→entity triple. The fundamental issue: **semantic role labeling in narrative prose is a language understanding task, not a classification task**. DistilBERT can identify *that* entities and event kinds are present but cannot determine *who did what to whom* — the relational direction that gives events meaning.

Additionally, the system lacks any mechanism for action arbitration — determining whether a player's stated action is physically or narratively possible given the world's constraints.

## Decisions

### Decision 1: Small LLM for Event Decomposition

**Replace DistilBERT as the authoritative event decomposition system with a small instruction-tuned LLM** (Qwen2.5:3b-instruct primary, with 7B available for complex cases).

**Why not more DistilBERT training data?** The ceiling is task complexity, not data volume. Understanding that "laughs at" carries a directed relational vector from child→sprite requires compositional language understanding — prepositional semantics, coreference, implied agency. These are capabilities DistilBERT structurally lacks regardless of training scale. 50K or 100K templates would cover more phrasings but literary prose is combinatorially infinite.

**Why a small LLM works here:** Event decomposition is an extraction task with structured output, not a generation task. The LLM receives narrative text and produces JSON triples. Small models (3B-7B) excel at this with constrained decoding — they understand prepositions, agency, and direction out of the box. No fine-tuning needed; prompt engineering with a JSON schema is sufficient.

**DistilBERT retained for D.2 fast pass.** The existing DistilBERT models continue to provide fast (~50ms) entity mention detection and event kind features during the `Classifying` stage (Phase D.2). These features feed character prediction — a role where ~80% precision is acceptable because character predictions are probabilistic anyway. DistilBERT may be retrained later for better D.2 precision, but this is not urgent.

### Decision 2: Rules Engine with LLM Fallback for Action Arbitration

**Extend the existing `WorldModel` + `GameDesignSystem` with typed constraints and a three-tier possibility check: deterministic rules for clear cases, small LLM fallback for ambiguous cases.**

**Why not narrator-handles-everything?** The narrator LLM handles soft constraints well with good prompting — a character "cautiously approaches" vs "strides forward" is narrative judgment the narrator is good at. But hard constraints (gravity, locked doors, capability requirements) and ambiguous constraints (can you leap a building in 0.3g in a spacesuit while startled?) need system-level enforcement. Relying on the narrator to notice constraint violations puts the fox in charge of the henhouse.

**Why not pure rules engine?** Interesting narrative situations always find the boundary between possible and impossible. "I dive for the rapier, roll across the floor, and slash the weapon from his hand" involves swordsmanship capability, spatial proximity, action sequencing, and physical plausibility — too many interacting factors for a static rule set. The LLM fallback handles genuine edge cases where rules are insufficient.

### Decision 3: One-Turn Delay for Event Decomposition is Acceptable

Events are aggregate signals, not immediate triggers. Rudeness doesn't make an enemy; patterns of disrespect do. A punch changes trust significantly but the relationship update doesn't need to be synchronous with the narrator's prose rendering. The narrator LLM already has enough context from the current turn to render appropriate prose — if someone punches someone, the narrator will narrate it well regardless of whether the event classifier has formally decomposed the action.

A one-turn delay may even *help* narrative quality by preventing the narrator from responding with too-stark a cadence to relational changes, allowing creative latitude.

### Decision 4: Capability Lexicon Pre-seeding

Player input uses natural language that rarely matches authored capability names directly. "Dive for the rapier, roll, slash" implies swordsmanship without ever saying "wield" or "sword fighting." Embedding similarity is too fuzzy for multi-hop inference ("rapier" → "swordsmanship" is multiple conceptual hops).

**Solution:** At story authoring time, expand each authored capability into a curated lexicon of synonyms, action verbs, implied objects, and idiomatic phrases using LLM-assisted generation with author review. At runtime, capability matching is fast string/token lookup against pre-computed sets — no ML or embedding search needed.

---

## Architecture

### Component 1: Structured LLM Provider

A new provider type, distinct from the narrator's `LlmProvider`, optimized for fast structured extraction.

```rust
/// Provider for fast, structured-output LLM calls used in event
/// decomposition and action arbitration. Distinct from the narrator's
/// LlmProvider — different model, different purpose, different latency.
pub trait StructuredLlmProvider: Debug + Send + Sync {
    fn extract(&self, request: StructuredRequest) -> StorytellerResult<serde_json::Value>;
}

pub struct StructuredRequest {
    /// System prompt establishing the extraction task
    pub system: String,
    /// The text to analyze
    pub input: String,
    /// JSON schema the output must conform to
    pub output_schema: serde_json::Value,
    /// Temperature (low — extraction, not creativity)
    pub temperature: f32, // default 0.1
}

pub struct StructuredLlmConfig {
    /// Ollama endpoint (reuses OLLAMA_BASE_URL, defaults to 127.0.0.1:11434)
    pub base_url: String,
    /// Model name — distinct from narrator model
    pub model: String,  // e.g., "qwen2.5:3b-instruct"
    /// Temperature for extraction tasks
    pub temperature: f32,  // default 0.1
    /// Timeout — fast calls
    pub timeout: Duration,  // default 10s
}

/// Bevy Resource wrapping the structured LLM provider
#[derive(Resource)]
pub struct StructuredLlmResource(pub Arc<dyn StructuredLlmProvider>);
```

Model selection (in preference order):
- **Qwen2.5:3b-instruct** — strong structured output, fast
- **Qwen2.5:7b** — available if 3B lacks accuracy on complex decompositions

Both event decomposition and action arbitration share this provider. They never run simultaneously (D.3 processes the previous turn while arbitration runs in the current turn), so a single Ollama model instance handles both without contention.

### Component 2: Event Decomposition

#### Extraction Schema

The small LLM receives combined narrator+player text (committed turn) and produces structured decomposition:

```json
{
  "events": [
    {
      "kind": "EmotionalExpression",
      "actor": { "mention": "the child", "category": "CHARACTER" },
      "action": "laughs at",
      "target": { "mention": "the small sprite", "category": "CHARACTER" },
      "relational_direction": "directed",
      "confidence_note": "clear directed emotional expression via 'at' preposition"
    }
  ],
  "entities": [
    { "mention": "the child", "category": "CHARACTER" },
    { "mention": "the small sprite", "category": "CHARACTER" },
    { "mention": "the corner", "category": "LOCATION" }
  ]
}
```

#### Relational Direction Enum

A new enum capturing the directionality of events:

```rust
pub enum RelationalDirection {
    /// Actor acts upon target: "laughs at", "strikes", "tells"
    Directed,
    /// Both parties involved mutually: "embrace", "argue", "negotiate"
    Mutual,
    /// Action directed at self: laughing alone, sighing, internal realization
    SelfDirected,
    /// Action directed at situation/world, not a specific entity:
    /// joyful laughter in a field, despair at circumstance
    Diffuse,
}
```

This directly solves the original design question: when someone walks in a field and laughs aloud from joy, the event is `SelfDirected` or `Diffuse` — not an error or missing target. The system can layer contextual meaning (insight, despair, wonder) onto diffuse events as self-loopback entries in the relational substrate.

#### System Prompt Pattern

```
You are an event extractor for interactive fiction. Given narrative text,
identify discrete events as entity→action→entity triples.

Rules:
- Every event needs at minimum an actor and an action
- A target is required for directed actions, optional for self/diffuse
- Use entity categories: CHARACTER, OBJECT, LOCATION, GESTURE, SENSORY, ABSTRACT, COLLECTIVE
- Use event kinds: StateAssertion, ActionOccurrence, SpatialChange, EmotionalExpression,
  InformationTransfer, SpeechAct, RelationalShift, EnvironmentalChange
- When a character acts without a clear target, set relational_direction to "self"
- When an action affects the general situation, set relational_direction to "diffuse"
- Extract ALL entities mentioned, even those not in events

Respond with JSON matching this schema: {schema}
```

#### Pipeline Integration

Event decomposition runs in the committed-turn classification phase (D.3):

```
Turn N:
  [D.2] DistilBERT: fast entity/kind detection → feeds character prediction
  [Narrator renders Turn N]

  Async background task (spawned after narrator renders):
    [D.3] Small LLM: deep decomposition of Turn N combined text (~500ms-1s)
    → EventAtom construction with proper actor/target/direction
    → Results stored for Turn N+1's commit phase

Turn N+1:
  [Commit] Turn N's decomposed events available for ledger/relational updates
  [D.2] DistilBERT: fast pass on new input
  [Continue pipeline]
```

#### Output Contract

The LLM extraction output maps to the existing `build_event_atoms()` pipeline. Key difference: `EventAtom` construction now receives explicit actor/target assignments from the LLM rather than heuristic role assignment. The `assign_participant_roles()` heuristic (~80% accuracy) becomes the fallback when the LLM is unavailable, not the primary path.

### Component 3: Action Arbitration

#### Action Possibility Enum

```rust
pub enum ActionPossibility {
    /// Rules engine is confident: action is permitted
    Permitted {
        conditions: Vec<ActionCondition>,
    },
    /// Rules engine is confident: action is impossible
    Impossible {
        reason: ConstraintViolation,
    },
    /// Rules engine cannot determine — needs deeper analysis
    Ambiguous {
        known_constraints: Vec<EnvironmentalConstraint>,
        uncertainty: String,
    },
}
```

#### Typed Genre Constraints

Replace `genre_physics: Vec<String>` with typed constraints:

```rust
pub enum GenreConstraint {
    /// A capability that does not exist in this world
    Forbidden { capability: String, reason: String },
    /// A capability that exists with conditions
    Conditional { capability: String, requires: Vec<String> },
    /// A physical law override (e.g., low gravity, no sound in vacuum)
    PhysicsOverride { property: String, value: String },
}
```

#### Deterministic Check Order

The rules engine evaluates in order:

1. **Genre physics** — hard constraints from `GenreConstraint`. Magic forbidden? Telekinesis is `Impossible`.
2. **Spatial zones** — existing `NarrativeDistanceZone` checks. Can't whisper at `Peripheral` distance.
3. **Capability check** — character's `CapabilityProfile` supports the action? Checked via capability lexicon matching (see below).
4. **Environmental constraints** — existing `EnvironmentalConstraint`. Locked door blocks passage.

All pass cleanly → `Permitted`. Any hard check fails → `Impossible`. Inconclusive → `Ambiguous`.

#### LLM Fallback for Ambiguous Cases

When the rules engine returns `Ambiguous`, the small LLM receives structured context:

```json
{
  "action": "leap to the top of the building",
  "actor": { "name": "Renko", "relevant_capabilities": ["athletics: 0.6", "spacesuit: equipped"] },
  "scene": { "gravity": "0.3g", "environment": "hull exterior, vacuum" },
  "known_constraints": ["wearing magnetic boots", "holding welding torch", "startled by xenomorph"],
  "question": "Is this action physically possible given the constraints? What conditions apply?"
}
```

LLM returns:

```json
{
  "ruling": "permitted_with_conditions",
  "conditions": [
    "Must release or drop welding torch first",
    "Magnetic boots must be disengaged",
    "Jump height plausible in 0.3g but landing accuracy reduced by panic"
  ],
  "graduated_outcome": "partial_success_likely",
  "narrative_note": "The leap is possible but messy — a scramble, not a graceful bound"
}
```

Ruling enum values: `permitted`, `permitted_with_conditions`, `impossible`, `implausible_but_narratively_possible`. The last value is an escape valve for extraordinary moments — physics says no, but stories sometimes require characters to exceed their limits.

#### Integration with Turn Cycle

Arbitration slots before the existing `GameDesignSystem::resolve()`:

```
Player Input → D.2 Classification → Character Prediction
    ↓
Action Arbitration Check
    ├── Permitted → resolve() as normal
    ├── Permitted w/ conditions → resolve() with conditions injected
    ├── Impossible → skip resolve(), inject constraint into narrator context
    └── Ambiguous → Small LLM → ruling feeds into one of the above
    ↓
Context Assembly → Narrator
```

When an action is `Impossible`, the narrator receives structured context about *why* so it can narrate the constraint naturally: "You reach for the staff, but it remains rooted — whatever force holds it there answers to no will but its own."

### Component 4: Capability Lexicon Pre-seeding

#### The Problem

Player input uses natural language that doesn't match authored capability names. "I dive for the rapier, roll across the floor, and slash the weapon from his hand" implies swordsmanship without ever saying "wield" or "sword fighting." This is multi-hop inference: rapier → weapon → swordsmanship. Embedding similarity is too fuzzy for these hops.

#### Lexicon Structure

```rust
pub struct CapabilityLexicon {
    entries: BTreeMap<String, LexiconEntry>,
}

pub struct LexiconEntry {
    /// The authored capability name
    capability: String,
    /// Direct synonyms: "swordsmanship" → ["fencing", "blade work", "sword fighting"]
    synonyms: Vec<String>,
    /// Action verbs: "swordsmanship" → ["slash", "parry", "thrust", "lunge", "riposte"]
    action_verbs: Vec<String>,
    /// Implied objects: "swordsmanship" → ["rapier", "sword", "blade", "saber", "foil"]
    implied_objects: Vec<String>,
    /// Multi-hop phrases: "swordsmanship" → ["dove for the blade", "steel rang", "crossed swords"]
    idiomatic_phrases: Vec<String>,
}
```

#### Generation Process

At story authoring time (not runtime):

1. Author defines capabilities in the `GameDesignSystem`
2. Each capability is fed to an LLM with genre context: *"For a low-fantasy medieval setting, generate synonyms, action verbs, implied objects, and idiomatic phrases for 'swordsmanship'"*
3. Author reviews, curates, and ships the lexicon as static data alongside the story definition

#### Runtime Matching

Capability matching at runtime is fast string/token lookup against pre-computed sets — no ML, no embeddings, no inference. The lexicon does the inferential work at authoring time so the runtime doesn't have to.

---

## Graceful Degradation

The system operates at whatever capability level is available:

| Available | Event Decomposition | Action Arbitration |
|-----------|--------------------|--------------------|
| Small LLM + DistilBERT | Full: LLM decomposition (D.3) + DistilBERT fast pass (D.2) | Full: rules engine + LLM fallback for ambiguous |
| DistilBERT only | Heuristic: DistilBERT for both D.2 and D.3 (current behavior) | Rules only: `Ambiguous` defaults to `Permitted` |
| Neither | Keyword fallback, no event extraction | Rules only |

The workshop remains fully playable without the structured LLM running — just with less precise event tracking and permissive arbitration.

---

## What We Build vs. What We Defer

### Build Now

- `StructuredLlmProvider` trait + Ollama implementation
- Event decomposition extraction schema, system prompt, and D.3 integration
- `RelationalDirection` enum
- `ActionPossibility` enum and arbitration contract
- Typed `GenreConstraint` replacing `Vec<String>` genre physics
- `CapabilityLexicon` structure and matching logic
- Integration points in the turn cycle
- Graceful degradation at every level
- Workshop debug panel extensions (structured LLM health check)

### Defer

- Event ledger writer (TAS-246) — separate bounded actor, consumes EventAtoms produced by this pipeline
- Rich `GameDesignSystem` implementations per story/genre
- `implausible_but_narratively_possible` path (requires Storykeeper input on narrative weight)
- DistilBERT retraining for improved D.2 precision
- Capability lexicon authoring tooling (LLM-assisted generation UI)
- Per-story capability lexicon curation workflow

---

## Relationship to Zone Assessment

This design addresses a gap identified in the zone assessment (`docs/technical/roadmap/2026-03-02-zone-assessment.md`): Zone 1 infrastructure (event grammar, entity model, turn cycle) is grounded and tested but not wired up into the workshop. This design connects the event grammar and entity identification to the live pipeline through reliable extraction, and establishes the contract for action arbitration that the World Agent concept requires.

The event ledger writer (TAS-246) becomes more tractable after this work — it receives well-structured `EventAtom` data with proper entity→action→entity triples and relational direction, rather than heuristically assigned participant roles. The ledger writes become meaningful because the events they record are semantically accurate.

## Relationship to Narrator Architecture

This design implements two components described in `docs/technical/narrator-architecture.md`:

1. **Event Classification — Expanded Responsibilities** (§Event Classification): Entity resolution, topic detection, emotional register detection via small LLM structured extraction rather than multiple specialized ML classifiers.
2. **The Resolver — Action Possibility** (§The Resolver): The arbitration check implements the "can this character do this thing?" question with the deterministic rules + LLM fallback architecture.

The `GameDesignSystem` trait and `WorldModel` already exist in code. This design extends them with typed constraints and integrates them into the turn cycle at the right point.
