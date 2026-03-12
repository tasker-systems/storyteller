# Scene Goals and Character Intentions

## Status: Design

## Context

The narrator produces strong prose and characters now show emergent behavioral richness (e.g., Arthur physically resisting a directive to rise — an unauthored moment that emerged from data-driven context assembly). However, playtesting across multiple scenes reveals a consistent pattern: the narrator starts strong (Turns 1-3) with clear emotional direction, then drifts into repetitive circling by Turns 5-7. Characters amplify existing beats rather than developing new ones. The root cause is structural — the narrator has no goal to move toward.

ML predictions tell the narrator *how* a character behaves (speech register, action type, emotional deltas) but not *what they're trying to accomplish*. Intent synthesis gives per-turn behavioral directives, but these lag behind or echo what the player already initiated. The narrator — like any capable mind — does its best work when it has a specific problem to solve. A character with a hidden letter and a reason to reach the mantel generates richer prose than any number of atmospheric fragments.

This spec introduces **scene goals** (dramaturgical direction) and **character intentions** (concrete per-character objectives) as a new layer in the scene composition pipeline. This is the playtest proof of concept for what the information threshold needs to be when narrative gravity concepts are applied more structurally.

### Relationship to Narrative Gravity

Scene goals and character intentions are a permanent architectural layer, not a stepping stone. Narrative gravity will become an additional *producer* of goals alongside authored descriptors. The design ensures both producers feed the same interface:

- **Authored goals** (this spec): Goal vocabulary in descriptors → set intersection → composition-time LLM generation → concrete intentions
- **Gravity-derived goals** (future): Gravitational calculations on scene exit/entry → high-mass downstream nodes contribute trajectory hints and transitional lexicon → same composition-time LLM generation → concrete intentions

The narrator receives the same shape of input regardless of whether the goal was authored or emergent. The lexicon enrichment pipeline is the common currency between both producers.

### Relationship to Event Evaluation

The four-part character intention structure (objective, constraint, behavioral_stance) and scene-level trajectory give the event classification pipeline formal structures to match against in future work. When the event system can detect "Arthur reached the mantel" as objective progress or "Margaret found the letter" as a trajectory climax, scene-aware pacing becomes possible. This spec builds the intention structures; event evaluation against them is a follow-up.

### Relationship to Connective-Tissue Scene Generation

The composition-time LLM generation step built here is the same mechanic that will later run as a background job generating intentions for unauthored connective-tissue scenes in the sparse narrative graph. Same inputs (goals + dimensional context + lexicon), same output shape (concrete situational intentions), different trigger (scene setup vs. background graph worker).

## Architecture

### Design Principle: Seed Agency Through Concrete Problems

The narrator produces emergent, alive-feeling prose when it has specific tensions to navigate — physical objects, spatial relationships, deceptions to maintain, interpersonal frictions. Abstract atmospheric guidance ("the scene feels heavy with unspoken grief") produces drift. Concrete situational intentions ("Arthur has hidden a letter in the tea caddy and needs to reach the mantel without Margaret noticing") produce agency.

The goal system's primary job is to produce concrete situational intentions per character that give the narrator problems to solve. Everything else — vocabulary, set intersection, lexicons — is machinery in service of that.

### Pipeline Overview

```
Author Time (build-time tooling):
  Goal vocabulary (goals.json)
    + Descriptor tagging (profiles, archetypes, dynamics reference goal IDs)
    → LLM enrichment (qwen 32b/14b)
    → Behavioral lexicons per goal × dimensional intersection
    → Persisted in storyteller-data

Scene Setup (composition time):
  Genre + Profile + Cast + Dynamics (from SceneSetup wizard)
    → Pass 1: Scene goal intersection (profile ∩ cast archetypes)
    → Pass 2: Per-character goal intersection (archetype ∩ dynamics - blocked)
    → Coherence filter (character goals compatible with scene goals)
    → Likeness pass (select lexicon fragments via dimensional + tensor affinity)
    → Composition-time LLM generation (qwen 32b/14b, single call)
    → Concrete situational intentions (scene + per-character)
    → Injected into narrator preamble
    → Persisted in session directory

Per Turn (no change to existing pipeline):
  ML predictions → intent synthesis → context assembly → narrator
  (narrator now has goal-directed compass from preamble)
```

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    BUILD TIME                            │
│                                                         │
│  goals.json ──┐                                         │
│  profiles ────┤                                         │
│  archetypes ──┼──→ qwen 32b/14b ──→ behavioral lexicons │
│  dynamics ────┤        per goal × dimensional intersection│
│  genres ──────┘        (persisted in storyteller-data)   │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                  COMPOSITION TIME                        │
│                                                         │
│  Scene Selections ──→ Two-Pass Set Intersection         │
│       │                    │                            │
│       │              Active Goals                       │
│       │                    │                            │
│       │              Likeness Pass ◄── Lexicons         │
│       │                    │                            │
│       │              Selected Fragments                 │
│       │                    │                            │
│       ▼                    ▼                            │
│  Full Scene Context + Fragments ──→ qwen 32b/14b       │
│                                        │                │
│                                  Situational            │
│                                  Intentions             │
│                                        │                │
│                          ┌─────────────┼──────────┐     │
│                          ▼             ▼          ▼     │
│                     Preamble      Session     Workshop  │
│                     Injection     Persist     UI        │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                     PER TURN                            │
│                                                         │
│  Player Input ──→ ML Predictions ──→ Intent Synthesis   │
│       │                                    │            │
│       │           Narrator Preamble ◄── Scene Direction │
│       │           (includes drives)    + Character Drives│
│       │                │                                │
│       └──────────► Context Assembly ──→ Narrator LLM    │
│                                            │            │
│                                      Narrator Output    │
│                                   (goal-directed prose) │
└─────────────────────────────────────────────────────────┘
```

## Goal Vocabulary

### Descriptor: goals.json

A new descriptor file in `storyteller-data/training-data/descriptors/`:

```json
{
  "goals": [
    {
      "id": "confront_shared_loss",
      "description": "Characters face a loss they both carry, moving toward acknowledgment",
      "category": "revelation",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    }
  ]
}
```

**Fields:**

- **id**: Stable identifier for set operations and cross-referencing
- **description**: Human-readable authorial intent (for authors and the enrichment LLM, not the narrator directly)
- **category**: Grouping for coherence filtering — `revelation`, `relational_shift`, `confrontation`, `discovery`, `bonding`, `departure`, `transaction`, `protection`. Not used in set intersection, only in the coherence check between scene and character goals.
- **visibility**: How the goal manifests to the player — `Overt` (explicitly stated), `Signaled` (behavioral clues), `Hidden` (subtext only), `Structural` (scene architecture, invisible). Shapes both lexicon generation and how the narrator renders goal-directed behavior.
- **valence**: Emotional weight — `heavy`, `light`, `tense`, `warm`, `ambiguous`. Used as a dimension in the likeness pass.
- **lexicon**: Populated by build-time enrichment pipeline. Empty in the authored source. Array of `LexiconEntry` objects (see Lexicon Enrichment section).

### Descriptor Tagging

Existing descriptors gain goal references:

**profiles.json** — what this scene type is *for*:
```json
{
  "id": "quiet_reunion",
  "scene_goals": ["confront_shared_loss", "approach_threshold_of_trust"]
}
```

**archetypes.json** — what this character type *can want*:
```json
{
  "id": "stoic_survivor",
  "pursuable_goals": ["protect_secret", "resist_comfort", "test_the_room", "honor_obligation"]
}
```

**dynamics.json** — what this relationship enables or prevents:
```json
{
  "id": "strangers_in_shared_grief",
  "enabled_goals": ["mutual_confession", "confront_shared_loss", "approach_threshold_of_trust"],
  "blocked_goals": ["betray_trust", "negotiate_trade_terms"]
}
```

Genre does not reference goals directly. Its existing `valid_profiles`, `valid_archetypes`, and `valid_dynamics` constraints implicitly bound the goal space.

### Initial Vocabulary

Target: 25-35 named goals across categories. Enough to meaningfully cover the three current genres without exhaustive enumeration. The LLM enrichment means each goal generates rich behavioral vocabulary across its dimensional intersections.

Example goals by category:

| Category | Example Goals |
|----------|--------------|
| Revelation | `confront_shared_loss`, `reveal_hidden_truth`, `discover_secret_identity` |
| Relational Shift | `approach_threshold_of_trust`, `test_loyalty`, `forgive_old_wound` |
| Confrontation | `challenge_authority`, `demand_accountability`, `defend_against_accusation` |
| Discovery | `investigate_anomaly`, `piece_together_clues`, `uncover_history` |
| Bonding | `offer_shelter`, `share_vulnerability`, `find_common_ground` |
| Departure | `prepare_to_leave`, `say_what_must_be_said`, `resist_farewell` |
| Protection | `protect_secret`, `shield_someone_vulnerable`, `maintain_deception` |

## Two-Pass Set Intersection

### Pass 1 — Scene Dramaturgical Goals

```
scene_goals = profile.scene_goals
              ∩ (union of all cast archetypes' pursuable_goals)
```

The profile declares what the scene is *for*. A scene goal survives only if at least one cast member can pursue it. This is a soft intersection: the profile proposes, the cast filters.

If a profile has `scene_goals: ["confront_shared_loss", "approach_threshold_of_trust"]` and the cast's combined `pursuable_goals` include `confront_shared_loss` but not `approach_threshold_of_trust`, only `confront_shared_loss` survives as a scene goal.

### Pass 2 — Per-Character Objectives

For each character:

```
character_goals = archetype.pursuable_goals
                  ∩ (union of dynamics.enabled_goals
                     for all dynamics involving this character)
                  - (union of dynamics.blocked_goals
                     for all dynamics involving this character)
```

Then a coherence filter against the scene goals:

```
character_goals = character_goals
                  where goal.category is affine
                  with any scene_goal.category
```

### Coherence / Affinity Check

Two goals are coherent if they share a category, *or* if their categories are compatible. A category affinity table defines which pairs work together:

| Category A | Compatible With |
|-----------|-----------------|
| revelation | protection, relational_shift, discovery |
| relational_shift | revelation, bonding, confrontation, departure |
| confrontation | relational_shift, protection, revelation |
| discovery | revelation, bonding, protection |
| bonding | relational_shift, discovery, departure |
| departure | relational_shift, bonding, revelation |
| protection | revelation, confrontation, discovery |
| transaction | transaction, confrontation |

This prevents moon-base-in-the-library incoherence while preserving productive tension. A character pursuing `protect_secret` (protection) is coherent with a scene goal of `confront_shared_loss` (revelation) — that's dramatic friction. A character pursuing `negotiate_trade_terms` (transaction) in a grief scene would be filtered out.

### Empty Set Behavior

If intersection produces zero scene goals, that is a valid state. The narrator operates as it does today — no goal-directed guidance, no failure. Goals enrich when present; their absence isn't an error. This keeps the path clean for gravity-derived goals to fill gaps that authored goals don't cover.

## Build-Time Lexicon Enrichment

### Pipeline

A Python tool in `tools/training/` (alongside existing training pipelines) that generates behavioral lexicons for each goal across its relevant dimensional intersections.

**For each goal, the LLM receives:**

- The goal's id, description, category, visibility, valence
- All profiles that reference it (characteristic_events, tension ranges, scene_types)
- All archetypes that can pursue it (personality axes, emotional profiles, action tendencies)
- All dynamics that enable it (role descriptions, relational edges)

**The LLM generates behavioral lexicon entries** — not atmospheric fragments, but descriptions of *what pursuing this goal looks like* in a specific dimensional context:

```json
{
  "goal_id": "protect_secret",
  "entries": [
    {
      "fragment": "small deflections dressed as courtesy — offering to help in the kitchen, finding reasons for others to sit rather than stand near the thing he's guarding",
      "register": "character_signal",
      "dimensional_context": {
        "archetypes": ["stoic_survivor"],
        "profiles": ["quiet_reunion"],
        "dynamics": ["strangers_in_shared_grief"],
        "valence": ["heavy"]
      }
    },
    {
      "fragment": "a need to be near something without seeming to need it, manufacturing proximity through small domestic tasks",
      "register": "character_signal",
      "dimensional_context": {
        "archetypes": ["stoic_survivor", "reluctant_leader"],
        "profiles": null,
        "dynamics": null,
        "valence": ["heavy", "tense"]
      }
    },
    {
      "fragment": "the conversation keeps circling back to certain topics — he steers it away each time, not forcefully but with a practiced change of subject that feels almost natural",
      "register": "character_signal",
      "dimensional_context": {
        "archetypes": null,
        "profiles": ["quiet_reunion", "vulnerable_admission"],
        "dynamics": null,
        "valence": ["tense", "ambiguous"]
      }
    }
  ]
}
```

### Fragment Registers

Three registers that serve different roles in the composition-time LLM prompt:

- **`character_signal`**: What pursuing this goal *looks like* — observable behavior, speech patterns, physical tells, relational moves. The primary register.
- **`atmospheric`**: What the goal does to the scene's texture — how the room feels when someone is guarding a secret, the quality of silence when confession is near.
- **`transitional`**: What could happen next if the goal progresses or fails — forward-looking behavioral possibilities. These are the natural insertion point for gravity-derived foreshadowing in the future.

### Dimensional Context

`null` is a wildcard — a fragment with `"archetypes": null` applies to any archetype pursuing this goal. This prevents over-fitting while allowing precision where it matters.

### Generation Volume

Each goal produces 15-30 fragments across registers and dimensional intersections. With 25-35 goals, total lexicon size is roughly 500-900 entries. Generated once at build time, persisted in `storyteller-data` alongside descriptors. Regenerate when goals or descriptors change.

Generation runs per-goal (not one massive prompt), with relevant descriptor context injected. This keeps the LLM focused and the output quality high.

## Likeness Pass and Fragment Selection

At composition time, after set intersection determines active goals, the likeness pass scores lexicon fragments against the specific scene context.

### Step 1 — Dimensional Match

Lookup-based filter. Each fragment's `dimensional_context` is compared against the current scene's specific genre, profile, archetypes, and dynamics. Fragments whose context matches score highest. Partial matches (fewer dimensions specified) score proportionally.

### Step 2 — Tensor Affinity Scoring

Character-specific refinement. Each character was instantiated with sampled tensor values from their archetype ranges. For character goal fragments, score against the character's tensor profile using a weighted sum over the character's top 3-5 activated tensor axes compared against the dimensional context the fragment was generated in.

Fragments generated in high-`distance_management` contexts score higher for a character with high `distance_management`. This pulls fragment selection toward behavioral language that fits the *specific instantiation* of the character, not just the archetype template.

This is a simple heuristic, not a neural similarity metric. Weighted sum is sufficient for this branch; more sophisticated scoring can be introduced if needed.

### Step 3 — Diversity Sampling

Sample with bias toward higher-scoring fragments but with enough entropy to avoid repetition across sessions. Two playthroughs with the same scene setup should select different fragments, producing different composition-time LLM inputs and therefore different concrete intentions.

**Selection budget per active goal:**

- 2-3 `character_signal` fragments
- 1-2 `atmospheric` fragments
- 0-1 `transitional` fragments

A scene with 2 scene goals and 2-3 character goals per NPC produces roughly 20-30 selected fragments total — enough material for the composition-time LLM without flooding its context.

## Composition-Time Intention Generation

### The Core Step

A single LLM call (qwen 32b or 14b via Ollama) at scene setup that transforms active goals + selected lexicon fragments + full scene context into concrete situational intentions.

### Input Prompt Structure

```
Scene Context:
- Genre: [genre description]
- Setting: [setting name, description, affordances, sensory palette]
- Profile: [profile description, tension range, characteristic events]

Scene Goals (from intersection):
- [goal_id] ([visibility]): [description]

Cast and Character Goals (from intersection):
- [Name] ([archetype]): [goal_ids]
  Dynamics: [dynamic descriptions for this character's relationships]

Behavioral Vocabulary (selected fragments):
- [goal_id × dimensional context]:
  "[fragment 1]"
  "[fragment 2]"
  ...

Generate concrete situational intentions for this scene. Each character needs
a specific objective grounded in the physical setting — objects, locations,
spatial relationships. Objectives should create inter-character tension where
characters' pursuits naturally complicate each other.
```

### Output Structure

```json
{
  "scene_intention": {
    "dramatic_tension": "A 1-3 sentence description of what this scene is
      actually about — the specific situation, not the abstract theme.",
    "trajectory": "Where the scene is headed — the moment it's building toward,
      the choice or revelation that would mark its climax."
  },
  "character_intentions": [
    {
      "character": "Arthur",
      "objective": "What Arthur is concretely trying to do — grounded in
        physical objects, spatial relationships, specific actions.",
      "constraint": "What makes it hard — typically another character's
        natural behavior creating unintentional obstacles.",
      "behavioral_stance": "How he pursues the objective — his manner,
        his tactics, what it looks like from the outside."
    }
  ]
}
```

### The Four-Part Character Intention

- **objective**: The concrete thing the character is trying to do. Must reference physical scene elements (objects, locations, spatial relationships). This is the *problem* the narrator solves.
- **constraint**: What makes the objective hard. Usually another character's natural behavior or the character's own nature working against them. This is the source of dramatic tension.
- **behavioral_stance**: How the character pursues the objective — bridges to ML predictions by describing manner and tactics. The ML predictions provide the "how and when"; the behavioral stance provides the "toward what."
- Scene-level **dramatic_tension** and **trajectory** give the narrator the dramaturgical compass — what the scene is about and where it's headed.

### Validation

A post-processing check verifies that generated intentions reference objects and locations that exist in the scene's setting description and affordances. If the LLM invents elements not in the setting, the check flags it. Given that setting affordances are in the prompt, this should be rare but is worth catching.

### Latency

Single LLM call at scene setup (not per-turn). Using the same Ollama infrastructure as intent synthesis. Acceptable latency for a one-time setup cost, particularly given the richness it produces. This is the same mechanic that will later run as background jobs for connective-tissue scene generation.

## Preamble Integration

### Narrator Preamble Structure (Updated)

```
## Your Voice
[narrator_identity — existing]

## Never Do
[anti_patterns — existing]

## The Scene
[setting_description — existing]

## Cast
[cast_descriptions — existing]

## Scene Direction
[scene_intention.dramatic_tension]
[scene_intention.trajectory]

## [Character Name]'s Drive
[objective + constraint + behavioral_stance — rendered as natural prose]

## [Character Name]'s Drive
[objective + constraint + behavioral_stance — rendered as natural prose]

## Player Context
[player character's known goals — based on visibility level]

## Boundaries
[hard constraints — existing]
```

### Token Budget

The preamble gains roughly 150-250 tokens of scene direction and character drives. Current preamble runs ~259 tokens; current total context assembly is ~773 tokens. There is substantial headroom within the existing narrator context budget.

### Player Grounding

The player's character goals are surfaced based on visibility:

- **Overt**: Displayed explicitly in the workshop UI and stated in the Player Context preamble section
- **Signaled**: Hinted at in the Player Context section; narrator drops behavioral clues
- **Hidden**: Not shown to the player; narrator renders through subtext
- **Structural**: Invisible to all; expressed through scene architecture

NPC goals are never shown directly to the player. The narrator renders NPC intentions through behavior — deflections, glances, steering of conversation — and the player discovers them through play.

For this branch, the player's character goals are displayed in the scene setup summary in the workshop UI.

## Session Persistence

### Session Directory

A new `goals.json` file in the session directory:

```
.story/sessions/{uuid}/
├── scene.json          # existing
├── characters.json     # existing
├── goals.json          # NEW
├── turns.jsonl         # existing
└── events.jsonl        # existing
```

### goals.json Contents

```json
{
  "active_scene_goals": [
    {
      "goal_id": "confront_shared_loss",
      "visibility": "Signaled",
      "selected_fragments": ["...", "..."]
    }
  ],
  "active_character_goals": {
    "arthur_entity_id": [
      {
        "goal_id": "protect_secret",
        "selected_fragments": ["...", "..."]
      }
    ]
  },
  "generated_intentions": {
    "scene_intention": { "dramatic_tension": "...", "trajectory": "..." },
    "character_intentions": [
      {
        "character": "Arthur",
        "objective": "...",
        "constraint": "...",
        "behavioral_stance": "..."
      }
    ]
  }
}
```

This stores the full audit trail: which goals were active, what lexicon material the LLM had, and what it produced. On session resume, intentions are rehydrated into the preamble from this file. No regeneration needed — intentions are stable for the life of the scene.

### turns.jsonl

Goals are static scene-level data, not per-turn. No changes to turns.jsonl structure for this branch. Future event evaluation work may add per-turn goal progress tracking.

## Future Work (Not This Branch)

### Event Evaluation Against Goals

The event classification pipeline gains a goal-matching pass. Categorized events are compared against character objectives and scene trajectory to detect threshold moments. This enables scene-aware pacing — the narrator can sense when a goal is approaching resolution.

### Gravity-Derived Intentions

The composition-time LLM pipeline accepts inputs from the narrative graph's gravitational calculations. High-gravity downstream scenes contribute transitional lexicon fragments and trajectory hints. On scene exit and entry, calculatory passes open/close narrative graph nodes, calculate gravitational force from upcoming high-mass nodes, and trace the relational web and recent event history to identify incipient state changes (fights, makeups, betrayals). These become anonymous goals expressed through the same lexicon interface — direction without named authorial intent.

### Connective-Tissue Scene Generation

Background jobs use the same pipeline to pre-generate intentions for unauthored scenes that bridge high-gravity authored moments. The goal vocabulary and dimensional lexicons provide raw material; the LLM generates scene-specific intentions maintaining narrative coherence across the sparse graph.

### Storyteller Modes

Configuration layer controlling player goal visibility and authorship:

- **Authored experience mode**: Player receives only authored goals, discovers everything through narrative. Storyteller controls information flow tightly.
- **Collaborative/RPG mode**: Player designs their character tensor and communicates goals pre-scene. Composition-time LLM weaves player-stated goals into intention generation alongside NPC goals, creating intentional friction points.

These are configuration choices on the same system — the intention generation pipeline doesn't change.

### Goal Completion and State Effects

When event evaluation detects a goal threshold has been met, the system triggers state changes — relational web updates, narrative graph node transitions, new goals becoming available. This closes the loop between intention generation and the living narrative state.
